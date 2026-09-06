//! The herdr CLI (`HERDR_BIN_PATH`, else `herdr` in PATH): reading what a pane shows, opening
//! plugin panes, and showing a notification.

use std::process::Command;

use serde::Deserialize;
use thiserror::Error;

use crate::process::{self, RunError, Stderr};
use crate::{Env, env_var};

/// Names declared in `herdr-plugin.toml`; `tests/version_sync.rs` keeps them equal.
pub const PLUGIN_ID: &str = "to4iki.fzf-terminal-browser";
pub const PICKER_ENTRYPOINT: &str = "picker";
pub const BROWSER_ENTRYPOINT: &str = "browser";

#[derive(Debug, Error)]
pub enum HerdrError {
    #[error("herdr binary not found (HERDR_BIN_PATH or `herdr` in PATH)")]
    NotFound,
    #[error("herdr: {0}")]
    Failed(String),
    #[error("could not parse `herdr pane get` output: {0}")]
    Parse(String),
}

impl From<RunError> for HerdrError {
    fn from(e: RunError) -> Self {
        match e {
            RunError::NotFound(_) => Self::NotFound,
            // herdr reports server errors as JSON on stderr; prefer its message when present.
            RunError::Failed { ref stderr, .. } => {
                Self::Failed(server_message(stderr).unwrap_or_else(|| e.to_string()))
            }
            RunError::Io { .. } => Self::Failed(e.to_string()),
        }
    }
}

/// Where a pane's viewport is, from `herdr pane get`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
struct Scroll {
    viewport_rows: u32,
    /// 0 when the pane shows its latest output; larger when the user scrolled back.
    offset_from_bottom: u32,
}

/// Which `herdr pane read` source returns exactly what the user sees, joining soft-wrapped lines
/// whenever herdr can: at the bottom, "the last `viewport_rows` rows, unwrapped" is the screen;
/// while scrolled back only `visible` follows the scroll position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadSource {
    Visible,
    RecentUnwrapped { lines: u32 },
}

fn choose_source(scroll: Scroll) -> ReadSource {
    if scroll.offset_from_bottom == 0 {
        ReadSource::RecentUnwrapped {
            lines: scroll.viewport_rows,
        }
    } else {
        ReadSource::Visible
    }
}

/// Reads what the pane shows right now: `pane get` for the viewport, then the matching `pane read`.
pub fn read_screen(env: &Env, pane_id: &str) -> Result<String, HerdrError> {
    let out = run(env, ["pane", "get", pane_id])?;
    let scroll = parse_pane_get(&String::from_utf8_lossy(&out.stdout))?;
    let out = run(env, pane_read_args(pane_id, choose_source(scroll)))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `herdr notification show <title> --sound none`: a quiet toast in the herdr UI.
pub fn notify(env: &Env, title: &str) -> Result<(), HerdrError> {
    run(env, ["notification", "show", title, "--sound", "none"]).map(|_| ())
}

/// A `herdr plugin pane open` request for one of this plugin's entrypoints.
#[derive(Debug, Clone, Default)]
pub struct PluginPaneOpen<'a> {
    pub entrypoint: &'a str,
    /// `None` keeps the manifest's placement (the popup for `picker`).
    pub placement: Option<&'a str>,
    pub target_pane: Option<&'a str>,
    pub direction: Option<&'a str>,
    pub no_focus: bool,
    pub env: &'a [(&'a str, &'a str)],
}

/// Opens a plugin pane entrypoint.
pub fn plugin_pane_open(env: &Env, opts: &PluginPaneOpen) -> Result<(), HerdrError> {
    run(env, plugin_pane_open_args(opts)).map(|_| ())
}

fn run<I, S>(env: &Env, args: I) -> Result<std::process::Output, HerdrError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let bin = env_var(env, "HERDR_BIN_PATH").unwrap_or("herdr");
    Ok(process::run(
        Command::new(bin).args(args),
        None,
        Stderr::Capture,
    )?)
}

fn pane_read_args(pane_id: &str, source: ReadSource) -> Vec<String> {
    let mut args: Vec<String> = ["pane", "read", pane_id, "--source"]
        .map(String::from)
        .to_vec();
    match source {
        ReadSource::Visible => args.push("visible".into()),
        ReadSource::RecentUnwrapped { lines } => {
            args.extend([
                "recent-unwrapped".into(),
                "--lines".into(),
                lines.to_string(),
            ]);
        }
    }
    args.extend(["--format", "text"].map(String::from));
    args
}

fn plugin_pane_open_args(opts: &PluginPaneOpen) -> Vec<String> {
    let mut args = [
        "plugin",
        "pane",
        "open",
        "--plugin",
        PLUGIN_ID,
        "--entrypoint",
        opts.entrypoint,
    ]
    .map(String::from)
    .to_vec();
    for (flag, value) in [
        ("--placement", opts.placement),
        ("--target-pane", opts.target_pane),
        ("--direction", opts.direction),
    ] {
        if let Some(value) = value {
            args.extend([flag.into(), value.into()]);
        }
    }
    if opts.no_focus {
        args.push("--no-focus".into());
    }
    for (k, v) in opts.env {
        args.extend(["--env".into(), format!("{k}={v}")]);
    }
    args
}

/// `herdr pane get` prints `{"result":{"pane":{...,"scroll":{...}}}}`; only `scroll` is read.
fn parse_pane_get(json: &str) -> Result<Scroll, HerdrError> {
    #[derive(Deserialize)]
    struct Pane {
        scroll: Scroll,
    }
    #[derive(Deserialize)]
    struct Result_ {
        pane: Pane,
    }
    #[derive(Deserialize)]
    struct Reply {
        result: Result_,
    }
    serde_json::from_str::<Reply>(json)
        .map(|r| r.result.pane.scroll)
        .map_err(|e| HerdrError::Parse(e.to_string()))
}

/// herdr's stderr for server errors is `{"error":{"code":..,"message":..}}`.
fn server_message(stderr: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(stderr).ok()?;
    let obj = value.get("error").unwrap_or(&value);
    let message = obj.get("message")?.as_str()?;
    Some(match obj.get("code").and_then(|c| c.as_str()) {
        Some(code) => format!("{message} ({code})"),
        None => message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from `herdr pane get` on herdr 0.8.2 (unrelated fields omitted; they are ignored).
    const PANE_GET: &str = r#"{"id":"cli:pane:get","result":{"pane":{"pane_id":"w1E:p1","tab_id":"w1E:t1","focused":true,"scroll":{"max_offset_from_bottom":57,"offset_from_bottom":0,"viewport_rows":69}},"type":"pane_info"}}"#;

    #[test]
    fn parses_pane_get_and_picks_the_source() {
        let scroll = parse_pane_get(PANE_GET).unwrap();
        assert_eq!(
            choose_source(scroll),
            ReadSource::RecentUnwrapped { lines: 69 }
        );
        assert_eq!(
            choose_source(Scroll {
                viewport_rows: 69,
                offset_from_bottom: 12
            }),
            ReadSource::Visible
        );
        assert!(parse_pane_get("{}").is_err());
    }

    #[test]
    fn builds_read_args() {
        assert_eq!(
            pane_read_args("w1:p1", ReadSource::RecentUnwrapped { lines: 69 }),
            [
                "pane",
                "read",
                "w1:p1",
                "--source",
                "recent-unwrapped",
                "--lines",
                "69",
                "--format",
                "text"
            ]
        );
        assert_eq!(
            pane_read_args("w1:p1", ReadSource::Visible),
            [
                "pane", "read", "w1:p1", "--source", "visible", "--format", "text"
            ]
        );
    }

    #[test]
    fn builds_popup_and_split_open_args() {
        let popup = PluginPaneOpen {
            entrypoint: PICKER_ENTRYPOINT,
            env: &[("FZF_TB_SOURCE_PANE", "w1:p1")],
            ..Default::default()
        };
        assert_eq!(
            plugin_pane_open_args(&popup),
            [
                "plugin",
                "pane",
                "open",
                "--plugin",
                PLUGIN_ID,
                "--entrypoint",
                "picker",
                "--env",
                "FZF_TB_SOURCE_PANE=w1:p1"
            ]
        );
        let split = PluginPaneOpen {
            entrypoint: BROWSER_ENTRYPOINT,
            placement: Some("split"),
            target_pane: Some("w1:p1"),
            direction: Some("right"),
            no_focus: true,
            env: &[("FZF_TB_URL", "https://a")],
        };
        assert_eq!(
            plugin_pane_open_args(&split),
            [
                "plugin",
                "pane",
                "open",
                "--plugin",
                PLUGIN_ID,
                "--entrypoint",
                "browser",
                "--placement",
                "split",
                "--target-pane",
                "w1:p1",
                "--direction",
                "right",
                "--no-focus",
                "--env",
                "FZF_TB_URL=https://a"
            ]
        );
    }

    #[test]
    fn recognises_json_server_errors() {
        assert_eq!(
            server_message(r#"{"error":{"code":"pane_not_found","message":"no pane w1:p9"}}"#),
            Some("no pane w1:p9 (pane_not_found)".into())
        );
        assert_eq!(server_message("missing required --plugin"), None);
    }
}

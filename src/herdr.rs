//! The herdr CLI (`HERDR_BIN_PATH`, else `herdr` in PATH): reading what a pane shows and opening
//! plugin panes.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde::Deserialize;
use thiserror::Error;

use crate::Env;

#[derive(Debug, Error)]
pub enum HerdrError {
    #[error("herdr binary not found (HERDR_BIN_PATH or `herdr` in PATH)")]
    NotFound,
    #[error("herdr: {0}")]
    Failed(String),
    #[error("could not parse `herdr {what}` output: {message}")]
    Parse { what: &'static str, message: String },
    #[error("could not run herdr: {0}")]
    Io(String),
}

/// The herdr binary to spawn.
#[must_use]
pub fn bin(env: &Env) -> PathBuf {
    env.get("HERDR_BIN_PATH")
        .filter(|p| !p.is_empty())
        .map_or_else(|| PathBuf::from("herdr"), PathBuf::from)
}

/// Where a pane's viewport is, from `herdr pane get`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Scroll {
    pub viewport_rows: u32,
    /// 0 when the pane shows its latest output; larger when the user scrolled back.
    pub offset_from_bottom: u32,
}

/// Which `herdr pane read` source to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSource {
    /// The rendered screen as-is. Follows the scroll position, but soft-wrapped lines stay split.
    Visible,
    /// The last `lines` rendered rows with soft wraps joined back into whole lines.
    RecentUnwrapped { lines: u32 },
}

/// Picks the source that returns exactly what the user sees, joining soft-wrapped lines whenever
/// herdr can: at the bottom, "the last `viewport_rows` rows, unwrapped" is the screen; while
/// scrolled back only `visible` follows the scroll position.
#[must_use]
pub fn choose_source(scroll: Scroll) -> ReadSource {
    if scroll.offset_from_bottom == 0 {
        ReadSource::RecentUnwrapped {
            lines: scroll.viewport_rows,
        }
    } else {
        ReadSource::Visible
    }
}

/// Reads what the pane shows right now: `pane get` for the viewport, then the matching `pane read`.
///
/// # Errors
///
/// [`HerdrError`] when herdr is missing, a command fails, or its output cannot be parsed.
pub fn read_screen(env: &Env, pane_id: &str) -> Result<String, HerdrError> {
    let scroll = pane_scroll(env, pane_id)?;
    read_pane(env, pane_id, choose_source(scroll))
}

/// `herdr notification show <title> --sound none`: a quiet toast in the herdr UI.
///
/// # Errors
///
/// [`HerdrError`] when herdr is missing or the command fails.
pub fn notify(env: &Env, title: &str) -> Result<(), HerdrError> {
    run(env, &notification_show_args(title)).map(|_| ())
}

/// `herdr pane get <pane>`, reduced to its scroll state.
///
/// # Errors
///
/// [`HerdrError`] when herdr is missing, the pane does not exist, or the JSON is unexpected.
pub fn pane_scroll(env: &Env, pane_id: &str) -> Result<Scroll, HerdrError> {
    let out = run(env, &pane_get_args(pane_id))?;
    parse_pane_get(&String::from_utf8_lossy(&out.stdout))
}

/// `herdr pane read <pane> --source … --format text`, as plain text.
///
/// # Errors
///
/// [`HerdrError`] when the binary is missing or the command fails.
pub fn read_pane(env: &Env, pane_id: &str, source: ReadSource) -> Result<String, HerdrError> {
    let out = run(env, &pane_read_args(pane_id, source))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A `herdr plugin pane open` request. The plugin id is `HERDR_PLUGIN_ID` when herdr set it
/// (inside actions and plugin panes), else [`crate::PLUGIN_ID`] (from a shell).
#[derive(Debug, Clone, Default)]
pub struct PluginPaneOpen<'a> {
    pub entrypoint: &'a str,
    /// `None` keeps the manifest's placement (the popup for `picker`).
    pub placement: Option<&'a str>,
    pub target_pane: Option<&'a str>,
    pub direction: Option<&'a str>,
    pub no_focus: bool,
    pub env: Vec<(String, String)>,
}

/// Opens a plugin pane entrypoint.
///
/// # Errors
///
/// [`HerdrError`] when the binary is missing or the command fails (e.g. the plugin is not linked).
pub fn plugin_pane_open(env: &Env, opts: &PluginPaneOpen) -> Result<(), HerdrError> {
    let plugin_id = env
        .get("HERDR_PLUGIN_ID")
        .filter(|s| !s.is_empty())
        .map_or(crate::PLUGIN_ID, String::as_str);
    run(env, &plugin_pane_open_args(plugin_id, opts)).map(|_| ())
}

fn run(env: &Env, args: &[String]) -> Result<Output, HerdrError> {
    let output = Command::new(bin(env))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => HerdrError::NotFound,
            _ => HerdrError::Io(e.to_string()),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(HerdrError::Failed(
            server_message(&stderr).unwrap_or(stderr),
        ));
    }
    Ok(output)
}

fn pane_get_args(pane_id: &str) -> Vec<String> {
    vec!["pane".into(), "get".into(), pane_id.into()]
}

fn notification_show_args(title: &str) -> Vec<String> {
    ["notification", "show", title, "--sound", "none"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn pane_read_args(pane_id: &str, source: ReadSource) -> Vec<String> {
    let mut args: Vec<String> = ["pane", "read", pane_id, "--source"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    match source {
        ReadSource::Visible => args.push("visible".into()),
        ReadSource::RecentUnwrapped { lines } => {
            args.push("recent-unwrapped".into());
            args.push("--lines".into());
            args.push(lines.to_string());
        }
    }
    args.push("--format".into());
    args.push("text".into());
    args
}

fn plugin_pane_open_args(plugin_id: &str, opts: &PluginPaneOpen) -> Vec<String> {
    let mut args = vec![
        "plugin".to_string(),
        "pane".to_string(),
        "open".to_string(),
        "--plugin".to_string(),
        plugin_id.to_string(),
        "--entrypoint".to_string(),
        opts.entrypoint.to_string(),
    ];
    if let Some(placement) = opts.placement {
        args.push("--placement".to_string());
        args.push(placement.to_string());
    }
    if let Some(target) = opts.target_pane {
        args.push("--target-pane".to_string());
        args.push(target.to_string());
    }
    if let Some(direction) = opts.direction {
        args.push("--direction".to_string());
        args.push(direction.to_string());
    }
    if opts.no_focus {
        args.push("--no-focus".to_string());
    }
    for (k, v) in &opts.env {
        args.push("--env".to_string());
        args.push(format!("{k}={v}"));
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
        .map_err(|e| HerdrError::Parse {
            what: "pane get",
            message: e.to_string(),
        })
}

/// herdr reports server errors as JSON on stderr (`{"error":{"code":..,"message":..}}`).
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
            scroll,
            Scroll {
                viewport_rows: 69,
                offset_from_bottom: 0
            }
        );
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
    fn builds_read_and_get_args() {
        assert_eq!(pane_get_args("w1:p1"), ["pane", "get", "w1:p1"]);
        assert_eq!(
            notification_show_args("No URLs on screen"),
            [
                "notification",
                "show",
                "No URLs on screen",
                "--sound",
                "none"
            ]
        );
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
            entrypoint: "picker",
            env: vec![("FZF_TB_SOURCE_PANE".into(), "w1:p1".into())],
            ..Default::default()
        };
        assert_eq!(
            plugin_pane_open_args("to4iki.fzf-terminal-browser", &popup),
            [
                "plugin",
                "pane",
                "open",
                "--plugin",
                "to4iki.fzf-terminal-browser",
                "--entrypoint",
                "picker",
                "--env",
                "FZF_TB_SOURCE_PANE=w1:p1"
            ]
        );
        let split = PluginPaneOpen {
            entrypoint: "browser",
            placement: Some("split"),
            target_pane: Some("w1:p1"),
            direction: Some("right"),
            no_focus: true,
            env: vec![("FZF_TB_URL".into(), "https://a".into())],
        };
        assert_eq!(
            plugin_pane_open_args("p", &split),
            [
                "plugin",
                "pane",
                "open",
                "--plugin",
                "p",
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

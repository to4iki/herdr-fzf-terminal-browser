//! [`TerminalBrowser`] implemented with the `terminal-browser` and `herdr` CLIs.
//!
//! Listing and `new-tab` go straight to `terminal-browser`. Opening a *new* browser does not use
//! `terminal-browser open --split`: that splits with `herdr pane split` and then types the launch
//! command into the new pane's shell, so the prompt and the echoed command are visible for the
//! seconds the browser takes to start. Instead we ask herdr for a plugin split pane running our
//! own `browser` entrypoint, which execs `terminal-browser open <url>` directly.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde::Deserialize;

use super::{BrowserInstance, TbError, TerminalBrowser};
use crate::Env;
use crate::context::SourcePane;
use crate::herdr::{self, PluginPaneOpen};

pub const BIN: &str = "terminal-browser";
/// Env for the `browser` entrypoint: the URL to open.
pub const ENV_URL: &str = "FZF_TB_URL";
/// Env for the `browser` entrypoint: the terminal-browser path the picker already resolved.
pub const ENV_BROWSER: &str = "FZF_TB_BROWSER";
pub const BROWSER_ENTRYPOINT: &str = "browser";

#[derive(Debug, Clone)]
pub struct CliTerminalBrowser {
    browser: PathBuf,
    env: Env,
}

impl CliTerminalBrowser {
    /// `browser` is the terminal-browser binary the caller resolved from `PATH`.
    #[must_use]
    pub fn new(browser: PathBuf, env: Env) -> Self {
        Self { browser, env }
    }

    /// Runs terminal-browser as if from inside the source pane: only the child's environment
    /// gets `HERDR_PANE_ID` / `HERDR_TAB_ID` replaced.
    fn run(&self, source: &SourcePane, args: &[&str]) -> Result<Output, TbError> {
        let output = Command::new(&self.browser)
            .args(args)
            .env("HERDR_PANE_ID", source.pane_id())
            .env("HERDR_TAB_ID", source.tab_id())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => TbError::NotFound,
                _ => TbError::Io(e.to_string()),
            })?;
        if !output.status.success() {
            return Err(TbError::CommandFailed {
                cmd: args.join(" "),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(output)
    }
}

/// One row of `ls --json`. Only what this plugin reads; other fields are ignored so
/// terminal-browser can add fields freely.
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Record {
    key: Option<String>,
    pid: Option<u64>,
    in_current_tab: bool,
}

#[derive(Deserialize)]
struct LsOutput {
    #[serde(default)]
    browsers: Vec<Record>,
}

fn parse_ls(json: &str) -> Result<Vec<BrowserInstance>, TbError> {
    let out: LsOutput = serde_json::from_str(json).map_err(|e| TbError::Parse {
        what: "ls --json output",
        message: e.to_string(),
    })?;
    Ok(out
        .browsers
        .into_iter()
        .filter_map(|r| {
            // terminal-browser itself falls back to the pid when a record has no key.
            let key = r.key.or_else(|| r.pid.map(|p| p.to_string()))?;
            Some(BrowserInstance {
                key,
                in_current_tab: r.in_current_tab,
            })
        })
        .collect())
}

impl TerminalBrowser for CliTerminalBrowser {
    fn list(&self, source: &SourcePane) -> Result<Vec<BrowserInstance>, TbError> {
        let out = self.run(source, &["ls", "--json"])?;
        parse_ls(&String::from_utf8_lossy(&out.stdout))
    }

    fn open_split(&self, source: &SourcePane, url: &str) -> Result<(), TbError> {
        herdr::plugin_pane_open(
            &self.env,
            &PluginPaneOpen {
                entrypoint: BROWSER_ENTRYPOINT,
                placement: Some("split"),
                target_pane: Some(source.pane_id()),
                direction: Some("right"),
                no_focus: true,
                env: vec![
                    (ENV_URL.to_string(), url.to_string()),
                    (ENV_BROWSER.to_string(), self.browser.display().to_string()),
                ],
            },
        )
        .map_err(TbError::from)
    }

    fn new_tab(&self, source: &SourcePane, key: &str, url: &str) -> Result<(), TbError> {
        self.run(source, &["new-tab", "--browser", key, url])
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from `terminal-browser ls --json` (v0.6.0) with the source pane's env.
    const LS_JSON: &str = r#"{
  "self": {"tab": "w12:t4", "pane": "w12:p7"},
  "browsers": [
    {"key": "86811-1", "pid": 86811, "socket": "/x/86811-1.sock", "pane": {"tab": "w12:t4", "pane": "w12:p8"},
     "paneTab": null, "inCurrentTab": true, "splitDir": "right"},
    {"pid": 700, "inCurrentTab": false}
  ]
}"#;

    #[test]
    fn parses_real_ls_output() {
        assert_eq!(
            parse_ls(LS_JSON).unwrap(),
            vec![
                BrowserInstance {
                    key: "86811-1".into(),
                    in_current_tab: true,
                },
                BrowserInstance {
                    key: "700".into(),
                    in_current_tab: false,
                },
            ]
        );
        assert!(parse_ls("nope").is_err());
    }
}

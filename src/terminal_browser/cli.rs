//! [`TerminalBrowser`] implemented with the `terminal-browser` and `herdr` CLIs.
//!
//! Listing and `new-tab` go straight to `terminal-browser`. Opening a *new* browser does not use
//! `terminal-browser open --split`: that splits with `herdr pane split` and then types the launch
//! command into the new pane's shell, so the prompt and the echoed command are visible for the
//! seconds the browser takes to start. Instead we ask herdr for a plugin split pane running our
//! own `browser` entrypoint, which becomes `terminal-browser open <url>` directly ([`exec_open`]).

use std::cell::RefCell;
use std::os::unix::process::CommandExt as _;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::{BrowserInstance, TbError, TerminalBrowser};
use crate::context::SourcePane;
use crate::herdr::{self, BROWSER_ENTRYPOINT, PluginPaneOpen};
use crate::process::{self, Running, Stderr};
use crate::{Env, find_in_path};

pub const BIN: &str = "terminal-browser";
/// Env for the `browser` entrypoint: the URL to open.
pub const ENV_URL: &str = "FZF_TB_URL";
/// Env for the `browser` entrypoint: the terminal-browser path the picker resolved. Only a
/// shell-run `open` needs it (the user's PATH may differ from the herdr server's); the herdr-run
/// action → popup → browser-pane chain would find it either way.
pub const ENV_BROWSER: &str = "FZF_TB_BROWSER";

pub struct CliTerminalBrowser<'a> {
    browser: PathBuf,
    env: &'a Env,
    /// A `ls --json` started by [`TerminalBrowser::prefetch`], drained by the next `list`.
    pending_ls: RefCell<Option<Running>>,
}

impl<'a> CliTerminalBrowser<'a> {
    /// Finds terminal-browser in `PATH`; fails early so nothing is picked when it is missing.
    pub fn locate(env: &'a Env) -> Result<Self, TbError> {
        Ok(Self {
            browser: find_in_path(env, BIN).ok_or(TbError::NotFound)?,
            env,
            pending_ls: RefCell::new(None),
        })
    }

    /// terminal-browser, run as if from inside the source pane (the child env alone gets
    /// `HERDR_PANE_ID` / `HERDR_TAB_ID` replaced).
    fn command(&self, source: &SourcePane, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.browser);
        cmd.args(args).envs(source.herdr_env());
        cmd
    }
}

/// Replaces the current process with `terminal-browser open <url>` (the `browser` pane); only
/// returns if that failed.
#[must_use]
pub fn exec_open(bin: &str, url: &str) -> std::io::Error {
    Command::new(bin).args(["open", url]).exec()
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
    let out: LsOutput = serde_json::from_str(json).map_err(|e| TbError::Parse(e.to_string()))?;
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

impl TerminalBrowser for CliTerminalBrowser<'_> {
    fn prefetch(&self, source: &SourcePane) {
        // Best effort: a failure here just means `list` runs the command itself.
        *self.pending_ls.borrow_mut() = process::spawn(
            &mut self.command(source, &["ls", "--json"]),
            None,
            Stderr::Capture,
        )
        .ok();
    }

    fn list(&self, source: &SourcePane) -> Result<Vec<BrowserInstance>, TbError> {
        let running = match self.pending_ls.borrow_mut().take() {
            Some(running) => running,
            None => process::spawn(
                &mut self.command(source, &["ls", "--json"]),
                None,
                Stderr::Capture,
            )?,
        };
        let out = process::finish(running)?;
        parse_ls(&String::from_utf8_lossy(&out.stdout))
    }

    fn open_split(&self, source: &SourcePane, url: &str) -> Result<(), TbError> {
        let browser = self.browser.to_string_lossy();
        Ok(herdr::plugin_pane_open(
            self.env,
            &PluginPaneOpen {
                entrypoint: BROWSER_ENTRYPOINT,
                placement: Some("split"),
                target_pane: Some(source.pane_id()),
                direction: Some("right"),
                no_focus: true,
                env: &[(ENV_URL, url), (ENV_BROWSER, &browser)],
            },
        )?)
    }

    fn new_tab(&self, source: &SourcePane, key: &str, url: &str) -> Result<(), TbError> {
        process::run(
            &mut self.command(source, &["new-tab", "--browser", key, url]),
            None,
            Stderr::Capture,
        )?;
        Ok(())
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

//! `open <url>`: the terminal-browser dispatcher on its own, for shells inside herdr
//! (`HERDR_PANE_ID` / `HERDR_TAB_ID` are then the source pane) and for scripting.

use std::process::ExitCode;

use crate::Env;
use crate::context::SourcePane;
use crate::opener::find_in_path;
use crate::terminal_browser::TbError;
use crate::terminal_browser::cli::{self, CliTerminalBrowser};
use crate::terminal_browser::open_url;
use crate::ui::NAME;

#[must_use]
pub fn run(env: &Env, url: &str) -> ExitCode {
    match open(env, url) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{NAME}: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn open(env: &Env, url: &str) -> Result<(), String> {
    let source = SourcePane::from_env(env).map_err(|e| e.to_string())?;
    let browser = find_in_path(env, cli::BIN).ok_or_else(|| TbError::NotFound.to_string())?;
    let tb = CliTerminalBrowser::new(browser, env.clone());
    open_url(&tb, &source, url).map_err(|e| e.to_string())
}

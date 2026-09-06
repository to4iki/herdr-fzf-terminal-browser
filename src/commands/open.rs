//! `open <url>`: the terminal-browser dispatcher on its own, for shells inside herdr
//! (`HERDR_PANE_ID` / `HERDR_TAB_ID` are then the source pane) and for scripting.

use std::error::Error;
use std::process::ExitCode;

use crate::context::SourcePane;
use crate::terminal_browser::cli::CliTerminalBrowser;
use crate::terminal_browser::open_url;
use crate::{Env, ui};

#[must_use]
pub fn run(env: &Env, url: &str) -> ExitCode {
    ui::report(open(env, url))
}

fn open(env: &Env, url: &str) -> Result<(), Box<dyn Error>> {
    let source = SourcePane::from_env(env)?;
    open_url(&CliTerminalBrowser::locate(env)?, &source, url)?;
    Ok(())
}

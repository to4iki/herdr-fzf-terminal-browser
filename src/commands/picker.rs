//! `picker`: runs in the popup (a real PTY). Read the source pane, extract URLs, let the user pick
//! one with fzf, then open it in terminal-browser (Enter) or copy it (ctrl-y).

use std::error::Error;
use std::process::ExitCode;

use crate::context::SourcePane;
use crate::extract::extract_urls;
use crate::fzf::{self, FzfError, Key};
use crate::terminal_browser::cli::CliTerminalBrowser;
use crate::terminal_browser::{TerminalBrowser as _, open_url};
use crate::{Env, clipboard, herdr, ui};

#[must_use]
pub fn run(env: &Env) -> ExitCode {
    match pick(env) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            ui::fail_and_wait(&e.to_string());
            ExitCode::FAILURE
        }
    }
}

fn pick(env: &Env) -> Result<(), Box<dyn Error>> {
    let source = SourcePane::from_env(env)?;
    // Fail before the user picks anything if terminal-browser is missing.
    let tb = CliTerminalBrowser::locate(env)?;

    // What the user sees is what they can pick: the pane's current screen, nothing more.
    // `pick` already checked for URLs; this only triggers if the screen changed in between.
    let urls = extract_urls(&herdr::read_screen(env, source.pane_id())?);
    if urls.is_empty() {
        ui::info_and_wait(ui::NO_URLS);
        return Ok(());
    }

    // `terminal-browser ls` takes ~175 ms; let it run while the user is choosing.
    tb.prefetch(&source);
    let selection = match fzf::run(&urls) {
        Ok(selection) => selection,
        Err(FzfError::Cancelled) => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    match selection.key {
        Key::CtrlY => clipboard::copy(env, &format!("{}\n", selection.url))?,
        Key::Enter => open_url(&tb, &source, &selection.url)?,
    }
    Ok(())
}

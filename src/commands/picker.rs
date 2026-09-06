//! `picker`: runs in the popup (a real PTY). Read the source pane, extract URLs, let the user pick
//! one with fzf, then open it in terminal-browser (Enter) or copy it (ctrl-y).

use std::process::ExitCode;

use crate::context::SourcePane;
use crate::extract::extract_urls;
use crate::fzf::{self, FzfError, Key};
use crate::opener::copy_to_clipboard;
use crate::terminal_browser::TbError;
use crate::terminal_browser::cli::{self, CliTerminalBrowser};
use crate::terminal_browser::open_url;
use crate::{Env, find_in_path};
use crate::{herdr, ui};

#[must_use]
pub fn run(env: &Env) -> ExitCode {
    match pick(env) {
        Ok(()) | Err(Outcome::Quiet) => ExitCode::SUCCESS,
        Err(Outcome::Info(msg)) => {
            ui::info_and_wait(&msg);
            ExitCode::SUCCESS
        }
        Err(Outcome::Fail(msg)) => {
            ui::fail_and_wait(&msg);
            ExitCode::FAILURE
        }
    }
}

enum Outcome {
    /// Nothing to say (the user cancelled).
    Quiet,
    /// Not an error, but the user should read it before the popup closes.
    Info(String),
    Fail(String),
}

fn fail(e: impl std::fmt::Display) -> Outcome {
    Outcome::Fail(e.to_string())
}

fn pick(env: &Env) -> Result<(), Outcome> {
    let source = SourcePane::from_env(env).map_err(fail)?;
    if find_in_path(env, fzf::BIN).is_none() {
        return Err(fail(FzfError::NotFound));
    }
    let browser = find_in_path(env, cli::BIN).ok_or_else(|| fail(TbError::NotFound))?;

    // What the user sees is what they can pick: the pane's current screen, nothing more.
    // `pick` already checked for URLs; this only triggers if the screen changed in between.
    let text = herdr::read_screen(env, source.pane_id()).map_err(fail)?;
    let urls = extract_urls(&text);
    if urls.is_empty() {
        return Err(Outcome::Info(ui::NO_URLS.to_string()));
    }

    let selection = match fzf::run(&urls) {
        Ok(sel) => sel,
        Err(FzfError::Cancelled) => return Err(Outcome::Quiet),
        Err(e) => return Err(fail(e)),
    };
    match selection.key {
        Key::CtrlY => copy_to_clipboard(env, &format!("{}\n", selection.url)).map_err(fail),
        Key::Enter => {
            let tb = CliTerminalBrowser::new(browser, env.clone());
            open_url(&tb, &source, &selection.url).map_err(fail)
        }
    }
}

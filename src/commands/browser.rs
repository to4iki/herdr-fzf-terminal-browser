//! `browser`: the split pane herdr opens next to the source pane. It simply becomes
//! `terminal-browser open <url>` (no shell in between, so nothing is echoed while the browser
//! starts). Errors are shown and wait for Enter, because the pane closes when we exit.

use std::os::unix::process::CommandExt as _;
use std::process::{Command, ExitCode};

use crate::Env;
use crate::terminal_browser::cli::{BIN, ENV_BROWSER, ENV_URL};
use crate::ui;

#[must_use]
pub fn run(env: &Env) -> ExitCode {
    let Some(url) = env.get(ENV_URL).filter(|u| !u.is_empty()) else {
        ui::fail_and_wait(&format!("{ENV_URL} is not set (the picker sets it)"));
        return ExitCode::FAILURE;
    };
    let bin = env
        .get(ENV_BROWSER)
        .filter(|b| !b.is_empty())
        .map_or(BIN, String::as_str);
    let err = Command::new(bin).arg("open").arg(url).exec();
    ui::fail_and_wait(&format!("could not start {bin}: {err}"));
    ExitCode::FAILURE
}

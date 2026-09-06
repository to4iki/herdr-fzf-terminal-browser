//! `browser`: the split pane herdr opens next to the source pane. It simply becomes
//! `terminal-browser open <url>` (no shell in between, so nothing is echoed while the browser
//! starts). Errors are shown and wait for Enter, because the pane closes when we exit.

use std::process::ExitCode;

use crate::terminal_browser::cli::{BIN, ENV_BROWSER, ENV_URL, exec_open};
use crate::{Env, env_var, ui};

#[must_use]
pub fn run(env: &Env) -> ExitCode {
    let Some(url) = env_var(env, ENV_URL) else {
        ui::fail_and_wait(&format!("{ENV_URL} is not set (the picker sets it)"));
        return ExitCode::FAILURE;
    };
    let bin = env_var(env, ENV_BROWSER).unwrap_or(BIN);
    let err = exec_open(bin, url);
    ui::fail_and_wait(&format!("could not start {bin}: {err}"));
    ExitCode::FAILURE
}

//! ctrl-y: copy the picked URL with whatever clipboard tool the platform has.

use std::process::Command;

use thiserror::Error;

use crate::process::{self, RunError, Stderr};
use crate::{Env, env_var, find_in_path};

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("no clipboard tool found (looked for pbcopy, wl-copy, xclip, xsel)")]
    NoTool,
    #[error("{0}")]
    Failed(RunError),
}

impl From<RunError> for ClipboardError {
    fn from(e: RunError) -> Self {
        match e {
            RunError::NotFound(_) => Self::NoTool,
            other => Self::Failed(other),
        }
    }
}

fn command(env: &Env) -> Result<Vec<&'static str>, ClipboardError> {
    let has = |name: &str| find_in_path(env, name).is_some();
    let set = |key: &str| env_var(env, key).is_some();
    if has("pbcopy") {
        Ok(vec!["pbcopy"])
    } else if set("WAYLAND_DISPLAY") && has("wl-copy") {
        Ok(vec!["wl-copy"])
    } else if set("DISPLAY") && has("xclip") {
        Ok(vec!["xclip", "-selection", "clipboard"])
    } else if set("DISPLAY") && has("xsel") {
        Ok(vec!["xsel", "--clipboard", "--input"])
    } else {
        Err(ClipboardError::NoTool)
    }
}

/// Copies `text` to the clipboard.
pub fn copy(env: &Env, text: &str) -> Result<(), ClipboardError> {
    let cmd = command(env)?;
    process::run(
        Command::new(cmd[0]).args(&cmd[1..]),
        Some(text.as_bytes()),
        Stderr::Capture,
    )?;
    Ok(())
}

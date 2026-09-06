//! ctrl-y: copy the picked URL with whatever clipboard tool the platform has.

use std::io::Write as _;
use std::process::{Command, Stdio};

use thiserror::Error;

use crate::{Env, find_in_path};

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("no clipboard tool found (looked for pbcopy, wl-copy, xclip, xsel)")]
    NoTool,
    #[error("`{cmd}` failed: {message}")]
    Failed { cmd: String, message: String },
}

fn clipboard_command(env: &Env) -> Result<Vec<&'static str>, ClipboardError> {
    let has = |name: &str| find_in_path(env, name).is_some();
    let set = |key: &str| env.get(key).is_some_and(|v| !v.is_empty());
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
///
/// # Errors
///
/// [`ClipboardError`] when no tool exists or the tool fails.
pub fn copy_to_clipboard(env: &Env, text: &str) -> Result<(), ClipboardError> {
    let cmd = clipboard_command(env)?;
    let failed = |message: String| ClipboardError::Failed {
        cmd: cmd.join(" "),
        message,
    };
    let mut child = Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| failed(e.to_string()))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| failed(e.to_string()))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| failed(e.to_string()))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(failed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

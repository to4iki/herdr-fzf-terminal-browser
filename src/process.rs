//! The one way this plugin runs an external program: spawn, optionally feed stdin, wait, and turn
//! "not installed" or a non-zero exit into a [`RunError`] that names the command.

use std::io::Write as _;
use std::process::{Child, Command, Output, Stdio};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("{0} was not found in PATH")]
    NotFound(String),
    #[error("`{cmd}` failed{}: {stderr}", code.map(|c| format!(" (exit {c})")).unwrap_or_default())]
    Failed {
        cmd: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("could not run `{cmd}`: {message}")]
    Io { cmd: String, message: String },
}

/// What to do with the child's stderr.
#[derive(Debug, Clone, Copy)]
pub enum Stderr {
    /// Capture it for the error message (herdr, terminal-browser, clipboard tools).
    Capture,
    /// Leave it on the terminal (fzf draws its UI there).
    Inherit,
}

/// A spawned command, to be completed with [`finish`].
pub struct Running {
    child: Child,
    cmd: String,
}

/// Spawns `cmd` with stdout captured, writes `input` to its stdin when given (a write failure is
/// left for the exit status to explain: fzf, for one, exits early on Esc).
pub fn spawn(cmd: &mut Command, input: Option<&[u8]>, stderr: Stderr) -> Result<Running, RunError> {
    let desc = describe(cmd);
    cmd.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(match stderr {
        Stderr::Capture => Stdio::piped(),
        Stderr::Inherit => Stdio::inherit(),
    });
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            RunError::NotFound(cmd.get_program().to_string_lossy().into_owned())
        }
        _ => RunError::Io {
            cmd: desc.clone(),
            message: e.to_string(),
        },
    })?;
    if let (Some(bytes), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(bytes);
    }
    Ok(Running { child, cmd: desc })
}

/// Waits for a spawned command; a non-zero exit becomes [`RunError::Failed`].
pub fn finish(running: Running) -> Result<Output, RunError> {
    let Running { child, cmd } = running;
    let output = child.wait_with_output().map_err(|e| RunError::Io {
        cmd: cmd.clone(),
        message: e.to_string(),
    })?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(RunError::Failed {
            cmd,
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

/// [`spawn`] then [`finish`].
pub fn run(cmd: &mut Command, input: Option<&[u8]>, stderr: Stderr) -> Result<Output, RunError> {
    finish(spawn(cmd, input, stderr)?)
}

fn describe(cmd: &Command) -> String {
    std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|a| a.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

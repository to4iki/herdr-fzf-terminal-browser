//! Running fzf and reading back what the user picked.
//!
//! fzf draws its full-screen UI on the pane's TTY (stderr is inherited), reads the candidates
//! from stdin, and prints the selection on stdout. With `--expect`, the first stdout line names
//! the key that ended the selection ("" for Enter).
//!
//! Do not add `--height`: fzf then reserves rows by printing newlines, which looks like the pane
//! scrolling before the list appears.

use std::fmt::Write as _;
use std::process::Command;

use thiserror::Error;

use crate::process::{self, RunError, Stderr};

pub const BIN: &str = "fzf";
const ARGS: [&str; 5] = [
    "--expect=ctrl-y",
    "--no-preview",
    "--layout=reverse",
    "--prompt=url> ",
    "--header=enter: open in terminal-browser | ctrl-y: copy | esc: cancel",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Open in terminal-browser.
    Enter,
    /// Copy to the clipboard.
    CtrlY,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub key: Key,
    pub url: String,
}

#[derive(Debug, Error)]
pub enum FzfError {
    #[error("fzf was not found in PATH (https://github.com/junegunn/fzf)")]
    NotFound,
    /// Esc / ctrl-c, or no match: nothing was chosen.
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Failed(RunError),
    #[error("unexpected fzf output: {0}")]
    Parse(String),
}

impl From<RunError> for FzfError {
    fn from(e: RunError) -> Self {
        match e {
            RunError::NotFound(_) => Self::NotFound,
            RunError::Failed {
                code: Some(1 | 130),
                ..
            } => Self::Cancelled,
            other => Self::Failed(other),
        }
    }
}

/// One line per URL: a right-aligned index, two spaces, the URL.
#[must_use]
pub fn format_items(urls: &[String]) -> String {
    let mut out = String::new();
    for (i, url) in urls.iter().enumerate() {
        let _ = writeln!(out, "{:>3}  {url}", i + 1);
    }
    out
}

/// Parses fzf's stdout: the `--expect` key line, then the selected line minus its index.
pub fn parse_output(stdout: &str) -> Result<Selection, FzfError> {
    let mut lines = stdout.lines();
    let key = match lines.next().unwrap_or("").trim() {
        "" => Key::Enter,
        "ctrl-y" => Key::CtrlY,
        other => return Err(FzfError::Parse(format!("unknown key {other:?}"))),
    };
    let Some(line) = lines.find(|l| !l.trim().is_empty()) else {
        return Err(FzfError::Cancelled);
    };
    // "  3  https://..." → drop the index; URLs never contain whitespace.
    match line.trim_start().split_once(char::is_whitespace) {
        Some((_, url)) if !url.trim().is_empty() => Ok(Selection {
            key,
            url: url.trim().to_string(),
        }),
        _ => Err(FzfError::Parse(format!("no URL in {line:?}"))),
    }
}

/// Runs fzf over the URLs and returns the selection ([`FzfError::Cancelled`] when the user aborts).
pub fn run(urls: &[String]) -> Result<Selection, FzfError> {
    let out = process::run(
        Command::new(BIN).args(ARGS),
        Some(format_items(urls).as_bytes()),
        Stderr::Inherit,
    )?;
    parse_output(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_parses_round_trip() {
        let urls = vec![
            "https://a/b".to_string(),
            "http://localhost:3000".to_string(),
        ];
        assert_eq!(
            format_items(&urls),
            "  1  https://a/b\n  2  http://localhost:3000\n"
        );

        let sel = parse_output("\n  2  http://localhost:3000\n").unwrap();
        assert_eq!(
            (sel.key, sel.url.as_str()),
            (Key::Enter, "http://localhost:3000")
        );
        let sel = parse_output("ctrl-y\n  1  https://a/b\n").unwrap();
        assert_eq!((sel.key, sel.url.as_str()), (Key::CtrlY, "https://a/b"));
        assert!(matches!(parse_output("\n"), Err(FzfError::Cancelled)));
        assert!(matches!(
            parse_output("ctrl-x\n  1  x\n"),
            Err(FzfError::Parse(_))
        ));
    }

    #[test]
    fn esc_and_no_match_are_cancellations() {
        let cancelled = |code| {
            FzfError::from(RunError::Failed {
                cmd: "fzf".into(),
                code: Some(code),
                stderr: String::new(),
            })
        };
        assert!(matches!(cancelled(130), FzfError::Cancelled));
        assert!(matches!(cancelled(1), FzfError::Cancelled));
        assert!(matches!(cancelled(2), FzfError::Failed(_)));
    }
}

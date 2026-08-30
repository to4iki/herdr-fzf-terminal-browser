//! Running fzf and reading back what the user picked.
//!
//! fzf draws its full-screen UI on the pane's TTY (stderr is inherited), reads the candidates
//! from stdin, and prints the selection on stdout. With `--expect`, the first stdout line names
//! the key that ended the selection ("" for Enter).
//!
//! Do not add `--height`: fzf then reserves rows by printing newlines, which looks like the pane
//! scrolling before the list appears.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};

use thiserror::Error;

pub const BIN: &str = "fzf";

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
    #[error("fzf exited with {code:?}")]
    Failed { code: Option<i32> },
    #[error("unexpected fzf output: {0}")]
    Parse(String),
    #[error("could not run fzf: {0}")]
    Io(String),
}

#[must_use]
pub fn args() -> Vec<String> {
    [
        "--expect=ctrl-y",
        "--no-preview",
        "--layout=reverse",
        "--prompt=url> ",
        "--header=enter: open in terminal-browser | ctrl-y: copy | esc: cancel",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
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
///
/// # Errors
///
/// [`FzfError::Cancelled`] when nothing was selected; [`FzfError::Parse`] for an unknown key or
/// a line without a URL.
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

/// Runs fzf over the URLs and returns the selection.
///
/// # Errors
///
/// [`FzfError::Cancelled`] when the user aborts; other variants for a missing or failing fzf.
pub fn run(urls: &[String]) -> Result<Selection, FzfError> {
    let mut child = Command::new(BIN)
        .args(args())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FzfError::NotFound,
            _ => FzfError::Io(e.to_string()),
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        // fzf may exit early (esc); a broken pipe here is not worth reporting.
        let _ = stdin.write_all(format_items(urls).as_bytes());
    }
    let output = child
        .wait_with_output()
        .map_err(|e| FzfError::Io(e.to_string()))?;
    match output.status.code() {
        Some(0) => parse_output(&String::from_utf8_lossy(&output.stdout)),
        Some(1 | 130) => Err(FzfError::Cancelled),
        code => Err(FzfError::Failed { code }),
    }
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
}

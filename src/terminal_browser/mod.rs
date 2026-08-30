//! The terminal-browser side: Data types, the [`TerminalBrowser`] trait (the process seam), and
//! [`open_url`], the one decision this plugin makes about browsers.

pub mod cli;

use thiserror::Error;

use crate::context::SourcePane;

/// One running browser as reported by `terminal-browser ls --json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserInstance {
    /// `--browser` key (`<pid>-<n>`).
    pub key: String,
    /// True when the browser lives in the same herdr tab as the source pane.
    pub in_current_tab: bool,
}

#[derive(Debug, Error)]
pub enum TbError {
    #[error(
        "terminal-browser was not found in PATH. Install it from https://terminal-browser.com/ and run `terminal-browser setup`"
    )]
    NotFound,
    #[error("`terminal-browser {cmd}` failed{}: {stderr}", code.map(|c| format!(" (exit {c})")).unwrap_or_default())]
    CommandFailed {
        cmd: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("could not parse terminal-browser {what}: {message}")]
    Parse { what: &'static str, message: String },
    #[error("could not run terminal-browser: {0}")]
    Io(String),
    #[error("could not open the browser pane: {0}")]
    Herdr(#[from] crate::herdr::HerdrError),
}

/// Everything the plugin needs from terminal-browser.
///
/// Two invariants are built into this signature, and both fail silently if broken.
///
/// Every method takes the [`SourcePane`] because the implementation has to run terminal-browser
/// with `HERDR_PANE_ID` / `HERDR_TAB_ID` set to it: without them terminal-browser falls through to
/// a non-herdr terminal adapter and splits outside herdr entirely.
///
/// And there is deliberately no `new_tab` without a key: with no browser running and a TTY,
/// `terminal-browser new-tab` takes over the pane it was called from — the picker's own popup.
pub trait TerminalBrowser {
    /// `terminal-browser ls --json`, scoped to the source pane's tab by env.
    ///
    /// # Errors
    ///
    /// [`TbError`] when the binary is missing, fails, or prints unparseable JSON.
    fn list(&self, source: &SourcePane) -> Result<Vec<BrowserInstance>, TbError>;

    /// A new browser in a split to the right of the source pane, showing `url`.
    ///
    /// # Errors
    ///
    /// [`TbError`] when the binary is missing or fails.
    fn open_split(&self, source: &SourcePane, url: &str) -> Result<(), TbError>;

    /// `terminal-browser new-tab --browser <key> <url>`.
    ///
    /// # Errors
    ///
    /// [`TbError`] when the binary is missing or fails.
    fn new_tab(&self, source: &SourcePane, key: &str, url: &str) -> Result<(), TbError>;
}

/// Opens the URL next to the source pane: as a new tab of a browser already in that herdr tab,
/// or — when there is none — as a new browser split off the source pane.
///
/// `ls --json` exposes no start time, so with several browsers in the tab the highest key
/// (newest pid) wins; the choice is at least deterministic.
///
/// # Errors
///
/// Any terminal-browser failure.
pub fn open_url<T: TerminalBrowser>(tb: &T, source: &SourcePane, url: &str) -> Result<(), TbError> {
    let existing = tb.list(source)?;
    match existing
        .iter()
        .filter(|b| b.in_current_tab)
        .map(|b| b.key.as_str())
        .max()
    {
        Some(key) => tb.new_tab(source, key, url),
        None => tb.open_split(source, url),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        List,
        Split(String),
        NewTab(String, String),
    }

    #[derive(Default)]
    struct Fake {
        calls: RefCell<Vec<Call>>,
        listing: Vec<BrowserInstance>,
    }

    impl TerminalBrowser for Fake {
        fn list(&self, _: &SourcePane) -> Result<Vec<BrowserInstance>, TbError> {
            self.calls.borrow_mut().push(Call::List);
            Ok(self.listing.clone())
        }
        fn open_split(&self, _: &SourcePane, url: &str) -> Result<(), TbError> {
            self.calls.borrow_mut().push(Call::Split(url.into()));
            Ok(())
        }
        fn new_tab(&self, _: &SourcePane, key: &str, url: &str) -> Result<(), TbError> {
            self.calls
                .borrow_mut()
                .push(Call::NewTab(key.into(), url.into()));
            Ok(())
        }
    }

    fn browser(key: &str, in_tab: bool) -> BrowserInstance {
        BrowserInstance {
            key: key.into(),
            in_current_tab: in_tab,
        }
    }

    fn run(listing: Vec<BrowserInstance>) -> Vec<Call> {
        let tb = Fake {
            listing,
            ..Default::default()
        };
        let env: crate::Env = [
            ("FZF_TB_SOURCE_PANE", "w1:p1"),
            ("FZF_TB_SOURCE_TAB", "w1:t1"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let source = SourcePane::from_env(&env).unwrap();
        open_url(&tb, &source, "https://a").unwrap();
        tb.calls.into_inner()
    }

    #[test]
    fn no_browser_in_tab_means_split() {
        assert_eq!(
            run(vec![browser("9-1", false)]),
            vec![Call::List, Call::Split("https://a".into())]
        );
    }

    #[test]
    fn browser_in_tab_gets_a_new_tab_preferring_the_highest_key() {
        assert_eq!(
            run(vec![
                browser("100-1", true),
                browser("200-1", true),
                browser("300-1", false)
            ]),
            vec![Call::List, Call::NewTab("200-1".into(), "https://a".into())]
        );
    }
}

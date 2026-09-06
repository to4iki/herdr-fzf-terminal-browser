//! The terminal-browser side: Data types, the [`TerminalBrowser`] trait (the process seam), and
//! [`open_url`], the one decision this plugin makes about browsers.

pub mod cli;

use thiserror::Error;

use crate::context::SourcePane;
use crate::process::RunError;

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
    #[error("{0}")]
    Failed(RunError),
    #[error("could not parse `terminal-browser ls --json` output: {0}")]
    Parse(String),
    #[error("could not open the browser pane: {0}")]
    Herdr(#[from] crate::herdr::HerdrError),
}

impl From<RunError> for TbError {
    fn from(e: RunError) -> Self {
        match e {
            RunError::NotFound(_) => Self::NotFound,
            other => Self::Failed(other),
        }
    }
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
    /// Starts gathering the listing early so [`Self::list`] returns at once later. Optional.
    fn prefetch(&self, _source: &SourcePane) {}

    /// `terminal-browser ls --json`, scoped to the source pane's tab by env.
    fn list(&self, source: &SourcePane) -> Result<Vec<BrowserInstance>, TbError>;

    /// A new browser in a split to the right of the source pane, showing `url`.
    fn open_split(&self, source: &SourcePane, url: &str) -> Result<(), TbError>;

    /// `terminal-browser new-tab --browser <key> <url>`.
    fn new_tab(&self, source: &SourcePane, key: &str, url: &str) -> Result<(), TbError>;
}

/// Opens the URL next to the source pane: as a new tab of a browser already in that herdr tab,
/// or — when there is none — as a new browser split off the source pane.
///
/// `ls --json` exposes no start time, so with several browsers in the tab the one with the highest
/// pid (the most recently started) wins.
pub fn open_url<T: TerminalBrowser>(tb: &T, source: &SourcePane, url: &str) -> Result<(), TbError> {
    let existing = tb.list(source)?;
    match existing
        .iter()
        .filter(|b| b.in_current_tab)
        .max_by_key(|b| (key_order(&b.key), b.key.as_str()))
    {
        Some(browser) => tb.new_tab(source, &browser.key, url),
        None => tb.open_split(source, url),
    }
}

/// `<pid>-<n>` as numbers, so `10000-1` outranks `9999-1`.
fn key_order(key: &str) -> (u64, u64) {
    let (pid, n) = key.split_once('-').unwrap_or((key, "0"));
    (pid.parse().unwrap_or(0), n.parse().unwrap_or(0))
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
        open_url(&tb, &SourcePane::new("w1:p1", "w1:t1"), "https://a").unwrap();
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
    fn browser_in_tab_gets_a_new_tab_preferring_the_highest_pid() {
        assert_eq!(
            run(vec![
                browser("9999-1", true),
                browser("10000-1", true),
                browser("30000-1", false)
            ]),
            vec![
                Call::List,
                Call::NewTab("10000-1".into(), "https://a".into())
            ]
        );
    }
}

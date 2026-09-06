//! The *source pane*: the pane whose text is scanned and next to which terminal-browser opens.
//!
//! The `pick` action runs in the focused pane, so herdr gives it `HERDR_PANE_ID` / `HERDR_TAB_ID`.
//! It forwards those to the `picker` popup as `FZF_TB_SOURCE_PANE` / `FZF_TB_SOURCE_TAB`, because a
//! popup process gets no `HERDR_PANE_ID` of its own (herdr strips it, even via `--env`).
//! [`SourcePane::from_env`] accepts either pair, so the action, the popup, and a shell-run `open`
//! all resolve the same way.

use thiserror::Error;

use crate::{Env, env_var};

pub const ENV_SOURCE_PANE: &str = "FZF_TB_SOURCE_PANE";
pub const ENV_SOURCE_TAB: &str = "FZF_TB_SOURCE_TAB";

/// Both ids are required: terminal-browser works out "the pane I am in" from `HERDR_PANE_ID` /
/// `HERDR_TAB_ID`, and without them it falls through to a non-herdr terminal adapter. Every
/// terminal-browser call takes a `SourcePane`, so that cannot happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePane {
    pane_id: String,
    tab_id: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "could not determine the source pane ({ENV_SOURCE_PANE}/{ENV_SOURCE_TAB} or HERDR_PANE_ID/HERDR_TAB_ID)"
)]
pub struct MissingSource;

impl SourcePane {
    /// Reads the source pane, trying `FZF_TB_SOURCE_PANE`/`_TAB` (set by the action for the popup)
    /// then `HERDR_PANE_ID`/`HERDR_TAB_ID` (injected into actions and shell panes).
    pub fn from_env(env: &Env) -> Result<Self, MissingSource> {
        [
            (ENV_SOURCE_PANE, ENV_SOURCE_TAB),
            ("HERDR_PANE_ID", "HERDR_TAB_ID"),
        ]
        .into_iter()
        .find_map(|(pane_key, tab_key)| {
            Some(Self {
                pane_id: env_var(env, pane_key)?.to_string(),
                tab_id: env_var(env, tab_key)?.to_string(),
            })
        })
        .ok_or(MissingSource)
    }

    #[cfg(test)]
    pub(crate) fn new(pane_id: &str, tab_id: &str) -> Self {
        Self {
            pane_id: pane_id.into(),
            tab_id: tab_id.into(),
        }
    }

    #[must_use]
    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    #[must_use]
    pub fn tab_id(&self) -> &str {
        &self.tab_id
    }

    /// The `--env` pairs the action passes to the popup so it can recover this pane.
    #[must_use]
    pub fn popup_env(&self) -> [(&'static str, &str); 2] {
        [
            (ENV_SOURCE_PANE, &self.pane_id),
            (ENV_SOURCE_TAB, &self.tab_id),
        ]
    }

    /// The variables that make terminal-browser treat this pane as "the pane I am in".
    #[must_use]
    pub fn herdr_env(&self) -> [(&'static str, &str); 2] {
        [
            ("HERDR_PANE_ID", &self.pane_id),
            ("HERDR_TAB_ID", &self.tab_id),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Env {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn source_env_wins_then_herdr_env() {
        let s = SourcePane::from_env(&env(&[
            (ENV_SOURCE_PANE, "w1:p9"),
            (ENV_SOURCE_TAB, "w1:t9"),
            ("HERDR_PANE_ID", "w1:p1"),
            ("HERDR_TAB_ID", "w1:t1"),
        ]))
        .unwrap();
        assert_eq!(s, SourcePane::new("w1:p9", "w1:t9"));
        assert_eq!(
            s.herdr_env(),
            [("HERDR_PANE_ID", "w1:p9"), ("HERDR_TAB_ID", "w1:t9")]
        );

        let s = SourcePane::from_env(&env(&[
            ("HERDR_PANE_ID", "w1:p1"),
            ("HERDR_TAB_ID", "w1:t1"),
        ]))
        .unwrap();
        assert_eq!(s, SourcePane::new("w1:p1", "w1:t1"));
    }

    #[test]
    fn half_a_pair_or_nothing_is_missing() {
        assert_eq!(
            SourcePane::from_env(&env(&[("HERDR_PANE_ID", "w1:p1"), ("HERDR_TAB_ID", " ")])),
            Err(MissingSource)
        );
        assert_eq!(SourcePane::from_env(&Env::new()), Err(MissingSource));
    }
}

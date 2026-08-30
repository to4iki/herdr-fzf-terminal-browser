//! The *source pane*: the pane whose text is scanned and next to which terminal-browser opens.
//!
//! The `pick` action runs in the focused pane, so herdr gives it `HERDR_PANE_ID` / `HERDR_TAB_ID`.
//! It forwards those to the `picker` popup as `FZF_TB_SOURCE_PANE` / `FZF_TB_SOURCE_TAB`, because a
//! popup process gets no `HERDR_PANE_ID` of its own (herdr strips it, even via `--env`).
//! [`SourcePane::from_env`] accepts either pair, so the action, the popup, and a shell-run `open`
//! all resolve the same way.

use thiserror::Error;

use crate::Env;

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
    ///
    /// # Errors
    ///
    /// [`MissingSource`] when neither pair is fully set.
    pub fn from_env(env: &Env) -> Result<Self, MissingSource> {
        let get = |key: &str| {
            env.get(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        for (pane_key, tab_key) in [
            (ENV_SOURCE_PANE, ENV_SOURCE_TAB),
            ("HERDR_PANE_ID", "HERDR_TAB_ID"),
        ] {
            if let (Some(pane_id), Some(tab_id)) = (get(pane_key), get(tab_key)) {
                return Ok(Self { pane_id, tab_id });
            }
        }
        Err(MissingSource)
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
    pub fn popup_env(&self) -> Vec<(String, String)> {
        vec![
            (ENV_SOURCE_PANE.to_string(), self.pane_id.clone()),
            (ENV_SOURCE_TAB.to_string(), self.tab_id.clone()),
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
        assert_eq!((s.pane_id(), s.tab_id()), ("w1:p9", "w1:t9"));
        assert_eq!(
            s.popup_env(),
            vec![
                (ENV_SOURCE_PANE.to_string(), "w1:p9".to_string()),
                (ENV_SOURCE_TAB.to_string(), "w1:t9".to_string()),
            ]
        );

        let s = SourcePane::from_env(&env(&[
            ("HERDR_PANE_ID", "w1:p1"),
            ("HERDR_TAB_ID", "w1:t1"),
        ]))
        .unwrap();
        assert_eq!((s.pane_id(), s.tab_id()), ("w1:p1", "w1:t1"));
    }

    #[test]
    fn half_a_pair_or_nothing_is_missing() {
        assert_eq!(
            SourcePane::from_env(&env(&[("HERDR_PANE_ID", "w1:p1")])),
            Err(MissingSource)
        );
        assert_eq!(SourcePane::from_env(&Env::new()), Err(MissingSource));
    }
}

//! herdr plugin: pick a URL from the current pane with fzf and open it in terminal-browser.
//!
//! Pure logic ([`extract`], [`context`]) is kept apart from process spawning ([`herdr`], [`fzf`],
//! [`opener`], [`terminal_browser`]). The [`commands`] modules are the four subcommands herdr and
//! the shell call.

pub mod cli;
pub mod commands;
pub mod context;
pub mod extract;
pub mod fzf;
pub mod herdr;
pub mod opener;
pub mod terminal_browser;
pub mod ui;

/// Snapshot of the process environment, taken once in `main`.
pub type Env = std::collections::BTreeMap<String, String>;

/// This plugin's id, as declared in `herdr-plugin.toml` (a test keeps them equal). Used when
/// `HERDR_PLUGIN_ID` is not in the environment, i.e. when `open` runs from a shell.
pub const PLUGIN_ID: &str = "to4iki.fzf-terminal-browser";

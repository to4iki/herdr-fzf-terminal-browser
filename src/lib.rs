//! herdr plugin: pick a URL from the current pane with fzf and open it in terminal-browser.
//!
//! Pure logic ([`extract`], [`context`]) is kept apart from process spawning ([`herdr`], [`fzf`],
//! [`opener`], [`terminal_browser`]). [`commands`] holds one module per subcommand.

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

/// Looks a program up in `PATH`.
#[must_use]
pub fn find_in_path(env: &Env, name: &str) -> Option<std::path::PathBuf> {
    let path = env.get("PATH")?;
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// This plugin's id, as declared in `herdr-plugin.toml` (a test keeps them equal). Used when
/// `HERDR_PLUGIN_ID` is not in the environment, i.e. when `open` runs from a shell.
pub const PLUGIN_ID: &str = "to4iki.fzf-terminal-browser";

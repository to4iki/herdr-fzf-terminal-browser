//! herdr plugin: pick a URL from the current pane with fzf and open it in terminal-browser.
//!
//! Pure logic ([`extract`], [`context`]) is kept apart from process spawning ([`process`] and the
//! modules built on it: [`herdr`], [`fzf`], [`clipboard`], [`terminal_browser`]). [`commands`]
//! holds one module per subcommand.

pub mod cli;
pub mod clipboard;
pub mod commands;
pub mod context;
pub mod extract;
pub mod fzf;
pub mod herdr;
pub mod process;
pub mod terminal_browser;
pub mod ui;

/// Snapshot of the process environment, taken once in `main`.
pub type Env = std::collections::BTreeMap<String, String>;

/// An environment variable, trimmed; `None` when unset or blank.
#[must_use]
pub fn env_var<'a>(env: &'a Env, key: &str) -> Option<&'a str> {
    env.get(key).map(|v| v.trim()).filter(|v| !v.is_empty())
}

/// Looks a program up in `PATH`.
#[must_use]
pub fn find_in_path(env: &Env, name: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(env.get("PATH")?)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

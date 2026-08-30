//! `pick`: the herdr action behind the keybinding. It has no TTY, so all it does is open the
//! `picker` popup, handing it the pane the key was pressed in.

use std::process::ExitCode;

use crate::Env;
use crate::commands::PICKER_ENTRYPOINT;
use crate::context::SourcePane;
use crate::herdr::{self, PluginPaneOpen};
use crate::ui::NAME;

#[must_use]
pub fn run(env: &Env) -> ExitCode {
    match open_picker(env) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            // Only visible via `herdr plugin log list --plugin <id>`, so be explicit.
            eprintln!("{NAME}: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn open_picker(env: &Env) -> Result<(), String> {
    let source = SourcePane::from_env(env).map_err(|e| e.to_string())?;
    herdr::plugin_pane_open(
        env,
        &PluginPaneOpen {
            entrypoint: PICKER_ENTRYPOINT,
            env: source.popup_env(),
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())
}

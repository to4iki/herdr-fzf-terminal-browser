//! `pick`: the herdr action behind the keybinding. It has no TTY, so it cannot show fzf itself:
//! it checks that the pane shows at least one URL, then opens the `picker` popup for that pane.
//! With nothing to pick it only raises a herdr notification, so no empty popup appears.

use std::error::Error;
use std::process::ExitCode;

use crate::Env;
use crate::context::SourcePane;
use crate::extract::extract_urls;
use crate::herdr::{self, PICKER_ENTRYPOINT, PluginPaneOpen};
use crate::ui::{self, NAME, NO_URLS};

#[must_use]
pub fn run(env: &Env) -> ExitCode {
    // Errors are only visible via `herdr plugin log list --plugin <id>`.
    ui::report(pick(env))
}

fn pick(env: &Env) -> Result<(), Box<dyn Error>> {
    let source = SourcePane::from_env(env)?;
    if extract_urls(&herdr::read_screen(env, source.pane_id())?).is_empty() {
        // Best effort: the notification is feedback, not something to fail over.
        let _ = herdr::notify(env, NO_URLS);
        eprintln!("{NAME}: {NO_URLS}");
        return Ok(());
    }
    // The popup reads the screen again itself; it is cheap and keeps `picker` self-contained.
    herdr::plugin_pane_open(
        env,
        &PluginPaneOpen {
            entrypoint: PICKER_ENTRYPOINT,
            env: &source.popup_env(),
            ..Default::default()
        },
    )?;
    Ok(())
}

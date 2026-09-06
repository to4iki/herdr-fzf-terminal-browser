//! Messages for the user. The popup closes the moment the process exits, so anything the user
//! must read there is followed by a wait for Enter on `/dev/tty`.

use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

pub const NAME: &str = env!("CARGO_PKG_NAME");
/// Logged by the action, and shown in the popup, when the pane has no URL to pick.
pub const NO_URLS: &str = "No URLs on screen";

/// Exit code for a command that reports errors on stderr (actions and shell use).
#[must_use]
pub fn report(result: Result<(), Box<dyn Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{NAME}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Error message, then wait so it can be read before the popup disappears.
pub fn fail_and_wait(msg: &str) {
    eprintln!("{NAME}: {msg}");
    wait_for_enter();
}

/// Informational message, then wait.
pub fn info_and_wait(msg: &str) {
    eprintln!("{msg}");
    wait_for_enter();
}

fn wait_for_enter() {
    let Ok(mut tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") else {
        return;
    };
    let _ = write!(tty, "\nPress Enter to close...");
    let _ = tty.flush();
    let mut line = String::new();
    let _ = BufReader::new(&tty).read_line(&mut line);
}

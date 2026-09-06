//! Messages for the popup. The popup closes the moment the process exits, so anything the user
//! must read is followed by a wait for Enter on `/dev/tty`.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

pub const NAME: &str = "herdr-fzf-terminal-browser";
/// Shown (as a herdr notification, or in the popup) when the pane has no URL to pick.
pub const NO_URLS: &str = "No URLs on screen";

/// Prints "Press Enter to close" and waits, when a controlling terminal exists.
pub fn wait_for_enter() {
    let Ok(mut tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") else {
        return;
    };
    let _ = write!(tty, "\nPress Enter to close...");
    let _ = tty.flush();
    let mut line = String::new();
    let _ = BufReader::new(&tty).read_line(&mut line);
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

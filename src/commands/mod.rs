//! The subcommands: two herdr entrypoints (`pick`, `picker`), the split pane herdr runs for the
//! browser (`browser`), and two that are useful from a shell (`open`, `extract`).

pub mod browser;
pub mod extract;
pub mod open;
pub mod pick;
pub mod picker;

pub const PICKER_ENTRYPOINT: &str = "picker";

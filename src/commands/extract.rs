//! `extract`: stdin in, one URL per line out (newest first). Handy for scripting and for checking
//! what the picker would list: `herdr pane read <pane> --source visible | ... extract`.

use std::io::Read as _;
use std::process::ExitCode;

use crate::extract::extract_urls;
use crate::ui::NAME;

#[must_use]
pub fn run() -> ExitCode {
    let mut text = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut text) {
        eprintln!("{NAME}: could not read stdin: {e}");
        return ExitCode::FAILURE;
    }
    for url in extract_urls(&text) {
        println!("{url}");
    }
    ExitCode::SUCCESS
}

use std::process::ExitCode;

use clap::Parser;

use herdr_fzf_terminal_browser::cli::{Cli, Command};
use herdr_fzf_terminal_browser::{Env, commands};

fn main() -> ExitCode {
    let env: Env = std::env::vars().collect();
    match Cli::parse().command {
        Command::Pick => commands::pick::run(&env),
        Command::Picker => commands::picker::run(&env),
        Command::Browser => commands::browser::run(&env),
        Command::Open { url } => commands::open::run(&env, &url),
        Command::Extract => commands::extract::run(),
    }
}

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "herdr-fzf-terminal-browser",
    version,
    about = "herdr plugin: pick a URL from the current pane with fzf and open it in terminal-browser"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum Command {
    /// herdr action entrypoint: open the picker popup for the focused pane (no TTY needed)
    Pick,
    /// herdr popup entrypoint: pick a URL from the source pane with fzf and open it
    Picker,
    /// herdr split-pane entrypoint: run `terminal-browser open $FZF_TB_URL` (used internally)
    Browser,
    /// Open a URL in terminal-browser next to the source pane
    Open { url: String },
    /// Read text on stdin and print the URLs it contains, newest first
    Extract,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_subcommands() {
        use clap::Parser;
        assert_eq!(
            Cli::try_parse_from(["x", "open", "https://a"])
                .unwrap()
                .command,
            Command::Open {
                url: "https://a".into()
            }
        );
        assert_eq!(
            Cli::try_parse_from(["x", "pick"]).unwrap().command,
            Command::Pick
        );
        assert!(
            Cli::try_parse_from(["x", "open"]).is_err(),
            "open needs a url"
        );
        assert!(
            Cli::try_parse_from(["x"]).is_err(),
            "a subcommand is required"
        );
    }
}

# Project Guide

## Overview

herdr-fzf-terminal-browser is a [herdr](https://herdr.dev) plugin: pick URLs printed in the
current pane with fzf and open them in [terminal-browser](https://terminal-browser.com/) — as a
split pane next to the source pane, or as a new tab when a browser is already open in the same
herdr tab. Only the concept comes from tmux-fzf-url; the implementation is original.

## Tech Stack

- Rust (edition 2024), single crate, lib + bin (`herdr-fzf-terminal-browser`)
- clap 4 (derive), serde / serde_json, regex, thiserror (kept lean: users build from source at install time)
- External processes: `herdr` (via `HERDR_BIN_PATH`), `terminal-browser`, `fzf`
- No configuration in the first release; candidates are listed in README ("Not yet / ideas for later")
- Release: release-plz (Conventional Commits drive the CHANGELOG); no crates.io publish

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo build --release && herdr plugin link .   # local E2E inside herdr; bind prefix+f to to4iki.fzf-terminal-browser.pick
```

## Coding Guide

- Keep `extract.rs` and `context.rs` pure. Every external program goes through `process::run`;
  terminal-browser additionally sits behind the `TerminalBrowser` trait so `open_url` is tested
  with an in-memory fake, and arg builders / parsers stay unit-testable.
- Spawn with argv arrays (`Command::args`), never a shell string.
- Keep the surface small: five subcommands, no config. Add features only when asked.
- Specify at least the major version when adding a crate.
- Conventional Commits (`feat:`, `fix:`, `feat(extract):` ...).

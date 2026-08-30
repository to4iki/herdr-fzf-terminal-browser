//! The CLI surface, through the real binary.

use assert_cmd::Command;

#[test]
fn extract_reads_stdin_and_prints_urls_newest_first() {
    Command::cargo_bin("herdr-fzf-terminal-browser")
        .unwrap()
        .arg("extract")
        .write_stdin("see https://a/1 and (https://b/2).\nlocalhost:5173\n")
        .assert()
        .success()
        .stdout("http://localhost:5173\nhttps://b/2\nhttps://a/1\n");
}

#[test]
fn open_without_a_source_pane_fails_before_touching_terminal_browser() {
    Command::cargo_bin("herdr-fzf-terminal-browser")
        .unwrap()
        .arg("open")
        .arg("https://a")
        .env_remove("HERDR_PANE_ID")
        .env_remove("HERDR_TAB_ID")
        .env_remove("FZF_TB_SOURCE_PANE")
        .env_remove("FZF_TB_SOURCE_TAB")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "could not determine the source pane",
        ));
}

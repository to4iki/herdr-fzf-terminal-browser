//! `herdr-plugin.toml` must agree with the crate: same version (or the marketplace and
//! `herdr plugin list` show a stale number after a release), same plugin id, and pane entrypoints
//! that match the names the code opens.

use herdr_fzf_terminal_browser::herdr;

#[test]
fn plugin_manifest_matches_the_crate() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/herdr-plugin.toml"))
            .expect("herdr-plugin.toml is readable");
    let table: toml::Table = manifest.parse().expect("herdr-plugin.toml is valid TOML");
    let plugin_version = table["version"].as_str().expect("version is a string");
    assert_eq!(
        plugin_version,
        env!("CARGO_PKG_VERSION"),
        "herdr-plugin.toml version must equal Cargo.toml version (run scripts/sync-plugin-version.sh)"
    );
    assert_eq!(
        table["id"].as_str().expect("id is a string"),
        herdr::PLUGIN_ID,
        "herdr::PLUGIN_ID must equal the manifest id"
    );
    let panes: Vec<&str> = table["panes"]
        .as_array()
        .expect("[[panes]]")
        .iter()
        .map(|p| p["id"].as_str().expect("pane id"))
        .collect();
    for entrypoint in [herdr::PICKER_ENTRYPOINT, herdr::BROWSER_ENTRYPOINT] {
        assert!(
            panes.contains(&entrypoint),
            "manifest lacks the `{entrypoint}` pane"
        );
    }
}

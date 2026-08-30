//! `herdr-plugin.toml` must advertise the same version as `Cargo.toml`, or the marketplace and
//! `herdr plugin list` show a stale number after a release.

#[test]
fn plugin_manifest_version_matches_cargo() {
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
        herdr_fzf_terminal_browser::PLUGIN_ID,
        "PLUGIN_ID in lib.rs must equal the manifest id"
    );
}

#!/bin/sh
# Copies the version from Cargo.toml into herdr-plugin.toml.
# release-plz only bumps Cargo files; the herdr manifest must follow so the marketplace and
# `herdr plugin list` show the released version. Only the first `version = "..."` line is touched,
# so `min_herdr_version` is left alone.
set -eu
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "sync-plugin-version: no version in Cargo.toml" >&2; exit 1; }
perl -pi -e "s/^version = \"[^\"]*\"/version = \"$version\"/" herdr-plugin.toml
echo "herdr-plugin.toml version = $version"

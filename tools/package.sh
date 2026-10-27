#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cargo build --manifest-path "$root/Cargo.toml" --release --locked -p nyx-archive
mkdir -p "$root/dist"
stage=$(mktemp -d "$root/dist/package.XXXXXX")
trap 'rm -rf -- "$stage"' EXIT
install -Dm755 "${CARGO_TARGET_DIR:-$root/target}/release/nyx-archive" "$stage/bin/nyx-archive"
install -Dm644 "$root/apps/archive/data/io.github.design_nexus.nyx.Archive.desktop"  "$stage/share/applications/io.github.design_nexus.nyx.Archive.desktop"
install -Dm644 "$root/apps/archive/data/io.github.design_nexus.nyx.Archive.svg"  "$stage/share/icons/hicolor/scalable/apps/io.github.design_nexus.nyx.Archive.svg"
cp "$root/LICENSE" "$stage/"
tar -C "$stage" -czf "$root/dist/archive-$(uname -m).tar.gz" .
sha256sum "$root/dist/archive-$(uname -m).tar.gz"

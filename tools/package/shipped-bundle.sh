#!/usr/bin/env bash
# The game data the installers ship (Phase 5, P4; docs/phase-5/data-bundles.md,
# "Shipped bundle"). CI cannot extract game data (ADR-005): the maintainer
# builds the bundle, uploads it as a zip to a release, and commits
# release/bundle.json naming it:
#
#   { "url": "https://github.com/.../bundle-poe1-<build>-schema<n>.zip",
#     "sha256": "<the zip's SHA-256>" }
#
# This script downloads the zip, checks its hash, unpacks it, runs
# bundle-check, and packs it signed with the data key
# ($LOOM_KEYS_DIR/data.secret, tools/package/data-key.sh; deleted here
# whatever happens, since no later build step needs it) into
# crates/loom-tauri/shipped/, which tauri.shipped.conf.json puts in the
# installers' resources (bundles/). It exports SHIPPED_CONFIG for the build
# steps. Without release/bundle.json nothing is shipped; a tag's run then
# fails (a release ships its data).
set -euo pipefail
out="${GITHUB_ENV:-/dev/null}"
secret="${LOOM_KEYS_DIR:?run tools/package/data-key.sh first}/data.secret"
trap 'rm -f "$secret"' EXIT
if [ ! -f release/bundle.json ]; then
  case "${GITHUB_REF:-}" in
    refs/tags/*) echo "a tagged release needs release/bundle.json (the shipped game data)" >&2; exit 1 ;;
  esac
  echo "SHIPPED_CONFIG=" >> "$out"
  echo "shipped bundle: none (no release/bundle.json)"
  exit 0
fi
url="$(jq -r .url release/bundle.json)"
sha="$(jq -r .sha256 release/bundle.json)"
work="${RUNNER_TEMP:-$(mktemp -d)}/shipped"
rm -rf "$work"
mkdir -p "$work/src"
curl -fsSL --retry 3 -o "$work/bundle.zip" "$url"
echo "$sha  $work/bundle.zip" | sha256sum -c -
python3 -m zipfile -e "$work/bundle.zip" "$work/src" 2>/dev/null || python -m zipfile -e "$work/bundle.zip" "$work/src"
# The zip holds the bundle folder, or its files at the top.
dir="$(dirname "$(find "$work/src" -name bundle.json -print -quit)")"
cargo run -q --release --locked -p loom-parity-cli -- bundle-check "$dir"
mkdir -p crates/loom-tauri/shipped
cargo run -q --locked -p loom-release -- pack "$dir" --secret "$secret" \
  --out crates/loom-tauri/shipped/game-data.loombundle
echo "SHIPPED_CONFIG=--config crates/loom-tauri/tauri.shipped.conf.json" >> "$out"
echo "shipped bundle: crates/loom-tauri/shipped/game-data.loombundle"

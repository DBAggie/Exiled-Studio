#!/usr/bin/env bash
# The game data the installers ship (Phase 5, P4; docs/phase-5/data-bundles.md,
# "Shipped bundle"). CI cannot extract game data (ADR-005): the maintainer
# builds the bundle, uploads it as a zip to a release, and commits
# release/bundle.json naming it:
#
#   { "url": "https://github.com/.../bundle-poe1-<build>-schema<n>.zip",
#     "sha256": "<the zip's SHA-256>" }
#
# The zip holds GGG's tree sprites unsigned, which ADR-005 Amendment 2 lets
# ship only inside signed bundles, so it is kept as an asset of a draft
# release (visible only to the repository's members) and `url` is that
# asset's API URL (https://api.github.com/repos/<o>/<r>/releases/assets/<id>),
# fetched with GH_TOKEN (the release jobs' token, contents: write).
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
case "$url" in
  https://api.github.com/*)
    # curl drops the Authorization header on the redirect to the storage
    # host, as that host requires.
    curl -fsSL --retry 3 -H "Authorization: Bearer ${GH_TOKEN:?a draft release asset needs GH_TOKEN}"       -H "Accept: application/octet-stream" -o "$work/bundle.zip" "$url" ;;
  *) curl -fsSL --retry 3 -o "$work/bundle.zip" "$url" ;;
esac
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

#!/usr/bin/env bash
# The data feed a release publishes (Phase 5, P4): data-feed.json listing
# the shipped pack, signed with LOOM_DATA_KEY. Its sequence is the release
# workflow's run number, which only grows (no feed rollback).
#
#   data-feed.sh <dir with the pack> <version> <owner/repo> <sequence>
set -euo pipefail
dir="$1"
version="$2"
repo="$3"
sequence="$4"
packs=("$dir"/*.loombundle)
[ -e "${packs[0]}" ] || { echo "data feed: no pack, none published"; exit 0; }
cargo run -q --locked -p loom-release -- feed --sequence "$sequence" \
  --base-url "https://github.com/$repo/releases/download/v$version" \
  --out "$dir/data-feed.json" "${packs[@]}"
cargo run -q --locked -p loom-release -- sign "$dir/data-feed.json" \
  --secret-env LOOM_DATA_KEY --out "$dir/data-feed.json.sig"
echo "data feed: sequence $sequence, ${#packs[@]} pack(s)"

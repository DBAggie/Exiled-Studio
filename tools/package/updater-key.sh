#!/usr/bin/env bash
# The release workflow's update key (Phase 5, P3; docs/release/signing.md).
# Runs before anything is built: the application compiles
# release/updater.pub in, and `tauri build` with tauri.updater.conf.json
# signs each update package with TAURI_SIGNING_PRIVATE_KEY.
#
# - LOOM_UPDATER_KEY set (the `release` environment's secret, with
#   LOOM_UPDATER_KEY_PASSWORD): release/updater.pub must hold its public
#   half (committed by the maintainer), or the run fails.
# - Absent: a throwaway key is made with `tauri signer generate`, its public
#   half written to the run's copy of release/updater.pub (never committed).
#   A tag's run refuses throwaway mode: installed copies would never accept
#   the next release.
set -euo pipefail
tauri=ui/node_modules/.bin/tauri
out="${GITHUB_ENV:-/dev/null}"
if [ -n "${LOOM_UPDATER_KEY:-}" ]; then
  [ -s release/updater.pub ] || {
    echo "release/updater.pub is empty: commit the update key's public half first" >&2
    exit 1
  }
  {
    echo "TAURI_SIGNING_PRIVATE_KEY<<LOOM_EOF"
    echo "$LOOM_UPDATER_KEY"
    echo "LOOM_EOF"
    echo "TAURI_SIGNING_PRIVATE_KEY_PASSWORD=${LOOM_UPDATER_KEY_PASSWORD:-}"
  } >> "$out"
  echo "update key: release (release/updater.pub)"
else
  case "${GITHUB_REF:-}" in
    refs/tags/*) echo "a tagged release needs the LOOM_UPDATER_KEY secret" >&2; exit 1 ;;
  esac
  dir="${RUNNER_TEMP:-$(mktemp -d)}/updater-key"
  mkdir -p "$dir"
  "$tauri" signer generate --ci -p "" -f -w "$dir/updater.key" >/dev/null
  tr -d '\r\n' < "$dir/updater.key.pub" > release/updater.pub
  key="$(tr -d '\r\n' < "$dir/updater.key")"
  echo "::add-mask::$key"
  {
    echo "TAURI_SIGNING_PRIVATE_KEY=$key"
    echo "TAURI_SIGNING_PRIVATE_KEY_PASSWORD="
  } >> "$out"
  echo "update key: throwaway (this run's builds only)"
fi

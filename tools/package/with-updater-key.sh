#!/usr/bin/env bash
# Runs one command (a `tauri build`) with the update key in its
# environment, read from $LOOM_KEYS_DIR (tools/package/updater-key.sh), so
# no other step of the release workflow carries it (Phase 5, P3;
# docs/release/signing.md).
#
#   with-updater-key.sh <command> [<args>...]
set -euo pipefail
keys="${LOOM_KEYS_DIR:?run tools/package/updater-key.sh first}"
TAURI_SIGNING_PRIVATE_KEY="$(cat "$keys/updater.key")" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat "$keys/updater.password")" \
  exec "$@"

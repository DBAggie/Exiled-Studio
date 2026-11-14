#!/usr/bin/env bash
# Runs the real-app workflow (ui/e2e/workflow.mjs) against the built e2e
# app: starts tauri-driver, then the script. From the repository root, in
# CI's e2e job (Linux under xvfb-run, Windows with msedgedriver):
#
#   ui/e2e/run.sh <app binary> [<native driver>]
set -euo pipefail

app="$1"
native="${2:-}"
root="$(pwd)"
work="$(mktemp -d)"
cp tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml "$work/import.xml"

export LOOM_E2E_DIR="$work"
export LOOM_DATA_DIR="$work/data"
export LOOM_BUNDLE_DIR="$root/crates/loom-data/tests/fixtures/bundle-poe1"

if [ -n "$native" ]; then
  tauri-driver --native-driver "$native" &
else
  tauri-driver &
fi
driver=$!
trap 'kill "$driver" 2>/dev/null || true' EXIT

# Wait for the driver to listen.
for _ in $(seq 1 50); do
  if node -e "fetch('http://127.0.0.1:4444/status').then(r => process.exit(r.ok ? 0 : 1), () => process.exit(1))"; then
    break
  fi
  sleep 0.2
done

node ui/e2e/workflow.mjs "$app" "$work"

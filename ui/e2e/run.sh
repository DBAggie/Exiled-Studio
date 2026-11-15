#!/usr/bin/env bash
# Runs the real-app workflow (ui/e2e/workflow.mjs) against the built e2e
# app: starts tauri-driver, then the script. From the repository root, in
# CI's e2e job on Linux, under xvfb-run (Windows uses run.ps1):
#
#   ui/e2e/run.sh <app binary> [<native driver>]
set -euo pipefail

app="$1"
native="${2:-}"
root="$(pwd)"
work="$(mktemp -d)"
# On Windows (Git Bash) the app and the drivers are native programs: give
# them Windows paths, and the app's path absolute.
if command -v cygpath >/dev/null 2>&1; then
  root="$(cygpath -m "$root")"
  work="$(cygpath -m "$work")"
fi
case "$app" in
  /* | [A-Za-z]:*) ;;
  *) app="$root/$app" ;;
esac
# The synthetic build as the synthetic tree bundle's Test Brute (passive
# tree plan T1a), so the workflow can allocate on the tree.
sed -e 's/className="Scion"/className="Test Brute"/' \
  -e 's/classId="0" ascendClassId="0" nodes=""/classId="1" ascendClassId="0" nodes="1000"/' \
  tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml >"$work/import.xml"

# A pack of the synthetic tree bundle, signed with the public test key
# (Phase 5, P4: the e2e installs it).
cargo run -q --locked -p loom-release -- pack crates/loom-data/tests/fixtures/bundle-poe1-tree --test-key --out "$work/bundle.loombundle"

export LOOM_E2E_DIR="$work"
export LOOM_DATA_DIR="$work/data"
export LOOM_BUNDLE_DIR="$root/crates/loom-data/tests/fixtures/bundle-poe1-tree"

if [ -n "$native" ] && [ ! -f "$native" ]; then
  echo "no native driver at $native" >&2
  exit 1
fi
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

#!/usr/bin/env bash
# Starts a package once and checks its build info (Phase 5, P1;
# docs/phase-5/packaging.md). The application, started with
# `--smoke <file>`, writes the file when its first screen has come up and
# quits (exit 0), or after 90 seconds without it (exit 2). Run it under a
# display (xvfb-run on CI).
#
#   smoke.sh <AppImage or installed executable> <artifact> [<version>]
#
# <artifact> is what the build must report: app-image or deb. An AppImage
# runs extracted (APPIMAGE_EXTRACT_AND_RUN: runners have no FUSE). The
# build must report LOOM_CHANNEL's channel (stable when unset;
# docs/release/channels.md).
set -euo pipefail
package="$1"
artifact="$2"
version="${3:-}"

work="$(mktemp -d)"
out="$work/smoke.json"
# A throwaway data folder, never the user's application data.
export LOOM_DATA_DIR="$work/data"
case "$package" in
  *.AppImage)
    chmod +x "$package"
    export APPIMAGE_EXTRACT_AND_RUN=1
    ;;
esac
set +e
timeout 150 "$package" --smoke "$out"
code=$?
set -e
[ -f "$out" ] || { echo "no smoke file (exit $code)" >&2; exit 1; }
cat "$out"
fail=0
check() { jq -e "$1" "$out" >/dev/null || { echo "failed: $1" >&2; fail=1; }; }
[ "$code" -eq 0 ] || { echo "exit code $code" >&2; fail=1; }
check '.ready == true'
check '.e2e == false'
check '.debug == false'
check ".artifact == \"$artifact\""
check '.portable == false'
check ".channel == \"${LOOM_CHANNEL:-stable}\""
[ -z "$version" ] || check ".version == \"$version\""
rm -rf "$work"
[ "$fail" -eq 0 ] || exit 1
echo "smoke ok: $artifact"

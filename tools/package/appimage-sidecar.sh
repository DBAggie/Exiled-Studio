#!/usr/bin/env bash
# Checks that an AppImage carries the knowledge MCP server (ADR-018;
# tools/package/knowledge-sidecar.sh) and that it runs: extracts the image
# into a temporary folder and asks the server for its version.
#
#   appimage-sidecar.sh <AppImage> [<version>]
set -euo pipefail
image="$(realpath "$1")"
version="${2:-}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
chmod +x "$image"
(cd "$work" && "$image" --appimage-extract >/dev/null)
bin="$work/squashfs-root/usr/bin/exiled-studio-knowledge-mcp"
[ -x "$bin" ] || { echo "no knowledge MCP in $image" >&2; exit 1; }
got="$("$bin" --version)"
echo "in the AppImage: $got"
if [ -n "$version" ] && [ "$got" != "exiled-studio-knowledge-mcp $version" ]; then
  echo "knowledge MCP version: $got, not $version" >&2
  exit 1
fi

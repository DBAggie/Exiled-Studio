#!/usr/bin/env bash
# Installs the .deb, starts the installed application once under Xvfb
# (smoke.sh), and removes the package (Phase 5, P1; release workflow).
#
#   install-deb.sh <package.deb> [<version>]
set -euo pipefail
deb="$1"
version="${2:-}"
here="$(cd "$(dirname "$0")" && pwd)"

name="$(dpkg-deb -f "$deb" Package)"
sudo apt-get install -y --no-install-recommends "$(realpath "$deb")"
mcp=/usr/bin/exiled-studio-knowledge-mcp
binary="$(dpkg -L "$name" | grep -E '^/usr/bin/[^/]+$' | grep -vx "$mcp" | head -n 1)"
[ -n "$binary" ] || { echo "no executable in $name" >&2; exit 1; }
echo "installed: $binary"
# The knowledge MCP server (ADR-018), beside the application.
dpkg -L "$name" | grep -qx "$mcp" || { echo "no knowledge MCP in $name" >&2; exit 1; }
got="$("$mcp" --version)"
echo "installed: $got"
if [ -n "$version" ] && [ "$got" != "exiled-studio-knowledge-mcp $version" ]; then
  echo "knowledge MCP version: $got, not $version" >&2
  exit 1
fi
xvfb-run -a bash "$here/smoke.sh" "$binary" deb "$version"
sudo dpkg -r "$name"
for f in "$binary" "$mcp"; do
  if [ -e "$f" ]; then
    echo "still installed after removing: $f" >&2
    exit 1
  fi
done
echo "removed: $name"

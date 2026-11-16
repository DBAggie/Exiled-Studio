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
binary="$(dpkg -L "$name" | grep -E '^/usr/bin/[^/]+$' | head -n 1)"
[ -n "$binary" ] || { echo "no executable in $name" >&2; exit 1; }
echo "installed: $binary"
xvfb-run -a bash "$here/smoke.sh" "$binary" deb "$version"
sudo dpkg -r "$name"
if [ -e "$binary" ]; then
  echo "still installed after removing: $binary" >&2
  exit 1
fi
echo "removed: $name"

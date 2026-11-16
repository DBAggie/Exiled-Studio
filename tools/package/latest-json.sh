#!/usr/bin/env bash
# Writes the update feed, latest.json, for a release (Phase 5, P3;
# docs/phase-5/updates.md) from the signed update packages in <dir>.
#
#   latest-json.sh <dir> <version> <owner/repo> > latest.json
#
# Entries: windows-x86_64-nsis, windows-x86_64-msi, linux-x86_64-appimage,
# and the plain windows-x86_64 and linux-x86_64 (the NSIS setup and the
# AppImage), which the portable zip and the .deb find; those only notify.
set -euo pipefail
dir="$1"
version="$2"
repo="$3"
base="https://github.com/$repo/releases/download/v$version"
entry() { # <file>
  local file sig
  file="$(basename "$1")"
  sig="$(tr -d '\r\n' < "$1.sig")"
  jq -n --arg s "$sig" --arg u "$base/$file" '{signature: $s, url: $u}'
}
nsis="$(ls "$dir"/*-setup.exe)"
msi="$(ls "$dir"/*.msi)"
appimage="$(ls "$dir"/*.AppImage)"
jq -n \
  --arg v "$version" \
  --arg d "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg n "See the release notes: https://github.com/$repo/releases/tag/v$version" \
  --argjson nsis "$(entry "$nsis")" \
  --argjson msi "$(entry "$msi")" \
  --argjson appimage "$(entry "$appimage")" \
  '{version: $v, notes: $n, pub_date: $d, platforms: {
     "windows-x86_64-nsis": $nsis, "windows-x86_64-msi": $msi,
     "linux-x86_64-appimage": $appimage,
     "windows-x86_64": $nsis, "linux-x86_64": $appimage }}'

#!/usr/bin/env bash
# Builds the knowledge MCP server (ADR-018) and puts it where `tauri build`
# takes it as a sidecar: crates/loom-tauri/binaries/, named with the host's
# target triple, which crates/loom-tauri/tauri.knowledge.conf.json names
# (`bundle.externalBin`). Every installer then carries it beside the
# application (exiled-studio-knowledge-mcp[.exe]; /usr/bin in the .deb and
# the AppImage). Only the release workflow and CI's package job pass that
# config, so a plain `cargo build` of the app does not need the binary.
# From the repository root:
#
#   knowledge-sidecar.sh [<version>]
#
# With <version>, the binary must report it (`--version`).
set -euo pipefail
version="${1:-}"

cargo build --release --locked -p loom-knowledge-mcp
ext=""
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) ext=".exe" ;;
esac
bin="target/release/exiled-studio-knowledge-mcp$ext"
if [ -n "$version" ]; then
  got="$("$bin" --version | tr -d '\r')"
  if [ "$got" != "exiled-studio-knowledge-mcp $version" ]; then
    echo "knowledge MCP version: $got, not $version" >&2
    exit 1
  fi
fi
triple="$(rustc -vV | sed -n 's/^host: //p')"
[ -n "$triple" ] || { echo "no host triple from rustc -vV" >&2; exit 1; }
mkdir -p crates/loom-tauri/binaries
cp "$bin" "crates/loom-tauri/binaries/exiled-studio-knowledge-mcp-$triple$ext"
echo "sidecar: crates/loom-tauri/binaries/exiled-studio-knowledge-mcp-$triple$ext"

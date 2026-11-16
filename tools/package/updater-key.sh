#!/usr/bin/env bash
# The release workflow's update key (Phase 5, P3; docs/release/signing.md).
# Runs before anything is built: the application compiles
# release/updater.pub in, and `tauri build` with tauri.updater.conf.json
# signs each update package (through tools/package/with-updater-key.sh).
#
# - LOOM_UPDATER_KEY set (the `release` environment's secret, with
#   LOOM_UPDATER_KEY_PASSWORD, given to this step only): release/updater.pub
#   must hold its public half, checked by signing a probe file and
#   comparing the signature's key ID with the public key's. A mismatch
#   would publish updates no installed copy accepts.
# - Absent: a throwaway key is made with `tauri signer generate`, its public
#   half written to the run's copy of release/updater.pub (never committed).
#   A tag's run refuses throwaway mode: installed copies would never accept
#   the next release.
#
# Either way the key and its password go to files in $RUNNER_TEMP/keys,
# never to the job's environment; only the `tauri build` commands see them.
set -euo pipefail
tauri=ui/node_modules/.bin/tauri
keys="${RUNNER_TEMP:-$(mktemp -d)}/keys"
mkdir -p "$keys"
chmod 700 "$keys"
# The 8-byte key ID of a minisign public key or signature file, as tauri
# writes them (base64 of the text file; its second line is base64 of
# algorithm, key ID, ...).
key_id() { base64 -d < "$1" | tr -d '\r' | sed -n 2p | base64 -d | head -c 10 | tail -c 8 | od -An -tx1 | tr -d ' \n'; }
if [ -n "${LOOM_UPDATER_KEY:-}" ]; then
  [ -s release/updater.pub ] || {
    echo "release/updater.pub is empty: commit the update key's public half first" >&2
    exit 1
  }
  (umask 077
   printf '%s' "$LOOM_UPDATER_KEY" > "$keys/updater.key"
   printf '%s' "${LOOM_UPDATER_KEY_PASSWORD:-}" > "$keys/updater.password")
  probe="$(mktemp -d)"
  echo "update key probe" > "$probe/probe"
  "$tauri" signer sign -k "$LOOM_UPDATER_KEY" -p "${LOOM_UPDATER_KEY_PASSWORD:-}" "$probe/probe" >/dev/null
  signed="$(key_id "$probe/probe.sig")"
  public="$(key_id release/updater.pub)"
  rm -rf "$probe"
  [ -n "$signed" ] && [ "$signed" = "$public" ] || {
    echo "the LOOM_UPDATER_KEY secret (key $signed) is not release/updater.pub's key ($public)" >&2
    exit 1
  }
  echo "update key: $public (release)"
else
  case "${GITHUB_REF:-}" in
    refs/tags/*) echo "a tagged release needs the LOOM_UPDATER_KEY secret" >&2; exit 1 ;;
  esac
  dir="$keys/updater-throwaway"
  mkdir -p "$dir"
  "$tauri" signer generate --ci -p "" -f -w "$dir/updater.key" >/dev/null
  tr -d '\r\n' < "$dir/updater.key.pub" > release/updater.pub
  echo "::add-mask::$(tr -d '\r\n' < "$dir/updater.key")"
  (umask 077
   tr -d '\r\n' < "$dir/updater.key" > "$keys/updater.key"
   : > "$keys/updater.password")
  rm -rf "$dir"
  echo "update key: $(key_id release/updater.pub) (throwaway; this run's builds only)"
fi
# `tauri build` with createUpdaterArtifacts reads the public key from the
# configuration when it signs (tauri.conf.json leaves it empty: the
# application takes release/updater.pub at run time, updates.rs), so the
# run's copy of tauri.updater.conf.json gets it (never committed). Without
# it the bundler fails with "Missing comment in public key" (first seen in
# the v0.9.0-1 run).
conf=crates/loom-tauri/tauri.updater.conf.json
jq --arg key "$(tr -d '\r\n' < release/updater.pub)" '.plugins.updater.pubkey = $key' "$conf" > "$keys/updater.conf.json"
mv "$keys/updater.conf.json" "$conf"
echo "LOOM_KEYS_DIR=$keys" >> "${GITHUB_ENV:-/dev/null}"

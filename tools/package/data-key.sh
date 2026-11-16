#!/usr/bin/env bash
# The release workflow's data key (Phase 5, P2; docs/release/signing.md,
# "Throwaway mode"). Runs before anything is built, since the application
# compiles release/trust.json in.
#
# - LOOM_DATA_KEY set (the `release` environment's secret, given to this
#   step only): its key must already be in release/trust.json, unrevoked,
#   or the run fails.
# - LOOM_DATA_KEY absent: a throwaway key is made and added to the run's
#   copy of release/trust.json (never committed). A tag's run refuses
#   throwaway mode: a published release must be signed by a key the next
#   release still trusts.
#
# Either way the secret goes to a file, $RUNNER_TEMP/keys/data.secret,
# never to the job's environment, so later build steps do not carry it;
# tools/package/shipped-bundle.sh signs with it and deletes it.
set -euo pipefail
tool() { cargo run -q --locked -p loom-release -- "$@"; }
keys="${RUNNER_TEMP:-$(mktemp -d)}/keys"
mkdir -p "$keys"
chmod 700 "$keys"
if [ -n "${LOOM_DATA_KEY:-}" ]; then
  id="$(tool trust --secret-env LOOM_DATA_KEY | jq -r .id)"
  jq -e --arg id "$id" '.keys[] | select(.id == $id)' release/trust.json >/dev/null \
    || { echo "the LOOM_DATA_KEY secret's key $id is not in release/trust.json" >&2; exit 1; }
  jq -e --arg id "$id" '.revoked[] | select(.key_id == $id)' release/trust.json >/dev/null \
    && { echo "the LOOM_DATA_KEY secret's key $id is revoked" >&2; exit 1; }
  (umask 077; printf '%s' "$LOOM_DATA_KEY" > "$keys/data.secret")
  echo "data key: $id (release)"
else
  case "${GITHUB_REF:-}" in
    refs/tags/*) echo "a tagged release needs the LOOM_DATA_KEY secret" >&2; exit 1 ;;
  esac
  dir="$keys/data-throwaway"
  tool keygen --out "$dir" --note "Throwaway key of release workflow run ${GITHUB_RUN_ID:-local}" >/dev/null
  secret="$(ls "$dir"/*.secret)"
  echo "::add-mask::$(tr -d '\n' < "$secret")"
  tool trust-add release/trust.json --secret "$secret" \
    --note "Throwaway key of release workflow run ${GITHUB_RUN_ID:-local}"
  (umask 077; tr -d '\n' < "$secret" > "$keys/data.secret")
  rm -rf "$dir"
  echo "data key: $(jq -r '.keys[-1].id' release/trust.json) (throwaway; this run's builds only)"
fi
echo "LOOM_KEYS_DIR=$keys" >> "${GITHUB_ENV:-/dev/null}"

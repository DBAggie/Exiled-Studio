#!/usr/bin/env bash
# The release workflow's data key (Phase 5, P2; docs/release/signing.md,
# "Throwaway mode"). Runs before anything is built, since the application
# compiles release/trust.json in.
#
# - LOOM_DATA_KEY set (the `release` environment's secret): its key must
#   already be in release/trust.json, or the run fails.
# - LOOM_DATA_KEY absent: a throwaway key is made, added to the run's copy of
#   release/trust.json (never committed), and exported as LOOM_DATA_KEY for
#   the later steps. A tag's run refuses throwaway mode: a published release
#   must be signed by a key the next release still trusts.
set -euo pipefail
tool() { cargo run -q --locked -p loom-release -- "$@"; }
if [ -n "${LOOM_DATA_KEY:-}" ]; then
  id="$(tool trust --secret-env LOOM_DATA_KEY | jq -r .id)"
  jq -e --arg id "$id" '.keys[] | select(.id == $id)' release/trust.json >/dev/null \
    || { echo "the LOOM_DATA_KEY secret's key $id is not in release/trust.json" >&2; exit 1; }
  jq -e --arg id "$id" '.revoked[] | select(.key_id == $id)' release/trust.json >/dev/null \
    && { echo "the LOOM_DATA_KEY secret's key $id is revoked" >&2; exit 1; }
  echo "data key: $id (release)"
else
  case "${GITHUB_REF:-}" in
    refs/tags/*) echo "a tagged release needs the LOOM_DATA_KEY secret" >&2; exit 1 ;;
  esac
  dir="${RUNNER_TEMP:-$(mktemp -d)}/data-key"
  tool keygen --out "$dir" --note "Throwaway key of release workflow run ${GITHUB_RUN_ID:-local}" >/dev/null
  secret="$(ls "$dir"/*.secret)"
  echo "::add-mask::$(tr -d '\n' < "$secret")"
  tool trust-add release/trust.json --secret "$secret" \
    --note "Throwaway key of release workflow run ${GITHUB_RUN_ID:-local}"
  echo "LOOM_DATA_KEY=$(tr -d '\n' < "$secret")" >> "${GITHUB_ENV:-/dev/null}"
  echo "data key: $(jq -r '.keys[-1].id' release/trust.json) (throwaway; this run's builds only)"
fi

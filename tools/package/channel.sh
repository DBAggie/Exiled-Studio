#!/usr/bin/env bash
# The release channel of a release workflow run (docs/release/channels.md),
# written to $GITHUB_OUTPUT as `channel` (stable or experimental) and
# `config` (the extra `tauri build` arguments).
#
# - A tag v<version>: the version must be the workspace's. A plain
#   version (1.2.0) is stable and its commit must be on main; a version
#   with a numeric pre-release (1.2.0-3) is experimental and its commit
#   must be on experimental. The MSI takes the pre-release as its fourth
#   field (1.2.0.3), so it must be a number up to 65535.
# - A run by hand (workflow_dispatch): the branch's channel, experimental
#   on experimental and stable anywhere else; nothing is released.
set -euo pipefail
out="${GITHUB_OUTPUT:-/dev/stdout}"
ref="${GITHUB_REF:?}"
version="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "loom-tauri") | .version')"
case "$ref" in
  refs/tags/v*)
    tag="${ref#refs/tags/v}"
    if [ "$tag" != "$version" ]; then
      echo "tag v$tag does not match the workspace version $version" >&2
      exit 1
    fi
    if [[ "$tag" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      channel=stable branch=main
    elif [[ "$tag" =~ ^[0-9]+\.[0-9]+\.[0-9]+-([0-9]+)$ ]] && [ "${BASH_REMATCH[1]}" -le 65535 ]; then
      channel=experimental branch=experimental
    else
      echo "v$tag is neither stable (1.2.0) nor experimental (1.2.0-3)" >&2
      exit 1
    fi
    git fetch --no-tags --quiet origin "$branch"
    if ! git merge-base --is-ancestor "${GITHUB_SHA:?}" "origin/$branch"; then
      echo "v$tag ($channel) must tag a commit on $branch" >&2
      exit 1
    fi
    ;;
  refs/heads/experimental) channel=experimental ;;
  *) channel=stable ;;
esac
config=""
[ "$channel" = experimental ] && config="--config crates/loom-tauri/tauri.experimental.conf.json"
echo "channel=$channel" >> "$out"
echo "config=$config" >> "$out"
echo "channel: $channel ($ref, version $version)" >&2

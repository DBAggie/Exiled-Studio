#!/usr/bin/env bash
# Spike A gate 1 (determinism): runs `loom-extractor extract` twice on the same
# install and checks that every output file and the manifest are byte-identical.
# Maintainer-run (needs a game install and the Oodle decoder, see AGENTS.md).
#
# usage: tools/extractor/scripts/check-determinism.sh <install> <poe1|poe2> [extract options]
#   e.g. tools/extractor/scripts/check-determinism.sh "D:/SteamLibrary/steamapps/common/Path of Exile" poe1
#
# Output goes to local/determinism/<game>/ (git-ignored), which is replaced.
# It must stay git-ignored: the manifest records whether the source tree is
# dirty, so output written inside a tracked folder would make run 2 differ.
# For gate evidence, run from a clean tree (manifest "dirty": false).
# Prints the SHA-256 of each run's manifest; exits non-zero on any difference.
set -euo pipefail

if [ "$#" -lt 2 ]; then
  sed -n '6,7p' "$0" >&2
  exit 2
fi
install=$1
game=$2
shift 2
case "$game" in
  poe1 | poe2) ;;
  *) echo "game must be poe1 or poe2" >&2; exit 2 ;;
esac

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
work="$root/local/determinism/$game"
rm -rf "$work"
mkdir -p "$work"

cargo build --release --locked --manifest-path "$root/Cargo.toml" -p loom-extractor --bin loom-extractor
bin="$root/target/release/loom-extractor"

for run in 1 2; do
  "$bin" extract "$install" --game "$game" --out "$work/run$run" "$@" > "$work/run$run.log"
  (cd "$work/run$run" && find . -type f | LC_ALL=C sort | xargs sha256sum) > "$work/run$run.sha256"
done

if ! diff -r -q "$work/run1" "$work/run2"; then
  echo "NOT deterministic: outputs differ (see above)" >&2
  exit 1
fi
cmp "$work/run1.sha256" "$work/run2.sha256"

files=$(wc -l < "$work/run1.sha256")
echo "deterministic: $files files identical in two runs ($game)"
echo "run 1 manifest sha256: $(sha256sum "$work/run1/manifest.json" | cut -d' ' -f1)"
echo "run 2 manifest sha256: $(sha256sum "$work/run2/manifest.json" | cut -d' ' -f1)"

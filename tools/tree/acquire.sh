#!/bin/sh
# Fetches GGG's passive tree export at the commit pinned in
# tools/tree/manifest.json into local/tree-export/ (ADR-005 Amendment 2;
# maintainer-run, network: git only). See acquire.ps1.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
field() { python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['export'][sys.argv[2]])" "$here/manifest.json" "$1"; }
repo=$(field repository); tag=$(field tag); commit=$(field commit)
out="$root/local/tree-export"
if [ ! -d "$out/.git" ]; then
  git clone --quiet --depth 1 --branch "$tag" "$repo" "$out"
else
  git -C "$out" fetch --quiet --depth 1 origin tag "$tag"
  git -C "$out" checkout --quiet --detach "$tag"
fi
head=$(git -C "$out" rev-parse HEAD)
[ "$head" = "$commit" ] || { echo "local/tree-export is at $head, the pin is $commit" >&2; exit 1; }
echo "local/tree-export at $tag ($head)"

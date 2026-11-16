#!/bin/sh
# Fetches GGG's passive tree export at the commit pinned in
# tools/tree/manifest.json into local/tree-export/ (ADR-005 Amendment 2;
# maintainer-run, network: git only). With a tree version (`3_28`), the
# commit pinned for that version under `versions` into
# local/tree-export-<version>/ instead (tree packs, `loom-parity-cli
# tree-pack`). See acquire.ps1.
#
#   sh tools/tree/acquire.sh [version]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
version=${1:-}
field() { python3 -c "import json,sys;m=json.load(open(sys.argv[1]));v=sys.argv[3];print(m['export']['repository'] if sys.argv[2]=='repository' else (m['versions'][v] if v else m['export'])[sys.argv[2]])" "$here/manifest.json" "$1" "$version"; }
repo=$(field repository); tag=$(field tag); commit=$(field commit)
if [ -n "$version" ]; then out="$root/local/tree-export-$version"; else out="$root/local/tree-export"; fi
if [ ! -d "$out/.git" ]; then
  git -c advice.detachedHead=false clone --quiet --depth 1 --branch "$tag" "$repo" "$out"
else
  git -C "$out" fetch --quiet --depth 1 origin tag "$tag"
  git -C "$out" checkout --quiet --detach "$tag"
fi
head=$(git -C "$out" rev-parse HEAD)
[ "$head" = "$commit" ] || { echo "$out is at $head, the pin is $commit" >&2; exit 1; }
echo "$out at $tag ($head)"

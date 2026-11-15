#!/usr/bin/env bash
# The passive tree against PoB (docs/tree/parity.md): layout, allocation,
# and PoB reading edited exports back. Maintainer-run, from the repository
# root, in Git Bash on Windows or a shell on Linux. Needs the oracle
# (tools/oracle/acquire) and a bundle with the layout (tools/tree, the
# tree-layout command). Writes the records to docs/tree/records/ and the
# working files to local/tree/pob/.
#
#   tools/tree/parity.sh [bundle-dir]     # default local/tree/bundle-poe1-tree-a
#
# Exit 0 when every record matches.
set -euo pipefail

BUNDLE="${1:-local/tree/bundle-poe1-tree-a}"
COUNT=2000
SEED=20261115
BUILDS="ice-nova-elementalist-campaign-lab1 facebreaker-luminary animate-guardian-guardian"

if command -v cygpath >/dev/null; then
  R="$(cygpath -m "$PWD")"
  LUAJIT="$R/local/oracle/bin/luajit.exe"
  CPATH="$R/local/oracle/bin/?.dll"
  ZLIB="$R/local/oracle/pob/runtime/zlib1.dll"
  CLI=target/release/loom-parity-cli.exe
else
  R="$PWD"
  LUAJIT="$R/local/oracle/bin/luajit"
  CPATH="$R/local/oracle/bin/?.so"
  ZLIB=libz.so.1
  CLI=target/release/loom-parity-cli
fi
W=local/tree/pob
OUT=docs/tree/records
mkdir -p "$W/readback" "$OUT"

cargo run --release --quiet -p loom-parity-cli -- oracle-verify tools/oracle/manifest.json local/oracle >/dev/null
cargo build --release --quiet -p loom-parity-cli

# PoB on the pinned oracle runtime, as tools/oracle/data/README.md runs it.
pob() {
  (cd local/oracle/pob/src && env -i SystemRoot="${SYSTEMROOT:-}" PATH="${PATH}" \
    LUA_PATH='./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua' LUA_CPATH="$CPATH" \
    "$LUAJIT" "$R/tools/oracle/data/pob_tree.lua" "$@" "zlib=$ZLIB" </dev/null >/dev/null 2>"$R/$W/pob.log")
}

status=0
pob layout "$R/$W/layout.json"
"$CLI" tree-parity layout "$W/layout.json" "$BUNDLE" >"$OUT/layout.json" || status=1

"$CLI" tree-parity alloc-requests "$BUNDLE" "$COUNT" "$SEED" >"$W/alloc-requests.txt"
pob alloc "$R/$W/alloc-requests.txt" "$R/$W/alloc.json"
"$CLI" tree-parity alloc "$W/alloc-requests.txt" "$W/alloc.json" "$BUNDLE" >"$OUT/alloc.json" || status=1

# Tests run in the crate's folder: the bundle's path made absolute.
case "$BUNDLE" in /* | ?:*) ABS="$BUNDLE" ;; *) ABS="$R/$BUNDLE" ;; esac
LOOM_TREE_BUNDLE="$ABS" cargo test --quiet -p loom-app --test tree_workflow -- \
  --ignored edited_builds_should_be_exported_for_pob_to_read_back >/dev/null
for b in $BUILDS; do
  pob readback "$R/$W/readback/$b.xml" "$R/$W/readback/$b.json"
  "$CLI" tree-parity readback "$W/readback/$b.xml" "$W/readback/$b.json" >"$OUT/readback-$b.json" || status=1
done

echo "records in $OUT (exit $status)"
exit $status

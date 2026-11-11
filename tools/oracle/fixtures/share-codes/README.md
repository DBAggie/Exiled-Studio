# Share codes made by PoB (Phase 2 plan, step 10)

Each `<name>.code` is a share code made by Path of Building v2.67.2 (commit `b32759ab0f31a1c8499a0d420cb0f0633d4fe478`, the pinned oracle in `../../manifest.json`) from the public fixture of the same name. Next to it, `<name>.pob.xml` is the XML PoB made the code from (`build:SaveDB("code")`). The test `crates/loom-serialization/tests/share_code_fixtures.rs` checks that each code decodes to exactly that XML and imports to the same build.

| Code | Made from | Code SHA-256 | XML SHA-256 |
|---|---|---|---|
| `minimal-level1.code` | `../minimal-level1.xml` (ours) | `7f216ee9…498df` | `80ec26d8…950db` |
| `boneshatter-juggernaut.code` | `../community/boneshatter-juggernaut.xml` (credited in `../community/SOURCES.md`) | `a6569688…4e66a` | `21455c35…01926` |

Made 2026-11-11 on Windows with zlib 1.3.1 (PoB's own `runtime/zlib1.dll`).

## How they are made
`tools/oracle/adapter/share_code.lua` (ours, MIT) runs on the pinned oracle runtime (`local/oracle/`, built by `tools/oracle/acquire.ps1` or `acquire.sh`). It loads the fixture with HeadlessWrapper's `loadBuildFromXML`, then evaluates PoB's own expression from `Classes/ImportTab.lua` line 504, `common.base64.encode(Deflate(build:SaveDB("code"))):gsub("+","-"):gsub("/","_")`. It writes the code and the `SaveDB` text.

PoB's `Deflate` is a native function of its GUI host, and HeadlessWrapper stubs it (returns `""`). The script puts zlib's `compress2` at the default level in its place, from the same zlib library. Any zlib stream inflates to the same text, so the code's content is PoB's: its XML, its base64, and its alphabet swap. `Inflate` is the oracle's inflate shim, and writes are refused as in `oracle.lua`. The boneshatter build loads Glorious Vanity data, so PoB tries to cache `Data/TimelessJewelData/GloriousVanity.bin`; the write is refused, as in Spike C, and the build in memory is the same.

From the repository root (Git Bash on Windows; on Linux use `bin/?.so` and `zlib=libz.so.1`), close stdin, with an absolute repository path in `R`:

```sh
R="$(cygpath -m "$PWD")"; O="$R/local/oracle"
(cd local/oracle/pob/src && env -i SystemRoot="$SYSTEMROOT" \
  LUA_PATH='./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua' LUA_CPATH="$O/bin/?.dll" \
  "$O/bin/luajit.exe" "$R/tools/oracle/adapter/share_code.lua" \
  "$R/tools/oracle/fixtures/minimal-level1.xml" \
  "$R/tools/oracle/fixtures/share-codes/minimal-level1.code" \
  "$R/tools/oracle/fixtures/share-codes/minimal-level1.pob.xml" \
  "zlib=$O/pob/runtime/zlib1.dll" </dev/null)
```

## A run is not reproducible byte for byte
PoB's `SaveDB` writes attributes, and some children (the `Calcs` sections and inputs), in Lua `pairs` order over hash tables. That order changes between runs, so each run gives another XML text and another code; five runs of `minimal-level1` gave five different codes, 3,540 to 3,588 characters, all from 15,806-byte XML. Attribute order does not change the parsed tree; child order does. That is why each code is committed with the exact XML it was made from, and the test compares the two, not a fresh run. Regenerating replaces both files and their hashes above.

Git keeps these files' bytes as committed (`.gitattributes`, `-text`).

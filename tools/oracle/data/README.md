# PoB data export (Phase 3 plan, step 2)

`pob_data.lua` (ours, MIT) writes the game data Path of Building v2.67.2 holds after startup, for the values a calculation reads, as one JSON document (format 1). `loom-parity-cli data-parity` compares it with a PoE1 bundle of format 2 (`docs/phase-3/data-parity.md`).

It is maintainer-run, not in CI, and outside the oracle adapter and its protocol (protocol 3 is unchanged). It runs on the pinned oracle runtime (`local/oracle/`, built by `tools/oracle/acquire.ps1` or `acquire.sh`, checked by `oracle-verify`), starts PoB with its own `HeadlessWrapper.lua`, and copies plain values from:

| Export field | PoB source |
|---|---|
| `skills` | `data.skills` (`src/Data/Skills/*.lua`): stats, per-level values, costs, crit, effectiveness, multipliers, cooldown, reservations, types, constant and quality stats |
| `gems` | `data.gems` (`src/Data/Gems.lua`) |
| `bases` | `data.itemBases` with `weapon` or `armour` |
| `monster_tables`, `game_constants`, `character_constants`, `monster_constants` | `src/Data/Misc.lua` |
| `tree` | `src/TreeData/<version>/tree.lua` as PoB reads it, before `PassiveTree.lua` adds display lines |

No value is converted; the comparison names each conversion PoB's exporter made. Not copied: level requirements, Vaal soul costs, and stat interpolation kinds (the bundle does not carry the first two; the third stays raw until P3-9a). Writes go only to the output file: `io.open` is the oracle's write guard, and a refused write stops the run.

From the repository root (Git Bash on Windows; on Linux use `bin/?.so`), close stdin, with an absolute repository path in `R`:

```sh
R="$(cygpath -m "$PWD")"; O="$R/local/oracle"
(cd local/oracle/pob/src && env -i SystemRoot="$SYSTEMROOT" \
  LUA_PATH='./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua' LUA_CPATH="$O/bin/?.dll" \
  "$O/bin/luajit.exe" "$R/tools/oracle/data/pob_data.lua" "$R/local/p3/pob-data.json" </dev/null)
```

`tree=<version>` picks another tree than PoB's `latestTreeVersion` (`3_29` at v2.67.2). Two runs give the same bytes (the JSON writer sorts keys), about 6.8 MB in 4 seconds.

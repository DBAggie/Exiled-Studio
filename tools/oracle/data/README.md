# PoB data export (Phase 3 plan, step 2)

`pob_data.lua` (ours, MIT) writes the game data Path of Building v2.67.2 holds after startup, for the values a calculation reads, as one JSON document (format 16; format 15 before P6-11av had no `tattoos`, format 14 before P6-10a had no `timeless`, format 13 before data schema 5 had no skills' `levelRequirement`, `vaalStoredUses`, `soulPreventionDuration`, or `Soul` cost, format 12 before Phase 6 P6-7b1 had no `minion_mods` or `minion_templates`, format 11 before Phase 6 P6-7a had no `minions`, format 10 before Phase 6 P6-4b had no `flasks`, format 9 before Phase 6 P6-1 had no skills' `base_flags`, `parts`, or `functions`, format 8 before the passive tree's T3 had no `cluster_jewels`, format 7 before B-2's card data had no `flavour_text`, format 6 before the passive tree's T-f had no `jewel_radii`, format 5 before B-2 had no `mods` or `base_requirements`, format 2 before P3-12g no `pantheons`, format 1 before P3-1d no `bosses`). `loom-parity-cli data-parity` compares it with a PoE1 bundle of format 2 (`docs/phase-3/data-parity.md`).

It is maintainer-run, not in CI, and outside the oracle adapter and its protocol (it reads no build, so the adapter protocol does not apply). It runs on the pinned oracle runtime (`local/oracle/`, built by `tools/oracle/acquire.ps1` or `acquire.sh`, checked by `oracle-verify`), starts PoB with its own `HeadlessWrapper.lua`, and copies plain values from:

| Export field | PoB source |
|---|---|
| `skills` | `data.skills` (`src/Data/Skills/*.lua`): stats, per-level values, costs, crit, effectiveness, multipliers, cooldown, reservations, types, constant and quality stats; PoB's hand-written base flags, parts, and skill function names (format 10, read by `loom-parity-cli pob-skills`) |
| `gems` | `data.gems` (`src/Data/Gems.lua`) |
| `bases` | `data.itemBases` with `weapon` or `armour` |
| `base_requirements` | `data.itemBases`: every base's `req` (level, str, dex, int) and whether it is a flask (format 6) |
| `mods` | the keyed `data.itemMods` files (`src/Data/Mod*.lua`): each mod's `affix`, `level`, `type`, and the files holding it (format 6) |
| `monster_tables`, `game_constants`, `character_constants`, `monster_constants` | `src/Data/Misc.lua` |
| `bosses` | `data.bosses` (`src/Data/Bosses.lua`): each boss's armour and evasion multipliers and uber mark |
| `pantheons` | `data.pantheons` (`src/Data/Pantheons.lua`): each god's major mark, its souls' names, and their mods' lines and values |
| `jewel_radii` | `data.jewelRadius` (`src/Modules/Data.lua`): the radii of the tree version picked, each entry's label, inner, and outer radius (format 7) |
| `flavour_text` | `data.flavourText` (`src/Data/FlavourText.lua`): each unique flavour text entry's ID, name, and lines as PoB's exporter cleaned them (format 8) |
| `minions` | `data.minions` (minions and spectres): each by its monster variety's ID (a minion's short name mapped through PoB's `Export/Minions/Minions.txt`), with life, defences, resistances, damage, damage spread, attack time and range, accuracy, damage fixup, life scaling, weapon types, tags, and skill list (format 12) |
| `minion_mods` | `Data/Minions.lua` and `Data/Spectres.lua` as text: the comment PoB's exporter wrote beside each `modList` line (`-- <mod> [<stat> = <value>]`), per minion, keyed as `minions` (format 13) |
| `minion_templates` | `Export/Minions/Minions.txt` and `Spectres.txt`, read as PoB's template processor reads them: each emitted minion's file, line, variety, `#limit`, `#hostile`, extra skills, and `#mod` lines, by PoB's name (format 13; `loom-parity-cli pob-minions` generates the engine's table from it) |
| `flasks` | `data.itemBases`: every flask base's `flask` table, its life and mana per use, duration, and buff lines as PoB's exporter rendered them (format 11) |
| `cluster_jewels` | `data.clusterJewels` (`src/Data/ClusterJewels.lua`): each jewel by base name with its size index, node counts, indices, and small passives' names and tags by passive ID; the notables' sort order by name; the keystones' names; each proxy's start indices by size (format 9) |
| `timeless` | `Data/TimelessJewelData/LegionPassives.lua`, the node index mapping, the jewel types and seed ranges, each zip's Adler-32 trailer (format 15) |
| `tattoos` | `Data/TattooPassives.lua`'s nodes by name: ID, override type, keystone flag, stats' ranges (format 16) |
| `tree` | `src/TreeData/<version>/tree.lua` as PoB reads it, before `PassiveTree.lua` adds display lines |

No value is converted; the comparison names each conversion PoB's exporter made. Not copied: stat interpolation kinds (raw until P3-9a). Level requirements and Vaal soul costs are copied since format 14. Writes go only to the output file: `io.open` is the oracle's write guard, and a refused write stops the run.

From the repository root (Git Bash on Windows; on Linux use `bin/?.so`), close stdin, with an absolute repository path in `R`:

```sh
R="$(cygpath -m "$PWD")"; O="$R/local/oracle"
(cd local/oracle/pob/src && env -i SystemRoot="$SYSTEMROOT" \
  LUA_PATH='./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua' LUA_CPATH="$O/bin/?.dll" \
  "$O/bin/luajit.exe" "$R/tools/oracle/data/pob_data.lua" "$R/local/p3/pob-data.json" </dev/null)
```

`tree=<version>` picks another tree than PoB's `latestTreeVersion` (`3_29` at v2.67.2). Two runs give the same bytes (the JSON writer sorts keys), about 9 MB in 4 seconds.

## Item copies (`pob_items.lua`)

B-2 item evidence: PoB's own reading of the game's item copies (Ctrl+Alt+C). For each copy given, PoB reads it as its item paste does (`ItemClass:ParseRaw`), then writes the text it keeps (`ItemClass:BuildRaw`) to `<out-dir>/<name>.txt`. The paste's later `NormaliseQuality` is left out, so the copy's quality stays the game's. Maintainer-run, not in CI, and outside the oracle adapter. Run it like `pob_data.lua` above, with the output folder and the copies as arguments:

```sh
(cd local/oracle/pob/src && env -i SystemRoot="$SYSTEMROOT" \
  LUA_PATH='./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua' LUA_CPATH="$O/bin/?.dll" \
  "$O/bin/luajit.exe" "$R/tools/oracle/data/pob_items.lua" \
  "$R/docs/ui/b2-data/game-copies/pob-oracle" "$R"/docs/ui/b2-data/game-copies/*.txt </dev/null)
```

The committed output is `docs/ui/b2-data/game-copies/pob-oracle/`, and `crates/loom-serialization/tests/game_copy_fixtures.rs` compares it with ours (`docs/ui/b2-data/item-evidence.md`, "PoB's reading against ours").

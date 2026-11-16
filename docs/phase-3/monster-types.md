# Phase 3: monster types for the default enemy (P3-1d)

Maintainer decision Q10 (2026-11-12, option a): the default enemy's armour and evasion come from game data. PoB v2.67.2's default enemy is a Guardian or Pinnacle boss (`enemyIsBoss`, default index 3). It scales the monster level's armour and evasion by `100 + mean` percent, where the mean is taken over the multipliers of the bosses PoB lists (`data.bossStats`, `src/Modules/Data.lua`). Those multipliers are the game's `MonsterTypes.Armour` and `.Evasion` (`src/Export/Scripts/bossData.lua`), and which bosses count is PoB's choice (`src/Export/Enemies/Bosses.txt`).

## What is added

| Part | Where |
|---|---|
| `MonsterTypes` among the extractor's property roots, in both games | `tools/extractor/src/extract.rs` (`ITEM_PROPERTY_TABLES`) |
| `data/monster_types.json` in bundle format 2: `MonsterType { id, armour, evasion }`, multipliers in percent; required; empty and duplicate IDs refused (`BUNDLE_EMPTY_ID`, `BUNDLE_DUPLICATE_ID`) | `loom_data::bundle::properties`, `validate.rs` |
| The converter reads `Id`, `Armour`, `Evasion` | `tools/parity-cli/src/bundle/base_properties.rs` |
| PoB's boss list as a cited rule: the 22 bosses, each with its PoB name, monster type, and uber mark | `loom_calc::poe1::enemy::BOSSES` |
| PoB's `data.bosses` in the data export (export format 2) | `tools/oracle/data/pob_data.lua` |
| `data-parity` compares each listed boss's two multipliers and uber mark with the bundle (table `bosses`) | `tools/parity-cli/src/data_parity/compare.rs` |

The enemy model that uses them is P3-11.

## Evidence (2026-11-12, Windows)

| Run | Result |
|---|---|
| `check-determinism.sh` on PoE1 and PoE2 (clean `d810f28`) | Deterministic: 571 and 975 files identical in two runs. Every data file is byte-identical to P3-1c's extract; only the manifest changed (the extractor commit, `MonsterTypes` now a root). `MonsterTypes` was already extracted as a referenced table |
| `bundle` twice per game (clean `c4d6b40`) | Byte-identical. Against P3-1c's bundles, every file is equal except `bundle.json` and the new `data/monster_types.json` (3,600 PoE1 rows). Manifest SHA-256: PoE1 `deaaa6fb…`, PoE2 `b1b50b5e…` |
| `bundle-check`, both games | Exit 0, no findings |
| `pob_data.lua` twice on the oracle runtime | Byte-identical; export format 2 with 22 bosses; oracle checkout clean |
| `data-parity` twice (PoE1 bundle against the export, `docs/phase-3/data-parity-rules-poe1.json`) | Byte-identical. **`bosses`: 66 values compared (22 bosses x armour, evasion, uber), 0 differences.** Every other table as P3-2's: 534 differences, `uncategorized` 0, 314 overlay rows. Summary `docs/phase-3/data-parity-poe1-26530219.json` (replaced: new tool commit, bundle, and export hashes, and the `bosses` row) |

The committed bundle summaries (`docs/phase-3/bundle-poe1-26530219.json`, `bundle-poe2-26476662.json`) are replaced with these bundles' (the `bundle-check` ones are unchanged: no findings).

Tests: converter (both games, missing columns, a type without an ID), validator mutations (a duplicate and an empty ID), the boss list (each name and type once, 7 uber), `data-parity` (equal bosses; a changed armour, evasion, or uber mark as one difference each; a boss on one side only), the export format check. 1069 workspace tests, clippy, fmt.

A format-2 bundle from before P3-1d no longer loads (the new file is required), so local bundles are remade: `local/p3/p3-1d/bundle-poe1-a` and `-poe2-a`. The P3-2 overlay names the old bundle's hash. Use `local/p3/p3-1d/parity-1/overlay.json` with the new bundle; its rows are the same 314.

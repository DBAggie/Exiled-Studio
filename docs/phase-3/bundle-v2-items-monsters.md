# Bundle format 2, item property and monster tables (P3-1c): evidence

Phase 3 plan, step 1c (`dev/active/phase-3-calc-slice/`), the last calc section of format 2. Maintainer workstation, Windows, 2026-11-11. Follows `bundle-v2-skills.md` (P3-1a) and `bundle-v2-passives.md` (P3-1b); the bundle summaries in this folder are replaced with P3-1c's. P3-1d (`monster-types.md`) then added `data/monster_types.json` and replaced them again with its bundles (every other file byte-identical to P3-1c's).

## Inputs
- Game data: PoE1 Steam build 26530219, PoE2 Steam build 26476662 (unchanged).
- Extracts: `loom-extractor extract` from clean commit `e43d8f6` (`ITEM_PROPERTY_TABLES` roots), in `local/determinism/<game>/run1`. P3-1b's extracts are kept in `local/determinism-<game>-p3-1b-backup`.
- Bundles: `loom-parity-cli bundle` from clean commit `9e8586a` (after the review fixes, which change no extractor code).

## Results
| Check | PoE1 | PoE2 |
|---|---|---|
| Extract, two runs | 571 files byte-identical; manifest `16c5c22c…` | 975 files byte-identical; manifest `ae7e1c2d…` |
| Extract files P3-1b's extract had, but the manifest | all byte-identical; added the 5 tables | all byte-identical; added the 5 tables and `WeaponClasses` (referenced by PoE2 `WeaponTypes`) |
| `bundle`, two runs | byte-identical; `bundle.json` `6099814e…` | byte-identical; `bundle.json` `944f2c0e…` |
| `bundle-check` | exit 0, no findings | exit 0, no findings |
| Files of P3-1b's bundle | 68 of 68 byte-identical | 611 of 611 byte-identical |
| Rows: weapon, armour, shield types, monster levels, game constants | 366, 481, 98, 100, 42 | 407, 1,280, 203, 100, 121 |
| Dangling references, dropped rows | 0 | 0 |
| Monster levels | 1 to 100, no gaps | 1 to 100, no gaps |
| Bundle size | 127.5 MB | 68.3 MB |
| `loom-extractor cross-check` | equal to the committed summary but `inputs` | same |

**Import is unchanged.** Corpus `build-round-trip` with the PoE1 bundle: two runs byte-identical, summary equal to P3-1b's except the tool commit. (A first run during the review fixes differed only in the `dirty` flag of the tool, because the tree was edited between its two runs; the re-run from the clean fix commit is the evidence.)

**Against PoB v2.67.2** (`src/Data/Misc.lua`, `src/Data/Bases/*.lua`, PoE1):
- `monsterEvasionTable`, `monsterAccuracyTable`, `monsterLifeTable`, `monsterLifeTable2` (`alt_life_1`), `monsterLifeTable3` (`alt_life_2`), `monsterAllyLifeTable`, `monsterAilmentThresholdTable`, `monsterPhysConversionMultiTable`: all 100 levels equal. `monsterDamageTable` and `monsterAllyDamageTable`: equal at `f32` precision (PoB prints the widened `f64`, `41.259998321533`; the bundle writes the `f32`'s shortest text, `41.26`).
- `data.gameConstants`: all 42 equal (`value / divisor`).
- `data.monsterConstants` (from `Metadata/Monsters/Monster.ot`, in the bundle since P3-1b): all 57 equal.
- Weapon bases: 360 compared by name, equal (damage, critical chance, attack rate as PoB rounds `1000 / Speed`, range); the 2 others share a name with a Royale base. Armour bases: 474 compared, equal (defence ranges, movement penalty).
- Monster armour: PoB does not read `DefaultMonsterStats.Armour` ("The table here is wrong") and derives it from `MonsterDamageReductionImprovement`; the bundle keeps both, and the calc decides with evidence (P3-11).

## Decisions recorded here
- `loom_data::bundle::properties`, required in format 2: weapon, armour, shield types by base item; per-level monster stats; game constants. The non-level monster defaults are the monster object template (P3-1b).
- One set of types for both games: PoE1 defence ranges, PoE2 one value (`min` = `max`); PoE2 reload time; PoE1 `alt_life_1` and conversion multiplier. Each game's shape is validated (`BUNDLE_GAME_SHAPE`), so a calc can rely on it.
- Validator: base item references; one row per base and constant; levels 1, 2, 3, ... without gaps (`BUNDLE_MONSTER_LEVELS`; the calc reads level n at row n - 1); value rules (`BUNDLE_VALUE_RANGE`).
- Format 2 is complete with this step: a later change to its sections is a new format.
- Not carried: PoE2 weapon classes, `DefaultMonsterStats`' experience-only, difficulty, and PoE2-only leech and damage columns.

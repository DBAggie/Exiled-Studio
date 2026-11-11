# Bundle format 2, skill tables (P3-1a): evidence

Phase 3 plan, step 1a (`dev/active/phase-3-calc-slice/`). Maintainer workstation, Windows, 2026-11-11.

## Inputs
- Game data: PoE1 Steam build 26530219, PoE2 Steam build 26476662 (unchanged since Spike A).
- Extracts: `loom-extractor extract` from clean commit `c9b58aa` (the `SKILL_TABLES` roots), in `local/determinism/<game>/run1`. The Phase 2 extracts are kept in `local/determinism-<game>-p2-backup`.
- Bundles: `loom-parity-cli bundle` from clean commit `13be856`.

## Results
| Check | PoE1 | PoE2 |
|---|---|---|
| Extract, two runs (`tools/extractor/scripts/check-determinism.sh`) | 557 files byte-identical; manifest `9a631317…` | 965 files byte-identical; manifest `dec05d47…` |
| Extract files the Phase 2 extract had, but the manifest | all byte-identical; added `EffectivenessCostConstants`, `GrantedEffectQualityStats`, `GrantedEffectStatSetsPerLevel` | same |
| Non-`ok` tables | none | `ItemVisualIdentity` (row-size mismatch, not a root; as since Spike A) |
| `bundle`, two runs | byte-identical; `bundle.json` `fa0b0cc9…` | byte-identical; `bundle.json` `97913df4…` |
| `bundle-check` | exit 0, no findings | exit 0, no findings |
| Format-1 files equal to the Phase 2 bundle's (`6c628cd6…`, `366342d2…`) | 53 of 53 | 596 of 596 |
| Skill rows: skill types, cost types, active skills, granted effects, stat sets, gems, gem effects, gem tags | 135, 13, 1,941, 12,829, 12,763, 847, 1,292, 54 | 290, 18, 1,148, 8,350, 8,284, 1,191, 1,194, 67 |
| Converter counts: dangling references, dropped stats or gems, unequal parallel lists, owner-less rows | 0 | 0 |
| Converter counts: level lists sorted | 1 (`IncaBossSummonAtziri`, a monster effect, levels 1, 3, 2) | 0 |
| Stat sets without level rows | 4 | 0 |
| Bundle size | 123.2 MB, of which skill tables 87.4 MB | 61.3 MB, of which 38.3 MB |
| `loom-extractor cross-check` against PoB's exports | equal to the committed summary but `inputs` | same |

Summaries: `bundle-poe1-26530219.json`, `bundle-poe2-26476662.json`, `bundle-check-*.json` (this folder); the extract manifests and cross-check summaries in `docs/spikes/spike-a/` are replaced with the new ones.

**Import is unchanged.** `build-round-trip` on the 21 corpus builds with the PoE1 format-2 bundle: two runs byte-identical; the summary equals `docs/phase-2/build-round-trip-poe1-26530219.json` except each build document's SHA-256. The only cause is the data version a document records: the bundle's `data_schema` went from 1 to 2. With the same bundle and `data_schema` set back to 1 in `bundle.json`, the summary equals the Phase 2 one except the tool commit.

**Spot check against PoB v2.67.2** (`src/Data/Skills/act_int.lua`, Fireball): base effectiveness 2.921, incremental effectiveness 0.0495, cast time 750 ms, level 20 mana cost 20, quality `active_skill_base_area_of_effect_radius` 150‰ (PoB 0.15), damage stats `spell_minimum_base_fire_damage` 0.8 and `spell_maximum_base_fire_damage` 1.2 with interpolation 3: the same. `data-parity` (P3-2) does this for every row the slice reads.

## Decisions recorded here
- Values are the game's, unscaled; the calc owns their reading (PoB divides crit chance by 100, adds 1 to damage effectiveness / 10000, interpolates by number). The interpolation number is kept as stored (1, 2, 3); P3-9a gives it a meaning with evidence.
- PoE2 `GrantedEffectsPerLevel.Reservation` is spirit: PoB's PoE2 exporter reads the column as `SpiritReservation` (`PathOfBuilding-PoE2@49e93925d:src/Export/Scripts/skills.lua:249`).
- `StatInterpolations` lists the float stats' numbers, then on some rows (PoE1 51,723 of 79,927; PoE2 11,707 of 41,558) one per additional stat; on those rows the extra numbers are 1 or 2.
- Not carried (no Phase 3 calc reads them): PoE1 Ruthless gem effects, level requirements, Vaal soul costs, PoE2 quality `AltStats` and `ApplyToStatSets`, PoE2 skill weapon requirements.
- Format 2 grows by whole sections in P3-1b and P3-1c, so a format-2 bundle made now does not load after those PRs (`BUNDLE_MISSING_FILE`); remake local bundles after each.
- `cargo deny check` fails on advisory RUSTSEC-2024-0370 (`proc-macro-error` unmaintained, through Tauri and GTK) on the base branch as well; this PR does not change `Cargo.lock`.

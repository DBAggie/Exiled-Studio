# Bundle format 2, character and passive tables (P3-1b): evidence

Phase 3 plan, step 1b (`dev/active/phase-3-calc-slice/`). Maintainer workstation, Windows, 2026-11-11. Follows `bundle-v2-skills.md` (P3-1a); the bundle summaries in this folder were replaced by P3-1c's (`bundle-v2-items-monsters.md`); the hashes below are P3-1b's.

## Inputs
- Game data: PoE1 Steam build 26530219, PoE2 Steam build 26476662 (unchanged).
- Extracts: `loom-extractor extract` from clean commit `678f902` (object templates, manifest format 5), in `local/determinism/<game>/run1`. P3-1a's extracts are kept in `local/determinism-<game>-p3-1a-backup`.
- Bundles: `loom-parity-cli bundle` from clean commit `db8b8c2` (after the review fixes, which change no extractor code).

## Results
| Check | PoE1 | PoE2 |
|---|---|---|
| Extract, two runs | 566 files byte-identical; manifest `d653477b…` | 969 files byte-identical; manifest `048c4764…` |
| Extract files P3-1a's extract had, but the manifest | all byte-identical; added 9 object templates | all byte-identical; added 4 |
| Object templates | `character.ot`, the 7 class templates (empty `Stats`, extend `Character`), `monster.ot` | `character.ot`, `character_login.ot`, `character_mobile.ot`, `monster.ot` (no class templates) |
| `bundle`, two runs | byte-identical; `bundle.json` `e2e97f09…` | byte-identical; `bundle.json` `e11fdb33…` (see note) |
| `bundle-check` | exit 0, no findings | exit 0, no findings |
| Files of P3-1a's bundle (format 1 and skill tables) | 61 of 61 byte-identical | 604 of 604 byte-identical |
| Rows: characters, ascendancies, templates, passives, mastery groups, mastery effects, tree groups | 7, 21, 9, 5,706, 61, 354, 797 | 12, 37, 4, 9,731, 81, 332, 1,623 |
| Dangling references, dropped stats | 0 | 0 |
| Template `Stats` entries kept as written (no stat or no integer) | 0 | 4 (`enable_weapon_sets = true`, `base_number_of_weapon_sets`, in `character` and `character_login`) |
| Tree: nodes, connections; every node a passive; orbits fit | 2,987, 3,359; yes; yes | 5,152, 6,070; yes; yes |
| Bundle size | 127.2 MB | 67.7 MB |
| `loom-extractor cross-check` | equal to the committed summary but `inputs` | same |

**Import is unchanged.** Corpus `build-round-trip` with the PoE1 bundle: two runs byte-identical, summary equal to P3-1a's except the tool commit (`data_schema` stays 2).

**Against PoB v2.67.2.** The 88 `Stats` entries of PoE1 `Metadata/Characters/Character.ot` equal PoB's `data.characterConstants` (`src/Data/Misc.lua`, exported from the same file); PoB's one extra key, `base_speed`, is from the `Pathfinding` block, which the bundle does not carry (movement speed is outside the slice). Class base life, mana, and attributes come from `Characters` (Marauder 38 / 34 / 32-14-14).

**Note on run 2 of the PoE2 bundle.** The run into `bundle-poe2-b` failed twice to rename its `.partial` folder into place (`Access is denied`, a handle held on that path by another process); the same conversion into `bundle-poe2-c` succeeded, and runs `a` and `c` are byte-identical.

## Decisions recorded here
- Object templates (`*.ot`) are read with the item-type grammar, widened for stat-ID keys (`%`, `+`), `Name { }`, and one-line script values (`{ HideWeapons(); }`, PoE2 client events). Only `Metadata/Characters/*.ot`, one folder below, and `Metadata/Monsters/Monster.ot` (for P3-1c) are read. Item type output is unchanged.
- The bundle keeps each template's `Stats` block unresolved, with its `extends`; the calc applies the chain. The validator refuses a chain that loops.
- The tree's identity is the game build in the manifest's provenance; the PoB tree version label it answers to is decided in P3-3 with evidence.
- Connections are undirected and stored once, as the graph file stores them. Orbit radii are not in the data read; a rule that needs distances brings them with their evidence.
- Not carried: other template blocks (movement speed), passive buffs and passive-granted skills, PoE1 Ruthless passive values, PoE2 attribute and weapon-set passive columns, alternate ascendancy and atlas graphs, the graph's unnamed group fields.

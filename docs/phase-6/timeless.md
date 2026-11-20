# P6-10a: timeless jewels

Plan decision 11: the timeless jewel tables come from our own extraction. The seed algorithm is ported as code and tested against PoB's lookup tables, and PoB's third-party tables are not shipped.

## Part 1: the extraction roots

**Done:** `loom-extractor` gains the PoE1 roots `TIMELESS_TABLES_POE1` (`tools/extractor/src/extract.rs`):
- `AlternatePassiveSkills`
- `AlternatePassiveAdditions`
- `AlternateTreeVersions`

Nothing the extract held referenced them. The extractor's 176 tests pass.

**Checked locally** (`local/extract/poe1/26530219-tl`, not shipped): the three tables read with the vendored schema.
- `AlternatePassiveSkills`: 381 rows. `AlternatePassiveAdditions`: 337 rows. `AlternateTreeVersions`: 13 rows.
- PoB v2.67.2's export of the same tables (`Data/TimelessJewelData/LegionPassives.lua`) has 718 entries, the same as 381 + 337.

**Why that extract is not the data:** the Steam install on this workstation is now build 26694821.
- The extractor does not read that build's passive tables (`PassiveSkills` and `PassiveSkillTrees` row sizes), so the run stops after writing the three tables.
- Build 26530219's files were not on this workstation when part 1 was written.
- A bundle that states 26530219 cannot carry another build's tables.

**Since 2026-11-19 the files are here:** the full depot of build 26530219 (depot 238961, manifest `2467965126630699088`) is in `local/b2/steam-26530219`. It was validated by DepotDownloader with `-validate`, which fetched the 10.7 GB the earlier download lacked. The stand-in `appmanifest_238960.acf` names the build.

## Q-P6-1: the seed algorithm's source (decided 2026-11-19)

**The licences, checked 2026-11-19.** The plan's two "MIT" projects are not MIT:

| Project | Licence (LICENSE file and GitHub) | Note |
|---|---|---|
| Vilsol/timeless-jewels | GPL-3.0 | Go |
| Nifth/TimelessCalc | GPL-3.0 | Its README says "MIT"; the LICENSE file governs |
| majorregal/TimelessEmulator | none | No licence: all rights reserved |

**Why the algorithm itself cannot be written from a description:**
- The game's placement uses a modified TinyMT32 generator.
- TimelessEmulator's README describes the steps, but not the generator's changes (state size, `next_state`, parameter use).
- Those exist only in code that is GPL or unlicensed.

So no implementation of the generator can ship under the project's MIT licence (ADR-000; the maintainer's rule of 2026-11-13: no GPL in shipped code).

**The decision (maintainer, 2026-11-19, on the recommendation):**
- Plan decision 11 is amended (`phase-6-mechanics-plan.md`): the timeless jewel results come from PoB's precomputed lookup tables (`Data/TimelessJewelData/`, in PoB's MIT repository), not from a ported generator.
- Our own extraction still supplies the tables those results index (`AlternatePassiveSkills`, `AlternatePassiveAdditions`, `AlternateTreeVersions`), checked by `data-parity` against `LegionPassives.lua`.
- The project stays MIT.

## The consistency check (local, 2026-11-19)

**Why:** the maintainer asked that the sources agree before PoB's tables are used.

**What was compared:**
- PoB v2.67.2's lookup tables, read as `readLUT` (`Modules/DataLegionLookUpTableHelper.lua`) and `PassiveSpec.lua` read them;
- Vilsol/timeless-jewels (Go 1.26.2);
- Nifth/TimelessCalc (TypeScript);
- majorregal/TimelessEmulator (C#).

**How:**
- Each tool was run locally through a small batch driver, on the same 150,280 lookups: the five original jewel types, 40 seeds each including the minimum and maximum.
- Nodes: for Glorious Vanity every node PoB tabulates (1,937); for the others every notable (455).
- Nothing from the tools is in the repository; the drivers and outputs are in `local/timeless-check/`.

**Compared fields:**
- every record's replacement and additions;
- for Glorious Vanity, the rolled values too. The other four types' rows all have fixed values (each row's minimum equals its maximum), and PoB stores no rolls for them.

**Compared by table row:**
- The game renamed some rows between the tools' data and PoB's, for example `karui_notable_add_faster_burn` is now `karui_notable_add_faster_ignite` at the same row.
- Compared by ID, those show as differences; compared by row, they agree.

**Result**, against PoB's tables:

| Tool | Glorious Vanity | Lethal Pride | Brutal Restraint | Militant Faith | Elegant Hubris |
|---|---|---|---|---|---|
| Vilsol | 77,480 of 77,480 | 18,160 of 18,160 | 18,160 of 18,160 | 18,160 of 18,160 | 18,160 of 18,160 |
| TimelessCalc | 77,240 of 77,240 | 18,080 of 18,080 | 18,080 of 18,080 | 18,080 of 18,080 | 18,080 of 18,080 |
| TimelessEmulator | 61,573 of 67,400 | 15,640 of 15,640 | 15,640 of 15,640 | 15,640 of 15,640 | 15,640 of 15,640 |

- **Coverage:** TimelessCalc and TimelessEmulator lack some nodes (older passive trees); those lookups were skipped, not counted.
- **Where the tools disagree outside PoB's tables:**
  - TimelessEmulator's Glorious Vanity rolls (5,826) and its Lethal Pride and Elegant Hubris rolls are its own: the other three sources agree.
  - TimelessCalc alone adds "+5 Devotion" to a Militant Faith notable it replaces (3,618 records); PoB, Vilsol, and TimelessEmulator do not.
- **Conclusion:** PoB's tables and Vilsol's implementation agree on every lookup PoB holds.

**Not checked yet:** Heroic Tragedy and the five Abyss jewels (PoB has tables; the reader here handles the five original types).

## What is left

1. **The data run** (maintainer): extract with these roots from build 26530219's files, now on this workstation. Then the bundle's `poe1/timeless.json`, its validator, and the `data-parity` table against `LegionPassives.lua`.
2. **PoB's lookup tables into the bundle:** a compact form of the five original types' results (170 MB of zip parts in PoB, the Abyss types included), read by the engine through the bundle (ADR-016: no file I/O in core crates), with a `data-parity` table against PoB's own.
3. **The engine:** the replaced and added nodes in a Timeless jewel's radius, and the replacement keystones (P6-11ao's `TimelessKeystone` reason).

Until then, a timeless jewel stays the reason it is today: its stats are unmapped, and the nodes it changes are not modelled. (Done for the six types that are not Abyss jewels: "Part 3".)

## Part 2: the tables in the bundle (2026-11-19)

The plan is `dev/active/phase-6-mechanics/p6-10a-timeless-plan.md`, decided by the maintainer: all 11 types ship, loaded only when needed.

**The extraction:** `local/extract/poe1/26530219-p6`, from build 26530219's full depot (`local/b2/steam-26530219`, validated by DepotDownloader 2026-11-19), read through `local/b2/steam-26530219-full` (a junction with the stand-in `appmanifest_238960.acf`).
- The game index SHA-256 is the evidence bundles' (`d23c7899…`).
- 109 tables, all `Ok`.
- `AlternatePassiveSkills` has 381 rows, `AlternatePassiveAdditions` 337, `AlternateTreeVersions` 12 (rows 1 to 11 are PoB's jewel types 1 to 11).

**The bundle files** (`loom_data::bundle::timeless`):
- **`poe1/timeless.json`** (written by `bundle`, `tools/parity-cli/src/timeless.rs`): the three extracted tables.
  - PoB's `Export/spec.lua` names the columns the schema leaves unnamed (stats 3 to 6, the conqueror index and version, the tree versions' replacement flags), so those are read by position, after the named columns are checked at theirs.
- **`poe1/timeless_luts.json`** (written by the new `loom-parity-cli timeless-pack <bundle> <pob-src> <pob-version> --out <dir>`):
  - PoB's `NodeIndexMapping.lua` (1,937 nodes, 454 notables, the local-to-global IDs);
  - the jewel types and seed ranges from `Modules/Data.lua`;
  - each results file's SHA-256.
- **`poe1/timeless/<Type>.bin`**: each `Data/TimelessJewelData/<Type>.zip` (or its parts) inflated, byte for byte: 466 MB for the 11 types.
- **Loading:** `Bundle::load` reads the two JSON files and never the `.bin` files. The app will read one type's file when a build socketes that jewel (plan decision T2).
- **Reader:** `loom_data::bundle::timeless::read` ports PoB's `readLUT` for the six non-Abyss types, with unit tests. Glorious Vanity's layout was checked on PoB's file: its length is the node count times the seeds plus every node's size, and each node's size is the sum of its seeds' lengths (all 1,937).

**The validator** (`BUNDLE_TIMELESS`):
- **Extracted tables:** their stats exist, and each row number appears once.
- **The mapping needs the extracted tables.** Each jewel's file is in the manifest under `poe1/timeless/` with the SHA-256 the mapping names, and each non-Abyss file has its layout's length.
- **Node indices** are unique and below the node count.

**PoB's export, format 15** (`tools/oracle/data/pob_data.lua`), adds `timeless`:
- `LegionPassives.lua`'s rows (ID and stat ranges), the node index mapping, and the jewel types and seed ranges;
- each zip's Adler-32 trailer, which is the checksum of its inflated bytes, read without inflating.

**Data parity, table `timeless`** (`tools/parity-cli/src/data_parity/timeless.rs`):
- **What it compares:** the extracted rows against PoB's legion rows, the mapping against PoB's, and each shipped `.bin`'s Adler-32 (computed by `data-parity` from the file) against PoB's trailer.
- **On the evidence bundle `b-tl`:** 7,041 rows, 80 differences, all categorized; all 11 checksums equal.
  - **6 IDs:** PoB's exporter rewrites six game IDs it treats as mislabelled (its `datErrors`, `legionPassives.lua` 7 to 55). Rule `timeless-pob-dat-errors`, `pob-derivation`.
  - **74 stat range values:** PoB stores ranges in the unit their descriptions show (per-myriad as a percent, per minute per second, milliseconds as seconds, a "reduced" stat as its positive magnitude). Every one was checked to be such a conversion. Rule `timeless-display-units`, `representation`.
- `uncategorized` stays 0.

**Evidence bundle** `local/p6/packs/b-tl`:
- `b-cl` plus `poe1/timeless.json`, then `timeless-pack` with PoB v2.67.2.
- 609 MB on disk. `bundle-check` reports no findings; it is local and unsigned.

**Calculation:** no value or state changed on `b-tl` (public 0 unexpected, corpus 0 unexpected, against `b-cl`): the engine does not read the tables yet.

**Next:** the engine (plan steps 2 to 4):
- the app reading a type's file on demand;
- the jewel's type, seed, and conqueror from its lines;
- the replaced and added nodes in its radius, and the conqueror's keystone.

## Part 3: the engine (2026-11-19)

Plan steps 2 and 3 (`dev/active/phase-6-mechanics/p6-10a-timeless-plan.md`), for the six types that are not Abyss jewels. The nodes a Timeless jewel conquers now carry the conquest's stats, as PoB's `PassiveSpec.lua` 1102 to 1376 gives them.

**Where it is:**
- `loom_pipeline::timeless` (core, no I/O):
  - **`socketed`:** each Timeless jewel in an allocated socket, with its type, seed, conqueror, and version from its stats (`local_unique_jewel_alternate_tree_version`, `_seed`, `_keystone`, `_internal_revision`) and its `Radius:` line's ring.
  - **`conquests`:** every node in the ring that is a keystone, notable, or normal passive of the tree itself. Class starts, sockets, ascendancy nodes, masteries, and cluster jewels' generated nodes are left out; PoB's `nodesInRadius` holds none of them.
  - **`needed`:** the types a build's jewels need, so the host reads only those results files.
- **The seed is the rolled value:** read from the line values PoB's ranges settle (`Rolls`, the import's item line values), never a range's minimum.
  - Elegant Hubris's line shows the seed times 20 (the bundle's `seed_divisor`).
  - The version is the range's minimum: the description's "1 and above" reads as 1.
- **The data:** `TimelessData` holds the extracted tables, the mapping, and the results bytes the host read.
  - `loom_app::GameData::timeless_results` reads a type's file on demand and checks its size and SHA-256 against the manifest (plan decision T2).
  - `loom-parity-cli calc-run` preloads the six types.
  - Without the data, each node the jewel conquers is a reason (`timeless-results`).
- **`loom_calc::input::ResolvedPassives.conquered`:** each conquered node's `Conquest`, whether it replaces the node's own stats, and the stats it gives. `collect` ingests them in place of, or beside, the node's lines.
  - P6-11ao's `PassiveState::TimelessKeystone` is gone: a keystone in the radius becomes the conqueror's keystone row, its stats applied (or a reason when the stat is unmapped).

**The rules, as PoB's:**
- **A keystone:** the row of the jewel type with the conqueror's index and version that replaces keystones (passive type 4).
- **A notable:** the results file (`loom_data::bundle::timeless::read`, PoB's `readLUT`): a replacement takes its row's stats (Glorious Vanity's rolls, else the row's fixed values), and an addition adds its row's.
- **A Glorious Vanity small passive:** the results file, as a notable.
- **Another type's small passive:** by the tree version's flags, the one replacement row, or the one addition row, of passive type attribute (1) when the node is named Strength, Dexterity, or Intelligence, else normal (2).
- **Glorious Vanity's additions:** the node becomes Might of the Vaal (row 76) or Legacy of the Vaal (row 77) by PoB's bias (at least half the IDs at most 21 is Might), each addition's rolls summed.

**Two changes made on the way:**
- **The parser:** the selected lines and the lines of variants not selected each form their own runs, and each runs on past the other's.
  - A Timeless jewel with variants (Cyclone's Lethal Pride) has its conqueror line and its "Passives in radius are Conquered by the Karui" line with the other conquerors' lines between them. Before, those lines broke the run, so the two-line description never resolved and the jewel was never read.
  - `ItemBody::check_links` follows the same rule: a continuation follows its line among the lines of its own selection.
  - `build-round-trip` on b-tl: the public fixtures' unresolved lines went from 315 to 279 (18 two-line descriptions now resolve); the corpus's went from 372 to 370. Nothing else changed.
- **The mapping:**
  - `damage_taken_goes_to_life_over_4_seconds_%` is life recoup (`Poe1Quantity::LifeRecoup`), read by `enemy_hit` for PoB's `anyRecoup` as mana recoup is.
  - `base_avoid_bleed_%` and `base_avoid_poison_%` are `NoEffect` (`ailment-on-self`).
  - The jewel's own stats are `NoEffect` prefixes: they act through the conquered nodes.

**Fixtures:** four new mechanic fixtures. Each is a level 90 Witch casting Ice Nova, with the jewel in socket 41263 and the passives of its Large radius that connect to the path allocated.
- The fixtures:
  - `timeless-glorious-vanity-ice-nova-witch.xml` (seed 1200);
  - `timeless-brutal-restraint-ice-nova-witch.xml`;
  - `timeless-elegant-hubris-ice-nova-witch.xml`;
  - `timeless-lethal-pride-ice-nova-witch.xml`.
- Each matches PoB in all 77 computed fields.
- Militant Faith and Heroic Tragedy fixtures were built the same way, with no value differences from PoB. They are not added, because their conquests give stats still unmapped: devotion (`base_devotion` and the devotion thresholds), and ward (`ward_+%`, `ward_from_armour_item_+%`, `restore_ward_on_block_%`).
- The Glorious Vanity seed was picked so its conquests carry no unmapped stat (seed 2365 gives one, `curse_skill_effect_duration_+%`).

**Checked against PoB's own conquered nodes:** a local PoB script listing each allocated node's `conqueredBy` and lines.
- **Kinetic Fusillade:** PoB conquers 18 allocated nodes; the 12 that gain Strength gain it here too, the same +36 in all.
- **Cyclone:** PoB conquers 23 allocated nodes; the 15 that gain Strength gain it here too, the same +46 in all.

**Evidence** (b-tl, against P6-10a part 2's run):
- **Public:** 0 unexpected; 231 values moved, 155 toward PoB, 75 away, 4 now equal.
- **Corpus:** 86 moved, 30 toward, 56 away, 1 now equal.
- **The away moves** are in builds whose other gaps are larger (Essence Drain, Penance Brand, RF Chieftain, Flicker Strike, Lacerate): their attribute nodes are replaced or added to as PoB's are, and the other gaps keep their totals apart.
- **Found: tattoos.** Kinetic Fusillade and Cyclone are farther from PoB in Strength because their tattoos are not modelled. A tattoo (PoB's `<Override>`) replaces a node's lines: Cyclone has 15, several on +10 Strength nodes. That gap was hidden while their Lethal Pride went unread. Tattoos are a task of their own (tasks file, P6-11).

**Left:**
- the Abyss jewels (PoB's `DataAbyssJewelLookUpTableHelper.lua`; done in "Part 4");
- the conquests' unmapped stats (devotion, ward);
- tattoos.

## Part 4: the Abyss jewels (2026-11-19)

The five Abyss jewels (types 7 to 11: Tecrod, Ulaman, Kurgal, Amanamu, Zorath) change the nodes their results list.

**The reader:** `loom_data::bundle::timeless::abyss` ports PoB's `Modules/DataAbyssJewelLookUpTableHelper.lua`.
- **The header:** four format letters, version 1, the jewel type, then the first seed, last seed, and step as `u16`s.
- **`ABYS` (`read_socket`):** Tecrod, Ulaman, Kurgal, and Amanamu. Each of 21 sockets has one record per seed: the nodes changed from that socket.
- **`ABYN` (`read_path`):** Zorath. Each of 2,143 nodes has one change per seed. Then the `ASCS` section: for each of 21 ascendancies, the nodes each seed picks.
- **A change:** its parts, each a kind (1 replaces, 2 adds), a local ID made global through the mapping, and signed rolls.
- The records vary in length, so a block is found by skipping the records before it, as PoB does.
- **Tests:**
  - Unit tests on PoB's own examples (`spec/System/TestAbyssTimelessJewel_spec.lua`).
  - A maintainer-run test on the shipped files (`crates/loom-data/tests/timeless_abyss_files.rs`): PoB's Kurgal example (seed 533, socket 61419) is the real file's record, part for part. Zorath's read of a path and every ascendancy takes 31 ms in a release build.

**The engine** (`loom_pipeline::timeless`):
- **Which jewels:** an Abyss jewel has no radius. Its type, seed, and conqueror come from the same `local_unique_jewel_alternate_tree_*` stats ("Subjugating # souls in the thrall of Kurgal", "Binding # souls to phylacteries to sustain Zorath").
- **Tecrod, Ulaman, Kurgal, Amanamu:** the nodes their socket's record lists for the seed.
- **Zorath:**
  - the nodes of the allocated shortest path from its socket to the class start (PoB's `GetShortestPathToClassStart`, breadth-first, not through a mastery or another start);
  - and in every ascendancy, the node the seed picks.
  - Between paths of the same length, PoB follows its tree's link order and the engine takes neighbours in ascending order. The two can differ in that case.
- **Applying a change:** each part in order.
  - A replacement takes its row's stats with the part's rolls as the stats' raw values, and wipes what came before.
  - An addition adds its row's stats.
  - Another kind is a reason. Sockets are never changed.
- **Precedence:** a node a radius jewel conquers is that jewel's, as in PoB, which reads the Abyss jewels first.
- **Reasons:** an Abyss jewel whose lines do not read, or whose results are missing, is the reason on its socket, since the nodes it changes are not known without them.
- **Loading:** `needed` now names the Abyss types too, so the app reads their files on demand. `calc-run` preloads all 11.

**Fixtures:** five mechanic fixtures on the Timeless fixtures' tree (socket 41263, its Large radius allocated), each checked against PoB's own conquered nodes (the local PoB script):
- `timeless-abyss-tecrod-ice-nova-witch.xml`: seed 597, 8 allocated nodes changed;
- `timeless-abyss-ulaman-ice-nova-witch.xml`: seed 3860, 19;
- `timeless-abyss-kurgal-ice-nova-witch.xml`: seed 521, 16;
- `timeless-abyss-amanamu-ice-nova-witch.xml`: seed 7805, 15;
- `timeless-abyss-zorath-ice-nova-witch.xml`: seed 7300, the 7 path nodes it changes.

The seeds were picked so their changes carry no unmapped stat. Many Abyss additions are still unmapped: added damage with a weapon type, minion chances, avoid shock.

**Results:** 383 fields match PoB, and two differ, both registered.
- **DIV-0294 and DIV-0295** (`pob-bug`), Amanamu's and Ulaman's mana regeneration, 12.5 against PoB's 12.6 and 13.0 against 13.1.
- The results hold `base_mana_regeneration_rate_per_minute` rolls as whole per-minute values (51; 46 and 33). PoB writes each into the node's text as `round(value / 60, 1)` per second (`PassiveSpec.lua` 1206) and parses that text, so 0.85 becomes 0.9.
- The engine applies the per-minute value, as it does every per-minute stat (`docs/phase-6/recovery.md`, "A display limit").
- **Test:** `loom_pipeline::timeless::tests::abyss_parts_should_apply_in_order_with_their_raw_rolls`.

**Evidence** (b-tl, against part 3's run):
- **Public:** 0 unexpected, 2 more expected (the entries above). No value moved: no public or corpus build has an Abyss jewel.
- **Corpus:** no value moved.

**Left:**
- the conquests' unmapped stats (devotion, ward, the Abyss additions);
- tattoos.

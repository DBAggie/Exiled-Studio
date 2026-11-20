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

Until then, a timeless jewel stays the reason it is today: its stats are unmapped, and the nodes it changes are not modelled.

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

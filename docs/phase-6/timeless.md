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

# P6-10a: Timeless jewels in the bundle and the engine (plan)

Status: **decided by the maintainer 2026-11-19:**
- **T1:** all 11 jewel types' results ship in the bundle (about 178 MB to download, 466 MB on disk; the maintainer confirmed after seeing those numbers).
- **T2:** load only when needed.
- **T3:** as proposed.

**The engine order:**
1. The six non-Abyss types (the five original types and Heroic Tragedy).
2. The five Abyss jewels, as their own step. PoB reads them through another module (`DataAbyssJewelLookUpTableHelper.lua`), per socket, and Zorath's result depends on the allocated path.

Context: Q-P6-1 was decided on 2026-11-19. The results come from PoB's lookup tables, and the tables they index come from our own extraction (`docs/phase-6/timeless.md`). This plan says how the tables reach the bundle and the engine.

## The data

| Part | Source | Size |
|---|---|---|
| Alternate passives, additions, tree versions | our extraction (`AlternatePassiveSkills`, `AlternatePassiveAdditions`, `AlternateTreeVersions`, build 26530219) | small (JSON) |
| The node index mapping and local-to-global ID tables | PoB's `NodeIndexMapping.lua` (MIT) | ~60 KB (JSON) |
| The results, per jewel type | PoB's `Data/TimelessJewelData/<Type>.zip` (MIT), inflated | all 11 types: 466 MB raw, ~178 MB compressed (the five original types 66 MB raw) |

The results are kept in PoB's own byte layout, as `readLUT` reads it. The engine's reader is a port of `readLUT` (`Modules/DataLegionLookUpTableHelper.lua`), which is MIT.

## Decisions for the maintainer

**T1. Ship the results in the data bundle.**
- **Files:** `poe1/timeless/<type>.bin`, one per jewel type, in PoB's inflated layout, compressed by the bundle's zip.
- **Size:** the five original types about +30 MB to download and +66 MB on disk; all 11 about +178 MB and +466 MB.
- **Decided:** all 11 types. Their sizes inflated: Glorious Vanity 51.7 MB; Lethal Pride, Brutal Restraint, Militant Faith, Elegant Hubris, Heroic Tragedy about 3.6 MB each; the Abyss jewels 73 to 103 MB each.

**T2. Load the results only when a build needs them.**
- **Bundle loading:** `Bundle::load` keeps reading every file but these.
- **The app:** when a build has a socketed Timeless jewel, the app reads that type's file through the bundle source and passes the bytes to the calculation as an input, beside the bundle.
- **The core crates** never read files (ADR-016), and the startup budget is untouched.
- **Alternative:** load all five at startup. That is simpler, but adds about 66 MB of memory and its read to startup.

**T3. Check the shipped tables.**
- **Byte equality:** a `data-parity` table `timeless` checks each shipped file against PoB's inflated file (equal bytes) and the mapping tables against PoB's.
- **Cross-check:** the local consistency check (PoB against Vilsol) stays the evidence that the tables are right.

## Engine steps after T1 to T3

1. **Bundle and validation:**
   - `poe1/timeless.json`: the extracted tables plus the mapping;
   - the `.bin` files, with the validator (file lengths match the mapping's node counts and seed ranges);
   - the `data-parity` table;
   - the synthetic test bundle's small tables.
2. **The jewel:** read a socketed Timeless jewel's type, seed, and conqueror from its lines.
   - The stats `local_unique_jewel_alternate_tree_seed`, `_keystone`, `_version` stop being unmapped and become the jewel's data.
   - The base name is the fallback when a line does not resolve (P6-11ao's case).
3. **The nodes in its radius** (PoB's `PassiveSpec.lua` 1230 to 1390):
   - a replaced notable or small passive takes the alternate passive's stats, with Glorious Vanity's rolls from the table;
   - an addition adds its stats;
   - a replaced keystone becomes the conqueror's keystone.

   P6-11ao's `TimelessKeystone` reason goes; the replacement's lines go through the mapping as other passives' do.
4. **Fixtures:** mechanic fixtures with each jewel type run through PoB, and the public and corpus evidence.

Each step is one commit with its evidence, as Phase 6's steps.

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
- Build 26530219's files, which the maintainer downloaded with DepotDownloader for the B-2 data run (`docs/ui/b2-data/README.md`), are not on this workstation.
- A bundle that states 26530219 cannot carry another build's tables.

## What is left (maintainer)

1. **The data run:** extract with these roots from build 26530219's files, or from a build the extractor reads. Then the bundle's `poe1/timeless.json`, its validator, and the `data-parity` table against `LegionPassives.lua`.
2. **The seed algorithm's source** (a decision, Q-P6-1):
   - The game's timeless jewel placement uses a modified TinyMT32 generator. It is known only from community reverse engineering, not from the game files.
   - PoB does not compute it: it ships precomputed lookup tables (`Data/TimelessJewelData/*.zip`), which the plan keeps out.
   - The best-known implementation, Vilsol's timeless-jewels, is GPL-3.0, which the project does not allow in shipped code. MIT-listed projects exist (Nifth/TimelessCalc, majorregal/TimelessEmulator).
   - Porting one of them, with its license and attribution, or deriving the algorithm another way, needs the maintainer's decision. The port would then be tested against PoB's lookup tables (local only).

Until both are done, a timeless jewel stays the reason it is today: its stats are unmapped, and the nodes it changes are not modelled.

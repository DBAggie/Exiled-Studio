Last Updated: 2026-11-24

# Maintainer follow-ups

Items an agent cannot finish without you. The open-items session (`open-items-plan.md`) adds to this list as it finds more (class B and C in `open-items-inventory.md`). Each item says what is needed and where it is written up.

## Gates
1. **Phase 5 gate** (`docs/phase-5/phase-5-report.md`, "For the maintainer"):
   - Gate 6: run `release.yml` by hand on `main` and record it in `docs/phase-5/packaging.md`, or accept the tagged 0.9 runs (`v0.9.0-1`: 43822600359; `v0.9.0`: 43866901437) as the record.
   - Gate 9: accept or refuse the NSIS size exception; measure tree paint and pan and zoom fps in the release app; say whether peak memory includes WebView2.
   - Copy the private signing keys offline twice (`docs/release/signing.md`, "First keys").
   - After the next release: a real update from 0.9 (`docs/phase-5/updates.md`, "Not tested here").
2. **Passive tree gate** (`docs/tree/tree-report.md`, "Maintainer-run", items 1 to 5): the signed bundle with the 3_25 to 3_28 packs; `data-parity` per pack; the private corpus run on the packs; T3's three cluster items (gate 3); `TREES="3_25 3_26 3_27 3_28" tools/tree/parity.sh <bundle>` on the signed bundle.
3. **Phase 6 gate** (`docs/phase-6/phase-6-report.md`, "The close-out (2026-11-24)"). Public: 0 unexpected, 38,757 `Complete`. Corpus: no `Complete` field yet.
4. When those gates pass: move `dev/active/passive-tree-and-phase-5/` and `dev/active/phase-6-mechanics/` to `dev/completed/`.

## Data
The evidence bundles on the workstation (`local/p6/packs/b-ct` and kin) are from build 26530219. They already carry data schema 5 with the cluster, timeless, and tattoo tables, so the agent works on those locally. What stays yours:
- **Shipping** the schema 5 tables (cluster jewels, timeless, tattoos, `poe1/flasks.json`, `poe1/minions.json` with `object_stats`) in the signed bundle (`dev/active/phase-6-mechanics/phase-6-mechanics-tasks.md`, the "Maintainer:" lines).
- **A data run on the installed game, build 26694821**, for what no bundle has: `VaalSouls` (soul costs), Skitterbots' minion data (S0: `SummonSkitterbots` and `Skitterbots` are the only class B gems, 5 public and 7 corpus builds), the Holy Relic's Nova (skill level requirements), and any `no-template` line from a later league that the session finds. Keep one game build per bundle. Add the build's row to `tools/knowledge-mcp/patches.json` (patch and league).
- **The 3_29 data gap of four ascendancy auras** (`docs/phase-6/stat-mop-up.md`, "P6-11hc").
- **A mine Assassin's remaining gaps** (`ethereal-knives-mines-assassin.xml`; Phase 6 tasks file): the session checks its 16 cluster nodes on `b-ct` first, and lists what remains here.
- **Fated uniques without flavour text** (28; `dev/active/ux-improvements/ux-improvements-notes.md`, "Maintainer tasks").

## In-game checks (game truth)
- The game-truth list (class C in `open-items-inventory.md`, 68 stats, 69 C reasons with the older wordings above): 68 lines PoB applies nothing for that change a slice field in the game (`docs/phase-6/phase-6-report.md`, "The maintainer's game-truth list"). A check in the game for each lets an agent map it with a `game-truth` register entry.
- The four game-truth items in the Phase 6 tasks file: Focal Point's "more Damage taken from other Enemies near your Marked Enemy", Mageblood's "Magic Utility Flasks cannot be Used", "Take no Extra Damage from Critical Strikes if you have taken a Critical Strike Recently", "increased Damage taken while Leeching".
- Vaal Summon Skeletons' minion level, DIV-0111 to DIV-0113 and DIV-0174: the game's data says 68 at gem level 20; PoB uses 70 (`docs/phase-6/phase-6-report.md`, "The register").
- Q12 (`docs/phase-3/attack-hits.md`): where global and skill attack speed enter. Open, not blocking.
- Any new difference from PoB the session finds without Q11 or `pob-bug` evidence (added by the session).

## Decisions
- **PoE2:** calculations and `.build` import are the rest of "Phase 6+". Not planned yet; a plan needs your go-ahead.
- **Deferred on purpose** (`docs/plan/ACTION-PLAN.md`): the public product name, a WASM addon runtime, hosted sharing, an in-app Claude panel, macOS as a release target.
- **Releases:** merging `dev` into `experimental` and `main` for the next release.
- Any calc result v2 variant an open item turns out to need (added by the session if found).
- **Older item wordings (S0, the largest class C reason).** Most `unresolved-line` `no-template` lines (public 1,259 fields in 17 builds, 67 of them blocked by it alone; corpus 757 in 10) are item text in an older wording than build 26530219's descriptions: "+1 to Maximum Endurance Charge" (the game now says "Charges"), pre-3.15 flask lines ("Immunity to Freeze and Chill during Flask effect", "Removes Bleeding on use", "Flask Effect does not Queue"), "chance to Dodge Spell Hits". PoB reads them with its own parser. Reading them stat-ID first needs a reviewed rewording table for items, as the tree packs have (`docs/tree/tree-packs.md`), and a new `Adapted` kind (for example `reworded`) in build v1 and calc result v1: an ADR-015 amendment and an additive v1 change, both yours to decide. Until then they stay reasons. The rest are note or flavour lines in items ("Vendor Recipe...", "simple ethos: ..."), which PoB reads into nothing; they stay reasons by design. Lines: `local/p6/tools/line_texts.py` and `template_check.py` (inventory, `dev/active/open-items/open-items-inventory.md`).
- **The oracle CI jobs' time limit.** It went from 45 to 90 to 150 minutes during Phase 6, each time marked "maintainer to review". The last green runs took 80 to 113 minutes, and every new fixture adds to them. The options: raise the limit again, split the fixtures across more oracle jobs, or run part of the fixtures on a schedule rather than on every PR.
- **Performance:** a profile before Phase 7. The bench's `calculate` rose 119.6% over Phase 6 and was re-baselined (`docs/phase-5/performance.md`, "Re-baselined 2026-11-20").
- **SmartScreen** (a note only): the installers are not code signed (`docs/phase-5/phase-5-report.md`, item 6).

## Planned for the session, may not be reached
The largest steps, deferred by you on 2026-11-24, are stretch work for the open-items session. The session removes each from this list when it is done:
- triggers and Spellslinger (plan S3);
- Blood and Sand, and PoB's full DPS pass (S4);
- the defences: ward's other lines, EHP's other pools, damage over time EHP, conversion smoothing in maximum hit, degeneration, a dual wielder's ailments and leech (S7).

## Housekeeping
- PR #167's oracle jobs were still running when it merged (2026-11-24 22:15 UTC). Check that the oracle jobs of `dev`'s run at `6deeb5da` passed.

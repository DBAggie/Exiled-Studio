Last Updated: 2026-11-24

# Maintainer follow-ups

Items an agent cannot finish without you. The open-items session (`open-items-plan.md`) adds to this list as it finds more (class B and C in `open-items-inventory.md`). Each item says what is needed and where it is written up.

## Gates
1. **Phase 5 gate** (`docs/phase-5/phase-5-report.md`, "For the maintainer"):
   - Gate 6: run `release.yml` by hand on `main` and record it in `docs/phase-5/packaging.md`, or accept the tagged 0.9 runs (`v0.9.0-1`: 43822600359; `v0.9.0`: 43866901437) as the record.
   - Gate 9: the NSIS size exception **accepted 2026-11-24**; tree paint, pan and zoom fps, and peak memory with WebView2: planned (`docs/phase-5/maintainer-tests.md`).
   - Copy the private signing keys offline twice (`docs/release/signing.md`, "First keys").
   - After the next release: a real update from 0.9 (`docs/phase-5/updates.md`, "Not tested here"); planned with the keyboard and screen reader checks (`docs/phase-5/maintainer-tests.md`). The passive tree packs check is deferred (maintainer 2026-11-24).
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
- **Vaal Summon Skeletons' full DPS (S1, cause not found):** PoB's `FullDPS` for `minion-vaal-summon-skeletons-witch.xml` is 416.996, while its minion's `TotalDPS` is 115.298 and the count 1 (`calcFullDPS`, `Calcs.lua` 204 to 240, adds `TotalDPS x count`). The engine gives 103.72 (the minion's DPS at the game's minion level, DIV-0124). The factor 3.617 has no source found yet; the full DPS pass may use another of the skill's minions. The field keeps `full-dps-vaal` for a Vaal skill that summons minions. Not game truth: an agent can trace it with an instrumented PoB copy later.

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

Not reached in the session of 2026-11-24 either (agent work, no decision needed; the next session can continue them from `open-items-tasks.md`):
- **S5:** mine, trap, and totem costs per second; warcry power and the cries that scale with it; General's Cry's mirage warriors; Sacred Wisps.
- **S6:** the minion's own damage over time; spectres (S0 did not check `b-ct`'s monster data for them yet).
- **S9:** the resolver and item gaps of class A (`ambiguous`, `no-variant-fits`, `only-shadowed`, scaled and excluded lines, the item kinds, tree lines, cluster and timeless nodes).
- **The rest of S1:** the other `pob-skill-data` skills (Cyclone, Fire Trap, Thrown Shield, the trigger builds), `socket-colour-quality`, `sigil-of-power-stages`, `skill-chill-effect`, `minion-buffs`.
- **The rest of S2:** the buffs and auras with global rows (`docs/phase-6/gems.md`, "Open items S2"), Cold Snap and Unbound Ailments with their chill effect.
- **The rest of S8:** the close-out's closed reasons (104 stats at S0; `open-items-inventory.md` lists each with its step), brittle (its enemy-side crit reaches minions), the minion inputs, Feeding Frenzy, Elemental Equilibrium's exposure.

- **Static Strike part 1 (S1, not reached):** its function and beam damage act only in part 2, but its zap frequency stats (`static_strike_base_zap_frequency_ms`, `static_strike_zap_speed_+%`) are unmapped; vetting part 1 needs them `NoEffect` outside part 2. Three Smite builds' `FullDPS` wait on it.

## Found in the maintainer's application test (2026-11-24, agent work)
- **Getting game data is hard for a new user.** The start page's **Add game data…** link (`ui/src/App.tsx` 726 to 732) opens the unsigned development folder picker (`chooseBundle`); the `APP_NO_DATA` message says to choose "the game data folder"; the signed install (**Install game data from a file…**) is only on the About page; and no screen offers to download the signed data from the feed. Wanted: the link and the message lead to the signed install, and a first-start "Download game data" action fetches the feed's pack. **Maintainer decision (2026-11-24): game data is handled completely automatically, not by the user.** Plan: on first start and whenever the installed data is missing or older than the feed's, the application fetches the signed pack from its channel's feed and installs it (signature checked, as `loom_tauri::data_updates` does on request today); strict offline refuses, as ADR-011 requires, and the start page then offers one "Download game data" action; the start page's link and the `APP_NO_DATA` message no longer lead to the development folder picker, which stays only on the About page for development.

## Housekeeping
- PR #167's oracle jobs were still running when it merged (2026-11-24 22:15 UTC). Check that the oracle jobs of `dev`'s run at `6deeb5da` passed (still running at 23:40 UTC, 85 minutes in; about 131 expected at 531 fixtures).
- **The oracle jobs' time:** this PR adds 4 fixtures (535 in all); at about 14.8 s each the Windows job should take about 132 minutes of its 150.

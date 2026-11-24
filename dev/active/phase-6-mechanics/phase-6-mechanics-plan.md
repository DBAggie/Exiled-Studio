Last Updated: 2026-11-17 (P6-0: plan, plan-reviewer findings applied)

# Phase 6: PoE1 mechanics expansion (plan)

## Summary
This plan covers the PoE1 part of `docs/plan/ACTION-PLAN.md`'s "Phase 6+: Expansion": "PoE1 mechanics expansion (each promoted through corpus and oracle gates); timeless jewels and alternate trees". PoE2 calculations and PoE2 `.build` import are not in it.

**The maintainer's request (2026-11-17):** "do phase 6 in full without stopping. We can do gate testing after. Include the stat mop up as well." The goal behind it: every gem, jewel, passive point, ascendancy point, and gear stat the engine can model updates the application's calculations. The open gates of the passive tree editor and Phase 5 (maintainer-run items) are tested afterwards, by the maintainer's word; the CLAUDE.md rule "do not start work for a later step before the earlier gate passes" is set aside for this phase by that instruction.

**Gate (proposed, run after the work, as the maintainer asked):** every build of the public fixtures and the private corpus calculates every field it can, against PoB v2.67.2:
1. every `Complete` field matches PoB or has a registered, evidenced divergence (as Phase 3);
2. no build leaves a field `Unsupported` for a mechanic this plan lists as done; the remaining `Unsupported` and `Partial` reasons are each in the report's closed list with why;
3. the corpus count of `Complete` fields rises from 0 of 504 (2026-11-17), and the public count from 2,270 of 3,240, with every remaining reason named.

## Current state (2026-11-17, `dev` at `665c548`)
Measured on the calc-run documents of PR #137's evidence (`local/card-fix/ev/new2-*`; the bundle `local/b2ev/run/bundle-poe1-tree-a`):

| Set | Builds | Fields `Complete` | Distinct unmapped stats |
|---|---|---|---|
| Public fixtures | 135 | 2,270 of 3,240 | 852 |
| Corpus | 21 | 0 of 504 | 407 |

**Bundle caveat (plan review):** that bundle predates #134: it has no `poe1/cluster_jewels.json` and no tree packs. T3 already feeds cluster jewel nodes into the calculation (`docs/tree/cluster-jewels.md`, "The calculation"), so most `cluster-jewel-node` and `tree-version` reasons below come from the bundle, not the engine. They wait on the maintainer's extraction rerun (cluster tables; the installed game is now build 26694821, so the extraction is the maintainer's) and on a bundle with packs (P6-0b below builds one locally for evidence). The gate's starting numbers are re-measured on that bundle.

**Baseline with packs (P6-0b, 2026-11-17):** `local/p6/packs/b-3_25`, the evidence bundle plus the packs 3_28 to 3_25 made by `tools/tree/acquire.ps1` and `tree-pack` (manifest SHA-256 `4814e3ac…`, the same as #134's record; `bundle-check` clean; local only, unsigned). On it, `local/p6/base-packs`:

| Set | Complete | Partial | Unsupported | Indeterminate | calc-parity unexpected |
|---|---|---|---|---|---|
| Public (136, 135 ok) | 2,270 | 386 | 482 | 102 | 0 |
| Corpus (21) | 0 | 193 | 141 | 170 | 0 |

This is the gate's starting point. The corpus runs on the packs also cover the tree report's maintainer-run item 3 locally (no corpus value is compared yet, as every corpus field is still `partial` or worse).

Reasons by the number of builds they touch (public / corpus): `unmapped-stat` 26 / 21; `unmodelled-config` 24 / 18; `unresolved-line` 22 / 12; `unmodelled-gem` 21 / 19; `unmodelled-item` (active flasks) 20 / 12; `unmodelled-passive` (cluster jewel nodes, unconnected nodes) 20 / 11; `tree-version` 16 / 21; `scaled-line` 12 / 4; `excluded-line` 10 / 5; `unmodelled-skill` 3 / 0. The main skill of most corpus builds is a damage-over-time, ailment, or minion skill, whose fields are `Unsupported` (`NotComputed`).

The engine (`loom-calc`, 18.6k lines in `poe1`) computes the 24 slice fields for a self-cast spell or a main-hand attack hit and the core defences. PoB's calculation is 22.8k lines of Lua in `Modules/Calc*.lua` (pinned v2.67.2 at `local/oracle/pob`), most of it in `CalcOffence.lua` (6.1k), `CalcPerform.lua` (3.9k), `CalcDefence.lua` (3.8k), `CalcTriggers.lua` (1.6k), `CalcSetup.lua` (1.9k).

The oracle adapter already writes PoB's whole `mainOutput` (with `Minion` for minion skills, `TotalDot`, `IgniteDPS`, `CombinedDPS`, ...), so new fields need only slice-fields entries, parity mapping, and engine work; no adapter protocol change.

## Design decisions
1. **Same method as Phase 3.** Each area ports PoB v2.67.2's logic with the same order of operations and rounding, cited by file and line; adds mechanic fixtures (small PoB XML builds of our own in `tools/oracle/fixtures/mechanic/`) that must match PoB; maps the stats the area needs (rows with tests, stat-ID first, ADR-015); and runs `calc-run` and `calc-parity` on the public fixtures and the corpus before and after. Any difference from PoB is fixed, or registered with evidence (`docs/parity/`; the game over PoB, ADR-001/006). Nothing is dropped silently: whatever stays unmodelled stays a reason.
2. **New slice fields, additive, emitted only when they apply.** `docs/phase-3/slice-fields.json` revision 3 adds the fields each area computes. Calc result v1's `field` is a string, so new names are additive (as build v1 and calc result v1 grew in Phase 3); the shape does not change. New `Field` variants and PoB keys:
   - damage over time: `dot-dps` (`TotalDot`), `ignite-dps` (`IgniteDPS`), `bleed-dps` (`BleedDPS`), `poison-dps` (`PoisonDPS`), `impale-dps` (`ImpaleDPS`), `combined-dps` (`CombinedDPS`), and the chances `ignite-chance`, `bleed-chance`, `poison-chance` (`IgniteChanceOnHit`, `BleedChanceOnHit`, `PoisonChanceOnHit`);
   - minions: `minion.<field>` for the minion's own life, hit fields, DoT fields, and `CombinedDPS`, PoB's `mainOutput.Minion.<key>`;
   - mines, traps, totems: PoB writes them into `Speed` and `TotalDPS` with their own keys (`TrapThrowingSpeed`, `MineLayingSpeed`, `ActiveTotemLimit`); compared as PoB prints them.
   - PoE1-only names are `Poe1Field` (`poe1.impale-dps`); DoT, ailment, and minion fields are shared names (PoE2 has them too).
   - **When a field is emitted:** the 24 Phase 3 fields stay in every document. A new field is emitted only when it applies to the build (its `applies` rule in slice revision 3; PoB writing the key, not `nil`, is the parity test), so a self-cast build does not carry dozens of `unsupported` fields. The gate's counts are per field set: the Phase 3 set (24 per build) and the Phase 6 set (the new fields that apply).
   - Readers that enumerate fields are updated in P6-1: the engine and parity tests that count 24 (`crates/loom-calc/tests/engine.rs`, `tools/parity-cli/src/calc_parity/slice.rs`, `tools/parity-cli/src/calc_run/tests.rs`), the golden documents (`crates/loom-pipeline/tests/testdata/synthetic-poe1.calc.json`, `crates/loom-serialization/src/dto/calc/v1/testdata/`), and the UI (`OFFENCE` in `ui/src/i18n/index.ts`, `FIRST` in `ui/src/screens/Dashboard.tsx`, the `field.*` labels in `ui/src/i18n/en.ts`, the default dashboards in `ui/src/dashboards.ts`). MCP and app API v1 need no shape change (field is a string).
3. **No new closed variants in calc result v1** (`dto/calc/v1/mod.rs`, "Adding to v1": after the Phase 3 gate a shape change is a new version). Release 0.9 shipped and MCP `get_breakdown` returns `FieldResult` and `Node`, so Phase 6 uses only the existing reasons, sources, and node kinds, with their `what` payloads (`unhandled-kind`, `unmodelled-skill`, `unmodelled-gem`, `unmodelled-item`, `rule`). A step that truly needs a new variant stops at that point for the maintainer's v2 decision and records it in the context file.
4. **One skill model, many kinds.** `poe1::skill` keeps deciding the main skill's kind; each area removes one exclusion from `spell_skill`/`attack_skill` and adds its kind's path (totem, trap, mine, brand, triggered, channelled, minion, dual-wield). A kind not yet modelled keeps `NotComputed`. Each step reads the gem attribute PoB reads for it (`skillPart`, `skillStageCount`, `skillMineCount`, `skillMinion`, `skillMinionSkill`, the oracle adapter's `GEM_CALCS`).
5. **PoB's per-skill data as a cited table.** Most active skills need PoB's hand-written skill data (`Data/Skills/*.lua`: `parts`, `baseFlags`, `baseMods`, per-skill `statMap`), which the game data does not carry; Phase 3 hand-wrote it for Smite (`hit::SKILL_PARTS`). P6-1 extends `tools/oracle/data/pob_data.lua` (export format 10) with each skill's `base_flags`, `parts`, and `base_mods`, and generates a committed Rust table in `loom-calc` from it (generator in `tools/`, the table cited and checked by a `data-parity` table, PoB is MIT). Skill functions (`preDamageFunc` and kin) are ported by hand per skill, with tests.
6. **Stat rows from PoB's own parser.** For the mop-up, a tool renders each unmapped stat's line from the game's stat descriptions and runs PoB's `modLib.parseMod` on it (`tools/oracle/data/pob_parse_lines.lua`, as ADR-015 Amendment 2's custom-line check does), giving candidate rows; each row is reviewed and tested before it is committed.
7. **Minions are a second actor.** PoB computes a minion with its own `env.minion` (`CalcPerform.lua`, `calcs.initEnv` minion branch; monster data in `Data/Minions.lua` and `Data/Spectres.lua`). The actor dimension lives in the shared engine core (PoE2 has minions too): the minion's own store, its base from monster data, the "Minions ..." stats mapped onto it, the player's auras and buffs that reach it, and the same hit and DoT phases run for it. The bundle has no monster varieties yet (P3-1d extracted only `MonsterTypes`), and PoB's minion `modList` and `skillList` are hand-written: a cited rule table, as `enemy::BOSSES`.
8. **Data first where data is missing.** Areas that need game data the bundle lacks add it with a `data-parity` table first (minion monster varieties and their skills, flask bases and buff stats, timeless jewel tables and seeds). Bundle schema changes are additive (schema 5 if needed), with the extractor root, the validator, synthetic rows, and the `data-parity` table. The real extraction is maintainer-run (ADR-005); until it runs, real-build evidence for those steps is pending and CI covers the synthetic bundles.
9. **Stat mop-up runs alongside and at the end.** Each area maps the stats it needs. P6-11 then takes the remaining distinct unmapped stats of the public fixtures and the corpus, in batches by kind, each row with a test; a stat becomes `NoEffect` only when PoB reads it into no output at all; a stat PoB reads into an output the engine does not compute yet (regen, leech, ward, EHP, movement speed, costs) stays `unmapped-stat` until P6-10b computes that output.
10. **Core crates stay wasm32-compatible, no I/O** (ADR-016). Timeless jewel lookups are data in the bundle, not files read by the engine.
11. **Timeless jewels from our own extraction** (ADR-005): the tables are derived from the extracted `AlternatePassiveSkills` and `AlternatePassiveAdditions` and the seed algorithm ported as code, tested against PoB's lookup tables; PoB's third-party tables are not shipped. Any decompression is pure Rust in `loom-data`.
    - **Amended 2026-11-19 (maintainer, Q-P6-1):**
      - **Why:** no implementation of the seed generator can ship under the project's MIT licence. The known ones are GPL-3.0 or unlicensed, and the generator's details exist only in their code.
      - **What changes:** the results come from PoB's precomputed lookup tables (PoB's MIT repository), carried in the bundle in a compact form, with a `data-parity` table against PoB's own.
      - **What stays:** the tables they index (`AlternatePassiveSkills`, `AlternatePassiveAdditions`, `AlternateTreeVersions`) still come from our own extraction.
      - **The check:** a local consistency check found PoB's tables and Vilsol's implementation in agreement on all 149,920 sampled lookups PoB holds (`docs/phase-6/timeless.md`, "The consistency check").
12. **Performance (ADR-013).** Each engine PR stays within the instruction-count limits or re-baselines with a stated reason (as #135); P6-12 re-checks the item-edit preview (300 ms) and allocation hover (150 ms) budgets.
13. **PRs as a cascade.** Each step is one PR from the previous step's branch (`docs/plan` workflow: the maintainer merges newest to oldest). Each PR carries its own evidence (ADR-012): tests, the before-and-after calc-run and calc-parity summaries, and the divergence-register entries.

## Planned PR breakdown
| Step | What | Fields it can complete |
|---|---|---|
| P6-0 | This plan, context, tasks; ACTION-PLAN and handoff | none |
| P6-0b | Local evidence bundle with tree packs 3_25 to 3_28 (`tools/tree/acquire`, `tree-pack`; local only, unsigned); baseline re-measured on it | none |
| P6-1 | Slice revision 3 (emission rule, new fields, readers updated); PoB skill table (export format 10, generated table, `data-parity`); skill damage over time (`CalcOffence.lua` 5647 on): a skill's own DoT (Righteous Fire, Essence Drain, Vortex, Caustic and burning ground), DoT multipliers, the enemy's DoT taken; `TotalDot`; DoT stats | `dot-dps` |
| P6-2a | Dual wielding (`CalcOffence.lua` 2003 on: per-hand passes and their combination); needed before ailments, which PoB computes per hand | attack fields of dual wielders |
| P6-2b | Ignite (4866 on) with the shared ailment base (4036 to 4287) | `ignite-dps`, `ignite-chance` |
| P6-2c | Bleed and poison (4288 to 4865) | `bleed-*`, `poison-*` |
| P6-2d | Impale (5472) and the remaining ailment outputs | `poe1.impale-dps` |
| P6-3 | Cluster jewels: what remains in the calculation once a bundle with the cluster tables exists (T3 feeds the nodes already); unconnected nodes as PoB treats them | removes the engine's part of `cluster-jewel-node` |
| P6-4 | Flasks (data: extractor root, bundle file, validator, `data-parity`; PoB's flask section in `CalcPerform.lua`) and the config inputs left `unmodelled-config` | removes `active-flask`, most `unmodelled-config` |
| P6-5 | Channelling (stages, cast rate; Scorching Ray's DoT); Vaal skills (one gem, two skills; the Vaal one as main skill) | channelled and Vaal skills |
| P6-6 | Totems, traps, mines, brands (skill type stats 1022 to 1455; placement and throwing speeds; active limits); `SKILL_PARTS` generalized from the PoB skill table | their hit and DoT fields |
| P6-7a | Minion data: monster varieties and their skills (extractor, bundle, validator, `data-parity`); PoB's hand-written minion mods as a cited table | none |
| P6-7b | The minion actor in the shared core; the player's auras, buffs, and curses on it; minion life and defences | `minion.life` and kin |
| P6-7c | Minion hits and DoT; spectres | `minion.*` damage |
| P6-8a | Triggers (`CalcTriggers.lua`): Cast on Crit, CWDT, Arcanist Brand, spellslinger, item triggers | triggered skills' fields |
| P6-8b | Mirages (`CalcMirages.lua`) | mirage parts |
| P6-9 | The remaining gems (`unmodelled-gem`): supports, marks and the curse-slot priority, exerted attacks from warcries, banners, heralds | removes `unmodelled-gem` |
| P6-10a | Timeless jewels and alternate trees (decision 11) | removes timeless reasons |
| P6-10b | Recovery and defence outputs: life, mana, and ES regen and leech rates, ward, physical damage reduction, maximum hit taken and EHP, movement speed, skill costs (`CalcDefence.lua`, `CalcOffence.lua` costs) | new defence fields |
| P6-11 | Stat mop-up (decision 6), in batches by stat kind, each with its evidence | every field with only stat reasons |
| P6-12 | `combined-dps` and PoB's `FullDPS` once every part exists (until then `combined-dps` is `Partial` with an `unhandled-kind` reason when a later step's part is present); report `docs/phase-6/phase-6-report.md`: the counts, the register, the gate checklist; performance budgets re-checked | `combined-dps` complete |

Steps can split further; the order can move only when a step depends on a later one, recorded in the tasks file.

## Risks
- **Size.** PoB's offence code is the bulk of its calculation; P6-2, P6-7, and P6-8 are each as large as Phase 3's spell and attack work. Mitigation: port module by module with citations; keep every unported branch a reason.
- **Data versions.** Our data is the current Steam build; PoB's is v2.67.2's. Differences in a mechanic's numbers are told apart with `data-parity` and the overlay first (Phase 3 decision 5).
- **Real data not in CI** (Q1 default): parity on real builds shows only in local runs; CI runs the mechanic fixtures and the synthetic bundles.
- **No maintainer between steps.** Decisions the maintainer would make are taken on the default that matches PoB and are listed in the context file's "Decisions taken without the maintainer" for review afterwards.

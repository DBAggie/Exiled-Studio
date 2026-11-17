Last Updated: 2026-11-17 (P6-0: plan)

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

Reasons by the number of builds they touch (public / corpus): `unmapped-stat` 26 / 21; `unmodelled-config` 24 / 18; `unresolved-line` 22 / 12; `unmodelled-gem` 21 / 19; `unmodelled-item` (active flasks) 20 / 12; `unmodelled-passive` (cluster jewel nodes, unconnected nodes) 20 / 11; `tree-version` 16 / 21; `scaled-line` 12 / 4; `excluded-line` 10 / 5; `unmodelled-skill` 3 / 0. The main skill of most corpus builds is a damage-over-time, ailment, or minion skill, whose fields are `Unsupported` (`NotComputed`).

The engine (`loom-calc`, 18.6k lines in `poe1`) computes the 24 slice fields for a self-cast spell or a main-hand attack hit and the core defences. PoB's calculation is 22.8k lines of Lua in `Modules/Calc*.lua` (pinned v2.67.2 at `local/oracle/pob`), most of it in `CalcOffence.lua` (6.1k), `CalcPerform.lua` (3.9k), `CalcDefence.lua` (3.8k), `CalcTriggers.lua` (1.6k), `CalcSetup.lua` (1.9k).

The oracle adapter already writes PoB's whole `mainOutput` (with `Minion` for minion skills, `TotalDot`, `IgniteDPS`, `CombinedDPS`, ...), so new fields need only slice-fields entries, parity mapping, and engine work; no adapter protocol change.

## Design decisions
1. **Same method as Phase 3.** Each area ports PoB v2.67.2's logic with the same order of operations and rounding, cited by file and line; adds mechanic fixtures (small PoB XML builds of our own in `tools/oracle/fixtures/mechanic/`) that must match PoB; maps the stats the area needs (rows with tests, stat-ID first, ADR-015); and runs `calc-run` and `calc-parity` on the public fixtures and the corpus before and after. Any difference from PoB is fixed, or registered with evidence (`docs/parity/`; the game over PoB, ADR-001/006). Nothing is dropped silently: whatever stays unmodelled stays a reason.
2. **New slice fields, additive.** `docs/phase-3/slice-fields.json` revision 3 adds the fields each area computes. Calc result v1's `field` is a string, so new names are additive (as build v1 and calc result v1 grew in Phase 3); the shape does not change. New `Field` variants and PoB keys:
   - damage over time: `dot-dps` (`TotalDot`), `ignite-dps` (`IgniteDPS`), `bleed-dps` (`BleedDPS`), `poison-dps` (`PoisonDPS`), `impale-dps` (`ImpaleDPS`), `combined-dps` (`CombinedDPS`), and the chances `ignite-chance`, `bleed-chance`, `poison-chance` (`IgniteChanceOnHit`, `BleedChanceOnHit`, `PoisonChanceOnHit`);
   - minions: `minion.<field>` for the minion's own life, hit fields, DoT fields, and `CombinedDPS`, PoB's `mainOutput.Minion.<key>`;
   - mines, traps, totems: PoB writes them into `Speed` and `TotalDPS` with their own keys (`TrapThrowingSpeed`, `MineLayingSpeed`, `ActiveTotemLimit`); compared as PoB prints them.
3. **One skill model, many kinds.** `poe1::skill` keeps deciding the main skill's kind; each area removes one exclusion from `spell_skill`/`attack_skill` and adds its kind's path (totem, trap, mine, brand, triggered, channelled, minion, dual-wield). A kind not yet modelled keeps `NotComputed`.
4. **Minions are a second actor.** PoB computes a minion with its own `env.minion` (`CalcPerform.lua`, `calcs.initEnv` minion branch; monster data in `Data/Minions.lua` and `Data/Spectres.lua`). The engine gains an actor dimension: the minion's own store, its base from the monster data (bundle: `MonsterVarieties`, already partly extracted for P3-1d), the "Minions ..." stats mapped onto it, and the same hit and DoT phases run for it.
5. **Data first where data is missing.** Areas that need game data the bundle lacks add it with a `data-parity` table first (minion monster varieties and their skills, flask bases and buff stats, timeless jewel tables and seeds). Bundle schema changes are additive (schema 5 if needed), with the validator and synthetic rows.
6. **Stat mop-up runs alongside and at the end.** Each area maps the stats it needs. P6-11 then takes the remaining distinct unmapped stats of the public fixtures and the corpus, in batches by kind, each row with a test; stats that reach no field become `NoEffect` with a reason, as P3-8n did.
7. **Core crates stay wasm32-compatible, no I/O** (ADR-016). Timeless jewel lookups are data in the bundle, not files read by the engine.
8. **PRs as a cascade.** Each step is one PR from the previous step's branch (`docs/plan` workflow: the maintainer merges newest to oldest). Each PR carries its own evidence (ADR-012): tests, the before-and-after calc-run and calc-parity summaries, and the divergence-register entries.

## Planned PR breakdown
| Step | What | Fields it can complete |
|---|---|---|
| P6-0 | This plan, context, tasks; ACTION-PLAN and handoff | none |
| P6-1 | Slice revision 3; skill damage over time (`CalcOffence.lua` "Calculate skill DOT components", 5647 on): a skill's own DoT (Righteous Fire, Essence Drain, Vortex, Caustic ground, Scorching Ray), DoT multipliers, the enemy's DoT taken; `TotalDot`; DoT stats | `dot-dps` |
| P6-2 | Ailments: ignite, bleed, poison (4036 to 5206: chance, base, stacking, duration, faster/slower), impale (5472); non-damaging ailment effects already read are kept; `CombinedDPS` (5982); ailment stats | ailment fields, `combined-dps` |
| P6-3 | Cluster jewel nodes in the calculation (the tree's generated nodes, `loom_tree::poe1` cluster model, into `collect`); unconnected nodes as PoB treats them | removes `cluster-jewel-node` |
| P6-4 | Flasks (active flasks as PoB applies them: `CalcPerform.lua` flask section; flask base data and buff stats) and the config inputs left `unmodelled-config` | removes `active-flask`, most `unmodelled-config` |
| P6-5 | Dual wielding (`CalcOffence.lua` "Combine stats from Main Hand and Off Hand", 2003 on) and channelling (stages, cast rate) | attack fields of dual wielders |
| P6-6 | Totems, traps, mines, brands (skill type stats 1022 to 1455; placement and throwing speeds; active limits) | their hit and DoT fields |
| P6-7 | Minions (decision 4): actor, monster data, minion modifiers, hit and DoT for the minion, `minion.*` fields | `minion.*` |
| P6-8 | Triggers and mirages (`CalcTriggers.lua`, `CalcMirages.lua`): Cast on Crit, Cast when Damage Taken, Arcanist Brand, Mjolner, Cospri's, CWDT, spellslinger, General's Cry mirages | triggered skills' fields |
| P6-9 | Unmodelled gems: every support and active gem the fixtures use that still has no model (`unmodelled-gem`) | removes `unmodelled-gem` |
| P6-10 | Timeless jewels and alternate trees (legion data in the bundle, seeds, `DataLegionLookUpTableHelper.lua`) | removes timeless reasons |
| P6-11 | Stat mop-up: the remaining unmapped stats, in batches; the remaining unresolved item lines where a template can be added | every field with only stat reasons |
| P6-12 | Report `docs/phase-6/phase-6-report.md`: the counts, the register, the gate checklist for the maintainer's run | none |

Areas can split into more than one PR when they grow; the order can move only when a step depends on a later one, recorded in the tasks file.

## Risks
- **Size.** PoB's offence code is the bulk of its calculation; P6-2, P6-7, and P6-8 are each as large as Phase 3's spell and attack work. Mitigation: port module by module with citations; keep every unported branch a reason.
- **Data versions.** Our data is the current Steam build; PoB's is v2.67.2's. Differences in a mechanic's numbers are told apart with `data-parity` and the overlay first (Phase 3 decision 5).
- **Real data not in CI** (Q1 default): parity on real builds shows only in local runs; CI runs the mechanic fixtures and the synthetic bundles.
- **No maintainer between steps.** Decisions the maintainer would make are taken on the default that matches PoB and are listed in the context file's "Decisions taken without the maintainer" for review afterwards.

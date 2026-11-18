Last Updated: 2026-11-17 (P6-0: plan)

# Phase 6: PoE1 mechanics expansion (context)

## Key files
- Engine: `crates/loom-calc/src/poe1/` (`mod.rs` quantities and slice; `skill.rs` main skill kind; `hit.rs` hit damage; `mapping.rs` stat rows, `NO_EFFECT`; `collect.rs` gathers the build; `phases.rs` phase order; `gems.rs` other gems).
- Fields: `crates/loom-calc/src/result.rs` (`Field`), `docs/phase-3/slice-fields.json` (PoB keys, comparison rules).
- Parity: `tools/parity-cli/src/calc_run.rs`, `calc_parity.rs`; oracle adapter `tools/oracle/adapter/oracle.lua` (writes the whole `mainOutput`).
- PoB pinned v2.67.2: `local/oracle/pob/src/Modules/CalcOffence.lua`, `CalcPerform.lua`, `CalcTriggers.lua`, `CalcMirages.lua`, `CalcSetup.lua`, `CalcActiveSkill.lua`; data `local/oracle/pob/src/Data/`.
- Evidence baseline: `local/card-fix/ev/new2-public`, `new2-corpus` (calc-run documents at #137); reason counts by `C:\...\scratchpad\reasons.py` (rewrite as `tools/` script if kept).
- Bundle for real runs: `local/b2ev/run/bundle-poe1-tree-a`.

## Decisions taken without the maintainer
(Each is the default that matches PoB; listed for review after the run.)
- D1 (P6-0): the phase gate is run after the work, as the maintainer asked; the proposed gate is in the plan.
- D2 (P6-0): new fields are added to calc result v1 by name (additive), not a v2 document.

- D3 (P6-0): no new closed variants in calc result v1 (plan decision 3); a step that needs one stops there for the maintainer.
- D4 (P6-0): PoB's hand-written skill data becomes a generated, cited table in `loom-calc` (plan decision 5).
- D5 (P6-0): the mop-up's candidate rows come from PoB's own parser on the game's text, each reviewed (plan decision 6).

- D6 (P6-1): the generated PoB skill table is guarded by pinned tests of the entries the engine reads (`pob_skill::tests`) and by regeneration being byte-identical (`docs/phase-6/dot.md`), not yet by a `data-parity` table; the table is PoB's own data, so a parity table would compare it with itself. A check against the game's skill IDs comes with P6-6, where parts are read.
- D7 (P6-1): Righteous Fire's degeneration and its own spell buff are no effect when it is the main skill (`dot::DOT_SKILL_STATS`), as the gem buff's `NoSlice("degeneration")` precedent. This is an exception to plan decision 9 until P6-10b computes degeneration, which must turn them back into reads.
- D8 (P6-1): burning, caustic and Corrupting Blood ground and stacking DoTs (the plan's P6-1 row lists Caustic and burning ground) stay present and `NotComputed`. PoB writes them to their own outputs, which `combined-dps` reads in P6-12.
- D9 (P6-1): the Essence Drain fixture (`local/p6/essence-drain-dot-witch.xml`, its DoT equal to PoB's) waits for P6-10b. Its leech stat stays unmapped (decision 9), so every field is partial and the fixture has no target.
- D10 (P6-9): P6-9 runs before P6-7c (the plan order moves only for a dependency): every minion build's fields carry the summoning gem's `unmodelled-gem` until P6-9 models it, so P6-7c's minion hit fields could not be `Complete` before it.

## Plan review (2026-11-17, plan-reviewer)
All 16 findings applied in the plan: the pre-#134 evidence bundle (P6-0b, P6-3 re-scoped); calc v1 closed variants (decision 3); fields emitted only when they apply and the readers that count fields (decision 2); minion and flask data and who extracts it (decisions 7, 8); timeless tables from our own extraction (decision 11); `NoEffect` only for stats PoB reads into no output (decision 9) and the recovery and defence outputs (P6-10b); Scorching Ray with channelling (P6-5); dual wielding before ailments (P6-2a); `combined-dps` last (P6-12); auras and buffs on minions (P6-7b); gem selection attributes (decision 4); Vaal skills, PoB skill data, marks, exerts, spectres, `FullDPS` (P6-5, decision 5, P6-9, P6-7c, P6-12); PoE2-shared names and the minion actor in the core (decisions 2, 7); performance (decision 12); steps split up front.

## Open questions
- Q12 (attack speed order) stays open; saved test in `docs/phase-3/attack-hits.md`.

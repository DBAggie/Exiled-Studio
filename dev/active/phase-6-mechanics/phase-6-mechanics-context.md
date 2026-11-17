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

## Open questions
- Q12 (attack speed order) stays open; saved test in `docs/phase-3/attack-hits.md`.

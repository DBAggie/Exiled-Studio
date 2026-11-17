Last Updated: 2026-11-17 (P6-0: plan, plan-reviewer findings applied)

# Phase 6: PoE1 mechanics expansion (tasks)

Order and evidence per PR are in the plan, "Planned PR breakdown". Tick a task in the PR that does it, with the PR number. Every engine PR: unit tests, mechanic fixtures where PoB has the mechanic, before-and-after `calc-run` and `calc-parity` on public and corpus (`sh local/p6/evidence.sh <step>`; baseline `local/p6/base`), register entries, the bench within limits or re-baselined with a reason, handoff and CLAUDE.md updated.

## P6-0: plan
- [x] Plan, context, tasks in `dev/active/phase-6-mechanics/`
- [x] plan-reviewer run; findings applied (context file, "Plan review")
- [x] ACTION-PLAN, handoff, CLAUDE.md updated
- [ ] Open PR

## P6-0b: evidence bundle with tree packs (local)
- [ ] `tools/tree/acquire` 3_28 to 3_25; `tree-pack` chained; `bundle-check`
- [ ] Baseline re-measured on it (`local/p6/base-packs`); plan's numbers updated

## P6-1: slice revision 3, PoB skill table, skill damage over time
- [ ] Slice revision 3: emission rule (`applies`), new fields, `Field` variants
- [ ] Readers that count fields: engine and parity tests, golden documents, UI (`OFFENCE`, `FIRST`, labels, dashboards)
- [ ] `pob_data.lua` format 10: `base_flags`, `parts`, `base_mods`; generator; generated table; `data-parity` table
- [ ] Skill DoT (`CalcOffence.lua` 5647 on), DoT multipliers, enemy DoT taken, `TotalDot`; DoT stat rows
- [ ] Mechanic fixtures; evidence

## P6-2a: dual wielding
## P6-2b: ignite and the shared ailment base
## P6-2c: bleed and poison
## P6-2d: impale
## P6-3: cluster jewels (what remains)
## P6-4: flasks and config inputs
## P6-5: channelling and Vaal skills
## P6-6: totems, traps, mines, brands
## P6-7a: minion data
## P6-7b: minion actor, auras and buffs on it, minion defences
## P6-7c: minion hits and DoT, spectres
## P6-8a: triggers
## P6-8b: mirages
## P6-9: the remaining gems (supports, marks, curse priority, exerts, banners, heralds)
## P6-10a: timeless jewels
## P6-10b: recovery and defence outputs
## P6-11: stat mop-up (batches)
## P6-12: combined and full DPS; report

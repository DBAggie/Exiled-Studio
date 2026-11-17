Last Updated: 2026-11-17 (P6-1: slice revision 3, PoB skill table, skill damage over time)

# Phase 6: PoE1 mechanics expansion (tasks)

Order and evidence per PR are in the plan, "Planned PR breakdown". Tick a task in the PR that does it, with the PR number. Every engine PR: unit tests, mechanic fixtures where PoB has the mechanic, before-and-after `calc-run` and `calc-parity` on public and corpus (`sh local/p6/evidence.sh <step>`; baseline `local/p6/base`), register entries, the bench within limits or re-baselined with a reason, handoff and CLAUDE.md updated.

## P6-0: plan
- [x] Plan, context, tasks in `dev/active/phase-6-mechanics/`
- [x] plan-reviewer run; findings applied (context file, "Plan review")
- [x] ACTION-PLAN, handoff, CLAUDE.md updated
- [x] Open PR: #139 (base `dev`)

## P6-0b: evidence bundle with tree packs (local)
- [x] `tools/tree/acquire` 3_28 to 3_25; `tree-pack` chained; `bundle-check` (manifest `4814e3ac…`, as #134 recorded)
- [x] Baseline re-measured on it (`local/p6/base-packs`); plan's numbers updated (#139)

## P6-1: slice revision 3, PoB skill table, skill damage over time
- [x] Slice revision 3: `applies: present`, `TotalDot`, `Field::DotDps` (emitted only when the main skill has a DoT)
- [x] Readers that count fields: parity tests (revision, `present`), UI (`OFFENCE`, `FIRST`, label); golden documents unchanged (no DoT in the synthetic build)
- [x] `pob_data.lua` format 10: `base_flags`, `parts`, `functions`; `loom-parity-cli pob-skills`; generated `poe1/pob_skills.rs` (1,231 skills, regenerates byte-identical). The `data-parity` check of the table itself is left for P6-6, where parts are first read; the base mods read here are `dotIsArea` and the ground flags.
- [x] Skill DoT (`poe1::dot`), DoT multipliers, enemy DoT taken, `TotalDot`; DoT stat rows; Righteous Fire and Vaal Righteous Fire; Righteous Fire as an other-gem buff
- [x] Found on the way: PoB's skills layout before skill sets (`SkillsLayout::Flat`, build v1 `skills.layout` additive); gems the data does not have are skipped as PoB does (a reason), renamed `skillId`s found by name
- [x] Mechanic fixtures (5, all equal to PoB; Essence Drain kept local until P6-10b, D9); code-architecture-reviewer run, findings applied or recorded (D6 to D9); evidence (`docs/phase-6/dot.md`)
- [x] Open PR: #140 (base `plan/phase-6-mechanics`)

## P6-2a: dual wielding
- [x] The off hand's weapon as its own quantities (`OffHandWeapon`, `OffHandWeaponMapping`); which hands attack (`CalcActiveSkill.lua` 318 to 335)
- [x] One pass per hand and PoB's `combineStat` (average, harmonic mean, `DPS` with double hits); inherent dual-wield speed and block; the item card's off-hand weapon
- [x] Fixture `smite-dual-wield-templar.xml` (21 fields equal to PoB; Q11 rate DIV-0072, DIV-0073; breakdown DIV-0074 `harness`); `docs/phase-6/dual-wield.md`
- [x] Open PR: #141 (base `feat/p6-1-dot`)
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

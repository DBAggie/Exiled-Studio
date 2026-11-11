Last Updated: 2026-11-11 (PR P3-0: plan written, plan-reviewer findings applied)

# Phase 3: calc slice (tasks)

Order and evidence per PR are in the plan, "Planned PR breakdown". Tick a task in the PR that does it, with the PR number.

## P3-0: plan
- [x] Plan, context, tasks in `dev/active/phase-3-calc-slice/`
- [x] plan-reviewer run; all findings applied (context file)
- [x] Handoff, CLAUDE.md Status, ACTION-PLAN Next action updated
- [ ] Open PR (base `feat/p2-1-domain-model`, per the maintainer's 2026-11-11 instruction); record its number

## P3-1a: bundle v2 manifest and skill tables
- [ ] v2 manifest (v1 still loads for import)
- [ ] Skills, skill types, granted effects, per-level and per-quality stat sets, gems, gem tags: types, loader, validator codes with mutation cases, synthetic rows, extractor table, converter
- [ ] Two-run determinism and `bundle-check` on real bundles; summaries

## P3-1b: character and passive tables
- [ ] Characters, ascendancies, passives, passive graph with tree version, mastery effects (same checklist as 1a)

## P3-1c: item property and monster tables
- [ ] Weapon, armour, shield properties; default monster stats (same checklist as 1a)

## P3-2: data parity
- [ ] `data-parity` (PoB data read by a script in `tools/oracle/data/`, outside the adapter protocol)
- [ ] Categories with cited evidence and tests; overlay file; report with `uncategorized` 0

## P3-3: survey and slice
- [ ] `calc-survey`: archetype classifier, tree-version rule, excluded-mechanics rules (tests each), mechanic inventory
- [ ] Skill choice (Q2), excluded mechanics (Q7), in-scope list with at least 3 builds per skill (credited public builds added if needed), `docs/phase-3/slice-fields.json` with comparison rules and breakdown groupings (Q3, Q6)
- [ ] Plan updated; maintainer confirmation before P3-6

## P3-4: engine core
- [ ] `CalcInput`, `CalcData`, options, `DataVersion` check
- [ ] Modifier store with conditions and provenance; graph evaluation
- [ ] Field states, `UnsupportedReason`, `CALC_*`, cancellation hook, `math`
- [ ] Property tests; calc PoE1-only names in `POE1_ONLY`; wasm32

## P3-13: calc result document v1 (right after P3-4)
- [ ] DTO, golden bytes, schema and TypeScript, cross-platform job

## P3-5: parity harness
- [ ] `calc-run` (with `--data-overlay`, timing), `calc-parity` (field mapping, rules, breakdown terms at displayed decimals, completeness, register, overlay rule)
- [ ] `mechanic` tier in `oracle_fixtures.rs` and the CI `oracle` job
- [ ] Tests per outcome; breakdown parser on every public-fixture line shape; AGENTS.md commands

## P3-6 to P3-12: engine
- [ ] P3-6 character, attributes, life, mana, ES
- [ ] P3-7 core defences
- [ ] P3-8 stat mappings (one or more PRs; coverage summary each)
- [ ] P3-11 enemy model
- [ ] P3-9a skill model
- [ ] P3-9b spell hits
- [ ] P3-10 attacks
- [ ] P3-12 buffs and config (one or more PRs)

## P3-14: corpus run and register
- [ ] `calc-parity` on public fixtures and corpus; every difference fixed or registered
- [ ] CI calc-parity job if Q1 is yes

## P3-15: report and gate
- [ ] `docs/phase-3/phase-3-report.md`; handoff, CLAUDE.md, ACTION-PLAN; folder to `dev/completed/`

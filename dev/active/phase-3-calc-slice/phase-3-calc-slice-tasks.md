Last Updated: 2026-11-12 (P3-8b: rolled values)

# Phase 3: calc slice (tasks)

Order and evidence per PR are in the plan, "Planned PR breakdown". Tick a task in the PR that does it, with the PR number.

## P3-0: plan
- [x] Plan, context, tasks in `dev/active/phase-3-calc-slice/`
- [x] plan-reviewer run; all findings applied (context file)
- [x] Handoff, CLAUDE.md Status, ACTION-PLAN Next action updated
- [x] Open PR: #59 (base `feat/p2-1-domain-model`, per the maintainer's 2026-11-11 instruction)

## P3-1a: bundle v2 manifest and skill tables
- [x] v2 manifest (v1 still loads for import)
- [x] Skills, skill types, granted effects, per-level and per-quality stat sets, gems, gem tags: types, loader, validator codes with mutation cases, synthetic rows, extractor table, converter (plus cost types and gem effects)
- [x] Two-run determinism and `bundle-check` on real bundles; summaries (`docs/phase-3/bundle-v2-skills.md`)
- [x] Open PR: #60 (base `plan/phase-3-calc-slice`)

## P3-1b: character and passive tables
- [x] Characters, ascendancies, passives, passive graph with tree version, mastery effects (same checklist as 1a); per-level growth from the character object template (extractor reads `*.ot`, manifest format 5)
- [x] Two-run determinism and `bundle-check` on real bundles; summaries (`docs/phase-3/bundle-v2-passives.md`)
- [x] Open PR: #61 (base `feat/p3-1a-bundle-v2-skills`)

## P3-1c: item property and monster tables
- [x] Weapon, armour, shield properties; default monster stats (same checklist as 1a); game constants
- [x] Two-run determinism and `bundle-check` on real bundles; summaries (`docs/phase-3/bundle-v2-items-monsters.md`)
- [x] Open PR: #63 (base `docs/ui-draft-v1`, #62)

## P3-2: data parity
- [x] `data-parity` (PoB data read by a script in `tools/oracle/data/`, outside the adapter protocol)
- [x] Categories with cited evidence and tests; overlay file; report with `uncategorized` 0 (`docs/phase-3/data-parity.md`)
- [x] Open PR: #64 (base `feat/p3-1c-bundle-v2-items-monsters`, #63)

## P3-3: survey and slice
- [x] `calc-survey`: archetype classifier, tree-version rule, excluded-mechanics rules (tests each), mechanic inventory (P3-3)
- [x] Proposed excluded mechanics (Q7) in `docs/phase-3/calc-scope-poe1.json`; `docs/phase-3/slice-fields.json` with comparison rules and breakdown groupings (Q3, Q6); survey summaries (`docs/phase-3/calc-survey.md`) (P3-3)
- [x] Plan updated (P3-3)
- [x] Open PR: #66 (base `dev`)
- [x] Maintainer: Q2 (Ice Nova, Smite), Q7 (three exclusions), Q8 (derived setups; minimum 3), 2026-11-11
- [x] P3-3b: chosen skills in the scope file (`confirmed`); seven credited derived fixtures; Ice Nova 3, Smite 4 in scope; public `calc-survey` exit 0
- [x] Open PR: #67 (base `dev`)

## P3-4: engine core
- [x] `CalcInput`, `CalcData`, options, `DataVersion` check
- [x] Modifier store with conditions and provenance; graph evaluation
- [x] Field states, `UnsupportedReason`, `CALC_*`, cancellation hook, `math`
- [x] Property tests; calc PoE1-only names in `POE1_ONLY`; wasm32
- [x] `code-architecture-reviewer` run; findings applied (context file)
- [x] Open PR: #68 (base `dev`)

## P3-13: calc result document v1 (right after P3-4)
- [x] `loom_serialization::dto::calc::v1` (DTO, `to_json`, `from_json`, `DTO_CALC_*`); `loom-calc` <- `loom-serialization` in AGENTS.md
- [x] Golden bytes for 4 results, second run, read-back, names, error codes
- [x] `schemas/calc-result.v1.schema.json`, `ui/src/generated/calc-result.v1.ts`, schema validation tests
- [x] Cross-platform: golden test in the `rust` job on Windows and Linux (calc-run documents join `dto cross-platform` in P3-5)
- [x] `code-architecture-reviewer` run; findings in the context file
- [x] Open PR: #69 (base `dev`)

## P3-5: parity harness
- [x] `calc-run` (with `--data-overlay`, timing), `calc-parity` (field mapping, rules, breakdown terms at displayed decimals, completeness, register, overlay rule, PoB's views check, mechanic targets)
- [x] `mechanic` tier in `oracle_fixtures.rs` (`mechanic/`, `targets`); the CI `oracle` job runs every tier of the manifest; CI runs `calc-run` on the synthetic builds and `dto cross-platform` diffs the documents
- [x] Tests per outcome; breakdown parser on every public-fixture breakdown (`testdata/calc-parity/breakdown-lines.json`); AGENTS.md commands; `slice-fields.json` revision 2 (`graph` per term)
- [x] Evidence (`docs/phase-3/calc-parity.md`): public and corpus runs byte-identical twice; `calc-parity` exit 5 `incomplete` (all fields `not-computed`, nothing passed); PoB's views differed in 12 public (all 7 in-scope) and 11 corpus builds (Q9)
- [x] Q9 (a), maintainer 2026-11-11: oracle adapter protocol 4 sets PoB's CALCS view to the main view (`probe.calcs_view`); `main_output` unchanged in 57 outputs; views agree everywhere; breakdown test data regenerated
- [x] `code-architecture-reviewer` run; findings in the context file
- [x] Open PR: #70 (base `dev`)

## P3-6 to P3-12: engine
- [x] P3-6 character, attributes, life, mana, ES (PR #71; `docs/phase-3/character-fields.md`; 19 mechanic fixtures match on their targets)
- [x] P3-7 core defences (PR #72; `docs/phase-3/defence-fields.md`; 23 mechanic fixtures match on 17 fields)
- [ ] P3-8 stat mappings (one or more PRs; coverage summary each)
  - [x] P3-8a stats that reach no slice field (PR #73; `docs/phase-3/stat-mapping.md`; in-scope unmapped 153 to 113)
  - [x] P3-8b rolled values of PoB's ranged lines, scaled lines as reasons (PR #80; `docs/phase-3/rolled-values.md`; in-scope builds no longer `indeterminate`)
- [x] P3-1d monster types for the default enemy (PR #75; `docs/phase-3/monster-types.md`; 22 bosses equal PoB's)
- [x] P3-9a skill model (PR #74; `docs/phase-3/skill-model.md`; survey byte-identical on the core rule)
- [x] P3-11 enemy model (PR #76; `docs/phase-3/enemy-model.md`; the default enemy equals PoB's; hit fixtures in P3-9b)
- [x] P3-9b spell hits (PR #77; `docs/phase-3/spell-hits.md`; 11 Ice Nova mechanic fixtures match PoB on every target)
- [x] P3-10 attacks (PR #78; `docs/phase-3/attack-hits.md`; 10 Smite mechanic fixtures match PoB on every target; Q11 open)
- [ ] P3-12 buffs and config (one or more PRs)

## P3-14: corpus run and register
- [ ] `calc-parity` on public fixtures and corpus; every difference fixed or registered
- [ ] CI calc-parity job if Q1 is yes

## P3-15: report and gate
- [ ] `docs/phase-3/phase-3-report.md`; handoff, CLAUDE.md, ACTION-PLAN; folder to `dev/completed/`

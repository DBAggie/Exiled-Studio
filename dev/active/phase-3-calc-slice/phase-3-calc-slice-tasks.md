Last Updated: 2026-11-13 (P3-8e: area damage, spell crit multiplier, area and light radius)

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
  - [x] P3-8c the Smite builds' global stats (PR #83, merged into `dev` as `bcb8c6d`; `docs/phase-3/stat-mapping.md`; 3 hit rows, 2 `NoEffect` rows with `NO_EFFECT_DEPENDS_ON`; `smite-inquisitor-act1` has no `unmapped-stat`; in-scope unmapped 108 to 103; mechanic fixtures `smite-global-attack-stats-templar.xml`, `ice-nova-attack-and-cast-speed-witch.xml`)
- [x] P3-1d monster types for the default enemy (PR #75; `docs/phase-3/monster-types.md`; 22 bosses equal PoB's)
- [x] P3-9a skill model (PR #74; `docs/phase-3/skill-model.md`; survey byte-identical on the core rule)
- [x] P3-11 enemy model (PR #76; `docs/phase-3/enemy-model.md`; the default enemy equals PoB's; hit fixtures in P3-9b)
- [x] P3-9b spell hits (PR #77; `docs/phase-3/spell-hits.md`; 11 Ice Nova mechanic fixtures match PoB on every target)
- [x] P3-10 attacks (PR #78; `docs/phase-3/attack-hits.md`; 10 Smite mechanic fixtures match PoB on every target; Q11 open)
  - [x] Q11 switch (PR #81): a weapon's attack time in floored ms (`docs/phase-3/attack-hits.md`, "The switch"; 25 `game-truth` entries; Q12 open)
- [ ] P3-12 buffs and config (one or more PRs)
  - [x] P3-12a config inputs that only set named conditions (PR #82, merged into `dev` with #81 as `636a482`; `docs/phase-3/config-inputs.md`; 16 inputs, reasons only; mechanic fixture `ice-nova-config-conditions-witch.xml`)
  - [x] P3-12b PoB's config placeholders (PR #84, merged into `dev` as `95b53a6`; `docs/phase-3/config-inputs.md`, "P3-12b"; placeholders typed in the domain, build v1 `kind`; classified as their inputs; reasons only; mechanic fixture `ice-nova-config-placeholders-witch.xml`)
  - [x] P3-12c custom modifier blocks in full (PR #85, merged into `dev` as `8c2998d`; `docs/phase-3/config-inputs.md`, "P3-12c"; lines resolved and applied; build v1 `custom_modifiers`, calc result v1 source `custom-modifier`; mechanic fixture `ice-nova-custom-modifiers-witch.xml`)
  - [x] P3-12d the legacy `customMods` input (PR #86, merged into `dev` as `e04419f`; `docs/phase-3/config-inputs.md`, "P3-12d"; PoB's attribute reading for a config string; lines applied by PoB's `Load` migration; build v1 input `lines`, calc result v1 source `config-input-line`; mechanic fixture `ice-nova-legacy-custom-mods-witch.xml`)
  - [x] P3-12e the old config layout (`ConfigLayout::Flat`, build v1 `layout`), block text PoB reads otherwise kept whole, power/frenzy/endurance charges (`poe1::charges`), fortification in `NAMED_ONLY` (PR #87, merged into `dev` as `4753b28`; `docs/phase-3/config-inputs.md`, "P3-12e"; mechanic fixtures `ice-nova-old-config-layout-witch.xml`, `ice-nova-charges-witch.xml`, `smite-charges-templar.xml`)
  - [x] P3-12f the enemy's damage taken in the hit (`poe1::damage_taken`: shock, the Uber boss kind, delirium), the bandit from the config set, Onslaught (`poe1::buffs`), rage and exposure in `NAMED_ONLY` with their source stats, culling strike `NoEffect` (PR #88, merged; `docs/phase-3/config-inputs.md`, "P3-12f"; mechanic fixtures `ice-nova-shocked-uber-witch.xml`, `smite-shocked-templar.xml`, `ice-nova-onslaught-rage-exposure-witch.xml`; DIV-0029, DIV-0030; merged into `dev` as `6d1d7ac`)
  - [x] P3-12g the pantheon (bundle `poe1/pantheons.json`, `data-parity` table, `poe1::pantheon`, 41 `NoEffect` soul stats), Intimidate, Unnerve, Blind (`Poe1Quantity::EnemyEvasion`), ten inputs in `NAMED_ONLY` (PR #89, merged into `dev` as `de85329`; `docs/phase-3/config-inputs.md`, "P3-12g"; six mechanic fixtures; DIV-0031 to DIV-0033)
  - [x] P3-12h other gems, first PR: `poe1::gems` (inert skills and supports, reservation with the supports' cost multipliers, eight buff and aura skills merged per name), `Ceil` node, PoB export format 4 and `data-parity` table `gem_effects` (PR #90, merged into `dev` as `0814131`; `docs/phase-3/other-gems.md`; four mechanic fixtures)
  - [x] P3-12i socket colour quality: the granted effect's colour in the bundle, PoB export format 5 and `data-parity` `color`, `poe1::sockets` (+10 quality in a socket of the gem's colour, PoB's index rule), `melee_physical_damage_+%` row (PR #91, open; `docs/phase-3/other-gems.md`, "P3-12i"; two mechanic fixtures)
  - [x] P3-12j the custom "Onslaught" line: custom-line rule (the only global, item-backed reading), `Poe1Quantity::Onslaught`, Onslaught once (PR #92, open, cascade on #91; `docs/phase-3/config-inputs.md`, "P3-12j"; two mechanic fixtures)
  - [x] P3-8d charge modifiers: `Poe1Quantity::Charge`, `charges::MODIFIERS`, PoB's count with minima and maxima (PR #93, open, cascade on #92; `docs/phase-3/stat-mapping.md`, "P3-8d"; two mechanic fixtures)
  - [x] P3-8e area damage, spell crit multiplier, area and light radius rows (PR #TBD, open, cascade on #93; `docs/phase-3/stat-mapping.md`, "P3-8e"; one mechanic fixture)
  - [ ] P3-8f weapon conditions ("while holding a staff/shield"; handoff Next action)
  - [ ] Next: the other gems left (golems and the Lightning Golem's Wrath, curses, Frost Bomb's exposure, warcries, Herald of Ash and Purity, Flesh and Stone, Arcane Surge, Righteous Fire); socket colour quality (bundle: the granted effect's `Attribute`); the custom "Onslaught" line; then the in-scope builds' unmapped stats and unresolved lines; Q13

## P3-14: corpus run and register
- [ ] `calc-parity` on public fixtures and corpus; every difference fixed or registered
- [ ] CI calc-parity job if Q1 is yes

## P3-15: report and gate
- [ ] `docs/phase-3/phase-3-report.md`; handoff, CLAUDE.md, ACTION-PLAN; folder to `dev/completed/`

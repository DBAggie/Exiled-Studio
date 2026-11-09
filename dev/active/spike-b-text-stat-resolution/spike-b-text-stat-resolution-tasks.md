Last Updated: 2026-11-09 (PR B0: plan)

# Spike B: tasks

## PR B0: plan
- [x] Create plan, context, tasks under `dev/active/spike-b-text-stat-resolution/`
- [x] Review with the `plan-reviewer` agent and apply findings
- [x] Maintainer answers the open questions (21 builds; `mod-backed` reported only; completed docs to `dev/completed/`)
- [x] Measure lookup-handler lines in the corpus (117 non-unique)
- [x] Move Spike A task docs to `dev/completed/`; record the A6b merge
- [x] Update handoff Current state / Next action, CLAUDE.md Status
- [x] Open PR (#13)

## PR B1: stat-description model and renderer (`loom-data`)
- [ ] Model types, serde loader from bytes, include-chain `Scope`, markup stripping
- [ ] Forward renderer (numeric handlers, lookup trait, placeholder formats, multi-line)
- [ ] Confirm each lookup handler's table in the schema (input for B1x)
- [ ] `parity-cli load-statdesc`; unit tests per construct; load counts on PoE1/PoE2; wasm32 build

## PR B1x: extractor roots for lookup tables
- [ ] Add missing roots (at least `PassiveTreeExpansionSkills`, `PassiveTreeExpansionJewels`)
- [ ] Determinism on both games; manifests replaced

## PR B2a: single-line matcher (`loom-parser`)
- [ ] Template index, normalization, value tokens, numeric handler inversion, conditions, hidden stats
- [ ] Unit and property tests

## PR B2b: multi-line, lookup handlers, checks
- [ ] Two-line variants; lookup-handler inversion
- [ ] `parity-cli round-trip` (PoE1, PoE2); failures categorized
- [ ] `parity-cli render-check` against PoB v2.67.2 mod text; differences categorized
- [ ] Count stat sets that depend on include precedence

## PR B3: PoB item reader and `resolve-corpus`
- [ ] Record the Steam build; re-extract if changed
- [ ] `loom-parser::pob_item` (prefixes, variants, alt variants, `ModRange`, magic base, flag lines, catalyst)
- [ ] `parity-cli resolve-corpus` with `report.json` / `summary.json`; scrubbing
- [ ] XML dependency through `cargo deny`
- [ ] Corpus run, two runs byte-identical, first rate, ambiguous set measured

## PR B4a: disambiguation and `mod-backed`
- [ ] Mods filter (domain, generation type, stat set; spawn tags tie-break only)
- [ ] `mod-backed` with listed exceptions

## PR B4b: categories, gate
- [ ] Category rules with positive evidence and tests; `uncategorized = 0`
- [ ] Spike report `docs/spikes/spike-b-resolution.md`, summary in `docs/spikes/spike-b/`, gate verdict
- [ ] ACTION-PLAN status and corpus count 21; ADR-007 count via the maintainer

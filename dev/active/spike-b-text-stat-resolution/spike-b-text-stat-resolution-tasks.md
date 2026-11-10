Last Updated: 2026-11-09 (PR B3)

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
- [x] Model types, serde loader from bytes, include-chain `Scope`, markup stripping
- [x] Forward renderer (numeric handlers, lookup trait, placeholder formats, multi-line)
- [x] Confirm each lookup handler's table in the schema (input for B1x; table in the plan)
- [x] `parity-cli load-statdesc`; unit tests per construct; load counts on PoE1/PoE2; wasm32 build
- [x] Review with `code-architecture-reviewer`; apply findings
- [x] Open PR (#14)

## PR B1x: extractor roots for lookup tables
- [x] Add missing roots: `PassiveTreeExpansionSkills`, `PassiveTreeExpansionJewels`, `PassiveTreeExpansionJewelSizes`, `IndexableSupportGems`, `IndexableNonActiveSupportGems`, `IndexableSkillGems` (the last two PoE1 only: no PoE2 file)
- [x] Determinism on both games; manifests replaced; cross-check summaries replaced (same families); `load-statdesc` unchanged
- [x] Open PR (#15)

## PR B2a: single-line matcher (`loom-parser`)
- [x] Template index, normalization, value tokens, numeric handler inversion, conditions, hidden stats
- [x] Unit and property tests (hand-rolled xorshift generators; no new dependency)
- [x] B1 leftovers: `invert_shown` (raw interval), `numeric_handler` binary search, `[Ref|...]` doc note
- [x] Review with `code-architecture-reviewer`; apply findings
- [x] Open PR (#17)

## PR B2b: multi-line, lookup handlers, checks
- [x] Multi-line variants (up to 8 lines); lookup-handler inversion by name expansion
- [x] Count samples with shadowed matches (`samples_with_shadowed_matches`); corpus lines in B3
- [x] Report `IndexStats` for `stat_descriptions` on PoE1 and PoE2
- [x] Lookup key forms: indexable gems `Index` (0-based row refuted); `passive_hash` graph id (hypothesis); `mod_value_to_item_class` row order refuted, unsupported
- [x] `parity-cli round-trip` (PoE1, PoE2); every sample under one outcome; 0 wrong readings
- [x] `parity-cli render-check` against PoB's exported mod text; 0 uncategorized
- [x] Count stat sets that depend on include precedence (item scope: 10 PoE1 / 26 PoE2 shadowed descriptions, all within the file)
- [x] Review with `code-architecture-reviewer`; apply findings
- [x] Open PR (#18)

## PR B3: PoB item reader and `resolve-corpus`
- [x] Record the Steam build; re-extract if changed (26530219, unchanged; no re-extract)
- [x] `loom-parser::pob_item` (prefixes, variants, alt variants, `ModRange`, magic base, flag lines, catalyst)
- [x] `parity-cli resolve-corpus` with `report.json` / `summary.json`; scrubbing
- [x] XML dependency through `cargo deny` (`roxmltree` 0.21.2, MIT OR Apache-2.0)
- [x] Corpus run, two runs byte-identical, first rate (79.1%), ambiguous set measured (380 lines, 289 texts)
- [x] Review with `code-architecture-reviewer`; apply findings
- [x] Open PR (#19)

## PR B4a: disambiguation and `mod-backed`
- [ ] Mods filter (domain, generation type, stat set; spawn tags tie-break only)
- [ ] `mod-backed` with listed exceptions

## PR B4b: categories, gate
- [ ] Category rules with positive evidence and tests; `uncategorized = 0`
- [ ] Class-requirement lines under `special-case/not-a-modifier`, class value kept on the item (maintainer decision 2026-11-09; plan step 4b)
- [ ] Spike report `docs/spikes/spike-b-resolution.md`, summary in `docs/spikes/spike-b/`, gate verdict
- [ ] ACTION-PLAN status and corpus count 21; ADR-007 count via the maintainer

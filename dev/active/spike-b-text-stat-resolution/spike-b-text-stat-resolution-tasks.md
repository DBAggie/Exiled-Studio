Last Updated: 2026-11-10 (PR B4c)

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

## PR B4x: item type files (inserted by the maintainer, 2026-11-09)
- [x] Measure: domain and generation type alone resolve at most 87 of 380 ambiguous lines; the rest need inherited spawn tags (`.it` files)
- [x] Survey `.it` grammar on both installs (PoE1 413, PoE2 277 files)
- [x] `tools/extractor/src/itemtype.rs`: parser, `item_types/**/*.json`, manifest section with chain checks; manifest format 4; `loom-data` reads 3 and 4
- [x] Cross-check derived tags with PoB's exported base tags (PoE1 1,090/1,090 equal; PoE2 differences all PoB's `genesis_tree_*` injection)
- [x] Determinism on both games (clean `c3c7e15`); earlier outputs unchanged; manifests and cross-check summaries replaced in `docs/spikes/spike-a/`
- [x] Review with `code-architecture-reviewer`; apply findings (no blockers)
- [x] Open PR (#21)

## PR B4a: disambiguation and `mod-backed`
- [x] Check PR #21 merge (`ad7a349`, main CI run 42739421598 green); Steam builds unchanged
- [x] Base tags: `InheritsFrom` chain (`tag`, `remove_tag`) plus `BaseItemTypes.TagsKeys`, from B4x's `item_types/` (`item_tags.rs`)
- [x] Mods filter (domain, generation type, stat set; spawn tags tie-break only; `values` tie-break; essence, influence, crafted guards) (`mod_filter.rs`)
- [x] `mod-backed` with listed exceptions (template-only share reported apart)
- [x] Corpus run from clean `b02003d`, two runs byte-identical; summary replaced in `docs/spikes/spike-b/`
- [x] Review with `code-architecture-reviewer`; apply findings (no blockers)
- [x] Open PR (#22)

## Maintainer decision (from B4a)
- [x] `Essences` and `CraftingBenchOptions`: option (a), extractor and filter in one PR (B4y, PR #23), 2026-11-10
- [x] Anoint source surveyed (none in the data); equivalence rule approved 2026-11-10 (step 4d)

## PR B4y: `Essences` for the Mods filter
- [x] Check PR #22 merge (`08b7d9c`, main CI run 42745121406 green); Steam build unchanged
- [x] Measure `CraftingBenchOptions`: class lists equal `Mods.CraftingItemClassRestrictions` (791/791); not added
- [x] `Essences` PoE1 root; Spike A report section 20; determinism (clean `674837f`), earlier outputs unchanged; manifest and cross-check summary replaced
- [x] Essence classes in `mod_filter` (fit, spawn-tags, guard for unknown classes); tests
- [x] Corpus run (clean `7c20caa`), two runs byte-identical; summary replaced
- [x] Review with `code-architecture-reviewer`; apply findings
- [x] Open PR (#23)

## PR B4c: cluster jewel small passives
- [x] Check PR #22 merge (`08b7d9c`, main CI run 42745121406 green)
- [x] Survey unsupported lookups on items (only `tree_expansion_jewel_passive`) and the anoint source (`BlightCraftingResults`: none)
- [x] `tree_expansion_jewel_passive` from game data; multi-line names in `loom-parser` and the renderer; tests
- [x] Corpus, round-trip (both games), render-check from clean `1cf7c3d`; summaries replaced
- [x] Review with `code-architecture-reviewer`; apply findings
- [x] Open PR (#24)

## PR B4d: anoint equivalence (after B4y)
- [ ] Rule in the filter; tests; corpus run; plan "Ambiguity" text

## PR B4b: categories, gate
- [ ] Category rules with positive evidence and tests; `uncategorized = 0`
- [ ] Class-requirement lines under `special-case/not-a-modifier`, class value kept on the item (maintainer decision 2026-11-09; plan step 4b)
- [ ] Categorize the 4 cluster lines that PoB's `Implicits:` count splits across the enchant and explicit sections (two-stat nodes; context file, "PR B4c"); check the first half cannot resolve alone to another node
- [ ] Spike report `docs/spikes/spike-b-resolution.md`, summary in `docs/spikes/spike-b/`, gate verdict
- [ ] ACTION-PLAN status and corpus count 21; ADR-007 count via the maintainer

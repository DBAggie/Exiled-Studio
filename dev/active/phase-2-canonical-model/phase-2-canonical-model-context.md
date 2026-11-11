Last Updated: 2026-11-10 (PR P2-1: domain model; P2-0 #41 merged as `fc4513d`)

# Phase 2: context

## Key files
- `docs/plan/ACTION-PLAN.md` "Phase 2: Canonical model and data": scope and gate. `docs/plan/IMPLEMENTATION-PLAN.md` "Phase 2": work packages and the extra gate items (invalid bundles rejected before signing; deterministic DTO/domain conversion).
- `crates/loom-parser/src/resolve/mod.rs`: reverse matcher (`Index`, `Resolution`, `Outcome`, `Unresolved`, `resolve_case_folded`).
- `crates/loom-parser/src/pob_item/mod.rs`: PoB item-text reader (`read`, `Item`, `Line`, `Modifier`, `Variants`, `magic_base`); moves to `loom-serialization` in P2-2 (plan decision 3).
- `tools/parity-cli/src/resolve_corpus.rs` (module docs record the scope, base, ambiguity, composed-line, and item-text rules), `mod_filter.rs`, `item_tags.rs`, `grafts.rs`, `properties.rs`, `categorize.rs`, `extract.rs` (`Read` closure over extract JSON): the pipeline P2-2 lifts.
- `crates/loom-data/src/stat_desc/`: stat-description model; `load.rs` (`SUPPORTED_MANIFEST_FORMATS` 3 and 4).
- `tools/extractor/src/extract.rs`: `MANIFEST_FORMAT` 4; table root lists `SPIKE_TABLES`, `LOOKUP_TABLES`, `LOOKUP_TABLES_POE1`, `MOD_SOURCE_TABLES_POE1`, `ITEM_LINE_TABLES_POE1` (the game-specific list in plan decision 2 must agree with these).
- `tools/oracle/fixtures/fixtures.json`: public PoB XML fixtures (PoB 3.13 by blob, ours, community by SHA-256); `tools/oracle/fixtures/community/SOURCES.md` (ADR-007 Amendment 1: name community fixtures only through the manifest).
- `docs/spikes/spike-b-resolution.md` section 6: the counts the lift and `resolve_build` must reproduce.

## ADRs this plan touches
- ADR-001: no PoE1 assumptions in shared crates; PoE2 data in the pipeline from Phase 2.
- ADR-003: Build aggregate, unknown content quarantined, DTOs separate from domain.
- ADR-005: successor JSON canonical, DTOs as source of truth with generated JSON Schema and TypeScript, sequential migrations, unknown content preserved opaquely, semantic legacy export; bundles: readable manifest and canonical data, compressed for distribution, derived data only.
- ADR-007: corpus local, scrubbed summaries only; community fixtures through the manifest.
- ADR-009: file-first JSON, originals never silently overwritten, imports and migrations atomic.
- ADR-010: builds pinned to their data version; signed bundles (signing in Phase 5).
- ADR-014: stable error codes; complete, partial, indeterminate, fatal.
- ADR-015 and Amendment 1: line is an enum (stat IDs, or a typed `kind`), `adapted` flag with the tolerance named; Phase 2 decides how the flag is persisted and whether export keeps the adapted text (plan decision 5).
- ADR-016: core crates wasm32, no fs; bundles through an abstract source.
No ADR needs an amendment for this plan. The crate graph is unchanged: the pipeline takes the `loom-domain` item, so `loom-parser` does not depend on `loom-serialization` (plan decision 4; plan-reviewer finding 1). Share codes stay first-class as ADR-005 says (decision 16).

## Decisions (plan, 2026-11-10; numbers match the plan's "Design constraints and decisions")
1. Type only what Phase 3 needs; keep everything else opaque, in position among typed siblings; keep every `<Item>` text child.
2. Typed values keep their source string; export writes it back while the value is unchanged. f64 equality compares source strings, so NaN is settled.
3. Items keep every line, typed by kind. A split-section pair is two lines, the second marked `continues`.
4. The pipeline takes the domain item. `pob_item` moves to `loom-serialization::legacy`, with a `to_domain` mapping. No new crate edge. Reason: AGENTS.md's graph stays as it is, and PoB's item text is a legacy format, like its XML.
5. No PoE1 or PoB rules in shared code. Mod domain scopes, generation types, influences, essence columns, and compositions are per-game bundle data (`item_rules`). PoE1-only tables go in the `poe1` section. PoB-only rules go in `legacy`.
6. Item export is generated from the model. Reason: copying raw text would pass the round-trip without testing the model.
7. Adapted lines persist the tolerance name; export keeps the source text.
8. Bundle v1 is our own canonical schema (mapped in the extractor): a directory with `bundle.json` (`signatures` reserved) plus canonical JSON. Compression, the container, and signing come in Phase 5.
9. `BundleSource` has `list()` and `read()`. `sha2` joins `loom-data`. No compression crate in core.
10. Real data stays local; CI uses synthetic PoE1 and PoE2 bundles.
11. `schemars` sits behind a `schema` feature; `tools/schema-gen` writes the schemas; TypeScript is generated from the JSON Schema.
12. `roxmltree` reads XML and our own writer writes it. Equality compares parsed trees, plus a closed list of extra rules.
13. File-first persistence with atomic writes; SQLite in Phase 4.
14. Sequential migrations over `serde_json::Value`; test-only migrations in Phase 2; the Build records its data bundle.
15. Error code families `BUNDLE_*`, `IMPORT_*`, `MIGRATE_*`, `PERSIST_*`. Import gives `Complete`, `Partial`, or `Fatal`; `Indeterminate` is a calculation state.
16. Share codes use `miniz_oxide` and are in the gate.

## plan-reviewer (2026-11-10) summary
Verdict: proceed with changes. All findings were applied to the plan.
- **Blocking 1:** moving `pob_item` would have added a `loom-parser` to `loom-serialization` edge, because the pipeline takes `pob_item` types (`resolve_corpus.rs`, `mod_filter.rs`). Fixed: the pipeline takes the domain item (decision 4), and step 2 now depends on step 1.
- **Blocking 2:** the lift would have moved PoE1 constants (`DOMAIN_SCOPES`, `ModGenerationType`, influence types, `ESSENCE_COLUMNS`) and PoB rules (`POB_SUFFIXED_BASES`) into a shared crate. Fixed: they become per-game bundle data or `legacy` code (decision 5); gate 3b and 3c check this in CI.
- **Should-fix, all applied:**
  - stronger PoE1-assumption checks (gate 3b, 3c);
  - several `<Item>` text children kept (decision 1);
  - normalization moved into step 5, with parsed-tree comparison (decision 12);
  - source strings kept for typed values (decision 2);
  - the round-trip defined with re-resolution, and lines counted as source lines (decision 3, step 8);
  - step 2 split into four PRs;
  - `COMPOSITIONS` placed as per-game data;
  - `BundleSource::list()` added, and `sha2` named;
  - our own canonical schema, with the step 3 table list cut to what Phase 2 reads;
  - share codes gated (ADR-005);
  - the exact Spike B fields named;
  - opaque content kept between typed siblings.
- **Nits, all applied:** a cross-OS hash comparison job (step 7); comments and processing instructions kept as opaque nodes; NaN (decision 2); `Indeterminate` explained (decision 15); `signatures` reserved in `bundle.json`.

## Open questions for the maintainer (the plan proceeds on the default)
- **Q1** Commit a real derived bundle (PoE1, maybe a subset) so CI can run resolution and the corpus-like checks? Default: no; maintainer-run with scrubbed summaries, as in Spikes A and B.
- **Q2** Bundle compression, single-file container, and signing in Phase 5 (with the update channel), Phase 2 only the canonical directory form and its validator? Default: yes.
- **Q3** SQLite index and cache deferred to Phase 4 (needed by the build list)? Default: yes.
- **Q4** Export keeps an adapted line's original text (not the game's)? Default: yes.
- **Q5** Move `pob_item` from `loom-parser` to `loom-serialization::legacy`, with the pipeline taking the domain item? Default: yes.
- **Q6** Share codes in the Phase 2 gate (P2-10), as ADR-005 makes them first-class? Default: yes.
- Still open from earlier phases (handoff): post the PoB issue (`local/upstream-issue-pob-vaal-variant.md`); Spike B per-PR assumptions.

## Facts found while planning (2026-11-10)
- PR #40 (ADR-015 Amendment 1) merged 2026-11-10 as squash commit `1f35323`; main CI run 42922016104 was in progress when this plan was written (check its result in P2-1).
- Local extracts: `local/determinism/poe1/run1/` and `local/determinism/poe2/run1/` (format 4) hold, among others, `Stats`, `Mods`, `BaseItemTypes`, `ItemClasses`, `Tags`, `Characters`, `Ascendancy`, `PassiveSkills`, `PassiveSkillMasteryEffects`, `PassiveSkillTrees`, `ActiveSkills`, `GrantedEffects`, `GrantedEffectsPerLevel`, `SkillGems`, `ClientStrings` in both games; `Essences`, `AlternateQualityTypes`, `BrequelGrafts`, `BrequelGraftGrantedSkillLevels`, `IndexableNonActiveSupportGems`, `IndexableSkillGems` in PoE1 only; PoE2 has `ClientStrings2`, `WieldableClasses`, `StartingPassiveSkills`, `SkillCraftingData`, and others not in PoE1.
- A PoB community fixture's top-level sections: `Build`, `Import`, `Party`, `Tree`, `TreeView`, `Notes`, `Skills`, `Items`, `Calcs`, `Config`, `TradeSearchWeights`, `TimelessData` (counted on `boneshatter-juggernaut.xml`).
- `tools/parity-cli` already depends on `roxmltree 0.21`.
- `loom-serialization` today depends only on `loom-domain`; `loom-parser` on `loom-domain` and `loom-data`.
- `tools/parity-cli/src/mod_filter.rs` holds PoE1 dat values as constants: `ModDomains` (1, 9, 10, 13, 16, 18-20, 28), `ModGenerationType` (1, 2, 3, 5, 10, 15-17, 21, 22, 24, 25, 28, 29, 31, 32), `NO_INFLUENCE`, `INFLUENCE_TYPES`, and `ESSENCE_COLUMNS` (22). `resolve_corpus.rs` has `DOMAIN_SCOPES` (34 tincture, 38 graft). All of these become `item_rules` bundle data (decision 5).
- Spike B `summary.json` fields that step 2 compares exist under these names in `resolve_corpus.rs`: `non_unique`, `unique`, `rate`, `resolved_kinds`, `adapted`, `ambiguous`, `categories`, `mod_backed`, `items`, `items_by_rarity`, `items_by_scope`, `line_kinds`.

## Decisions in P2-1 (domain model, 2026-11-10)
- **Layout.** `loom-domain/src/`: `build.rs` (`Build`, `Character`, `BuildPart`), `game.rs` (`Game`, `GameSpecific`, `Poe2Choices`, `DataVersion`), `poe1.rs` (`Poe1Choices`: bandit, pantheon; the only module that names them, checked by `shared_modules_should_not_name_poe1_only_choices`), `id.rs`, `value.rs` (`Sourced<T>`, `Number`), `opaque.rs` (`OpaqueNode`, `Ordered<T>`, `Entry<T>`, `Attributes`), `passive.rs`, `skill.rs`, `config.rs`, `item/mod.rs`, `item/resolution.rs`, `quarantine.rs`, `error.rs`, `tests.rs`. No dependencies.
- **Untyped content in place.** A list of typed children is `Ordered<T>`: typed entries with `OpaqueNode`s (element, text, comment, processing instruction) between them. A node's single typed child is placed by a layout marker among its opaque children: `Build::layout` (`BuildPart`), `PassiveSpec::layout` (`SpecPart::JewelSockets`), `Item::layout` (`ItemPart::Body`; the other `<Item>` texts and `ModRange` stay opaque around it). Attributes a typed node does not type are kept in order in its `attributes`.
- **Game data as names, not enums.** Classes, ascendancies, gems, skills, slots, influences, tree versions, stats are string IDs; nodes and build-local IDs are `u32`. `ModSection` has `Enchant`, `Implicit`, `Explicit` and `Other(String)`: P2-2a maps PoB's scourge, crucible, and class-requirement lists to `Other("scourge" | "crucible" | "class-requirement")`, so no PoE1 league section is a Rust variant; P2-2c's generation-type rules key on the name. `Rarity` keeps `Relic`. `Rarity::parse` and `names_base_in_name` stay in the PoB adapter.
- **Item lines.** `ItemLine { source, kind }`; `source` is the authority for export, the kind's fields are the adapter's reading (an edit replaces the whole line). Kinds: `Header(ItemClass | Rarity | Separator | Name | Base)`, `Spec { key, value, known }`, `VariantBase`, `Influence(InfluenceId)`, `Flag`, `Modifier(ModifierLine)`, `Unrecognized`. `ItemBody` also has `item_class`, `rarity`, `name`, `base`, `implicits`, `variants` (`VariantSelection` with `alternates`).
- **Modifier line state** (`LineResolution`): `Pending`; `Excluded(NotSelected | Custom | Section)`; `Resolved { result: Stats(Vec<StatLine>) | Typed(GrantedSkill | ItemProperty | CatalystQuality), adapted: Option<CaseFolded | SplitSection> }`; `Continues { line }`; `Unresolved(NoTemplate | NoVariantFits | OnlyShadowed | Ambiguous(candidates))`. `StatValue` mirrors the resolver's `RawValue` (`Exact`, `Ranged`, `Hidden { assumed }`, `Lookup`).
- **Continuations** (review blocker 1): `Continues` serves both a description over several consecutive lines and the second line of a split-section pair. `ItemBody::check_links` requires the target to be a resolved modifier line followed directly (among modifier lines) by its continuations, and a `SplitSection` line to be continued exactly once (`DOMAIN_BROKEN_CONTINUATION`).
- **Report-only pipeline details** (review should-fix 2): the `Mods` filter's evidence (`filter`, `filter_notes`, `dropped`), `backed`/`backed_by`, and `composed` are not in the model. A line the filter narrows to one description is `Resolved`; one it cannot narrow is `Unresolved(Ambiguous)` with the candidates' stats. The categories stay in the tool (plan step 2d).
- **Quarantine and completeness** are derived from the lines, never stored: `Build::quarantine()` lists `Unrecognized` lines, unknown specs, `Pending` and `Unresolved` modifier lines with stable codes (`LINE_*`); `Excluded`, `Resolved`, `Continues` are not quarantined. `Completeness` is `Complete` or `Partial`; `Indeterminate` is a calculation state and a fatal import is an error.
- **Values.** `Sourced<T>` keeps the source text until `set` replaces the value. `Number(f64)` compares bit patterns (`NaN == NaN`, `0.0 != -0.0`).
- **Errors.** `DomainError` with `code()`: `DOMAIN_GAME_MISMATCH` (`Build::set_game_specific` with another game's choices), `DOMAIN_BROKEN_CONTINUATION`.

## code-architecture-reviewer (P2-1, 2026-11-10) summary
- **Blocking 1, applied:** a same-section multi-line description had no valid form (`Continues` was accepted only after a split-section line). `Continues` now follows any resolved line; three tests added.
- **Should-fix, applied:** filter evidence declared report-only (above); `source` declared the export authority (no drift: edits replace the line); `Spec` has `value` and `ItemBody` has `item_class`; the class-requirement mapping documented on `ModSection::Other` and a test shows excluded lines are not quarantined.
- **Nits:** report-only fields documented on `Resolved`; `Number`'s signed zero documented. Not done: a range check on `AltVariant::index` (the adapter writes 0 to 4; nothing reads it yet).

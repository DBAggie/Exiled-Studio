Last Updated: 2026-11-10 (PR P2-5b: PoB XML items; stack #42 <- ... <- #50 <- P2-5b; session ended here at the maintainer's request)

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

## PR stack (maintainer decision 2026-11-10)
The maintainer asked for the Phase 2 PRs to be done unattended, one after another. Each PR branches from the previous PR's branch and targets it as its base. The maintainer merges the stack from the newest PR down to the oldest, then into `main`. Every PR keeps its own evidence and handoff update. If a maintainer decision comes up, the plan's default is taken and recorded. Work stops only for a failed gate, an ADR amendment, or a hard rule.

## Decisions in P2-2a (item input, 2026-11-10)
- **Move.** `pob_item` moved by `git mv` to `crates/loom-serialization/src/legacy/pob_item/`, with its tests unchanged. `loom-serialization` warns on `clippy::indexing_slicing`, as `loom-parser` does, because legacy text is untrusted.
- **`to_domain`** (`legacy/pob_item/domain.rs`):
  - re-exported with `domain_section` and `domain_rarity`;
  - spec and item-class values come from the reader's own `spec_value`, so they are parsed once, the same way;
  - the influence ID is the display name (`Shaper Item` becomes `Shaper`);
  - only the alternate variants the item has are kept.
- **Pipeline on the domain item.**
  - `resolvable` is now `LineResolution::Pending`, which equals the old `selected && !custom && !class-requirement` (`to_domain` sets the exclusions in that order).
  - `mod_filter::pob_section` maps `ModSection` back to PoB's `Section` by variant, with `Other` matched by name. An unknown `Other` name gives no filter context, so the line stays ambiguous and visible.
  - `section_name` gives the report's static name.
- **PoB-reader diagnostics** stay on PoB's reader. `unsupported_features` is counted at the call site from the read item, and unknown prefixes use `pob_item::is_known_prefix`.
- **`Rarity::names_base_in_name`** moved onto `loom_domain::Rarity` (review should-fix 2).
- **Evidence tooling.** `tools/parity-cli/scripts/compare-spike-b.py` compares the step 2 field list. `docs/phase-2/lift-evidence.md` collects the before-and-after runs of P2-2a to P2-2d.

## code-architecture-reviewer (P2-2a, 2026-11-10) summary
- **No blockers.** The crate graph is correct, and no PoB logic is in `loom-domain` or `loom-parser`.
- **Should-fix:**
  - `after_key` stripped every leading colon: replaced by the reader's `spec_value`;
  - the base-line rule was written twice: now `Rarity::names_base_in_name`;
  - `pob_section` matched by string: now matches by variant, with the `None` case documented.
- **Not changed:** the "stale comment" names the `domain_section` re-export, which exists.
- **Notes for 2b to 2d:**
  - `Spec { known }` and the `Custom`/`Section` exclusions are PoB concepts in the domain (accepted in P2-1);
  - `influence_type` still parses the source line, so switch it to the `InfluenceId` in 2c;
  - a `From` impl could replace `to_domain` when the XML adapter arrives;
  - the spec test now finds its line by key.

## Decisions in P2-2b (scope and bases, 2026-11-10)
- **`loom-parser::import`** is the pipeline's home (`import/mod.rs`, `import/base.rs`).
  - `ItemRules { default_scope, domain_scopes }` holds the per-game rules. For a domain listed more than once, the first entry wins.
  - `ItemData` is the trait over game data: `item_rules`, `base_names`, and `base(name) -> Option<(&str, &[BaseRow])>`, where the slice is never empty.
  - `BaseQuery { name, fallback }`.
  - `find_base` reads an empty slice as no base, and its errors carry ADR-014 codes (`IMPORT_BASE_NOT_FOUND`, `IMPORT_BASE_SCOPE_CONFLICT`). The corpus report's names (`not-in-base-item-types`, `scope-conflict`) are mapped in the tool.
- **`BaseRow.domain` is `i64`, with 0 when the data has none.** No rule lists 0, so the result equals the old `None`, and `mod_filter` is unchanged.
- **PoB naming stays PoB's.** `base_name` (the `Superior`/`Synthesised`/`Vestigial` prefixes) and `magic_base` stay in `legacy::pob_item`, and so does the suffixed-base list (`SUFFIXED_BASES`, `suffix_removed`, moved from the tool). The tool builds a `BaseQuery` from them.
- **PoE1 rules in the tool:** `tools/parity-cli/src/poe1.rs::item_rules()` (domains 34 and 38) holds them until step 3 puts them in the bundle. The harness's scope list comes from `ItemRules::scopes()`, and the summary's `scopes` map is keyed by `String`; its serialization is unchanged.
- **Gate 3c scan:** `crates/loom-parser/tests/no_poe1_identifiers.rs` scans every core crate's `src/` for PoE1-only identifiers by substring, a heuristic.
  - Allowed: any `poe1.rs` or `poe1/` module, `loom-serialization/src/legacy/`, and `tests.rs` files.
  - The list grows with each lift PR.
  - It fails on an injected identifier (checked).

## code-architecture-reviewer (P2-2b, 2026-11-10) summary
- **No blockers.** `find_base` matches the old `Bases::find`.
- **Should-fix, applied:**
  - an empty row slice is no base (test added);
  - the allow-list is narrowed (`/tests/` directories dropped, along with the redundant `loom-domain` clause);
  - the scan is documented as a heuristic.
- **Nits, applied:** ADR-014 codes on `BaseError`; "first entry wins" documented.
- **Design notes, decided for later PRs:**
  - **P2-2c:** mod rows, item tags, grafts, and properties come through a separate `ModData` trait (or sibling traits) next to `ItemData`, so `find_base`'s bound stays minimal.
  - **P2-6:** `ItemData` uses `impl Iterator` in a trait method, so it is not object-safe. `resolve_build` takes it as a generic. If the app layer needs `dyn`, decide before P2-6 (for example, `base_names` returning a `Vec` or a boxed iterator).
  - **P2-3/P2-6:** `BaseRow::implicits` are row indices into `Mods`. The bundle's canonical model should use stable mod IDs and interned tags.

## Decisions in P2-2c (Mods filter, 2026-11-10)
- **`loom-parser::import::mods`** holds the Spike B filter logic: `ModRow`, `Mods` (an index filled by `from_rows`, plus `locality`), `Context` (with `rules`), `LineContext { section: ModSection, flags }`, `fits`, `filter`, `numbered_stat`, `unnumbered`, `backing`.
  - The Spike B module docs, with the evidence for each step, stay on the tool's `mod_filter.rs`.
  - `numbered_stat`, `type_spawns_on`, and `essence_unknown` are public for the tool's tests.
- **`ModRules` (in `ItemRules.mods`)** holds every PoE1 number and name the filter used, as data:
  - `fits: Vec<FitRule>`. Each rule has a `ModSection`, an optional line flag, and an optional rarity. For a line, the first matching rule is the only one used, and the row fits when one of its `FitClause`s holds (generation types, `DomainMatch { own, fixed, related }`, `craft_class`, `essence_class`).
  - The gates `spawn_exclusion` (with `spawn_excluded_generations`) and `base_implicit` are `Unflagged { section, flags }`.
  - `no_spawn_flags`, `local_carriers`, and `influences` (keyed by the domain `InfluenceId`).
  - `ModRules::validate` refuses empty rules, empty clauses, and shadowed rules, with `IMPORT_MOD_RULES_*` codes.
  - The PoE1 rules are `tools/parity-cli/src/poe1.rs::mod_rules()`. The `domain` and `generation` constant modules and `NO_INFLUENCE` moved there.
- **Kept in the tool, by decision:**
  - `item_tags.rs` and the essence-column map (`ESSENCE_COLUMNS`). They load from extractor JSON, and `loom-parser` must not gain `serde_json`. The step 3 converter must precompute base tags (`BaseRow.tags`) and essence classes (`ModRow.essence_classes`) into the bundle.
  - Loading `Mods`, `Stats`, and `Tags` rows (`mod_filter::load`), `eldritch_share`, and PoB section naming.
- **No `ModData` trait** (this replaces P2-2b's design note). `Mods` is a concrete core index, so any loader (extract today, bundle in P2-6) fills it with `from_rows`.
- **Tool tests:** the 253 Spike B tests were kept. A test-local `LineContext` with the old bool shape converts to the core context, and the `fits`, `filter`, and `backing` wrappers keep the call sites.
- **Gate 3c list** gained the moved constant names (`NO_INFLUENCE`, `FLASK_ENCHANTMENT`, `WEAPON_TREE`, `SCOURGE`, `EXARCH`, `UNVEILED`, `DELVE`, `ABYSS_JEWEL`).

## code-architecture-reviewer (P2-2c, 2026-11-10) summary
- **No blockers.** The reviewer checked the old `fits` against `mod_rules()` case by case, and the gates are equivalent.
- **Should-fix, applied:**
  - sections are typed `ModSection` (a typo can no longer drop content silently);
  - `validate`, plus the PoE1 test that every PoB section except class requirement has a rule;
  - rule order documented.
- **Open, for P2-3 and P2-6:**
  - `ModRow`'s crafting-class and essence fields, and the `craft_class`/`essence_class` clause flags, are PoE1-shaped. The bundle model may need them optional, or PoE2 may need other clause predicates. Decide when the PoE2 bundle is built (P2-3).
  - `mods.rs` has a large public surface (open-field structs). Consider `#[non_exhaustive]` and constructors once the bundle loader (P2-6) is the only builder.
  - An influence the rules do not name gives no value. The item's other influences still count, as in Spike B.

## Decisions in P2-2d (compositions, grafts, properties, tolerances, 2026-11-10)
- **`loom-parser::import` gains three modules:**
  - `grafts`: `Grafts::new(bases, wrapper, levels)` and `resolve(base, line) -> GrantedSkill { graft, level: u32 }`;
  - `properties`: `Properties::new(&TextRules, texts, Option<Vec<QualityType>>)` and `resolve(line) -> PropertyMatch::{Template, Label, Quality}` with `kind()`. `missing()` returns the typed `Missing::{Text, QualityType}`;
  - `tolerance`: `case_folded(index, text, &resolution, composed)` and `split_section(index, &body, n)`.
- **`split_section` reads the body as read.** It reads `Pending` modifier lines, which is what the old `resolvable` checked. P2-6 must call it before writing resolutions into the body, or pass a copy.
- **`TextRules` (`ItemRules.text`)** holds `compositions`, `property_ids`, `label_ids`, and `granted_skill_wrapper`. The PoE1 values (`PROPERTY_IDS`, `LABEL_IDS`, `GRAFT_WRAPPER_ID`, `COMPOSITIONS`) are in `tools/parity-cli/src/poe1.rs`, and `categorize` reads compositions through the rules.
- **The tool keeps** table loading (`grafts::load`, `properties::load`), the summary inputs, the report's `detail` strings (`properties::property`, and the graft detail in `resolve_corpus`), and the categories and legacy-text evidence (plan). `properties::missing` maps `Missing::QualityType` to Spike B's `AlternateQualityTypes:<description>`.
- **Domain change:** `TypedResolution::CatalystQuality.quality` is now the quality's text (`String`), not `i64`. The game data allows a decimal part (`number()`), and the line keeps its text, like `PropertyValue.text`.
- **For P2-6 (mapping onto the domain):**
  - `PropertyMatch::Template` maps to `ItemProperty { property, values: PropertyValue { slot, text } }`.
  - `Label` maps to `ItemProperty` with one value.
  - `Quality` maps to `CatalystQuality { quality_type, stats (StatValue::Exact), quality }`, or to `ItemProperty` when it has no stats.
  - `GrantedSkill` maps to `GrantedSkill { source: graft id, gem, level }`. The gem is looked up from the skill's base name through the bundle's graft data, so P2-3 should store the gem ID with each graft.
- **Step 2 is complete.** The lifted pipeline gives Spike B's exact result: the 12 fields are equal and `report.json` is byte-identical (`docs/phase-2/lift-evidence.md`).

## code-architecture-reviewer (P2-2d, 2026-11-10) summary
- **No blockers.** The moved logic is equivalent: graft conflicts, property order, quality dedup, the case-fold conditions, and the split-section neighbour order and filter.
- **Should-fix, all applied:**
  - documented that `split_section` needs the body before resolution is written, and added tests (earlier neighbour wins; an excluded neighbour does not pair; case-fold on a resolved line);
  - the quality is text in the domain;
  - the granted-skill level is `u32`;
  - `Missing` is an enum.
- **Nits, not done:**
  - `PropertyMatch::kind` repeats the domain's kind names;
  - the `composed: bool` argument;
  - the graft-named types and the fixed `{1}`/`{0}` wrapper shape are PoE1-flavoured. They are game-data shaped, the IDs are data, and "Graft" is not in the gate 3c list. Rename when PoE2 data shows the general form;
  - `categorize::compositions()` rebuilds the rules on each call (tool only).

## Decisions in P2-3a (bundle model and loader, 2026-11-10)
- **Rule types moved down.** The rule types (`ItemRules`, `TextRules`, `ModRules`, `FitRule`, `FitClause`, `DomainMatch`, `Unflagged`, `LocalCarriers`, `ModRulesError`) live in `loom-data::rules`, with serde. `loom-parser` re-exports them under their old paths, and `Unflagged` matching is a parser-private function. Reason: `loom-data` sits below `loom-parser`, and a bundle carries its rules (`data/item_rules.json`).
- **`ModSection` in JSON is its name.** `enchant`, `implicit`, and `explicit` are the shared sections; any other name is `Other`. `ModRules.sections` lists a game's non-shared names, so `validate` catches typos (PoE1: `scourge`, `crucible`, `class-requirement`).
- **Bundle v1 layout.** The layout table is in `loom-data/src/bundle/mod.rs`:
  - `bundle.json` holds format 1, `data_schema` 1, the game, `Provenance` (Steam build, index SHA-256, extractor commit, decoder, dat schema), every other file with its SHA-256 and size, and `signatures` (reserved; the validator refuses a non-empty one in format 1).
  - `data/*.json` holds the canonical types, keyed by ID with no row indices.
  - `stat_descriptions/*.json` uses the extractor's own format.
  - `poe1/*.json` holds quality types and grafts (with gem IDs and levels). Essences need no file, because their classes are precomputed into `ModEntry.essence_classes`.
  - JSON has sorted keys and a stable row order.
- **Loader.** `Bundle::load(&dyn BundleSource)`:
  - checks the format, then the game, then the manifest paths (plain relative paths, unique, no `poe1/` in a PoE2 bundle), then the required files;
  - parses every listed file, with `BUNDLE_*` codes;
  - does not verify anything: hashes, sizes, references, and unlisted files are P2-4's validator. The app uses a bundle only after the validator accepts it (a gate for P2-6).
  - `sha2` enters with the validator.
- **Deriving `ModRow` in P2-6.** `ModRow.unset` comes from a mod's `(0, 0)` stats, and `type_spawn` is computed by `Mods::from_rows` (it already is). The converter does not precompute them. P2-6 maps class and mod-type IDs onto the parser's row numbers, or the parser moves to IDs.
- **Fixtures.** Synthetic bundles live in `crates/loom-data/tests/fixtures/bundle-{poe1,poe2}/`, written by `make_bundles.py` (made-up rows), and `.gitattributes` keeps their bytes exact.

## code-architecture-reviewer (P2-3a, 2026-11-10) summary
- **No blockers.** Rules in `loom-data` are the right home, the ADRs are conformant, and the design is not over-engineered.
- **Should-fix, applied:**
  - path safety and duplicate paths in the loader;
  - the "unverified" doc warning and the P2-6 gate;
  - `type_spawn` and `unset` derivation decided (above);
  - section names checked by `validate`.
- **Nits:**
  - **Applied:** a `poe1/` file in a PoE2 bundle is refused in the loader.
  - **Not done:**
    - the `Lookup` linear scans (index them when P2-6 calls them per line);
    - the double parse of `bundle.json` (it lets the format check come first);
    - the tuple-shaped JSON entries;
    - a `BUNDLE_SOURCE_UNREADABLE` test (the code exists);
    - the duplicated game field.

## Decisions in P2-3b (bundle converter, 2026-11-10)
- **Where the converter lives:** `loom-parity-cli bundle <extract> --out <dir>` (`tools/parity-cli/src/bundle.rs`), not `loom-extractor bundle` as the plan said. Every extract reader it needs is in parity-cli: tables, `item_tags`, `lookup`, the essence-column map, the graft and quality loaders, and the PoE1 rules. Tools depend only on core crates, so the extractor cannot use them. The extractor stays the dat-to-extract step.
- **Mapping:**
  - IDs replace row indices: mod types by `ModType.Name` (the table has no `Id`), classes by `ItemClasses.Id`, implicits by `Mods.Id`, and tags by `Tags.Id`.
  - `(0, 0)` stats are kept in `ModEntry.stats`.
  - Stat-description files keep their path relative to the extract below `stat_descriptions/`, because PoE2 has files of one name in several folders.
  - Both schemas' column names are read through `column()` and `StatSlot` (PoE2 `Stat<n>` with a `Stat<n>Value` `[min, max]` pair).
- **Checks and counts:**
  - The converter fails on a duplicate `Mods`, `Stats`, or `ItemClasses` ID.
  - The summary counts dangling references, skipped nameless bases, stat slots without a range, missing client texts, and duplicate `ModType` names, and carries the lookup report.
  - On 26530219 and 26476662, everything counts 0 except the nameless bases (PoE1 420, PoE2 421).
- **Output:** written to `<dir>.partial`, loaded back with `Bundle::load`, then renamed.
- **PoE2 item rules (`poe2.rs`) are a hypothesis.** They use the schema enums both games share and only the fit rules both games have. There are no domain scopes, client texts, or sections. Measure them when PoE2 item import starts (backlog, ADR-005). A PoE2 bundle with 0 client texts is expected, not an error.

## code-architecture-reviewer (P2-3b, 2026-11-10) summary
- **No blockers.** The deviation is justified, the summaries are scrubbed, and the output is deterministic by construction.
- **Should-fix, applied:** fail on duplicate IDs; count dangling references, skipped nameless bases, stat slots without a range, and missing client texts; keep the lookup report; write to a partial directory and rename.
- **Recorded for P2-6:** rebuilding `Mods` and `BaseRow` from a bundle must equal the extract readers for both games (tasks file). `(0, 0)` stats go back to `unset`.
- **Nits, not done:**
  - Nothing in the manifest marks the PoE2 rules as unmeasured (the module docs say so);
  - there is no end-to-end converter fixture test (the in-session runs on both real extracts are the evidence; P2-6's equality test will be one).

## Decisions in P2-4 (bundle validator, 2026-11-10)
- **`loom_data::bundle::validate`** returns a `Report` of `Finding { code, at, detail }`. It runs these checks in order:
  - structural: SHA-256 and size per listed file, unlisted files, empty `signatures`;
  - schema: the `Bundle::load` error code;
  - referential: `BUNDLE_DANGLING_REF`, and `BUNDLE_MISSING_CLIENT_TEXT`;
  - domain: `BUNDLE_DUPLICATE_ID`, and `BUNDLE_ITEM_RULES`;
  - smoke: `BUNDLE_SCOPE`. Each rule scope is exactly one file, found by stem (PoE1 `.txt`, PoE2 `.csd` sources); it builds; and one description renders with every stat at 1.
  
  Structural findings stop before the schema check, and a schema finding stops the rest. An unknown format or a malformed manifest falls through to the loader's code.
- **`sha2` is in `loom-data`** (pure Rust; `cargo deny` and wasm32 ok).
- **The `poe1/` section is optional for PoE1** (an extract without its tables gives none). PoE2 must not have it (the loader refuses).
- **Gate 3d** is a parity-cli test. It reads `tools/extractor/src/extract.rs` as text (tools cannot depend on each other) and checks two things: each converter PoE1-only table (`POE1_ONLY_TABLES`) is in a `*_POE1` list, and none is in `SPIKE_TABLES` or `LOOKUP_TABLES`.
- **`bundle-check <dir>`** uses a folder source that refuses non-plain paths and symbolic links.
- **Evidence:** the 13-case mutation set (each case gives exactly its code). 0 findings on the real PoE1 26530219 and PoE2 26476662 bundles and on both fixtures (`docs/phase-2/bundle-check-*.json`).

## code-architecture-reviewer (P2-4, 2026-11-10) summary
- **No blockers.** The order of the stop and continue checks is correct, and an unknown format gives exactly `BUNDLE_FORMAT`.
- **Should-fix, applied:** the sample render and the unique scope file; the optional `poe1/` section documented; the gate 3d scan hardened; symbolic links refused.
- **Nits, not done:**
  - a non-plain manifest path gives the source's `NOT_FOUND` before the loader's `BAD_PATH`;
  - a changed file size gives two codes (hash and size);
  - the hash comparison is case-sensitive, and the converter writes lowercase;
  - `list()` failures have an empty `at`.

## Decisions in P2-5a (PoB XML structure, 2026-11-10)
- **`loom_serialization::legacy::pob_xml`** has `import`, `export`, `xml_equal`, `NORMALIZATIONS`, and `kept_whole`. `roxmltree` 0.21 was added to `loom-serialization` (`cargo deny` and wasm32 pass).
- **Typed:**
  - `<Build>`: class, ascendancy, level, main socket group; bandit and pantheon into `GameSpecific::Poe1`;
  - `<Tree>`: `activeSpec`;
  - `<Spec>`: title, tree version, nodes, mastery effects as `Sourced` lists, and `<Sockets>`/`<Socket>`, where `itemId="0"` means no item;
  - `<Skills>`: `activeSkillSet`, with `<SkillSet>`, `<Skill>`, and `<Gem>`;
  - `<Config>`: `activeConfigSet`, with `<ConfigSet>` and `<Input>` (exactly one of `boolean`, `number`, `string`).
  
  Specs get `SpecId` by order.
- **Kept whole (the fallback):** an element whose typed fields do not read is kept whole and listed by `kept_whole`, the import's visible quarantine for typed content (`IMPORT_KEPT_WHOLE`). That covers a number that is not canonical, a boolean that is not `true` or `false`, a missing required attribute, or children where none are expected.
  - Required: a gem's `level`, `quality`, and `enabled`, and a group's `enabled`. PoB reads a missing `enabled` as false and a missing `quality` as nil, but typing them would not round-trip a missing attribute.
  - A second typed section (two `<Tree>`) is kept whole.
- **Opaque content:** every other element, attribute, comment, and processing instruction stays in place. Content before and after the root goes into `Build.prolog` and `Build.epilog`, and the root's attributes into `Build.attributes`.
- **Whitespace:** whitespace-only text between elements is layout and is dropped. In mixed content (an element with text), every text child is kept, the export writes no layout there, and `xml_equal` compares it exactly. This matters for P2-5b's item text around `<ModRange>`.
- **`xml_equal`:** compares parsed trees with two normalizations, attribute order and whitespace between elements (not in mixed content). The parser already reads CRLF as LF and resolves entities, so no line-ending rule is needed.
- **Writer:** writes typed attributes first, then the kept ones in order. An empty list attribute without source text is omitted (a source `nodes=""` is written back). Escaping: `& < > "`, and `
 	 
` in attributes; `
` in text.
- **Domain:** `Character.level` is optional (PoB may omit it; a consumer defaults it to 1). `Build` gained `attributes`, `prolog`, and `epilog`.
- **Known non-goals:** namespaces, DOCTYPE, and the declaration's encoding and standalone attributes are not kept. PoB writes none.

## code-architecture-reviewer (P2-5a, 2026-11-10) summary
- **No blockers.**
- **Should-fix, applied:**
  - exact mixed content (otherwise item-text changes would pass in P2-5b);
  - `kept_whole`, so a fallback is visible;
  - canonical numbers;
  - `itemId="0"` is no item;
  - the line-endings normalization removed.
- **Kept by decision:** required `enabled` and `quality`, as above.
- **Nits, not done:**
  - `children()` clones nodes just to test them;
  - `set_game_specific(..).ok()`: it is always PoE1, so it cannot fail;
  - for duplicate sections, the first typed one wins, and an earlier one that does not read is kept whole.

## Decisions in P2-5b (PoB XML items, 2026-11-10)
- **Items:** `<Items>` is typed (`legacy/pob_xml/items.rs`).
  - An `<Item>` reads its body from the last non-blank text child (PoB's rule), through `pob_item::read` and `to_domain`.
  - Every other child (texts, including whitespace-only ones; `<ModRange>`; anything else) stays in place around `ItemPart::Body`.
  - Item sets and slots are typed (`itemId="0"`: empty). `<SocketIdURL>` and anything else stay in place.
- **Item text export:** the text is generated from the lines' sources plus `Item::text_spacing` (a new domain field: the whitespace before, between, and after the lines as read). If `spacing()` cannot rebuild the text, the item is kept whole. If the spacing does not fit, the export joins the lines with `
`.
- **`mainActiveSkill="nil"`** (PoB's unset value) stays an untyped attribute, and the group is typed.
- **Evidence:** all 24 fixtures pass with every section typed and nothing kept whole. In-session: the corpus 21/21 (1,055 items) and PoB 3.13 5/5 (100 items).

## P2-5b review findings, applied (code-architecture-reviewer, 2026-11-10; no blockers; applied 2026-11-10 on PR #51)
All applied as docs and tests; no behavior change. Nit 1: `text_spacing` stays on `Item`, documented as legacy-XML formatting only. Nit 2: documented in `items.rs` for Phase 3. Nit 4: documented on `spacing()`.

1. **The `spacing()` invariant needs a comment** in `items.rs`. The reader trims each line and drops empty ones, so a line starts and ends with non-whitespace, and `find` cannot match inside a whitespace gap. Add tests for a non-breaking-space gap and a CRLF body.
2. **Stale `text_spacing`:** document on `Item::text_spacing` that any edit to an item's lines must clear it, because the writer checks only the length. Add a test.
3. **The `
` join fallback drops blank lines and indentation.** Document it.
4. **Nits:**
   - Consider moving `text_spacing` into the layout (`ItemPart::Body { spacing }`), or document it as legacy-XML formatting only, which canonical JSON may ignore.
   - Phase 3 needs an index of item IDs and slots, with diagnostics for dangling or duplicate IDs. `itemId="0"` collides with a real item 0.
   - Missing tests: an item with no body text but a `<ModRange>`; a non-numeric item id (kept whole); a `<Slot>` with children (opaque).
   - Rust's `trim` is Unicode-aware, but PoB's `%s` is ASCII-only, so a line of only non-breaking spaces may differ from PoB. This comes from Spike B; note it.

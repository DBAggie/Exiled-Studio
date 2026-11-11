Last Updated: 2026-11-11 (PR P2-11: Phase 2 report and gate verdict; Phase 2 complete)

# Phase 2: tasks

## PR P2-0: plan
- [x] Read ACTION-PLAN, IMPLEMENTATION-PLAN Phase 2, ADRs 001, 003, 005, 006, 007, 009, 010, 012, 013, 014, 015 (with Amendment 1), 016
- [x] Survey the crates and tools (import pipeline in `tools/parity-cli`; empty domain and serialization crates)
- [x] Write plan, context, tasks
- [x] plan-reviewer; record its summary and the changes in the context file (2 blocking, 11 should-fix, 5 nits; all applied)
- [x] Update handoff (Current state, Next action; record PR #40's merge), CLAUDE.md Status, ACTION-PLAN Next action
- [x] Open PR (#41); write the PR number into the docs right after

## PR P2-1: domain model (step 1)
- [x] Check main CI run 42922016104 (PR #40, green) and PR #41's merge (`fc4513d`, main CI run 42926701060 green)
- [x] `Game`, IDs, Build, character, `GameSpecific`, passive spec, skills, items, item sets, slots, config inputs
- [x] Modifier line enum (stat IDs, typed `kind`, unresolved with reason), `Adapted`
- [x] Every item line typed by kind; `continues` for split pairs; typed values keep source strings
- [x] `Opaque` container (content in place between typed siblings), quarantine, completeness
- [x] Tests (PoE1 and PoE2 Builds); wasm32
- [x] `code-architecture-reviewer`; findings recorded in the context file
- [x] Open PR (#42); write the PR number into the docs right after

## PR P2-2a: item input (step 2a)
- [x] Move `pob_item` to `loom-serialization::legacy::pob_item` (tests unchanged)
- [x] `to_domain` mapping (PoB-only rules stay in `legacy`); parity-cli pipeline on the domain item
- [x] Spike B field list equal (run in the session: all 12 fields, `report.json` byte-identical; `docs/phase-2/lift-evidence.md`)
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#43, base `feat/p2-1-domain-model`); write the PR number into the docs right after

## PR P2-2b: scope and bases (step 2b)
- [x] `ItemData` trait (bases, `item_rules`); move base lookup and scope to `loom-parser::import`
- [x] parity-cli `ItemData` over the extract, with a `poe1` rules module; start the PoE1-identifier scan test
- [x] Spike B field list equal (run in the session; `docs/phase-2/lift-evidence.md`)
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#44, base `feat/p2-2a-item-input`); write the PR number into the docs right after

## PR P2-2c: Mods filter and item tags (step 2c)
- [x] Move the `mod_filter` logic to `loom-parser::import::mods`; mod domains, generation types, line flags, carrier tags, and influences become `ModRules` data in `item_rules`
- [x] Decided: `item_tags` and the essence-column map stay in the tool (extract loading; no `serde_json` in `loom-parser`; the step 3 converter precomputes base tags and essence classes)
- [x] Spike B field list equal (run in the session; `docs/phase-2/lift-evidence.md`)
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#45, base `feat/p2-2b-scope-bases`); write the PR number into the docs right after

## PR P2-2d: compositions, grafts, properties, tolerances (step 2d)
- [x] Move grafts, properties, case-fold, split-section to `loom-parser::import` (`grafts`, `properties`, `tolerance`); `COMPOSITIONS`, the property and label ids, and the graft wrapper as per-game `TextRules` data
- [x] Spike B field list equal (run in the session; `docs/phase-2/lift-evidence.md`, with the step 2 result)
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#46, base `feat/p2-2c-mods-filter`); write the PR number into the docs right after

## PR P2-3a: canonical data model, bundle v1, loader (step 3a)
- [x] Canonical types (our schema, IDs not row indices) for what import reads: stats, mods, base items (precomputed tags), item classes, client texts, lookup tables, stat-description files; `item_rules` (the rule types moved to `loom-data::rules`); `poe1` section (quality types, grafts with gem IDs)
- [x] `bundle.json` (reserved `signatures`), `BundleSource` (`list`, `read`) and `MemorySource`, loader with `BUNDLE_*` codes
- [x] Synthetic PoE1 and PoE2 bundles (`crates/loom-data/tests/fixtures/`, `make_bundles.py`) and loader tests
- [x] Decided: `sha2` and hash checks move to the validator (P2-4), which is the first code that hashes
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#47, base `feat/p2-2d-compositions-tolerances`); write the PR number into the docs right after

## PR P2-3b: bundle converter (step 3b)
- [x] Decided: the converter is `loom-parity-cli bundle`, not `loom-extractor bundle` (the extract readers it needs are in parity-cli; tools depend only on core crates)
- [x] Map the extract's dat-schema rows to the canonical types, for both schemas' column names; write `item_rules` (PoE1 `poe1.rs`; PoE2 `poe2.rs`, a recorded hypothesis)
- [x] Precompute base tags (item types), essence classes, lookup tables, graft gems
- [ ] Gate 3d (the game-specific table list agrees with the extractor's lists): moved to P2-4, with the validator's domain checks
- [x] Two `bundle` runs per game byte-identical (in-session, also from a clean tree); scrubbed summaries `docs/phase-2/bundle-*.json`
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#48, base `feat/p2-3a-bundle-model`); write the PR number into the docs right after

## PR P2-4: bundle validator (step 4)
- [x] Structural, schema, referential, domain, smoke checks with `BUNDLE_*` codes (`loom_data::bundle::validate`; `sha2` in `loom-data`, `cargo deny` ok)
- [x] `loom-parity-cli bundle-check`
- [x] Mutation set in CI (`crates/loom-data/tests/bundle_validate.rs`, 13 cases, each its one code)
- [x] Gate 3d: the converter's PoE1-only tables are in the extractor's `*_POE1` lists (parity-cli test)
- [x] Real PoE1 and PoE2 bundles, 0 findings (in-session; `docs/phase-2/bundle-check-*.json`)
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#49, base `feat/p2-3b-bundle-converter`); write the PR number into the docs right after

## PR P2-5a: PoB XML import and semantic export, structure (step 5a)
- [x] Typed Build, Tree, Skills, Config; opaque rest in place; fallback to opaque for elements whose typed fields do not read; Items whole until 5b
- [x] Export from the model; own XML writer
- [x] `xml_equal` (parsed trees) and the closed normalization list (attribute order, whitespace between elements, line endings)
- [x] Round-trip on the 24 public fixtures (CI) and, in-session, the 21-build corpus and PoB's 5 3.13 builds; mutation tests (unknown attribute, element between gems, section, comment, instruction, prolog); malformed-input codes
- [x] `code-architecture-reviewer`; findings in the context file
- [x] Open PR (#50, base `feat/p2-4-bundle-validator`); write the PR number into the docs right after

## PR P2-5b: PoB XML items, item sets, slots (step 5b)
- [x] `<Items>`: items (every `<Item>` child kept; the last non-blank text read with `pob_item::to_domain`; `ModRange` and other children opaque in place), item sets, slots; export generated from the item lines plus `Item::text_spacing` (decision 6)
- [x] Item round-trip and mutation tests (several item texts, comment between items, unknown slot attribute, empty slot, export from the lines); all 24 fixtures with every section typed and nothing kept whole; corpus 21/21 (1,055 items) and PoB 3.13 5/5 (100 items) in-session
- [x] `code-architecture-reviewer`: no blockers; findings recorded in the context file ("P2-5b review findings, applied")
- [x] Review findings applied (docs, 6 tests); fixtures, corpus 21/21, and 3.13 5/5 round-trips rerun
- [x] Open PR (#51, base `feat/p2-5a-pob-xml-structure`); write the PR number into the docs right after

## PR P2-6a: bundle-backed `ItemData` (step 6, first half)
- [x] `loom_parser::import::BundleData`: `Mods` (split `(0, 0)` stats into `unset`; class, mod-type, and implicit IDs onto row numbers), base rows, local stats, grafts and gems, properties and quality types, compositions, an index per scope; `Bundle::scope_root` shared with the validator
- [x] Tests on the synthetic PoE1 and PoE2 bundles (`crates/loom-parser/tests/bundle_data.rs`)
- [x] `bundle-equal`: the bundle-built data equal `mod_filter::load` and `Bases::load` on the PoE1 extract (40,355 mod rows, 4,309 base names, 0 mismatches). PoE2: the harness's extract readers are PoE1-only, so the PoE2 check is the ignored real-bundle test (builds, every scope indexes, counts match)
- [x] `resolve-corpus --bundle`: the 12 step 2 fields equal Spike B, `report.json` byte-identical (`docs/phase-2/lift-evidence.md`)
- [x] `code-architecture-reviewer`: no blockers; should-fix and nits applied (context file)
- [x] Open PR (#52, base `feat/p2-5b-pob-xml-items`); write the PR number into the docs right after

## PR P2-6b: `resolve_build` (step 6, second half)
- [x] `loom_parser::import::resolve_build(&mut Build, &data, base_of)` and `Resolver` over every item: lines to `LineResolution`s (stats, typed `kind`, or unresolved with a reason; `adapted`; `Continues`); completeness; the bundle's `DataVersion` recorded; `ResolveError` codes
- [x] The PoB base-name rules reach the core as an `ItemBaseName` from the adapter through a closure (`pob_item::item_base`); domain `UnresolvedReason::NoBase` (`LINE_NO_BASE`)
- [x] Outcome tests on synthetic PoE1 and PoE2 bundles (gate 3b), each outcome and `kind`; source lines in equal source lines out (`crates/loom-parser/tests/resolve_build.rs`; fixtures grown in `make_bundles.py`)
- [x] `resolve-corpus --bundle` cross-check: 6,061 of 6,061 corpus modifier lines equal the harness; 12 fields and `report.json` unchanged (`docs/phase-2/lift-evidence.md`)
- [x] `code-architecture-reviewer`: no blockers; should-fix applied (context file)
- [x] Open PR (#53, base `feat/p2-6-resolve-build`); write the PR number into the docs right after

## PR P2-7: DTO v1 and generated schemas (step 7)
- [x] `loom_serialization::dto::v1` DTOs, deterministic conversion both ways (`to_json`, `from_json`, `DTO_*` codes); golden synthetic PoE1 and PoE2 documents; all 24 public fixtures round-trip
- [x] `tools/schema-gen`, `schemas/build.v1.schema.json`, TypeScript (`pnpm generate:schema`); schema validation test (`boon`); CI `schema freshness`
- [x] `dto-hashes` and CI `dto cross-platform` (in-session: Windows and WSL Linux identical); `cargo deny` ok
- [x] Gate 3b: resolved synthetic builds round-trip through JSON
- [x] `code-architecture-reviewer` (resumed after an API rate limit): 1 blocker (the banner's backticks under `sh`) and 2 should-fix (read the header before the strict parse, with `from_value`; `DTO_TOO_DEEP`) applied, plus nits; context file
- [x] Open PR (#54, base `feat/p2-6b-resolve-build`); write the PR number into the docs right after

## PR P2-8: `build-round-trip` and CI gate job (step 8)
- [x] Harness: import, resolve, JSON, export, re-import, re-resolve; checks (a) to (d); mutation set; v1 documents as input (PoE2); PoE1-identifier scan in the gate job
- [x] CI job `round-trip` in `ci-ok` (in-session: the job's commands on Windows and WSL Linux, JSON hashes equal); corpus run 21/21, scrubbed summary `docs/phase-2/build-round-trip-poe1-26530219.json`
- [x] Maintainer decisions on P2-7: `DTO_*` in plan decision 15 with the layers; gate 3b test moved to the harness, `loom-parser` dev-dependency and AGENTS.md exception removed
- [x] `code-architecture-reviewer`: no blockers; 4 should-fix and most nits applied (context file)
- [x] Open PR (#55, base `feat/p2-7-dto-v1`); write the PR number into the docs right after

## PR P2-9: migrations and persistence (step 9)
- [x] Migration framework and report; `MIGRATE_*` (`loom_serialization::migrate`, `dto::read`; test-only v0 to v1)
- [x] Atomic save, load with migration; `PERSIST_*` (`loom_persistence::Store`; injected `FileOps` failures; WSL Linux in-session)
- [x] `code-architecture-reviewer`: no blockers; atomic `Never`, full read before replacing, docs and tests applied (context file)
- [x] Open PR (#56, base `feat/p2-8-build-round-trip`); write the PR number into the docs right after

## PR P2-10: share codes (step 10, gated)
- [x] Import and export through the XML adapter (`legacy::share_code`, `miniz_oxide`, our base64; `IMPORT_SHARE_CODE_*`)
- [x] Our code of every public fixture; two PoB-made codes from the oracle runtime with their XML (`tools/oracle/adapter/share_code.lua`, `tools/oracle/fixtures/share-codes/`)
- [x] `code-architecture-reviewer`: no blockers; code length cap, one-pass decoding, fixture-count-free tests, nits applied (context file)
- [x] Open PR (#57, base `feat/p2-9-migrations-persistence`); write the PR number into the docs right after

## PR P2-11: report and gate (step 11)
- [x] `docs/phase-2/phase-2-report.md`, gate verdict: met; maintainer-run evidence re-run at `3982838`, equal to the committed summaries (none replaced)
- [x] Handoff, CLAUDE.md, ACTION-PLAN; move the folder to `dev/completed/`
- [x] `code-architecture-reviewer`: no blockers; report precision, `ModDomain` in the gate 3c scan, CI run recorded (context file)
- [ ] Open PR (base `feat/p2-10-share-codes`); write the PR number into the docs right after

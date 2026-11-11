Last Updated: 2026-11-10 (PR P2-5a: PoB XML structure)

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
- [ ] Open PR (base `feat/p2-4-bundle-validator`); write the PR number into the docs right after

## PR P2-5b: PoB XML items, item sets, slots (step 5b)
- [ ] `<Items>`: items (every `<Item>` text child kept; the last non-blank one read with `pob_item::to_domain`; `ModRange` and other children opaque in place), item sets, slots; export generated from the item model (decision 6)
- [ ] Item round-trip and mutation tests (several item texts, comments); corpus in-session

## PR P2-6: `resolve_build` (step 6)
- [ ] Bundle `ItemData`; map outcomes onto domain lines; completeness
- [ ] Rebuilding `Mods` (split `(0, 0)` stats into `unset`; class, mod-type, and implicit IDs onto rows) and `BaseRow` from a bundle equals `mod_filter::load` and `Bases::load` on the extract, for both games (P2-3b review)
- [ ] Outcome tests on synthetic PoE1 and PoE2 bundles (gate 3b); maintainer-run corpus fields equal Spike B

## PR P2-7: DTO v1 and generated schemas (step 7)
- [ ] DTOs, deterministic conversion
- [ ] `tools/schema-gen`, JSON Schema, TypeScript; CI freshness check
- [ ] Cross-OS hash comparison job in CI; `cargo deny`

## PR P2-8: `build-round-trip` and CI gate job (step 8)
- [ ] Harness: import, resolve, JSON, export, re-import, re-resolve; checks (a) to (d); mutation set; PoE1-identifier scan in the gate job
- [ ] CI job; maintainer-run corpus summary

## PR P2-9: migrations and persistence (step 9)
- [ ] Migration framework and report; `MIGRATE_*`
- [ ] Atomic save, load with migration; `PERSIST_*`

## PR P2-10: share codes (step 10, gated)
- [ ] Import and export through the XML adapter

## PR P2-11: report and gate (step 11)
- [ ] `docs/phase-2/phase-2-report.md`, scrubbed summaries, gate verdict
- [ ] Handoff, CLAUDE.md, ACTION-PLAN; move the folder to `dev/completed/`

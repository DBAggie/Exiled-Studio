Last Updated: 2026-11-10 (PR P2-2b: scope and bases)

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
- [ ] Move `mod_filter` and `item_tags`; generation types, influences, and essence columns through `item_rules`
- [ ] Maintainer-run: Spike B field list equal

## PR P2-2d: compositions, grafts, properties, tolerances (step 2d)
- [ ] Move grafts, properties, case-fold, split-section; `COMPOSITIONS` as per-game data
- [ ] Maintainer-run: Spike B field list equal

## PR P2-3: canonical data model, bundle v1, converter (step 3; may split 3a/3b)
- [ ] Canonical types (our schema) for what import and validation read; `item_rules`; `poe1` section; game-specific table list and its test
- [ ] `bundle.json` (reserved `signatures`), `BundleSource` (`list`, `read`), loader errors; `sha2` with `cargo deny`
- [ ] `loom-extractor bundle`; synthetic PoE1 and PoE2 bundles
- [ ] Maintainer-run: two-run determinism on PoE1 and PoE2; scrubbed summaries

## PR P2-4: bundle validator (step 4)
- [ ] Structural, schema, referential, domain, smoke checks with `BUNDLE_*` codes
- [ ] `loom-parity-cli bundle-check`
- [ ] Mutation set in CI
- [ ] Maintainer-run: real PoE1 and PoE2 bundles, 0 errors

## PR P2-5: PoB XML import and semantic export (step 5; may split 5a/5b)
- [ ] Typed sections, opaque rest, item lines as source text
- [ ] Export from the model; own XML writer
- [ ] `xml_equal` (parsed trees) and the closed normalization list
- [ ] Round-trip and mutation tests on public fixtures (in-between content, several item texts, comments); malformed-input tests

## PR P2-6: `resolve_build` (step 6)
- [ ] Bundle `ItemData`; map outcomes onto domain lines; completeness
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

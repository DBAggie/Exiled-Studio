Last Updated: 2026-11-09 (PR A4 opened)

# Spike A: tasks

## PR A0: plan
- [x] Create plan, context, tasks under `dev/active/spike-a-bundles2-extraction/`
- [x] Review with the `plan-reviewer` agent and apply findings
- [x] Maintainer answers the open questions (2026-11-08, recorded in the context file)
- [x] Record the gate decisions in the ACTION-PLAN Spike A gate text
- [x] Update handoff Current state / Next action
- [x] Open PR (#3, merged 2026-11-08)

## PR A1a: cargo-deny
- [x] `deny.toml` with an MIT-compatible license allow-list, bans, and sources; passes locally with cargo-deny 0.20.2
- [x] CI `cargo-deny` job (cargo-deny-action v2.1.1, SHA-pinned), required via `ci-ok`
- [x] Green CI run (42532581603); merged as PR #4 on 2026-11-08, which accepts the MPL-2.0 exceptions

## Follow-up (dependency policy)
- [ ] Non-blocking (scheduled) `cargo deny check advisories` job

## PR A1: crates and decompressor decision (step 1)
- [x] Survey existing permissive Rust crates for the Bundles2 index and datc64 (none usable without ooz-derived code; report section 1)
- [x] Survey decompressors; record license, codecs, maintenance, `unsafe` needs (all derive from GPL ooz)
- [x] Maintainer decision: external tool boundary (`unoodle` from `oozextract` 0.5.7, installed separately, not linked)
- [x] Identify `_.index.high.bin`, `_.index.low.bin`, `.index.dbg` (not needed; report section 5)
- [x] Write format notes (bundle container and index observed; path hash and datc64 marked "to confirm" in A2/A3)
- [x] Start `docs/spikes/spike-a-extraction.md` with the decision
- [x] `Decompressor` trait, `ExternalTool`, `Identity` in `tools/extractor/src/decompress.rs`; `#[ignore]` test `tests/install_decompress.rs` passed on PoE1 and PoE2 (maintainer workstation)
- [x] Enable `clippy::indexing_slicing` for `tools/extractor` (crate attribute in `lib.rs` and `main.rs`)
- [x] PR open, CI green, maintainer merge (PR #6, merged 2026-11-08 as `e4b8402`; main CI run 42550157380 green)

## PR A2: bundle and index (step 2)
- [x] `Decompressor` trait and the chosen implementation (done in A1)
- [x] Bundle header/chunk parser (`src/bundle.rs`; one decoder call per bundle) with synthetic-byte tests
- [x] Index parser and path reconstruction (`src/index.rs`); path hash confirmed: PoE1 all 1,245,702 records match, PoE2 4,324,291 of 4,324,296 (5 + 5 unmatched listed, report section 4); 12 path-hash vectors from the PoE1 index
- [x] Corrupt-input tests (exhaustive truncation and single-byte corruption instead of random property tests)
- [x] `list-files` subcommand; maintainer run on PoE1 and PoE2 (counts and hashes in report sections 4 and 8)
- [x] `#[ignore]` install test `tests/install_index.rs` passed on both installs
- [x] PR open (#7), CI green, maintainer merge (merged 2026-11-09 as `89ed48c`; main CI run 42552158054 green)

## PR A3: dat reader (step 3)
- [x] Vendor the schema file with source commit (upstream has no tags), hash, and third-party license notice (maintainer OK 2026-11-08)
- [x] Schema loader filtered by game (`src/schema.rs`)
- [x] datc64 reader for every column type; row-size mismatch reporting (`src/dat.rs`)
- [x] Unit tests per column type; exhaustive truncation and single-byte corruption tests
- [x] `read_files` (one decode per bundle) and `#[ignore]` install test `tests/install_dat.rs`, passed on PoE1 and PoE2 (report section 9)
- [x] PR open (#8), CI green, maintainer merge (merged 2026-11-09 as `2c2abbe`)

## PR A4 (a/b): extract and determinism (step 4)
- [x] `extract` for Stats, Mods, BaseItemTypes, PassiveSkills, ActiveSkills, GrantedEffects plus the transitive closure of referenced tables (PoE1 72, PoE2 85); not split
- [x] `BTreeMap`-only output assembly; every-order determinism unit test; golden manifest test
- [x] Canonical JSON output and manifest (build ID, index and bundle hashes, schema commit and hash, extractor commit, dirty flag, decoder program and version, per-table status, key range checks)
- [x] Determinism check script `tools/extractor/scripts/check-determinism.sh`; byte-identical on PoE1 and PoE2 from clean tree `d047e4a`
- [x] Commit manifests (`docs/spikes/spike-a/`; no machine paths, so committed unchanged)
- [x] `code-architecture-reviewer` findings applied or recorded (context file, "PR A4")
- [ ] PR open (#9), CI green, maintainer merge

## PR A5: stat descriptions (step 5; can run in parallel with A3/A4)
- [ ] UTF-16LE `.csd` reader, includes, description blocks, conditions, format strings, handlers
- [ ] Unparsed-construct counting in the manifest
- [ ] Unit tests per construct; maintainer-run counts

## PR A6: cross-check and gate (step 6)
- [ ] Search upstream for data exports newer than 3.29.3 / 0.5.5; use the newest and record the commits
- [ ] Clone `PathOfBuilding` and `PathOfBuilding-PoE2` at those commits into `local/`
- [ ] `cross-check` with like-for-like keys (bases by English name; PoB-generated mods as their own category)
- [ ] Categorized gap report from the tool; spike report with the gate verdict
- [ ] Note that gap entries become divergence-register entries once Spike C defines the format
- [ ] Update ACTION-PLAN gate status, CLAUDE.md Status, handoff

## Follow-up (outside Spike A)
- [ ] Phase 2: record extractor provenance at build time (or in a signed build step) instead of querying git at run time
- [ ] Fix stale `docs/plan/IMPLEMENTATION-PLAN.md` Phase 2 item 4 ("converter from legacy exporter output") to match ADR-005

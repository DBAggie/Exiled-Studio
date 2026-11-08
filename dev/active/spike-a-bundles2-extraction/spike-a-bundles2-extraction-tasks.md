Last Updated: 2026-11-08

# Spike A: tasks

## PR A0: plan
- [x] Create plan, context, tasks under `dev/active/spike-a-bundles2-extraction/`
- [x] Review with the `plan-reviewer` agent and apply findings
- [x] Maintainer answers the open questions (2026-11-08, recorded in the context file)
- [x] Record the gate decisions in the ACTION-PLAN Spike A gate text
- [x] Update handoff Current state / Next action
- [ ] Open PR

## PR A1a: cargo-deny
- [ ] `deny.toml` with an MIT-compatible license allow-list and bans; CI job; green run

## PR A1: crates and decompressor decision (step 1)
- [ ] Survey existing permissive Rust crates for the Bundles2 index and datc64
- [ ] Survey decompressors; record license, codecs, maintenance, `unsafe` needs
- [ ] Identify `_.index.high.bin`, `_.index.low.bin`, `.index.dbg`
- [ ] Write format notes (bundle, index, path hash, datc64) with sources
- [ ] Start `docs/spikes/spike-a-extraction.md` with the decision
- [ ] Add the dependency with its test vectors (if any) and an `#[ignore]` test using `LOOM_POE1_INSTALL`/`LOOM_POE2_INSTALL` (or record the fallback trigger)
- [ ] Enable `clippy::indexing_slicing` for `tools/extractor`

## PR A2: bundle and index (step 2)
- [ ] `Decompressor` trait and the chosen implementation
- [ ] Bundle header/chunk parser with synthetic-byte tests
- [ ] Index parser and path reconstruction; path-hash test vectors
- [ ] Corrupt-input property tests
- [ ] `list-files` subcommand; maintainer run on PoE1 and PoE2, counts and hashes in the PR

## PR A3: dat reader (step 3)
- [ ] Vendor the schema file with tag, hash, and third-party license notice (after maintainer OK)
- [ ] Schema loader filtered by game
- [ ] datc64 reader for every column type; row-size mismatch reporting
- [ ] Unit tests per column type; property tests on corrupt and truncated input

## PR A4 (a/b): extract and determinism (step 4)
- [ ] `extract` for Stats, Mods, BaseItemTypes, PassiveSkills, ActiveSkills, GrantedEffects (plus referenced tables)
- [ ] `BTreeMap`-only output assembly; shuffled-input determinism unit test
- [ ] Canonical JSON output and manifest (build ID, hashes, schema tag, extractor commit, dirty flag)
- [ ] Determinism check script; run twice on PoE1 and PoE2 from a clean tree
- [ ] Commit scrubbed manifests

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
- [ ] Fix stale `docs/plan/IMPLEMENTATION-PLAN.md` Phase 2 item 4 ("converter from legacy exporter output") to match ADR-005

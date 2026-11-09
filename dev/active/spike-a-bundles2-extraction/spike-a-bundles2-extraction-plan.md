Last Updated: 2026-11-08

# Spike A: Bundles2 game-data extraction (plan)

## Summary
Spike A in `docs/plan/ACTION-PLAN.md` (Phase 1, first spike). Prove that the project's own Rust extractor (`tools/extractor`, ADR-005) can read the PoE1 and PoE2 Bundles2 installs, decompress bundles with a license-compatible decompressor, parse the needed `.dat` tables using the community dat schema, parse stat-description files, and do this reproducibly with recorded inputs. Spike B (text to stat-ID resolution) consumes this output.

This spike produces **spike-level output** (deterministic JSON per table plus a manifest). The canonical game-data schema and signed bundle format are Phase 2 work (IMPLEMENTATION-PLAN Phase 2, item 3) and are not designed here.

## Current state (2026-11-08)
- `tools/extractor` is an empty binary that prints "not implemented" and exits with failure. It depends on `loom-data` (also empty).
- Workspace lints: `unsafe_code = "deny"`, no `unwrap`/`expect` outside tests, `#[expect]` only.
- Game installs present: PoE1 `D:\SteamLibrary\steamapps\common\Path of Exile` (Steam build 26474023) and PoE2 `...\Path of Exile 2` (Steam build 26476662). Both have `Bundles2/_.index.bin` (plus `_.index.high.bin`, `_.index.low.bin`, `.index.dbg`, which the spike must identify).
- Legacy PoB v2.67.2 (oracle pin, tagged 2026-08-08) carries the "3.29.2 Export" data (commit `b383679a4`). Legacy HEAD has a "3.29.3 Export" (`1b4078dc4`). PoB uses its own `src/Export/spec.lua`, not the community schema, and filters what it exports. PoB v2.67.2 is PoE1 only.

## Target state
- `loom-extractor` subcommands, run by the maintainer against a local install:
  - `list-files <install>`: index contents (count, sample paths).
  - `extract <install> --game poe1|poe2 --out <dir>`: writes per-table JSON (Stats, Mods, BaseItemTypes, PassiveSkills, ActiveSkills, GrantedEffects, plus the tables they reference by key that are needed to resolve rows), parsed stat-description files, and `manifest.json`.
  - `cross-check <out-dir> <pob-checkout-dir>`: compares keys against PoB's exported Lua data and writes a categorized gap report. The PoB directory is a fresh clone (or `git archive`) of the public repository at the agreed commit, placed in `local/`. The legacy repo at `D:\POE Dev\PathOfBuilding` is never checked out, worktree'd, or otherwise written to (CLAUDE.md hard rule).
- `manifest.json` records: game (poe1/poe2), Steam build ID, game version string if readable, SHA-256 of `_.index.bin` and every bundle read, dat-schema release tag and SHA-256, extractor version, git commit and whether the tree was dirty, and SHA-256 plus row count of every output file. No timestamps or machine paths in hashed output.
- A spike report `docs/spikes/spike-a-extraction.md` with the decompressor decision, format findings, and gate results with every gap categorized.
- Outputs go in the git-ignored `local/extract/<game>/<steam-build-id>/` (AGENTS.md: extractor output goes in `local/` or `out/`). Only the report and a scrubbed copy of the manifest are committed.

## Design constraints and decisions
- **Location:** all format code (bundle, index, dat, stat descriptions) lives in `tools/extractor` as a library (`src/lib.rs`) plus a thin binary. It is not a core crate: ADR-005 makes extraction maintainer-only, so the app never needs it, and keeping it out of `loom-data` keeps core small. Parsers take `&[u8]`/`&str`; filesystem access stays in the binary and an `InstallReader` at the edge, so parsers are unit-testable without a game install.
- **Decompression** sits behind a `Decompressor` trait so container/framing tests can use an identity decompressor and synthetic bytes. **Decided in A1 (maintainer, 2026-11-08):** every open Oodle decoder derives from GPL ooz, so none is linked; `ExternalTool` runs a separately installed `unoodle` (`oozextract` 0.5.7) as a child process. Details in `docs/spikes/spike-a-extraction.md`, section 2.
- **Schema:** vendor the pinned community dat-schema file (`poe-tool-dev/dat-schema` release `schema.min.json`, if its license permits) into `tools/extractor/schema/` with its release tag, a third-party license notice, and a hash over the exact committed bytes. No network at extraction time (ADR-011 spirit; reproducibility per ADR-005).
- **Unknown content is never silently dropped (ADR-015):** columns the schema does not describe, tables whose row size disagrees with the schema, and stat-description constructs the parser does not understand are reported in the manifest with counts.
- **No GGG assets (ADR-000):** only tables and text data are read. No art, audio, or raw game files are committed; tests use synthetic bytes.
- **Errors:** typed errors with stable codes (ADR-014), via `thiserror` in the library.
- **Untrusted input:** game files and the schema are untrusted bytes. `tools/extractor` enables `clippy::indexing_slicing` (and `arithmetic_side_effects` where offsets are computed). Parsers use `get` and checked arithmetic and return typed errors. Property tests feed truncated and corrupt input and assert an error, never a panic.
- **Determinism by construction:** no `HashMap`/`HashSet` iteration in anything that reaches output (use `BTreeMap` or sorted `Vec`); no parallel output assembly during the spike.
- **License policy:** dependencies are checked by `cargo-deny` (licenses and bans) with an MIT-compatible allow-list, added before any third-party format dependency (PR A1a).
- **Divergence register:** data gaps against PoB are differences from PoB (ADR-006). The categorized gap list in the spike report becomes divergence-register entries once Spike C defines the register format.

## Steps

### 1. Existing crates, decompressor, and format research (S-M)
- First look for existing permissively licensed Rust crates that already read the Bundles2 index and/or datc64 (crates.io, poe-tool-dev). If a good one exists, A1-A3 collapse into one evaluate-and-adopt PR, and only stat descriptions and the cross-check remain as new code.
- Identify candidate Oodle-compatible decompressors (for example a pure-Rust port such as `oozextract`, or the C++ `ooz` used by community tools via FFI). Check each one's license, maintenance, supported codecs (Kraken/Mermaid/Selkie/Leviathan), and whether it needs `unsafe` in our crates.
- Document the Bundles2 container layout, the index layout, the path-hash algorithm, and the datc64 layout as found, with sources.
- Identify what `_.index.high.bin`, `_.index.low.bin`, and `.index.dbg` are and whether they are needed.
- **Testing real decompression:** no permissive Oodle encoder exists to make synthetic data, and GGG bytes cannot be committed. Use the crate's own permissively licensed test vectors if it has them, plus an `#[ignore]` integration test that reads the install path from `LOOM_POE1_INSTALL` / `LOOM_POE2_INSTALL`; the maintainer runs it and pastes the result in the PR.
- **Accept:** decision section in the spike report: chosen crate(s), license (MIT-compatible: MIT/Apache-2.0/BSD/Zlib/Unlicense), and reasons; or a recorded "no compatible option" that triggers the fallback. `cargo deny check licenses bans sources` passes in CI with the dependency added.
- **Depends on:** A1a (`cargo-deny`).

### 2. Bundle container and index reading (M)
- Parse bundle headers and chunk tables; decompress via the trait; parse `_.index.bin` (bundle list, file records, path reps) and rebuild the path list; hash paths to look up files.
- `list-files` subcommand.
- **Accept:** unit tests on synthetic container bytes (identity decompressor), path-hash test vectors, and corrupt-input property tests; maintainer run lists PoE1 and PoE2 files, with counts and index hashes in the PR.
- **Depends on:** 1.

### 3. Dat reader and schema loader (M)
- Load the vendored schema, select tables valid for the target game, and read `.datc64` rows (fixed-width section, `0xBBBBBBBBBBBBBBBB` boundary, variable data section) into typed values: scalars, strings (UTF-16LE), arrays, foreign rows/keys, enums.
- Detect row-size mismatches against the schema and report them as findings.
- **Accept:** unit tests over synthetic dat bytes for every column type, including empty arrays, null keys, and a row-size mismatch; schema-loading test on a minimal schema fixture; property test that truncated or corrupt input returns an error, not a panic.
- **Depends on:** 2 for real files; the reader itself only needs bytes.

### 4. Table extraction, manifest, determinism (M)
- `extract` for the Spike A tables on PoE1 and PoE2; canonical JSON (sorted keys, row order by row index, locale-independent number formatting); manifest with all hashes.
- **Accept:** two consecutive runs produce byte-identical outputs and manifest (script `tools/extractor/scripts/check-determinism.sh` or a `--verify` flag); a CI unit test builds output from shuffled input order and asserts identical bytes; maintainer run results from a clean tree in the PR.
- **Split if large:** A4a = manifest and determinism on Stats only; A4b = the remaining tables.
- **Depends on:** 3.

### 5. Stat-description parsing (M-L)
- Read `Metadata/StatDescriptions/*.csd` (UTF-16LE), parse `include`, `description` blocks, stat ID lists, language blocks, value conditions (ranges, `#`, `!`), format strings, and handlers (`negate`, `divide_by_one_hundred`, ...). English only for now; other languages are counted, not parsed.
- Unrecognized constructs are counted and listed in the manifest.
- **Accept:** unit tests over synthetic snippets for each construct; maintainer run reports description counts per file and lists every unparsed construct with an explanation.
- **Depends on:** 2 only, so A5 can run in parallel with A3/A4 if they stall (Spike B needs it).

### 6. PoB cross-check and gate (M)
- Before starting: search upstream for data exports newer than 3.29.3 (PoE1) and 0.5.5 (PoE2) and use the newest; record the commits in the report.
- `cross-check` reads a PoB clone in `local/` at that commit (PoE1: `PathOfBuilding`; PoE2: `PathOfBuilding-PoE2`), never the legacy repo itself. Compare like with like (PoE2 file layout may differ; confirm per source):
  - Mods: PoB `src/Data/Mod*.lua` keys vs Mods `Id`. First check whether PoB generates mods that are not in Mods.dat; those get their own category.
  - Bases: PoB `src/Data/Bases/*.lua` is keyed by **English display name**, not `Id`. Compare against the BaseItemTypes English `Name`; record duplicate names separately.
  - Skills: PoB `src/Data/Skills/*.lua` keys vs GrantedEffects `Id`.
  - Stats: stat IDs referenced in PoB `src/Data/StatDescriptions/*` vs Stats `Id`.
  - Passives: PoB tree node keys vs the matching PassiveSkills column (confirm the key in this step).
- Every gap gets exactly one category from a **fixed list**, and each category's evidence comes from a tool, not prose:
  - `not-in-index`: the file or row is absent from the installed index (evidence: `list-files`/extract output).
  - `pob-filtered`: excluded by PoB's export filter (evidence: the cross-check reproduces the filter rule and names it).
  - `pob-generated`: PoB-generated entries with no dat row.
  - `patch-delta`: differs between PoB's export patch and the installed patch (evidence: diff against the next PoB export, if one exists).
  - `schema-mismatch`: row-size or column mismatch reported in the manifest.
  - `uncategorized`: anything else.
- **Accept:** cross-check tool output with counts and categorized gaps; spike report with the gate verdict.
- **Depends on:** 4, 5.

## Gate (objective)
1. **Determinism:** two runs of `extract` on the same install produce identical SHA-256 for every output file and the manifest, checked by script, for PoE1 and PoE2.
2. **Coverage vs PoB (PoE1 and PoE2):** compare against the newest PoB data export available when A6 starts (PoE1: `PathOfBuilding`, currently 3.29.3 `1b4078dc4`; PoE2: `PathOfBuilding-PoE2`, currently 0.5.5 `49e93925d`), each cloned into `local/`. Per table family, the cross-check reports `uncategorized = 0` for PoB keys missing from our output, and `patch-delta` at most 2% of PoB keys. Extracted keys not in PoB are categorized the same way. (Maintainer decision 2026-11-08, recorded in the ACTION-PLAN.)
3. **No silent drops:** the manifest lists every schema mismatch and unparsed stat-description construct, and each has a category.
4. **Recorded inputs:** the manifest contains the game build ID, index hash, schema tag and hash, extractor commit, and `dirty: false`.
5. **License:** `cargo deny check licenses bans sources` passes in CI, covering transitive dependencies.

## Fallback
If extraction is blocked (no compatible decompressor, unusable format, or gate 2 fails with uncategorized gaps): use RePOE as the input and amend ADR-005 (maintainer decision). Partial fallback is possible: if only stat descriptions fail, RePOE's stat-translation data can serve Spike B while extraction continues.

## Risks and mitigations
| Risk | Mitigation |
|---|---|
| Only GPL or unlicensed decompressors exist | Happened (A1). Resolved by the external tool boundary: no ooz-derived code in the tree or `Cargo.lock`; `cargo-deny` enforces the rest. |
| Installed game patch differs from any PoB export, so counts cannot match "for the same patch" | Newest available export, re-searched at A6; `patch-delta` category with tool evidence and a 2% ceiling. |
| `patch-delta` used to explain away real gaps | Fixed category list, tool evidence per category, 2% ceiling, `uncategorized = 0`. |
| Community schema lags the installed patch (row-size mismatch) | Detect and report mismatches; pin the newest schema release; record the schema tag. |
| Steam updates the game mid-spike, changing inputs | Record build ID and index hash on every run; the determinism check compares runs on the same hashes only. |
| `unsafe_code = "deny"` conflicts with an FFI decompressor | Prefer pure Rust; FFI only through a separate third-party crate (the lint applies to our crates only), recorded in the report. |
| Parser panics on malformed input | `indexing_slicing` lint, checked arithmetic, property tests on corrupt input. |
| Format knowledge comes from community tools with unclear licenses | Write code from format documentation and observation; cite sources; do not copy code from unlicensed projects. |
| Cross-check accidentally modifies the legacy repo | The tool reads a separate clone in `local/`; no git commands run in `D:\POE Dev\PathOfBuilding`. |
| PoB's Lua data shape differs per file, making cross-check parsing brittle | Extract only keys with narrow patterns; test each pattern on small Lua snippets. |

## Planned PR breakdown
Each PR is one session and updates the handoff and this task folder (`/dev-docs-update`).

| PR | Content | Evidence |
|---|---|---|
| A0 | This plan, handoff update. | Tests: None (docs). Parity: None. Coverage: none. |
| A1a | `cargo-deny` config (license allow-list, bans, sources) and CI job. | CI run with `cargo deny` green. |
| A1 | Step 1: spike report started with the existing-crate survey, decompressor decision, and format notes; add the chosen dependency with its test vectors (if any) and an `#[ignore]` install-backed test. | Vector test; maintainer-run ignored test output; `cargo deny` green. |
| A2 | Step 2: bundle and index reader, `list-files`. | Unit and property tests (synthetic bytes, hash vectors); maintainer-run counts and index hashes for PoE1/PoE2. |
| A3 | Step 3: dat reader and vendored schema. | Unit tests per column type, mismatch detection, property tests. |
| A4 (a/b) | Step 4: `extract`, manifest, determinism script (split if large). | Unit tests including the shuffled-input test; determinism results for PoE1/PoE2 on a clean tree; scrubbed manifest committed. |
| A5 | Step 5: stat-description parser. | Unit tests per construct; maintainer-run counts and unparsed-construct list. |
| A6 | Step 6: `cross-check`, gate verdict, ACTION-PLAN gate status and handoff update. | Cross-check tool output (counts, categorized gaps); gate pass/fail. |

A1 may merge into A2 if the decision is quick and the dependency is small. A5 may run in parallel with A3/A4.

Reviewed by `plan-reviewer` on 2026-11-08; all blockers and should-fix items are applied above (summary in the context file).

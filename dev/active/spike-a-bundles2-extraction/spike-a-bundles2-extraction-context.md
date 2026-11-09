Last Updated: 2026-11-08 (PR A1 opened)

# Spike A: context

## Key files
- `tools/extractor/` (`loom-extractor`): where all Spike A code goes. `src/lib.rs` (library, `#![warn(clippy::indexing_slicing)]`), `src/decompress.rs` (`Decompressor` trait, `ExternalTool`, `Identity`), `tests/install_decompress.rs` (`#[ignore]` install-backed test), `src/main.rs` (still a stub).
- `tools/extractor/src/bin/fake-unoodle.rs` + `tests/external_tool.rs`: a test double for `unoodle` (behaviour picked by the input prefix `ok:`/`lie:`/`silent:`/`fail:`) so CI tests `ExternalTool` without a decoder. Added after the `code-architecture-reviewer` review (should-fix: the tool path was only covered by the ignored test).
- `docs/spikes/spike-a-extraction.md`: the spike report (survey, decompressor decision, bundle and index layout, evidence). Add a section per PR.
- `crates/loom-data/`: core crate, empty. Not changed by Spike A (the canonical data schema is Phase 2).
- `Cargo.toml`: workspace lints (`unsafe_code = "deny"`, `unwrap_used`/`expect_used` warn, errors in CI).
- `.gitignore`: `/local/` and `/out/` are ignored; extractor output and PoB clones go in `local/`.
- `docs/spikes/`: holds `spike-c-legacy-oracle.md`; Spike A's report will be `docs/spikes/spike-a-extraction.md`.
- `docs/handoff/SUCCESSOR-HANDOFF.md`: Current state / Next action, updated every PR.

## ADRs that apply
- **ADR-005:** own Rust extractor; Bundles2 plus the community dat schema; PoB data is cross-check only; RePOE is the fallback; record game version, schema revision, and input hashes; maintainer-run; users get signed bundles only.
- **ADR-000:** derive data only; no GGG art or audio in the repo.
- **ADR-001/006:** the game is the truth; PoB v2.67.2 is the oracle for calcs; differences from PoB go in the divergence register.
- **ADR-011:** network only in integration adapters, so the schema is vendored, not fetched.
- **ADR-012:** small PRs with evidence.
- **ADR-014:** typed errors with stable codes; no `unwrap`/`expect` outside tests.
- **ADR-015:** never drop unresolved content silently; report it.
- **ADR-016:** core crates are wasm32-only; the extractor is a tool, not core, so it may use the filesystem.

## External references (verify license before using code or data)
- Community dat schema: `poe-tool-dev/dat-schema` on GitHub (GraphQL schema files; releases publish `schema.min.json` with `validFor` per game).
- Oodle-compatible decompression: `ooz` (C++, powzix) and Rust ports (for example the `oozextract` crate). Licenses not yet verified.
- Format write-ups and tools to consult (not copy): `poe-tool-dev/poe-dat-viewer`, LibGGPK3, PyPoE forks, RePOE (fallback input).
- Legacy PoB (read-only, never run git commands that write): `D:\POE Dev\PathOfBuilding`. Exporter: `src/Export/` (`spec.lua`, `Scripts/*.lua`, `ggpk/` uses `bun_extract_file.exe`). Exported data: `src/Data/`. For the cross-check, clone the public repo into `local/` instead.

## Findings so far (2026-11-08)
- PoE1 install: Steam app 238960, build 26474023. PoE2 install: Steam app 2694490, build 26476662. Read from `D:\SteamLibrary\steamapps\appmanifest_*.acf`.
- Both `Bundles2/` folders contain `_.index.bin`, `_.index.high.bin`, `_.index.low.bin`, `.index.dbg`. The purpose of the high/low/dbg files is unknown (step 1).
- PoB v2.67.2 (tag commit `b32759ab0`, 2026-08-08) data is the "3.29.2 Export" (`b383679a4`, 2026-08-06). A later "3.29.3 Export" (`1b4078dc4`, 2026-08-23) is on legacy HEAD. Latest tree version in PoB: `3_29`.
- PoB `src/Data/Bases/*.lua` is keyed by English display name, not the BaseItemTypes `Id`.
- `IMPLEMENTATION-PLAN.md` Phase 2 item 4 ("converter from legacy exporter output to canonical bundles") predates revised ADR-005 and is stale. The ACTION-PLAN is authoritative. Tracked as a follow-up task in the tasks file.

## Decisions (with reasons)
- **Format code lives in `tools/extractor` (lib + bin), not in a core crate.** Extraction is maintainer-only (ADR-005); keeping it out of core avoids wasm and dependency constraints on spike code.
- **Parsers work on bytes; filesystem only at the edge.** CI can test without a game install (AGENTS.md test rules).
- **Decompressor behind a trait.** Framing tests use an identity decompressor; no GGG bytes in fixtures.
- **Vendor the pinned schema file** (license permitting) with tag, license notice, and hash. Reproducible, no network.
- **Outputs in `local/extract/<game>/<steam-build-id>/`** (AGENTS.md); commit only the report and a scrubbed manifest.
- **Spike output is plain deterministic JSON**, not the Phase 2 canonical bundle format.
- **`cargo-deny` before any third-party format dependency** (PR A1a), so the license gate is checked by CI, transitive dependencies included.
- **Cross-check reads a clone in `local/`,** never the legacy repo, which must not be written to (CLAUDE.md).

## Maintainer decisions (2026-11-08)
1. **PoE1 cross-check baseline:** use the newest PoB data export available, and search upstream for newer ones when A6 starts. As of 2026-11-08 the newest is the "3.29.3 Export" `1b4078dc4` (2026-08-23) in `PathOfBuildingCommunity/PathOfBuilding`. The installed build (Steam 26474023, November 2026) is probably newer, so remaining differences are categorized `patch-delta`. This is data cross-checking only; the calc oracle pin stays v2.67.2.
2. **Vendor the community schema file** into `tools/extractor/schema/` (license permitting): agreed.
3. **Dependency license policy:** only MIT-compatible permissive licenses (MIT, Apache-2.0, BSD, Zlib, Unlicense), enforced by `cargo-deny`. If only GPL decompressors exist, take the RePOE fallback: agreed. (List extended with other permissive licenses in A1a; see "PR A1a" below.)
4. **PoE2 cross-check:** check against the public `PathOfBuildingCommunity/PathOfBuilding-PoE2` repo, cloned into `local/`, at its newest data export (search again at A6). As of 2026-11-08: "Export 0.5.5 data" `49e93925d` (2026-09-10). GitHub reports its license as `NOASSERTION`; we only read its data for comparison and copy nothing into this repo.
5. **Patch-delta ceiling N = 2%** of PoB keys per table: agreed.

These decisions are written into the ACTION-PLAN Spike A gate (2026-11-08).

## PR A1a: cargo-deny (PR #4, merged 2026-11-08 as `be077e9`)
- Files: `deny.toml` (new), `.github/workflows/ci.yml` (new `cargo-deny` job, id `deny`, added to `ci-ok` needs), `AGENTS.md` (command + "Dependencies" section).
- CI uses `EmbarkStudios/cargo-deny-action` v2.1.1 pinned to `3c6349835b2b7b196a839186cb8b78e02f7b5f25`, which bundles cargo-deny 0.20.2; the maintainer workstation has the same version (`cargo install --locked cargo-deny@0.20.2`). Command: `cargo deny --all-features --locked check licenses bans sources`.
- Allow-list: 0BSD, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, CC0-1.0, ISC, MIT, MIT-0, Unicode-3.0, Unlicense, Zlib. This extends decision 3's list (MIT/Apache/BSD/Zlib/Unlicense) with other permissive licenses already in the tree (0BSD, CC0, MIT-0, Unicode-3.0, LLVM-exception) plus ISC.
- **MPL-2.0 exceptions (accepted by the maintainer by merging PR #4, 2026-11-08):** the tree had 5 MPL-2.0-only crates, all reached only through Tauri via `loom-tauri` (checked with `cargo tree -i`): `cssparser`, `cssparser-macros`, `dtoa-short`, `selectors` (tauri-utils -> dom_query) and `option-ext` (tauri -> dirs -> dirs-sys). They are allowed by crate name, not globally, so a new MPL dependency still fails. Removing them would mean dropping Tauri (ADR-002). Without the exceptions the check fails with exactly these 5 crates; that run is the evidence the policy rejects non-allowed licenses.
- `multiple-versions = "warn"`: Tauri brings ~20 duplicate crates (windows-sys, toml, syn, ...); denying would block on upstream.
- Advisories (`cargo deny check advisories`) are deliberately not in the required job: the RustSec DB changes without a code change and would turn unrelated PRs red. Follow-up task: a scheduled or non-blocking advisories job.
- Quirk: in the agent's PowerShell, cargo is not on PATH; prefix commands with `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`. Git Bash has no cargo either.

## PR A1: decompressor decision (2026-11-08)
- **Survey result:** every open Oodle decoder derives from powzix/ooz (GPL-3.0-or-later). `oozextract` 0.5.7 says MIT on crates.io, but its repo has no LICENSE file and calls itself a "rust port of ooz". `poe_data_tools` (MIT/Apache) depends on it. Neither PoE install ships an Oodle DLL. Full table in the report, section 1.
- **Maintainer decision (2026-11-08): external tool boundary.** No ooz-derived code is linked, vendored, or in `Cargo.lock`. `ExternalTool` runs a separately installed `unoodle` (`cargo install --locked oozextract@0.5.7 --features cli`) as `unoodle <in> --length <n> --header 0 --verbose --output <out>`, through scratch files, and checks its `Extracted <n> bytes` line. The tool always writes a buffer of the requested size, so that line is the only real length check. RePOE was rejected as a fallback here because it also decodes with ooz (via PyPoE).
- **No ADR change:** ADR-005's "own Rust extractor" still holds; only Oodle decoding is delegated. If the maintainer wants this in an ADR, it would be an amendment to ADR-005 (not proposed).
- **Performance:** one spawn per 256 KiB chunk takes about 27 ms. Decoding all chunks concatenated in one call works (33.4 MB in about 0.1 s, matching the per-chunk output). A2 should call the decoder once per bundle.
- **Index files:** `_.index.high.bin` and `_.index.low.bin` are identical bundles that decode to 8 zero bytes; `.index.dbg` is a stale non-bundle hash table. None are needed.
- **Error codes:** the extractor starts a convention: each `thiserror` enum has `code() -> &'static str` returning `EXTRACT_<AREA>_<WHAT>` (ADR-014). No repo-wide convention exists yet.
- **Clippy quirk:** `allow-expect-in-tests` does not cover helper functions in integration-test files, so `tests/install_decompress.rs` uses a file-level `#![expect(clippy::expect_used, reason = ...)]`.
- **Install test** (PowerShell; agent shells need the cargo PATH prefix): set `LOOM_POE1_INSTALL` and `LOOM_POE2_INSTALL` to the install folders, then `cargo test -p loom-extractor --test install_decompress -- --ignored --nocapture`. `LOOM_OODLE_TOOL` overrides the decoder path. Passed 2026-11-08 on both installs.
- `unoodle` 0.5.7 is installed on the maintainer workstation in `%USERPROFILE%/.cargo/bin`.

## Open questions for the maintainer
None.

## Plan review (2026-11-08)
`plan-reviewer` verdict: proceed with changes. No ADR conflicts. Applied:
- Blockers: the cross-check uses a clone in `local/`, never the legacy repo; the PoE2 cross-check source became open question 4; fixed gap categories with tool evidence, `uncategorized = 0`, and an N% patch-delta ceiling.
- Should-fix: like-for-like keys (bases by English name, PoB-generated mods); output in `local/`; `cargo-deny` PR before dependencies; ignored install-backed decompression test; shuffled-input determinism test, `BTreeMap` only, dirty-tree flag; `indexing_slicing` lint and corrupt-input property tests; gap list feeds the divergence register.
- Nits: schema license notice and hash over committed bytes; A4 may split; A5 can run in parallel; the stale IMPLEMENTATION-PLAN item is a tracked task; survey existing Rust crates first (may collapse A1-A3).

Last Updated: 2026-11-08 (PR A2 opened)

# Spike A: context

## Key files
- `tools/extractor/` (`loom-extractor`): where all Spike A code goes. `src/lib.rs` (library, `#![warn(clippy::indexing_slicing)]`), `src/decompress.rs` (`Decompressor` trait, `ExternalTool`, `Identity`; A1), `src/reader.rs` (private bounds-checked cursor, `Short` error), `src/hash.rs` (MurmurHash64A `path_hash`), `src/bundle.rs` (`Bundle::parse`/`decode`, `decode_bundle`, test helper `build_identity_bundle`), `src/index.rs` (`Index::parse`/`paths`/`list`, `Listing`), `src/install.rs` (filesystem edge: `read_index`, SHA-256, `hex`), `src/main.rs` (`list-files` subcommand). Install-backed `#[ignore]` tests: `tests/install_decompress.rs` (A1), `tests/install_index.rs` (A2).
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
- Both `Bundles2/` folders contain `_.index.bin`, `_.index.high.bin`, `_.index.low.bin`, `.index.dbg`. Only `_.index.bin` is needed (A1, report section 5).
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
- **ADR-005 Amendment 1** records this decision (added to PR #6 at the maintainer's request; approved by merging PR #6). It allows swapping the external tool without another amendment, but linking or distributing a decoder needs a new one.
- **Performance:** one spawn per 256 KiB chunk takes about 27 ms. Decoding all chunks concatenated in one call works (33.4 MB in about 0.1 s, matching the per-chunk output). A2 should call the decoder once per bundle.
- **Index files:** `_.index.high.bin` and `_.index.low.bin` are identical bundles that decode to 8 zero bytes; `.index.dbg` is a stale non-bundle hash table. None are needed.
- **Error codes:** the extractor starts a convention: each `thiserror` enum has `code() -> &'static str` returning `EXTRACT_<AREA>_<WHAT>` (ADR-014). No repo-wide convention exists yet.
- **Clippy quirk:** `allow-expect-in-tests` does not cover helper functions in integration-test files, so `tests/install_decompress.rs` uses a file-level `#![expect(clippy::expect_used, reason = ...)]`.
- **Install test** (PowerShell; agent shells need the cargo PATH prefix): set `LOOM_POE1_INSTALL` and `LOOM_POE2_INSTALL` to the install folders, then `cargo test -p loom-extractor --test install_decompress -- --ignored --nocapture`. `LOOM_OODLE_TOOL` overrides the decoder path. Passed 2026-11-08 on both installs.
- `unoodle` 0.5.7 is installed on the maintainer workstation in `%USERPROFILE%/.cargo/bin`.

## PR A2: bundle and index reader (PR #7, opened 2026-11-08, branch `spike-a/a2-index-reader`)
- PR A1 (#6) merged 2026-11-08 as `e4b8402`; main CI run 42550157380 green. That merge approved ADR-005 Amendment 1.
- **Confirmed on both installs:** path encoding (phase-switched fragments) and path hash (MurmurHash64A, seed `0x1337b33f`, lowercase path; directory records hash the directory without trailing slash, root = `""`). Full description and counts in the report, section 4. First explored with a throwaway Python script in the session scratchpad (not committed), then reproduced by the Rust code.
- **PoE2 index inconsistency:** 5 paths without a record and 5 records without a path (art/effects only, listed in the report). So `Index::list` returns `Listing` with `files`, `paths_without_record`, `records_without_path`, `path_reps_without_directory`; nothing is dropped (ADR-015). A path matching several records is listed once per record.
- **Strictness choices (from observation, PoE1):** fragment number `n - 1` greater than the prefix count, a block not ending at a fragment boundary, a file record outside its bundle, and a path block outside the path data are errors, not findings. If a future patch trips one, inspect it before relaxing.
- **Bundle decoding:** one `Decompressor` call per bundle on the whole payload (A1 timing). A zero-length bundle returns empty without calling the tool.
- **`sha2` 0.10** added as a workspace dependency for the index SHA-256 (A4's manifest needs it too). It was already in `Cargo.lock` through Tauri, so `cargo deny` sees no new crate.
- **Corrupt-input tests are exhaustive, not `proptest`:** every truncation and every single-byte change (6 values per byte) of a synthetic bundle and index. Deterministic and adds no dependency; the plan's "property tests" wording is met by this.
- **Hash test vectors** are path strings plus hashes from the PoE1 index (no game bytes). There is no independent permissive MurmurHash64A implementation to cross-check against; the full-index match is the real evidence.
- **Clippy quirks:** `clippy::as_conversions` is not enabled, so `#[expect]` on it fails as unfulfilled; plain `as` with a comment is used for u32 -> usize (`reader::widen`) and usize -> u64. Test modules that index buffers carry `#[expect(clippy::indexing_slicing, reason = ...)]`. In `tests/install_index.rs` all `expect` calls are inside the `#[test]` fn, which `clippy.toml` already allows, so no file-level expect there (unlike `install_decompress.rs`).
- **`code-architecture-reviewer` (2026-11-08): no blockers.** Applied: prefixes and paths capped at `MAX_PATH_LEN` = 4096 bytes (`EXTRACT_INDEX_PATH_TOO_LONG`), because chained prefixes could otherwise use memory quadratic in a hostile block; `FIXED_HEAD_LEN` used instead of literal 48; redundant `debug_assert` dropped; `Listing`/`ListedFile` docs corrected (a path rebuilt twice is listed twice; paths are as stored); `PathBundle` documented as reusing the inner error code; compile-time `usize::BITS >= 32` assert for `widen`; tests for re-entering the prefix phase and for `offset + size` above `u32::MAX`; `list-files` uses `create_dir` so a pre-existing scratch dir fails loudly. Not applied: moving instead of cloning paths in `list()` and dropping the `previous_dir` shortcut (micro-optimisations; the shortcut avoids ~26M set inserts on PoE2); a decoded-size cap in `Bundle::decode` (the external tool bounds allocation); pinning exact unmatched counts in the install test (`MAX_UNMATCHED = 100`, so a patch does not break it).
- **For A3/A4:** PoE1 tables are in `data/`, PoE2 tables in `data/balance/`. Localized PoE1 copies (`data/<language>/stats.datc64`) point at the same bundle range. Many files share `Tiny*` bundles: decode each bundle once and slice files out.
- **Verify:** `cargo test -p loom-extractor`; with `LOOM_POE1_INSTALL`/`LOOM_POE2_INSTALL` set, `cargo test --release -p loom-extractor --test install_index -- --ignored --nocapture`; `cargo run --release -p loom-extractor -- list-files "<game folder>"`. Agent shells need the cargo PATH prefix (Git Bash: `export PATH="/c/Users/justi/.cargo/bin:$PATH"`).

## Open questions for the maintainer
None.

## Plan review (2026-11-08)
`plan-reviewer` verdict: proceed with changes. No ADR conflicts. Applied:
- Blockers: the cross-check uses a clone in `local/`, never the legacy repo; the PoE2 cross-check source became open question 4; fixed gap categories with tool evidence, `uncategorized = 0`, and an N% patch-delta ceiling.
- Should-fix: like-for-like keys (bases by English name, PoB-generated mods); output in `local/`; `cargo-deny` PR before dependencies; ignored install-backed decompression test; shuffled-input determinism test, `BTreeMap` only, dirty-tree flag; `indexing_slicing` lint and corrupt-input property tests; gap list feeds the divergence register.
- Nits: schema license notice and hash over committed bytes; A4 may split; A5 can run in parallel; the stale IMPLEMENTATION-PLAN item is a tracked task; survey existing Rust crates first (may collapse A1-A3).

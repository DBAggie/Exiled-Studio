Last Updated: 2026-11-09 (PR A6a)

# Spike A: context

## Key files
- `tools/extractor/` (`loom-extractor`): where all Spike A code goes. `src/lib.rs` (library, `#![warn(clippy::indexing_slicing)]`), `src/decompress.rs` (`Decompressor` trait, `ExternalTool`, `Identity`; A1), `src/reader.rs` (private bounds-checked cursor, `Short` error), `src/hash.rs` (MurmurHash64A `path_hash`), `src/bundle.rs` (`Bundle::parse`/`decode`, `decode_bundle`, test helper `build_identity_bundle`), `src/index.rs` (`Index::parse`/`paths`/`list`, `Listing`), `src/install.rs` (filesystem edge: `read_index`, `read_files` (decodes each bundle once), SHA-256, `hex`), `src/schema.rs` (`Schema` loader, `VENDORED` embedded schema and its hash; A3), `src/dat.rs` (`DatFile::parse`, `read_table`, `Value`; A3), `src/extract.rs` (`Plan`, `Extraction`, canonical table JSON, `Manifest`; A4), `src/statdesc.rs` (stat-description decoder, parser, findings, `Collection`; A5), `src/crosscheck.rs` + `src/crosscheck/{pob,ours}.rs` (`cross-check`: PoB readers, Lua pattern matcher, our-output readers, rules, report; A6a), `src/main.rs` (`list-files`, `extract`, `cross-check` subcommands). `scripts/check-determinism.sh` (two-run byte comparison; A4). Install-backed `#[ignore]` tests: `tests/install_decompress.rs` (A1), `tests/install_index.rs` (A2), `tests/install_dat.rs` (A3).
- `tools/extractor/schema/`: vendored `schema.min.json` (community dat schema, MIT), its license, and `README.md` with provenance, hash, and update steps.
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

## PR A2: bundle and index reader (PR #7, merged 2026-11-09 as `89ed48c`; main CI run 42552158054 green)
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

## PR A3: dat reader and schema loader (PR #8, merged 2026-11-09 as `2c2abbe`; main CI run 42555106211)
- **Maintainer approved vendoring** `poe-tool-dev/dat-schema` on 2026-11-08 (combined `schema.min.json`, 2.5 MB, over per-game files). Upstream has no version tags (`latest` is rebuilt on every commit), so the copy is pinned by source commit `717d788d6ec0` (verified via upstream Publish run 42056729237) and SHA-256. `.gitattributes` marks the file `binary` so the committed bytes stay exact. Embedded with `include_bytes!`; a unit test checks the hash, so a swapped file fails CI until README and `VENDORED_SHA256` are updated together.
- **Schema loader is strict:** `deny_unknown_fields` on every level, unknown type names and `validFor` bits are errors, version must be 7. A schema update that changes the format fails loudly. `description`, `until`, `file`, `files` are parsed but unused.
- **datc64 layout confirmed** (report section 6). Facts that are easy to get wrong: offsets count from the start of the `0xBB` boundary; strings end at two zero UTF-16 units; `foreignrow` is 16 bytes and its second word is 0 / null (now enforced); `enumrow` is 4 bytes; null keys are the full 64-bit `0xFE` pattern.
- **Row-size mismatch handling:** not an error. `read_table` returns `Table::mismatch` and decodes the leading columns that fit (`columns_read`). On real data every mismatch is a wider row (unknown trailing columns), so all schema columns were read. A4's manifest must carry the mismatch list.
- **Boundary detection:** schema width first; fallback scan starts at one byte per row. Zero-width rows with a non-zero count are impossible by construction (a corrupt count can otherwise cost memory with no input behind it).
- **Bug found while testing:** `DatFile::row` must check the index against the row count; with no rows the width is 0 and every empty slice is "in range" (an install-test loop never ended). Fixed with a regression assertion.
- **`poe-dat-viewer` has no license:** used only to confirm layout facts, no code taken. `dat-schema` (MIT) is the documented source.
- **`serde` 1.0 (derive) and `serde_json` 1.0** added as workspace dependencies; both already locked through Tauri, so `Cargo.lock` only gains two edges and `cargo deny` passes.
- **Install results** (report section 9): PoE1 942 of 957 tables decode with the schema width, 2 mismatches, 0 failures, 13 without schema; PoE2 1,008 of 1,025, 6 mismatches, 0 failures, 11 without schema. All six Spike A tables are clean on both games.
- **For A4:** use `install::read_files` with the `ListedFile` records of the tables to extract (stems match schema names ignoring case: `Schema::table(game, stem)`). `Value` is not serialisable yet; A4 owns the canonical JSON form (floats need locale-independent, deterministic formatting; `f32` NaN/inf handling to decide). Foreign keys are raw row indices; resolving them to target rows (and checking they are in range) is A4/A6 work.
- **`code-architecture-reviewer` (2026-11-08): no blockers.** Applied: decode output cap (`DECODE_BUDGET_PER_BYTE` = 8, `EXTRACT_DAT_TOO_LARGE`) against rows sharing one large array or string; `read_files` visitor returns `Result<(), E>` with `E: From<InstallError>` so A4 can stop on the first error; bundle names from the index are checked before they become paths (`EXTRACT_INSTALL_BAD_BUNDLE_NAME`); string offsets inside the boundary or odd are rejected (none in real data); `ValueError` has a readable `Display`; install test trimmed to outcomes and now asserts no table fails; tests for a float interval and a bad element inside an array. Its doc-staleness finding was a timing artefact (docs were updated in parallel). Deferred to A4: non-finite `f32` handling and shortest-round-trip `f32` formatting (do not widen to `f64` and print); emit `u64` keys and offsets as exact integers, not via `f64`; `localized` columns are not used, the English text comes from reading `data/` (PoE1) or `data/balance/` (PoE2), make that explicit in the manifest; one bad value fails the whole table even when it has a mismatch (consider keeping the mismatch finding); trailing bytes of wider rows are not exposed (use `DatFile::row` if needed); `Interval` boxes its pair (cosmetic).
- **Verify:** `cargo test -p loom-extractor`; with `LOOM_POE1_INSTALL`/`LOOM_POE2_INSTALL` set, `cargo test --release -p loom-extractor --test install_dat -- --ignored --nocapture`.
- **Agent shell quirk:** Bash heredocs passed through the tool can mangle backslashes (a `'\\0'` became a NUL byte) and some non-ASCII. Write edit scripts with the Write tool and run them with `python -I <script> <file>`. PowerShell 5.1 `Set-Content -Encoding utf8` adds a BOM; strip it.

## PR A4: extract, manifest, determinism (PR #9, merged 2026-11-09 as `d480d1f`)
- **Not split into A4a/A4b:** the dat reader is generic, so all six tables plus the referenced ones cost the same code as Stats alone.
- **Which tables:** the six roots plus the transitive closure over every column that references a table (enum references skipped): PoE1 72 tables, PoE2 85. Only direct references would be 27/38; the closure was chosen so every key in the output can be range-checked. Tables come from `data/` (PoE1) and `data/balance/` (PoE2), the English text; the manifest says so (`table_dir`, `text_language`).
- **JSON layout:** `{"columns": [...], "rows": [[...], ...], "table": ...}`, one row per line. Rows are arrays, not objects, because many schema columns have no name. Details in the report, section 10.
- **Floats:** `serde_json::to_string(&f32)` uses `ryu` for `f32` (shortest round-trip, no widening). Non-finite floats become the strings `"NaN"`/`"Infinity"`/`"-Infinity"` and are counted per table (0 in both installs). serde_json writes exponents as `e+38`.
- **Watch out: `serde_json/preserve_order` is enabled in the workspace** (through `schemars`), so `serde_json::Map` keeps insertion order in some builds and not in others. The manifest uses derived structs whose fields are declared alphabetically, plus `BTreeMap`. A golden test pins the exact bytes. Keep fields alphabetical when adding any.
- **Steam build ID:** from `<library>/steamapps/appmanifest_*.acf`, matched by `installdir`. **Game version string:** not recorded (no reliable source).
- **Provenance:** the git commit and dirty flag are read at run time from `CARGO_MANIFEST_DIR` (the reviewer's should-fix 1). Accepted and documented rather than moved to `build.rs`: a `build.rs` would not rerun when untracked files appear, and `cargo run` and the script rebuild right before running anyway. Revisit for Phase 2 signed bundles.
- **Exit code / `problems()`:** non-zero if any table failed, a root is not `ok` (a row-size mismatch on a root counts), or any key column has dangling keys. Everything else is recorded only in the manifest.
- **Output folder:** checked before decoding. Written to `<out>.partial`, then renamed. CLI errors have codes `EXTRACT_CLI_*`.
- **`read_files` now returns `BundleHashes`** (SHA-256 per bundle file). Each table also records `source_sha256` (the table bytes after the bundle is decoded).
- **`default-run = "loom-extractor"`** added: since A3's `fake-unoodle` bin, `cargo run -p loom-extractor` needed `--bin`.
- **Results** (report section 11): clean tree `d047e4a`; both games byte-identical over two runs (PoE1 73 files, PoE2 86 files); 0 dangling keys of 905,891 (PoE1) and 388,069 (PoE2) checked; PoE2 `ItemVisualIdentity` is the only non-ok table (row-size mismatch, not a root). Manifests committed in `docs/spikes/spike-a/`.
- **`code-architecture-reviewer` (2026-11-09): no blockers in code.**
  - Applied: process blocker (docs); should-fix 2 (root mismatch and dangling keys fail the run, with tests); 3 (early output check, `.partial` + rename); 4 (CLI error codes); 5 (`key_target` cases, huge keys, NaN in intervals, `planned_files` test, all 6 add orders over 3 tables).
  - Nits applied: decoder `--version` with null stdin; `source_sha256`; empty-parent guard.
  - Not applied: build-time provenance (see above); a defensive table-name check (the schema is vendored and hashed); mismatch details on stdout (they are in the manifest).
- **For A5/A6:**
  - A6 reads `tables/*.json` (rows as arrays; find a column's position by `columns[].name`).
  - Foreign keys are row indices into the target's `rows`.
  - The manifest's per-table `status` gives the `schema-mismatch` category evidence.
- **Verify:**
  - `cargo test -p loom-extractor`.
  - `tools/extractor/scripts/check-determinism.sh "<install>" poe1|poe2` (Git Bash with the cargo PATH prefix; output in `local/determinism/`).

## PR A5: stat-description parser (PR #10, opened 2026-11-09, branch `spike-a/a5-stat-descriptions`)
- PR A4 (#9) merged 2026-11-09 as `d480d1f`.
- **The plan's `.csd` assumption held only for PoE2.** PoE1 uses `metadata/statdescriptions/*.txt` (45 files), PoE2 `data/statdescriptions/**/*.csd` (589, most per skill). Same UTF-16LE grammar; full description in the report, section 12.
- **How the grammar was found:** a throwaway example binary (not committed) dumped every file to `local/a5/`, converted to UTF-8 with `iconv`, then surveyed with `grep`/`awk` (line kinds, condition shapes, handler argument shapes, placeholders, markup). PoB's `src/Export/statdesc.lua` (legacy repo, read only) agrees on the grammar. The dumps are still in `local/a5/` on the maintainer workstation (git-ignored); recreate them the same way if needed.
- **Decisions:**
  - Parsing is token based per non-blank line, not indentation based: many files put block lines at column 0.
  - English only is parsed; other languages are counted per file (`languages`). Non-English lines contain constructs the English never uses (`1|1|#`, `{0%}`, `{0 }`); parsing them would add findings that do not matter for Spike B.
  - A grammar problem is a finding (fixed `FindingKind` list, line, text, skipped count), never an error. Only an odd byte count or a bad surrogate fails a file (`EXTRACT_STATDESC_ODD_LENGTH`, `EXTRACT_STATDESC_BAD_UTF16`), and that, or finding no stat-description file at all, fails the run.
  - Handler names are an open set: any `<name> <stat>` parses, and the manifest counts each name. Their meaning (PoB implements about 50 of them in `describeStats`) is Spike B work. Only `reminderstring <id>` and the bare `canonical_line`/`markup` are special. Variant flags are a closed list (`gem_quality`, `table_only`); an unknown one is a finding.
  - `{{`/`}}` are literal braces; markup (`<tag>{...}`, `[Ref|Text]`, `<<Icon>>`) is kept as written in `text`. Spike B must strip it before matching.
  - `*_filters` files (PoE1 `skillpopup_stat_filters.txt`, a `group {}` format) are hashed and listed as `skipped`.
  - Stat-description output goes through the same `Extraction` as tables (`Extraction::add` routes by path), so it shares the determinism and the `.partial` output folder. Manifest format bumped to 2.
- **Steam updated PoE1 overnight** (build 26474023 to 26530219). Every bundle and table A4 reads has the same hash; the A4 table results stand. The A4 PoE1 manifest was replaced by the new one; the old one is in git history.
- **Results** (report section 13): zero unparsed English lines on both games; counts match independent `grep` counts; determinism holds (PoE1 117 files, PoE2 675) from clean tree `9394d75` (after the review fixes).
- **`code-architecture-reviewer` (2026-11-09): no blockers.** (The first attempt stopped on an account usage limit; it was resumed after the reset.)
  - Applied: should-fix 1 (stat-description paths with `..`, empty parts, `\`, `:`, NUL fail the file with `EXTRACT_STATDESC_BAD_PATH`, since the path becomes an output path); 2 (a bad English count keeps the description without variants; a consumed `lang` line counts as skipped); 3 (`EXTRACT_UNPLANNED_FILE`/`EXTRACT_DUPLICATE_FILE` instead of the table codes; `TableError` documented as covering files too, not renamed); 4 (new findings `include-not-parsed` and `include-cycle`); nit: counts, conditions, handler stats reject a leading `+`.
  - Not applied (nits): `min > max` ranges kept as written (documented; none occur); bare-`
` line endings (none occur); line numbers for `includes`/`no_description` in the JSON; renaming `Handler.stat` (1-based) vs `Placeholder.index` (0-based), documented instead; a warning for a non-empty file with no constructs (an even-length non-text file would decode and give `unknown-line` findings, so it is not silent).
- **For Spike B:** read `stat_descriptions/**/*.json`. A description is matched by its stat IDs; a variant applies when every condition holds for its stat's value; `handlers` transform values before formatting (`stat` is 1-based); `placeholders[].index` is 0-based (`null` means 0). `includes` chain files (a skill file includes `skill_stat_descriptions`, which includes `stat_descriptions`); which file wins for a stat is not decided yet. `no_description` lists stats that are deliberately not shown. Findings do not fail the run: read `findings_by_kind` in the manifest.
- **Verify:** `cargo test -p loom-extractor`; `tools/extractor/scripts/check-determinism.sh "<install>" poe1|poe2` and read `local/determinism/<game>/run1.log`.

## PR A6a: cross-check (PR #11, opened 2026-11-09, branch `spike-a/a6a-cross-check`)
- PR A5 (#10) merged 2026-11-09 as `ce5221b`.
- **Maintainer decision 2026-11-09: split A6.** The gate needs every gap categorized in both directions; our PassiveSkills rows missing from PoB's tree (PoE1 2,310, PoE2 4,817) have no honest evidence in the extracted tables. A6a = the tool and four families; A6b = read the passive skill graph and close the gate.
- **Upstream search 2026-11-09:** no export newer than PoE1 `1b4078dc4` / PoE2 `49e93925d` (dev, beta, master branches; open PRs with "export" in the title). Clones: `local/pob/PathOfBuilding-1b4078dc4`, `local/pob/PathOfBuilding-PoE2-49e93925d` (`git init`, `git fetch --depth 1 <url> <full sha>`, checkout `FETCH_HEAD`).
- **How the rules were found:** throwaway Python prototypes in the session scratchpad (not committed) measured what stayed uncategorized, then each remainder was traced to PoB's export code. Hard-to-rediscover facts:
  - Skill and base spec files are only those the scripts list (`skills.lua` `for _, name in pairs({...})`; `bases.lua` `local itemTypes = {...}`). PoE2 `SkillGemsExport.txt` and `soulcore.txt` are not processed. `#skillEnd` is not a `#skill` line.
  - `#baseMatch [column] value` is a Lua pattern match (`GetRowList(key, value, true)`), PoB column `BaseType` = our `InheritsFrom`. `#base` Ids match exactly (one PoE1 spec line has `Metadata/items/...` and PoB skips it as invalid).
  - Neither game's `ModItemExclusive` uses the unique specs; they filter intrinsic mods by domain, family, and Id patterns. The Id patterns are read from `mods.lua`.
  - Most mods PoB drops have no line from `describeMod`: stats 0 to 0 are skipped, buff-template stats are left out, only the files `mods.lua` loads count (PoE1 tincture + graft, both including stat_descriptions; PoE2 stat_descriptions.csd), and a variant must match the minimum values. `[DNT]` becomes `DNT` through `escapeGGGString` before PoE1's DNT check.
  - PoB writes the stat list of handed descriptions as `"0"`; 192 of PoB's 764 PoE2 stat-description files are stale (no installed counterpart).
  - PoE2 BuffTemplates names its stat array `Stats`, PoE1 `StatsKey`; Mods `BuffTemplate`/`BuffTemplate1` and `BuffTemplate2` (array in the community schema). Columns are picked by reference.
- **`patch-delta` evidence without a newer export:** reproduced PoB behaviour. PoE1 `V2MinPowerChargesCorruptedNew` (checked by hand against PoB `dev`) and `EDSHarvestMinerHammerSmoke` (listed in PoB's spec but missing from its output; `skills.lua` writes the key before any check, so the output predates the spec line).
- **`code-architecture-reviewer` (2026-11-09):** one blocker, applied: `schema-mismatch` gaps now fail the family gate. Should-fix applied: mod `Id`-pattern and writer-condition rules are scoped to (domain, generation) pairs where no exported mod breaks them, and specialised writers' mods (Id matches a pattern PoB still exports for the pair, e.g. `^Synthesis`) are left out of the generic rules (the first scoping attempt moved 121 Watcher's Eye `AuraBonus` mods into patch-delta because `ModSynthesis` exports three `AuraBonus` implicits); skill rule renamed `listed-not-in-output` with a correct anchor; `stat-only-in-game-descriptions` removed (would hide an extraction gap); stale PoB statdesc files checked against `list-files --all`; heuristic limits, append order, expected disabled rules documented. Not applied: parsing `writeMods` condFuncs (follow-up task), renumbering appended descriptions, null `Id`/`Name` rows (none occur).
- **Evidence-run pitfall:** copy the summaries into `docs/` only after both games ran; copying the first one dirtied the tree and the second report recorded `tool_dirty: true`.
- **Results** (report section 15): PoE1 and PoE2 mods, bases, skills, stats: 0 uncategorized, 0 unparsed, patch-delta 1+1 (PoE1) / 0 (PoE2). Passives: 2,310 / 4,817 uncategorized. Tool commit `33730ed` (after the review), two runs byte-identical, on the A5 extract output (Steam builds unchanged: PoE1 26530219, PoE2 26476662).
- **Divergence register:** the categorized gaps are differences from PoB; not in a register yet (Spike C defines the format).
- **Verify:** `cargo test -p loom-extractor`; `cargo run --release -p loom-extractor -- cross-check local/extract/<game>/<build> local/pob/<checkout> --game <game> --out local/cross-check/<game>/<build>` (the A5 extracts are in `local/determinism/<game>/run1`; reports from this PR in `local/cross-check/`). Exit 1 while passives are uncategorized.
- **Agent shell quirk:** multi-line Python heredocs in Git Bash broke on quotes in one call; writing the script with the Write tool and running `python -I <script>` works (same as the A3 note).

## Open questions for the maintainer
None.

## Plan review (2026-11-08)
`plan-reviewer` verdict: proceed with changes. No ADR conflicts. Applied:
- Blockers: the cross-check uses a clone in `local/`, never the legacy repo; the PoE2 cross-check source became open question 4; fixed gap categories with tool evidence, `uncategorized = 0`, and an N% patch-delta ceiling.
- Should-fix: like-for-like keys (bases by English name, PoB-generated mods); output in `local/`; `cargo-deny` PR before dependencies; ignored install-backed decompression test; shuffled-input determinism test, `BTreeMap` only, dirty-tree flag; `indexing_slicing` lint and corrupt-input property tests; gap list feeds the divergence register.
- Nits: schema license notice and hash over committed bytes; A4 may split; A5 can run in parallel; the stale IMPLEMENTATION-PLAN item is a tracked task; survey existing Rust crates first (may collapse A1-A3).

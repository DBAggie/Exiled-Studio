# Spike A: Bundles2 game-data extraction (report)

Plan: `dev/active/spike-a-bundles2-extraction/`. Gate: `docs/plan/ACTION-PLAN.md`, Spike A. This report grows one section per PR (A1 to A6); the gate verdict is written at A6.

Status (2026-11-09, PR A4): decompressor decided (A1); bundle container and index read by `loom-extractor` on both installs, path encoding and path hash confirmed (A2); datc64 layout confirmed and every table with a schema entry decoded on both installs with the vendored community schema (A3); `extract` writes the Spike A tables and every table they reference as deterministic JSON with a manifest, checked byte-identical over two runs on both installs (A4, sections 10 and 11). Gate items 1 and 4 met; 2 and 3 pending (A5, A6).

## 1. Existing crates (survey, 2026-11-08)
Searched crates.io (`oodle`, `kraken`, `ooz`, `ggpk`, `datc64`, `poe bundle`, `path of exile`) and the GitHub repos behind each hit. Licenses are from crates.io metadata and the source files.

| Crate / project | What it does | License | Verdict |
|---|---|---|---|
| `oozextract` 0.5.7 (lvlvllvlvllvlvl/oozextract) | Pure-Rust Kraken/Mermaid/Selkie/Leviathan/LZNA/Bitknit decoder; `unoodle` CLI behind feature `cli` | crates.io says MIT; the repo has **no LICENSE file** and describes itself as a "rust port of ooz" | Probably a derivative of GPL code; **not linked**. Used only as a separately installed tool (section 2). |
| powzix/ooz (C++) | Original open-source Oodle decoder | GPL-3.0-or-later (header in `kraken.cpp`) | Not usable as a dependency. |
| zao/ooz (C++) | ooz fork with Bundles2 tools (`bun_extract_file`, used by PoB's exporter) | No license file; GPL headers from ooz | Not usable. |
| `oozle` 0.1.5 | C++ ooz via `cxx` | GPL-3.0-or-later | Not usable. |
| `ooz-sys` 0.2.0 | ooz bindings | GPL-3.0-or-later, yanked | Not usable. |
| `poe_bundle` 0.2.1 | Bundles2 reader over C++ ooz | LGPL-3.0-only | Not usable. |
| `poe_data_tools` 4.0.1 | Bundles, dat, stat descriptions, CLI | MIT OR Apache-2.0 | Depends on `oozextract` (same provenance problem) and brings `reqwest`, `tokio`, `rayon`, `arrow`. Not adopted; a possible reference for format behaviour (permissive, so reading it is fine). |
| `primase-oozextract` 0.0.0 | Pure-Rust decoder | MIT | Repo not found (404); a fork of `oozextract` by its description. Not usable. |
| `oodle`, `oodle-safe`, `oodle-sys` | Wrappers around the proprietary Oodle SDK | n/a | Need RAD/Epic's binary; neither PoE install ships an Oodle DLL (Oodle is linked into the game executable). Not usable. |
| `ggpk` 1.2.2 | Legacy single-file GGPK (pre-Bundles2) | MIT | Wrong format. |

Finding: **every open Oodle decoder found derives from powzix/ooz (GPL-3.0-or-later).** No clean-room permissive implementation exists. A web search for one (2026-11-08) found only ooz derivatives. No existing crate collapses A1 to A3 without bringing in ooz-derived code, so the container, index, and dat readers are written here (plan step 1, "collapse" case does not apply).

## 2. Decompressor decision (maintainer, 2026-11-08)
Options put to the maintainer: (a) run a separately installed decoder across a process boundary, (b) accept `oozextract` as MIT at face value, (c) take the RePOE fallback and amend ADR-005. **Chosen: (a), external tool boundary.**

- No ooz-derived code is linked into or vendored in this repository, and none is in `Cargo.lock`. `cargo deny` sees nothing new except `thiserror`.
- `tools/extractor/src/decompress.rs` defines the `Decompressor` trait. `ExternalTool` runs the maintainer-installed `unoodle` (from `cargo install --locked oozextract@0.5.7 --features cli`) once per call, through two scratch files, and checks the decoded length the tool reports. `Identity` serves framing tests on synthetic bytes.
- The decoder is a maintainer workstation tool, like a compiler. It is not distributed by this project and never reaches end users (ADR-005: users get signed bundles only).
- The manifest (A4) must record the decoder program and version, because outputs depend on it.
- Why not (c): RePOE itself decodes with PyPoE, which uses ooz, so it does not remove GPL code from the pipeline; it only hides it. Its data files are GGG-owned (RePOE `LICENSE.md`), the same as ours.
- Recorded as ADR-005 Amendment 1 (`docs/adr/005-build-and-data-compatibility.md`), approved by merging PR #6. The extractor is still the project's own Rust code; only Oodle decoding is delegated.

Performance (PoE1 `_.index.bin`, maintainer workstation): one call per 256 KiB chunk takes about 27 ms per spawn; one call on all chunks concatenated decodes 33.4 MB in about 0.1 s and matches the per-chunk output. A2 uses one call per bundle.

## 3. Bundle container (observed, PoE1 and PoE2)
All integers little-endian. Verified on `_.index.bin` (both games), `_.index.high.bin`, `_.index.low.bin`, and PoE1 `Tiny.bundle.bin`: `file size = 12 + head_size + payload_size`, chunk sizes sum to `payload_size`, and `chunk_count = ceil(decompressed_size / granularity)`.

| Offset | Type | Field |
|---|---|---|
| 0 | u32 | decompressed size |
| 4 | u32 | payload size (all chunks) |
| 8 | u32 | head size (bytes from offset 12 to the first chunk; `48 + 4 * chunk_count`) |
| 12 | u32 | encoder id (observed 8, 12, 13; matches the community-documented Oodle compressor enum, Kraken = 8, Leviathan = 13; not needed for decoding) |
| 16 | u32 | unknown (observed 1) |
| 20 | u64 | decompressed size (repeat) |
| 28 | u64 | payload size (repeat) |
| 36 | u32 | chunk count |
| 40 | u32 | granularity (observed 262144 = 256 KiB) |
| 44 | u32 x 4 | unknown (observed 0) |
| 60 | u32 x chunk count | compressed size of each chunk |
| 12 + head size | bytes | chunks, back to back |

Each chunk decodes to `granularity` bytes (the last to the remainder). Chunks start with an Oodle block header (`0x8C`; observed `8C 0C` in large bundles, `CC 06` in the 8-byte high/low bundles).

`Bundle::parse` (`tools/extractor/src/bundle.rs`, A2) rejects a file unless every redundant field agrees: the `u64` copies equal the `u32` fields, `head size = 48 + 4 * chunk count`, `chunk count = ceil(decompressed size / granularity)` with a non-zero granularity, the chunk sizes sum to the payload size, and the payload ends exactly at the end of the file. The fields at 16 and 44 are not checked.

## 4. Index (`_.index.bin`; layout walked on PoE1 in A1, read in full on PoE1 and PoE2 in A2)
`_.index.bin` is a bundle. Its decoded bytes (PoE1: 33,449,776; PoE2: 151,240,657) are laid out as:

1. `u32` bundle count (PoE1 46,884; PoE2 64,994), then per bundle: `u32` name length, name bytes (e.g. `Metadata/Shrines`, no extension), `u32` decompressed size.
2. `u32` file count (PoE1 1,245,702), then per file 20 bytes: `u64` path hash, `u32` bundle index, `u32` offset in the decoded bundle, `u32` size.
3. `u32` path-rep count (PoE1 86,209), then per rep 20 bytes: `u64` directory hash, `u32` offset, `u32` size, `u32` recursive size.
4. The rest is an embedded bundle (same container) holding the path data; its decoded size equals the first rep's recursive size (PoE1: 26,706,151), and it ends exactly at the end of the index.

PoE2 (A2): 4,324,296 file records, 97,028 path reps, path data 127,635,349 bytes.

**Path data (confirmed in A2 on both installs).** Each path rep's `(offset, size)` is a block of the decoded path data; the blocks tile the data exactly, and the first rep (the root directory) is 8 bytes. A block is a sequence of `u32 n`: `n = 0` toggles between a prefix phase and a path phase (a block starts in the path phase; entering the prefix phase clears the prefixes). Otherwise a NUL-terminated fragment follows; if `n - 1` names an existing prefix, the fragment is appended to it, and `n - 1` equal to the number of prefixes starts a new string. In the prefix phase the result becomes the next prefix; in the path phase it is a file path. Observed on PoE1: every block toggles exactly twice and ends exactly at a fragment boundary, and `n - 1` is never greater than the prefix count, so `Index::paths` treats all three as errors. All paths are lowercase ASCII with `/` separators.

**Path hash (confirmed in A2).** MurmurHash64A, seed `0x1337b33f`, over the lowercase path (`tools/extractor/src/hash.rs`, written from the published public-domain algorithm). Directory records use the same hash over the directory path without a trailing slash; the root is the empty string. FNV-1a (older patches) matches nothing. Results:

| Game | Paths rebuilt | Records matched | Paths without record | Records without path | Path reps matched |
|---|---|---|---|---|---|
| PoE1 | 1,245,702 | 1,245,702 of 1,245,702 | 0 | 0 | 86,209 of 86,209 |
| PoE2 | 4,324,296 | 4,324,291 of 4,324,296 | 5 | 5 | 97,028 of 97,028 |

The 10 unmatched PoE2 entries are a real inconsistency in the installed index, not a parser gap: 5 paths (`art/uiart/scrolling_hotmetal.{dds,dds.header,mat}`, `art/uiart/scrolling_blue_hotmetal.{dds,dds.header}`) have no record, and 5 records (in bundles `Tiny.V4`, `Tiny.V5`, `Tiny.V9`, and two `Streaming/Content/metadata/...*.ao` bundles) have no path. All are art or effect files that Spike A does not read. `list-files` prints them all (ADR-015).

**Other observations for A3.** Every file record lies inside its bundle's decoded size. PoE1 keeps tables in `data/` (`data/stats.datc64`, with the 8 language copies `data/<language>/stats.datc64` pointing at the same bundle range); PoE2 keeps them in `data/balance/` (`data/balance/stats.datc64`). Many small files share the `Tiny*` bundles, so A3/A4 should decode each bundle once and slice files out of it.

Install files and SHA-256 (2026-11-08):

| Game | Steam build | `_.index.bin` SHA-256 |
|---|---|---|
| PoE1 | 26474023 | `60264b4fc9182a442d95c29d49c0bb5097586b5081ac8f8b3032b74cdcbf19ac` |
| PoE2 | 26476662 | `9342956efd2ac4cb7864b46c3f7cb889fa3e1b49b3ca05cca7efddc9cdad7ed9` |

## 5. Other files in `Bundles2/`
- `_.index.high.bin`, `_.index.low.bin`: identical 74-byte bundles, byte-identical in PoE1 and PoE2, that decode to 8 zero bytes (an index with zero bundles). Their timestamps are older than `_.index.bin`, so the game does not rewrite them on patch. **Not needed**; not read by the extractor.
- `.index.dbg`: not a bundle (starts `u32 1`, `u32 count`; PoE1 count 1,166,053), followed by records that look like 64-bit hashes. Older than `_.index.bin` in both installs, so not updated per patch. **Not needed**; not read.

## 6. datc64 (confirmed on PoE1 and PoE2 in A3)
Layout as documented by the community (`poe-tool-dev/dat-schema`, MIT, vendored; `poe-dat-viewer`'s reader, which has no license, consulted for layout facts only, no code taken). Read by `tools/extractor/src/dat.rs`; checked on every table of both installs (section 9).

- `u32` row count, then `rows x width` bytes of fixed-width rows, then the variable section. The variable section starts with the 8-byte boundary `BB BB BB BB BB BB BB BB`; string and array offsets count from the first boundary byte (so real offsets are at least 8).
- The file does not store the row width. The reader takes the schema's width when the boundary sits exactly at `4 + rows x width`; otherwise it takes the first boundary at a whole, non-zero number of bytes per row and reports a row-size mismatch. Boundary bytes can occur inside rows, so the schema width is tried first. A table with no rows starts with the boundary.
- Column widths (the schema's `type`, `array`, `interval`):

| Schema type | Bytes | Value |
|---|---|---|
| `bool` | 1 | 0 or 1 (anything else is an error; none seen) |
| `i16`, `u16` | 2 | little-endian |
| `i32`, `u32`, `f32`, `enumrow` | 4 | little-endian; `enumrow` is the enumerator value, offset by the enumeration's `indexing` |
| `string` | 8 | `u64` offset of UTF-16LE text, ended by two zero units (a zero `u32`); every real offset is even and past the boundary, and the reader rejects others |
| `row` | 8 | `u64` row index in the same table; null is `0xFEFEFEFEFEFEFEFE` |
| `foreignrow` | 16 | `u64` row index in another table, then a `u64` that is **0 when the key is set and `0xFEFE...FE` when it is null** (counted on the 1,453,408 PoE1 and 1,036,443 PoE2 scalar `foreignrow` values of the tables whose width matches; the reader rejects anything else, since it signals misaligned columns, and no decoded table, array elements included, tripped it) |
| `interval: true` | 2 x type | two values (only `i32` intervals in the schema) |
| `array: true` | 16 | `u64` count, `u64` offset of `count x width` element bytes in the variable section; the offset of an empty array is not meaningful |
| `array` (element type unknown) | 16 | count and offset only; elements are not read |

- No key had only its low half equal to the null pattern, so the reader compares all 64 bits.
- Output cap: `read_table` stops with `EXTRACT_DAT_TOO_LARGE` beyond 8 decoded values plus string units per input byte (`DECODE_BUDGET_PER_BYTE`), so a corrupt table whose rows all point at one large array cannot multiply its output. No real table comes near it.
- Schema: version 7, 1,550 table entries (264 PoE1-only, 580 PoE2-only, 706 both; a name appears once per game it differs in), 149 enumerations. `validFor` is a bitmask (1 = PoE1, 2 = PoE2). File stems are lowercase; the reader matches table names ignoring ASCII case.

## 7. Evidence (PR A1)
- CI tests: `cargo test -p loom-extractor`. Unit tests cover identity, length checks, tool-output parsing, and spawn errors. `tests/external_tool.rs` runs `ExternalTool` against `fake-unoodle` (`src/bin/fake-unoodle.rs`, a test double, not a decoder) and covers success, reported and actual length mismatches, a missing length line, tool failure with stderr, a missing work dir, scratch cleanup, and 8 concurrent calls.
- Maintainer-run `#[ignore]` test `tools/extractor/tests/install_decompress.rs` (2026-11-08, `unoodle` 0.5.7): passed on both installs. PoE1 index 20,826,488 -> 33,449,776 bytes (128 chunks); PoE2 index 116,427,003 -> 151,240,657 bytes (577 chunks); the first and last chunks decoded on their own match the whole-payload decode; high/low bundles decode to 8 zero bytes in both.

## 8. Evidence (PR A2)
- CI tests: `cargo test -p loom-extractor`. Synthetic bundles (stored uncompressed, read with `Identity`) and synthetic indexes cover parsing, path rebuilding with shared prefixes, listing with a path that has no record, a record that has no path, a directory record that matches no directory, and a path that matches two records; and each error: inconsistent header fields, trailing bytes, undefined prefix (also after re-entering the prefix phase, which clears prefixes), a path over 4096 bytes, a block ending inside a fragment, non-UTF-8 names and paths, a file record naming a missing bundle or running past its bundle (including `offset + size` above `u32::MAX`), and a path block outside the path data. Corrupt-input tests are exhaustive rather than random: every truncation of a synthetic bundle and index is an error, and every single-byte change (6 values per byte) parses and lists without a panic. Huge counts are rejected before allocating. Path-hash vectors are 12 path/hash pairs read from the PoE1 index (directory records and file records), covering tail lengths 0 to 7 and multi-block input.
- Maintainer-run `#[ignore]` test `tools/extractor/tests/install_index.rs` (2026-11-08, `unoodle` 0.5.7, release build): passed on both installs; it checks that every path and every record is accounted for exactly once, that no directory record is unmatched, and that the stats table resolves. `tests/install_decompress.rs` now reads headers through `Bundle::parse` and still passes on both.
- `loom-extractor list-files <install>` (release build, maintainer workstation): PoE1 in about 1.5 s, PoE2 in about 5.8 s, index hashes as in the table above (unchanged since A1). Counts as in section 4.

## 9. Evidence (PR A3)
- Schema vendored at `tools/extractor/schema/schema.min.json` from `poe-tool-dev/dat-schema` release `latest`, built by upstream run 42056729237 on commit `717d788d6ec081821c33473b2a0ec35629144826` (2026-11-05); SHA-256 `e8e7e4c858924655bcfd56c23b275101f0321f435f90b19f8267bf43344ea16a`, checked by a unit test. Provenance and update steps in `tools/extractor/schema/README.md`. Upstream has no version tags, so the commit and hash pin it.
- CI tests: `cargo test -p loom-extractor`. Schema: a minimal two-game fixture (lookup by game and case, intervals, references, unknown arrays, enumerations), and each rejection (invalid JSON, unknown field, unsupported version, unknown or empty `validFor`, unknown type, `array` type that is not an array, interval on a string, array plus interval, duplicate table or enumeration per game, also by case); the vendored file loads for both games. Dat: a synthetic two-row table with every column kind (values row, and a row of nulls, empty arrays, and an empty string); wider rows (mismatch, all columns read); narrower rows (mismatch, only the columns that fit read); a schema with no columns; an empty table under any schema; boundary bytes inside a row (schema width wins); fallback to the first aligned boundary; each value error (bad bool, string offset out of range, inside the boundary, or odd, unterminated, unpaired surrogate, array out of range including overflowing counts, bad `foreignrow` second word, including inside an array with its row and column in the message) and each framing error; a float interval; 100 rows sharing one 1,000-element array stop at the output cap. Every truncation of the synthetic table is an error, and every single-byte change (7 values per byte) returns without a panic. `read_files`: files sliced from two bundles (one in a subdirectory) in bundle order, and each error (record past the bundle, unknown bundle, decoded size differs from the index, missing bundle file, corrupt bundle, and bundle names that are absolute, contain `..`, `.`, empty parts, a backslash, or a colon, since index names become paths).
- Maintainer-workstation run of the `#[ignore]` test `tests/install_dat.rs` (2026-11-08, `unoodle` 0.5.7, release build, about 19 s for both games). Index hashes unchanged since A1. Every table directly in the game's table directory was read with the vendored schema; the test fails if any table fails to decode. Re-run after the review fixes (output cap, stricter string offsets, bundle-name check) with identical results:

| | PoE1 (`data/`) | PoE2 (`data/balance/`) |
|---|---|---|
| Tables in the directory | 957 | 1,025 |
| Decoded, width matches the schema | 942 (2 with no rows; 564,393 rows) | 1,008 (91 with no rows; 429,900 rows) |
| Row-size mismatch (all decoded, rows wider than the schema) | 2 | 6 |
| Failed to decode | 0 | 0 |
| No schema entry | 13 | 11 |
| Schema entries with no file | 26 | 272 |

  Mismatches (schema bytes -> file bytes): PoE1 `hideoutdoodads` 156 -> 157, `mapdevices` 110 -> 126. PoE2 `coreleagues` 238 -> 254, `endgamemapcontentvisualidentity` 40 -> 43, `hideoutdoodads` 172 -> 173, `hideoutsalvagebenchinterfacevisuals` 48 -> 50, `itemvisualidentity` 968 -> 992, `mapdevices` 110 -> 126. In every case the rows are wider, so all schema columns were read and the extra trailing bytes are unknown columns.

  Tables with no schema entry: PoE1 `abyssmonstermodreplacement`, `additionallifescalingperlevel`, `breachstashtablayout`, `dynamiccraftingbenchinventorytype`, `eventcoins`, `goldmodprices`, `graphicalitemreceptacleadditionalbutton`, `graphicalitemreceptacleslot`, `leaderboardmapdevice`, `leaderboardmapdevicelabel`, `mercenarydisableduniques`, `missiontilemap`, `statistictrackingmicrotransactionsonkill`. PoE2 `additionallifescalingperlevel`, `atlaspoem`, `endgameplayerhideoutapplymods`, `endgameplayerhideoutrewards`, `eventcoins`, `missiontilemap`, `mobileatlaseldermemories`, `mobileatlasinventorylayout`, `mobilequestaudio`, `specialareabonusmapstats`, `tencentcharacterpanelstatslistener`.

  All six Spike A tables (Stats, Mods, BaseItemTypes, PassiveSkills, ActiveSkills, GrantedEffects) decode with no mismatch on both games; the test asserts this. None of the mismatched or schema-less tables is one A4 extracts.

## 10. Extraction output (PR A4)
`loom-extractor extract <install> --game poe1|poe2 --out <dir>` (`tools/extractor/src/extract.rs`, edge in `src/main.rs`). Spike-level output, not the Phase 2 canonical bundle format.

- **What is extracted:** the six Spike A tables plus every table they reach, transitively, through any column that references a table (enumeration references are not tables and are not followed). PoE1: 72 tables; PoE2: 85. Tables come from the main table directory (`data/` for PoE1, `data/balance/` for PoE2), which holds the English text; per-language copies are not read. The manifest records this as `table_dir` and `text_language`.
- **Table files:** `tables/<SchemaName>.json`, with one row per line. Keys are `columns` (one descriptor per decoded column: `array`, `interval`, `name`, `references`, `type`), then `rows` (arrays in file row order, one value per decoded column), then `table`. Numbers are formatted by Rust and `serde_json` without locale. `f32` is written as the shortest decimal that reads back to the same `f32` (`ryu` via `serde_json::to_string(&f32)`, never widened to `f64`, so `0.1` stays `0.1`). `u64` keys are exact integers. Null keys are `null`. Enum columns hold the stored number. Arrays of unknown element type are `{"count","offset"}`. NaN and infinities are the strings `"NaN"`, `"Infinity"`, `"-Infinity"` and are counted per table (none in either install). Final newline, LF only.
- **Manifest (`manifest.json`, format 1):** every key sorted. Manifest structs declare their fields in alphabetical order and maps are `BTreeMap`, because something in the workspace enables `serde_json`'s `preserve_order` (via `schemars`), so `serde_json::Map` order depends on the build. The manifest records the game, `steam_build_id` (from Steam's `appmanifest_*.acf` whose `installdir` matches the install folder), the index path and SHA-256, the SHA-256 of every bundle read, the schema source, commit and SHA-256, the extractor version, git commit, and `dirty` (`git status --porcelain` not empty, untracked files included), and the decoder program name and its `--version` output. For each output file it records the row count and SHA-256. Every planned or referenced table appears with a `status` (`ok`, `row-size-mismatch`, `failed`, `no-file`, `no-schema`), its source path, row width, mismatch, columns decoded, non-finite float count, the tables that reference it, and a range check per key column (`foreignrow` and `row` keys at or past the target's row count; `dangling: null` when the target was not decoded). No timestamps or machine paths: the decoder is recorded by file name only.
- **Game version string:** not recorded. Neither install has a version file that the extractor reads reliably; the Steam build ID and the index hash identify the input.
- **Failure policy:** a table that does not decode is recorded (`failed`, with its error code), and the other tables are still written. The command exits non-zero if any table failed or a root table was not decoded. The output folder must not exist, so no stale file survives a run.
- **Determinism:** by construction (`BTreeMap` everywhere output is assembled, file row order, no parallel assembly), checked by the unit test `extract_should_not_depend_on_add_order` (same bytes whichever order tables arrive in, which `read_files` sets by bundle) and a golden manifest test. Two-run check: `tools/extractor/scripts/check-determinism.sh <install> <game>` (output in `local/determinism/<game>/`).

## 11. Evidence (PR A4)
- CI tests (`cargo test -p loom-extractor`):
  - Plan: transitive closure, enum references not followed, a table with no schema still planned, per-game differences.
  - Exact table JSON for a synthetic two-row table: escaped and non-ASCII strings, null keys, arrays, `0.1_f32` written as `0.1`, NaN as a string.
  - Number formatting: shortest `f32` round-trip (`0.1`, `-0.0`, `1e-7`, `3.4028235e+38`), `u64` keys above 2^53 kept exact, non-finite floats counted inside intervals and arrays.
  - Every order of adding three tables gives byte-identical files and manifest.
  - Golden manifest with sorted keys.
  - Key range checks: `foreignrow`, `row` self keys, keys above `usize`, keys into an undecoded target left unchecked; `key_target` for every column shape.
  - `problems()`: failed table, root not decoded, root with a row-size mismatch, dangling keys.
  - A failed table and a mismatched table in one run.
  - Rejected `add` paths: unplanned, schema-less, a language copy, wrong extension, duplicate.
  - `read_files` returns bundle hashes.
  - Steam `.acf` parsing and build-ID lookup.
  - `planned_files`: match ignoring case, missing file, a path listed twice.
  - `write_output`: written through `.partial` and renamed; an existing output or partial folder is refused.
- Maintainer workstation run, 2026-11-09. Tree clean at extractor commit `d047e4a` (manifest `dirty: false`), `unoodle` 0.5.7, release build. `tools/extractor/scripts/check-determinism.sh` ran `extract` twice per game and compared every file byte for byte. The manifests are committed unchanged (they hold no machine paths) as `docs/spikes/spike-a/manifest-poe1-26474023.json` and `manifest-poe2-26476662.json`.

| | PoE1 | PoE2 |
|---|---|---|
| Steam build / index SHA-256 | 26474023 / `60264b4f…cdcbf19ac` | 26476662 / `9342956e…c9cdad7ed9` |
| Tables planned (roots + referenced) | 72 | 85 |
| Status | 72 `ok` | 84 `ok`, 1 `row-size-mismatch` (`ItemVisualIdentity` 968 -> 992 bytes, all 90 schema columns read; section 9) |
| Rows written | 308,214 | 224,218 |
| Bundles read | 21 | 61 |
| Key columns / key values checked / dangling | 262 / 905,891 / 0 | 322 / 388,069 / 0 |
| Non-finite floats | 0 | 0 |
| Root rows (Stats, Mods, BaseItemTypes, PassiveSkills, ActiveSkills, GrantedEffects) | 23,362, 40,355, 5,461, 5,706, 1,941, 12,829 | 27,299, 16,911, 5,496, 9,731, 1,148, 8,350 |
| Two runs identical | 73 files | 86 files |
| Manifest SHA-256 | `64a9e8eb627b9269a50b0d4002dca6de9dea908dacf7426da04787b669189471` | `952e63fdc5d206e519409e39dc665516fcb728ecb6fe7e7c047bcbac2a658ff6` |
| Time per run | about 4 s | about 12 s |

  Gate items this covers: 1 (determinism, both games) and 4 (recorded inputs: build ID, index hash, schema commit and hash, extractor commit, `dirty: false`). Gate 3 is partial: every row-size mismatch of an extracted table is in the manifest with a status; stat-description constructs are A5. Gate 2 (coverage against PoB) is A6.
- Not exposed: trailing bytes of rows wider than the schema (`ItemVisualIdentity` in PoE2). Their width is in the manifest; reading them needs a schema update, not extractor work.

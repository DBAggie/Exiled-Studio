# Spike A: Bundles2 game-data extraction (report)

Plan: `dev/active/spike-a-bundles2-extraction/`. Gate: `docs/plan/ACTION-PLAN.md`, Spike A. This report grows one section per PR (A1 to A6b); the gate verdict is written at A6b.

Status (2026-11-09, PR A6a): decompressor decided (A1); bundle container and index read by `loom-extractor` on both installs, path encoding and path hash confirmed (A2); datc64 layout confirmed and every table with a schema entry decoded on both installs with the vendored community schema (A3); `extract` writes the Spike A tables and every table they reference as deterministic JSON with a manifest, checked byte-identical over two runs on both installs (A4, sections 10 and 11); stat-description files of both games parsed with zero unparsed English lines, every finding categorized in the manifest (A5, sections 12 and 13); `cross-check` compares the output with PoB's newest data exports in both directions and categorizes every gap of mods, bases, skills, and stats on both games (A6a, sections 14 and 15). Gate items 1, 3, 4, and 5 met; item 2 met for mods, bases, skills, and stats, not yet for passives (our PassiveSkills rows that PoB's tree lacks are uncategorized until A6b reads the passive skill graph).

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
- Maintainer workstation run, 2026-11-09. Tree clean at extractor commit `d047e4a` (manifest `dirty: false`), `unoodle` 0.5.7, release build. `tools/extractor/scripts/check-determinism.sh` ran `extract` twice per game and compared every file byte for byte. The manifests are committed unchanged (they hold no machine paths) as `docs/spikes/spike-a/manifest-poe1-26474023.json` and `manifest-poe2-26476662.json`. A5 replaced both with format-2 manifests (section 13); the A4 versions are in git history at `d480d1f`.

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

## 12. Stat-description files (observed on PoE1 and PoE2 in A5)
Parser: `tools/extractor/src/statdesc.rs`, run by `extract`. Format knowledge comes from the files themselves (surveyed line by line on both installs) and was compared with PoB's exporter `src/Export/statdesc.lua` (read only, no code taken).

- **Where:** PoE1 `metadata/statdescriptions/*.txt` (45 files; the plan's `.csd` guess was wrong for PoE1). PoE2 `data/statdescriptions/**/*.csd` (589 files: 27 at the top, the rest per skill in `specific_skill_stat_descriptions/`, some as `<skill>/statset_<n>.csd`). Both are UTF-16LE with a byte-order mark and CRLF lines, and share one grammar.
- **Not stat descriptions:** PoE1 `skillpopup_stat_filters.txt` holds `group <name> { ... }` lists for the skill pop-up. Any `*_filters` file is hashed, listed as `skipped`, and not parsed.
- **Indentation is not reliable** (many blocks start at column 0), so the parser works on tokens per non-blank line: bare words, and `"..."` strings. No string holds an escaped quote or spans lines (checked on both installs).
- **Top level:** `include "<path>"` (path as in the index, mixed case: `Metadata/StatDescriptions/x.txt`, `Data/StatDescriptions/x.csd`); `has_identifiers` / `no_identifiers` (file flags, meaning unknown, recorded as written); `no_description <stat_id>`; `description [<name>]`; `handed_description [<name>]`.
- **Description block:** `<n> <stat_id> x n`, then the English variant list `<m>` followed by `m` variant lines, then any number of `lang "<Language>"` lists of the same shape. `handed_description` lists `n` main-hand IDs and then `n` off-hand IDs, which may span lines (PoE1 `skill_stat_descriptions.txt`, 5 blocks; none in PoE2). Its variants have `n` conditions.
- **Variant line:** `n` conditions, optional flags (`gem_quality`, PoE2 also `table_only`), the format string, then handlers. Conditions: `#` (any), `<v>`, `<a>|<b>` (either side `#`), `!<v>` (not `v`). Handlers: `<name> <stat>` with a 1-based stat number (61 names on PoE1, 54 on PoE2, counted in the manifest), `reminderstring <ReminderText id>`, and the bare `canonical_line` and `markup`.
- **Format string:** placeholders `{<index>[:<format>]}` with a 0-based index (`{}` and `{:+d}` mean stat 0) and formats `d`, `+d`, `-d`, `+`, or none. `{{` and `}}` are literal braces: `<enchanted>{{{0}% ...}}` is a markup span around a placeholder. `\n` is a line break. Markup is display text and is kept as written: `<tag>{...}`, `[Ref|Text]` / `[Text]` links, `<<Icon>>`. Spike B has to strip it before matching item text.
- **Languages:** English is the list before the first `lang` and is the only one parsed. Others are counted per file (PoE1 10 languages, PoE2 11, Turkish only in PoE2). They contain constructs that the English text never uses (for example `1|1|#` in Portuguese, `{0%}`, `{0 }`); not parsing them is deliberate.
- **Findings:** a line that does not fit the grammar is not an error. It becomes a finding with one kind from a fixed list (`FindingKind`: `unknown-line`, `unterminated-string`, `bad-include`, `missing-include`, `include-not-parsed`, `include-cycle`, `bad-no-description`, `bad-description-header`, `bad-stat-list`, `bad-variant-count`, `bad-language`, `bad-condition`, `condition-count`, `unknown-variant-flag`, `missing-string`, `bad-handler`, `handler-stat-out-of-range`, `bad-placeholder`, `placeholder-out-of-range`), its line number, the line text, and how many lines were left out because of it. A bad variant line drops only that line; a bad block header, stat list, or other-language count drops the block up to the next top-level construct, and the count is recorded. A bad English count keeps the description (stats and name) with no variants. Includes are checked across files: one that names no file in the folder, names a file that was not parsed (failed or skipped), or leads back to the including file is a finding. Counts, conditions, and handler stats accept digits with an optional `-` only. Range conditions with `min > max` are kept as written (none occur).
- **Output:** `stat_descriptions/<path below the folder>.json`, one description per line (`handed`, `line`, `name`, `off_hand_stats`, `stats`, `variants` with `conditions`, `flags`, `handlers`, `line`, `placeholders`, `text`), then the file's `file_flags`, `includes`, `languages`, `no_description`, and `source`. The manifest (format 2) adds `stat_descriptions`: the folder, every file with its kind, status, hash, counts, includes, and languages, every finding, finding counts by kind, handler counts, and flag counts. The run fails if no stat-description file is found, one is not UTF-16, or its path is not a plain relative path (`EXTRACT_STATDESC_BAD_PATH`; the path becomes an output path). Findings alone do not fail it: Spike B and A6 must read `findings_by_kind`.

## 13. Evidence (PR A5)
- CI tests (`cargo test -p loom-extractor`, 112 tests):
  - Decoding: BOM dropped, surrogate pairs kept, odd length and unpaired surrogate rejected with their codes.
  - Tokens and every condition form, including rejected ones (`1|1|#`, `1|`, `!#`).
  - Every construct on synthetic snippets: includes, file flags, `no_description`, named and unnamed descriptions, multi-stat variants with flags and every handler shape, other languages counted, a handed description spanning lines, placeholders (every format, literal braces, markup).
  - Every finding kind, with its line and skipped count, and recovery at the next description.
  - Every prefix of a file parses without a panic; every truncation and seven byte values at every offset of a UTF-16 file decode or fail without a panic.
  - Collection: filter file skipped, missing include, include of a failed or skipped file, include cycles, paths with `..`, empty parts, `\`, or `:` failed, duplicate and unplanned paths (`EXTRACT_DUPLICATE_FILE`, `EXTRACT_UNPLANNED_FILE`), nested PoE2 output paths, handler and flag counts.
  - A description with a bad English count is kept without variants; a leading `+` in counts, conditions, and handler stats is a finding.
  - Exact per-file JSON; golden manifest (format 2) with a finding; same bytes for every rotation of adding three tables and a stat-description file; `problems()` for no stat-description file and an undecodable one; `planned_files` picks the game's stat-description files only.
- Maintainer workstation run, 2026-11-09. Tree clean at extractor commit `9394d75` (`dirty: false`; after the review fixes), `unoodle` 0.5.7, release build. `check-determinism.sh` ran `extract` twice per game. Manifests committed as `docs/spikes/spike-a/manifest-poe1-26530219.json` and `manifest-poe2-26476662.json`.
- **Steam updated PoE1 between A4 and A5** (build 26474023 to 26530219, index SHA-256 `60264b4f…` to `d23c7899…`). Every bundle and table file that A4 reads has the same SHA-256 as before, and the row counts are identical, so the A4 table results stand.

| | PoE1 | PoE2 |
|---|---|---|
| Steam build / index SHA-256 | 26530219 / `d23c7899…0eb516f1488df4a1ccf39b0d51be54d0` | 26476662 / `9342956e…c9cdad7ed9` |
| Stat-description files | 45 (44 `ok`, 1 `skipped`: `skillpopup_stat_filters.txt`) | 589 (589 `ok`) |
| Descriptions / English variants | 17,739 / 26,794 | 19,657 / 29,837 |
| `no_description` / includes | 100 / 39 | 412 / 583 |
| Other-language lists (languages) | 176,483 (10) | 195,682 (11) |
| Handler names / flags | 61 / `gem_quality` 34 | 54 / `gem_quality` 203, `table_only` 1,621 |
| Findings (unparsed lines) | 0 | 0 |
| Tables | 72 `ok` (unchanged from A4) | 84 `ok`, 1 `row-size-mismatch` (unchanged) |
| Two runs identical | 117 files | 675 files |
| Manifest SHA-256 | `201fd82bcbbd7efd60e1006aff25309515074be5a90abd3863e6b3a49dbb453c` | `776d9acb09815a88082c3cb2815a99ec58c6dc8700606329c436bc0ed9e40b92` |

  Counts were checked independently of the parser with `grep`/`awk` over the decoded files: description keywords, `no_description` lines, `lang` lines, and English variant lines (56,631 for both games together) all match.
- Gate 3 (no silent drops) is met: every row-size mismatch and every unparsed stat-description line is in the manifest with a category; on these installs there are no unparsed English lines. Gates 1 and 4 hold again for the new output. Gate 2 is A6.
- PoE2's manifest is 1.2 MB (A4: 106 KB): it lists 589 files, each with an 11-language count map.

## 14. Cross-check against PoB (A6a)
Tool: `loom-extractor cross-check <extract-dir> <pob-dir> --game poe1|poe2 --out <dir>` (`tools/extractor/src/crosscheck.rs`, PoB readers in `crosscheck/pob.rs`, our-output readers in `crosscheck/ours.rs`). It writes `report.json` (every gap with its category and rule) and `summary.json` (the same without the key lists of `pob-filtered` rules) and exits non-zero when the gate is not met.

- **PoB baseline (searched upstream 2026-11-09):** no data export newer than PoE1 "3.29.3 Export" `1b4078dc4` (2026-08-23) or PoE2 "Export 0.5.5 data" `49e93925d` (2026-09-10). Checked: the default `dev` branches (newest commits to `src/Data/Mod*.lua` and `src/Data/Skills` are those exports or code fixes), `beta` and `master`, and open pull requests with "export" in the title. Both repositories are shallow-cloned at those commits into `local/pob/` (`git init` + `git fetch --depth 1 <url> <sha>`); the legacy repository is not touched.
- **Families, like for like:** mods (PoB `src/Data/Mod*.lua` entry keys vs `Mods.Id`), bases (`src/Data/Bases/*.lua` names vs `BaseItemTypes.Name`, cleaned as PoB does: trimmed, `ö` written as `o`), skills (`src/Data/Skills/*.lua` keys vs `GrantedEffects.Id`), stats (stat IDs in `src/Data/StatDescriptions/**/*.lua` vs `Stats.Id`), passives (node IDs in the newest `src/TreeData/<major>_<minor>/tree.lua` vs `PassiveSkills.PassiveSkillGraphId`). Four mod files are keyed by something other than a mod Id (`ModCache` by text, `ModScalability` by stat text, `ModFoulbornMap` by unique name, PoE2 `ModRunes` by rune and soul-core base name); they are listed under `excluded_pob_files` with how many of their keys equal a `Mods.Id` (0 or 2 of about 15,000, so a different key space).
- **Both directions:** PoB keys missing from our output (`pob_missing`) and our keys PoB does not have (`ours_extra`). Each gap gets the first rule that matches, from the fixed category list (plan step 6; A6b added `not-on-tree`, see section 17). A gap no rule explains is `uncategorized`.
- **Readers are narrow line patterns, not a Lua parser.** A line that starts like a key and does not complete the pattern is reported as `unparsed`, and an unparsed line fails the family. `#baseMatch` values are Lua patterns (`Wand[MEK]?%d+`); the tool has a small Lua pattern matcher (classes, sets, `? * + -`, anchors) that rejects captures, `%b`, `%f`, and back-references instead of guessing.
- **Rules reproduce PoB's export from its own files.** Spec lists are read from the scripts (`skills.lua`'s `for _, name in pairs({...})`, `bases.lua`'s `itemTypes`), mod Id exclusions from every `not mod.Id:match("<pattern>")` in `mods.lua`, and the stat files `describeMod` uses from `mods.lua`'s `loadStatFile(...)`. Rules taken from script logic are tied to a snippet of the script; if the snippet is missing (PoB changed the script, or the other game's script never had it), the rule is disabled and listed in `disabled_rules`, so it cannot explain a gap.

Rules (category, rule: what it checks):

| Family | Direction | Rule |
|---|---|---|
| mods | ours extra | `pob-filtered` id-pattern-excluded: the Id matches a `not mod.Id:match(...)` pattern of `mods.lua` (per-pattern counts in `breakdown`) |
| | | `pob-filtered` zero-values: every stat `describeMod` reads (slots 1 to 6, without buff-template stats) is 0 to 0; `describeStats` skips such stats (`v.min ~= 0 or v.max ~= 0`, `Export/statdesc.lua`) |
| | | `pob-filtered` no-description-line: reproducing `describeStats` over the files `mods.lua` loads gives no line (`#orders > 0`) |
| | | `pob-filtered` dnt-first-line: the first line, after `escapeGGGString`, starts with `DNT` (PoE1 only) |
| | | `pob-filtered` aura-bonus-family, unique-generation-without-family (PoE1), aura-flags (PoE1): the writer conditions in `mods.lua` |
| | | `pob-filtered` domain-generation-not-exported: PoB has no mod of this (domain, generation type) pair (per-pair counts in `breakdown`, with schema enum names); a heuristic, see below |
| | | `patch-delta` passes-reproduced-filters: PoB exports mods of this pair and no rule above leaves it out; a heuristic, see below |
| bases | PoB missing | `pob-generated` display-name-in-spec (`#base <Id> <display name>`), energy-blade-rename |
| | ours extra | `pob-filtered` not-in-base-spec, royale (PoE1), dnt-name (PoE2), renamed-by-spec, energy-blade-rename |
| skills | PoB missing | `pob-generated` hand-written-in-spec (`skills["Id"] = {` in a processed spec, not a `#skill`) |
| | ours extra | `pob-filtered` not-in-skill-spec; `patch-delta` listed-not-in-output (a `#skill` line whose key is not in PoB's output; `skills.lua` writes the key of every listed skill before any check, so PoB's output predates the spec line) |
| stats | PoB missing | `pob-generated` handed-placeholder (PoB writes `0` as the stat list of handed descriptions); `not-in-index` stale-pob-description-file (only PoB files with no installed counterpart name it; PoB keeps converted files of removed game files). A PoB stat that an installed description names but the Stats table lacks is left `uncategorized` (none occur): it could be an extraction gap |
| | ours extra | `pob-filtered` handed-description; not-in-converted-descriptions (no installed counterpart of a file PoB converts names it) |
| passives (A6b) | ours extra | PoE2 `pob-filtered` empty-name, dnt-name, skipped-ascendancy (the three node filters of `passivetree.lua`, on the Default tree's graph only), not-on-default-graph (PoB's PoE2 tree is built from that graph only); PoE1 `not-on-tree` not-on-default-graph (section 17) |
| all | both | `schema-mismatch` table-not-ok: the table's manifest status is not `ok`. Since A6b this rule comes first, so every gap of the family falls under it and fails the gate, because the decoding could be their cause (no family table is affected on these installs) |

- **Writer scoping of the mod rules** (after the review): PoB has several mod writers, and a pattern one excludes another may write (`^Synthesis` is left out of `ModExplicit` but is `ModSynthesis`'s whole content). So an `Id`-pattern rule only explains a gap whose (domain, generation type) pair has no exported mod matching that pattern. The generic writer conditions (AuraBonus family, no family, aura flags) only explain a gap whose pair has no exported mod breaking them; mods of a specialised writer (their Id matches a pattern that PoB still exports for their pair) neither break those conditions nor fall under them. A new Synthesis or Necropolis mod therefore lands in `patch-delta`, counted against the ceiling.
- **Heuristics and their blind spot:** `domain-generation-not-exported` and `passes-reproduced-filters` come from PoB's output (which pairs it exports), not from its writer conditions (`writeMods(file, condFunc)` in `mods.lua` is not parsed). A (domain, generation type) pair that a game patch adds is called `pob-filtered`, not `patch-delta`; a writer filter that is not reproduced (the DelveFossil spawn-tag skips, `ModJewelCluster`'s conditions) would show up as `patch-delta`. Reproducing the writer list is a follow-up task.
- **`describeMod` reproduction** (the largest rule, from PoB's `Export/statdesc.lua`): the files are loaded in order with includes read in place, and the last description naming a stat wins (`no_description` maps a stat to nothing); the stats are the mod's slots 1 to 6 without the stats of its buff templates (`BuffTemplates` key array, picked by reference because the column names differ between the games' schemas); a stat that is 0 to 0 is skipped; each description takes its first English variant (`table_only` skipped) whose conditions hold for the minimum values, retrying once with a minimum of 0 raised to 1 for ranges above 0. Our output stores a file's `no_description` list apart from its descriptions, so they are applied first; PoB applies them in line order, and the files list them before the descriptions. Description order runs over all files; PoB restarts it at 1 for a file loaded in append mode, which can only change which line comes first (the `dnt-first-line` rule), not whether there is one.
- **Disabled rules are expected per game:** PoE1 `bases` dnt-name (PoE1's `bases.lua` has no DNT check); PoE2 `bases` royale, and `mods` dnt-first-line, unique-generation-without-family, aura-flags (PoE2's scripts do not have those conditions). Any other disabled rule means PoB changed a script and the rule needs review.
- **`patch-delta` without a newer PoB export:** the plan asked for a diff against the next PoB export; none exists. The two `patch-delta` rules instead rest on reproduced PoB behaviour and keep their keys in the summary; the 2% ceiling bounds them. Both PoE1 cases were checked by hand. The mod: PoB has `V2MinPowerChargesCorrupted` with the text "+1 to Minimum Frenzy Charges"; the install adds `V2MinPowerChargesCorruptedNew` with power charges, and PoB `dev` (2026-11-06) does not have it yet. The skill: `EDSHarvestMinerHammerSmoke` is listed in `Export/Skills/spectre.txt` at the export commit but missing from the generated `spectre.lua`; since `skills.lua` writes the key before any check, PoB's output was generated before that spec line (a difference between PoB's export and its own spec, counted as `patch-delta`).
- **Divergence register:** these gaps are differences from PoB (ADR-006). They become register entries once Spike C defines the register format; `summary.json` lists the keys of every category except `pob-filtered` (counts only).

## 15. Evidence (PR A6a)
- CI tests (`cargo test -p loom-extractor`, 124 library tests, 25 of them for the cross-check, plus 3 CLI tests in `main.rs`; after the review the synthetic run also covers writer scoping, a specialised writer's mod, an installed-only stat left uncategorized, and `schema-mismatch` failing the gate): every PoB reader on snippets, including malformed lines and `#skillEnd`; the Lua pattern matcher against `string.find` behaviour and its rejected features; `escapeGGGString` (one case checked against Lua's greedy match); a synthetic PoB checkout and extract output that puts one key under every rule of every family, `schema-mismatch` for a table that is not `ok`, the patch-delta ceiling, and the gate's `not_met` lines; disabled rules when script snippets are missing; unparsed lines failing a family; wrong game, missing file, unknown `#baseMatch` column, unsupported pattern, and missing spec list as errors; the `describeMod` reproduction (load order, includes, `no_description`, `table_only`, the zero retry, `!` conditions, zero stats); the summary keeping all but `pob-filtered` keys; identical report bytes for two runs; CLI parsing, checkout commit reading, and file listing.
- Maintainer workstation run, 2026-11-09, tool commit `33730ed` (`tool_dirty: false`, after the review fixes), on the A5 extract output (extractor `9394d75`, clean; the Steam builds have not changed since). Two runs per game gave byte-identical `report.json` and `summary.json`. Summaries committed as `docs/spikes/spike-a/cross-check-poe1-26530219.json` and `cross-check-poe2-26476662.json`.

| | PoE1 (3.29.3 `1b4078dc4`, tree 3_29) | PoE2 (0.5.5 `49e93925d`, tree 0_5) |
|---|---|---|
| **mods** keys PoB / ours / matched | 23,539 / 40,355 / 23,539 | 9,059 / 16,911 / 9,059 |
| ours extra: `pob-filtered` id-pattern-excluded | 4,440 | 2,647 |
| ours extra: `pob-filtered` zero-values | 179 | 191 |
| ours extra: `pob-filtered` no-description-line | 3,319 | 2,941 |
| ours extra: `pob-filtered` dnt-first-line | 27 | 0 (rule not in the PoE2 script) |
| ours extra: `pob-filtered` aura-bonus-family | 121 | 14 |
| ours extra: `pob-filtered` unique-generation-without-family | 5 | 0 (rule not in the PoE2 script) |
| ours extra: `pob-filtered` domain-generation-not-exported | 8,724 | 2,059 |
| ours extra: `patch-delta` passes-reproduced-filters | 1 (`V2MinPowerChargesCorruptedNew`) | 0 |
| **bases** keys PoB / ours / matched | 1,106 / 4,310 / 1,091 | 1,769 / 4,638 / 1,764 |
| PoB missing: `pob-generated` display-name-in-spec / energy-blade-rename | 13 / 2 | 3 / 2 |
| ours extra: `pob-filtered` not-in-base-spec / dnt-name / renamed-by-spec / energy-blade-rename | 3,217 / 0 / 1 / 1 | 2,822 / 51 / 0 / 1 |
| **skills** keys PoB / ours / matched | 1,521 / 12,829 / 1,508 | 1,429 / 8,350 / 1,429 |
| PoB missing: `pob-generated` hand-written-in-spec | 13 | 0 |
| ours extra: `pob-filtered` not-in-skill-spec | 11,320 | 6,921 |
| ours extra: `patch-delta` listed-not-in-output | 1 (`EDSHarvestMinerHammerSmoke`) | 0 |
| **stats** keys PoB / ours / matched | 13,665 / 23,362 / 13,664 | 14,535 / 27,299 / 14,526 |
| PoB missing: `pob-generated` handed-placeholder | 1 (`0`) | 0 |
| PoB missing: `not-in-index` stale-pob-description-file | 0 | 9 (192 of PoB's 764 PoE2 files have no installed counterpart; checked against the installed index with `list-files --all`: 589 `.csd` files, the same 192 absent, and spot checks such as `arc` and `advancing_strike` find no file under any name) |
| ours extra: `pob-filtered` handed-description / not-in-converted-descriptions | 20 / 9,678 | 0 / 12,773 |
| **passives** keys PoB / ours / matched | 3,396 / 5,706 / 3,396 | 4,914 / 9,731 / 4,914 |
| ours extra: `uncategorized` (categorized in A6b, section 17) | 2,310 | 4,817 |
| Uncategorized / unparsed PoB lines (mods, bases, skills, stats) | 0 / 0 | 0 / 0 |
| patch-delta (limit = 2% of PoB keys) | mods 1 (470), skills 1 (30) | 0 |
| Gate 2 verdict | met for mods, bases, skills, stats; not met for passives | same |

- Every PoB key of every family is in our output except the ones categorized above: no PoB mod, passive node, or (PoE2) skill is missing, and the missing bases, skills, and stats are all PoB-made names or keys of removed game files.
- **Review:** `code-architecture-reviewer` (2026-11-09). Applied: blocker (`schema-mismatch` gaps fail the gate); writer scoping of the `Id`-pattern and writer-condition rules; the skill rule's evidence and anchor (`listed-not-in-output`); the installed-only stat rule removed; the index check of the stale files; documentation of the heuristic rules, the append order, and the expected disabled rules. Not applied: parsing `writeMods` condFuncs (follow-up task); renumbering appended descriptions; null `Id`/`Name` rows (none occur) becoming an empty key.
- **Passives (A6b):** PoB's PoE1 tree comes from GGG's published tree file, not from a filter over PassiveSkills, and PoE2's from PoB's own tree export. The extra rows (PoE1: atlas, alternate-ascendancy, royale, and legacy nodes; PoE2: atlas, Brequel, and others) need the game's own placement evidence, the passive skill graph file, which the extractor does not read yet. Maintainer decision 2026-11-09: split A6 into A6a (this PR) and A6b (read the passive skill graph, categorize the passive gaps, final gate verdict).

## 16. Passive skill graphs (observed on PoE1 and PoE2 in A6b)
A passive tree's layout is a `.psg` file; `PassiveSkillTrees.PassiveSkillGraph` names the graph of each tree (the row `Default` is the character tree, `Metadata/PassiveSkillGraph` in both games). A `PassiveSkills` row is on a tree only if its `PassiveSkillGraphId` is a node of that tree's graph. The installs have 7 (PoE1) and 5 (PoE2) graphs:

| Graph (`metadata/...`) | Kind byte | PoE1 nodes / connections / roots | PoE2 nodes / connections / roots |
|---|---|---|---|
| `passiveskillgraph.psg` (character tree) | 0 | 2,987 / 3,359 / 23 | 5,152 / 6,070 / 6 |
| `alternateskillgraphs/customascendancypassiveskillgraph.psg` | 0 | 2,928 / 3,284 / 21 | - |
| `alternateskillgraphs/royalepassiveskillgraph.psg` | 0 | 212 / 185 / 6 | 6 / 0 / 6 |
| `atlasskillgraphs/atlasskillgraph.psg` | 1 | 1,029 / 1,004 / 1 | 576 / 586 / 7 |
| `atlasskillgraphs/atlasskillgraphcurrentleague.psg` | 1 | 959 / 932 / 1 | - |
| `atlasskillgraphs/atlasskillgraphblank.psg` | 1 | 1 / 0 / 1 | - |
| `atlasskillgraphs/endgamemaplayoutgraph.psg` | 1 | - | 611 / 800 / 0 |
| `leagueskillgraphs/chayulatreepassiveskillgraph.psg` | 2 | 134 / 132 / 4 | 248 / 245 / 5 |

Layout (little-endian), found by hex inspection and confirmed by parsing all 12 files to their last byte:

```text
u8   version                 3 (every file)
u8   kind                    0 passive tree, 1 atlas, 2 league (observed meaning)
u8   orbit count n           PoE1 7, PoE2 10
u8   slots[n]                nodes per orbit ring: PoE1 1,6,16,16,40,72,72; PoE2 1,12,24,24,72,72,72,24,72,144
u32  root count, roots       u32 node IDs (PoE1) or u64 (PoE2)
u32  group count, then per group:
  f32 x, f32 y               group centre
  u32 flags, u32, u8         flags 0-2 observed; the u32 0-5, the u8 0-1 (meaning unknown, kept as stored)
  u32 node count, then per node:
    u32 id                   PassiveSkills.PassiveSkillGraphId
    u32 orbit, u32 index     always orbit < n and index < slots[orbit]
    u32 connection count, then per connection:
      u32 id                 always a node of the same graph
      i32 orbit              PoE2 only: signed arc, -9..9 and 2147483647 observed
```

- **The two games differ in two widths**, not in the version byte: PoE1 roots are `u32` and PoE1 connections are bare node IDs; PoE2 roots are `u64` and each connection carries an `i32`. The parser takes the game as input; parsing a file with the other game's widths fails (unit test), so a misread is never silent.
- **PoB's readers:** PoE1 `src/Export/psg.lua` reads an older layout (7-byte header, nodes without orbit checks) and is not used for PoB's PoE1 tree anymore: PoB's PoE1 `tree.lua` comes from GGG's published tree export. PoE2 `src/Export/Scripts/passivetree.lua` reads this layout (`u16` version 3, skips 11 bytes, `u64` roots, `(id, radius)` connections) for the `Default` tree only.
- **Duplicates and dangling references:** one PoE2 atlas node ID is listed twice in `atlasskillgraph.psg` (counted in the manifest as `duplicate_nodes`); no other graph has duplicates. No connection points outside its graph; that is treated as an invariant, so a dangling connection fails the run. Roots outside their graph are counted (`dangling_roots`), not fatal.
- **Which tree uses which graph** (`PassiveSkillTrees`): PoE1 `Default`, `DefaultAltAscendancies` (custom-ascendancy graph), `Royale`, `Atlas` and `AtlasCurrentLeague` (both `AtlasSkillGraph`; `atlasskillgraphcurrentleague.psg` is named by no tree), `AtlasEmpty`, `BrequelTree` (Chayula graph); PoE2 `Default`, `Royale`, `Atlas`, `EndgameMap`, `BrequelTree`.
- **Output:** `extract` writes each graph as `passive_skill_graphs/<lowercase index path>.json` (header fields, one group per line) and lists every `.psg` in the manifest (format 3, `passive_skill_graphs`) with status, counts, and source hash. Errors: `EXTRACT_PSG_TRUNCATED`, `_VERSION`, `_NON_FINITE`, `_ORBIT`, `_TRAILING`, `_BAD_PATH`. `PassiveSkillTrees` is now a root table of `extract` (it and its references add 4 tables in PoE1, 1 in PoE2).

## 17. Evidence (PR A6b) and gate verdict
- **Passive rules.** PoE2's tree is made by PoB's `src/Export/Scripts/passivetree.lua`, which walks the groups of the graph of `PassiveSkillTrees` row `Default` and skips three kinds of node. The cross-check reproduces all of it, anchored to the script's lines: `empty-name`, `dnt-name` (except the 5 `[DNT]` names PoB keeps), `skipped-ascendancy` (ascendancy name `[DNT` or `Disabled`), and `not-on-default-graph` for rows off that graph (breakdown: the other graphs they are on, or none). The last rule is disabled if any PoB key is off the Default graph, which would falsify it; none is. PoE1's tree is GGG's own tree export, and no PoB script can be reproduced. It has 409 nodes on no graph (361 flagged `IsExpansion`; the others are cluster-jewel notables and masteries and special ascendancy nodes), so being off the graph does not predict PoB's choice. Our PoE1 rows off the character tree's graph therefore get a new category, **`not-on-tree`**: "not on the game tree PoB's data covers, by the game's own placement". The maintainer decided this on 2026-11-09 after the review, as an amendment to the plan's category list.
- **Review** (`code-architecture-reviewer`, 2026-11-09, on `1bb261b`). Three blockers, all applied:
  - The category split above.
  - `table-not-ok` now comes before every other rule, so a misread table cannot be explained by a filter. Passives also check `PassiveSkillTrees` and (PoE2) `Ascendancy`.
  - `cross-check` reads the graph statuses from the manifest and stops on a failed or missing graph (`EXTRACT_XCHECK_GRAPH`), so a broken graph cannot make its nodes look off-tree.
  - Should-fix items were applied too: rows without a unique graph ID stop the cross-check (`EXTRACT_XCHECK_PASSIVE_ROWS`; none occur), the falsification check, a `dangling_roots` count, and more tests. Details are in the context file.
- **CI tests** (`cargo test -p loom-extractor`: 147 library tests, plus 9 CLI tests). The graph parser is tested on:
  - both layouts, and the other game's widths failing;
  - truncation at every length, trailing bytes, the version, non-finite positions, orbits out of range, and huge counts;
  - counts of dangling connections and roots and of duplicates, kept connection orbits, the rendered JSON, a bad path, and a failed graph failing the run.

  The passive cross-check is tested on:
  - the PoE1 category with its breakdown and notes;
  - every PoE2 rule, rules disabled by a missing snippet, and the falsified graph rule;
  - `schema-mismatch` taking every gap of a bad table (stats and passives);
  - every error path: a missing graph file, a failed graph, a format-2 manifest, no Default graph or row, and unkeyable rows.
- **Maintainer workstation run, 2026-11-09, from clean tool commit `b4eebe0`.** The Steam builds are unchanged: PoE1 26530219, PoE2 26476662.
  - `check-determinism.sh` gave byte-identical output on both games: 128 files for PoE1 (76 tables, 45 stat-description files, 7 graphs), 681 for PoE2 (86 tables, 589, 5).
  - Two cross-check runs per game gave byte-identical reports.
  - Manifests and summaries were replaced in `docs/spikes/spike-a/`.
  - Mods, bases, skills, and stats gave the same results as in A6a (section 15).

| Passives | PoE1 (tree 3_29) | PoE2 (tree 0_5) |
|---|---|---|
| keys PoB / ours / matched | 3,396 / 5,706 / 3,396 | 4,914 / 9,731 / 4,914 |
| Default graph nodes / PoB keys on it | 2,987 / 2,987 | 5,152 / 4,914 |
| PoB keys off every graph | 409 (361 `IsExpansion`) | 0 |
| ours extra: `pob-filtered` empty-name / dnt-name / skipped-ascendancy | - | 120 / 105 / 13 |
| ours extra: off the Default graph (`not-on-tree` PoE1, `pob-filtered` PoE2) | 2,310 | 4,579 |
| of which: atlas graphs | 1,076 | 1,185 (atlas 574, endgame map 610, both 1) |
| of which: other graphs (custom ascendancy, royale, Chayula league) | 362 / 141 / 134 | - / 0 / 248 |
| of which: no graph | 597 (1 `IsExpansion`) | 3,146 (1 `IsExpansion`) |
| uncategorized / patch-delta | 0 / 0 | 0 / 0 |

- **Spike A gate verdict (2026-11-09): met.**
  1. **Determinism** holds on both games (above).
  2. **Coverage vs PoB:** uncategorized is 0 in both directions for every family on both games. Patch-delta is 2 PoE1 keys (mods 1, skills 1; limits 470 and 30) and 0 on PoE2.
  3. **No silent drops:** the manifest lists every schema mismatch (only PoE2 `ItemVisualIdentity`, not a root table), every stat-description finding (0 unparsed), and every graph with its status. The cross-check stops on anything it cannot place.
  4. **Recorded inputs:** the manifests carry the Steam build, the index and bundle hashes, the schema commit and hash, the extractor commit `b4eebe0`, and `dirty: false`.
  5. **Licenses:** `cargo deny` passes in CI.
- **For later spikes:** Spike B reads `stat_descriptions/**`. Phase 2 and the passive-tree work can read `passive_skill_graphs/**` (layout, orbits, connections) together with `PassiveSkillTrees`. Every categorized gap becomes a divergence-register entry once Spike C defines the register format.

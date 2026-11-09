# Spike A: Bundles2 game-data extraction (report)

Plan: `dev/active/spike-a-bundles2-extraction/`. Gate: `docs/plan/ACTION-PLAN.md`, Spike A. This report grows one section per PR (A1 to A6); the gate verdict is written at A6.

Status (2026-11-08, PR A2): decompressor decided (A1); bundle container and index read by `loom-extractor` on both installs, path encoding and path hash confirmed (A2). No gate result yet.

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

## 6. datc64 (to confirm in A3)
Community-documented layout, from `poe-tool-dev/dat-schema` (MIT, the schema we will vendor) and `poe-dat-viewer`: `u32` row count, fixed-width rows, an 8-byte `0xBB` boundary marker, then a variable data section that strings (UTF-16LE, null-terminated) and arrays (count + offset) point into, with offsets counted from the marker. A3 checks this against real tables and records the results here.

## 7. Evidence (PR A1)
- CI tests: `cargo test -p loom-extractor`. Unit tests cover identity, length checks, tool-output parsing, and spawn errors. `tests/external_tool.rs` runs `ExternalTool` against `fake-unoodle` (`src/bin/fake-unoodle.rs`, a test double, not a decoder) and covers success, reported and actual length mismatches, a missing length line, tool failure with stderr, a missing work dir, scratch cleanup, and 8 concurrent calls.
- Maintainer-run `#[ignore]` test `tools/extractor/tests/install_decompress.rs` (2026-11-08, `unoodle` 0.5.7): passed on both installs. PoE1 index 20,826,488 -> 33,449,776 bytes (128 chunks); PoE2 index 116,427,003 -> 151,240,657 bytes (577 chunks); the first and last chunks decoded on their own match the whole-payload decode; high/low bundles decode to 8 zero bytes in both.

## 8. Evidence (PR A2)
- CI tests: `cargo test -p loom-extractor`. Synthetic bundles (stored uncompressed, read with `Identity`) and synthetic indexes cover parsing, path rebuilding with shared prefixes, listing with a path that has no record, a record that has no path, a directory record that matches no directory, and a path that matches two records; and each error: inconsistent header fields, trailing bytes, undefined prefix (also after re-entering the prefix phase, which clears prefixes), a path over 4096 bytes, a block ending inside a fragment, non-UTF-8 names and paths, a file record naming a missing bundle or running past its bundle (including `offset + size` above `u32::MAX`), and a path block outside the path data. Corrupt-input tests are exhaustive rather than random: every truncation of a synthetic bundle and index is an error, and every single-byte change (6 values per byte) parses and lists without a panic. Huge counts are rejected before allocating. Path-hash vectors are 12 path/hash pairs read from the PoE1 index (directory records and file records), covering tail lengths 0 to 7 and multi-block input.
- Maintainer-run `#[ignore]` test `tools/extractor/tests/install_index.rs` (2026-11-08, `unoodle` 0.5.7, release build): passed on both installs; it checks that every path and every record is accounted for exactly once, that no directory record is unmatched, and that the stats table resolves. `tests/install_decompress.rs` now reads headers through `Bundle::parse` and still passes on both.
- `loom-extractor list-files <install>` (release build, maintainer workstation): PoE1 in about 1.5 s, PoE2 in about 5.8 s, index hashes as in the table above (unchanged since A1). Counts as in section 4.

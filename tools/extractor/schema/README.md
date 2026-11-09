# Vendored community dat schema

`schema.min.json` is the community schema for the game's `.datc64` tables, from
[`poe-tool-dev/dat-schema`](https://github.com/poe-tool-dev/dat-schema). It is MIT
licensed; the license is in `LICENSE-dat-schema.txt` and applies to that file only.
Vendoring it was approved by the maintainer on 2026-11-08 (Spike A, PR A3).

| Field | Value |
| --- | --- |
| Source | release `latest`, asset `schema.min.json` |
| Source commit | `717d788d6ec081821c33473b2a0ec35629144826` (2026-11-05, "update Expedition2VerisiumCrafts (#252)") |
| Built by | upstream workflow run `Publish` 42056729237 on that commit |
| Schema `version` / `createdAt` | 7 / 1793893019 (2026-11-05 15:36:59 UTC) |
| Size | 2,494,228 bytes |
| SHA-256 | `e8e7e4c858924655bcfd56c23b275101f0321f435f90b19f8267bf43344ea16a` |

Upstream has no version tags: `latest` is a rolling release whose assets are
rebuilt on every commit. The source commit and the hash above pin the copy.

`loom-extractor` embeds the file at compile time (`src/schema.rs`, `VENDORED`), and
a unit test checks the SHA-256 above, so a changed file fails CI until this table
and the constant are updated together.

## Updating
1. Download `schema.min.json` from the `latest` release and note the commit of the
   `Publish` run that uploaded it.
2. Replace the file, update this table and `VENDORED_SHA256` in `src/schema.rs`.
3. Run the install check (`tests/install_dat.rs`) on both games and record the
   row-size results in `docs/spikes/spike-a-extraction.md`.

# B-2 data run: data schema 4 bundle (2026-11-14)

The maintainer's run after PR #109 (B-2's data part, PR 1), on the main workstation. It is the "Maintainer action" in `dev/active/ux-improvements/b2-item-data-plan.md`, PR 1, item 5.

## Inputs
- **Game data:** Steam build **26530219**, the build every earlier bundle came from. The install had patched to 26664355 (league launch, 2026-11-13), and the extractor does not read that build: 13 tables changed layout, and the passive tree files changed format. PoB has no release for the new league, so v2.67.2 stays the oracle.
- **The old build's files:** only the 40 files the extractor reads (the index and 39 bundle files, 114 MB), downloaded with DepotDownloader. Depot 238961, manifest `2467965126630699088` (the manifest of build 26530219 on SteamDB); the file list is the bundles the extracts of builds 26530219 and 26664355 read.
- **Build number:** the extractor reads it from Steam's `appmanifest_238960.acf`, which a depot download has none of. A stand-in was written beside the download (`steamapps/appmanifest_238960.acf`, with `appid` 238960, `name` Path of Exile, `buildid` 26530219, `installdir` Path of Exile; the extractor reads `installdir` and `buildid`). So `steam_build` in the bundle and every summary is **asserted, not read from Steam**. The files are the build's: DepotDownloader checks each against manifest `2467965126630699088`, which SteamDB lists as depot 238961's manifest in the update that made build 26530219. Nothing in the bundle yet marks this provenance; a source field for it belongs in the bundle update process (follow-up).
- **Bundle:** `loom-parity-cli bundle`, twice, byte-identical. Manifest SHA-256 `32d3d74a9deac8c8d3b279d7d578145f4503b6e5e3c414f32ad6906eaf2efff2`, data schema 4, 126 MB. `bundle-check` reports no findings. Against the schema 3 bundle (P3-8n's), only `bundle.json`, `data/mods.json`, and `data/base_items.json` differ, and `data/tags.json` is new.
- **PoB export:** format 6, two runs byte-identical.

## Records
| File | What |
|---|---|
| `data-parity-poe1-26530219.json` | `data-parity` summary: the new tables `mod_affixes` and `base_requirements` (`docs/phase-3/data-parity.md`, "B-2") |
| `b2-public-calc-run-summary.json`, `b2-corpus-poe1-26530219-calc-run-summary.json` | `calc-run` on the public fixtures (136 builds) and the anonymous corpus (21) |
| `b2-public-calc-parity-summary.json`, `b2-corpus-poe1-26530219-calc-parity-summary.json` | `calc-parity` against the oracle |
| `b2-public-vs-close.json`, `b2-public-overlay-vs-close.json`, `b2-corpus-poe1-26530219-vs-close.json` | Each calc document compared with the Phase 3 close-out's (`docs/phase-3/phase-3-report/`) |
| `rules-rebuild.md`, `rules-public-calc-run-summary.json` | The bundle rebuilt for B-2 PR 2's item rules (`affix_generations`, `mod_tag_names`), 2026-11-15; manifest `4cdd0bdb…` |
| `card-data.md` | The item card's game data, 2026-11-15: mod tag names from `Tags.Name` (Q-B2-2, bundle rebuilt), socket caps in the item type files, unique flavour text by name convention (Q-B2-1), shipped as `poe1/unique_flavour.json` |
| `data-parity-poe1-26530219-flavour.json` | `data-parity` summary of the flavour text bundle (PoB export format 8, table `flavour_text`) |
| `item-evidence.md`, `game-copies/` | The maintainer's in-game item copies, 2026-11-15, and what they settle: the game copy reader, PoB's reading of the same copies, the card's values, requirements, affix headers, socket caps (`poe1/socket_caps.json`), the tree flavour (`poe1/tree_flavour.json`), and what the next data run must do |

## Result
**No calculation changed.**
- Every calc document of the public, overlay, and corpus runs is identical to the close-out's.
- `calc-parity` counts are equal: 0 unexpected differences; 2,078 of 2,078 mechanic targets met over 99 fixtures.
- Runs are byte-identical twice.
- The calc-run summaries differ from the close-out's only in `bundle.data_schema` (3 to 4), the bundle hash, and the tool commit.
- The tests that need a real bundle (`loom-calc`, `loom-data`, `loom-parser`, `--ignored`) pass.

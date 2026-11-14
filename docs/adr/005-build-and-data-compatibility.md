# ADR-005: Build, Schema, and Game-Data Compatibility

- Status: Accepted (revised 2026-11-08; Amendment 1, 2026-11-08; Amendment 2, proposed 2026-11-14, accepted on merge)
- Date: 2026-02-01

## Decision
Use a versioned successor JSON document as the canonical build format. Legacy XML and share codes remain first-class external adapters. Successor JSON uses versioned Rust persistence DTOs as the source of truth; JSON Schema and TypeScript types are generated. Sequential migrations are maintained in a shared migration library.

Known content maps to typed domain models. Unknown content is preserved opaquely and unsupported mechanics are quarantined visibly. Legacy export uses semantic round-trip compatibility. Item modifiers are modelled on stat IDs (ADR-015).

## Game data source (revised)
- Game data is produced by the project's **own Rust extractor**. It reads the local game install (Bundles2 index, `.dat` tables, stat-description files) using the community dat schema, and produces canonical bundles for PoE1 and PoE2.
- Path of Building's exported data is a **cross-check only**, not an input. RePOE is the documented fallback input if extraction is blocked.
- Extraction is a maintainer-run tool, not an application feature. CI cannot legally download the game, so a maintainer runs the extractor against a recorded game version and bundle-index hash. CI then validates, reproduces from recorded inputs where possible, and signs.
- **End users receive only shipped, signed bundles.** The application does not extract from the user's install.
- Bundles are hybrid: readable manifest and canonical structured data, compressed for distribution, with optional binary indexes. They contain derived data only, no GGG art or audio.

## Additional import formats
The PoE2 in-game Build Planner `.build` JSON is a backlog import target.

## Consequences
- Patch-day data turnaround depends on maintainer availability.
- The extractor must record game version, schema revision, and input hashes for reproducibility.
- Data schema and application schema version independently.

## Amendment 1 (2026-11-08): Oodle decompression through an external tool
Approved by the maintainer by merging the PR that adds it (Spike A, PR A1, #6). Evidence: `docs/spikes/spike-a-extraction.md`, sections 1 and 2.

### Context
Bundles2 bundles are Oodle-compressed (Kraken/Leviathan). The Spike A survey found that every open-source Oodle decoder derives from powzix/ooz, which is GPL-3.0-or-later. The only one labeled permissive, the `oozextract` crate (MIT on crates.io), is a self-described Rust port of ooz with no license file in its repository. The official Oodle SDK is proprietary, and neither game install ships it as a separate library. The project is MIT and accepts only permissive dependencies (`deny.toml`), so no decoder can be a dependency. The RePOE fallback does not avoid the problem: RePOE decodes with PyPoE, which also uses ooz.

### Decision
- The extractor does not link, vendor, or depend on any Oodle decoder. Decoding is delegated to a **separately installed command-line tool** that the maintainer runs as a child process, behind the extractor's `Decompressor` trait. Everything else stays the project's own Rust code: bundle container, index, path hashing, `.dat` tables, stat descriptions, manifest.
- The tool is a maintainer workstation tool, like a compiler. The project does not distribute it, and it never reaches end users, who still receive only signed bundles.
- The tool and its exact version are pinned in the extractor documentation (currently `unoodle` from `oozextract` 0.5.7: `cargo install --locked oozextract@0.5.7 --features cli`). The extraction manifest records the decoder program and version with the other inputs.
- The extractor checks each decode against the decompressed length recorded in the bundle, so a wrong or truncated decode fails instead of producing data.
- CI does not run the decoder. Code around it is tested on synthetic bytes and with a test-double tool; real decoding is checked by maintainer-run install-backed tests.
- Replacing the tool (another ooz build, or a permissive clean-room decoder if one appears) requires only a change to the `Decompressor` implementation and the pinned version, not a further amendment. Linking any decoder into the project, or distributing one, does require a new amendment.

### Consequences
- The maintainer needs one more local install, and the extractor depends on that tool's command-line interface.
- CI can re-check outputs from decompressed or extracted inputs, but cannot reproduce decompression itself; reproducibility rests on the recorded index hash, decoder version, and output hashes.
- The repository and its dependency tree stay free of GPL-derived code, so the project license and the `cargo-deny` policy are unaffected.
- The RePOE fallback stays in place for the case where extraction is blocked for other reasons.

## Amendment 2 (2026-11-14): GGG's passive tree export as a pinned input
Decided by the maintainer on 2026-11-14 in the passive tree and Phase 5 session (`dev/active/passive-tree-and-phase-5/`, questions MQ1 and MQ8, after `plan-reviewer` finding B1); proposed in that PR and accepted when the maintainer merges it.

### Context
The passive tree editor (ADR-008 Amendment 1) needs three things the game files read by our extractor do not give: the orbit radii and the angle tables that place nodes, the trees of older versions that builds still use (the corpus has builds on 3_27 and 3_28), and the tree's art. GGG publishes the PoE1 passive tree for tool developers (https://www.pathofexile.com/developer/docs/data, the export repository `grindinggear/skilltree-export`: `data.json` per release, sprite sheets in `assets/`). ADR-000 allows GGG assets "where GGG publishes them for tool developers"; this ADR said game data comes only from our extractor and bundles hold no GGG art.

### Decision
- **Pinned input.** GGG's passive tree export is an allowed input for (a) the layout constants (orbit radii, slots per orbit, and the angle tables the export documents), (b) tree packs of older tree versions, converted with node text resolved to stat IDs (ADR-015; unresolved lines quarantined), and (c) the tree's sprite sheets. It is fetched by a maintainer-run script (`tools/tree/`) at commits pinned in a committed manifest, never by application code.
- **Provenance.** A bundle records the export's repository, commit, and the SHA-256 of every file it read, per tree pack and for the layout constants; the bundle writer refuses a checkout that does not match the pin.
- **Sprites in bundles.** A bundle may carry the export's sprite sheets for its tree versions, unmodified, with their provenance; they are never committed to this repository and ship only inside signed bundles. The app shows GGG's third-party notice ("This product isn't affiliated with or endorsed by Grinding Gear Games in any way."). Other GGG art and audio stay excluded.
- **Unchanged.** The game's own tree (positions from its graph files, stats as stat IDs) stays the source for the version of the game install; PoB stays a cross-check only.

### Consequences
- The data schema gains provenance blocks and tree pack files; `bundle-check` validates them.
- If GGG withdraws the export or states terms that do not permit this use, the sprite sheets leave the bundles and the renderer's own shapes (always present as the fallback) take over; the data inputs are re-examined.
- The maintainer accepted the remaining risk that the export states no licence (session 2026-11-14).

# ADR-005: Build, Schema, and Game-Data Compatibility

- Status: Accepted (revised 2026-11-08)
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

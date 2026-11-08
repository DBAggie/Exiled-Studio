# ADR-009: Persistence, Undo, and Recovery

- Status: Accepted
- Date: 2026-02-01

## Decision
Use file-first persistence with SQLite indexes/caches. Versioned JSON build files are authoritative and portable; SQLite stores search metadata, workspace state, indexes, and caches. Autosave and recovery use versioned snapshots.

Use domain command history plus periodic snapshots. Imports and migrations are atomic commands. Successor JSON has long-term read support through explicit sequential migrations and an optional standalone converter. Original files are not silently overwritten.

## Consequences
- Index loss cannot lose builds.
- Undo/redo is consistent across panels.
- Crash recovery and branching are supported without full event sourcing.
- Persistence DTOs and domain models remain separate.

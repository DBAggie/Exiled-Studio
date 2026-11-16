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

## Amendment 1 (2026-11-16): a build keeps its own dashboards and snapshots (accepted)
Raised by the UI and UX audit (`docs/ui/ux-audit.md`, "Left for later": dashboards per build, and pinned numbers that survive a restart). **The maintainer decided on 2026-11-16 that both are kept in the build file**; implemented with the audit's follow-up (PR #128).

### Context
This ADR puts "workspace state" in SQLite, beside the build files. Two pieces of what a user arranges belong to one build, not to the workspace: a snapshot of the build's numbers kept by name ("before the new helmet"), and a set of dashboards made for that build. Kept outside the file, they would be lost when the build is moved, shared, or restored from a recovery copy.

### Decision
- The build document (v1, additive) gains one optional field, `workspace`: the UI's document for that build. The core does not read it; it is text, as the application's `ui_prefs` is. The UI owns its format and versions it (`ui/src/workspace.ts`).
- It is changed by one command (`set-workspace`), so a change is an undo step and is saved and recovered with the build like any other. It is limited to 262,144 bytes (`APP_WORKSPACE`).
- A document the UI does not read is treated as empty, never an error, and never stops a build from opening.
- PoB's format has no place for it: an export to PoB leaves it out.
- Unchanged: the user's own dashboards and other preferences stay outside builds (`ui_prefs`); indexes and caches stay in SQLite.

### Consequences
- A build file carries its snapshots and, when the user asks, its dashboards; another application version may drop the parts it does not know when it next writes the document.
- A change to them marks the build unsaved and starts a calculation with the same result (the version changed).
- The field is not part of what a calculation or the parity harness reads.

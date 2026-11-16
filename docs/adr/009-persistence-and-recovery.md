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

## Amendment 1 (2026-11-16): a build keeps its own dashboards (accepted)
Raised by the UI and UX audit (`docs/ui/ux-audit.md`, "Follow-up"). Decided by the maintainer on 2026-11-16, in these words: dashboards per build, "yes, in the build file"; "we should update the opaque text field so the AI can see it"; "the numbers on a dashboard should be derived from the combination of stats on gear, passive / ascendancy point allocation, and buffs granted by skills or minions. The numbers shouldn't be static"; "the core components and how they are laid out on a dashboard matters most so it looks the same for a user receiving a build file from someone else"; and "the build file should only contain the active variants of the build and not snapshots of undo history". Implemented with the audit's follow-up (PR #128). The rest of this amendment is the session's wording of that decision.

### Context
This ADR puts "workspace state" in SQLite, beside the build files. One piece of what a user arranges belongs to a build, not to the workspace: the dashboards made for that build. An energy shield build has no use for an armour panel, and a physical attack build none for spell damage; whoever receives the file should see the build as its author laid it out. Kept outside the file, that layout is lost when the build is moved or shared.

### Decision
- The build document (v1, additive) gains one optional typed section, `dashboards`: the set of dashboards the build shows, each a named list of components (its kind, its width, and the stats or input it is about), and which one is shown first.
- **Layout only.** The section holds no calculated number. Every number a component shows is calculated from the build as it is (items, passive and ascendancy allocation, skills, config) each time.
- The core types and checks the section (`set-dashboards`, limits, `APP_DASHBOARDS`) and gives it to AI clients in MCP `get_build`: it tells a client which numbers matter for this build. A component's kind and width are text, so the UI can add a kind without a schema change and a file from a newer version still opens; a version shows the components it knows.
- It is changed by one command, so a change is an undo step and is saved and recovered with the build like any other.
- PoB's format has no place for it: an export to PoB leaves it out.
- **Not in the build file:** snapshots of the build's numbers kept to compare with (the audit's "Snapshots"), the session's pinned numbers, and undo history. A build file holds the build's active variants, not a record of what it was. Kept snapshots are the user's own, on that computer (the UI's local storage, under the build's file; `ui/src/kept.ts`).
- Unchanged: the user's own dashboards and other preferences stay outside builds (`ui_prefs`); indexes and caches stay in SQLite.

### Consequences
- A shared build file opens with its author's dashboards; the receiver can switch to their own ("Every build") or change the build's.
- A change to a build's dashboards marks the build unsaved and starts a calculation with the same result (the version changed).
- Kept snapshots do not travel with a build file or to another computer, and are not seen by AI clients.
- The section is not part of what a calculation or the parity harness reads.

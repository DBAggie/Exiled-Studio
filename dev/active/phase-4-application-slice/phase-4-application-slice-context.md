Last Updated: 2026-11-13

# Phase 4: application slice and MCP addon (context)

## Key files
- Plan: `dev/active/phase-4-application-slice/phase-4-application-slice-plan.md`.
- Pipeline being moved into the app: `tools/parity-cli/src/calc_run.rs` (`one`: import, `resolve_build_ranged`, item bases, line values, `CalcOptions`, `calculate`).
- Persistence: `crates/loom-persistence/src/lib.rs` (`Store`, `FileOps`).
- Item reader: `crates/loom-serialization/src/legacy/pob_item/` (`read`, `domain::to_domain`, `item_base`, `line_values`).
- XML and share codes: `crates/loom-serialization/src/legacy/pob_xml/mod.rs` (`import`, `export`, `xml_equal`), `legacy/share_code.rs`.
- Resolver: `crates/loom-parser/src/import/build.rs` (`Resolver::new`, `resolve_build_ranged`, `resolve_item_ranged`).
- Engine: `crates/loom-calc/src/engine.rs` (`calculate_cancellable`).
- Schema generation: `tools/schema-gen/src/lib.rs`; UI types `ui/src/generated/`.
- UI reference: `docs/ui/UI-DESIGN-BRIEF.md`, `docs/ui/exiled-studio-ui-draft-v1.zip`.
- Synthetic inputs for tests: `crates/loom-data/tests/fixtures/bundle-poe1`, `tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml`.

## ADRs
002 (shell, generated bindings), 003 (snapshots, background jobs, stale results), 004 (command/query/event, single writer, Redux Toolkit, state machines), 008 (UI, WCAG 2.2 AA, localization-ready), 009 (file-first, autosave, undo), 010 (addons API-first, permissions), 011 (no telemetry, network only in integrations), 013 (first usable milestone), 014 (stable codes), 015 (warnings shown, never dropped), 016 (core wasm), 017 (MCP reference addon).

## Decisions and reasons
- One PR for the phase: maintainer request 2026-11-13.
- App API DTOs live in `loom-serialization` (core) because `loom-schema-gen` is a tool and tools depend on core crates only (AGENTS.md).
- The MCP transport is localhost HTTP, not stdio: the app is a GUI process, so a stdio server would need a second bridge process; ADR-017 allows either and asks for loopback binding and a per-session token on localhost.
- The SQLite index is not needed for the slice's recent list; ADR-009 keeps JSON files authoritative, and the index comes with search.

## Open questions (maintainer)
- `axe-core` (MPL-2.0, test-only): approve the exception or not (report section 6).
- Gate item 1b: CI `e2e` result, else the real-app checklist (report section 5).
- UI brief open questions (Skills layout, dashboards, stages) stay open; nothing in this slice depends on them.

## plan-reviewer findings (2026-11-13; verdict "proceed with changes", no blockers)
All 18 applied in the plan; where each landed:
1. Pipeline would exist twice: new core crate `loom-pipeline`, `calc-run` calls it (decision 0; gate 4 byte-identical check).
2. Reopen must not need data: open keeps stored resolutions, a clone recovers range ends, stable codes otherwise (decision 7). The alternative of storing range ends in build v1 was not taken: it changes the document, which needs the maintainer.
3. Undo must restore range ends: `Arc<State>` with build and ends (decisions 2, 4).
4. Stale requests and edits: base version and expected line texts, `APP_STALE_EDIT` (decisions 6, 11; gate 3).
5. MCP security: Host check, constant time, caps, token lifetime, session-only global grant, no paths, no new HTTP crate (decision 11; gate 3).
6. Out-of-scope build in the gate (decision 14).
7. Real app e2e: `tauri-driver` CI job, with a recorded maintainer smoke as fallback (decision 14 d; gate 1b).
8. Accessibility: error states, `aria-live`, manual checklist for zoom, target size, screen reader (decision 13).
9. Contrast: committed pair table plus a tokens-only CSS test (decision 13).
10. Command names: committed list compared by a Rust test (decision 9).
11. Paths: no path in webview commands, issued IDs, capability test (decisions 3, 10).
12. Recovery: versioned file, per-instance lock, restore through open, gate test (decision 8; gate 1).
13. Item text through the export's writer; `SetRange` for ranged lines (decision 6).
14. History entries with origin and request ID (decision 4).
15. Preview off the writer thread (decision 6).
16. Fields from the calc document; tree table keyed by game (decisions 1, 6).
17. `axe-core` MPL-2.0: test-only, listed as an exception awaiting the maintainer; pnpm license check (gate 5).
18. MCP results are app API v1 DTOs (decision 11).

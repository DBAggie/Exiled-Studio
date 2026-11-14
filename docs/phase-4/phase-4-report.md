# Phase 4 report: application slice and MCP addon

Phase 4 of `docs/plan/ACTION-PLAN.md` ("Application slice + MCP addon"), planned in `dev/active/phase-4-application-slice/` (plan, context with both reviews, tasks). Plan, implementation, and this report are one PR, #105, at the maintainer's request (2026-11-13: "Complete phase 4 in the fewest number of PRs possible").

**Verdict.** Gate items 1, 2, 3, 4, and 5 hold on Windows locally (section 3) and are run by CI on Windows and Linux. Item 1b (the real application under WebDriver) is a new CI job, `e2e`, first run on this PR; its result is recorded in the handoff with the PR's CI run. If it cannot be made to run, the plan's fallback applies: the maintainer's smoke run (section 5) is recorded here as the gate item. The `axe-core` license exception (MPL-2.0, test-only) was approved by the maintainer 2026-11-13 (section 6).

## 1. What was built
| Part | Where | What |
|---|---|---|
| One pipeline | `crates/loom-pipeline` (new core crate) | Import (PoB XML, share code, build document), resolution with range ends, PoB's settled values, the engine with a cancel hook, the calc result document. `calc-run` and the application both call it (decision 0). |
| App API v1 | `loom_serialization::dto::app::v1`, `schemas/app-api.v1.schema.json`, `ui/src/generated/app-api.v1.ts` | Projections (snapshot, build, items, lines, warnings, history, calculation status, data), commands, outcomes, events, errors, previews, MCP status, recent and recovery entries. No command takes a path. |
| Persistence | `loom_persistence::{recovery, recent}` | Recovery snapshots (versioned file, per-instance lock, listed only when their instance ended), the recent list (a cache, IDs from the path), `Store::write_replacing`. |
| Application | `crates/loom-app` | `Session` (state with range ends, snapshot undo with history entries, item edits with stale checks, open without data, save, export, share code); `App` (single writer thread, calculation worker with cancellation and stale-result discard, autosave, change requests bound to their build). |
| MCP server | `loom_integrations::mcp` | Streamable HTTP request-response subset on 127.0.0.1, per-start bearer token, Host and Origin checks, a deadline per request, at most 4 connections, tool calls one at a time; tool contract v1 in `schemas/mcp-tools.v1.json` (9 tools; results are app API v1 DTOs). |
| Shell | `crates/loom-tauri` | 16 commands (`ui/src/api/commands.json`), one event channel, file dialogs opened from Rust only (`FilePicker`), capability without `dialog:*`/`fs:*`, MCP start and stop, stop on exit, an `e2e` feature with a fixed-file picker. |
| UI | `ui/src` | Start (import, open, recent, recovery, game data), import result, dashboard (fields from the calc document with states and reasons), breakdown (provenance tree), equipment and item editor (line statuses, edit, remove, add, preview, apply), warnings, AI connection (server, token, permissions, approvals), about. Redux Toolkit projections, an import state machine, design tokens, one message catalog, Radix menu and dialog. |

## 2. Decisions taken in the PR
The plan's decisions 0 to 14 stand, with the two reviews applied (context file). What changed during implementation:
- **Requests are bound to their build** (code review 1): a request records the build generation its preview was made against; opening, importing, restoring, or closing fails pending requests as stale (`APP_STALE_EDIT`), and so does approving one for another build. Within one build, the edit's expected line texts guard it; the preview shown is the one made when the request arrived.
- **Edits stay modifier edits** (code review 11): an edited or added line must read as a modifier or unrecognized line; at most 50 operations and 500 characters per line (`APP_EDIT_LINE`).
- **A save changes no version** (code review 7): no recalculation follows a save.
- **The standing grant lives only while the server runs** (code review 6): it cannot be set while the server is stopped, and a start or stop clears it.
- **No file paths reach MCP clients**: bundle load errors carry codes only (code review 12); projections given to tools have no path fields.
- **SQLite index**: not built; the recent list is a JSON cache (plan decision 8). It waits for search (IMPLEMENTATION-PLAN Phase 6 WP5).

## 3. Gate
| Item | Check | Result (Windows, local, 2026-11-13) |
|---|---|---|
| 1 | `cargo test --workspace`: `loom-app` workflow (`crates/loom-app/tests/workflow.rs`: import equals `calc-run`'s golden document; edit, preview, stale edit; undo and redo byte-equal; save, reopen byte-equal; save in place; recent by ID; export XML re-reads; share code; out-of-scope fields with reasons; origin never overwritten; reopen without data; recovery restore; no recovery after save; rapid changes end on the latest version; requests and grants; requests fail on another build; structural lines refused; save does not recalculate; roll edit keeps its range through export) and `loom-tauri` IPC workflow (`crates/loom-tauri/tests/ipc_workflow.rs`) | pass; CI `rust` runs both on Windows and Linux |
| 1b | CI `e2e`: the built app under `tauri-driver` (`ui/e2e/workflow.mjs`): import, dashboard, edit and apply, recalculation, undo, save, export | **Linux (WebKitGTK, Ubuntu 24.04): pass** (CI run 43540606102: start screen, import, edit and apply, undo, save, export). **Windows (WebView2 156): fails** at session creation (`DevToolsActivePort file doesn't exist` from msedgedriver; the same app starts and runs locally). The same workflow passes on Windows under WebView2 when run from PowerShell (maintainer, 2026-11-13, at `205560b`: every step, "e2e workflow passed"); CI's Windows step now runs `ui/e2e/run.ps1` instead of the Git Bash script |
| 2 | `pnpm test`: UI workflow, axe on every screen (start, import result, dashboard, breakdown, equipment, item editor with preview, warnings, AI connection, about, open menu), keyboard, `aria-live` announcement, error states (no data, import failed, calculation failed, stale edit), token contrast and tokens-only CSS, client command list | 14 tests pass; CI `frontend` runs them on Windows and Linux |
| 3 | MCP contract (`crates/loom-integrations/tests/mcp_contract.rs`): tool list equals the committed contract; no token, wrong token, wrong-length token, foreign Origin, foreign Host refused; protocol version, GET, path, size, chunked, malformed, batch refused; notification 202; a stopped server's token is dead; read tools; read-only default; approval as one undoable step; stale request; a slow client does not hold the server; refusal before the body is read. With `loom-app`'s: decline, standing grant and its revocation, the 17th pending request refused | pass |
| 4 | Schema freshness covers `app-api.v1`; clippy `-D warnings`, wasm32 (with `loom-pipeline`), `cargo deny`, fmt | pass |
| 4 | `calc-run` byte-identical before and after decision 0: the public fixtures (`tools/oracle/fixtures`, `mechanic`, `community`, PoB's 3.13 builds; 136 builds) with the P3-8n bundle and the scope file | 135 documents and `report.json` identical (malformed.xml fails at read, as before) |
| 5 | `pnpm licenses:check` | 191 packages; exceptions named in `ui/scripts/check-licenses.mjs` |

## 4. Parity
No calculation changed: `calc-run`'s documents are byte-identical (item 4), and the application's calculation of the synthetic build equals `calc-run`'s golden document (`crates/loom-pipeline/tests/testdata/synthetic-poe1.calc.json`). No divergence-register entry.

## 5. Real-application checklist (maintainer; and the fallback for item 1b)
Run `pnpm build` in `ui/`, then `cargo run -p loom-tauri` with `LOOM_BUNDLE_DIR` set to a bundle (for example `local/p3/p3-8n/bundle-poe1-a`), on Windows and on Linux. Record commit, system, and pass or fail per step:
1. Import a PoB share code; the import result shows counts and warnings.
2. The dashboard shows fields with states; a field without a value says why.
3. A field's breakdown opens; Back returns.
4. Edit an item line, preview, apply; "Recalculating" then "Calculation up to date"; Undo and Redo.
5. Save, close, open from the recent list; the same numbers.
6. Export PoB XML and import it in PoB v2.67.2.
7. Start the MCP server; `claude mcp add --transport http exiled-studio <url> --header "Authorization: Bearer <token>"`; ask the client to read the build; allow requests; approve one; undo it.
8. Quit with unsaved changes; start again; restore.
9. Accessibility: zoom to 200 % (no lost content or function), target sizes, a screen-reader pass (NVDA on Windows, Orca on Linux) over the start screen, dashboard, and item editor.

## 6. Open for the maintainer
- **`axe-core` (MPL-2.0)**: a test-only dependency of `ui/` (accessibility checks), never shipped; the exception was approved by the maintainer 2026-11-13 and is listed in `ui/scripts/check-licenses.mjs`. `lightningcss` (Vite's CSS tool, MPL-2.0) was already in the tree.
- **UI brief questions** (`docs/ui/UI-DESIGN-BRIEF.md`, section 7) stay open; this slice depends on none of them.

## 7. Known limitations (later phases)
Stages, dashboard customization, skills and passive tree screens, configuration editing, new builds, character import, PoE2 build export, SQLite index, structured logs and diagnostic export (Phase 5 WP7), installers, signing, updates (Phase 5), MCP resources and prompts.

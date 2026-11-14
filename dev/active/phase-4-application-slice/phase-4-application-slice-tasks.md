Last Updated: 2026-11-13

# Phase 4: application slice and MCP addon (tasks)

One PR (maintainer request 2026-11-13). Tick a task in the commit that does it.

- [x] Plan, context, tasks
- [x] plan-reviewer run; findings applied (context file)
- [x] `loom-pipeline` (decision 0); `calc-run` byte-identical
- [x] App API DTOs v1 (`loom_serialization::dto::app::v1`), schema, TS types
- [x] `loom-app` session: commands, history, item edits, preview, reopen
- [x] `loom-app` jobs and events: worker, cancel, stale discard
- [x] `loom-persistence`: recovery snapshots, recent index
- [x] `loom-integrations`: MCP server, tools contract v1, permissions, approvals, contract tests
- [x] `loom-tauri`: commands, events, dialogs, capabilities, IPC workflow test
- [x] UI: store, import state machine, screens, fake API, tests, axe, keyboard, contrast
- [x] CI: `pnpm test` and `pnpm licenses:check` in `frontend`; `e2e` job
- [x] code-architecture-reviewer run; findings applied (report section 2)
- [x] Report `docs/phase-4/phase-4-report.md`; handoff, CLAUDE.md, ACTION-PLAN, AGENTS.md, README
- [x] Gate 1b: CI `e2e` passes on Linux; Windows by the maintainer's run (2026-11-13); the Windows CI job runs without gating
- [x] Maintainer: `axe-core` license exception (approved by the maintainer 2026-11-13)
- [x] Open PR: #105 (base `dev`)

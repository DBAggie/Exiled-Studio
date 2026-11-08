# Exiled Studio (`loom`): New-Session Handoff

Independent successor (working name **Exiled Studio**, repo `DBAggie/Exiled-Studio`, public): the current Path of Building and this successor are maintained by different open-source groups. Do not assume control over the other project. Crates use the codename `loom`; the name gets a final trademark check before the first release (ADR-000). License: MIT.

Read in order: `README.md`, this handoff, all ADRs (000–017), `docs/plan/ACTION-PLAN.md` (authoritative sequence), then `docs/plan/IMPLEMENTATION-PLAN.md` for supporting detail.

## Key decisions (revised 2026-11-08)
- Full Rust rewrite; Tauri 2 + React/TypeScript/Vite; in-process core; core crates compile to wasm32.
- The game is the truth. PoB v2.67.2 is the pinned regression oracle. Every difference is explained; accepted ones go in the divergence register with evidence.
- Modifiers are stat-ID first; text is an import adapter; unresolved lines are quarantined and shown.
- Game data comes from the project's own Rust extractor reading Bundles2, maintainer-run. Users receive shipped signed bundles only.
- Addons are API-first and sandboxed. The local MCP server is the first addon and the Claude integration.
- PoE1 first; PoE2 data in the pipeline from Phase 2; PoE2 calcs after the Phase 4 gate.
- Windows + Linux gated; macOS best-effort.
- Team: maintainer + AI agents. Maintainer + CI review until first release. Small PRs with evidence are mandatory.

## Local resources
- Legacy repo: `D:\POE Dev\PathOfBuilding` (read-only reference; do not modify).
- Game installs: `D:\SteamLibrary\steamapps\common\Path of Exile` and `...\Path of Exile 2`.
- Maintainer PoB builds (corpus seed, scrub before committing): `D:\Documents\Path of Building\Builds`.
- PoE2 Build Planner files: `D:\Documents\My Games\Path of Exile 2\BuildPlanner`.

## Workstation state (checked 2026-11-08)
- Present: git 2.55, gh 2.102 (logged in as `DBAggie`), git identity configured, WebView2, winget, WSL 1 (Ubuntu).
- Missing or outdated: Rust (none), Node (14.15.3; need 20+), pnpm (none), MSVC (VS 2017 only; need VS 2022 Build Tools + Windows 11 SDK), LuaJIT (needed only at Spike C).

## Next action: bootstrap session
The maintainer has decided the bootstrap session starts Phase 0. Confirm authorization at the start of the session, then:
1. Install the missing toolchain via winget, asking approval for each install (Node upgrade first warns about other projects using Node 14).
2. Phase 0 per `docs/plan/ACTION-PLAN.md`: git init in this folder, public repo `DBAggie/Exiled-Studio`, workspace, CI, AGENTS.md/CLAUDE.md, PR template.
3. Use `/dev-docs` to plan Spike A and review it with the `plan-reviewer` agent before implementing.

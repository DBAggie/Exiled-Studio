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

## How sessions work
Each PR is done in its own fresh session; the repository is the only memory. Every PR updates this file's **Current state** and **Next action**, plus any other document it made stale (list in `AGENTS.md`, "One PR per session"). If something here disagrees with the code or CI, trust the code and CI, and fix this file in your PR.

## Current state (updated 2026-11-08, PR-evidence workflow PR)
- **Phase 0 (bootstrap): done.** Gate passed: CI run 42526884777 green on Linux and Windows, including the wasm32 build of the core crates (<https://github.com/DBAggie/Exiled-Studio/actions/runs/42526884777>). `main` is protected, admins included: PR required (0 approvals, since the maintainer cannot approve PRs opened under their own account), required status checks `ci-ok` and `PR evidence` (strict: branch up to date), linear history, no force pushes. Merges are squash-only and branches auto-delete. Secret scanning and push protection are on. Agents use the maintainer's `gh` account, so protection is the guard against direct pushes. The maintainer changes branch-protection settings themselves; agents give the exact setting.
- PR #1 (Phase 0 handoff docs) merged 2026-11-08.
- PR-evidence check moved to `.github/workflows/pr-evidence.yml` (this PR). It runs on `opened, edited, synchronize, reopened`, reads the live body with `gh pr view`, so editing the description (or a manual re-run) checks the current text. Logic in `.github/scripts/check-pr-evidence.sh` with tests in `test-check-pr-evidence.sh`, run by the workflow. The same PR fixed a bug: multi-line HTML comments in the template (the Handoff hint) counted as content, so an untouched Handoff section passed. Known limit: the check runs the script from the PR's own checkout, so a PR could weaken it; the maintainer's review covers that.
- Repository: Cargo workspace (9 `loom-*` crates in `crates/`, `tools/extractor`, `tools/parity-cli`), all empty skeletons with boundary docs only, no tests yet; `ui/` React + Vite shell; CI in `.github/workflows/ci.yml`; rules in `AGENTS.md`.
- Pinned toolchain: Rust 1.99.1 (`rust-toolchain.toml`), Node 26 (`.node-version`), pnpm 12.18.0 (`ui/package.json`), Tauri 2.12.
- Not done in Phase 0 (not required by the action plan): `cargo-deny` licence/advisory policy, Dependabot, fixture manifest schema, CODEOWNERS (deferred by ADR-012).
- Open questions for the maintainer: none.

## Workstation (maintainer, Windows)
- Installed 2026-11-08: rustup + Rust 1.99.1 with wasm32; Node 26.13 LTS (replaced 14.15.3); pnpm 12.18 (winget); C++ workload + Windows 11 SDK in VS 2022 and VS 2026 Community. rustc links with VS 2026 (MSVC 14.51), the newest VS with a linker.
- Present before: git 2.55, gh 2.102 (logged in as `DBAggie`), WebView2, winget, WSL 1 (Ubuntu).
- Commits use the GitHub noreply address (repo-local `user.email`).
- Still missing: LuaJIT (needed at Spike C).

## Next action
Spike A, game-data extraction (`docs/plan/ACTION-PLAN.md`):
1. Run `/dev-docs Spike A - Bundles2 extraction` to create `dev/active/spike-a-.../` with a plan, context, and PR breakdown.
2. Review the plan with the `plan-reviewer` agent and get the maintainer's approval.
3. Implement in small PRs, one per session, each updating this section.

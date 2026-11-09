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

## Current state (updated 2026-11-08, PR A1 opened)
- **PR A1: decompressor decision (PR #6, open, branch `spike-a/a1-decompressor`).** Every open Oodle decoder derives from GPL-3.0 ooz; `oozextract` (labeled MIT on crates.io) is a port of it with no LICENSE file. **Maintainer decision 2026-11-08: external tool boundary.** No ooz-derived code is in the tree or `Cargo.lock`; `loom-extractor` runs a separately installed `unoodle` (`oozextract` 0.5.7 CLI) through the `Decompressor` trait (`tools/extractor/src/decompress.rs`). The `#[ignore]` install test decoded both games' `_.index.bin` on the maintainer workstation. Report: `docs/spikes/spike-a-extraction.md` (survey, decision, bundle and index layout). `_.index.high.bin`, `_.index.low.bin`, and `.index.dbg` are not needed. Details in the Spike A context file, section "PR A1".
- **PR A1a: `cargo-deny` (PR #4, merged 2026-11-08 as `be077e9`; CI run 42532581603 green).** `deny.toml` checks licenses, bans, and sources; new CI job `cargo-deny` (`EmbarkStudios/cargo-deny-action` v2.1.1, cargo-deny 0.20.2, SHA-pinned) is required through `ci-ok`. Passes locally with `cargo deny --all-features --locked check licenses bans sources`. Five MPL-2.0-only crates reached only through Tauri (`cssparser`, `cssparser-macros`, `dtoa-short`, `selectors`, `option-ext`) are per-crate exceptions; the maintainer accepted them by merging PR #4 (the PR stated that merging accepts them). Advisories are not checked yet (follow-up). Details in the Spike A context file, section "PR A1a".
- **Phase 0 (bootstrap): done.** Gate passed: CI run 42526884777 green on Linux and Windows, including the wasm32 build of the core crates (<https://github.com/DBAggie/Exiled-Studio/actions/runs/42526884777>). `main` is protected, admins included: PR required (0 approvals, since the maintainer cannot approve PRs opened under their own account), required status checks `ci-ok` and `PR evidence` (strict: branch up to date), linear history, no force pushes. Merges are squash-only and branches auto-delete. Secret scanning and push protection are on. Agents use the maintainer's `gh` account, so protection is the guard against direct pushes. The maintainer changes branch-protection settings themselves; agents give the exact setting.
- PR #1 (Phase 0 handoff docs) and PR #2 (PR-evidence workflow) merged 2026-11-08.
- **Spike A: planned (PR #3, merged 2026-11-08).** Plan, context, and tasks in `dev/active/spike-a-bundles2-extraction/`; reviewed by `plan-reviewer` (findings applied, summary in the context file). Planned PRs A1a-A6 are listed in the plan file.
- PR-evidence check lives in `.github/workflows/pr-evidence.yml` (PR #2). It runs on `opened, edited, synchronize, reopened`, reads the live body with `gh pr view`, so editing the description (or a manual re-run) checks the current text. Logic in `.github/scripts/check-pr-evidence.sh` with tests in `test-check-pr-evidence.sh`, run by the workflow. The same PR fixed a bug: multi-line HTML comments in the template (the Handoff hint) counted as content, so an untouched Handoff section passed. Known limit: the check runs the script from the PR's own checkout, so a PR could weaken it; the maintainer's review covers that.
- Repository: Cargo workspace (9 `loom-*` crates in `crates/`, `tools/extractor`, `tools/parity-cli`). All are empty skeletons with boundary docs only, except `tools/extractor`, which has the decompressor module and tests (PR A1); `ui/` React + Vite shell; CI in `.github/workflows/ci.yml`; rules in `AGENTS.md`.
- Pinned toolchain: Rust 1.99.1 (`rust-toolchain.toml`), Node 26 (`.node-version`), pnpm 12.18.0 (`ui/package.json`), Tauri 2.12.
- Not done yet (not required by the action plan so far): `cargo deny` advisories job, Dependabot, fixture manifest schema, CODEOWNERS (deferred by ADR-012).
- Spike A decisions by the maintainer (2026-11-08; details in `dev/active/spike-a-bundles2-extraction/spike-a-bundles2-extraction-context.md`): cross-check against the newest PoB data export available, re-searched at A6 (now PoE1 3.29.3 `1b4078dc4`, PoE2 0.5.5 `49e93925d` from `PathOfBuilding-PoE2`), cloned into `local/`; vendor the community dat schema; permissive licenses only via `cargo-deny`, else RePOE fallback (A1 found only GPL-derived decoders; the maintainer chose the external tool boundary instead, see PR A1 above); patch-delta ceiling 2%. The gate reading is recorded in the ACTION-PLAN Spike A section.
- Open questions for the maintainer: none.
- Known stale doc: `docs/plan/IMPLEMENTATION-PLAN.md` Phase 2 item 4 ("converter from legacy exporter output") predates revised ADR-005; the ACTION-PLAN and ADR-005 win.

## Workstation (maintainer, Windows)
- Installed 2026-11-08: rustup + Rust 1.99.1 with wasm32; Node 26.13 LTS (replaced 14.15.3); pnpm 12.18 (winget); C++ workload + Windows 11 SDK in VS 2022 and VS 2026 Community. rustc links with VS 2026 (MSVC 14.51), the newest VS with a linker.
- Present before: git 2.55, gh 2.102 (logged in as `DBAggie`), WebView2, winget, WSL 1 (Ubuntu).
- Commits use the GitHub noreply address (repo-local `user.email`).
- cargo-deny 0.20.2 installed 2026-11-08 (`cargo install --locked cargo-deny@0.20.2`).
- `unoodle` (oozextract 0.5.7 CLI, the Oodle decoder for the extractor) installed 2026-11-08 (`cargo install --locked oozextract@0.5.7 --features cli`). It is a workstation tool, deliberately not a dependency (Spike A report, section 2).
- Agent shells: neither PowerShell nor Git Bash has cargo on PATH; prefix with `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`.
- Still missing: LuaJIT (needed at Spike C).

## Next action
Spike A, game-data extraction:
1. If PR A1 (#6) is still open: wait for the maintainer to merge it, then record the merge (commit, CI run) here.
2. Next PR is **A2** (plan step 2): bundle container parser and index reader in `tools/extractor`, using the layout in `docs/spikes/spike-a-extraction.md` sections 3 and 4. Rebuild the path list from the embedded path bundle and confirm the path hash by matching all file records. Call the decoder once per bundle. Add a `list-files` subcommand, synthetic-byte and corrupt-input property tests, and a maintainer run with counts and index hashes for PoE1 and PoE2. Tick tasks in `spike-a-bundles2-extraction-tasks.md` and run `/dev-docs-update`.

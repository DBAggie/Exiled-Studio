# CLAUDE.md - Exiled Studio (codename `loom`)

Read `AGENTS.md` first: crate boundaries, commands, code expectations, and the PR evidence rule. This file adds Claude Code specifics.

## Status
Phase 0 (bootstrap) is done. Spike A (game-data extraction, `docs/plan/ACTION-PLAN.md`) is done: gate **met**, A6b merged 2026-11-09; task docs in `dev/completed/spike-a-bundles2-extraction/`. Spike B (text to stat-ID resolution) is done: gate **met** with PR #27 (B5b); every Spike B PR merged by 2026-11-10 (last PR #32, B5e): 99.7% of non-unique lines resolve against 98%, `uncategorized` 0; report `docs/spikes/spike-b-resolution.md`; task docs in `dev/completed/spike-b-text-stat-resolution/`. Spike C (thin oracle harness) is done: gate **met** with PR #39 (C5, merged 2026-11-10), decision `FEASIBLE_WITH_ADAPTATION` (build define `PRNG=0` and the shims `inflate`, `write_guard`, `variant_order`, all made); PRs #33 to #37 merged by 2026-11-10 (#38 squashed into #37's branch); the corpus and 30 public fixtures run identically 3 times on Windows and Linux and across them; the CI `oracle` jobs are required in `ci-ok`; report `docs/spikes/spike-c-oracle.md`; task docs in `dev/completed/spike-c-oracle-harness/`. ADR-015 Amendment 1 (typed resolutions and `adapted` lines) merged 2026-11-10 with PR #40. Phase 2 (canonical model and data) is in progress: the plan PR #41 (P2-0) merged 2026-11-10; task docs in `dev/active/phase-2-canonical-model/`. The Phase 2 PRs form a stack (maintainer decision 2026-11-10: each PR targets the previous PR's branch, and the maintainer merges newest to oldest): P2-1 (domain model) is PR #42, P2-2a (item input, `pob_item` moved to `loom-serialization::legacy`) is PR #43, P2-2b (scope and bases, `loom-parser::import::ItemData`) is PR #44, P2-2c (Mods filter, `loom-parser::import::mods` with per-game `ModRules`) is PR #45, and P2-2d (compositions, grafts, properties, tolerances; completes step 2, Spike B results unchanged) is PR #46, and P2-3a (bundle format v1 model and loader in `loom-data`; item rules moved there) is PR #47, and P2-3b (bundle converter, `loom-parity-cli bundle`) is PR #48, and P2-4 (bundle validator, `bundle-check`) is PR #49, and P2-5a (PoB XML import and semantic export, structure) is PR #50, and P2-5b (PoB XML items; step 5 complete) is PR #51, open, in review (review findings applied). Next: P2-6 (`resolve_build`) (handoff, Next action). Do not start work for a later step before the earlier gate passes.

## One PR per session
Each PR is done in a fresh session. Before opening the PR, update every document it made stale (the handoff's Current state and Next action on every PR; the full list is in `AGENTS.md`, "One PR per session"). Run `/dev-docs-update` for planned tasks. **Right after `gh pr create`, before any other work, push a follow-up commit that writes the PR number into the handoff's Current state and every other doc that names the PR** (`AGENTS.md`, "When to update"). The next session must be able to resume from the repository alone.

## Authoritative sources
1. `docs/plan/ACTION-PLAN.md` - sequence, spikes, gates.
2. `docs/adr/` - accepted decisions. Do not contradict an ADR; propose an amendment and wait for the maintainer.
3. `docs/handoff/SUCCESSOR-HANDOFF.md` - context for new sessions.

## Hard rules
- Never modify the legacy repository at `D:\POE Dev\PathOfBuilding`. Read only.
- Game files are read only to derive data for accuracy. Never copy GGG art, audio, or other assets into this project (ADR-000).
- The game is the truth; PoB is the pinned regression oracle. Every difference from PoB needs evidence and a divergence-register entry (ADR-001/006).
- Modifiers are stat-ID first; never silently drop content you can't resolve (ADR-015).
- Core crates must stay wasm32-compatible: no fs, threads, clocks, or network (ADR-016).
- No telemetry. Network access only in integration adapters (ADR-011).
- PRs are small, single-purpose, and carry their own evidence: tests, parity/divergence diff, and coverage touched (ADR-012).
- `main` is protected. Work on a branch and open a PR; commit or push only when the maintainer asks.

## Workflow
- Persistent task docs: `/dev-docs <task>` creates `dev/active/<task>/`; `/dev-docs-update` refreshes them before a context reset.
- Review plans with the `plan-reviewer` agent before implementing; review code with the `code-architecture-reviewer` agent before opening a PR.
- Rust code follows the `rust-best-practices` skill. Lint: `cargo clippy --all-targets --all-features --locked -- -D warnings`. Use `#[expect(...)]` with a reason instead of `#[allow(...)]`.

## Maintainer workstation
- VS 2022 and VS 2026 Community both have the C++ workload; rustc links with the newest (VS 2026, MSVC 14.51). CI's Windows runner uses its own image's MSVC.
- Game installs and corpus paths are in `docs/handoff/SUCCESSOR-HANDOFF.md`. Never commit anything from them unscrubbed.

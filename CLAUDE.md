# CLAUDE.md - Exiled Studio (codename `loom`)

Read `AGENTS.md` first: crate boundaries, commands, code expectations, and the PR evidence rule. This file adds Claude Code specifics.

## Status
Phase 0 (bootstrap) is done. Spike A (game-data extraction, `docs/plan/ACTION-PLAN.md`) is done: gate **met**, A6b merged 2026-11-09; task docs in `dev/completed/spike-a-bundles2-extraction/`. Spike B (text to stat-ID resolution) is done: gate **met** with PR #27 (B5b); every Spike B PR merged by 2026-11-10 (last PR #32, B5e): 99.7% of non-unique lines resolve against 98%, `uncategorized` 0; report `docs/spikes/spike-b-resolution.md`; task docs in `dev/completed/spike-b-text-stat-resolution/`. Spike C (thin oracle harness) is done: gate **met** with PR #39 (C5, merged 2026-11-10), decision `FEASIBLE_WITH_ADAPTATION` (build define `PRNG=0` and the shims `inflate`, `write_guard`, `variant_order`, all made); PRs #33 to #37 merged by 2026-11-10 (#38 squashed into #37's branch); the corpus and 30 public fixtures run identically 3 times on Windows and Linux and across them; the CI `oracle` jobs are required in `ci-ok`; report `docs/spikes/spike-c-oracle.md`; task docs in `dev/completed/spike-c-oracle-harness/`. ADR-015 Amendment 1 (typed resolutions and `adapted` lines) merged 2026-11-10 with PR #40. Phase 2 (canonical model and data) is done: gate **met** 2026-11-11 (P2-11, PR #58; report `docs/phase-2/phase-2-report.md`), every maintainer-run item re-run at `3982838` equal to its committed record; plan PR #41 (P2-0) merged 2026-11-10; task docs in `dev/completed/phase-2-canonical-model/`. The Phase 2 PRs form a stack, all open, in review (maintainer decision 2026-11-10: each PR targets the previous PR's branch, and the maintainer merges newest to oldest): #42 P2-1 (domain model), #43 P2-2a (item input), #44 P2-2b (scope and bases), #45 P2-2c (Mods filter), #46 P2-2d (compositions, grafts, properties, tolerances), #47 P2-3a (bundle model and loader), #48 P2-3b (bundle converter), #49 P2-4 (bundle validator), #50 P2-5a (PoB XML structure), #51 P2-5b (PoB XML items), #52 P2-6a (bundle-backed `ItemData`), #53 P2-6b (`resolve_build`), #54 P2-7 (DTO v1 and generated schemas), #55 P2-8 (`build-round-trip` and CI gate job), #56 P2-9 (migrations and file persistence), #57 P2-10 (share codes), and #58 P2-11 (report and gate verdict). The maintainer squash-merged #58 down to #43 on 2026-11-11, so #42's branch holds all of Phase 2; #42 into `main` is blocked by an external DNS problem, and until it clears new PRs branch from the open PR's branch and target it, with CI not checked (maintainer instruction 2026-11-11). Phase 3 (calc slice): plan PR P3-0 open, in review (`dev/active/phase-3-calc-slice/`, base `feat/p2-1-domain-model`); next P3-1a (bundle v2 manifest and skill tables) per the handoff's Next action. Do not start work for a later step before the earlier gate passes.

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

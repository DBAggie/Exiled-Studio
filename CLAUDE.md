# CLAUDE.md - Exiled Studio (codename `loom`, planning stage)

## Status
Planning documents only. Do not bootstrap code, CI, or a git repository until the maintainer explicitly authorizes Phase 0. A fuller `AGENTS.md`/`CLAUDE.md` is a Phase 0 deliverable (ADR-012).

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

## Workflow
- Persistent task docs: `/dev-docs <task>` creates `dev/active/<task>/`; `/dev-docs-update` refreshes them before a context reset.
- Review plans with the `plan-reviewer` agent before implementing; review code with the `code-architecture-reviewer` agent before opening a PR.
- Rust code follows the `rust-best-practices` skill. Lint: `cargo clippy --all-targets --all-features --locked -- -D warnings`. Use `#[expect(...)]` with a reason instead of `#[allow(...)]`.

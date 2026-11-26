# ADR-012: Repository, Toolchain, and Governance

- Status: Accepted (revised 2026-11-08); Amendment 1 (related work bundled into one PR, 2026-11-25)
- Date: 2026-02-01

## Decision
The successor is a separate repository (codename `loom`) with protected `main`, GitHub Flow, short-lived branches, release tags, and feature flags for incomplete work. Use Rust stable with a pinned toolchain, Cargo workspace, Tauri 2, React/TypeScript, Vite, pnpm, generated bindings, and GitHub Actions for Windows and Linux (required) and macOS (best-effort). Core crates are also checked against `wasm32` (ADR-016).

## Governance before first public release (revised)
- The team is the maintainer plus AI agents. The **maintainer plus CI are the reviewers**; CI must substitute for the domain reviewers that do not exist yet.
- **PR evidence rule (enforced by CI and the PR template):** every PR is small and single-purpose and includes its test results, any parity/divergence report diff, and the mechanics or coverage it touches. PRs without evidence are not reviewed.
- The repository includes `AGENTS.md`/`CLAUDE.md` describing crate boundaries, commands, test expectations, and the evidence rule for agent contributors.
- CODEOWNERS and formal domain owners are deferred until after the first public release, when a core reviewer team reviews all PRs (human or agent-authored).
- Lightweight ADRs remain mandatory for architectural changes.

## Absolute gates
Calculation/data compatibility (per ADR-001/006), security, and migration safety are absolute gates.

## Consequences
- No successor bootstrap is performed in the legacy repository.
- Cross-repository oracle use is explicit and pinned.
- Maintainer review capacity is the main throughput limit; PR size and evidence rules protect it.

## Amendment 1 (2026-11-25): related work is bundled into one PR
Decided by the maintainer on 2026-11-25: "lets change the pr rule. I care more about bundling work into a single pr where possible because the ci tests take a very long time to complete." Written in that session on PR #183; the maintainer approves the wording by merging it.

### Context
The evidence rule above says every PR is small and single-purpose, and `AGENTS.md` added "one PR per session". A full CI run takes a long time, and it runs for every PR, so a list of small tasks done one PR each spends most of its time waiting on CI. In practice the maintainer has already asked for bundles many times (Phase 3's close-out #104, Phase 4 #105, Phase 6 #167, the open items and UI revamp stack #168 to #179).

### Decision
- **Bundle where possible.** Work that belongs together (the tasks of one list, the steps of one plan, fixes found along the way) goes into one PR. A session continues the open PR on its branch when there is one that fits, and opens a new PR only when the work is unrelated to it, or the open PR is already merged or the maintainer is in the middle of reviewing it.
- **A commit per step.** Each task or step is its own commit with a message that names it, so a bundle can still be read and reverted step by step.
- **The evidence rule stays, per step.** The PR's Tests, Parity / divergence, Coverage touched, and Handoff sections cover every step in it; a step that changes numbers carries its parity or divergence evidence as before.
- **A PR is not tied to a session.** A session may do several steps of a PR; a PR may span several sessions. The handoff rule stays: every push keeps the handoff true, so a fresh session can resume from the repository alone.
- "Small and single-purpose" is replaced by "one coherent body of work". The absolute gates are unchanged.

### Consequences
- Fewer CI runs and less waiting; larger diffs to review, read commit by commit.
- A failing step holds up the rest of its bundle. A step that is risky or likely to be argued about can still be its own PR.
- Stacked PRs (one per task, each on the previous branch) are no longer the default for unattended work; one branch with a commit per task is.

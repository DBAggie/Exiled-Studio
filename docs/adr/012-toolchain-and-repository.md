# ADR-012: Repository, Toolchain, and Governance

- Status: Accepted (revised 2026-11-08)
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

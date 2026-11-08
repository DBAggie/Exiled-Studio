# Exiled Studio - planning set (codename `loom`)

## Purpose
This directory contains the accepted architecture decisions, new-session handoff, and action plan for an independent, modern successor to Path of Building. Working product name: **Exiled Studio** (public GitHub repo `DBAggie/Exiled-Studio`); internal crate codename `loom`. Not affiliated with or endorsed by Grinding Gear Games or the Path of Building Community project. It contains documentation only; no successor repository has been bootstrapped.

## Reading order
1. `docs/handoff/SUCCESSOR-HANDOFF.md` - concise context for a new session.
2. `docs/adr/000-context-and-governance.md` - independent-project relationship, naming, license.
3. Remaining ADRs in numeric order (001–017).
4. `docs/plan/ACTION-PLAN.md` - **authoritative** sequence and gates (agreed 2026-11-08).
5. `docs/plan/IMPLEMENTATION-PLAN.md` - supporting work-package detail; superseded where it conflicts.
6. `docs/spikes/spike-c-legacy-oracle.md` - detail for Spike C; reduced in scope by ADR-006.

## Directory map
```
README.md              entry point (this file)
CLAUDE.md              rules for AI agents working in this repository
.claude/               agent tooling
  skills/              rust-best-practices
  agents/              plan-reviewer, code-architecture-reviewer
  commands/            /dev-docs, /dev-docs-update
docs/
  adr/                 Architecture Decision Records + index (README.md)
  plan/                ACTION-PLAN.md (authoritative), IMPLEMENTATION-PLAN.md (detail)
  handoff/             new-session context
  spikes/              spike specifications (Spike C: legacy oracle harness)
dev/                   created by /dev-docs: active/ and completed/ task docs
```
Phase 0 adds the Rust workspace and frontend at the repository root alongside `docs/`.

## Current next action
After explicit authorization to bootstrap, execute Phase 0, then Spikes A, B and C in order (see `docs/plan/ACTION-PLAN.md`). Do not edit or bootstrap the legacy repository.

## Decision summary
Full Rust rewrite; game is truth with PoB as pinned regression oracle; stat-ID-first modifiers; own Bundles2 extractor; shipped signed data bundles; API-first sandboxed addons with a local MCP server as the Claude integration; PoE1 first with a PoE2-ready core; Windows/Linux gated, macOS best-effort; WASM-compatible core; Tauri 2 + React/TypeScript/Vite; versioned JSON with XML compatibility; maintainer + CI review until release; MIT.

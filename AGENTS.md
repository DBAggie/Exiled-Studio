# AGENTS.md - working in Exiled Studio (`loom`)

Rules for anyone contributing, human or AI agent. `CLAUDE.md` adds Claude Code specifics. The ADRs in `docs/adr/` are authoritative; `docs/plan/ACTION-PLAN.md` sets the order of work.

## Hard rules
- The game is the truth; PoB v2.67.2 is the pinned regression oracle. Every difference from PoB needs evidence and a divergence-register entry (ADR-001/006).
- Never modify the legacy PoB repository. Legacy source is fetched at a pinned public tag, never copied in (ADR-006).
- Game files are read only to derive data. Never commit GGG art, audio, other assets, or raw game files (ADR-000). Local game data and extractor output go in `local/` or `out/` (both ignored).
- Modifiers are `(stat_id, values)`. Text parsing lives only in import adapters; unresolved content is quarantined and shown, never dropped (ADR-015).
- No telemetry. Network access only in `loom-integrations` (ADR-011).
- Do not contradict an ADR. Propose an amendment and wait for the maintainer.

## Layout
```
crates/
  loom-domain         core   build aggregate, IDs, value objects
  loom-data           core   canonical game-data model, abstract bundle source
  loom-parser         core   text -> stat-ID resolution, import parsing
  loom-calc           core   calculations over immutable snapshots
  loom-serialization  core   versioned DTOs, migrations, legacy XML adapters
  loom-persistence    host   files, autosave, SQLite indexes
  loom-app            host   command/query/event API, single-writer build task
  loom-integrations   host   network adapters, MCP addon transport
  loom-tauri          shell  Tauri 2 desktop app
tools/
  extractor           loom-extractor: maintainer-run Bundles2 extractor
  parity-cli          loom-parity-cli: thin oracle harness
ui/                   React + TypeScript + Vite (pnpm)
docs/                 ADRs, plans, handoff, spikes
```

### Crate boundaries
Dependencies flow one way, with no cycles:
- `loom-domain` <- `loom-data` <- `loom-parser`, `loom-calc`
- `loom-domain` <- `loom-serialization` <- `loom-persistence`
- all of the above <- `loom-app` <- `loom-integrations` <- `loom-tauri`
- `tools/*` depend on core crates only.

**Core crates** (loom-domain, loom-data, loom-parser, loom-calc, loom-serialization) must:
- build for `wasm32-unknown-unknown` (CI checks every PR; ADR-016);
- not use the filesystem, threads, clocks, environment, or network. Each has a `clippy.toml` that bans these; hosts inject them;
- depend only on other core crates and pure, wasm-compatible libraries;
- carry no PoE1-only assumptions in shared types (ADR-001).

Adding a crate to the core set means updating the CI `wasm` job, this list, and copying the core `clippy.toml` into the crate.

## Commands
Toolchain: Rust pinned by `rust-toolchain.toml`; Node per `.node-version`; pnpm per `packageManager` in `ui/package.json`. Windows needs the MSVC C++ build tools and a Windows SDK; Linux needs the Tauri system packages listed in `.github/workflows/ci.yml`.

```sh
# Rust (same as CI)
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --locked --target wasm32-unknown-unknown -p loom-domain -p loom-data -p loom-parser -p loom-calc -p loom-serialization

# Frontend (in ui/)
pnpm install --frozen-lockfile
pnpm format:check && pnpm lint && pnpm build

# Desktop app in dev mode: run `pnpm dev` in ui/, then in another terminal
cargo run -p loom-tauri
```

## Code expectations
- Rust follows `.claude/skills/rust-best-practices`. Workspace lints are errors in CI.
- No `unwrap`/`expect` outside tests (ADR-014). Use typed errors with stable codes.
- Use `#[expect(lint, reason = "...")]`, never `#[allow(...)]` (enforced by clippy).
- Every `TODO` links an issue: `// TODO(#42): ...`.
- Tests: descriptive names (`resolve_should_quarantine_unknown_line`), deterministic, no network and no game install. Fixtures from real builds are scrubbed of account names and machine paths (ADR-007).

## One PR per session: leave a handoff
Each PR is done in its own fresh session, and nothing carries over except what is in the repository. **A PR is not finished until the next session can pick up where it left off from the documents alone.** In the same PR, update every document that the PR made stale:

| Document | Update when |
|---|---|
| `docs/handoff/SUCCESSOR-HANDOFF.md` | Every PR: rewrite **Current state** (what is done, what is in flight, open questions, blockers) and **Next action** (the exact next PR or step). |
| `CLAUDE.md` **Status** | The current phase/spike or its next step changes. |
| `docs/plan/ACTION-PLAN.md` **Next action** / gate status | A gate passes or fails, or the next step changes. Changing the sequence itself needs the maintainer. |
| `dev/active/<task>/` | Every PR in a planned task: run `/dev-docs-update`; tick tasks, record decisions and exact next steps. |
| `AGENTS.md` | Commands, crate boundaries, layout, or rules change. |
| `README.md` | User-visible status or build steps change. |
| `docs/adr/README.md` | An ADR or amendment is added (with maintainer approval). |

Write state as facts with dates and links (PR numbers, CI runs, file paths), not as "see conversation". Record anything learned that is hard to rediscover from the code: failed approaches, environment quirks, open decisions waiting on the maintainer.

## Pull requests (ADR-012 evidence rule)
- Small and single-purpose; branch from `main` (GitHub Flow).
- Fill in every evidence section of `.github/pull_request_template.md`: **Tests**, **Parity / divergence**, **Coverage touched**, and **Handoff** (which documents above were updated). CI fails a PR with an empty section. Write `None: <reason>` when a section does not apply.
- All CI checks green (the `ci-ok` check is required). The maintainer reviews and merges; `main` is protected.
- Architectural changes need an ADR or amendment (`docs/adr/README.md`).

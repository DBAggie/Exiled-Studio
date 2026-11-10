# AGENTS.md - working in Exiled Studio (`loom`)

Rules for anyone contributing, human or AI agent. `CLAUDE.md` adds Claude Code specifics. The ADRs in `docs/adr/` are authoritative; `docs/plan/ACTION-PLAN.md` sets the order of work.

## Hard rules
- The game is the truth; PoB v2.67.2 is the pinned regression oracle. Every difference from PoB needs evidence and a divergence-register entry (ADR-001/006).
- Never modify the legacy PoB repository. Legacy source is fetched at a pinned public tag, never copied in (ADR-006).
- Game files are read only to derive data. Never commit GGG art, audio, other assets, or raw game files (ADR-000). Local game data and extractor output go in `local/` or `out/` (both ignored).
- Modifiers are `(stat_id, values)`. Text parsing lives only in import adapters; unresolved content is quarantined and shown, never dropped (ADR-015).
- No telemetry. Network access only in `loom-integrations` (ADR-011). One exception outside the crates: the maintainer and CI acquire scripts in `tools/oracle/` fetch the pinned oracle sources with `git`; no Rust code fetches (Spike C plan step 1, accepted by the maintainer 2026-11-10; the scripts are not application code, so ADR-011 is unchanged).
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
  parity-cli          loom-parity-cli: maintainer-run oracle and corpus harnesses
  oracle              not a crate: PoB oracle manifest, acquire scripts, adapter, output schema, fixtures (Spike C)
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
# Dependency licenses, bans, sources (policy in deny.toml; once: cargo install --locked cargo-deny@0.20.2)
cargo deny --all-features --locked check licenses bans sources

# Maintainer only: extractor tests against a local game install (not in CI).
# Needs the Oodle decoder installed separately (never a dependency; docs/spikes/spike-a-extraction.md):
#   cargo install --locked oozextract@0.5.7 --features cli
# Set LOOM_POE1_INSTALL and/or LOOM_POE2_INSTALL to the game folders, then:
cargo test -p loom-extractor --test install_decompress -- --ignored --nocapture
cargo test --release -p loom-extractor --test install_index -- --ignored --nocapture
cargo test --release -p loom-extractor --test install_dat -- --ignored --nocapture
# List an install's files (counts, index SHA-256, unmatched entries; --all prints every file):
cargo run --release -p loom-extractor -- list-files "<game folder>" [--decoder <program>] [--sample <n> | --all]
# Extract the Spike A tables (and every table they reference), the stat-description files, and the passive skill graphs (*.psg) to JSON plus manifest.json; <dir> must not exist:
cargo run --release -p loom-extractor -- extract "<game folder>" --game poe1|poe2 --out local/extract/<game>/<steam build id>
# Determinism check: two extract runs, byte comparison (Git Bash; output in local/determinism/<game>/):
tools/extractor/scripts/check-determinism.sh "<game folder>" poe1|poe2
# Cross-check an extract output against a PoB checkout cloned into local/ (never the legacy repo);
# writes report.json and summary.json, exits 1 while the gate is not met:
cargo run --release -p loom-extractor -- cross-check local/extract/<game>/<build> local/pob/<checkout> --game poe1|poe2 --out local/cross-check/<game>/<build>
# Load an extract's stat-description files through loom-data; prints counts as JSON, exits 1 if a check fails:
cargo run --release -p loom-parity-cli -- load-statdesc local/extract/<game>/<build>
# Render every stat-description variant and resolve it back; writes summary.json and report.json (<dir> must not exist):
cargo run --release -p loom-parity-cli -- round-trip local/extract/<game>/<build> --out <dir>
# Compare the renderer with PoB's exported mod text (PoE1; PoB checkout in local/pob/); exits 1 on an uncategorized difference:
cargo run --release -p loom-parity-cli -- render-check local/extract/poe1/<build> local/pob/<checkout> --out <dir>
# Resolve every item line of the PoB builds (*.xml) in <builds-dir> (PoE1 extract with item_types/, manifest format 4);
# ambiguous lines go through the Mods filter, resolved lines get mod-backed, every other line gets a
# category; summary.json is scrubbed (builds by content hash, no item names or author text); exits 1
# while the Spike B gate is not met. Each --pob is a `git archive <tag> src/Data` of the legacy repo
# (read-only), the legacy-text evidence; Spike B used the six tags listed in docs/spikes/spike-b-resolution.md:
cargo run --release -p loom-parity-cli -- resolve-corpus "<builds-dir>" local/extract/poe1/<build> --out <dir> --pob <tag>=local/pob-<tag> [--pob ...]

# PoB oracle (Spike C). Acquire: fetch PoB, LuaJIT and luautf8 at the commits in tools/oracle/manifest.json
# into local/oracle/ and build the runtime there (network: git fetch only; nothing installed elsewhere).
# Windows needs Visual Studio with the C++ workload; Linux needs git, make, cc, python3:
powershell -ExecutionPolicy Bypass -File tools/oracle/acquire.ps1    # or: tools/oracle/acquire.sh
# Verify (offline): PoB commit, tree, clean checkout including ignored files, runtime sources, built
# files against local/oracle/build.json, runtime version and build flags; JSON report on stdout;
# exits 10 (ORACLE_ACQUISITION) or 11 (ORACLE_ENVIRONMENT) on failure:
cargo run --release -p loom-parity-cli -- oracle-verify tools/oracle/manifest.json local/oracle
# Run fixtures through the adapter (tools/oracle/adapter/oracle.lua, protocol 2, with the shims in
# shims.lua: Inflate through zlib, io.open write guard) on the pinned runtime, 3 times each (offline;
# verifies first; checks the checkout after each fixture). Writes report.json, summary.md, outputs/,
# runs/ to <dir> (must not exist); exits 3 if a fixture is partial, else with the smallest failing
# class (10-18). The public and synthetic fixtures:
cargo run --release -p loom-parity-cli -- oracle-run tools/oracle/manifest.json local/oracle --out <dir> local/oracle/pob/spec/TestBuilds/3.13 tools/oracle/fixtures/minimal-level1.xml tools/oracle/fixtures/timeless-militant-faith.xml
# Fake adapters, the real adapter on the synthetic fixtures (and a partial result without zlib), and the
# Lua tests of json.lua and shims.lua, all on the real runtime (ignored in plain `cargo test`; Windows
# paths handed to LuaJIT must be ASCII). LOOM_ORACLE_DIR must be absolute (cargo test runs in the crate):
LOOM_ORACLE_DIR="$PWD/local/oracle" cargo test -p loom-parity-cli -- --ignored

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
| `dev/active/<task>/` | Every PR in a planned task: run `/dev-docs-update`; tick tasks, record decisions and exact next steps. When the task closes, move its folder to `dev/completed/` (maintainer decision 2026-11-09). |
| `AGENTS.md` | Commands, crate boundaries, layout, or rules change. |
| `README.md` | User-visible status or build steps change. |
| `docs/adr/README.md` | An ADR or amendment is added (with maintainer approval). |

When to update:
1. **Before opening the PR:** commit every update in the table on the PR branch, writing the PR as "open, in review".
2. **Right after `gh pr create`, before any other work:** push a follow-up commit to the same branch that writes the new PR number (`PR #N, open, branch <name>`) into the handoff **Current state** and into every other document that names the PR (`CLAUDE.md` Status, `docs/plan/ACTION-PLAN.md`, `dev/active/<task>/`, where the "Open PR" task is ticked with the number). The PR number does not exist until the PR is opened, and without it the next session cannot find the PR.
3. **On every later push to the PR** (review fixes, rebases): keep the handoff true in the same push.
4. **The merge is recorded by the next session:** first thing, it checks the PR's state (`gh pr view <N>`), and if it merged, its own PR records the squash commit and the main CI run in **Current state**.

Write state as facts with dates and links (PR numbers, CI runs, file paths), not as "see conversation". Record anything learned that is hard to rediscover from the code: failed approaches, environment quirks, open decisions waiting on the maintainer.

## Pull requests (ADR-012 evidence rule)
- Small and single-purpose; branch from `main` (GitHub Flow).
- Fill in every evidence section of `.github/pull_request_template.md`: **Tests**, **Parity / divergence**, **Coverage touched**, and **Handoff** (which documents above were updated). CI fails a PR with an empty section. Write `None: <reason>` when a section does not apply.
- All CI checks green. Two checks are required: `ci-ok` (`.github/workflows/ci.yml`) and `PR evidence` (`.github/workflows/pr-evidence.yml`). `PR evidence` re-runs when the description is edited, so fix an empty section by editing the PR, not by pushing. The check logic is `.github/scripts/check-pr-evidence.sh`; test it with `bash .github/scripts/test-check-pr-evidence.sh`. The maintainer reviews and merges; `main` is protected.
- Architectural changes need an ADR or amendment (`docs/adr/README.md`).

## Dependencies
- Licenses must be permissive and MIT-compatible; the allow-list is in `deny.toml` and the `cargo-deny` CI job enforces it on the whole tree. Never add a license to the global allow-list to get a crate in. A crate that needs another license gets a per-crate exception with a reason, and only with maintainer approval.
- crates.io only; git or other registries need an explicit `deny.toml` entry.

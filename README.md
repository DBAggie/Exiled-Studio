# Exiled Studio

An independent, open-source build planner for Path of Exile, written in Rust with a Tauri 2 desktop shell. Internal codename `loom`; crates are named `loom-*`.

> **Not affiliated with or endorsed by Grinding Gear Games or the Path of Building Community project.** Exiled Studio is a separate project with its own governance. Path of Building is used only as a pinned regression reference.

**Status: pre-release scaffold.** The repository has a workspace, CI, and contributor rules, but no build-planning features yet. The game-data extraction spike (Spike A) has met its gate; next is text-to-stat-ID resolution (Spike B, see `docs/plan/ACTION-PLAN.md`). The name gets a final trademark check before the first release.

## Principles
- The game is the source of truth. Path of Building v2.67.2 is a pinned regression oracle; every intentional difference is recorded with evidence.
- Modifiers are modelled on the game's stat IDs, not on English text.
- Game data comes from the project's own extractor and ships as signed bundles of derived data only: no Grinding Gear Games art or audio.
- Works offline. No account, no telemetry.
- PoE1 first, with a core ready for PoE2. Windows and Linux are supported; macOS is best-effort.
- The calculation core compiles to WebAssembly.

## Building
Requirements: Rust (version pinned in `rust-toolchain.toml`, installed via rustup), Node (`.node-version`), pnpm (`ui/package.json`), and the platform prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/).

```sh
cargo test --workspace
cd ui && pnpm install && pnpm build
```

`AGENTS.md` lists every command CI runs and the contributor rules.

## Repository map
```
crates/        Rust workspace: core crates (domain, data, parser, calc, serialization) and host crates (persistence, app, integrations, tauri)
tools/         maintainer tools: game-data extractor, parity (oracle) CLI
ui/            React + TypeScript + Vite frontend
docs/adr/      Architecture Decision Records (authoritative) + index
docs/plan/     ACTION-PLAN.md (authoritative sequence), IMPLEMENTATION-PLAN.md (detail)
docs/handoff/  context for new contributor/agent sessions
docs/spikes/   spike specifications
.claude/       AI agent tooling (skills, review agents, commands)
AGENTS.md      contributor rules for humans and AI agents
```

## Contributing
Read `AGENTS.md`. Pull requests are small and single-purpose and carry their own evidence (tests, parity/divergence diff, coverage touched); the PR template and CI enforce this.

## License
[MIT](LICENSE).

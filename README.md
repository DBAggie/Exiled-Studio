# Exiled Studio

An independent, open-source build planner for Path of Exile, written in Rust with a Tauri 2 desktop shell. Internal codename `loom`; crates are named `loom-*`.

> **Not affiliated with or endorsed by Grinding Gear Games or the Path of Building Community project.** Exiled Studio is a separate project with its own governance. Path of Building is used only as a pinned regression reference.

**Status: pre-release scaffold.** The repository has a workspace, CI, and contributor rules, but no build-planning features yet. The three spikes (game-data extraction, text-to-stat-ID resolution, and the legacy PoB oracle harness) have met their gates, and so has Phase 2, the canonical build model and data (import and export of PoB builds and share codes, the successor's JSON build document, game-data bundles; `docs/phase-2/phase-2-report.md`), and Phase 3, the first calculations (a self-cast spell's and a main-hand attack's hit, the character and its core defences, each field with a completeness state and a breakdown, checked against PoB; `docs/phase-3/phase-3-report.md`). Phase 4 adds the desktop application: import a PoB build, a dashboard and breakdowns, item editing with background recalculation and undo, save and export, and a local MCP server an AI client can read the build through (changes need the user's approval; `docs/phase-4/phase-4-report.md`). Phase 5 adds the release: installers for Windows and Linux, signed application updates, signed game-data bundles with a signed feed, local logs, crash records, and a diagnostic package the user previews, plus the security and performance reports (`docs/phase-5/`). No version has been released yet. The name gets a final trademark check before the first release.

## Principles
- The game is the source of truth. Path of Building v2.67.2 is a pinned regression oracle; every intentional difference is recorded with evidence.
- Modifiers are modelled on the game's stat IDs, not on English text.
- Game data comes from the project's own extractor and ships as signed bundles of derived data only: no Grinding Gear Games art or audio.
- Works offline. No account, no telemetry.
- PoE1 first, with a core ready for PoE2. Windows and Linux are supported; macOS is best-effort.
- The calculation core compiles to WebAssembly.

## Installing
Download from **https://dbaggie.github.io/Exiled-Studio/** once the first release is published. The page lists two channels (`docs/release/channels.md`):
- **Stable**, released from `main`: the tested version.
- **Experimental**, released from the `experimental` branch: newer and tested less. It installs as a separate application, Exiled Studio Experimental, with its own settings and data, beside the stable one.

Each channel has these packages:
- **Windows:**
  - the setup (`*_x64-setup.exe`), which also installs Microsoft's WebView2 runtime if it is missing, with no network needed;
  - the MSI.
- **Linux:** an AppImage and a `.deb`.

The setup, the MSI, and the AppImage update themselves, within their channel, when you approve. The `.deb` tells you a new version exists.

**Windows SmartScreen:** the installers are not code signed. The first time you run a downloaded setup, Windows shows "Windows protected your PC". Click **More info**, then **Run anyway**. Updates and game data are signed with the project's own keys and checked before they are used (`docs/release/signing.md`).

## Privacy
Exiled Studio has no account and no telemetry, and it works offline. It connects to the network only for these:
- **The check for a new version and the check for new game data.** Each is an HTTPS request to GitHub, which hosts the releases and the download page (the experimental application reads its feeds from the download page's site). GitHub sees your IP address, and the request shows the application's version. Nothing else is sent. Under About, Updates, choose **manual** (check only when you ask) or **strict offline** (never connect).
- **The local MCP server**, when you turn it on: loopback only, with a token, and changes need your approval.

Logs and crash records stay on your computer. A diagnostic package is made only when you ask, and you see its contents before saving it.

## Building
Requirements: Rust (version pinned in `rust-toolchain.toml`, installed via rustup), Node (`.node-version`), pnpm (`ui/package.json`), and the platform prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/).

```sh
cargo test --workspace
cd ui && pnpm install && pnpm build && pnpm test
# run the app (a game-data bundle folder is needed to import builds):
cargo run -p loom-tauri
```

`AGENTS.md` lists every command CI runs and the contributor rules.

## Repository map
```
crates/        Rust workspace: core crates (domain, data, parser, calc, serialization, pipeline, tree) and host crates (persistence, app, integrations, tauri)
tools/         maintainer tools: game-data extractor, parity (oracle) CLI, schema generator, release tool, benchmarks, packaging scripts
fuzz/          fuzz targets of the hostile-input readers (cargo-fuzz; its own workspace)
release/       the trust lists and public keys compiled into the application
site/          the download page on GitHub Pages (docs/release/channels.md)
schemas/       generated JSON Schemas (build, calc result, app API) and the MCP tool contract
ui/            React + TypeScript + Vite frontend
docs/adr/      Architecture Decision Records (authoritative) + index
docs/plan/     ACTION-PLAN.md (authoritative sequence), IMPLEMENTATION-PLAN.md (detail)
docs/handoff/  context for new contributor/agent sessions
docs/spikes/   spike specifications
.claude/       AI agent tooling (skills, review agents, commands)
AGENTS.md      contributor rules for humans and AI agents
```

## Contributing
Read `AGENTS.md`. Pull requests bundle related work, a commit per step, and carry their own evidence (tests, parity/divergence diff, coverage touched); the PR template and CI enforce this.

## License
[MIT](LICENSE).

The community builds in `tools/oracle/fixtures/community/` are test fixtures credited to their creators, not covered by this project's license; any creator can have their build removed (see [SOURCES.md](tools/oracle/fixtures/community/SOURCES.md)).

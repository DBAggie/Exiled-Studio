# ADR-018: Knowledge MCP Server (Read-Only Game Data for Claude Sessions)

- Status: Accepted (proposed 2026-11-15; accepted by the maintainer 2026-11-16 with its five defaults, "Not decided here"; implemented in the same PR, #123)
- Date: 2026-11-15

## Context
The knowledge pack (`DBAggie/exiled-studio-knowledge`, plan `KNOWLEDGE-PACK-PLAN.md` agreed 2026-11-15, decision D6a) makes Claude sessions Path of Exile experts. Its rule for game data (layer 1: gems, mods, uniques, bases, passives) is **tool lookups only, never recall from memory**. The app MCP (ADR-017) cannot serve that:
- it runs inside the desktop app, only while the user has started it, and answers about the open build;
- it has no lookup of the game data itself (no search over mods, gems, uniques, bases, or passives);
- it has no economy snapshots.

The data already exists: the signed bundles (ADR-005, ADR-010; `loom_data::bundle`, installed by `loom_app::install` under `<app data>/bundles/<game>/<steam build>-s<schema>/`) and Spike B's text-to-stat-ID resolution (`loom-parser`). The economy snapshots are dated JSON files the knowledge repo's refresh job writes (plan D4 rule 2, D7).

## Decision
Add a **knowledge MCP server**, a host tool at `tools/knowledge-mcp` (crate `loom-knowledge-mcp`, binary `exiled-studio-knowledge-mcp`).

### Transport and trust
- **stdio only.** No listening socket, so no port and no token (ADR-017's localhost rules do not apply). The MCP client starts it as a child process; it exits with its stdin.
- **Read-only.** It has no write tool, no command path, and changes no file. It never opens the app's builds, settings, recovery copies, or logs.
- **No network.** It makes no outbound or inbound connection (ADR-011). Economy data reaches it only as snapshot files on disk, written by the knowledge repo's refresh job (which runs elsewhere and is not part of this tool).
- **No telemetry**, and it logs only to stderr, never to a file.

### Data it serves
- **Bundles:** by default the newest installed bundle under `<app data>/bundles/` whose status is `Status::Trusted` under the trust list compiled into the binary from `release/trust.json` (`trust_list(false)`). That is stricter than the app: `install::scan` also returns `Status::Accepted` bundles the user accepted, and this tool does not serve those. The public test key is never trusted, because anyone can sign with it. `--bundle <dir>` picks a folder for development, labelled `unsigned` in every response, as the app labels it. `--data-dir <dir>` overrides the app data folder.
- **File integrity:** `scan` checks only the manifest signature, so at startup the tool runs `verify_full` on the chosen bundle (every file against its manifest hash) and refuses to serve a bundle that fails.
- **Snapshots:** `--snapshots <dir>`, the knowledge plugin's `skills/poe1-economy/snapshots/` folder. Each snapshot file carries the pack's header (`game`, `patch`, `league`, `as_of`, `sources`, `verification`). Snapshots are unsigned, untrusted input: parsed with size caps and a schema check, and every price response says `signed: false` and carries the snapshot's `verification`.
- **Games:** each bundle is one game; a response never mixes games. A request names `game` (`poe1` or `poe2`); a game without a bundle answers with an error saying so, never with the other game's data.

### Tools (contract `schemas/knowledge-mcp-tools.v1.json`, tested like `mcp-tools.v1.json`)
| Tool | Answers from |
|---|---|
| `get_snapshot_info` | Which bundle and snapshots are loaded: game, patch, Steam build, bundle hash, signed or not, each snapshot's league and `as_of` |
| `search_mods` | Mods by stat ID, text, tag, generation type, domain |
| `get_gem` | A skill or support gem: tags, levels, stats per level, costs |
| `get_unique` | A unique by name: base, mod lines with stat IDs, variants |
| `get_base` | An item base: class, implicits, requirements, defences or weapon stats, socket cap |
| `get_passive` | A passive node or notable or keystone by name or ID: stats with stat IDs, kind, ascendancy |
| `resolve_stat` | Text plus a `scope` (item, passive, gem) to `(stat_id, values)` through `loom_parser::resolve` (`Index::build(scope)`, Spike B); returns its `Outcome` (`Resolved`, `Ambiguous`, `Unresolved`) and `shadowed`; unresolved text is returned as unresolved, never dropped (ADR-015) |
| `get_price` | An item's or currency's price from the snapshot for the league asked, with the snapshot's `as_of` |

Every response carries `game`, `patch`, `steam_build` (`null` when the manifest has none, with `patch: "unknown"`), `manifest_sha256` (SHA-256 of `bundle.json`), `source.index_sha256`, and `signed`; a price response also carries the snapshot's `league`, `as_of`, and `verification`.

### Refusals
- `get_price` refuses when the snapshot's `game` does not match the loaded bundle's, or when no snapshot exists for the league asked. It never falls back to another league's prices. A `patch` mismatch (a hotfix such as 3.30.0c changes the Steam build) adds a warning to the response and is not a refusal.
- A lookup for a name that exists only in the other game answers "not in <game> <patch>", never the other game's record.

### Crate boundaries
This tool ships, so the `AGENTS.md` exception for `loom-release` and `loom-bench` ("tools, never shipped") does not cover it, and depending on `loom-app` would pull calc, persistence, tree, zip, and tracing into its binary. Instead:
- `loom-knowledge-mcp` depends on `loom-data` (`bundle`, `signature`), `loom-domain`, `loom-parser`, and one new small host crate.
- **Before the tool is built**, a separate PR (done in the same PR, #123, at the maintainer's request 2026-11-16) moves the host-side bundle scan (`install::status_of` and `scan`, `Status`, `trust::trust_list`) out of `loom-app` into that host crate (non-core: it reads files), which `loom-app` then uses too, so the trust rule cannot drift between the two.
- It does not depend on `loom-integrations`: the stdio JSON-RPC framing is small and lives in the tool, sharing only the MCP protocol version string.

### Scope against ADR-010 and ADR-017
This is a host tool, not an addon. It never talks to the app, so ADR-010's addon API and ADR-017's rules for the app's MCP server do not apply to it.

### Patch label
Bundles name their Steam build, not a patch number. The tool maps a Steam build to a patch through a reviewed table in the tool (`patches.json`, one row per extracted build, set by the maintainer at each data run; the handoff's data-run checklist gains that row so the table does not go stale). First row (maintainer, 2026-11-16): Steam build 26530219 is patch 3.30.0 (Covens of Umbra). A bundle whose build is not in the table reports `patch: "unknown"`, and `get_price` then refuses.

### Distribution
Built and signed with the application's release (ADR-010), shipped as a separate release asset and beside the installed app. The knowledge plugin's `.mcp.json` runs `${EXILED_STUDIO_KNOWLEDGE_MCP:-exiled-studio-knowledge-mcp}`, so a developer can point it at `target/release/`.

## Consequences
- Claude sessions get game data from the bundles we ship, with their provenance on every answer, whether or not the app is running.
- A new public contract (`knowledge-mcp-tools.v1.json`), versioned and tested like ADR-017's.
- `AGENTS.md` gains the crate-boundary exception and the tool's commands; `docs/adr/README.md` lists this ADR.
- Snapshot data is only as fresh as the last reviewed refresh PR in the knowledge repo; the tool reports `as_of` and never fetches.
- PoE2: the same tool serves a PoE2 bundle once one is installed; the knowledge pack's PoE2 plugin waits for PoE2 1.0 (plan D2).

## Implementation (PR #123, 2026-11-16)
- **`crates/loom-bundles`** (host, read-only): `DirSource`, the trust lists and manifest check (`trust_list`, `verify_manifest`, moved from `loom_app::trust`), the bundle status and scan (`Status`, `Found`, `status_of`, `scan`, `bundle_id`, moved from `loom_app::install`), and `verify_full`. `loom-app` re-exports them; its own `scan` first sweeps stale staging folders (a write), then calls the read-only one.
- **`tools/knowledge-mcp`** (`loom-knowledge-mcp`, binary `exiled-studio-knowledge-mcp`): `knowledge` (open the newest `Trusted` bundle of the game, or `--bundle` unsigned; every file hashed on load), `patches` (`patches.json`), `snapshots` (32 MiB per file, 400 files, header check; skipped files reported), `lookup` (the eight tools), `tools` (the contract), `rpc` (JSON-RPC 2.0 over stdio, 1 MiB lines, the app MCP's error codes).
- **Contract:** `schemas/knowledge-mcp-tools.v1.json`, compared by `tests/contract.rs` (rewritten with `LOOM_WRITE_KNOWLEDGE_CONTRACT=1`).
- **`get_unique`** returns the flavour text and `mods: null`: the client data does not list a unique's mods (the tool says so instead of guessing).
- **Release:** the `release` workflow builds the binary on Windows and Linux, ships it as its own asset of the draft release (`exiled-studio-knowledge-mcp_<version>_x64.exe`, `..._amd64`) and in the portable zip beside the app, where the `portable` marker makes it serve the zip's `data\` folder. The NSIS, MSI, AppImage, and .deb installers do not carry it yet (handoff, Next action). "Signed" for this asset means it is built by the same reviewed `release` run and published with the draft release the maintainer reviews; it has no signature file of its own, and the Linux asset needs `chmod +x` after download.
- **Data folder:** without `--data-dir` it takes `LOOM_DATA_DIR`, else the portable `data\` beside it, else the stable app's data folder for this user, or the experimental app's (`io.github.dbaggie.exiledstudio.experimental`) when only that one has installed bundles.
- **Untrusted snapshots:** besides the per-file caps, at most 128 MiB over all files (newest first); a price answer carries only listed row fields (name, base type, variant, links, levels, values, listing count), strings cut to 200 characters, at most 100 matches.
- **Patch:** a loaded bundle whose Steam build is not in `patches.json` makes `get_price` refuse (`KNOWLEDGE_PATCH_UNKNOWN`), as "Patch label" says; with no bundle loaded, prices answer with a warning that the patch was not checked.
- **Input:** stdin lines are read at most 1 MiB at a time (a longer one is answered with a parse error and skipped), and invalid UTF-8 is a parse error, not the end of the server. The schemas' limits are enforced as -32602.

## Defaults (accepted by the maintainer 2026-11-16 with the ADR)
1. **Bundles the user accepted in the app** are not served here (only `Status::Trusted`). Alternative: serve `Status::Accepted` too, labelled as such.
2. **The patch table** lives in the tool. Alternative: a `patch` field added to the bundle manifest (a data schema change, ADR-005).
3. **Distribution** as a release asset beside the app. Alternative: users build it with `cargo install --path tools/knowledge-mcp`.
4. **`get_price` input**: item name plus optional variant and links, as poe.ninja's overview rows have them.
5. **The host crate** for the bundle scan (name and exact contents) is settled in its own PR (it became `loom-bundles`, in PR #123 itself). Alternative: do not ship the tool (`cargo install` only, option 3), so the existing "tools, never shipped" exception applies and it may use `loom-app` directly.

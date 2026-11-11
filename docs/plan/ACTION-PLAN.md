# Action Plan (agreed 2026-11-08)

This plan supersedes the phase order in `IMPLEMENTATION-PLAN.md` and replaces the single spike in `docs/spikes/spike-c-legacy-oracle.md` with three ordered spikes. Where they conflict, this document and the revised ADRs win. Work packages in `IMPLEMENTATION-PLAN.md` not contradicted here remain valid detail.

## Decisions made in the 2026-11-08 review
| Topic | Decision | ADR |
|---|---|---|
| Authority | The game is the truth; PoB v2.67.2 is the pinned regression oracle; differences need evidence in the divergence register | 001, 006 |
| Modifiers | Stat-ID first; text is an import adapter; unresolved lines quarantined | 015 |
| Game data | Own Rust extractor from Bundles2, maintainer-run; PoB data as cross-check; RePOE fallback | 005 |
| Data delivery | Shipped signed bundles only | 005, 010 |
| Addons | API-first, sandboxed; trust levels, not a ban | 010 |
| Claude | Local MCP server as the reference addon; read-only by default | 017 |
| PoE2 | PoE1 first; PoE2 data in the pipeline from Phase 2; PoE2 calcs after the Phase 4 gate | 001 |
| Platforms | Windows + Linux gated; macOS best-effort | 001, 012 |
| Web | Core must compile to wasm32; shipping a web app is not decided | 016 |
| Team | Maintainer + AI agents; maintainer + CI review until first release; then a core reviewer team | 012 |
| First milestone | Narrow archetype slice | 013 |
| Name / license | Distinct name before release; codename `loom`; MIT | 000 |

## Sequence

### Phase 0: Bootstrap (done 2026-11-08)
- **Step 0, workstation toolchain** (installed by the bootstrap session via winget, with maintainer approval per install): rustup + stable Rust; Node 20 LTS or newer (replaces Node 14.15.3, so warn about other projects first); pnpm; Visual Studio 2022 Build Tools with the C++ workload and Windows 11 SDK (only VS 2017 is present). LuaJIT is deferred to Spike C. WSL is on version 1; WSL 2 is optional, since CI covers Linux.
- `git init` in `D:\POE Dev\PathOfBuilding-V2`, keeping `docs/` and `.claude/` as the first commit; create the **public** GitHub repo `DBAggie/Exiled-Studio` and push; protect `main`. MIT license; README with the non-affiliation statement; crates prefixed `loom-`.
- Cargo workspace (domain, calc, data, parser, serialization, app, persistence, integrations, tauri, tools/extractor, tools/parity-cli) and pnpm frontend.
- CI: fmt, clippy, tests; Windows/Linux required, macOS best-effort; `wasm32` build of core crates.
- `AGENTS.md`/`CLAUDE.md`, PR template enforcing the evidence rule, ADR index.
- **Gate:** clean checkout builds and tests on Windows and Linux; wasm32 check passes.

### Spike A: Game-data extraction
- Read the PoE1 and PoE2 Bundles2 index; confirm a usable decompression implementation and its license (Oodle-compatible reimplementation).
- Parse with the community dat schema: Stats, Mods, BaseItemTypes, PassiveSkills, ActiveSkills/GrantedEffects, and stat-description files.
- Record game version, schema revision, and input hashes.
- **Gate:** two runs produce identical output hashes; record counts match PoB's data for the same patch, with every gap explained.
  - *Reading of the gate (maintainer decision 2026-11-08):* Steam keeps only the current patch, so "the same patch" means the newest PoB data export available (searched upstream when the cross-check runs): `PathOfBuilding` for PoE1, `PathOfBuilding-PoE2` for PoE2, read from clones in `local/`. Every gap gets a category with tool evidence; uncategorized gaps must be zero and gaps due to patch changes at most 2% of PoB keys per table. Details: `dev/completed/spike-a-bundles2-extraction/`.
  - *Gate status (2026-11-09, PR A6b): **met.*** Items 1 (determinism), 3 (no silent drops), 4 (recorded inputs), and 5 (licenses) met; item 2 (coverage) met for mods, bases, skills, stats, and passives on PoE1 and PoE2 (0 uncategorized in both directions; patch-delta 2 PoE1 keys, 0 PoE2). Passive rows are placed with the game's passive skill graphs; PoE1 rows off the character tree get the category `not-on-tree` (maintainer decision 2026-11-09, PoB's PoE1 tree being GGG's export, not a reproducible PoB rule). Evidence: `docs/spikes/spike-a-extraction.md`, sections 14 to 17.
- **Fallback:** RePOE as input; ADR-005 amended.

### Spike B: Text to stat-ID resolution
- Using Spike A stat descriptions, resolve every item mod line in the maintainer's 21 PoE1 builds (22 until 2026-11-09; the maintainer confirmed 21).
- **Gate:** ≥ 98% of non-unique explicit and implicit lines resolve; every unresolved line is categorized (uniques, special cases, data gaps, parser gaps). Excluded from the rate, counted apart (maintainer decisions): PoB custom lines, flag and class-requirement lines, unselected variants, author notes (2026-11-10), and legacy text the game no longer has (`data-gap/legacy-text`, 2026-11-10). This redefines the gate (maintainer decisions 2026-11-10, B4b and B5b): exclusions total 572 lines (14.5%) of the corpus; without the legacy-text exclusion the rate is 97.1% (3,315 / 3,414). The exclusion is a rate rule only: per ADR-015, excluded lines stay quarantined and shown, never dropped.
- **Fallback:** hybrid model per ADR-015, decided per area.

### Spike C: Thin oracle harness (about two weeks maximum)
- Rust CLI drives legacy `HeadlessWrapper.lua` on pinned LuaJIT at PoB v2.67.2 (public source, not copied).
- Emit normalized `mainOutput` plus selected breakdowns for the corpus; record all metadata.
- **Gate:** deterministic repeated runs on Windows and Linux; environment failures are distinguished from behavior differences; divergence register format defined.
  - *Gate status (2026-11-10, PR #39, C5): **met**, decision `FEASIBLE_WITH_ADAPTATION`.* Items 1 (determinism: 30 public fixtures in CI and 21 corpus builds, 3 runs each on Windows and Linux, identical across platforms too), 2 (every failure class injected by a test), 3 (register schema, validator, docs; in CI), and 4 (recorded inputs) met. Adaptations, all made and recorded in every output: LuaJIT define `LUAJIT_SECURITY_PRNG=0`; adapter shims `inflate` (timeless-jewel data), `write_guard` (no writes into the checkout), `variant_order` (PoB's own nondeterminism for old-format Vaal gems). Not done (outside the gate): packaged-artifact mode. Evidence: `docs/spikes/spike-c-oracle.md`.

### Phase 2: Canonical model and data
DTOs and generated schemas, Build aggregate, bundle format and validator, PoB XML import through stat resolution, JSON persistence, migrations. PoE2 data passes through the same pipeline (no calcs).
**Gate:** semantic round-trip of the corpus; unknown content preserved; PoE2 bundles validate with no PoE1 assumptions in shared crates.
**Status:** gate met 2026-11-11 (P2-11, `docs/phase-2/phase-2-report.md`); #43 to #58 merged down into #42 on 2026-11-11; #42 into `main` pending.

### Phase 3: Calc slice
Self-cast spell and non-triggered attack archetypes plus core defences; structured breakdown graph; completeness states; divergence register in use.
**Gate:** corpus builds in scope match the oracle or have registered, evidenced divergences.

### Phase 4: Application slice + MCP addon
Tauri/React: import, dashboard, breakdown viewer, item edit, async recalc, save JSON, export XML. Local MCP server with read-only default and approved writes.
**Gate:** end-to-end workflow on Windows and Linux; accessibility checks; MCP contract tests.

### Phase 5: Packaging, signing, updates
Installers, two-key signing, prompted app updates, signed data bundles with rollback, security checks.

### Phase 6+: Expansion
PoE1 mechanics expansion (each promoted through corpus and oracle gates); passive-tree editor; PoE2 calculations begin; PoE2 `.build` import.

## Deferred on purpose
Public product name (before first release), WASM addon runtime, hosted sharing, in-app Claude assistant panel, macOS as a release gate, offline-root signing ceremony.

## Next action
Phase 0 is done (gate passed 2026-11-08). Spike A is done (gate met 2026-11-09, PR A6b). Spike B is done (gate met 2026-11-10 with PR #27 (B5b); every Spike B PR merged by 2026-11-10, last PR #32 (B5e)): 99.7% of non-unique lines resolve (98% asked), every line categorized (`uncategorized` 0); report `docs/spikes/spike-b-resolution.md`; task docs `dev/completed/spike-b-text-stat-resolution/`. Spike C is done (gate met 2026-11-10 with PR #39 (C5, merged 2026-11-10); PRs #33 to #37 merged 2026-11-10, #38 squashed into #37): decision `FEASIBLE_WITH_ADAPTATION`; report `docs/spikes/spike-c-oracle.md`; task docs `dev/completed/spike-c-oracle-harness/`. ADR-015 Amendment 1 (typed resolutions and adapted lines) merged 2026-11-10 (PR #40). Phase 2 (canonical model and data) is done: gate met 2026-11-11 (P2-11; report `docs/phase-2/phase-2-report.md`); plan PR #41 (P2-0) merged 2026-11-10; plan and task docs in `dev/completed/phase-2-canonical-model/`. The Phase 2 PRs are an open stack for the maintainer to merge: #42 P2-1 (domain model), #43 P2-2a (item input), #44 P2-2b (scope and bases), #45 P2-2c (Mods filter), #46 P2-2d (compositions, grafts, properties, tolerances), #47 P2-3a (bundle model and loader), #48 P2-3b (bundle converter), #49 P2-4 (bundle validator), #50 P2-5a (PoB XML structure), #51 P2-5b (PoB XML items), #52 P2-6a (bundle-backed `ItemData`), #53 P2-6b (`resolve_build`), #54 P2-7 (DTO v1 and generated schemas), #55 P2-8 (`build-round-trip` and CI gate job), #56 P2-9 (migrations and file persistence), #57 P2-10 (share codes), and #58 P2-11 (report and gate verdict). The stack was merged down to #42 on 2026-11-11; #42 into `main` waits on an external DNS problem. Phase 3 (calc slice): plan PR #59 (P3-0) open, in review, stacked on #42 (maintainer instruction 2026-11-11); plan in `dev/active/phase-3-calc-slice/`. P3-1a (bundle format 2 with the skill tables) PR #60 open, stacked on #59; P3-1b (character and passive tables) PR #61 open, stacked on #60. The live state and the exact next step are in the handoff's **Current state** and **Next action**.

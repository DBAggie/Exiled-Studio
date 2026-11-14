Last Updated: 2026-11-14

# Passive tree editor and Phase 5 (context)

## Why this task exists
- 2026-11-13, maintainer: after the Phase 4 PR, approve the UI redesign and implement the passive skill tree now (it was Phase 6+); order: UI fidelity (PR #106), then the tree, then Phase 5. Tree scope: a full editor (allocate and deallocate with path finding, masteries, jewel sockets, ascendancy, cluster jewels, tree version switching). Tree art: GGG's published sprites only within the developer page's stated terms, own shapes where a use is not permitted.
- 2026-11-13, maintainer, after #106 merged (squash `218cdc0`): "complete the passive tree and phase 5 in the next PR."

## Key files
- Tree data: `crates/loom-data/src/bundle/passives.rs` (format 2 passive tables, `PassiveTree`, `TreeGroup`, `TreeNode`), `crates/loom-data/src/bundle/validate.rs`.
- Domain: `crates/loom-domain/src/passive.rs` (`PassiveSpec`, `MasteryEffect`, `JewelSocket`), `id.rs` (`NodeId`, `SpecId`, `TreeVersion`).
- Calc: `crates/loom-calc/src/poe1/collect.rs` (`passives`, connectivity, `cluster-jewel-node`, tree version check), `input.rs` (`CalcOptions.tree_version`, `PassiveLinks`), `state.rs` (reasons).
- Pipeline and app: `crates/loom-pipeline/src/lib.rs`, `crates/loom-app/src/{app,session,edit,view,data}.rs`, `crates/loom-serialization/src/dto/app/v1.rs`, `crates/loom-integrations/src/mcp/tools.rs`, `crates/loom-tauri/{tauri.conf.json,src/lib.rs}`.
- PoB XML: `crates/loom-serialization/src/legacy/pob_xml/{read,write}.rs`.
- Extractor: `tools/extractor/src/extract.rs` (`LOOKUP_TABLES` already has `PassiveTreeExpansionSkills`, `...Jewels`, `...JewelSizes`; `PASSIVE_TABLES`).
- UI: `ui/src/screens/*`, `ui/src/components.tsx`, `ui/src/theme`, `docs/ui` (no tree mockup).
- CI: `.github/workflows/ci.yml` (jobs fmt, rust, dto-compare, schema, round-trip, oracle, oracle-compare, wasm, deny, frontend, e2e, ci-ok).
- Pinned PoB (read only, `local/oracle/pob/src`, v2.67.2): `Classes/PassiveTree.lua` (orbit constants 204-208, positions 873-874, `CalcOrbitAngles` 1015), `Classes/PassiveSpec.lua` (`AllocNode` 770, `DeallocNode` 812, `CountAllocNodes` 822, `BuildPathFromNode` 896, `NodesInIntuitiveLeapLikeRadius` 1067, `BuildAllDependsAndPaths` 1098, `BuildClusterJewelGraphs` 1723, `BuildSubgraph` 1885), `Data/ClusterJewels.lua`, `TreeData/<v>/tree.lua` (2_6 to 3_29, alternate and ruthless variants).

## ADRs touched
- ADR-008: needs **Amendment 1** (order and renderer details); step T0.
- ADR-000: no GGG art committed; decision T-a.
- ADR-001/006: PoB is the oracle for allocation rules and cluster IDs; differences registered.
- ADR-004/009: tree commands through the single-writer session and snapshot undo.
- ADR-005: data bundle format 2 additions (layout, cluster jewels, tree packs); GPL code stays out (PoB is read, not copied).
- ADR-010 (revised): two keys, prompted app updates, signed data bundles, rollback, quarantine, offline modes. Network "confined to Tauri/application integration adapters".
- ADR-011: no telemetry; network only in adapters.
- ADR-013: release quality, performance tiers and budgets.
- ADR-014: structured logs, redaction, correlation IDs, diagnostic export.
- ADR-015: tree pack stat text resolved to stat IDs, unresolved quarantined.
- ADR-016: `loom-tree` is a core crate (wasm32).
- ADR-017: MCP tree tools with approved writes.

## External references
- GGG developer data page, https://www.pathofexile.com/developer/docs/data (read 2026-11-14): lists the PoE1 Passive Skill Tree export (https://github.com/grindinggear/skilltree-export), the atlas tree, and the PoE2 tree as "the exception" to "We do not officially provide access to any in-game data outside of our supported APIs". No licence, permission, or attribution text on the page or in the repository's README (which has `data.json`, `alternate.json`, `ruthless.json`, `ruthless-alternate.json`, `assets/`; sprite sheets since 3.18.1).
- GGG developer docs, https://www.pathofexile.com/developer/docs (read 2026-11-14): "As a general rule, we cannot allow our Intellectual Property to be used to generate commercial revenue."; apps that interact with the game or its files are against the Terms of Use; third-party notice: "Every public or widely available application should include some form of the following statement in a visible location": "This product isn't affiliated with or endorsed by Grinding Gear Games in any way."; "We reserve the right to remove access to any or all of our APIs and services without notice at our discretion."
- GGG Terms of Use, clause 7 (a, b, c, f, i) and clause 24 (no GGG logos in links).

## Decisions (defaults taken; maintainer questions below)
- T-a to T-i, A-a to A-d, P-a to P-h: see the plan.

## Maintainer answers (2026-11-14, this session)
- MQ1 tree art: **ship GGG's sprites** (asked twice; told there is no stated licence and that GGG bars commercial use of its IP; chose it anyway). Done as: fetched at a pinned commit, shipped only inside signed bundles, never committed; own shapes as fallback; hard rules reworded (ADR-005 Amendment 2).
- MQ2 and MQ8: **amend ADR-005** (export as pinned input, `tools/tree` acquire script).
- MQ3: **ship derived data** (game and export) in signed bundles, with GGG's notice.
- MQ4: **no OS code signing now**; SmartScreen warning documented.
- MQ5: **GitHub Releases** as host.
- MQ6: **the maintainer generates the real keys** per the runbook; throwaway keys in CI until then.
- MQ7: **cut line as planned**.

## Open questions for the maintainer (as asked; answers above)
- **MQ1. GGG sprites.** The terms are silent on redistributing the export's sprite sheets; default: own shapes, no GGG art shipped or committed.
- **MQ2. `tools/tree` acquire script** fetching `grindinggear/skilltree-export` with `git` at pinned commits (a tools script, as the oracle's acquire scripts, AGENTS.md hard rule exception widened); default yes.
- **MQ3. Shipping tree data** derived from the game and GGG's export in signed data bundles (as game-derived bundles are shipped under ADR-010), with GGG's "not affiliated" notice; default yes.
- **MQ4. OS code signing** (Authenticode, notarization): out by default (not one of ADR-010's two keys; cost); SmartScreen and unsigned-package warnings documented.
- **MQ5. Release host**: GitHub Releases of this repository for updates and the data feed; drafts only, the maintainer publishes; default yes.
- **MQ6. Real keys**: the maintainer generates the app (updater) and data keys offline and stores them as repository secrets per the runbook; until then CI uses throwaway keys.

- **MQ7. Cut line**: if the budget runs out, ship T0 to T2, T5a, T6, T7 (bundle's own tree version) and P1 to P5; cluster jewels, older versions, P6, P7 as follow-up PRs.
- **MQ8. ADR-005 Amendment 2**: GGG's tree export as a pinned data input (layout constants, older versions) with provenance; needed by review B1.

## Review
`plan-reviewer`, 2026-11-14: proceed with changes. Every finding is taken into the plan's 2026-11-14 version:
- B1 ADR-005 conflict: ADR-005 Amendment 2 in T0; provenance block (T-c); angle tables cited to the export's README, not PoB (T-b).
- B2 PoB's tie order is Lua hash order: loom's own tie rule; parity compares length and validity, exact nodes only when unique (T-f, gate 2).
- B3 edits must not drop unmodelled nodes: T-g, a test per command, a property test (gate 4).
- B4 CI has no real bundle: synthetic tree fixture bundle T1a; each gate item marked CI or MR.
- M1 cluster tables, socket expansion data, hash formats 1 and 2, driving stat IDs: T-i. M2 PoE1-only rules in `poe1` modules and the `poe1/` bundle section: T-e, T-i. M3 calc stays independent of `loom-tree`, pipeline resolves passives, `TreeData` trait for the overlay: T-e. M4 order: T5a/T5b, layout parity in T1, P5 and P1 early. M5 typed class and ascendancy per spec, `<URL>` regeneration: T-h. M6 installers installed, launched, uninstalled in a recorded release run; `e2e` feature absent: P-a. M7 per-artifact update behaviour, app key rotation, version policy: P-a, P-c, P-d. M8 release environment: P-b. M9 data schema 4 and compatibility rule: D-a. M10 shipped bundle source: P-e. M11 hostile-input unpacking, signed feed with sequence, verification in `loom-data`: P-c, P-e. M12 advisory ignores, `native-tls`: P-d, P-g. M13 budgets fixed now, instruction-count CI check: P-h. M14 cut line, draft PR after T0, Phase 5 steps early: Summary, step order.
- Minor: ascendancy offset citation fixed (T-b); edges compared (gate 1); radius rule and source (T-f); CLAUDE.md hard rule cited (T-a); AGENTS.md exception widened (T-d); privacy note on update checks (P-d); ACTION-PLAN line "each with its own plan PR" (T0); `signatures` field (P-e); verification cost at load (P-e); WebView2 offline installer (P-a); oracle script protocol 5 and seed (T8); blocker defined (P-g).

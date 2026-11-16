Last Updated: 2026-11-16 (T3, T4, T5b, T8 on `feat/tree-remaining`; T1's last part closed as decided)

# Passive tree editor and Phase 5 (tasks)

In the plan's commit order. CI = checked in CI; MR = maintainer-run with a committed record.

- [x] Record #106's merge (squash `218cdc0`) in the handoff
- [x] Plan, context, tasks (`/dev-docs`)
- [x] `plan-reviewer` on the plan; findings in the context file
- [x] Maintainer answers on MQ1 to MQ8 (2026-11-14, context file)
- [x] T0 ADR-008 Amendment 1, ADR-005 Amendment 2, ACTION-PLAN change, ADR README, hard rules
- [x] Open the draft PR (#107); follow-up commit with the PR number
- [x] Mark #107 ready for review (maintainer 2026-11-14: merge the plan now; every step below in follow-up PRs from `dev`)
- [x] (Before P5: the maintainer's UI and UX side project, its own PR from `dev`; done with PRs #108 to #113, the last merged 2026-11-15 as `565f5b0`)
- [x] P5 Logs (tracing, JSON lines, rotation, redaction, correlation IDs), diagnostics export with preview, crash records (PR #115, branch `feat/p5-logs`; `docs/phase-5/logs.md`)
- [x] P1 Packaging: bundler config, version policy (0.1.0), `package` CI job (portable, AppImage smoke), WebView2 offline
- [x] T1a Synthetic tree fixture bundle and its generator (`bundle-poe1-tree`, `make_bundles.py`; `loom-tree` `synthetic_tree` tests; the e2e runs on it; branch `feat/ux-sprint-2`, 2026-11-15)
- [x] T1 Layout file, provenance block, data schema 4, `loom-tree` crate, `tree-parity layout` (MR record). Partly done on `feat/ux-sprint-2` (2026-11-15, `docs/tree/layout.md`): the layout file with its provenance (optional, no schema bump), `tools/tree` acquire and manifest, `tree-layout`, and `loom-tree`. The per-node record against PoB's tree is done too (`docs/tree/records/layout.json`, `docs/tree/parity.md`; 0.01 on the main tree, 0.1 for the export's rounding elsewhere, one offset per ascendancy). Closed 2026-11-16 as decided on `feat/ux-sprint-2`: the layout stays an optional listed file with no schema bump (`docs/tree/layout.md`), so bundles made before it, the 0.9 release's included, stay valid. Making it required now would break them.
- [x] T2 Allocation rules, tie rule, unknown-node preservation, resolved passive set in `loom-pipeline`; Phase 3 documents byte-identical or version-only. Partly done (`docs/tree/editor.md`): the rules, the tie rule, and kept nodes, with tests. Cluster areas are left out as PoB's `PassiveSpec:Init` does (`Node::cluster_area`). The resolved passive set (`CalcOptions.passives`, `loom_pipeline::resolve_passives`; documents byte-identical) and radius jewels that allocate without a path (T-f: `poe1::radius`, `PassiveJewelRadii` in the bundle, data-parity `jewel_radii`) are done too (2026-11-15). Left: real radii need the maintainer's extraction rerun; timeless and other radius jewels are not modelled.
- [x] T5a App commands and queries (bundle version), typed spec class and ascendancy, DTO v1, schemas, MCP tools and contract. Partly done: the six tree commands, `tree_layout`, `preview_tree`, DTO v1, and schemas. `socket-jewel` (cluster jewels refused) and the spec commands `new-spec`, `rename-spec`, `delete-spec` are done too. The MCP tools are done too (`get_tree`, `find_tree_nodes`, `get_tree_node`, `preview_tree_change`, `request_tree_change`; `docs/tree/editor.md`, "MCP"). The typed class and ascendancy are done too (T-h, `PassiveSpec::class` and kin, 2026-11-15).
- [x] T6 Tree screen (canvas, sprites when present, overlay), non-visual view, vitest, axe, keyboard, e2e. Partly done: the canvas in our shapes, the card, the List view, vitest, and axe. The jewel picker and the spec bar are done too. Looked at in the running application (2026-11-15, `docs/tree/screens/`). Sprites (T-a: `tree-layout` copies the export's sheets into bundles, `tree_sprites` query, the canvas draws them, never committed) and the e2e steps (on the synthetic tree bundle) are done too (2026-11-15).
- [x] T7 Export of edited specs (URL regeneration, consistency, cluster hash v2); PoB read-back (MR). Partly done: URL regeneration and the build's class and ascendancy kept consistent. The PoB read-back record is done (`docs/tree/records/readback-*.json`: three community builds, every spec equal, jewels included). Cluster hash v2 done with T3 (`feat/tree-remaining`): a spec edited here exports `clusterHashFormatVersion="2"`; a legacy spec is migrated on import (`docs/tree/cluster-jewels.md`, "The two numberings").
- [x] P2 Signing: trust list, revocation, verification in `loom-data`, runbook `docs/release/signing.md`, release environment
- [x] P3 App updates: plugin (`native-tls`), modes, per-artifact behaviour, prompt, downgrade refusal
- [x] P4 Data bundles: `.loombundle`, hostile-input unpacking, install, verify, pin, rollback, quarantine, signed feed, shipped bundle source
- [x] Tree data run (the maintainer-run work #113 left; branch `feat/tree-data-run`, PR #114): extraction with the jewel tables, bundle with `poe1/jewel_radii.json`, data-parity at format 7 (`jewel_radii` 0 differences once the table is read times the tree scale 1.2, maintainer decision 2026-11-15), `tree-layout` with sprites, calc check (`docs/tree/editor.md`, "The real radii")
- [ ] **Cut line** (maintainer 2026-11-14): the steps above ship; the ones below may become their own PRs
- [x] T3 Cluster jewels: tables, socket expansion data, data-parity, subgraphs, hash formats, calc (`feat/tree-remaining`; `docs/tree/cluster-jewels.md`). Maintainer-run left: the extraction with `PassiveTreeExpansionSpecialSkills`, `data-parity` at export format 9 (`cluster_jewels`), the corpus builds with cluster jewels through `calc-parity`.
- [x] T4 Tree versions: `tools/tree` acquire and manifest (sprites too), `tree-pack`, packs 3_25 to 3_28, calc picks the version (`feat/tree-remaining`; `docs/tree/tree-packs.md`). Every pack under the 1% rule (0.56% to 0.98%) with the reviewed rewordings (`tools/tree/rewordings.json`); layout and allocations equal PoB's for all four. Maintainer-run left: the signed bundle with packs, and `data-parity` per pack.
- [x] T5b Cluster sockets and version conversion in app and UI (`feat/tree-remaining`; `docs/tree/editor.md`, "Cluster sockets" and "Converting a tree"). The e2e sockets a cluster jewel and allocates its node; it cannot show conversion (the synthetic bundle names no tree version).
- [x] T8 `tree-parity alloc` vs PoB, corpus run, `docs/tree/tree-report.md` (`feat/tree-remaining`). Layout, allocations, and conversion drops for 3_25 to 3_28 against PoB; the 12 public builds on older trees; the B3 property test. Maintainer-run left: the private corpus run on the packs (`docs/tree/tree-report.md`, "Maintainer-run").
- [x] P6 Security: advisories and ignores, secret scan, fuzz and properties, capability check, `docs/phase-5/security.md`
- [x] P7 Performance: `tools/bench`, baselines, instruction-count CI check, `docs/phase-5/performance.md`
- [x] P8 Reports, handoff, CLAUDE.md, ACTION-PLAN, AGENTS.md, README (notice, install, privacy), task docs (P1 to P8: the Phase 5 sprint below, PR #118)
- [ ] `code-architecture-reviewer` before marking the PR ready

## Phase 5 sprint (maintainer 2026-11-15: "perform all open p5 steps in this next sprint"; branch `feat/p5-release`)
One PR for P1, P2, P3, P4, P6, P7, P8, one or more commits per step, in this order. Tick each when its commit is pushed; the handoff's Current state names the last one.
- [x] Housekeeping: #117's merge recorded
- [x] P1 packaging (`docs/phase-5/packaging.md`)
- [x] P2 signing (`docs/release/signing.md`)
- [x] P3 app updates (`docs/phase-5/updates.md`)
- [x] P4 data bundles (`docs/phase-5/data-bundles.md`)
- [x] P6 security (`docs/phase-5/security.md`; the CI fuzz record from PR #118's `fuzz.yml` run 43779992583: no crash)
- [x] P7 performance (`docs/phase-5/performance.md`; the baseline is committed from the PR's `bench` log)
- [x] P8 reports and docs; review; PR #118 (`docs/phase-5/phase-5-report.md`)
- [x] The maintainer's requests after the review (2026-11-15), on the same PR:
  - [x] the startup fix: sizes for installed bundles, one load, the window before the data, shipped packs checked by sizes (`performance.md`, "Startup": median 3.11 s to 1.77 s)
  - [x] channels: the `experimental` branch made and protected like `main`; the experimental application (`docs/release/channels.md`)
  - [x] the download page: `site/`, `tools/site/`, `pages.yml`; Pages turned on
  - [x] the `bench` baseline for the new `start` scenario (5,679,942, CI run 43787174465; `count.sh` now counts it)

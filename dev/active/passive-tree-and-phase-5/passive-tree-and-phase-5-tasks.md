Last Updated: 2026-11-14

# Passive tree editor and Phase 5 (tasks)

In the plan's commit order. CI = checked in CI; MR = maintainer-run with a committed record.

- [x] Record #106's merge (squash `218cdc0`) in the handoff
- [x] Plan, context, tasks (`/dev-docs`)
- [x] `plan-reviewer` on the plan; findings in the context file
- [x] Maintainer answers on MQ1 to MQ8 (2026-11-14, context file)
- [x] T0 ADR-008 Amendment 1, ADR-005 Amendment 2, ACTION-PLAN change, ADR README, hard rules
- [ ] Open the draft PR; follow-up commit with the PR number
- [ ] P5 Logs (tracing, JSON lines, rotation, redaction, correlation IDs), diagnostics export with preview, crash records
- [ ] P1 Packaging: bundler config, version policy (0.1.0), `package` CI job (portable, AppImage smoke), WebView2 offline
- [ ] T1a Synthetic tree fixture bundle and its generator
- [ ] T1 Layout file, provenance block, data schema 4, `loom-tree` crate, `tree-parity layout` (MR record)
- [ ] T2 Allocation rules, tie rule, unknown-node preservation, resolved passive set in `loom-pipeline`; Phase 3 documents byte-identical or version-only
- [ ] T5a App commands and queries (bundle version), typed spec class and ascendancy, DTO v1, schemas, MCP tools and contract
- [ ] T6 Tree screen (canvas, sprites when present, overlay), non-visual view, vitest, axe, keyboard, e2e
- [ ] T7 Export of edited specs (URL regeneration, consistency, cluster hash v2); PoB read-back (MR)
- [ ] P2 Signing: trust list, revocation, verification in `loom-data`, runbook `docs/release/signing.md`, release environment
- [ ] P3 App updates: plugin (`native-tls`), modes, per-artifact behaviour, prompt, downgrade refusal
- [ ] P4 Data bundles: `.loombundle`, hostile-input unpacking, install, verify, pin, rollback, quarantine, signed feed, shipped bundle source
- [ ] **Cut line** (maintainer 2026-11-14): the steps above ship; the ones below may become their own PRs
- [ ] T3 Cluster jewels: tables, socket expansion data, data-parity, subgraphs, hash formats, calc
- [ ] T4 Tree versions: `tools/tree` acquire and manifest (sprites too), `tree-pack`, packs 3_25 to 3_28, calc picks the version
- [ ] T5b Cluster sockets and version conversion in app and UI
- [ ] T8 `tree-parity alloc` vs PoB, corpus run, `docs/tree/tree-report.md`
- [ ] P6 Security: advisories and ignores, secret scan, fuzz and properties, capability check, `docs/phase-5/security.md`
- [ ] P7 Performance: `tools/bench`, baselines, instruction-count CI check, `docs/phase-5/performance.md`
- [ ] P8 Reports, handoff, CLAUDE.md, ACTION-PLAN, AGENTS.md, README (notice, install, privacy), task docs
- [ ] `code-architecture-reviewer` before marking the PR ready
- [ ] Mark the PR ready for review

# POE Builder Successor: Complete Implementation Plan

> **Revised 2026-11-08:** `docs/plan/ACTION-PLAN.md` and the revised ADRs supersede this document where they conflict (sequence, authority, data source, addons, governance, first milestone).

## Scope and execution rule
This plan covers the successor from repository foundation through PoE1 maturity and later PoE2 enablement. Work may proceed in parallel only where dependencies permit. A phase gate must pass before dependent work is promoted. This is a plan only; it does not bootstrap a repository.

## Phase map
- Phase 0: Foundation and oracle bootstrap
- Phase 1: Legacy-oracle feasibility spike
- Phase 2: Canonical data and model foundation
- Phase 3: Rust vertical parity core
- Phase 4: Tauri/React vertical parity slice
- Phase 5: Cross-platform, packaging, update, and security validation
- Phase 6: Progressive PoE1 feature expansion
- Phase 7: Successor beta and staged adoption
- Phase 8: PoE2 game-specific implementation

## Cross-cutting rules
- Current application and successor are independently governed.
- Frozen milestone oracle is the release gate; latest legacy release is informational.
- Calculation/data compatibility, migration safety, and security are absolute blockers.
- Every discrepancy becomes a minimized fixture, disposition, and regression test.
- Unknown content is preserved and unsupported mechanics are visible.
- No phase may silently change an accepted ADR.

## Phase 0: Foundation and oracle bootstrap
### Depends on
None.
### Work packages
1. Create the separate successor repository outside the legacy checkout, only when authorized.
2. Pin Rust, Node/pnpm, Tauri, frontend, and platform toolchains.
3. Create Cargo workspace boundaries and empty contract/tooling packages.
4. Configure protected main, GitHub Flow, CODEOWNERS, ADR process, dependency/license policy, and secret scanning.
5. Configure CI for formatting, linting, unit tests, frontend checks, and Windows/Linux smoke builds.
6. Define contract versioning, fixture manifests, artifact hashes, and reproducible-build metadata.
7. Document public legacy-source/release acquisition without assuming cooperation.
### Gate
A clean checkout builds on Windows and Linux; governance checks pass; no legacy repository is modified; the repository can record oracle metadata.
### Outputs
Repository scaffold, CI, ADR index, contributor guide, contract conventions, fixture manifest schema, oracle acquisition instructions.

## Phase 1: Legacy-oracle feasibility spike
### Depends on
Phase 0 toolchain and CI.
### Work packages
1. Pin one public legacy source commit/release and record hashes.
2. Validate native headless Lua/LuaJIT execution on Windows and Linux.
3. Implement a minimal Lua adapter for XML input and normalized JSON output.
4. Implement the Rust parity CLI for process launch, metadata, timeouts, logs, and artifacts.
5. Add a small stratified PoE1 smoke corpus.
6. Normalize build state, summary values, warnings, supported/unsupported content, and available breakdown data.
7. Add repeated-run determinism checks and one intentional discrepancy classification.
8. Run selected packaged-runtime checks separately.
### Gate
Layered threshold passes: oracle runs on both platforms, normalized output is stable, smoke reports are valid, an intentional difference is detected/classified, and environment errors are distinguishable from behavior differences.
### Outputs
Versioned oracle adapter contract, Rust CLI, smoke fixture manifest, parity reports, platform instructions, feasibility decision.
### Blockers
If native headless execution is not reproducible, stop and adapt the oracle strategy before building the Rust calculation core.

## Phase 2: Canonical data and model foundation
### Depends on
Phase 1 oracle contract and Phase 0 schemas.
### Work packages
1. Define versioned Rust persistence DTOs and generate JSON Schema/TypeScript types.
2. Define Build aggregate, IDs, value objects, substructures, unknown-content containers, and completeness states.
3. Define independent canonical game-data schema and signed bundle manifest.
4. Build converter from legacy exporter output to canonical bundles.
5. Implement structural, referential, domain, index, and smoke validation.
6. Implement XML/share import adapters and semantic export adapter.
7. Implement sequential migrations and migration reports.
8. Implement file-first JSON persistence and SQLite index/cache boundaries.
### Gate
Representative builds round-trip semantically, unknown fields are preserved, invalid bundles are rejected before signing, and DTO/domain conversion is deterministic.
### Outputs
Schemas, DTOs, migrations, data converter, bundle validator, XML adapters, persistence contracts.

## Phase 3: Rust vertical parity core
### Depends on
Phase 2 canonical model/data and Phase 1 differential runner.
### Work packages
1. Implement single-writer application task and command/query/event contracts.
2. Implement command history, snapshots, autosave/recovery hooks, and versioned projections.
3. Implement immutable BuildSnapshot calculation input.
4. Implement core offense/defense calculation phases with f64 and typed canonicalization.
5. Implement provenance graph and completeness propagation.
6. Implement background jobs, cancellation, stale-result rejection, and structured errors.
7. Add representative attack/spell paths and targeted minion, DoT, conversion, reservation, charge, party, jewel, and condition fixtures.
8. Run differential, golden, property, migration, and semantic round-trip tests.
### Gate
Core results and supported/unsupported behavior meet layered parity rules across the broad engine corpus; command/snapshot determinism and provenance are validated.
### Outputs
Rust domain/calculation/application crates, parity output adapters, provenance model, calculation reports.

## Phase 4: Tauri/React vertical parity slice
### Depends on
Phase 3 command/query/event API and Phase 2 persistence/import contracts.
### Work packages
1. Implement Tauri capabilities with minimal permissions and user-mediated file access.
2. Generate and consume TypeScript bindings.
3. Implement Redux projections and workflow state machines.
4. Build import/build-list entry, dashboard, provenance viewer, and workspace layout.
5. Implement layered item editor: existing item display, modifier status, replace/remove, supported roll edits, comparison, undo/redo.
6. Add XML-first import and character-import adapter with explicit partial/offline states.
7. Implement save/reopen/migrate/export workflow.
8. Add keyboard, focus, scaling, contrast, screen-reader, and error-state tests.
### Gate
A user can import, inspect, edit an item, observe asynchronous recalculation, save successor JSON, and export semantic legacy XML on both target platforms.
### Outputs
Tauri shell, React workspace, item workflow, generated bindings, accessibility tests, end-to-end reports.

## Phase 5: Cross-platform, packaging, update, and security validation
### Depends on
Phase 4 vertical slice.
### Work packages
1. Build Windows installer/portable and Linux AppImage/.deb artifacts.
2. Implement prompted application updates and signed data-bundle updates.
3. Implement offline/strict-offline behavior, rollback, quarantine, and pinned-data selection.
4. Establish offline-root and delegated signing-key procedures in a secure release environment.
5. Run dependency, license, secret, static, fuzz, parser, capability, signature, downgrade, and recovery checks.
6. Run startup, import, calculation, item-edit, tree-projection, memory, and package smoke benchmarks on reference tiers.
7. Validate structured logs, redaction, diagnostic-package preview, and crash recovery.
### Gate
Installers and portable packages work on Windows/Linux; update, signature, and rollback checks pass; security blockers are clear; performance and accessibility thresholds are documented and met or have approved exceptions.
### Outputs
Release artifacts, update manifests, signing runbooks, security reports, performance baselines, diagnostics runbook.

## Phase 6: Progressive PoE1 feature expansion
### Depends on
Phase 5 release-quality baseline and Phase 3 core boundaries.
### Work packages
1. Expand skills, support gems, skill groups, and main-skill workflows.
2. Add passive-tree projections, hybrid renderer, search, allocation, paths, jewels, and non-visual workflows.
3. Expand item editing, crafting, bases, influences, sockets, links, and advanced comparisons.
4. Expand configuration, conditions, charges, reservations, party effects, minions, DoT, traps/mines, totems, and support builds.
5. Add notes, build comparison, import variants, share-code compatibility, and search/index improvements.
6. Promote every supported feature through corpus, parity, accessibility, and performance gates.
### Gate
The agreed PoE1 supported feature scope has parity evidence, migration coverage, user workflows, and documented unsupported boundaries.
### Outputs
Progressive PoE1 releases, expanded corpus, feature ADRs, migration and compatibility reports.

## Phase 7: Successor beta and staged adoption
### Depends on
Phase 6 PoE1 scope and all release-quality gates.
### Work packages
1. Publish a separate successor beta without implying control over the current project.
2. Maintain compatibility adapters and diagnostic export.
3. Collect user-controlled reports and add minimized discrepancies to the corpus.
4. Run latest-legacy informational comparisons and milestone baseline promotion reviews.
5. Publish independent support policy, data support window, and migration documentation.
6. Define successor release criteria; do not define deprecation for the other project.
### Gate
Beta users can complete core workflows safely, compatibility status is visible, rollback and recovery are proven, and support ownership is explicit.
### Outputs
Successor beta releases, support policy, migration guides, compatibility dashboard.

## Phase 8: PoE2 game-specific implementation
### Depends on
Mature PoE1 shared infrastructure, explicit game-version boundaries, and a new PoE2 oracle/data baseline.
### Work packages
1. Define the PoE2 data and behavior baseline independently.
2. Extend shared infrastructure only where concepts are genuinely common.
3. Implement PoE2 game-specific rules, schemas, imports, and data bundles.
4. Add PoE2 corpus and differential tests.
5. Add UI game selection and game-specific capability reporting.
### Gate
PoE2 has its own parity corpus and release gates; PoE1 behavior remains unchanged by unproven PoE2 code.

## Ongoing workstreams
- Maintain ADRs and contract versions.
- Promote legacy baselines only at milestones.
- Add every discrepancy as a minimized regression fixture.
- Review security, privacy, accessibility, performance, and data freshness continuously.
- Keep the current project independent in governance and support policy.

## Stop and adapt conditions
Stop and revisit the plan if the legacy oracle cannot run reproducibly, the XML/share contract cannot be preserved, data access changes, signed bundle trust cannot be operated safely, or a proposed change conflicts with independent-project governance.

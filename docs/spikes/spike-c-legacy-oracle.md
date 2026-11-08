# Spike C: Legacy Oracle Harness (detailed specification)

> **Revised 2026-11-08:** `docs/plan/ACTION-PLAN.md` and the revised ADRs supersede this document where they conflict (sequence, authority, data source, addons, governance, first milestone).

## Objective
Prove that the independently governed successor can reproducibly execute a pinned public legacy implementation on Windows and Linux, load a small stratified PoE1 corpus, emit normalized machine-readable results, and detect/classify behavioral differences. This spike must not bootstrap the successor product or modify the legacy repository.

## Scope
In scope: public source/release acquisition, native headless execution, minimal Lua adapter, Rust CLI orchestration, normalized JSON, metadata, deterministic reruns, smoke fixtures, one intentional discrepancy, packaged-artifact check, and CI feasibility.

Out of scope: complete Rust calculations, full data conversion, production signing, full UI, PoE2, exhaustive corpus, and replacement of the legacy application.

## Preconditions
- A separate successor repository is authorized and available.
- Public legacy source/release and licensing terms permit the selected acquisition mode.
- Windows and Linux runners are available.
- A pinned PoE1 XML fixture and expected metadata are available.
- Runtime/library versions are recorded; no private service or cooperation is assumed.

## Deliverables
1. Pinned oracle manifest.
2. Minimal Lua adapter.
3. Rust parity CLI prototype.
4. Stratified smoke fixture manifest.
5. Normalized output schema.
6. Human-readable and machine-readable reports.
7. Windows/Linux run instructions.
8. Feasibility report and decision log.

## Pinned oracle manifest
Record: legacy repository URL, commit or release, source hash, packaged artifact hash if used, Lua/LuaJIT version, native libraries, data/tree versions, adapter revision, platform, launcher mode, and acquisition timestamp. The manifest must distinguish source-headless and packaged-artifact modes.

## Proposed executable layout
- `oracle/manifest.json` - pinned source, runtime, data, and platform metadata.
- `oracle/legacy-adapter/` - minimal Lua adapter and launch instructions.
- `oracle/fixtures/` - small smoke inputs and fixture metadata.
- `oracle/schema/oracle-output.schema.json` - normalized output contract.
- `tools/parity-cli/` - Rust CLI prototype.
- `reports/` - generated reports excluded from normal source history unless explicitly retained.

## Adapter contract
### Input
The CLI provides an input directory containing: fixture identifier, source build XML or supported input, requested game/data version, output path, timeout, and adapter protocol version.

### Lua adapter responsibilities
1. Start the pinned headless legacy runtime.
2. Load the legacy wrapper and required initialization.
3. Load one fixture.
4. Run one or more requested callbacks/queries.
5. Capture warnings, errors, unsupported content, and available breakdown data.
6. Emit exactly one versioned result document plus optional raw diagnostics.
7. Exit nonzero for launch/protocol failures, not for a valid partial calculation.

### Normalized output minimum
- Protocol and schema versions.
- Fixture ID and input hash.
- Oracle source/release/commit and runtime metadata.
- Platform and launcher mode.
- Game/data version.
- Normalized build state sufficient to separate import from calculation differences.
- Summary calculations and typed numeric metadata.
- Supported, unsupported, quarantined, warning, partial, and indeterminate states.
- Available breakdown/provenance records.
- Fatal error code or successful completion status.
- Duration and resource metadata where available.

## Rust CLI responsibilities
1. Validate manifest and fixture hashes.
2. Resolve a source checkout or packaged artifact without silently fetching unknown content.
3. Launch the adapter with an explicit timeout and environment.
4. Capture stdout, stderr, exit code, and artifacts.
5. Validate normalized output against the schema.
6. Normalize ordering and permitted representation differences.
7. Compare repeated runs for determinism.
8. Produce JSON report and human-readable Markdown/text report.
9. Classify failures as environment, protocol, import, calculation, normalization, or expected discrepancy.
10. Record all versions and hashes in the report.

## Fixture strategy
Use a small stratified set: a low-level/campaign build, an endgame build, different classes/archetypes, one imported/migrated case, and one known issue or edge case. Keep inputs small and reproducible. Every fixture records class, ascendancy, archetype, progression, data version, source, expected coverage, and issue links.

## Execution procedure
### Local Windows and Linux run
1. Verify toolchain and platform prerequisites.
2. Verify manifest and fixture hashes.
3. Acquire or locate the pinned oracle; never use an unrecorded latest checkout.
4. Run each fixture twice in native headless mode.
5. Validate both output documents against the schema.
6. Compare repeated outputs after normalization.
7. Generate a report per fixture and one aggregate report.
8. Run one deliberately modified input or expected-difference fixture.
9. Run a selected check against a packaged legacy artifact where available.
10. Archive manifest, logs, outputs, and report hashes.

### Suggested CLI shape
- `parity oracle verify-manifest --manifest oracle/manifest.json`
- `parity oracle run --fixture oracle/fixtures/<id> --mode native-headless`
- `parity oracle repeat --fixture <id> --runs 2`
- `parity oracle compare --left <report> --right <report>`
- `parity oracle report --input reports/ --format markdown,json`
- `parity oracle packaged-check --artifact <path> --fixture <id>`
Names are illustrative; the spike must document the final commands before implementation begins.

## Required test cases
1. Happy-path launch on Windows native headless.
2. Happy-path launch on Linux native headless.
3. Missing runtime dependency.
4. Wrong source/runtime hash.
5. Malformed fixture input.
6. Fixture timeout.
7. Legacy startup error.
8. Valid partial/unsupported calculation.
9. Stable repeated run.
10. Intentional input difference detected and classified.
11. Output schema violation.
12. Packaged artifact comparison.
13. Path/encoding differences across platforms.
14. Offline run with no network access.

## Failure taxonomy
- `ORACLE_ACQUISITION`: source/artifact unavailable or hash mismatch.
- `ORACLE_ENVIRONMENT`: missing runtime/library/platform prerequisite.
- `ORACLE_LAUNCH`: process failed to initialize.
- `ORACLE_TIMEOUT`: run exceeded configured limit.
- `ORACLE_PROTOCOL`: adapter output missing, duplicated, or malformed.
- `ORACLE_IMPORT`: fixture could not be loaded.
- `ORACLE_CALCULATION`: valid input produced an application error.
- `ORACLE_NORMALIZATION`: output cannot be canonicalized.
- `ORACLE_NONDETERMINISM`: repeated valid runs differ unexpectedly.
- `ORACLE_EXPECTED_DIFFERENCE`: known discrepancy reproduced.
- `ORACLE_UNEXPECTED_DIFFERENCE`: behavior differs without disposition.

Environment failures must never be reported as parity passes or calculation discrepancies.

## Pass/fail gate
The spike passes only when:
- Native headless execution works on Windows and Linux with pinned metadata.
- At least the stratified smoke fixtures run successfully.
- Two repeated runs produce equivalent normalized outputs.
- Reports contain all source/runtime/data/platform hashes and versions.
- The CLI distinguishes environment/protocol failures from behavior differences.
- At least one intentional discrepancy is detected and classified.
- The normalized schema validates all successful outputs.
- Offline execution succeeds without hidden network dependency.
- A packaged-artifact check is either successful or has a documented platform limitation and follow-up.

## Feasibility decision outcomes
- `FEASIBLE_NATIVE`: native headless mode is reproducible and becomes the routine oracle.
- `FEASIBLE_DUAL_MODE`: native mode is reliable and packaged checks are retained as a separate release validation.
- `FEASIBLE_WITH_ADAPTATION`: a documented runtime/adapter change is required before Phase 2.
- `BLOCKED`: no reproducible oracle; stop Rust parity implementation and adapt the strategy.

## Evidence retention
Retain the oracle manifest, adapter revision, fixture manifest, input hashes, normalized outputs, raw diagnostics when permitted, CLI version, platform details, report hashes, and the final feasibility decision. Do not copy legacy source into the successor repository unless licensing permits it.

## Security and privacy
Run the oracle with no unnecessary network access. Do not include credentials, private paths, or unrelated user data in fixtures. Treat imported builds as project test data but remove account tokens and machine-specific metadata.

## Completion report template
- Decision:
- Oracle source/release and hashes:
- Runtime versions:
- Platforms tested:
- Fixtures tested:
- Determinism result:
- Intentional discrepancy result:
- Packaged-artifact result:
- Known limitations:
- Required adaptations:
- Next unblocked phase:

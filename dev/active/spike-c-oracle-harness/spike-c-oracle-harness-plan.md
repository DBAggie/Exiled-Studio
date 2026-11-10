Last Updated: 2026-11-10 (PR C2b: step 2b done; adapter protocol 2)

# Spike C: thin oracle harness (plan)

## Summary
Spike C in `docs/plan/ACTION-PLAN.md` (Phase 1, third spike; about two weeks maximum). Prove that a Rust CLI can drive the legacy `HeadlessWrapper.lua` of PoB **v2.67.2** on a pinned LuaJIT, on Windows and Linux, and emit normalized JSON of `mainOutput` plus selected breakdowns that is identical across repeated runs. Failures of the environment (runtime missing, wrong hash, timeout, adapter protocol) must be reported apart from behavior differences. The spike also defines the divergence register format (ADR-006). Phase 3 (calc slice) compares against this oracle; Phase 2 does not need it.

Authority: the ACTION-PLAN and ADR-006 define the spike. `docs/spikes/spike-c-legacy-oracle.md` is the older, longer specification; its banner says the ACTION-PLAN wins where they conflict. This plan takes from it the failure taxonomy, the manifest contents, and the required test cases, and leaves out what the ACTION-PLAN gate does not ask for (packaged-artifact mode; see "Out of scope").

## Current state (2026-11-10)
- Spike A and Spike B are done (both gates met; task docs in `dev/completed/`).
- `tools/parity-cli` (`loom-parity-cli`) has the Spike B subcommands, hand-rolled argument matching in `main.rs`, and depends on `loom-data`, `loom-parser`, `loom-serialization`, `roxmltree`, `serde_json`, `sha2`. No oracle code exists.
- No LuaJIT on the workstation (ACTION-PLAN step 0 deferred it to this spike). `local/pob-v2.67.2/` holds only a `src/` copy used by Spike B's `render-check`, not a git checkout, so it cannot prove its own commit.
- The legacy working tree (`D:\POE Dev\PathOfBuilding`, read only) is at `v2.67.2-122-g3dbca4463`, **past the pin**. Its `HeadlessWrapper.lua` differs from the tag's (the tag has no `_SimpleGraphic.def.lua`). Only `v2.67.2:` paths are evidence.
- Facts measured while planning (context file, "Facts found while planning"): tag `v2.67.2` = commit `b32759ab0f31a1c8499a0d420cb0f0633d4fe478`; `Inflate` is a headless stub (returns `""`), so timeless jewel data (zlib-wrapped `.zip.partN` files) does not load, and **9 of the 21 corpus builds hold a timeless jewel**; `Modules/Common.lua` requires the C module `lua-utf8`; PoB's own test image pins LuaJIT commit `2460b3ff93a1c955de3d62cfc825de7d68dc272e` and `luautf8 0.1.6-1`.

## Target state
- `tools/oracle/` (new, non-Rust, MIT, our own code):
  - `manifest.json`: PoB repository URL, tag, commit, expected git tree hash of the checkout; LuaJIT repository, commit, build defines, expected `jit.version` and `jit.security` values; `luautf8` source, version, commit, module name; adapter protocol version. **Binary hashes are not in the manifest** (C1 measured that MSVC builds embed the build path, so hashes differ per machine): the acquire scripts record them in `local/oracle/build.json`, `oracle-verify` checks the files against it and probes the runtime's version and flags (context file, Decisions).
  - `adapter/oracle.lua`: the minimal Lua adapter (protocol v1).
  - `schema/oracle-output.schema.json`: normalized output contract.
  - `fixtures/`: fixture manifest and synthetic fixtures (our own XML, committed).
  - `acquire.ps1`, `acquire.sh`: maintainer and CI scripts that clone PoB at the tag and LuaJIT and `luautf8` at their commits into `local/oracle/`, and build them. These are the only steps that use the network; the CLI never fetches (ADR-011 spirit, spec "Rust CLI responsibilities" 2).
- `loom-parity-cli` subcommands (all offline):
  - `oracle-verify <manifest> <oracle-dir>`: checks the PoB checkout's commit and tree hash, a clean tree including ignored files (`git status --porcelain --ignored`), and the runtime hashes. Exit codes per failure class.
  - `oracle-run <manifest> <oracle-dir> <fixtures> --out <dir> [--runs N] [--timeout S]`: runs each fixture N times (default 3), validates every output against the schema, normalizes, compares runs, writes `report.json` and `summary.md`.
  - `oracle-compare <left> <right> [--register <file>] --out <dir>`: compares two normalized outputs (or two reports) field by field and classifies each difference as expected (registered) or unexpected.
  - `divergence-check <register>`: validates the divergence register against its schema.
- `docs/parity/divergence-register.json` (empty `entries`) and `docs/parity/divergence-register.schema.json`, with the format documented in `docs/parity/README.md`.
- CI: an `oracle` job on `ubuntu-latest` and `windows-latest` that acquires and builds the pinned runtime (cached by commit), runs `oracle-verify`, then `oracle-run --runs 2` on the public fixtures, offline.
- Spike report `docs/spikes/spike-c-oracle.md` with the gate verdict and the feasibility decision (`FEASIBLE_NATIVE`, `FEASIBLE_WITH_ADAPTATION`, or `BLOCKED`); committed summaries in `docs/spikes/spike-c/`.

## Design constraints and decisions
- **Legacy code is run, never copied (ADR-006, AGENTS.md).** The checkout lives in `local/oracle/pob/` (ignored). Our adapter is original code. It may call PoB's globals and functions; it never patches PoB files. `oracle-verify` fails if the checkout's tree differs from the manifest (a stale `.bin` cache or any edit counts as a difference; see the next point).
- **The adapter drives `HeadlessWrapper.lua`, as the ACTION-PLAN says.** `oracle.lua` sets `package.path`, then `dofile("HeadlessWrapper.lua")` with the working directory at `<checkout>/src` (the wrapper's `GetScriptPath()` returns `""`), then loads one fixture with the wrapper's own `loadBuildFromXML`, then reads `build.calcsTab.mainOutput` (the same path PoB's `spec/GenerateBuilds.lua` uses). It writes exactly one JSON document to a file named by the CLI, never to stdout, so PoB's prints cannot corrupt the protocol.
- **Headless shims are adapter-owned, named, and recorded.** The tag's wrapper stubs `Inflate` (returns `""`, `v2.67.2:src/HeadlessWrapper.lua`). The jewel loader reads the `.zip.partN` files with `io.open` and passes them to `Inflate` (`v2.67.2:src/Modules/DataJewelFileLoader.lua`), so with the stub timeless jewels compute as if empty: a harness artifact, not PoB behavior. Decision (to confirm with the maintainer, open question 1): the adapter replaces the global `Inflate` with a zlib binding through LuaJIT FFI **after** `dofile("HeadlessWrapper.lua")` and before any fixture loads. That is enough: jewel data loads only when a build is loaded (`v2.67.2:src/Modules/Data.lua` creates an empty `data.timelessJewelLUTs`; `PassiveSpec.lua` reads it through `data.readLUT`), and the loader looks `Inflate` up as a global at call time. The zlib library is pinned and recorded (Windows: the checkout's `runtime/zlib1.dll`, loaded in place, not copied; Linux: the system `libz` with version and hash, or a pinned build). The wrapper's `NewFileSearch` stub (returns nil) **stays**: the loader uses it only to compare file times with a `.bin` cache, and with the stub it always reads the compressed parts, which is deterministic; a working `NewFileSearch` would let a leftover `.bin` win by file time. Every shim is listed in the manifest (`shims[]`) and in every output document; a jewel probe in the output shows the shim ran. A build whose result depends on a shim that failed is `partial`, never `ok`. Other stubs stay as the tag has them and are recorded as relied on: `GetTime` returns 0 (it is also why `Launch.lua` skips the update check).
- **The checkout stays clean.** The loader is called without `cacheUncompressed` (`v2.67.2:src/Modules/DataLegionLookUpTableHelper.lua`), so it defaults to true and writes `src/Data/TimelessJewelData/*.bin` after `Inflate` (mode `"wb+"`); even the bare wrapper writes an empty `.bin`. PoB's `.gitignore` ignores these files, so a plain clean-tree check misses them. Decision: a second named shim wraps `io.open` and refuses write modes for paths inside the checkout (the loader ignores a failed open), and `oracle-verify` uses `git status --porcelain --ignored` and fails on any untracked or ignored file in the checkout's `src/`. A tier-1 timeless-jewel fixture checks the tree after the run in CI. Alternative if the guard breaks something: run on a fresh copy each time.
- **Startup errors.** On a startup error the wrapper prints `promptMsg`, calls `io.read` (stdin is closed, so it returns at once), and `return`s from `dofile`; the adapter continues with `build` nil. So after `dofile` the adapter checks `launch.promptMsg` (`launch` is a global; `mainObject` is local to the wrapper, found in C1) and `build` and writes a startup-stage result, which the CLI classifies `ORACLE_LAUNCH`.
- **Environment is explicit.** The CLI starts `luajit` with a cleared environment plus an allow-list: `LUA_PATH`, `LUA_CPATH`, and on Windows `SystemRoot` and a minimal `PATH` (the CRT and DLL loading need them). The allow-list is recorded in each report. `CI` in particular is unset (the wrapper sets `continuousIntegrationMode` from it, which skips `ModCache`), so local and CI runs take the same code path. The working directory is `<checkout>/src`: with `GetScriptPath()` returning `""`, the jewel loader probes `./Data/...` first, which is right only from there. `GetUserPath()` returns `""`, so PoB looks for `/Path of Building/Settings.xml`; the CLI runs with a temporary working root and records whether a settings file was found (it must not be).
- **No network.** `lcurl.safe` is already stubbed by the wrapper. The CI job runs the oracle step with networking removed (`unshare -rn` on Linux; on Windows a firewall rule is not available on hosted runners, so Windows records the limitation and relies on the Linux result plus the adapter's `require` log).
- **Normalization (the open risk named in ADR-006).** `mainOutput` is a nested Lua table with numbers, booleans, strings, and tables. The adapter writes it with sorted keys and numbers as `%.17g` (exact round trip of a double); `inf`, `-inf`, and `nan` become tagged strings; functions and cycles become a typed placeholder with the path, never dropped silently. The schema allows exactly these shapes. "Selected breakdowns": C2a measures which `calcsTab` breakdown tables exist for the fixtures and picks a first set (candidates: `breakdown` entries for `TotalDPS`, `Life`, `EnergyShield`); the selection is recorded in the manifest so it is versioned.
- **Determinism means byte-identical normalized outcome per platform.** The outcome is the whole result of a run: the normalized output, or the failure class and stage. Repeated runs of one fixture on one platform must match exactly (3 runs by default). A fixture that fails the same way every time is deterministic but reported with its class, never as `ok`. LuaJIT 2.1 has build options that randomize string hashing (`LUAJIT_SECURITY_STRHASH`, `LUAJIT_SECURITY_PRNG`), which would change `pairs` order and with it float summation order; C1 checks their defaults at `2460b3ff` (`lj_arch.h`), builds with randomization off if it is on, and records the build flags in the manifest. Windows vs Linux outputs are compared and every difference is reported and classified (float formatting, path separators, ordering), but cross-platform equality is not gated: the ACTION-PLAN gate asks for deterministic repeated runs on each platform. A cross-platform difference becomes a divergence-register candidate if it is real.
- **Failure taxonomy** from the spec, used as exit codes and report fields: `ORACLE_ACQUISITION`, `ORACLE_ENVIRONMENT`, `ORACLE_LAUNCH`, `ORACLE_TIMEOUT`, `ORACLE_PROTOCOL`, `ORACLE_IMPORT`, `ORACLE_CALCULATION`, `ORACLE_NORMALIZATION`, `ORACLE_NONDETERMINISM`, `ORACLE_EXPECTED_DIFFERENCE`, `ORACLE_UNEXPECTED_DIFFERENCE`. Only the CLI assigns a class; the adapter reports facts (stage reached, Lua error text, PoB `promptMsg`). An environment-class failure is never counted as a pass or as a calculation difference (spec, "Failure taxonomy").
- **Fixtures (scrubbing, ADR-007).** Three tiers:
  1. *Public, in CI:* PoB's own `spec/TestBuilds/3.13/*.xml` (5 builds), read from the pinned checkout by path and blob hash (not copied), plus synthetic fixtures we write (a minimal level-1 build, one malformed XML, and a minimal build with one timeless jewel, so CI covers the `Inflate` shim and the no-write check). PoB's 3.13 builds are marked `old-tree` in the fixture manifest (PoB converts their tree on load). Timeout, protocol violations, startup errors, and nondeterminism are tested with separate fake adapter scripts that the Rust tests launch; `oracle.lua` holds no test code.
  2. *Corpus, local only:* the maintainer's 21 builds, keyed by content hash, run on the workstation. Only summaries (hashes, counts, classes) are committed, as in Spike B.
  3. *Intentional discrepancy:* a synthetic fixture pair whose second member changes one input (for example a gem level), so `oracle-compare` must report an `ORACLE_UNEXPECTED_DIFFERENCE` on the expected fields, and then `ORACLE_EXPECTED_DIFFERENCE` once a test register entry covers it.
  Corpus builds are not committed for CI (open question 2, maintainer 2026-11-10).
- **Divergence register (ADR-006):** one versioned JSON file. Each entry: `id`, `mechanic`, `fixture` (fixture id or content hash), `field` (normalized output path), `comparison` (`exact`, or `abs` / `rel` with a tolerance, so tolerances are field-specific and written down, ADR-006), `oracle_value`, `successor_value`, `kind` (`game-truth`, `pob-bug`, `data-version`, `harness`), `evidence` (links or citations; game evidence per ADR-001), `test` (the test that pins it), `added`, `status`. Unregistered differences fail a parity gate; Spike C only defines the format, the validator, and the comparison hook. `oracle-compare` compares exactly unless a register entry names a tolerance for that field; default tolerances per field family are Phase 3 work, when there is a successor value to compare (oracle-vs-oracle runs must match exactly). The register is in `docs/parity/` because it is a reviewed project record, not tool data.
- **Rust side stays in `tools/parity-cli`** (AGENTS.md: "maintainer-run oracle and corpus harnesses"). It may use `std::process`, files, and the clock; no core crate changes. The output schema is small, so the check is written by hand in Rust against it (no JSON-schema crate); the `.schema.json` file documents the contract. Any new dependency (possibly `wait-timeout`) goes through `cargo deny` in the PR that adds it.
- **PoE2:** out of scope (no PoE2 oracle; the spec says so).

## Steps

### 1. Pinned runtime and manifest (S-M)
`tools/oracle/manifest.json`, `acquire.ps1` / `acquire.sh`, `parity-cli oracle-verify`. Build LuaJIT at `2460b3ff` (Windows: `msvcbuild.bat` from a VS developer shell; Linux: `make`) and `luautf8 0.1.6` against it, into `local/oracle/`. Check the hash-randomization build options at `2460b3ff` and record the build flags. One line in AGENTS.md exempts maintainer and CI acquire scripts (outside the crates) from the network rule. Smoke: `luajit -e "print(jit.version)"`, and the bare wrapper starts and reaches `OnFrame` (by hand, recorded in the context file; no adapter yet).
- Accept: `oracle-verify` passes on Windows; fails with `ORACLE_ACQUISITION` on a wrong commit or a dirty tree and with `ORACLE_ENVIRONMENT` on a missing or wrong `luajit` (unit tests with temporary directories); the build is repeated once and the binary hashes are compared (recorded either way).
- Depends on: nothing. Maintainer approval for the local builds (no system install).

### 2a. Adapter, protocol v1, `oracle-run`, no shims (M)
`oracle.lua`, the output schema and its hand-written check, `oracle-run` (launch with the env allow-list and timeout, capture stdout and stderr, schema check, normalization, repeats, `report.json`). Measure the breakdown selection.
- Accept: the 5 PoB test builds and the minimal synthetic build run 3 times on Windows with byte-identical outcomes; every required test case that is local (spec list 3-11, 13, 14) has a test (Rust unit tests with fake adapter scripts for the classification, plus real-runtime runs recorded in the context file).
- Depends on: 1.

### 2b. Shims: `Inflate`, write guard, jewel probe (S-M)
The FFI `Inflate` with pinned zlib, the `io.open` write guard, the jewel probe, the ignored-files tree check after runs.
- Accept: the synthetic timeless-jewel fixture and the 9 timeless-jewel corpus builds show jewel data loaded (probe); no file is written into the checkout (tree check, test); outcomes of the 2a fixtures without jewels are unchanged.
- Depends on: 2a.
- Done in C2b (context file, "Decisions in C2b"). Changes from this plan: the document gained `shims` and `probe`, so the adapter protocol is 2 and whole-document hashes changed; "unchanged" was checked on the computed data (`main_output`, `breakdowns`), byte-identical for all 6 C2a fixtures. The write guard refuses every write, not only paths in the checkout. A `partial` fixture makes the report `partial` (exit 3).

### 3. Fixtures, `oracle-compare`, divergence register (M)
Fixture manifest (tiers 1-3), `oracle-compare`, register schema, `divergence-check`, `docs/parity/README.md`. The 21 corpus builds run twice locally.
- Accept: the intentional discrepancy is reported as unexpected, then expected with a test register; `divergence-check` accepts the empty register and rejects malformed entries (tests); corpus summary committed with no build names or item text (scrub test as in Spike B).
- Depends on: 2b.

### 4. CI on Windows and Linux (M)
`oracle` job in `ci.yml` (matrix `ubuntu-latest`, `windows-latest`), runtime cached by commit hashes, Linux run without network, Windows vs Linux outputs compared and reported.
- Accept: green on both runners; the job fails on an injected wrong hash (one-off branch run, linked); the job is informational (not in `ci-ok`); C5 makes it required after 10 consecutive green runs on both runners (open question 4, maintainer 2026-11-10).
- Depends on: 3 (fixtures) and 2b.

### 5. Report and gate verdict (S-M)
`docs/spikes/spike-c-oracle.md` with the completion report template from the spec, the gate verdict, the feasibility decision, known limitations (packaged mode, Windows network isolation), and the next unblocked phase. ACTION-PLAN status, handoff.
- Depends on: 1-4.

## Gate (objective, from the ACTION-PLAN)
Each item is checkable from committed reports or CI:
1. **Deterministic repeated runs on Windows and Linux:** every tier-1 fixture runs 3 times in the CI `oracle` job on both runners with byte-identical outcomes (normalized output, or failure class and stage), and every corpus build runs 3 times on the Windows workstation with byte-identical outcomes (`report.json` field `determinism.identical == runs`). Fixtures with a consistent failure are listed with their class and do not count as `ok`.
2. **Environment failures distinguished from behavior differences:** each failure class that the CLI can produce has a test that injects it and asserts the class (missing runtime, wrong hash, dirty checkout, timeout, startup error, malformed fixture, protocol violation, schema violation, nondeterminism by a fake adapter), and none of them is reported as `ok` or as a difference. The intentional discrepancy is classified as unexpected, then expected with a register entry.
3. **Divergence register format defined:** schema, empty register, validator, and documentation committed; `divergence-check` runs in CI.
4. **Recorded inputs:** every report holds the manifest hash, PoB commit and tree hash, LuaJIT commit and binary hash, `luautf8` version and module hash, adapter protocol version and file hash, shims, zlib version and library hash, LuaJIT build flags, environment allow-list, platform, fixture hashes, the CLI's commit and dirty flag.

## Fallback
- If the bare runtime cannot be built on one platform: use PoB's shipped Windows `runtime/lua51.dll` with a `luajit.exe` built against it (packaged-runtime mode) and record it as `FEASIBLE_WITH_ADAPTATION`.
- If shims cannot be made to work: timeless-jewel builds become `partial` with a named reason, the remaining corpus is the oracle set, and the report says so (`FEASIBLE_WITH_ADAPTATION`).
- If repeated runs differ and the cause cannot be removed in the adapter: `BLOCKED` for the affected fields; the report lists them and the maintainer decides whether Phase 3 compares only the stable fields.

## Out of scope (named so the gap is visible)
- Packaged-artifact mode (spec deliverable; not in the ACTION-PLAN gate). The report records it as a follow-up with the reason.
- PoE2, the in-app use of the oracle, PoB's character-JSON import path.
- Comparing the successor's results: there is no successor calc yet (Phase 3).

## Risks and mitigations
| Risk | Mitigation |
|---|---|
| `mainOutput` is not a stable contract (ADR-006) | Exact number encoding, sorted keys, placeholders for non-data values, schema; the selection of breakdowns is versioned in the manifest. |
| Headless stubs change results (timeless jewels, 9 of 21 builds) | Named `Inflate` shim, recorded per output, probed; failed shim gives `partial`; `NewFileSearch` stub kept so no `.bin` cache can win by file time. |
| LuaJIT string-hash randomization changes `pairs` order between runs | Build flags checked and recorded in C1; 3 repeats. |
| PoB writes into its checkout (`.bin` cache, ignored by its `.gitignore`) or reads user files (`Settings.xml`) | `io.open` write guard shim; tree check including ignored files; temporary working root; settings probe in the output. |
| `CI` env var changes PoB's code path | Cleared environment. |
| LuaJIT or `lua-utf8` build is not reproducible byte for byte | Source commit is the pin; binary hash recorded per run; a change between runs is `ORACLE_ENVIRONMENT`. |
| Windows build needs the MSVC developer environment in CI | Use `vswhere` to find `vcvars64.bat` (no third-party action); fallback is a pinned action, reviewed. |
| PoB prints to stdout or blocks on `io.read` after a startup error | Protocol output goes to a file; stdin is closed, so `io.read` returns at once; startup errors become `ORACLE_LAUNCH` with `promptMsg`. |
| Personal data from the corpus leaks into committed reports | Content hashes only; scrub test as in Spike B. |
| Floating-point differences across platforms read as nondeterminism | Determinism is per platform; cross-platform differences are reported and classified separately. |

## Planned PR breakdown
Each PR is one session and updates the handoff and this task folder (`/dev-docs-update`).

| PR | Content | Evidence |
|---|---|---|
| C0 | This plan; Spike B close-out (merges of #26-#32, folder to `dev/completed/`, ACTION-PLAN, CLAUDE.md, README). | Tests: None (docs). Parity: None. Coverage: none. |
| C1 | Step 1: manifest, acquire scripts, `oracle-verify`. | Unit tests per failure class; `oracle-verify` output on Windows; build-hash repeat result. |
| C2a | Step 2a: adapter, schema, `oracle-run`, no shims. | Unit tests (fake adapter scripts); 3-run outcomes for the public fixtures on Windows. |
| C2b | Step 2b: `Inflate` shim, write guard, jewel probe. | Tests; probe on the synthetic and corpus timeless-jewel builds; tree check. |
| C3 | Step 3: fixtures, `oracle-compare`, divergence register, corpus run. | Tests; intentional discrepancy report; corpus summary (scrubbed); `cargo deny`. |
| C4 | Step 4: CI `oracle` job. | CI runs on both runners; injected-failure run; cross-platform comparison. |
| C5 | Step 5: report, gate verdict, ACTION-PLAN status. | Committed summaries; gate pass/fail. |

Reviewed by `plan-reviewer` on 2026-11-10 (verdict: proceed with changes; 2 blockers, 11 should-fix, 4 nits, all applied above; summary in the context file).

# Spike C: thin oracle harness (report)

Status: **gate met** (2026-11-10, PR #39, C5). Decision: **`FEASIBLE_WITH_ADAPTATION`**: the adaptations are already made, documented, and recorded in every output; nothing blocks Phase 2 or Phase 3 (limitations in section 9).

Plan and session records: `dev/completed/spike-c-oracle-harness/` (plan, context, tasks). Older, longer specification: `docs/spikes/spike-c-legacy-oracle.md` (the ACTION-PLAN wins where they conflict).

## 1. Question and gate
Can a Rust CLI drive the legacy `HeadlessWrapper.lua` of PoB **v2.67.2** on a pinned LuaJIT, on Windows and Linux, and emit normalized JSON of `mainOutput` plus selected breakdowns that is identical across repeated runs, while keeping failures of the environment apart from differences in behavior?

ACTION-PLAN gate: deterministic repeated runs on Windows and Linux; environment failures distinguished from behavior differences; divergence register format defined. The plan made it checkable as four items (section 5).

## 2. What was built
| PR | Step | Content |
|---|---|---|
| #33 (C0) | plan | Plan, plan review, Spike B close-out. |
| #34 (C1) | 1 | `tools/oracle/manifest.json` (pins), `acquire.ps1` / `acquire.sh` (the only network steps), `loom-parity-cli oracle-verify`. |
| #35 (C2a) | 2a | Adapter `tools/oracle/adapter/oracle.lua` + `json.lua` (protocol 1), output schema, `oracle-run` (cleared environment, time limit, schema check, normalization, repeats, report). |
| #36 (C2b) | 2b | Shims `inflate` (zlib through LuaJIT FFI) and `write_guard`; jewel probe; clean-checkout check after each fixture (protocol 2). |
| #37 (C3, with #38 squashed into it) | 3, 3b, 4 | Fixture manifest, `oracle-compare`, divergence register and `divergence-check`; shim `variant_order` (protocol 3); 21 credited community builds (ADR-007 Amendment 1); CI jobs `oracle` (Linux, Windows) and `oracle cross-platform`. |
| #39 (C5, this PR) | 5 | This report; corpus evidence on Windows and Linux from `main`; the `oracle` jobs made required (section 7). |

All oracle code is in `tools/parity-cli` (Rust) and `tools/oracle/` (Lua adapter, manifest, schema, fixtures, scripts). No core crate changed. PoB is run from a verified checkout in `local/oracle/pob/` and never copied or patched (ADR-006).

## 3. Inputs (recorded in every report)
| Input | Value |
|---|---|
| PoB | `https://github.com/PathOfBuildingCommunity/PathOfBuilding.git`, tag `v2.67.2`, commit `b32759ab0f31a1c8499a0d420cb0f0633d4fe478`, tree `74b90d3b89994d6e738d3cb836e9c48162ca6b57`; checkout clean including ignored files, before and after every fixture |
| LuaJIT | commit `2460b3ff93a1c955de3d62cfc825de7d68dc272e` (PoB's own test image pin), `jit.version` `LuaJIT 2.1.1784580905`, define `LUAJIT_SECURITY_PRNG=0`; `jit.security` probed: `prng` 0, `strhash` 1, `strid` 1, `mcode` 1 |
| luautf8 | `0.1.6` (rock `0.1.6-1`), commit `1bb70d45208d4033dcc75efee968be70534dd3ab` |
| zlib | Windows: PoB's `runtime/zlib1.dll` in the verified checkout (zlib 1.3.1, `27f24e10…a5c8`); Linux: the system `libz.so.1`, hashed per run |
| Adapter | protocol 3; `oracle.lua`, `json.lua`, `shims.lua` hashed per report; shims `inflate`, `write_guard`, `variant_order` |
| Manifest | SHA-256 per report (`bb2d4876…6e2e` at `main` `aa940fd`) |
| Environment | cleared, allow-list `LUA_PATH`, `LUA_CPATH`, and on Windows `SystemRoot`; working directory `pob/src` |
| Binaries | SHA-256 per built file in `local/oracle/build.json`, checked by `oracle-verify` (per machine: MSVC and gcc embed the build path) |
| Tool | `loom-parity-cli` commit and dirty flag |

## 4. Fixtures
| Tier | Count | Where it runs | Pinned by |
|---|---|---|---|
| `pob-test` (PoB's `spec/TestBuilds/3.13`, `old-tree`) | 5 | CI, workstation | git blob id in the pinned checkout (CRLF on Windows, LF on Linux) |
| `synthetic` (minimal level 1, malformed XML, Militant Faith timeless jewel) | 3 | CI, workstation | SHA-256 |
| `discrepancy` (movement-speed pair partner) | 1 | CI, workstation | SHA-256 |
| `community` (credited public builds, 3.29; `tools/oracle/fixtures/community/SOURCES.md`) | 21 | CI, workstation | SHA-256 |
| Maintainer corpus (ADR-007) | 21 | workstation only, `--anonymous` | SHA-256 (summaries committed, builds never) |

The fixture manifest is `tools/oracle/fixtures/fixtures.json`; each fixture names its expected outcome (`ok`, or a failure class for `malformed.xml`).

## 5. Gate verdict
| # | Item | Verdict | Evidence |
|---|---|---|---|
| 1 | Deterministic repeated runs on Windows and Linux | **met** | CI: every fixture of `fixtures.json` 3 runs, identical, on `ubuntu-latest` and `windows-latest` in every completed run (section 7). Corpus: 21 builds x 3 runs, identical, on Windows and on Linux (WSL 2 Ubuntu 24.04, no network) from clean `main` `aa940fd` (`docs/spikes/spike-c/corpus-{windows,linux}-*`). Across platforms the computed data is identical too (section 6). |
| 2 | Environment failures distinguished from behavior differences | **met** | Each class has a test that injects it and asserts the class; none is reported as `ok` or as a difference (table below). The intentional discrepancy is `ORACLE_UNEXPECTED_DIFFERENCE` (exit 20) without a register and `ORACLE_EXPECTED_DIFFERENCE` (exit 19) with the test register. |
| 3 | Divergence register format defined | **met** | `docs/parity/divergence-register.schema.json`, empty `docs/parity/divergence-register.json`, `docs/parity/README.md`, `loom-parity-cli divergence-check`; runs in `cargo test` and as a step of the required `rust` job. |
| 4 | Recorded inputs | **met** | Every `report.json` holds the inputs in section 3 (`oracle.checks` holds the verified commits, tree, defines, binary hashes, and runtime probe). |

Item 2, by class (Rust tests in `tools/parity-cli/src/`):
| Class | Exit | Injected by |
|---|---|---|
| `ORACLE_ACQUISITION` | 10 | `oracle_verify` tests: wrong PoB commit, wrong tree, missing checkout, untracked, ignored, or modified file, wrong runtime source commit, malformed or unreadable manifest; `oracle-run` refuses to launch on an unverified oracle and stops on a dirty checkout after a fixture |
| `ORACLE_ENVIRONMENT` | 11 | `oracle_verify` tests: missing or changed `luajit`, missing or broken module, missing build record, other defines, platform, or version, string hashing randomized (`prng` 1); `oracle_run`: runtime does not start, settings file found, network module loaded, non-ASCII Windows path |
| `ORACLE_LAUNCH` | 12 | crash before output; startup error (`promptMsg`) by a fake adapter on the real runtime |
| `ORACLE_TIMEOUT` | 13 | fake adapter that sleeps (also after writing a complete document) |
| `ORACLE_PROTOCOL` | 14 | no document with exit 0, unreadable or non-JSON document, two documents, schema violation, exit code that disagrees with the stage, wrong protocol version, failed Inflate without `partial` |
| `ORACLE_IMPORT` | 15 | `malformed.xml` through the real adapter and PoB (also an expected outcome in every CI run) |
| `ORACLE_CALCULATION` | 16 | scripted document failing in stage `calculate` |
| `ORACLE_NORMALIZATION` | 17 | scripted document failing in stage `normalize`; the Lua writer's 39 cases (keys starting with `$`, table keys, invalid UTF-8, depth) |
| `ORACLE_NONDETERMINISM` | 18 | runs that differ; failure then success; fake adapter `flaky.lua` on the real runtime |
| `ORACLE_EXPECTED_DIFFERENCE` | 19 | `oracle_compare` tests and the real discrepancy pair with the test register |
| `ORACLE_UNEXPECTED_DIFFERENCE` | 20 | the discrepancy pair without a register; unpaired fixtures; an expected difference cannot hide an unexpected one |
| `partial` (not a class) | 3 | zlib not loadable (real runtime): the jewel fixture is `partial`, never `ok` |

The real-runtime tests (`#[ignore]`, fake adapters and the real adapter on PoB) run by hand with `LOOM_ORACLE_DIR` set (AGENTS.md); `cargo test` in CI runs the scripted ones.

## 6. Results
- **CI** (`oracle` jobs, 30 fixtures from the second community batch on, 20 before): `ok` 3/3 each and `malformed.xml` `ORACLE_IMPORT` 3/3 as expected; `oracle-run` exit 0 on both runners. `oracle cross-platform` (Windows vs Linux, `oracle-compare`): **identical** in every completed run (29 identical, plus `malformed.xml`, which fails with `ORACLE_IMPORT` on both sides as expected; PoB's fixtures carry `input_differs` for CRLF vs LF with identical output). Linux runs inside `unshare -rn` after checking that a TCP connect fails.
- **Corpus, Windows** (`main` `aa940fd`, clean): 21 `ok`, 3/3 each; every output hash equal to the committed C3b run from `2535abe`. The adapter changed since then only in comments and in a callback for a registered variant table with a non-string key, which no fixture reaches (`git diff 2535abe aa940fd -- tools/oracle/adapter/`). 5 builds load timeless-jewel data (up to 51,651,890 bytes inflated); 3 go through `variant_order`.
- **Corpus, Linux** (same commit, WSL 2 Ubuntu 24.04, `unshare -rn`): 21 `ok`, 3/3 each. Whole-document hashes differ from Windows (the zlib version and module paths are part of the document), but **`oracle-compare` Windows vs Linux: 21 identical, 0 differences** (`docs/spikes/spike-c/corpus-compare-*`).
- **Intentional discrepancy:** `minimal-level1.xml` vs `discrepancy-movement-speed.xml` differ in exactly `MovementSpeedMod` and `EffectiveMovementSpeedMod` (1 vs 1.1): exit 20 without a register, exit 19 with `tools/oracle/fixtures/discrepancy-register.json` (one `exact`, one `abs` entry).
- **Cross-check against a published number:** Maxroll's Winter Orb Elementalist page shows `TotalDPS` 219,114.9; the oracle's `CombinedDPS` for that build is 219,115.
- Run time: about 3 minutes for 21 corpus builds x 3 runs on the workstation; the CI `oracle` job takes 8 to 12 minutes per runner, a runtime cache miss included (the `main` run of #37 acquired and built the runtime on both runners in that time).

## 7. CI: the `oracle` jobs become required
The maintainer decided (2026-11-10) that the `oracle` job starts informational and becomes part of `ci-ok` after 10 consecutive green runs on both runners, counted from PR #38's runs on. Runs cancelled because a newer push superseded them are not results and do not break the count.

| # | Run | Commit | Event | `oracle` Linux / Windows | Cross-platform |
|---|---|---|---|---|---|
| 1 | [42888981382](https://github.com/DBAggie/Exiled-Studio/actions/runs/42888981382) | `ff59fcd` (PR #38) | dispatch | green / green | identical (20) |
| 2 | [42890650849](https://github.com/DBAggie/Exiled-Studio/actions/runs/42890650849) | `4d9102b` (PR #38) | pull_request | green / green | identical (20) |
| 3 | [42894655422](https://github.com/DBAggie/Exiled-Studio/actions/runs/42894655422) | `402001a` (PR #38) | pull_request | green / green | identical (30) |
| 4 | [42896631829](https://github.com/DBAggie/Exiled-Studio/actions/runs/42896631829) | `17edb76` (PR #37) | pull_request | green / green | identical (30) |
| 5 | [42898641516](https://github.com/DBAggie/Exiled-Studio/actions/runs/42898641516) | `aa940fd` (`main`) | push, runtime cache miss | green / green | identical (30) |
| 6-10 | [42900481125](https://github.com/DBAggie/Exiled-Studio/actions/runs/42900481125), [42902380634](https://github.com/DBAggie/Exiled-Studio/actions/runs/42902380634), [42904023279](https://github.com/DBAggie/Exiled-Studio/actions/runs/42904023279), [42905960826](https://github.com/DBAggie/Exiled-Studio/actions/runs/42905960826), [42907423940](https://github.com/DBAggie/Exiled-Studio/actions/runs/42907423940) | `aa940fd` (`main`) | dispatch, one at a time | green / green | identical (30) |

Not counted: 42890599670 and 42894633985 (cancelled by a newer push), the injected wrong-hash run 42889081685 (C4, a one-off branch), and the injected-difference run below. Runs 6 to 10 repeat one commit with a warm runtime cache, so they show that the run is stable, not that acquisition is; acquisition from scratch is shown by run 5 (and run 1, the job's first run).

With the count met, this PR adds `oracle` and `oracle-compare` (`oracle cross-platform`) to `ci-ok`'s `needs`, and `oracle-compare` exit 20 (an unregistered cross-platform difference) now fails the job (ADR-006: unregistered differences fail a parity gate). Exit 19 (only registered differences) passes. The compare job also no longer runs when the run was cancelled (`!cancelled()` instead of `always()`), so a superseded run no longer shows a red compare job (`ci-ok` of a cancelled run is still red, as before). When an `oracle` leg fails, the compare job still runs and fails too (a run is missing).

Exit 20 failing the job is shown by a one-off run on the branch `spike-c/c5-inject-difference` (never merged), which adds 1 to `main_output.Life` of `minimal-level1.xml` in the Linux output before the comparison: run [42901853806](https://github.com/DBAggie/Exiled-Studio/actions/runs/42901853806), `oracle-compare` "unexpected (ORACLE_UNEXPECTED_DIFFERENCE) ... 28 identical, 1 unexpected, 1 expected failures", exit 20; `oracle cross-platform` failed and so did `ci-ok`, with every other job green.

What a required oracle costs: the job runs on every PR, docs-only ones included (no path filter). On a runtime cache miss (new pins, or 7 days without use) it fetches PoB, LuaJIT and luautf8 from GitHub and builds them; the Linux leg needs `sudo sysctl` to allow user namespaces on `ubuntu-latest`, a floating image. A network failure or an image change can therefore block unrelated PRs. The maintainer accepted this with the 10-run rule; a red run caused by the runner (not the oracle's outcome) is cleared with a full `gh run rerun`.

## 8. Completion report (spec template)
- **Decision:** `FEASIBLE_WITH_ADAPTATION`. Native headless mode is reproducible and is the routine oracle, but only with the adaptations below; they are done, named, recorded in every output, and tested.
- **Oracle source/release and hashes:** PoB `v2.67.2`, commit `b32759ab…`, tree `74b90d3b…`, from source (section 3).
- **Runtime versions:** LuaJIT `2460b3ff` (`2.1.1784580905`, `PRNG=0`), luautf8 `0.1.6`, zlib 1.3.1 (Windows, PoB's DLL) or the system zlib (Linux).
- **Platforms tested:** Windows 11 (workstation, MSVC 19.51), Windows Server (`windows-latest`, its image's MSVC), Ubuntu 24.04 (`ubuntu-latest`; WSL 2 on the workstation, gcc 13.3). macOS not tested (best-effort per ADR-001; not in the gate).
- **Fixtures tested:** 30 public (5 PoB, 3 synthetic, 1 discrepancy, 21 community) in CI and on the workstation; 21 corpus builds on the workstation.
- **Determinism result:** identical across 3 runs for every fixture on both platforms; identical computed data across platforms.
- **Intentional discrepancy result:** classified unexpected (20), then expected with a register entry (19).
- **Packaged-artifact result:** not done (out of the ACTION-PLAN gate; plan "Out of scope"). PoB's Windows release ships `runtime/lua51.dll` of unknown LuaJIT commit and no `luajit.exe`; a packaged check would compare the release's results with the source-built oracle. Follow-up, not scheduled: it would show whether `PRNG=0` and our build differ from what users run (see limitations).
- **Known limitations:** section 9.
- **Required adaptations:** section 10.
- **Next unblocked phase:** Phase 2 (canonical model and data), which does not need the oracle. Phase 3 (calc slice) compares against it.

## 9. Known limitations
1. **No packaged-artifact mode** (section 8).
2. **Windows runs have no network isolation.** Hosted runners allow no firewall rule; Windows relies on the Linux result, the wrapper's `lcurl.safe` stub, and the module log (a network module is `ORACLE_ENVIRONMENT`).
3. **The oracle chooses where PoB leaves a result to chance.** For gems saved in the pre-3.23 format (no `variantId`), PoB picks one of three Vaal variants by `pairs` order, which changes per run even in PoB's own runtime (`v2.67.2:src/Classes/SkillsTab.lua` 446-451; 37 gem ids in v2.67.2 data). The `variant_order` shim makes it pick the base gem, which is what such a save meant; the choice is in `docs/parity/README.md` ("Where the oracle chooses for PoB"), and a successor that loads such a gem differently needs a register entry. The upstream report is drafted (maintainer posts it).
4. **`LUAJIT_SECURITY_PRNG=0` is a harness choice.** PoB's own runtime uses the default (seeded from the OS). Any other result that depends on `pairs` order over string keys would differ from what a user sees run to run; none was found beyond limitation 3 (51 builds, 3+ runs each, two platforms).
5. **A startup error through real PoB** (spec case 7) is tested only with a fake adapter on the real runtime; a real one needs a broken checkout copy. The adapter reads PoB's global `launch.promptMsg`, which the fake reproduces.
6. **Real-runtime tests are not in CI.** The `#[ignore]` tests run by hand; CI covers the real runtime through `oracle-run` on the fixture manifest (including the expected `ORACLE_IMPORT`).
7. **Whole-document hashes are per platform** (zlib version, module paths, settings path). Cross-platform equality is checked on the computed data by `oracle-compare`.
8. **Windows paths must be ASCII** (LuaJIT opens files through the ANSI code page); `oracle-run` refuses others as `ORACLE_ENVIRONMENT`.
9. **PoB's 3.13 test builds are old-tree** (PoB converts them on load) and PoB's own expected `.lua` outputs from 3.13 are not a reference (PoB's CI does not run them).
10. **Community builds are credited, used without the creators' consent, and removed on request** (ADR-007 Amendment 1; `tools/oracle/fixtures/community/SOURCES.md`).

## 10. Adaptations (all made)
| Adaptation | Why | Where recorded |
|---|---|---|
| `LUAJIT_SECURITY_PRNG=0` build define | default seeds string hashing from OS entropy, so `pairs` order changes per run | manifest `luajit.defines`; probed by `oracle-verify` |
| Shim `inflate` (zlib through LuaJIT FFI) | the wrapper stubs `Inflate`, so timeless-jewel data would load empty (a harness artifact) | manifest `adapter.shims`, each document's `shims` and `probe.inflate`; a failed shim makes the result `partial` |
| Shim `write_guard` (refuses `io.open` writes) | PoB writes a `.bin` cache into its checkout, hidden by its `.gitignore` | same; plus the ignored-files clean check after each fixture |
| Shim `variant_order` | PoB's own nondeterminism for old-format Vaal gems (limitation 3) | same, `probe.variant_order`; `docs/parity/README.md` |
| Cleared environment with an allow-list | `CI` changes PoB's code path; user settings must not be read | each report's `environment` |

## 11. Reproduce
```
tools/oracle/acquire.ps1            # Windows (acquire.sh on Linux): local/oracle
cargo build --release --locked -p loom-parity-cli
target/release/loom-parity-cli oracle-verify tools/oracle/manifest.json local/oracle
target/release/loom-parity-cli oracle-run tools/oracle/manifest.json local/oracle --out out/oracle tools/oracle/fixtures/fixtures.json
target/release/loom-parity-cli oracle-run tools/oracle/manifest.json local/oracle --out out/corpus --anonymous <corpus folder>
target/release/loom-parity-cli oracle-compare <run dir A> <run dir B> --out out/compare
target/release/loom-parity-cli divergence-check docs/parity/divergence-register.json
```
On Linux, run `oracle-run` inside `unshare -rn` for no network.

## 12. Open for the maintainer
- Confirm the decision `FEASIBLE_WITH_ADAPTATION` (by merging this PR).
- Post the upstream PoB issue (draft `local/upstream-issue-pob-vaal-variant.md`, on the workstation).
- From Spike B: the ADR-015 clarification on typed resolutions (`docs/spikes/spike-b-resolution.md` section 7) was accepted by the maintainer 2026-11-10 and written as ADR-015 Amendment 1.

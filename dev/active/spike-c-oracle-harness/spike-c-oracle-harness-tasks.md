Last Updated: 2026-11-10 (PR C3b+C4: determinism shim, community builds, CI job)

# Spike C: tasks

## PR C0: plan
- [x] Record the merges of PRs #26-#32; move Spike B task docs to `dev/completed/`
- [x] Create plan, context, tasks under `dev/active/spike-c-oracle-harness/`
- [x] Review with the `plan-reviewer` agent and apply findings
- [x] Update handoff Current state / Next action, CLAUDE.md Status, ACTION-PLAN Next action, README status
- [x] Open PR (#33; merged 2026-11-10 as `cb1ec21`)
- [x] Maintainer answers open questions 1, 3, 5 (2026-11-10, recommendations accepted)
- [x] Maintainer answers open questions 2 (PoB's 5 test builds + synthetic in CI) and 4 (`oracle` job informational until C5) (2026-11-10)

## PR C1: pinned runtime and manifest
- [x] `tools/oracle/manifest.json`, `acquire.ps1`, `acquire.sh`; AGENTS.md layout and commands
- [x] Check LuaJIT hash-randomization defaults at `2460b3ff`; record build flags (`LUAJIT_SECURITY_PRNG=0`)
- [x] Build LuaJIT `2460b3ff` and `luautf8 0.1.6` in `local/oracle/` (Windows); wrapper smoke by hand
- [x] AGENTS.md: acquire scripts exempt from the network rule
- [x] `parity-cli oracle-verify`; unit tests per failure class (26 `oracle_verify` tests with temporary git checkouts, 8 manifest tests)
- [x] Repeat the build, compare binary hashes; record (identical in the same folder; path-dependent, so recorded per machine in `build.json`)
- [x] Injected-failure runs with the real runtime (wrong runtime -> 11, ignored file in checkout -> 10)
- [x] `acquire.sh` full run on Linux (WSL 2, Ubuntu 24.04, gcc 13.3; also 18.04 with gcc 7.5; 2026-11-10): `oracle-verify` ok, rebuild identical, wrapper smoke ok also under `unshare -rn`
- [x] Review with `code-architecture-reviewer`; apply findings (no blockers; 4 should-fix and most nits applied, context file)
- [x] Open PR (#34; merged 2026-11-10 as `fffc476`, main CI run 42847724319 green)

## PR C2a: adapter and `oracle-run` (no shims)
- [x] `tools/oracle/adapter/oracle.lua` + `json.lua` (protocol 1), output schema, hand-written schema check (`oracle_output.rs`)
- [x] Startup-error handling (global `launch.promptMsg`), settings probe (`/Settings.xml`, not found)
- [x] `oracle_process.rs`: launcher shared with `oracle-verify`'s probe (cleared environment, closed stdin, pipes drained on threads, time limit)
- [x] `parity-cli oracle-run` (verify first, env allow-list, timeout, normalization, 3 repeats, clean check after, report and summary)
- [x] Breakdown selection measured (182 names over 6 fixtures) and recorded in the manifest (10 names)
- [x] Tests: scripted classification per class, fake Lua adapters and the real adapter on the real runtime (ignored test), writer Lua tests; real runs of the public fixtures on Windows (3/3, stable over 5 invocations)
- [x] Review with `code-architecture-reviewer`; apply findings (no blockers; S1-S6 applied, nits recorded in the context file)
- [x] Open PR (#35; merged 2026-11-10 as `e50848c`, main CI run 42854543956 green)

## PR C2b: shims
- [x] `adapter/shims.lua`: `Inflate` through LuaJIT FFI (zlib recorded and hashed per platform); `io.open` write guard; probe in the document (protocol 2); `partial` reasons (spec case 8 on the real runtime: zlib not loadable -> `partial`, exit 3)
- [ ] A real PoB startup error through the adapter (spec case 7; only a fake adapter so far): not done; needs a broken checkout copy, left for C3 or C5 with a reason
- [x] Ignored-files clean check after each fixture (a dirty checkout stops the run); synthetic `timeless-militant-faith.xml`
- [x] Corpus timeless-jewel builds show jewel data loaded; non-jewel computed data unchanged (6 C2a fixtures byte-identical)
- [x] Linux (WSL 2 Ubuntu 24.04): unit and real-runtime tests, fixtures 3/3 under `unshare -rn`
- [x] Review with `code-architecture-reviewer`; apply findings (no blockers; context file)
- [x] Open PR (#36; merged 2026-11-10 as `91b1438`; main CI run 42860826266 failed only on a cancelled macOS job, rerun)

## PR C3: fixtures, compare, divergence register
- [x] Fixture manifest (PoB test builds by blob hash, synthetic fixtures, discrepancy pair); expectations in `oracle-run` (exit 4 unmet); `--anonymous`
- [x] `parity-cli oracle-compare`; register schema, empty register, `divergence-check`, `docs/parity/README.md`
- [x] Intentional discrepancy: unexpected (exit 20), then expected with the test register (exit 19); real-runtime ignored test
- [x] Corpus run 3 times on Windows (twice); scrubbed summary in `docs/spikes/spike-c/` with a scrub test. Result: 3 of 21 builds nondeterministic (context file)
- [x] Review with `code-architecture-reviewer`; apply findings (1 blocker fixed; should-fix applied)
- [x] Open PR (#37)

## PR C3b+C4: corpus nondeterminism and CI (one PR, maintainer decision 2026-11-10)
- [x] C3b: reproduce on the 3 builds with `--runs 20`; find where PoB picks a transfigured gem variant (`SkillsTab:Load` old-format lookup; string-ID `pairs` order)
- [x] C3b: adapter-side fix (named, recorded shim) or fallback (`BLOCKED` for the affected fields); corpus rerun 3 times (`variant_order` shim, protocol 3; 4 corpus invocations identical)
- [x] Community builds (maintainer decision 2026-11-10; 11 of 16 acquired, 7 guides publish no PoB export; ADR-007 Amendment 1 after the terms check): acquire the agreed list (context file, "Community builds") with source, author, date, terms; `tools/oracle/fixtures/community/` + `SOURCES.md` (opens with the removal-on-request note; one line in `README.md`); tier `community` in `fixtures.json`; 3 runs each; propose CI inclusion
- [x] C4: `oracle` job (Linux, Windows), cached runtime build, offline run on Linux; first full run of `acquire.sh`; `divergence-check` step
- [x] C4: injected wrong-hash run; cross-platform comparison (run 42889081685: exit 11 on both runners)
- [x] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C5: report and gate
- [ ] `docs/spikes/spike-c-oracle.md` (completion report, gate verdict, feasibility decision)
- [ ] Promote the `oracle` job into `ci-ok` after 10 consecutive green runs on both runners; then `oracle-compare` exit 20 fails the job
- [ ] ACTION-PLAN status, handoff, CLAUDE.md
- [ ] Open PR

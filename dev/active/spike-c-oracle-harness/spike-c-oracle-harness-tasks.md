Last Updated: 2026-11-10 (PR C1: pinned runtime, manifest, `oracle-verify`)

# Spike C: tasks

## PR C0: plan
- [x] Record the merges of PRs #26-#32; move Spike B task docs to `dev/completed/`
- [x] Create plan, context, tasks under `dev/active/spike-c-oracle-harness/`
- [x] Review with the `plan-reviewer` agent and apply findings
- [x] Update handoff Current state / Next action, CLAUDE.md Status, ACTION-PLAN Next action, README status
- [x] Open PR (#33; merged 2026-11-10 as `cb1ec21`)
- [x] Maintainer answers open questions 1, 3, 5 (2026-11-10, recommendations accepted)
- [ ] Maintainer answers open questions 2 (CI fixtures, before C3) and 4 (CI gating, before C4)

## PR C1: pinned runtime and manifest
- [x] `tools/oracle/manifest.json`, `acquire.ps1`, `acquire.sh`; AGENTS.md layout and commands
- [x] Check LuaJIT hash-randomization defaults at `2460b3ff`; record build flags (`LUAJIT_SECURITY_PRNG=0`)
- [x] Build LuaJIT `2460b3ff` and `luautf8 0.1.6` in `local/oracle/` (Windows); wrapper smoke by hand
- [x] AGENTS.md: acquire scripts exempt from the network rule
- [x] `parity-cli oracle-verify`; unit tests per failure class (26 `oracle_verify` tests with temporary git checkouts, 8 manifest tests)
- [x] Repeat the build, compare binary hashes; record (identical in the same folder; path-dependent, so recorded per machine in `build.json`)
- [x] Injected-failure runs with the real runtime (wrong runtime -> 11, ignored file in checkout -> 10)
- [ ] `acquire.sh` full run on Linux: deferred to C4 (no compiler in the workstation's WSL)
- [x] Review with `code-architecture-reviewer`; apply findings (no blockers; 4 should-fix and most nits applied, context file)
- [x] Open PR (#34)

## PR C2a: adapter and `oracle-run` (no shims)
- [ ] `tools/oracle/adapter/oracle.lua` (protocol v1), output schema, hand-written schema check
- [ ] Startup-error handling (`build` nil; `mainObject` is local to the wrapper, so `promptMsg` is not readable after `dofile`, context file), settings probe
- [ ] `oracle-run` reads stdout/stderr while waiting (C1's probe runner polls, fine only for tiny output)
- [ ] `parity-cli oracle-run` (env allow-list, timeout, normalization, 3 repeats)
- [ ] Breakdown selection measured and recorded in the manifest
- [ ] Tests with fake adapter scripts for each failure class; real runs of the public fixtures on Windows
- [ ] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C2b: shims
- [ ] `Inflate` through LuaJIT FFI, pinned zlib recorded; `io.open` write guard; jewel probe
- [ ] Ignored-files tree check after runs; synthetic timeless-jewel fixture
- [ ] Corpus timeless-jewel builds show jewel data loaded; non-jewel outcomes unchanged
- [ ] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C3: fixtures, compare, divergence register
- [ ] Fixture manifest (PoB test builds by blob hash, synthetic fixtures, discrepancy pair)
- [ ] `parity-cli oracle-compare`; register schema, empty register, `divergence-check`, `docs/parity/README.md`
- [ ] Corpus run 3 times on Windows; scrubbed summary in `docs/spikes/spike-c/`
- [ ] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C4: CI
- [ ] `oracle` job (Linux, Windows), cached runtime build, offline run on Linux; first full run of `acquire.sh`
- [ ] Injected wrong-hash run; cross-platform comparison
- [ ] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C5: report and gate
- [ ] `docs/spikes/spike-c-oracle.md` (completion report, gate verdict, feasibility decision)
- [ ] ACTION-PLAN status, handoff, CLAUDE.md
- [ ] Open PR

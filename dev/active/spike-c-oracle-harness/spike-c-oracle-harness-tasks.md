Last Updated: 2026-11-10 (PR C0: plan)

# Spike C: tasks

## PR C0: plan
- [x] Record the merges of PRs #26-#32; move Spike B task docs to `dev/completed/`
- [x] Create plan, context, tasks under `dev/active/spike-c-oracle-harness/`
- [x] Review with the `plan-reviewer` agent and apply findings
- [x] Update handoff Current state / Next action, CLAUDE.md Status, ACTION-PLAN Next action, README status
- [x] Open PR (#33)
- [ ] Maintainer answers the open questions (context file)

## PR C1: pinned runtime and manifest
- [ ] `tools/oracle/manifest.json`, `acquire.ps1`, `acquire.sh`; AGENTS.md layout
- [ ] Check LuaJIT hash-randomization defaults at `2460b3ff`; record build flags
- [ ] Build LuaJIT `2460b3ff` and `luautf8 0.1.6` in `local/oracle/` (Windows); wrapper smoke by hand
- [ ] AGENTS.md: acquire scripts exempt from the network rule
- [ ] `parity-cli oracle-verify`; unit tests per failure class
- [ ] Repeat the build, compare binary hashes; record
- [ ] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C2a: adapter and `oracle-run` (no shims)
- [ ] `tools/oracle/adapter/oracle.lua` (protocol v1), output schema, hand-written schema check
- [ ] Startup-error handling (`promptMsg`, `build` nil), settings probe
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
- [ ] `oracle` job (Linux, Windows), cached runtime build, offline run on Linux
- [ ] Injected wrong-hash run; cross-platform comparison
- [ ] Review with `code-architecture-reviewer`; apply findings
- [ ] Open PR

## PR C5: report and gate
- [ ] `docs/spikes/spike-c-oracle.md` (completion report, gate verdict, feasibility decision)
- [ ] ACTION-PLAN status, handoff, CLAUDE.md
- [ ] Open PR

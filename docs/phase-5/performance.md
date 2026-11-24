# Performance (Phase 5, P7)

The plan's step P7 and decision P-h (`dev/active/passive-tree-and-phase-5/`), with ADR-013 (absolute budgets per hardware tier, relative regression limits). Gate 9: the budgets are met on the reference tier or each exception is listed for the maintainer, and CI's instruction-count check passes.

## Two kinds of measure
**Instruction counts in CI (regressions).**
- The tool: `tools/bench` (`loom-bench`).
- The paths: the core paths on the synthetic tree bundle and the largest public PoB fixture.
- The measure: callgrind on Linux, CI job `bench`, required in `ci-ok`.
- The rule: a count 10% over `tools/bench/baseline.json` fails.
  - A count 10% under is reported, so the baseline can be lowered.
  - Counts are stable across runners, unlike time.

**Wall clock on the reference tier (the budgets).**
- The reference tier is the maintainer's workstation (this one), release builds, the real 3_29 bundle, and the largest public fixture.
- These are maintainer-run and recorded below.

### The scenarios
Each is one `measured_*` function, so callgrind counts only that work and `wall` times only that work; reading the files comes first.

| Scenario | Work |
|---|---|
| `validate` | the bundle validator with every file's SHA-256 (structure, references, domain rules), as a folder chosen by hand is checked |
| `load` | the bundle's tables loaded, the resolver built, the passive tree built |
| `start` | what the start does with an installed bundle: the validator with sizes only, which hands over the tables it loaded, then the resolver and the passive tree |
| `import` | a PoB build read and its lines resolved against the bundle |
| `calculate` | one calculation of the resolved build, its passives resolved against the tree |
| `allocate` | the path to a node on the tree: the first node by ID with a path of five or more |

```sh
cargo run --release -p loom-bench -- wall [--bundle <dir>] [--build <file>] [--runs <n>]
bash tools/bench/count.sh [--update]      # Linux with valgrind
```

## The baseline
- `tools/bench/baseline.json` holds CI's own counts: the ubuntu-24.04 runner and the pinned toolchain.
- A scenario without a baseline fails in CI (`CI` set), after printing its counts. This PR's first `bench` run therefore fails, and its counts are committed from that log. The baseline: validate 10,460,879; load 4,788,501; import 174,568,115; calculate 18,083,064; allocate 445,995 instructions (CI run 43779626395, 2026-11-15). `start`, added with the startup fix: 5,679,942 instructions, from that change's first run that counted it (CI run 43787174465, 2026-11-16; `count.sh` had left it out of its loop until `3bce84e`, and a `loom-bench` test now holds the script's list equal to the binary's).
- A change meant to cost more updates the baseline from its CI log (`count.sh --update` on Linux, or the printed counts), and says so in the PR.
- **Re-baselined 2026-11-16 after PR #134** (the passive tree's T3 to T8), from that PR's `bench` run (CI run 43916808219): validate 18,513,018; load 7,732,381; start 9,434,929; import 174,562,281; calculate 18,095,962; allocate 452,683. The bench bundle, `bundle-poe1-tree`, grew in #134: it now carries a second tree (the synthetic 3_28 pack), the cluster jewel tables, and a passive stat descriptions file. Validate (+77%), load (+62%), and start (+66%) read, check, and build that extra data; start also builds the pack's tree. Import, calculate, and allocate moved by 1.5% or less. The startup budget (2.0 s) is measured on the real bundle, not on this fixture; a real bundle with the four packs has not been timed yet, and that belongs with the maintainer-run signed bundle (`docs/tree/tree-report.md`).
- **Re-baselined 2026-11-20 in PR #167** (Phase 6's open tasks), from that PR's `bench` run on `c12823c` (CI run 44683273308): validate 18,674,060; load 7,774,594; start 9,462,126; import 180,553,544; calculate 39,733,502; allocate 453,112. Calculate rose 119.6% over the previous baseline; the others 3.4% or less. On `dev` after the first Phase 6 squash (`1097108`, CI run 44239677047) calculate counted 19,532,118 (+7.9%), so the rise is this PR's. Its cause is no one step: the bench build's calculation, timed locally (`loom-bench wall`, 100 runs, medians), grows with each mechanic the PR adds: 1.4 ms at `1097108`, 1.8 ms after Mirage Archer (P6-8b), 2.0 to 2.3 ms after the mana cost per second (P6-10b part 6), 2.4 ms after the leech caps (part 7), 2.8 ms at P6-11bj. Turning the build's "Include in Full DPS" off changes nothing, so the full DPS pass is not the cause. Scaled from the reference tier's 6.7 ms (measured before Phase 6), about 13 ms there, it stays far inside the item-edit preview's 300 ms and the allocation preview's 150 ms. For the maintainer: a profile of where the calculation spends its time, before Phase 7 adds more. A candidate: every store query (`sum`, `increased`, `more`, `phases::has`) scans all the effects, as the store keeps no index by quantity (`crates/loom-calc/src/store.rs`), so each new mechanic's queries cost the store's size. Merging the hit's 30 conversion checks into one pass (tried 2026-11-20) changed nothing measurable, so no single query dominates.
- **Re-baselined 2026-11-22 in PR #167** (after P6-11hm), from that PR's `bench` run on `698abc6` (CI run 45061004032): validate 18,677,539; load 7,770,264; start 9,492,209; import 180,993,243; calculate 44,815,968; allocate 453,177. Calculate rose 12.8% over the 2026-11-20 baseline, from the mechanics added after P6-9g (`c12823c`) up to P6-11hm (each step's new store queries, as above); the others moved by 0.3% or less. The profile asked for above is still the maintainer's, before Phase 7.

## Budgets on the reference tier
Workstation: Windows 11, the maintainer's machine. Release builds, 2026-11-15:
- the real bundle: build 26530219, schema 4, `local/b2ev/bundle-poe1-tree-a`;
- the build: `tools/oracle/fixtures/community/forged-frostbearer-spectre-necromancer.xml`, the largest public fixture;
- `loom-bench wall --runs 20`.

| Budget | Measured | Met |
|---|---|---|
| Startup to first screen ≤ 2.0 s | 1.41 to 1.84 s with the real bundle installed (median 1.77 s, 11 runs); 0.71 to 0.82 s with no bundle (7 runs). Was 3.0 to 3.8 s (median 3.11 s) before the startup fix ("Startup", below) | yes |
| Import and first calculation ≤ 1.5 s | import 28.8 ms + calculate 6.7 ms (medians; slowest 31.0 + 7.8 ms) | yes |
| Item-edit preview ≤ 300 ms | one calculation, 6.7 ms (the preview applies the edit and calculates the edited build; the current result is reused) | yes |
| Allocation preview (hover) ≤ 150 ms | allocate 1.0 ms + calculate 6.7 ms | yes |
| Tree first paint ≤ 500 ms, pan and zoom ≥ 50 fps | not measured here: a webview measurement | exception: maintainer-run (below) |
| Version conversion preview ≤ 300 ms | 0.01 to 0.41 ms: `loom_app::tree::converted` on four public builds saved on 3_27 and 3_28, with the real bundle and its packs (release build, 21 runs each; measured 2026-11-16 on `feat/tree-remaining`) | yes |
| Peak memory ≤ 600 MB | 319 MB, the application's process at startup with the real bundle (peak working set); the WebView2 processes are not counted | yes, for the application's process |
| Installer ≤ 40 MB without the bundle | the MSI, the AppImage, the `.deb`, the portable zip: measured by the release workflow's run (gate 6); the NSIS setup with the WebView2 bootstrapper was 5.8 MB locally (P1) | exception: the NSIS setup |

**The core paths with the real bundle (`loom-bench wall`):**

| Scenario | Median | Slowest | Runs |
|---|---|---|---|
| `validate` | 585.0 ms | 625.0 ms | 5 |
| `load` | 726.2 ms | 886.4 ms | 20 |
| `start` | 994.4 ms | 1,040.0 ms | 7 (after the startup fix; the same session had `validate` at 690 ms and `load` at 870 ms) |
| `import` | 28.8 ms | 31.0 ms | 20 |
| `calculate` | 6.7 ms | 7.8 ms | 20 |
| `allocate` | 1.0 ms | 1.3 ms | 20 |

**Startup** is the release application's `--smoke` run, timed from launch to the process's exit.
- The run exits when the first screen has rendered (`app_ready`, after the UI's first refresh), so the time includes the exit.
- The data folder is a scratch folder (`LOOM_DATA_DIR`).
- The bundle is installed and accepted there, or given by `LOOM_BUNDLE_DIR`.

### Startup
The first measurement, 3.5 to 4.7 s with the real bundle, was over the 2.0 s budget. Its timer counted in whole seconds (`Start-Process -Wait`); timed again with the process's own exit (`WaitForExit`), the same build took 3.0 to 3.8 s, median 3.11 s. The data loaded before the window, and paid twice:
- the validator ran on every start: every file's SHA-256 over 130 MB, references, domain rules, and its own load of the tables;
- then the tables loaded a second time, with the resolver and the tree.

The maintainer asked for the recommended fixes (2026-11-15). All three are in:
- **(a) Sizes, not hashes, for an installed bundle** (`loom_data::bundle::validate::Integrity::Sizes`; `bundles::Library::load`).
  - The start's scan has already checked the signature and every file's size (`loom_app::install::status_of`).
  - The hashes were checked at install, and are checked again on **Verify**.
  - The validator's other checks still run.
  - A folder chosen by hand, or `LOOM_BUNDLE_DIR`, is still hashed in full.
  - This is what plan decision P-e meant. Test: `install.rs`, `the_start_checks_an_installed_bundles_sizes_and_other_folders_hashes`.
- **(b) One load** (`validate::check`). The validator returns the bundle it loaded, and the application uses it.
- **(c) The window before the data** (`loom_app::Config::load_in_background`, which the shell sets).
  - `App::start` returns at once, and the data loads on the writer thread before any command, so a command sent at once waits for it.
  - The snapshot says `loading` until then (app API v1 `DataStatus.loading`, additive), and the start screen says "Loading game data…" instead of "No game data yet."
  - Tests: `workflow.rs`, `a_background_start_should_say_loading_then_publish_the_data`; UI `errors.test.tsx`.

| Start, release build, median of 7 | Before | (a) and (b) | (a), (b), and (c) |
|---|---|---|---|
| the real bundle installed | 3.11 s | 2.15 s | 1.77 s (11 runs: 1.41 to 1.84 s) |
| the real bundle from `LOOM_BUNDLE_DIR` (hashed) | 3.10 s | 2.35 s | 1.92 s |
| no bundle | 1.23 s | 1.21 s | 0.76 s |

- With (c) the data loads while the webview starts. The log's `data at start` line gives its time: about 1.08 s from the application's start. So the data is ready at about the same time as the first screen.
- `loom-bench wall` on the real bundle: `start` 994 ms, against `validate` 690 ms plus `load` 870 ms run one after the other.
  - In one `start`: parsing the tables 350 to 490 ms, the validator's other checks 125 to 250 ms, the resolver 250 to 325 ms, `BundleData` 60 ms, the tree 5 ms.
- How it was timed: `tools/package/smoke.ps1`'s `--smoke` start, timed from launch to the process's exit. The bundle was installed and accepted in a scratch data folder.

### Exceptions for the maintainer
1. **Startup:** met since the startup fix (above). Nothing to decide.
2. **The NSIS setup is over 40 MB.**
   - It carries Microsoft's offline WebView2 installer, about 130 MB, so installing needs no network (ADR-010; `docs/phase-5/packaging.md`).
   - Without it the setup is about 6 MB.
   - The MSI and the portable zip use the WebView2 runtime Windows already has.
   - Proposed: accept. The budget excludes the bundle, and the WebView2 runtime is Microsoft's, not this project's.
3. **Tree first paint and pan and zoom are not measured yet.**
   - They are the webview's. The core part, the tree's layout, is in `load`.
   - Proposed: the maintainer measures them in the release app with the browser tools (WebView2 DevTools, Performance panel) on the real bundle and records them here.
   - The CI e2e on the synthetic bundle checks the tree renders.
4. **Version conversion** is measured since T5b (2026-11-16): inside budget by three orders of magnitude.
5. **Peak memory counts the application's process only.** The WebView2 processes are Microsoft's runtime, so they are not counted. The maintainer can add them from Task Manager's process tree if the budget means the whole tree.

## Accessibility (WCAG 2.2 AA)
- **axe** (`expectAccessible`, `ui/src/test/render.tsx`) runs the WCAG 2.0, 2.1, and 2.2 A and AA rules.
  - `a11y.test.tsx` checks every screen; eight other test files check their own dialogs and panels.
  - Clean in the 82 UI tests run 2026-11-15.
- **Colour contrast** needs layout that jsdom lacks, so `theme.test.ts` checks it from the tokens.
- **Keyboard:** `keyboard.test.tsx` imports a build, moves between screens, and uses the export menu without a mouse, and checks that a calculation is announced politely.
- **The e2e** (`ui/e2e/workflow.mjs`) drives the real application by role and accessible name.
- **Not covered by a test:**
  - keyboard allocation on the passive tree;
  - a screen reader pass (NVDA or Narrator).

  Both are exceptions for the maintainer to check by hand.

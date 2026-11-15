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
| `validate` | the bundle validator, which the application runs before loading a bundle (structure, every file's SHA-256, references, domain rules) |
| `load` | the bundle's tables loaded, the resolver built, the passive tree built (the core part of startup) |
| `import` | a PoB build read and its lines resolved against the bundle |
| `calculate` | one calculation of the resolved build, its passives resolved against the tree |
| `allocate` | the path to a node on the tree: the first node by ID with a path of five or more |

```sh
cargo run --release -p loom-bench -- wall [--bundle <dir>] [--build <file>] [--runs <n>]
bash tools/bench/count.sh [--update]      # Linux with valgrind
```

## The baseline
- `tools/bench/baseline.json` holds CI's own counts: the ubuntu-24.04 runner and the pinned toolchain.
- The first run had no baseline, so it printed its counts and passed. They were committed from this PR's `bench` log.
- A change meant to cost more updates the baseline from its CI log (`count.sh --update` on Linux, or the printed counts), and says so in the PR.

## Budgets on the reference tier
Workstation: Windows 11, the maintainer's machine. Release builds, 2026-11-15:
- the real bundle: build 26530219, schema 4, `local/b2ev/bundle-poe1-tree-a`;
- the build: `tools/oracle/fixtures/community/forged-frostbearer-spectre-necromancer.xml`, the largest public fixture;
- `loom-bench wall --runs 20`.

| Budget | Measured | Met |
|---|---|---|
| Startup to first screen ≤ 2.0 s | 3.46 to 4.69 s with the real bundle (5 runs, the first cold); 0.59 to 1.48 s with no bundle (3 runs) | **no: exception 1** |
| Import and first calculation ≤ 1.5 s | import 28.8 ms + calculate 6.7 ms (medians; slowest 31.0 + 7.8 ms) | yes |
| Item-edit preview ≤ 300 ms | one calculation, 6.7 ms (the preview applies the edit and calculates the edited build; the current result is reused) | yes |
| Allocation preview (hover) ≤ 150 ms | allocate 1.0 ms + calculate 6.7 ms | yes |
| Tree first paint ≤ 500 ms, pan and zoom ≥ 50 fps | not measured here: a webview measurement | exception: maintainer-run (below) |
| Version conversion preview ≤ 300 ms | no version conversion yet (T4 is not done) | not applicable |
| Peak memory ≤ 600 MB | 319 MB, the application's process at startup with the real bundle (peak working set); the WebView2 processes are not counted | yes, for the application's process |
| Installer ≤ 40 MB without the bundle | the MSI, the AppImage, the `.deb`, the portable zip: measured by the release workflow's run (gate 6); the NSIS setup with the WebView2 bootstrapper was 5.8 MB locally (P1) | exception: the NSIS setup |

**The core paths with the real bundle (`loom-bench wall`):**

| Scenario | Median | Slowest | Runs |
|---|---|---|---|
| `validate` | 585.0 ms | 625.0 ms | 5 |
| `load` | 726.2 ms | 886.4 ms | 20 |
| `import` | 28.8 ms | 31.0 ms | 20 |
| `calculate` | 6.7 ms | 7.8 ms | 20 |
| `allocate` | 1.0 ms | 1.3 ms | 20 |

**Startup** is the release application's `--smoke` run, timed from launch to exit.
- The run exits when the first screen has its data (`app_ready`, after the UI's first refresh), so the time includes the exit.
- The data folder is a scratch folder (`LOOM_DATA_DIR`), and the bundle is `LOOM_BUNDLE_DIR`.

### Exceptions for the maintainer
1. **Startup with the real bundle takes 3.5 to 4.7 s, over the 2.0 s budget.** Without a bundle it is 0.6 to 1.5 s, so loading the data costs about 3 s. Measured parts:
   - the validator, 585 ms. `GameData::load` runs it on every start: every file's SHA-256 over 130 MB, references, domain rules, and its own load of the tables;
   - the load, 726 ms: the tables a second time, the resolver, the tree;
   - reading the 130 MB from disk;
   - the rest of the first snapshot.

   The data loads before the first screen. Proposed, for the maintainer to choose, cheapest first:
   - (a) For an installed bundle the start checks only the signature and the file sizes, as plan decision P-e intends: its hashes were checked at install and on **Verify**. The validator's other checks stay, and a folder chosen for development is validated in full.
   - (b) The validator hands the tables it loaded to the application, which then does not load them a second time.
   - (c) The first screen comes before the data, with the data loading in the background.

   (a) and (b) together should save roughly 1 s. Only (c) is sure to meet 2.0 s on this workstation.
2. **The NSIS setup is over 40 MB.**
   - It carries Microsoft's offline WebView2 installer, about 130 MB, so installing needs no network (ADR-010; `docs/phase-5/packaging.md`).
   - Without it the setup is about 6 MB.
   - The MSI and the portable zip use the WebView2 runtime Windows already has.
   - Proposed: accept. The budget excludes the bundle, and the WebView2 runtime is Microsoft's, not this project's.
3. **Tree first paint and pan and zoom are not measured yet.**
   - They are the webview's. The core part, the tree's layout, is in `load`.
   - Proposed: the maintainer measures them in the release app with the browser tools (WebView2 DevTools, Performance panel) on the real bundle and records them here.
   - The CI e2e on the synthetic bundle checks the tree renders.
4. **Version conversion** has no feature to measure until T4.
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

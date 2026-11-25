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
| Allocation preview (hover) ≤ 150 ms | allocate 1.0 ms + calculate 6.7 ms; with the largest build after Phase 6 about 110 ms ("The calculation profile"); in the application on the laptop (2026-11-24): the path 8 to 64 ms, the numbers 369 to 494 ms | the path yes; the numbers not on the laptop (to measure on the reference tier) |
| Tree first paint ≤ 500 ms, pan and zoom ≥ 50 fps | first paint 305, 197, 168 ms; after the fix pan 54 fps typical, 49 lowest; zoom 58 to 59 typical, 52 to 55 lowest (before: pan 27 and 23, zoom 52 and 42; measured 2026-11-24 on a laptop, "The maintainer's application test", below) | yes (pan's lowest 49 on the laptop, one frame of 51 ms) |
| Version conversion preview ≤ 300 ms | 0.01 to 0.41 ms: `loom_app::tree::converted` on four public builds saved on 3_27 and 3_28, with the real bundle and its packs (release build, 21 runs each; measured 2026-11-16 on `feat/tree-remaining`) | yes |
| Peak memory ≤ 600 MB | 319 MB, the application's process at startup with the real bundle (peak working set); 2026-11-24 on a laptop with the largest build and the tree open: the application 321 MB, with its WebView2 processes 616 MB, 650 MB after a minute of use | yes: the budget counts the application's process only (maintainer 2026-11-24) |
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
   - **Accepted by the maintainer, 2026-11-24.**
3. **Tree first paint, pan, and zoom: measured 2026-11-24** ("The maintainer's application test", below). First paint is inside budget; pan and zoom were not, and are fixed in PR #168, to be measured again on its next build.
4. **Version conversion** is measured since T5b (2026-11-16): inside budget by three orders of magnitude.
5. **Peak memory counts the application's process only.** The WebView2 processes are Microsoft's runtime, so they are not counted. Measured 2026-11-24 with them: 616 MB with the tree open, 650 MB at most after a minute of use, over 600 MB. **The maintainer decided (2026-11-24): the budget counts the application's process only**, so it is met.

### The maintainer's application test (2026-11-24)
`docs/phase-5/maintainer-tests.md`, part A and B1, run by an agent session on the maintainer's second workstation, through the WebView2 debugging port (synthetic input; the tree's sprites from the signed 0.9 data).
- **Workstation:** Intel Core i7-1255U (10 cores, 12 threads, Iris Xe graphics), 15.7 GB, Windows 11 Home 26H2 (build 26300), 1920×1080 at 150%, 60 Hz. A laptop, slower than the reference tier.
- **The build:** the portable package of PR #168's CI, with `forged-frostbearer-spectre-necromancer.xml` open.

**First paint** (the click on Passive tree to the first screenshot with the tree): 305, 197, 168 ms. The click's handler took 206, 119, and 94 ms of it; long frames after the click were at most 135 ms. The maintainer's own recording earlier the same day showed two frames of 1,015 and 867 ms when opening the tree; the three runs here did not repeat them (the first open of a session, not profiled).

**Pan and zoom** (frame times in the page; the DevTools FPS meter agreed):

| Action | Input | Typical | Lowest | Worst frame |
|---|---|---|---|---|
| Idle | none | 60 | 60 | 17 ms |
| Pan, from Fit | a 300×200 px circle each second, 5 s | 27 | 23 | 166 ms |
| Zoom, from Fit | 10 wheel ticks a second at the centre | 52 | 42 | 135 ms |
| Zoom, from Fit | 20 wheel ticks a second | 28 | 13 | 367 ms |

Also seen: the wheel scrolled the page as well as zooming the tree; the node card stayed up while dragging; a 200 px drag seemed to move the tree about 380 px (not isolated; to check by hand).

**The fix** (`ui/src/screens/Tree.tsx`, `TreeCanvas`, PR #168): every frame drew the whole tree with one `stroke()` per link (thousands) and reset the canvas's size, which reallocates its buffer; each pointer or wheel event drew again; and pan and zoom built on the camera of the last render, so events between two renders were lost (zoom ticks dropped).
- The links are gathered into one `Path2D` per look and stroked once each.
- The buffer is resized only when the canvas's size changes.
- Nodes and links outside the canvas are skipped.
- Drawing waits for the next animation frame, at most once a frame.
- Pan and zoom update the latest camera.
- The wheel listener is the canvas's own and not passive, so the page no longer scrolls.
- The node card closes when a drag starts.

Not measured here (no WebView2 run on this workstation in this session): the pan and zoom rates after the fix are for the next run of part A on PR #168's next build. The maintainer's own look (2026-11-24): dragging shows no noticeable delay.

**The hover preview** (the maintainer, 2026-11-24: a long delay before a node's card and its path appear). The window asked for the preview 120 ms after the pointer reached a node, and the answer came with a full calculation of the build after the change (about 110 ms for the largest build on the reference tier, nested full DPS included; more on a laptop), so the path showed only with the numbers. Every node the pointer rested on started its own calculation, and none was cancelled when the pointer moved on, so they competed for the processor. Now (PR #168):
- the path and the nodes removed come first, with no calculation (shell `preview_tree` with `fields: false`, additive), and the card says "Calculating the change…" until the numbers come;
- a hover preview with fields cancels the previous one's calculation (`App::preview_tree_hover`), through the engine's cancel hook.

**After the fixes** (the same session, same laptop and build, PR #168's package of CI run 45542395784):

| Run | Typical fps (before, now) | Lowest fps (before, now) | Worst frame (before, now) |
|---|---|---|---|
| Idle | 60, 60 | 60, 60 | 17, 19 ms |
| Pan, from Fit | 27, 54 | 23, 49 | 166, 51 ms |
| Zoom, 10 ticks a second | 52, 58 | 42, 52 | 135, 67 ms |
| Zoom, 20 ticks a second | 28, 59 | 13, 55 | 367, 48 ms |

The 20-ticks run before waited for each event and did not start from Fit, so that row is not like for like. The wheel no longer scrolled the page, and the node card closed as a drag began.

The hover (from the pointer's event in the page; five notables at three zoom steps, two passes each): the card 3 to 6 ms; its path ("Click to allocate", drawn with it) 8 to 64 ms; the numbers 369 to 494 ms. After a sweep of 6 or 10 nodes, 100 ms each, the last node's numbers came in 395 and 430 ms, so the superseded calculations no longer pile up. The numbers' time is one calculation of the largest build on this laptop (the reference tier's 110 ms, about 3.5 times slower here); the next lever is the per-calculation cache of resolved skills ("The calculation profile", "Next levers").

**Memory** (private working set, Task Manager's figure):

| Process | Tree open | After a minute of switching screens |
|---|---|---|
| Exiled Studio | 321.3 MB | 321.3 MB |
| WebView2 GPU | 130.9 MB | 157.0 MB |
| WebView2 renderer | 112.5 MB | 117.1 MB |
| WebView2 browser | 36.0 MB | 38.5 MB |
| WebView2 others (network, storage, crash reporting, the recording's tracing) | 15.7 MB | 15.6 MB |
| Total | 616.4 MB | 649.5 MB |

## The calculation profile (2026-11-24)

The re-baselines above asked for a profile before Phase 7 (the maintainer asked for it on 2026-11-24). Measured with `loom-bench wall` on the evidence bundle `local/p6/packs/b-ct` (build 26530219, data schema 5) and the largest public fixture (`forged-frostbearer-spectre-necromancer.xml`), release build, Windows workstation. The bench's synthetic bundle has a few rows per table, so CI's instruction counts could not show what follows.

**Where the time went** (temporary timers and counters in a local build, never committed: each phase, each step of `poe1::collect`, `poe1::states::states` and `conditions::settle`, the store's queries, and each data lookup):

- **One `calculate` was four calculations:** the build's own and three nested ones for the full DPS (`poe1::full_dps`, one per included skill). The phases themselves took about 12 ms in all; the rest was collecting.
- **Data lookups were linear scans.** `CalcData for Bundle` found a row with `iter().find` over the whole table: per `calculate` about 12,000 `granted_effect` lookups over 12,829 rows, and about 11,000 each of `skill_gem` (847 rows) and `gem_effect` (1,292 rows), plus `passive_stats`, `passive_kind` (5,706 rows), `active_skill`, and `stat_set`: on the order of 100 million string comparisons.
- **The settled conditions were settled three times per calculation** (`super::states::states(input)` in `collect` for the buffs and for the exposure, and the engine's `rules.states` after collecting), each resolving the main skill and the active skills again.
- **Not the cause:** the store's queries. They scan all effects (no index by quantity), but per `calculate` that was about 2,750 queries over about a million effects in all, a few milliseconds. The re-baselines' candidate is not the bottleneck.

**Changed** (no result changes: every public and corpus calc-run document is byte-identical to the run before, `local/p6/s2` against `local/p6/perf-pub` and `perf-cor`):

- `loom_data::bundle::Bundle` keeps a key index per table (`granted_effect`, `gem_effect`, `skill_gem`, `active_skill`, `stat_set`, `passive`), built on first use. Each lookup checks the row it finds and falls back to the scan on a miss or when the table's length changed (the tables are public), so its answer is always the scan's. One `calculate`: about 44,000 hits, 84 misses.
- The engine settles the conditions once, before collecting, and hands them to the rules through `Ingest::with_states`; `poe1::collect` reads them there.

**Result:**

| | Before | After |
|---|---|---|
| `calculate`, the largest fixture, `b-ct` (median of 30) | 231 ms | 106 to 112 ms |
| The public calc-run, 535 builds (calculation time) | 5.14 s | 2.38 s |

Both stay inside the item-edit preview's 300 ms; at 231 ms the largest builds were close to it on this workstation.

**Next levers** (for Phase 7, not done here; see "The skills and the body armour check" below for what a later profile found first): the main skill and the active skills are still resolved many times per calculation (`states`, `settle`'s checks, the curses, the gems, the costs): a per-calculation cache of the resolved skills would cut most of what is left; the full DPS pass recollects the whole build per included skill, where the items, tree, and config are the same each time; and the store could index its effects by quantity once queries grow.

### The skills and the body armour check (RV-21, 2026-11-25)

The hover numbers on the passive tree are one calculation of the build, so the calculation was profiled again (the same bundle, fixture, and method: temporary timers in a local build, never committed). The cache of resolved skills named under "Next levers" was not what the time went to. Two things were, about 80 of the 109 ms:

- **`skill::active_skills` copied the whole build once per active skill** (`with_main_skill`, 18 copies, about 1.8 ms each): it asked for each skill as "the main skill of a copy of the build that picks it". It now asks `skill::skill_at` for the skill by its group and position on the build itself. The trigger, full DPS, and enemy-hit paths still make their own copy for a nested calculation; they need one.
- **`states::no_life_modifiers_on_body_armour` built the set of every stat of every mod without the `life` tag**, six times a calculation (about 9.5 ms each), to ask about the handful of stats on one item. `CalcData::mods_by_tag_among` gives the tagged mods and, of the other mods' stats, only those asked about. Only a build with "no Life Modifiers on Equipped Body Armour" on its tree paid this; the fixture is one.

**No result changes:** every public calc-run document (535) and corpus document (21 on this workstation) is byte-identical before and after (`local/rv21/before` against `local/rv21/after`, bundle `b-ct`).

| | Before | After |
|---|---|---|
| `calculate`, the largest fixture, `b-ct` (median of 20) | 109.0 ms | 43.9 ms |
| The public calc-run, 536 builds (calculation time) | 2.40 s | 1.67 s |
| The corpus calc-run, 21 builds (calculation time) | 0.57 s | 0.27 s |

On the test laptop the hover's numbers took 369 to 494 ms, about 3.5 times this workstation's calculation; the same ratio puts them near 150 ms, the budget. Measuring it there is the maintainer's (with RV-09's run).

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
- **Keyboard allocation, checked 2026-11-24** (`maintainer-tests.md` B1, synthetic keys): it works through the List view only. Passive tree by Tab and Enter from the sidebar; Tab to "Search nodes", type a name, and the list shows the results; Enter on "Allocate …" allocates the node and its path, Enter again on "Remove …" removes it; Ctrl+Z undoes. Problems found (agent work, `dev/active/open-items/maintainer-follow-ups.md`):
  - the canvas takes no focus, so nodes cannot be picked or the tree panned by keyboard (only the zoom buttons and Fit);
  - Allocate added six nodes (the path) without saying so first, and Remove took off only the one node;
  - several buttons share one name ("Allocate Minion Life") and cannot be told apart;
  - every row of a list is a Tab stop with no way past (60 Tabs did not leave the Notables list);
  - the Delete tree confirmation does not close on Escape, and after Cancel the focus falls to the page;
  - Enter on the screen already shown does not move the focus to its heading.
  Fixed in PR #168 (2026-11-24) except the canvas: the confirmations are a modal (Escape cancels, the focus returns), each list is one Tab stop with the arrow keys inside, each list button is described by its node's lines and what it changes, and the current screen's navigation button moves the focus to its heading. Tests: `ui/src/test/tree.test.tsx`, "is one Tab stop, says what a button changes, and asks with Escape to cancel".
- **The screen reader pass** is still the maintainer's (it needs a person listening).

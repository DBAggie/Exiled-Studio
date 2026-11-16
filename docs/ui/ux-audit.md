# UI and UX audit (2026-11-15)

A whole-app audit of the interface at `dev` `b89587b` (the state released as 0.9.0-1), asked for by the maintainer on 2026-11-15, with the fixes made on the same branch. It follows the walkthrough of 2026-11-14 (`ux-walkthrough.md`), which fixed what loses work or misleads; this audit is about how the app looks and reads, and what the dashboards can show.

- **Refined version** (this document): branch `feat/ux-audit`, base `dev`. The design language of `UI-DESIGN-BRIEF.md` is kept; layout, density, and components change.
- **Free redesign** (the maintainer asked for both): branch `feat/ux-audit-redesign`, stacked on this one; see `ux-redesign.md` there.

## What the maintainer asked for
| Ask (2026-11-15) | Where it is answered |
|---|---|
| Minimalist dashboards and components, with a click to see more | D-02, D-06, B-01 |
| Detail, for users who want it | D-04's detail views, D-06, D-07 |
| Charts and graphs | D-04, D-05 |
| Comparing changes | D-03 |
| Flexibility | D-01, A-03 |
| A sleek, friendly interface; no wall of text in a small space | B-01 to B-03, A-04 |
| Hover displays always in view (the flask tooltip was cut off until the page was scrolled) | A-02 |
| The side menu fills only the window's height | A-01 |
| Clean equipment and skills displays | C-01 to C-04 |
| Dashboard components for the breadth of stats, calculations, and how they scale over time | D-01, D-04, D-05 |

Decisions the maintainer made before the work (2026-11-15): the audit may change anything it needs, the calculation engine included; "time based scaling" means all four of progression across stages, a fight timeline, the edit history, and scaling curves; both a refined and a freely redesigned version; commits pushed with a draft PR.

## How it was run
- The UI in a browser over the in-memory API (`ui/src/api/fake.ts`), now with demo data (`FakeApi({ demo: true })`: a full set of fields that follow the stage, the main gem's level, and the enemy's level; full gear with long flask tooltips). In `pnpm dev`, `?screen=skills` opens the sample build on a screen (`&dash=trends`, `&pin`, `&stage=2`, `&item=1`, `&field=life`), so a screen can be captured by a headless browser.
- Screenshots: headless Chrome at 1440 wide, before from `dev` `b89587b` and after from this branch, in `ux-audit/`. All numbers and items in them are sample data.
- Not covered: the packaged app (the WebView2 shell) was not rebuilt and driven for this audit; a screen reader pass; the start page and the import review, which the walkthrough reworked.

## Findings and fixes
Severity: **High** gets in the way on every visit; **Medium** costs clarity on a common task; **Low** polish.

### A. The frame
| ID | Finding | Sev | Fix (done) |
|---|---|---|---|
| A-01 | **The header and the side menu scroll away with the page.** `.app` was `min-height: 100vh` with the page as the scroller: on a long page (the dashboard, 1,299 px at 1440 x 900) the side menu was 1,242 px tall, its lower group (AI connection, About, Close build) below the fold, and Save and the stage picker out of reach once scrolled. | High | The workspace is the window: `.app` is `100dvh` with the header and the side menu fixed in the grid and `.content` the only scroller (a stable scrollbar gutter, so pages do not shift). A new page starts at its top. |
| A-02 | **Hover displays can be cut off.** The item tooltip was placed once, from the size it had before its text settled, and never again; a tooltip taller than the window had nowhere to go. The passive node card was placed at the pointer inside the canvas with no check at all, so it ran off the right and bottom edges. | High | One rule for every hover display (`ui/src/floating.ts`): measured, placed beside its anchor (or by the pointer), flipped to the other side when it does not fit, clamped to the window, and scaled down (never below 60%) when taller or wider than the window, as the game shrinks a long tooltip. It is placed again when its own size or the window's changes, and follows its item when the page scrolls. Unit tests cover the flask row at the window's lower edge, the right edge, a tooltip taller than the window, and all four corners for the node card. |
| A-03 | **The side menu is text only and cannot give its width back.** | Low | Icons (our own line drawings, ADR-000) beside each name, the current page marked by an accent bar as well as its fill, and "Collapse the menu": a 60 px rail of icons with each page's name as the button's tooltip and accessible name. Kept in the UI preferences (`rail`). |
| A-04 | **Three pills in the header** ("Partial · 1 warning", "Unsaved changes", "Up to date"), the last one green for the normal case. | Low | "Up to date" is a small dot and muted text; running and failed keep their badge. |
| A-05 | **Pages stretch to any width**: at 2,560 wide a stat's label and its value are a metre apart. | Low | Pages keep to 1,320 px, centred. |

### B. Text density
| ID | Finding | Sev | Fix (done) |
|---|---|---|---|
| B-01 | **Every page opens with a paragraph** (Skills: 49 words; Configuration: 36; AI connection: 37; Warnings; the tree's canvas help under the canvas). They are read once and then cost a line or three on every visit. | Medium | A `Hint` beside the page's heading: an "i" that shows the same text on hover, on keyboard focus, and on a press, placed inside the window; the text is the control's description for a screen reader. |
| B-02 | **The item set picker explains itself in the page head** ("The calculation uses this set. Switching is one undoable step."). | Low | The same `Hint`. |
| B-03 | **A stat list shows every row**: Defence is 17 rows on a real build, the headline (Hit DPS, Life) wherever the engine's order puts it. | Medium | See D-02. |

### C. Equipment, skills, tree
| ID | Finding | Sev | Fix (done) |
|---|---|---|---|
| C-01 | **The inventory is an island in a wide black field**, and the item list starts below the fold. | Medium | The Equipment page is two columns (the brief's screen 7): the inventory, which stays in view, and beside it "Paste an item", the items in use, and the items not in use. The inventory's dark ground hugs its slots. One column under 1,100 px. |
| C-02 | **The inventory spills out of a narrow panel** (a half-width dashboard cell at 1,024 wide). | Medium | The slots' size follows the panel's width (a container query). |
| C-03 | **The tree's controls take three rows and a line under the canvas**, and the canvas is a fixed 70% of the window's height. | Low | The heading row holds the page's help (a `Hint`) and the tree's own actions; one row of controls with the points as chips; the canvas fills what the window leaves. |
| C-04 | **The dashboard's main skill panel is two lines of text**: no gem colours, nothing about the build's other skills. | Low | Gem marks (round for a skill, square for a support, as on the Skills page), and "Other skills": each other enabled group in a line with its support count. |

### D. Dashboards
| ID | Finding | Sev | Fix (done) |
|---|---|---|---|
| D-01 | **Five panels and a stat tile, two widths, four dashboards.** | Medium | A six-column grid with four widths (a third, a half, two thirds, full), up to eight dashboards, and ten new components (below). The library is grouped (Summaries, Defence, Charts and comparison, Build) with a search; a chart or a stat tile can be added as often as wanted, a summary once. |
| D-02 | **Lists are long and unordered** (B-03). | Medium | The headline stats come first (Hit DPS, average hit, hit rate, crit; life, energy shield, mana, armour, evasion, block, resistances); a list longer than eight rows shows six and "Show all N". The default Overview opens with three stat tiles (Hit DPS, Life, Energy shield). |
| D-03 | **Nothing can be compared.** A player changes a gem and has to remember the old number. | High | **Pin to compare** on the dashboard keeps the numbers as they are; from then on every stat row and tile shows its change (▲ or ▼, the amount, the percent; a glyph and words, never colour alone), the breakdown's heading too, and the **Compare** component lists what changed, most changed first, as bars from the middle, with a table of every stat one click away. Kept for the session, never in a build. |
| D-04 | **No charts.** | High | Our own SVG marks (`ui/src/charts.tsx`; no chart library: the app is offline and its dependencies few): a line chart with round gridlines, the build's current point marked, a target line, and a readout that follows the pointer or the arrow keys; meters with a cap mark; bars from the middle; a sparkline. One measure per chart, one axis. Every chart opens to a larger view with its numbers as a table. |
| D-05 | **Nothing shows how the numbers move over time.** | High | Four components, one per meaning the maintainer chose: **Stat over your changes** (the stat after each change this session, from the trail the UI keeps; also a sparkline on every stat tile and a chart on the breakdown); **Progression by stage** (chosen stats at each stage, one small chart each on its own scale); **Scaling curve** (a stat against the main gem's level, the character's level, or a configuration number); **Fight timeline** (damage dealt over time against a life to get through, with the time and the number of hits; an estimate, and it says so). The middle two ask the application's new what-if query (below). |
| D-06 | **The breadth of the calculation is not visible in one place.** | Medium | **All stats** (every calculated stat with a search, each row opening its breakdown), **Attributes**, **Life, energy shield, mana** (each pool with the reserved part lighter), and **Resistances** (each on one scale against the 75% mark, with how far under). |
| D-07 | **A component's choices need Customize.** | Low | A chart's own options (which stat, against what, the damage to deal) are a small popover on its panel, always there; Customize is for the layout. |

## Before and after
Sample data; 1440 wide. Before is `dev` `b89587b` with its small sample build; after has the demo data.

| Screen | Before | After |
|---|---|---|
| Dashboard, Overview | [before](ux-audit/before-dashboard.png) | [after](ux-audit/after-dashboard.png) |
| Dashboard, Defence (pools, resistances) | | [after](ux-audit/after-defence.png) |
| Dashboard, Trends (compare, history, stages, curve, fight), numbers pinned and the stage switched | | [after](ux-audit/after-trends.png) |
| Equipment | [before](ux-audit/before-equipment.png) | [after](ux-audit/after-equipment.png) |
| Skills | [before](ux-audit/before-skills.png) | [after](ux-audit/after-skills.png) |
| Passive tree | [before](ux-audit/before-tree.png) | [after](ux-audit/after-tree.png) |
| Configuration | [before](ux-audit/before-config.png) | [after](ux-audit/after-config.png) |

## The what-if query (app API v1, additive)
The charts by stage and the scaling curves need the calculation at states the build is not in. The application gains one read-only query, `what_if` (`loom_app::what_if`, Tauri command `what_if`): the calculated fields at each stage (loadout) of the build, or with one input swept over a range (a gem's level, the character's level, a numeric configuration input). It runs the existing calculation on copies of the build off the writer thread, as the item edit preview does: it never changes the build, adds no undo step, and changes no calculation (`loom-calc` is untouched). At most 41 points a request; `APP_WHAT_IF` for a range it refuses. It is not an MCP tool (the tool list is fixed, ADR-017).

## What the engine cannot say yet
The fight timeline is a straight line because the engine computes hits only: no damage over time, no ramp from stacks, no buff uptime. When the engine computes those (the plan's later calc phases), the timeline should take them; until then it is labelled an estimate. The resistance bars mark 75%: a build's own maximum resistance is not a field yet.

## Left for later (recommendations)
| Item | Why not now |
|---|---|
| Dragging components to arrange them | The shell turns off HTML5 drag and drop on Windows to receive dropped files (`loom-tauri`); it needs a pointer-event drag. The arrow buttons and the width picker do the same job with the keyboard. |
| Dashboards per build | They are per user (`ui-prefs.json`), as B-5 decided; a build that wants its own set needs a place in the build document (an ADR-009 question). |
| Pinned numbers that survive a restart, and comparing against a named snapshot or another build | The pin is session state. Kept snapshots belong with the build document. |
| Stage against stage on the tree and the equipment page | The progression table compares every stat by stage; a visual diff of two trees or two item sets is its own piece. |
| A compact Skills view | The page is clear at the Phase 4 build sizes; a build with twelve socket groups wants the brief's table view (screen 10, D). |
| A light theme | The tokens are in one file and the contrast test would hold it to AA; nobody asked. |
| Start page and import review density | Reworked in the walkthrough sprint; left alone here. |

## Evidence
- Rust: `cargo test -p loom-app --locked --test what_if` (8 tests: one point per stage with the build unchanged, a gem level equal to the real edit, the character level, a config number, refused ranges, cancel, data that cannot calculate, the app's own call), the serde test of the wire form in `loom-serialization`, the command list test in `loom-tauri` (`ipc_workflow`), clippy on the three crates, `cargo fmt --all --check`, `loom-schema-gen --check`. The synthetic test bundle computes little, so the gem and character level tests prove equality with the real edit, not a moving number; a real bundle was not run. The stage series re-picks the main skill as `use-loadout` does, up to about twelve calculations a stage that do not poll cancel (the known `main_skill` cost): not timed on a real build with many stages.
- UI: `pnpm test` (94 tests; new `src/test/audit.test.tsx`: hover placement, chart axes, preference reading, the menu rail, pin and compare, the stage, curve, and fight charts with their tables, short lists), `pnpm typecheck`, `pnpm lint`, `pnpm format:check`; the dashboard test in `ux.test.tsx` follows the new width picker and the limit of eight. The test timeout is 20 s (`vite.config.ts`): the workflow tests passed the default 5 s on a busy machine.
- Contrast: the chart tokens are in `theme/pairs.ts` (3:1 for marks on a panel and in a dialog); `theme.test.ts` passes.
- Screenshots: `ux-audit/` (before and after).

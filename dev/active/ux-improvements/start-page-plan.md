# Pick-list group 2: start page B, game data found for the user, new build

Status: reviewed 2026-11-14 (plan-reviewer: proceed with changes, no blockers; taken as below). PR A done 2026-11-14 (PR #111, branch `feat/start-page`; `docs/ui/ux-walkthrough.md`, "Start page B, PR A"); PR B done 2026-11-14 in UX sprint 2 (branch `feat/ux-sprint-2`; `docs/ui/ux-walkthrough.md`, "UX sprint 2"). Maintainer 2026-11-14: "Finish what ui tasks you can until I can get you the bundle" (B-2's data PR 2 waits on the schema 4 bundle), taken as the pick of group 2 (`docs/ui/ux-walkthrough.md`, "Pick list"): B-1, B-3, B-4, UX-17, the open halves of UX-19 and UX-20. One PR from `dev`, branch `feat/start-page`.

## Review outcome (2026-11-14)

- **Split (ADR-012).** PR A (this branch): sections 2 to 4, and from section 1 only "no error up front" (UX-17). PR B: section 1, reduced as below.
- **Section 1 reduced to the reviewer's simpler alternative.** No copy into `bundles/`: installing an unsigned folder at P-e's signed-store paths would need a migration when P-e lands (ADR-010: unsigned packs are quarantined) and could shadow the shipped signed bundle; copying a chosen folder is hostile-input code that belongs to P-e's installer. PR B scans `bundles/` (what P-e will install), keeps the folder picker labelled "unsigned (development)", adds `use-bundle`, and caches loaded bundles by ID (`GameData::load` leaks one per load).
- **Pinned data (ADR-010, P-e D-a).** PR B: when a build's pinned data version is not the loaded one but an installed bundle matches it exactly, the error names it and offers "Use"; never a silent switch.
- **New build:** made through the import path from a template; it carries what any empty PoB build carries (the build reads as an import of that file, with no origin path). A bundle with no known tree version (not in `TREE_VERSIONS`, the synthetic one) gives a spec with no `treeVersion` (PoB reads its latest; the calculation reports `unchecked-tree-version`).
- **Recent file:** `VERSION` stays 1 and the new field is optional; an older app drops a newer list, which ADR-009 allows (the file is a cache). Said in the PR.
- **Keys:** Ctrl N and Ctrl O call `preventDefault` (WebView2 binds them); the page's `paste` handler ignores pastes into a field. Checked in the real app.
- **File drop:** Tauri's drag-drop event turns off HTML5 drag and drop in the webview on Windows; nothing uses it today; noted for the tree editor and gem reordering.
- **MCP:** `schemas/mcp-tools.v1.json` is a fixed list; `new-build` and `use-bundle` are not exposed (ADR-017).
- P-e's text says `manifest.json`; the code's manifest is `bundle.json`: fixed in PR B with the P-e note.

## 1. Game data found for the user (B-1, UX-17) — original draft, superseded by the review outcome

Phase 5's plan (P-e) installs signed bundles side by side in `<app data>/bundles/<game>/<steam build>-s<schema>/` and keeps `LOOM_BUNDLE_DIR` and the folder picker for development, labelled unsigned. This PR does the development-side part with the same layout, no signing:

- `loom_app::data::installed(data_dir)`: the bundles under `<data>/bundles/*/*/`, read from each `bundle.json` manifest only (game, Steam build, data schema), skipped when unreadable. Cheap; the full validation stays at load.
- **Start order**: `LOOM_BUNDLE_DIR`; else the remembered choice (`settings.json`, now an installed bundle's ID or a folder); else the newest installed bundle (highest Steam build, then schema). A remembered bundle that fails to load falls back to the newest installed one, and the snapshot says so.
- **"Add game data…"** (replaces "Choose data folder"): the user picks a bundle folder (the one holding `bundle.json`); the host validates it (`validate`, as load does), copies it into a temporary folder under `bundles/`, checks the copy's file hashes against the manifest, and renames it into `bundles/<game>/<build>-s<schema>/` (an install of the same ID with the same manifest hash is a no-op; another manifest under the same ID is refused with `APP_BUNDLE`). Then it loads it and remembers its ID. The source folder can then be deleted.
- **Switching**: App API v1 (additive) `DataStatus.installed: Vec<InstalledBundle {id, game, steam_build, schema, active}>` and command `use-bundle {id}`; About lists them with "Use".
- **No error up front** (UX-17, brief note 1): without data the start page shows one quiet footer line ("No game data yet", "Add game data…"); New build and import are disabled with that reason, instead of failing after a file is picked.
- Paths never cross to the webview (decision 3): installed bundles are named by ID; the picker stays on the Rust side.

## 2. New build (B-3)

`Command::NewBuild` (additive): a PoE1 build as PoB starts one: Scion, level 1, one empty passive spec on the bundle's tree version (`data::tree_version`), one skill set, one item set, one config set. Made from a fixed PoB XML template through the import path (`read_pob`, `resolve`), so it is read, exported, and saved like any build. Not dirty until edited, no path, named from itself (UX-15). Needs PoE1 data (`APP_NO_DATA` otherwise). Class and ascendancy choice waits for the tree editor (Phase 5 plan, `SelectClass`).

## 3. Start page B (B-4; the maintainer's choice of the four mockups)

- One **command bar**: pasting a share code or PoB XML imports it (as the textarea did; links still get UX-18's message); typing filters the recent builds by name.
- Below it the actions **New build (Ctrl N)**, **Import from clipboard (Ctrl V)**, **Open a file (Ctrl O)**. Ctrl V on the page with no field focused imports the clipboard text (the `paste` event, no clipboard permission); the button reads the clipboard with `navigator.clipboard.readText` and falls back to focusing the bar with "Press Ctrl+V" when that is refused.
- **Recovery** first when there is any (UX-19), then recent builds.
- **A file dropped anywhere** in the window: the host receives the drop (Tauri's window drag-drop event, Rust side, so the webview never sends a path): `.xml` imports, `.json` opens, else `UI_DROP` "not a build file". Unit-tested as a path to command function; the UI shows a drop hint while a file is over the window (the event's `enter` and `leave`).
- **Footer**: the data status as one line ("Game data ready · Path of Exile, game build 26530219") and the About link.

## 4. Recent builds: class, ascendancy, level, main skill (UX-20, open half)

`loom_persistence::recent::Entry` gains an optional `summary {class, ascendancy, level, skill}` written when a build is saved or opened (serde default; the file is a cache, `deny_unknown_fields` means an older app drops a newer list, which loses no build). `api::RecentEntry` gains `summary` (additive). The start page shows "Inquisitor 92 · Smite".

## Not in this PR

- UX-19's other half (keeping the undo history across a restore): recovery stores the build, not its history; changing the recovery format is its own decision.
- UX-14 (patch version): needs the patch in the bundle manifest (extractor, a data run).
- Signing, quarantine, `.loombundle` archives, the data feed: Phase 5 P-e.

## Evidence

Workflow tests (start picks the newest installed bundle; a failed remembered one falls back; install copies, verifies, is idempotent, refuses a conflict; `use-bundle`; new build calculates, saves, exports; recent summary written and read; an old recent file still reads); `loom-tauri` drop-dispatch test; UI tests (command bar filter and import, Ctrl N/V/O, disabled actions without data, footer, recent summary); fake API updated; the real app over WebDriver (start without data, add data, new build, screenshots); clippy, fmt, wasm32 (core crates untouched apart from `loom-persistence`, which is not core), schemas regenerated.

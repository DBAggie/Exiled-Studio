# UI fidelity (2026-11-13)

The Phase 4 screens restyled to match the mockups in `docs/ui/exiled-studio-ui-draft-v1.zip`, the bundled fonts, the widened license allow-lists, and the issues found in the Phase 4 real-app checklist. Approved by the maintainer 2026-11-13 after PR #105: the redesign first, then the passive tree (its own plan, with an ADR-008 amendment). No new features: stages, dashboard tabs and Customize, the equipment drawing, skills, tree, configuration, and the Guided / Workspace switch stay in the plan's order.

## Screenshots

Taken from the UI on its in-memory fake API (`pnpm dev`, sample data), 1440 px wide, in headless Edge through the DevTools protocol. They show the app's own pages only; there is no game art. Compare each with the mockup's screenshot in the zip.

| File | Screen | Mockup |
|---|---|---|
| `01-start.png` | Start | `01-Main.png` |
| `02-import-result.png` | Import result | `02-ImportReview.png` |
| `03-dashboard.png` | Dashboard | `03-Workspace.png` |
| `04-breakdown.png` | Breakdown of Life | `04-Breakdown.png` |
| `05a-equipment.png` | Equipment list | (none; list form of `07-Equipment.png`'s slots) |
| `05-item-editor.png` | Item editor with an edit previewed | `05-ItemEditor.png` |
| `06-ai-connection.png` | AI connection with a pending request | `06-AiApproval.png` |
| `07-warnings.png` | Warnings | (none; the review list of `02-ImportReview.png`) |

## What changed in the look

- **Fonts**: Inter, Space Grotesk, and JetBrains Mono are bundled from the `@fontsource-variable` packages (5.3.0, SIL OFL 1.1; weight axis only) and served from the app, never a CDN. Each license text ships in `ui/public/licenses/` and a test checks the copies equal the packages'. Ligatures are off in monospace text so `--` and `->` show as typed.
- **Tokens**: the mockups' surfaces (header, menu, hover, selected), row lines, link colour, and tinted badge fills, all in `ui/src/theme/tokens.css`; every pair the CSS uses is in `theme/pairs.ts` and passes WCAG 2.2 AA in `theme.test.ts`. Button borders use the mockups' `#3A414C` (a button is identified by its label); text fields and checkboxes keep the 3:1 border.
- **Header**: logo mark (the project's own drawing), build name, status pill ("Partial · 1 warning"), unsaved pill, calculation pill, icon Undo and Redo, Export with a chevron, Save.
- **Side menu**: 44 px items, filled current item, round count badges (warnings; requests waiting), AI connection, About, and Close build at the bottom.
- **Start**: "What do you want to do?", the import card, side cards for opening a build and for the game data, recent builds as a table, the footer with the notice.
- **Import result**: status pills, the build's name, four count cards, lines to review with their codes, a side card with the character, main skill, and pinned data version.
- **Dashboard**: each stat is one row button (label left, value right in the heading font, badge after it); units (75%, 2.5 / s); negative resistances in orange; data version chip; warnings strip with Review all.
- **Breakdown**: breadcrumb, "Why is Life 70?", the factors the field's node combines ("How it is built"), the contributions with their sources ("Where it comes from", items by name), what is not counted, and every step folded below.
- **Item editor**: header card in the rarity colour, status legend, one row per line (section, text field, game stats under it, status, × remove), a warning row tinted, Add a modifier, and the "Impact of this edit" and Apply panels beside it. The rarity, name, base, and separator lines are not listed again under the header that shows them, unless one carries a warning (ADR-015: a warning is never hidden); every line stays in the item and its export.

## The checklist's issues (Phase 4 review notes, section 2)

| Issue | Fix |
|---|---|
| A change request showed its edit as raw JSON | Remove and Add blocks in words with the item named: "Remove from Lich's Circlet: +15 ...", "Add to Lich's Circlet: +30 to maximum Life" (a test checks no JSON is shown). |
| The effect listed field IDs in a code font | Effect chips with the dashboard's labels and units: "Life 4,111 → 4,169"; the fake API now attaches a preview to a request as the app does. |
| Permission checkboxes above their labels | Each checkbox on its label's line, the help text linked by `aria-describedby`, the setting's state as a badge. |
| Dashboard values a few pixels above their stat buttons | The row is one button; label, value, and badge align on one line. Each row is named "Life: 4,111, Exact." for screen readers. |
| Hit chance "Not yet" for a self-cast spell | **Checked, not changed; needs the maintainer.** PoB v2.67.2's `BuildDisplayStats.lua` shows `HitChance` only for an attack (or an enemy with spell block, not modelled), and a self-cast spell has no `HitSpeed`, so "Not needed" (the brief's grey badge) is right for both. Saying so needs a new reason in the calc result (`not-applicable`), and calc result v1 takes new enum variants only until the Phase 3 gate (`loom_serialization::dto::calc::v1`, "Adding to v1"; Phase 2 plan, decision 14), which is met. The options: (a) reopen v1 for additive reasons until the first release, or (b) a v2 with a migration. Drafted and reverted in this PR after review; nothing in the engine changed. |
| MCP server under `D:/` vs `d:/` | The command now uses `--scope user`; the Phase 4 report's step 7 says why. |
| Token changes on every start; add a copy button | **Copy command** copies the full command with the token; the screen says to copy it again after a restart. |
| The data folder prompt does not say which folder | The start screen, the errors, and the native folder dialog's title name "the folder that holds bundle.json". |

## Licenses (maintainer decision 2026-11-13)

"We can open up licenses that are still considered open source." `ui/scripts/check-licenses.mjs` and `deny.toml` now allow OSI-approved licenses (permissive, MPL-2.0, EPL-2.0) and OFL-1.1 for fonts; the named MPL-2.0 exceptions (`axe-core`, `lightningcss`, and Tauri's `cssparser` family) are no longer needed. Strong copyleft (GPL, AGPL, LGPL) is not on either list, because ADR-005 keeps GPL-derived code out of the shipped app; the JavaScript check names it as such when it fails. `argparse`'s `PSF-2.0` stays a named exception.

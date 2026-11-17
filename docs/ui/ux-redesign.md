# The free redesign (2026-11-15)

The maintainer asked for two versions of the UX audit's result: the design language refined (`ux-audit.md`, branch `feat/ux-audit`, PR #128) and a free redesign. This is the second: branch `feat/ux-audit-redesign`, PR #129 (draft), stacked on the first, so it has every layout and component change of the audit and differs only in how the app looks. The redesign is a token change, one stylesheet, and one control.

**The maintainer chose the redesign** (2026-11-16, after running both versions in the desktop shell: "Let's go with the second one"): PR #129 is the version to carry forward, on top of #128; `docs/ui/UI-DESIGN-BRIEF.md` section 2 and decision 14 hold its values.

## The direction: numbers first
The brief's look (amber on warm dark, Space Grotesk headings) reads as a game companion. The redesign reads as an instrument: the frame recedes and the numbers carry the page.

| | Refined (the brief) | Redesign |
|---|---|---|
| Surfaces | Warm dark: `#101215`, panels `#181b20` | Neutral near-black: `#0b0c0f`, panels `#121419`; the side menu is the page's ground, not a panel |
| Accent | Amber `#e8a33d` | Violet `#a995ff` (links `#c3b5ff`); it is no status's colour (green, blue, orange, red), no rarity's, and not the charts' blue |
| Type | Space Grotesk headings and numbers, Inter body | Inter throughout, by weight and size; tabular figures for every number; Space Grotesk only in the brand mark |
| Panel titles | Headings (16 px) | Small capital labels (11 px, muted): the title names the panel, the number leads |
| Stat tiles | A number in a panel | A larger number (32 px) under an accent rule |
| Controls | Filled buttons | Outlines until pressed; one filled button a screen (Save) |
| Dashboard tabs | Bordered buttons | Text with a rule under the one shown |
| The stage | A drop-down in the header | A stepper (Leveling › Endgame): the build's progression, one press each; a drop-down still when there are more than five stages or the sets are mixed |
| Side menu rail | Icons only, names as tooltips | Icons with their names under them |
| Corners | 8 and 10 px | 10 and 14 px |

Unchanged on purpose: the game-style item tooltip and inventory (the maintainer's rule: they mirror the game), item rarity and gem colours, the status badges' colours and words, the charts' marks.

## What it is in the code
- `ui/src/theme/tokens.css`: the surface, line, text, accent, and tree-allocation values; `--font-heading` is Inter and `--font-brand` is Space Grotesk.
- `ui/src/redesign.css`: the restyle, loaded last. Removing its import (and restoring the tokens) gives the refined version back.
- `ui/src/App.tsx`: the stage stepper (`StagePicker`).
- Two tests follow the stepper (`ux.test.tsx`, `audit.test.tsx`).

## Evidence
- Contrast: every pair in `theme/pairs.ts` holds with the new values (`theme.test.ts`: text 4.5:1, marks and focus 3:1).
- `pnpm test` (94; 98 with `dev`'s #134 merged in; 109 with the audit's follow-up, whose new controls take this look unchanged), `typecheck`, `lint`, `format:check`.
- CI on PR #129 (2026-11-16, at `5c9e4ed`): every required job passed, among them `e2e (ubuntu-24.04)`, which drives the real shell with the stage stepper; `e2e (windows-latest)` fails before the app starts, as on `dev`.
- Screenshots (sample data, 1440 wide): [dashboard](ux-redesign/redesign-dashboard.png), [defence](ux-redesign/redesign-defence.png), [trends](ux-redesign/redesign-trends.png), [equipment](ux-redesign/redesign-equipment.png), [skills](ux-redesign/redesign-skills.png), [passive tree](ux-redesign/redesign-tree.png), [configuration](ux-redesign/redesign-config.png). Compare with `ux-audit/after-*.png`.

## Not done
- The start page, About, and the import review take the tokens and the control styles but were not composed for the new look (the start page and the import review were looked at in a browser with it after #128's density pass, 2026-11-16).
- The passive tree's sprites (GGG's, from a data bundle) were not checked against the violet allocation colour: the browser run draws the tree in the app's own shapes.
- The packaged app was not rebuilt with it.
- `UI-DESIGN-BRIEF.md` section 2 (visual language) is rewritten to these values (2026-11-16, when this version was chosen); the brief's mockups and its live canvas still show the first draft's look.

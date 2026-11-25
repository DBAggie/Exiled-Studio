# UI revamp plan (2026-11-25)

The maintainer asked for a review of the whole UI and a plan to revamp it (2026-11-25, with the `unslop` skill installed in the maintainer's personal skills folder). The review ran the application on the Raise Spectre community fixture at 1,280 px and read the CSS. This file is the audit, the direction the maintainer approved, and the steps. Nothing in the application is changed by this plan.

**Standing rules that apply:** mockups are approved before any step is built; the UI does not name PoB outside import and export; no game art (ADR-000); every colour is a token with its contrast pair checked (`ui/src/theme/tokens.css`, `pairs.ts`).

## Decisions (maintainer, 2026-11-25)
- **Depth:** the full revamp, all five steps below.
- **Direction:** approved as written below.
- **Accent:** not decided. The maintainer asked to see alternatives; four are on the design canvas "Exiled Studio revamp" (https://claude.ai/artifact/3YUT56ZDe4Gmdq7iQvHvPB, private to the maintainer): violet used rarely, brass, star-chart blue, verdigris. Step 1 waits for the pick.

## Audit
The verdict is light slop with a strong template look. There are no emoji, gradients, glows, or looping motion, the icons are one line set, and colours come from tokens. What is wrong is that near-black, Inter, and a violet accent is the default dark application, and several craft issues make screens look unfinished.

| Area | Finding | Where |
|---|---|---|
| Colour | The accent is on everything, so nothing leads: Save, four stacked Restore buttons, links, tabs, checkboxes, tree nodes, a mark on each stat card | every screen; start page |
| Colour | The amber "Partial" pill repeats on every row of a panel | Dashboard, Defence |
| Colour | Four pills in the header and page head at once | shell |
| Colour | Three ink colours for text the user acts on | Warnings |
| Type | Uppercase tracked labels on cards and table headers (7 rules) | Dashboard, start page, Equipment |
| Type | 24 distinct font sizes, six weights (400 to 700), negative letter spacing | `index.css`, `dash.css`, `redesign.css` |
| Spacing | Off a 4 px grid: 10 px 45 times, 6 px 36 times, 14 px 12 times | the same files |
| Layout | The header wraps to two rows at 1,280 px; Undo and Redo are apart | shell |
| Layout | The Inputs column wraps at three words a line, breaks technical names mid-word, and shows names the "technical detail" rule hides | Configuration |
| Layout | An unstyled box around the update modes; a narrower column than other pages | About |
| Layout | An outlined "Show technical detail" button on every row | Warnings |
| Layout | Near-equal cards with no lead; the second card is a paragraph saying Offence is not calculated | Dashboard |
| Details | Eight corner radii (2, 3, 4, 6, 8, 10, 14 px, pill) | CSS |
| Copy | A headline that fits any product; four recovery rows all named "Imported build"; periods on short hints | start page, throughout |
| Moment | The passive tree, the one thing only this product has, draws as small dots under controls with no surface | Passive tree |

## Direction
- **Product:** a build planner for Path of Exile that calculates a character's numbers and shows why each one is what it is.
- **Character:** exact, patient, a little arcane.
- **World:** a cartographer's workbench: the passive tree as a star chart, ink and instrument markings, lamp light on a dark desk.
- **Moments:** the passive tree drawn with care, and the reason behind a number (the breakdown).
- **Signature:** one small completeness mark beside every number (a filled circle for exact, a half for partial, a ring for not counted), which opens the number's reasons. It replaces the amber pills. It tells by shape, so it does not depend on colour.

Surfaces stay neutral grey. The world shows in the tree, the charts, and the mark, never in the backgrounds.

## Steps
Each step is its own PR from `dev`, with its mockups approved first and its check in the running application.

1. **Foundations.** Tokens only, no layout change: five type sizes, weights 400, 500, 600, sentence-case labels, a 4 px spacing scale, three radii (control, card, panel), the accent budget (selection and the allocated tree path; the primary button off-white with dark text unless the accent pick says otherwise). The contrast pairs test is updated with the tokens.
2. **Shell.** A one-row header with Undo and Redo together, status as quiet text, a side menu of 34 px rows.
3. **The completeness mark.** One component used by the dashboard cards, the panels' rows, breakdowns, and tables; the "Partial" pills go.
4. **Screens, worst first.** Configuration, Warnings, the start page, About, then the Dashboard (a lead row of compact figures, varied widths), Equipment, and Skills.
5. **The passive tree.** Node shapes by kind, link weight, the allocated path's colour, surfaces for the floating controls, labels.

## Open
- The accent (above).
- Whether figures are set in the bundled mono face (the mockups show JetBrains Mono for numbers; Inter stays for text). Decided with step 1's mockups.
- The mockups were not rendered here (the canvas is checked by the maintainer).

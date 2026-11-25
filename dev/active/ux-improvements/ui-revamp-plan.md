# UI revamp plan (2026-11-25)

The maintainer asked for a review of the whole UI and a plan to revamp it (2026-11-25, with the `unslop` skill installed in the maintainer's personal skills folder). The review ran the application on the Raise Spectre community fixture at 1,280 px and read the CSS. This file is the audit, the direction the maintainer approved, and the steps. Nothing in the application is changed by this plan.

**Standing rules that apply:** mockups are approved before any step is built; the UI does not name PoB outside import and export; no game art (ADR-000); every colour is a token with its contrast pair checked (`ui/src/theme/tokens.css`, `pairs.ts`).

## Decisions (maintainer, 2026-11-25)
- **Depth:** the full revamp, all five steps below.
- **Direction:** approved as written below.
- **Accent:** violet, used rarely (board A; "I like A", 2026-11-25), picked from four on the design canvas "Exiled Studio revamp" (https://claude.ai/artifact/3YUT56ZDe4Gmdq7iQvHvPB, private to the maintainer; the others were brass, star-chart blue, verdigris). The accent marks the selection, the active tab, links, and the allocated tree path; the primary button is off-white with dark text.

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

## Step 1, built (2026-11-25)
PR #169, branch `feat/ui-revamp-foundations`, stacked on `feat/open-items` (PR #168 is not in `dev` yet, and the screens it restyles are its own). Approved by the maintainer from the canvas's Foundations, Start page, and Configuration boards ("Approved, use mono for figures, build step 1").

- **Tokens** (`ui/src/theme/tokens.css`): the graphite surfaces and text roles, `--c-primary` and `--c-on-primary` (the off-white primary button), `--font-figure`, the five `--fs-*` sizes, the four radii. `--c-on-accent` is gone with its use. The contrast pairs (`pairs.ts`) cover the primary button and its hover; muted text is `#959BA6`, since the mockup's `#8A909B` missed 4.5:1 on a selected row.
- **The three stylesheets**, by one pass over every declaration: 75 font sizes onto the five tokens, 42 weights onto 400, 500, 600, 7 uppercase rules and all 9 letter spacings removed, 89 paddings, margins, and gaps onto the 4 px scale, 16 radii onto the tokens. Figures (values, counts, chart ticks, table cells, number inputs) are in the mono face.
- **The accent budget:** the primary button, the stat card's corner mark, the import box's border, and its icon no longer use the accent. The start page's Restore buttons are plain buttons (five filled buttons in a column were louder off-white than violet).
- **One layout value changed**, because the text grew: the collapsed side menu is 104 px wide (was 84), so its labels fit at 12 px.
- **Not changed** (later steps): the header's two rows, the status pills, the "Partial" pills, every screen's layout, the tree's drawing.
- **Checked in the running application** on the Raise Spectre fixture at 1,280 px: every screen, the collapsed menu (no label cut), no sideways scroll. Measured on the Dashboard: no uppercase text, no letter spacing, radii 4, 8, 12 px and the 2 px marks.

## Step 2, built (2026-11-25)
PR #170, branch `feat/ui-revamp-shell`, stacked on step 1's (#169). The mockup is board A's header and side menu ("Board A is approved, build step 2").

- **The header is one row** (`Header` in `ui/src/App.tsx`, "The revamp's shell" in `redesign.css`): the build and its state at the left; "Up to date", the stage, Undo and Redo side by side, Export, and Save at the right. Its controls are 32 px tall.
- **The state is one quiet line of text**, not pills: "Partial · 5 warnings · Unsaved changes". Only a failure keeps its colour.
- **A narrow window:** the state gives way first, then the name (its whole text on hover); under 1,000 px the row wraps to two.
- **The side menu:** 34 px rows (were 40), the page shown marked by a bar and its icon in the accent, counts as quiet figures instead of amber pills.
- **Checked in the running application** on the Raise Spectre fixture: one row of 52 px at 1,100, 1,280, and 1,600 px, two rows at 900 and 1,000 px, no sideways scroll at any; the menu's rows measured 34 px.
- **Seen and left for step 4:** the import review's "Lines to review" column is squeezed to a few words a line at 1,280 px.

## Step 3, built (2026-11-25)
PR #171, branch `feat/ui-revamp-mark`, stacked on step 2's (#170). The mockup is board A's stat card and Defence rows ("Board A is approved, build step 3").

- **`StateMark`** (`ui/src/components.tsx`) takes the place of the state pill everywhere it was drawn: the dashboard's stat cards and panel rows, the panels in `DashPanels.tsx`, and a breakdown's heading. A filled circle is Exact, a half Partial, a ring Not yet, a dashed ring Cannot settle; in the secondary text colour, so the state is told by shape.
- **Every number is marked** now, the exact ones too (before, only the exceptions carried a pill). The mark's name is its tooltip, and is read aloud except for Exact, which a row already says in its own name (UX-12 holds).
- **A stat card's mark is beside its figure.** A breakdown's heading writes the state's name beside the mark.
- **The legend** (`StateLegend`) is under the dashboard, once a page. The panel hint is now "Select a number to see why".
- **Checked in the running application** on the Raise Spectre fixture: the dashboard has no amber pill left (the only chip is the game version), the legend shows the four states, a breakdown's heading reads "Partial" beside its mark, no sideways scroll.
- **Not in this step:** other chips that are not a calculation state (the Configuration page's "Not calculated" and "Note, no effect", an item line's status, "1 with warnings"): they go with their screens in step 4.

## Step 4, first part, built (2026-11-25): the start page and Configuration
Branch `feat/ui-revamp-start-config`, stacked on step 3's. The mockups are the canvas's Start page and Configuration boards ("Build the two approved screens first").

**The start page** (`ui/src/screens/Start.tsx`):
- The title is "Builds". The import bar leads, with the page's one filled button; the other ways in are under it.
- Unsaved work is a panel beside the page (first, above it, under 1,000 px; first in the document either way), with Restore and Discard as quiet links.
- A recent build shows what it is under its name. The header's "Offline · no account" is quiet text.
- **Not as the mockup:** a recovery copy is still named by its build's name, which for an import without one is "Imported build" (the mockup showed class and skill; `RecoveryEntry` does not carry them, an app API addition); no "Discard all"; the button still reads "Import build", and the link to import a Path of Building file is kept.

**Configuration** (`ui/src/screens/Config.tsx`):
- An input is one row: its question on one line, its help under it, what the calculation does with it (the ring mark and "Not calculated yet", or a quiet "Changes no number"), its control, Remove. The technical name is shown only with **Show technical detail**.
- **Find an input** filters the inputs by their words or technical name.
- The inputs take the room; the custom modifiers are a 26 rem column beside them (from 1,100 px).
- A custom modifier block shows its lines as they were read: a counted line with the filled mark, one not calculated with the ring, a note of the user's as a quiet heading. **Edit as text** opens the text (open at once for an empty block).
- **Not as the mockup:** the League picker and the "N inputs are not calculated yet" sentence stay; the note headings are the user's own text as written (the mockup tidied their wording).

**The header again** (found while checking, fixed here): at the real 1,280 px window the name ended in an ellipsis while the state was still showing, since the two shared the loss; the state now gives way entirely first, and "Up to date" is its dot alone, named on hover and for a screen reader, so the name and the state both fit at 1,280 px.

**Checked in the running application** on the Raise Spectre fixture: Configuration's questions each on one line (20 px), no sideways scroll at 1,000, 1,280, and 1,600 px, no technical name shown until asked; the start page with eight recovery copies and one recent build.

## Open
- Step 4's other screens need their mockups first: Warnings, About, the import review (its "Lines to review" column is squeezed at 1,280 px), the Dashboard's layout, Equipment, and Skills.
- A recovery copy's class and main skill (an additive `RecoveryEntry` field), so unsaved work is told apart when the builds have no name.
- The mockups were not rendered here (the canvas is checked by the maintainer).

# Exiled Studio: UI design brief (rough draft v1)

Status as of 2026-11-11. This brief records everything settled during the first UI drafting
sessions so a future session can continue without repeating the work. Read it before changing the
designs.

- Repository: https://github.com/DBAggie/Exiled-Studio (reviewed on `main` plus the open stacked PRs
  up to P2-9 / PR #56, `feat/p2-9-migrations-persistence`).
- Live design canvas (private to the owner): https://claude.ai/artifact/ECz5DeeNukjcxPEiPiCh9B
  The editable source of that canvas is in `canvas-source/` in this package.
- All numbers, items and builds in the mockups are **sample data for layout only**. Nothing is
  calculated.

---

## 1. The product, as understood

Exiled Studio is a free, open-source (MIT), **offline desktop build planner for Path of Exile**. It is
an independent rewrite meant to do the job Path of Building does today, with two goals:

1. **Fidelity to the game.** Item modifiers are matched to the game's own stat descriptions and stat
   IDs instead of English-text guessing. Path of Building v2.67.2 is pinned as a regression check;
   every intentional difference is recorded with evidence.
2. **Explaining every number.** Any stat can be broken down to the sources and game stats behind it.

Core flows (Phase 4 scope in the repo plan): import a PoB file or share code → dashboard → breakdown
viewer → full item editor (line status, comparison, undo) → background recalculation → save as the
app's own JSON → export back to PoB without losing anything.

Other facts the UI must respect (from the repo's ADRs):

- **Lines that can't be read are never dropped.** They are kept and shown, are not used in
  calculations, and export unchanged. Unsupported calculations show as partial or unsupported,
  never as a confident wrong number.
- **No Grinding Gear Games art or audio.** No game icons, tree art or item pictures.
- **Offline, no account, no telemetry.** Game data ships as signed bundles that the project extracts
  itself. Each build stays on the data version it was made with until the user upgrades it.
- **AI connection (MCP).** Local only and read-only by default. Any change an AI tool proposes needs
  the user's approval and is undoable.
- **Interface (ADR-008).** Two modes: guided task entry points, and a workspace with panels. The
  passive tree will be drawn on a canvas, with an accessible overlay and a keyboard and screen-reader
  equivalent for everything.
- **Standards.** WCAG 2.2 AA. All text written so it can be translated later. Windows and Linux are
  required; macOS is best effort.
- **Games.** PoE1 first; PoE2 data is already in the pipeline (calculations come later).
- **About screen.** Must state the project is not affiliated with Grinding Gear Games or the Path of
  Building Community project.

---

## 2. Visual language (draft)

| Token | Value | Use |
|---|---|---|
| Background | `#101215`, panels `#181B20`, borders `#2A2F37` | Dark theme throughout |
| Text | `#E9E6DF`, muted `#A3A9B3` | Muted text still passes 4.5:1 on panels |
| Accent | `#E8A33D` (amber) | Primary buttons, selection, focus of attention. A Tweaks control exists to try `#7FB2E5`, `#9FD39B` or `#D98A6A` |
| Focus ring | `#8DC1F2` 2px | All interactive elements |
| Fonts | Inter (body), Space Grotesk (headings), JetBrains Mono (stat IDs, item text) | |
| Rarity | Normal `#C9CED6`, Magic `#8DC1F2`, Rare `#F2D97A`, Unique `#E39A6E` | Item borders and names |
| Gem attribute | Strength `#E0796A`, Dexterity `#7FC27A`, Intelligence `#6FA6E6`, White `#D6D9DE` | Gems, sockets, attribute cards |
| Links | Gold bar `#B9A57A` | Joins linked gems or sockets |

**Status badges.** These must stay consistent everywhere:

- **Green.** Matched, Exact, Met, Complete, Up to date.
- **Blue.** Adapted (text differs from the game's), info, Recalculating.
- **Orange.** Warning(s), Partial, "N short".
- **Grey.** Not yet, Not needed, Coming later, Edited.
- **Reddish "Str 120" tag.** Requirement not met.

**Item and gem pictures.** These are original line drawings by slot type (wand, circlet, shield,
body armour, gloves, boots, belt, ring, amulet, flask, jewel), never game art.

- **Gem shapes.** A skill gem is a hexagon and a support gem is a circle. Colour is the attribute.
- **Accessibility.** Colour is never the only signal: every status also has a text label or an
  `aria-label`.

---

## 3. Terminology (user-facing)

| Use in the UI | Meaning | Internal / code term |
|---|---|---|
| **Warning / Warnings** | A stat line with an issue: the app could not read it, so it is kept but not calculated | Quarantined line (`LINE_NO_TEMPLATE`, `LINE_AMBIGUOUS`, `LINE_NO_VARIANT_FITS`, …) |
| **Matched** | Line resolved to game stats | Resolved |
| **Adapted** | Matched only after a relaxed match (for example, letter case) | Adapted line (for example, case-folded) |
| **Edited** | Line changed by the user in this session | — |
| **Exact / Partial / Not yet** | Calculation support for a number | Completeness / support state |
| **Stage** | A point in the character's progression (Leveling, Early maps, Mapping, Endgame) with its own gear, passive tree and skills | Not in the model yet (see section 6) |
| **Linked group** | Gems in linked sockets that work together | Socket group |

The user explicitly rejected "held lines" as a term. Do not reintroduce "held" or "quarantine" in
UI copy.

---

## 4. Screen inventory

Screenshots are in `screenshots/` (full page, 1440 px wide). Source files are in
`canvas-source/project/`.

| # | Screen | Source | Purpose and key elements |
|---|---|---|---|
| 1 | Start screen (guided) | `Main.dc.html` | Main options: import from PoB (paste code or open .xml), open a saved build, start a new build (PoE1 / PoE2). Recent builds show status and data version. Footer carries the non-affiliation notice. |
| 2 | Import result | `ImportReview.dc.html` | Counts: lines read, matched, adapted, warnings. Lists the warning lines with reason codes and which parts of the build can be calculated. Shows the pinned data version. |
| 3 | Dashboard | `Workspace.dc.html` | Dashboard tabs (Overview, Defence, Bossing, +) and a **Customize** button. Rows of at most two components: Offence and Defence, then Equipment (compact visual) and Main skill group, then Warnings (full width). |
| 4 | Breakdown | `Breakdown.dc.html` | "Why is Total DPS X?": a chain of factors, a table of sources with game stat IDs, share by origin, and what is not counted (lines with warnings). |
| 5 | Item editor | `ItemEditor.dc.html` | Each line has a status (Matched, Adapted, Edited, Warning), editable values, and a remove button. A side panel shows the effect of the edit before Apply. Apply is one undoable step. |
| 6 | AI approval (MCP) | `AiApproval.dc.html` | Connected client and its permissions (read, suggest, apply = ask each time). A change request shows what is removed and added, the effect calculated by the app (not the AI), and Approve / Decline. |
| 7 | Equipment | `Equipment.dc.html` | Gear laid out roughly as the game arranges it, plus flasks and jewels. The item list is docked to the right (type, name, notable stats). The **Attributes vs requirements** panel compares Str/Dex/Int with the highest item requirement and flags items that can't be used yet. The view follows the global stage picker. Interactive. |
| 8 | Item breakdown | `ItemDetail.dc.html` | One item: the item card, "Without this item" (deltas when the slot is empty), what each line adds after the rest of the build is applied, an overcap note, and lines not counted. |
| 9 | Customize dashboard | `DashboardEdit.dc.html` | Up to 4 dashboards. Rows hold 1–2 components. Add a component into a chosen space or a new row; move up or down, swap, remove. Component library with search and categories. Interactive. |
| 10 | Skills | `SkillsA.dc.html` | **Cards / Table switch** (both kept while the owner decides). Cards: one per item, gold bar joins linked gems. Table: one row per item with inline sockets and links, boxed linked groups, colour counts. Interactive. |
| 11 | Skill group breakdown | `SkillGroup.dc.html` | For one linked group: each gem's role, DPS lost without each support, and interaction warnings (for example, Elemental Focus stops Arc from shocking). Opened by selecting a group on screen 10. |

**Global header** (all workspace screens): logo › build name › **Stage picker** (dropdown) ›
build status badge … status › **Export ▾** › **Save**.

**Side menu:** Dashboard, Equipment, Skills, Passive tree, Configuration, Warnings (count), and AI
connections (count) on some screens.

---

## 5. Decisions log (in order)

1. **Mockups are rough drafts with sample data, built over several rounds.** Static pictures first,
   with interaction added where it shows behaviour.
2. **Walkthrough video** made of screens 1–6 (`video/`). It is now **out of date**: it says "held"
   and predates the stage picker, the equipment screens, dashboards, skills and Export.
3. **Equipment page** uses a game-like gear layout with simple slot drawings. Selecting an item
   opens the item breakdown (screen 8). A quick-look hover card was tried; it can be turned off in
   the canvas Tweaks panel.
4. **Stages are global.** Item-set tabs inside the equipment panel were replaced by a **Stage
   dropdown in the header**. A stage is a point in the character's progression from level 1 to 100.
   Changing it swaps gear in every slot, **and also the passive tree and skills**. Default stages in
   the draft: Leveling (1–40), Early maps (68–84), Mapping (85–94), Endgame (95–100), plus
   "+ Add stage" and "Manage stages…". Items new since the previous stage get a blue dot.
5. **Dashboard equipment component** replaced the text list of equipped items with a compact copy
   of the equipment visual.
6. **Dashboards.** Up to **4 per build**. Users add components from a library and arrange them in a
   grid. **No more than 2 components side by side** in a row. The library will grow large; the
   draft shows 15, two marked "Coming later". Keyboard alternative to drag: move up, move down,
   swap, remove.
7. **Skills.** Four variations were drafted (A item cards, B socket layout, C skill-first,
   D compact table). The owner **dropped B and C**, kept **A and D behind a Cards/Table switch**
   while deciding, and **kept C's per-group breakdown** as the page opened when a linked group is
   selected.
8. **Export is a dropdown with three options:**
   - **Exiled Studio file** (.json, default).
   - **Path of Building** (share code or .xml).
   - **Path of Exile 2 build file** for importing into the game. It is **only enabled for PoE2
     builds**; on a PoE1 build it is greyed out with the reason.
9. **Equipment item list** is docked to the right of the gear visual inside one panel. It keeps only
   slot type, name and notable stats. The badges column was removed.
10. **Equipment summary** no longer shows passive points or skill groups (not relevant there). It
    shows **attributes versus item requirements** and flags any item whose requirement isn't met.
11. **Terminology:** "held lines" became **Warning / Warnings** everywhere in the UI.

---

## 6. Implications for the data model and engineering

- **Stage.** A build has an ordered list of stages. Each stage has a name, a level range, an item
  set (all slots, flasks, jewels), a passive tree and skill groups.
  - The stage is global UI state: every workspace view reads the selected stage.
  - It needs create, rename, reorder and delete ("Manage stages…").
  - PoB export maps stages to PoB's item, skill and tree sets.
- **Dashboard layout** is per build (or per user, see open questions): up to 4 dashboards, each an
  ordered list of rows, each row holding 1–2 component IDs.
  - Needs a component registry with ID, name, category and availability.
- **Requirement check.** Needs the player's attributes per stage and each item's attribute
  requirements. Requirements depend on base type and item level; the draft uses fixed sample
  numbers.
- **Skill model.**
  - Items have sockets with colour and link state. Linked runs of sockets form groups; a group holds
    active and support gems, with levels and quality.
  - One group is the main skill.
  - The group breakdown needs per-support "DPS without it" figures and conflict rules (for example,
    Elemental Focus prevents ailments).
- **Warnings** map to the existing line-quarantine model (reason codes such as `LINE_NO_TEMPLATE`).
  They need counts per build, per item and per stage.
- **Export.**
  - Exiled Studio JSON keeps everything: stages, notes, lines with warnings.
  - PoB export keeps lines with warnings as written.
  - A PoE2 in-game build file format is needed (format not yet specified in the repo).
- **Accessibility and translation.** Every interactive control in the drafts is a real button, link
  or input with a label; keep that in implementation. All copy is literal and ready to translate.

---

## 7. Open questions for the owner

1. **Skills:** Cards or Table as the final layout, or keep the switch?
2. **Dashboards:**
   - Should components support a "full width" lock?
   - Can the same component appear on several dashboards, or twice on one?
   - Can dashboards be renamed?
   - Which default dashboards should a new build get?
   - Is the layout stored per build or per user?
3. **Save and Export:** keep both (Save in place, Export for formats) or merge into one menu?
4. **Stages:** the default stage set and level ranges for a new or imported build. How should an
   imported PoB build with several item sets map to stages?
5. **Passive tree screen** (not drafted yet). It should switch with the stage picker and ideally
   show which points each stage adds, with a stage-to-stage comparison.
6. **Other not-yet-drafted screens:** Configuration, Notes, Manage stages, About/settings, data
   bundle upgrade prompt, PoE2 build variant.
7. **Other dashboard copies:** should the dashboard's compact equipment component follow the stage
   picker too? (The equipment page already does.)

---

## 8. Suggested next steps

- Draft the passive tree screen with stage comparison (the biggest gap).
- Pick the Skills layout, then draft the skill editing flow (sockets, colours, links).
- Draft "Manage stages…" and the stage comparison component.
- Re-record the walkthrough video once the screens settle (tooling in `tools/`).

# B-2 item evidence: game copies, pasted items, card values, tree flavour (plan)

Maintainer 2026-11-15: 27 in-game items (Ctrl+Alt+C copies plus plain and Alt screenshots, 29 copies with two items at 0% and 9% quality) and four passive tree screenshots, then "include it" (the tree points in the same PR). "We can copy the flavour text" (maintainer 2026-11-15, ADR-000 decision recorded in `item-evidence.md`). Branch `feat/b2-item-evidence` from `dev` after #116 (B-2 card data: tag names from `Tags.Name`, unique flavour from `poe1/unique_flavour.json`). One PR, one commit per step, each with its evidence.

**Status (2026-11-15): E1 to E11 done on `feat/b2-item-evidence`, one commit each (E1 and E2 together; E7's flavour check in E3's commit); E12 is the PR (PR #117, open).** E2b's comparison is `crates/loom-serialization/tests/game_copy_fixtures.rs` against `pob-oracle/`. E9 and E10 wait for the main workstation's data run (`item-evidence.md`, "For the main workstation's next data run").

Revised after the plan-reviewer (verdict "proceed with changes"; findings C1 to C4 and the ADR notes are taken below) and after #116.

## What the evidence settled

Recorded in `docs/ui/b2-data/item-evidence.md`; copies in `docs/ui/b2-data/game-copies/` (no notes, account, or character names; flavour kept). Screenshots carry GGG art and stay out of git; every number a test needs (each Alt view's `value (min-max)` base rolls) is written into `item-evidence.md` so the tests reproduce from the repo.

1. **Defences and weapon damage** are `round((base + flat) x (1 + inc) x (1 + quality))`, as PoB and our engine compute them. Nine items fit, including 63.504 → 64, 597.8 → 598 and 4818.07 → 4818. No divergence. The shield's block (24 x 1.46 = 35.04 → 35) does not tell `floor` from `round`; the engine's `floor` (PoB's) stands.
2. **Copy format:** sections, `(augmented)`, `Requirements:` including socketed gems' (no `(gem)` mark), `(unmet)`, mod headers with the game's affix names, tiers and tags, `v(min-max)` values (possibly outside the range), `— Unscalable Value` with an em dash, `( ... )` explanation lines, `(enchant)` per line, flavour and usage-hint sections, unordered `Corrupted` / influence / `Fractured Item` lines, trailing spaces, `Akoya(...)`.
3. **The tooltip's two views:** plain (summed per stat, stat order, fractured first) and Alt (headers, mod order). The card stays the Alt view.
4. **Socket caps** by item level for two-handers and body armour: 2, 3 (2–24), 4 (25–34), 5 (35–49), 6 (50+); seen at 14, 25, 46. #116 found them in the item types' `Sockets` component.
5. **Tree:** the game shows keystone flavour and each ascendancy's quote; our layout file carries neither.

## Steps

| Step | What | Where | Tests |
|---|---|---|---|
| E1 | Copies as fixtures, findings doc with every base roll | `docs/ui/b2-data/game-copies/`, `item-evidence.md` | - |
| E2 | **Game copy reader** (PoE1): game copy → PoB item text, with an account of every line. Rules follow PoB's `ParseRaw` (`local/oracle/pob`, read only) for what PoB already reads (headers, `v(min-max)`, reminders, `Requirements:`, separators). | `loom_serialization::legacy::game_copy::poe1` | Line accounting on every fixture (each non-blank line kept, rewritten, or dropped by a named rule; counts reported); the PoB reader finds no `Unrecognized` line; header kinds to sections and flags; `Akoya(...)` kept; unidentified, synthesised, superior; non-items (gems, currency, cards) and PoE2 copies refused |
| E2b | **PoB comparison** (local, oracle here): PoB's parse of each raw copy vs ours of the converted text (mod lines, sections, flags, quality, sockets, item level, influences). Each difference fixed or registered (ADR-006). | `tools/oracle` script, summary in `docs/ui/b2-data/` | 29 copies, 0 unexplained |
| E3 | **Add an item:** app command `add-item { text, slot? }` (additive). Game copy or PoB text; adds to the build, with `slot` equips in the active item set (a new set if none). Slot must fit the item class; a two-hander empties the off hand as the game does. Item IDs as import allocates them. One undoable step, `item-added`, through the recovery history. Text capped (64 KiB) and logged by size and SHA-256 only. MCP not in this PR. | `loom-app`, app API v1, schema, shell, UI ("Paste an item" on the Equipment page; a page paste handler that ignores text inputs) | Workflow: paste, equip, undo, recovery round trip, calc reruns; refusals |
| E4 | **Shown values kept (C1):** the converter keeps `Armour`, `Evasion Rating`, `Energy Shield`, `Ward` as known specs, and the engine derives each base percentile from the shown total as PoB's `Item.lua` 2323–2325 does, which also closes today's `base-percentile` reason for PoB items with a shown total. Calc change, so its own commit. | `loom-calc::poe1` (`collect`, `phases::piece`) | A pasted armour piece's calc contribution equals the copy's total (Ghoul Pelt, Tempest Refuge, Loath Crown, Soul Ascension, Emperor's Vigilance); `calc-run` / `calc-parity` on public fixtures: only the documents that had the reason change, each toward PoB |
| E5 | **Card values (C2):** a `loom-calc` entry that runs the existing pipeline on one item alone (level 1, no config, the item in a fitting slot) and reads back the existing nodes: piece defences, weapon damage, rate, crit, shield block. No calculation code moved. `ItemCard.values` (additive), cached per item version. Ward not shown. | `loom-calc`, `loom-app::view`, app API v1, UI | Game truth: maul 0% and 9%, Footman Sword, Iron Staff, Long Staff, Double Axe, Bastard Sword damage and rate; Widowmaker crit; the armour pieces above |
| E6 | **Requirements (C3):** the card's level is the copy's `Level:` when the item was pasted (stored as `LevelReq`, as PoB does), noted as possibly raised by socketed gems; otherwise unchanged (base only, stated). | `game_copy`, `view.rs` | Pasted unique shows its copy's level |
| E7 | **Flavour checks against #116:** each unique copy's flavour section equals the bundle's text for that name (real bundle, local); the converter drops the section by rule (the card reads the data). | `loom-app` real-bundle test | 12 uniques; any mismatch recorded |
| E8 | **Affix headers as game truth:** the copies' headers (name, tier, kind) vs the card's inferred affixes on the real bundle. | real-bundle test (`game_truth_affixes`) | Every headed line agrees, or the difference is listed |
| E9 | **Socket caps from data:** extractor roots the item types' `Sockets` component; bundle rule (optional file); `ItemCard.max_sockets` and a card-only reason when exceeded (never a calc state). Fixture bundle in CI; real data needs the main workstation's run. | extractor, `tools/parity-cli` bundle, `loom-data`, `view.rs` | Fixture tests at item levels 14, 25, 46, 72; corpus listing of flagged items in the data run |
| E10 | **Tree flavour from the game files** (ADR-005's main rule, not the export): keystone flavour from the passive skills table, ascendancy quote, colour and box from the ascendancy table; keyed by node hash and ascendancy ID, missing ones reported; optional bundle file. Hover card and ascendancy circle. Real data needs the main workstation's run. | extractor, bundle, `loom-tree`, `loom-app::tree`, UI | Fixture bundle tests; the real run's counts against the export (57 keystones, 21 ascendancies) |
| E11 | **Attribute totals on the tree screen** (Str, Dex, Int from the calculation) | UI | vitest |
| E12 | Docs, handoff, review, PR | | |

## Compatibility

New fields (`ItemCard.values`, `max_sockets`, the tree flavour fields, the `add-item` command) and bundle files are additive in the project's usual sense: a new app reads old documents and bundles; the DTOs deny unknown fields, so an old app rejects new ones, as with `league`, `kind` and `custom_modifiers`. New bundle files are optional, so no data schema bump.

## Not in this PR

The plain view; gem tooltips; cluster subgraphs (T3); timeless and "Passive Skills in Radius" jewels (reasons stay); the staged "Apply points" mode; a `flavour` field on items (the card reads #116's data); ward on the card.

## Risks

| Risk | Mitigation |
|---|---|
| E4 changes calculations | Own commit; public `calc-run` diff limited to documents that had `base-percentile`, each toward PoB; corpus run by the maintainer |
| Converter disagrees with PoB | E2b comparison, register entries |
| Socket-cap false positives | Corpus listing in the data run |
| E9 and E10 need a data run | Code and fixture tests here; the real bundle from the main workstation before the walkthrough check |

# B-2 data part: modifier headers, tiers, and attribute requirements

Status: reviewed 2026-11-14 (plan-reviewer: proceed with changes; this version takes them). PR 1 done 2026-11-14 (uncommitted on `feat/ux-walkthrough`); the maintainer's data run and PR 2 open. Part of the UX sprint (`ux-improvements-notes.md`); B-2's first part (the equipment panel and the game's tooltip, `docs/ui/ux-walkthrough.md`, "B-2, first part") is done.

## Goal

The tooltip's Alt view shows what the game shows with Alt held (maintainer's screenshots, 2026-11-14):

```
Prefix Modifier "Urchin's" (Tier: 2) — Life, Defences
+34(33-48) to Armour
+28(24-28) to maximum Life
```

and the requirement line names attributes: `Requires Level 65, 85 Str, 62 Dex`. The game is the truth; PoB is the oracle (ADR-001/006). Everything here is **display only**: it never feeds line resolution or the calculation, so calc parity cannot move (ADR-015: a line with no identified mod keeps its text and gets no header).

## What exists

- Raw extraction already holds `Mods.Name` (the affix name), `Mods.Level`, `BaseItemTypes.DropLevel`, and `Tags.DisplayString` (the header's "Life", "Defences"); the bundle converter (`tools/parity-cli/src/bundle.rs`) drops them. Bundle mods carry `implicit_tags` (tag IDs), stats with ranges, `mod_type`, `generation`, spawn weights.
- `ComponentAttributeRequirements` (base → Str/Dex/Int) is not extracted: nothing references it.
- Many PoB items name their mods: `Prefix: {range:0.718}LocalBaseArmourAndEnergyShield4` (3607 prefix and 3591 suffix lines in the fixtures); `Prefix: None` for an open affix. Items imported from the game do not.
- `loom-parser::import::mods` has the row index, fit rules, spawn weights, essence and influence handling: the inference in PR 2 reuses them.
- This workstation has no game install: code and synthetic-table tests are done here; the maintainer re-runs `extract` and `bundle` on the main workstation.

## PR 1: data schema 4, exact headers, requirements

1. **Data schema 4** (converter and `loom-data`): `mods` gain `name` and `level`; `base_items` gain `drop_level` and `requirements {strength, dexterity, intelligence}` (from `ComponentAttributeRequirements`, a **PoE1-only** extraction root until PoE2's file is confirmed); a new `data/tags.json` lists each tag's `id` and `display`. Readers gate on `manifest.data_schema >= 4` (as `IMPLICIT_TAGS_SCHEMA`), not on empty fields; schema 3 bundles keep loading and show no names, levels, tags, or attribute requirements.
2. **Exact headers** (`loom-parser::import::affixes`, wasm-safe): an item's `Prefix:`/`Suffix:` spec lines naming a mod ID. The kind (prefix or suffix) is PoB's key, so no per-game generation numbers are needed. A spec is used only when the ID is in the bundle and the mod's stats account for the lines it is given: each explicit, non-crafted line whose resolved stats are all the mod's, with values in its ranges; lines may be anywhere in the item (a hybrid's lines are not always adjacent). A line two mods share (the game sums them: hybrid plus pure "increased Armour") goes under every mod whose stat it carries only when the sum fits their summed ranges; else it stays under no header. A stale or edited spec (unknown ID, stats that match no line) is dropped. `None` specs are open affixes and skipped.
3. **App API v1, additive**: `ItemCard.affixes: Vec<AffixView {kind, name, tier, tags, lines}>` in the game's order (prefixes, then suffixes), `lines` the line indices; `ItemCard.requirements: Option<{level, strength, dexterity, intelligence}>`, the level as written (`LevelReq`), else the base's drop level; attributes from the base (0 left out).
4. **UI**: the Alt view groups each mod's lines under its header (`Prefix Modifier "Name" — Tags`; no tier in PR 1), lines with no mod after them without a header; the requirement line with attributes.
5. **Evidence**: converter tests on synthetic tables; `loom-data` schema 3 and 4 load tests; `affixes` tests (exact, hybrid not adjacent, summed line, stale ID, `None`, unknown base tags); app workflow and UI tests; wasm32 build of `loom-parser`. The maintainer's run regenerates the bundle, the overlays and calc-run records (`data_schema` is part of their identity, `calc_run.rs`), and adds a `data-parity` table for mod `affix`/`level` and base `req` against PoB's `Data/ModItem.lua` and `Data/Bases` (`uncategorized` 0; differences registered).

## PR 2: inferred mods and tiers

- Lines with no PoB spec: candidates from the parser's mod filter (fit rules, spawn weights, essences, influences), one candidate or no header.
- Tier: the grouping key (mod type, family, or both) is proven against game-truth fixtures from the maintainer's screenshots (Urchin's, Tier 2 on a Pig-Faced Bascinet; Thorny, Athlete's, of the Lost, of the Genius, of Abjuration) before it is used; PoB has no tier oracle.
- Implicit (`Implicit Modifier — Tags`) and crafted (`Master Crafted Suffix Modifier "Name" (Rank: n)`) headers.

## Later

`(gem)` requirement marks, requirement changes from modifiers, the level from mod levels (the game's `floor(0.8 x` highest mod level)`) when not written, unique flavour text (Q-B2-1: where the game stores it for uniques), computed weapon and defence values on the tooltip.

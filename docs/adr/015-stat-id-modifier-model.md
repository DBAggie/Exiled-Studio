# ADR-015: Stat-ID-First Modifier Model

- Status: Accepted
- Date: 2026-11-08

## Context
Path of Building's core is `ModParser.lua` (about 7,000 lines), which pattern-matches English modifier text. That ties accuracy to text patterns, makes localization hard, and needs manual updates every league. The game data identifies modifiers by stat IDs with values and renders text through stat-description tables.

## Decision
- Modifiers in the domain model are **(stat_id, values)** sourced from game data (mods, passives, skills, bases).
- The calculation engine maps stat IDs to typed effects. Each mapping is data-driven where possible and code where necessary, and has tests.
- Text is an **import adapter**: item text from PoB XML, the clipboard, or trade is resolved to stat IDs using the game's stat-description tables (reverse matching).
- Lines that cannot be resolved are **quarantined and shown to the user**, never silently dropped. They are preserved for round-trip export.
- Spike B (see plan) measures resolution coverage. If it falls below the gate, the fallback is a **hybrid**: stat IDs everywhere resolution works, plus curated text patterns for the categorized remainder, decided per area by ADR amendment.

## Consequences
- Parity comparison must bridge two models: oracle results come from text-parsed mods, the successor's from stat IDs.
- New leagues mostly need a data re-extraction rather than parser edits; new stat-to-effect mappings still need code and tests.
- Breakdowns can cite the exact game stat behind every contribution.

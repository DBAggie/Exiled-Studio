# ADR-015: Stat-ID-First Modifier Model

- Status: Accepted (Amendment 1, 2026-11-10: typed resolutions and adapted lines)
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

## Amendment 1 (2026-11-10): Typed resolutions and adapted lines
Proposed in the Spike B report (`docs/spikes/spike-b-resolution.md`, section 7) and accepted by the maintainer on 2026-11-10 (in the Spike C C5 session, after PR #39 opened); approved by merging the PR that adds it. Evidence: the Spike B report, sections 4c, 4d, 4e, and 7; the B5g, B5d, and B5e entries of `dev/completed/spike-b-text-stat-resolution/spike-b-text-stat-resolution-context.md`.

### Context
The original decision says a resolved line becomes `(stat_id, values)` from the stat-description tables. Spike B met its gate without the hybrid fallback, but three kinds of item text needed more than that:
- **Granted skills.** A graft's `Uses level {1} {0}` names the skill the graft grants (B5g); no stat stands for it. The skill and its level range are read from game data (`BrequelGrafts`, `SkillGem`, `BrequelGraftGrantedSkillLevels`).
- **Item properties.** Lines such as `Lasts {0} Seconds`, `Memory Strands: 63`, or a catalyst quality (B5d) are display values, not modifiers. Their labels and descriptions come from game data (`ClientStrings`, `AlternateQualityTypes`); the values are read from the line.
- **Lines resolved through an import tolerance.** Hand-edited text in PoB builds differs from the game's text in letter case (`+132 to Maximum Life`), and PoB writes some two-line descriptions into two item sections (B5e). The resolver reads both (`case-folded`, `split-section`) and flags the line `adapted`: the stat IDs come from a stat description, but the text is not the game's.

The Spike B resolver reports the first two as resolved with a `kind` (`granted-skill`, `item-property`, `catalyst-quality`), counts them apart (`resolved_kinds`), and gives a stat-only count and rate beside the overall rate (`rate.stat_resolved`, `rate.stat_rate`). Without a rule, later phases could assume every resolved line is a stat ID, or lose the fact that a line's text was adapted.

### Decision
- **Typed resolutions.** A resolved modifier line is either (1) stat IDs with values from a stat description, or (2) a typed result read from game data, with a `kind` naming what it is (today `granted-skill`, `item-property`, `catalyst-quality`; a typed result may also carry stat IDs, as `catalyst-quality` carries the catalyst's stat). Both count as resolved. They are reported apart (per `kind`, and a stat-only rate beside the overall rate). The data model and the calculations must not assume that every resolved line is a stat ID; each `kind` is a typed variant with its own fields, whose names and values come from game data tables.
- **Adapted lines.** A line resolved only through an import tolerance (today `case-folded` and `split-section`) carries an `adapted` flag, with the tolerance named, through import and every quarantine or display view, because its text is not the game's. A tolerance is tried only when exact matching finds no template; `case-folded` is refused when the text folds to more than one game key, and `split-section` is taken only when the pair resolves to one description. Each tolerance is named in the code and the report.
- **Unchanged.** Lines that cannot be resolved are quarantined and shown to the user, never dropped, and preserved for round-trip export. Text stays an import adapter.

### Consequences
- The Phase 2 model types a resolved line as an enum (stat IDs, or one typed result per `kind`) plus the `adapted` flag; Phase 2's build schemas carry both. Phase 2 decides how the flag is persisted and whether export keeps an adapted line's original text.
- The calculations handle each `kind` explicitly; how each maps to an effect is decided in the calc phases with tests, and a `kind` without a handler is reported, never ignored.
- Coverage reports show the overall rate and the stat-only rate, so typed results cannot hide a fall in stat resolution.
- The UI can mark adapted lines, so users see where their build's text differs from the game's.

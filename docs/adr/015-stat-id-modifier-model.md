# ADR-015: Stat-ID-First Modifier Model

- Status: Accepted (Amendment 1, 2026-11-10: typed resolutions and adapted lines; Amendment 2, 2026-11-13: custom modifier notes and patterns, proposed; accepted when the maintainer merges its PR; Amendment 3, 2026-11-24: reviewed rewordings for item lines, proposed in PR #168, accepted by the maintainer 2026-11-26 with its two open questions decided)
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
Proposed in the Spike B report (`docs/spikes/spike-b-resolution.md`, section 7) and accepted by the maintainer on 2026-11-10 (in the Spike C C5 session, after PR #39 opened); proposed in that PR and accepted when the maintainer merges it. Evidence: the Spike B report, sections 4c, 4d, 4e, and 7; the B5g, B5d, and B5e entries of `dev/completed/spike-b-text-stat-resolution/spike-b-text-stat-resolution-context.md`.

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

## Amendment 2 (2026-11-13): Custom modifier notes and patterns
Decided by the maintainer on 2026-11-13 in the Phase 3 close-out session (open question Q13, `dev/completed/phase-3-calc-slice/phase-3-calc-slice-context.md`; the maintainer chose option (b) after asking for an online check, and chose to apply the lines PoB reads); proposed in that PR and accepted when the maintainer merges it. Evidence: `docs/phase-3/custom-lines.md`.

### Context
A config set's custom modifier lines (PoB's custom modifier blocks and its legacy `customMods` input, P3-12c and P3-12d) are the player's own text, not the game's. PoB v2.67.2 parses each with its `ModParser` and applies a line only when the whole of it parses (`Classes/ConfigTab.lua` 1115 to 1116, `mods and not extra`); any other line does nothing. Two kinds of line kept builds of the Phase 3 gate `Partial` for good:
- **Labels and comments**: "Perfect Forest Warrior" (a heading over the spectre's two buffs, Onslaught and Culling Strike: poewiki.net/wiki/Forest_Warrior), "Ice Nova Buffs", "=== AG item mods ===". No stat description reads them and PoB ignores them; as unresolved lines they were reasons forever.
- **PoB's own readings**: "20% more Damage", "24% more block chance", "+1.5% to critical strike chance". No stat description reads them as PoB does (several unique `_final` stats render "#% more Damage"; the crit line is local to a weapon in the game), yet PoB applies them.

### Decision
For custom modifier lines of PoE1 config sets only (item lines are unchanged):
- **Custom note.** A line with no template, no curated pattern, and either a comment marker first (`//`, `--`, `=`, `~`) or no digit and no word of a modifier's sentence (`MODIFIER_WORDS`: "cannot", "you", "have", "is", "gain", "while", "enemies", ...), that names no keystone, resolves to the typed kind `custom-note`. It has no effect, is kept for round-trip export, and is shown: the calc result lists it under `notes` (calc result v1, additive), and it is never a reason for a state.
- **Custom pattern.** A line that no stat description reads as one stat (no template, or ambiguous) but a curated pattern does resolves to the typed kind `custom-pattern` with the pattern's name and value. Each pattern names the PoB mod it stands for and maps to one effect in the calculation (`loom_calc::poe1::mapping::CUSTOM_PATTERNS`). Patterns are added one at a time, each with a test and PoB's parse as evidence.
- **Checked against PoB.** `loom-parity-cli custom-lines` reads every custom line of the committed fixtures as a build does and compares it with PoB's own parse (`tools/oracle/data/pob_parse_lines.lua`): a note PoB applies, or a pattern whose effect differs from PoB's mods, fails it. It is maintainer-run, as the oracle is.
- **Unchanged.** Stat IDs stay first: a line a stat description reads as one stat is never a note or a pattern. Every other unresolved custom line stays quarantined with its reason.

### Consequences
- The note rule is a heuristic outside the fixtures. A numberless line PoB applies is a sentence ("Cannot be Stunned", "You have Onslaught"), so a line with a modifier's word stays unresolved, a reason; a numberless line PoB parses without such a word (a keystone's name is excluded) would still become a note. The `custom-lines` check finds such a line among the committed fixtures; a new one found later becomes a pattern or an exception.
- The patterns are text, the hybrid fallback this ADR allows per area: here the area is the player's custom lines, whose text no game data describes.
- Build v1 gains the typed kinds `custom-note` and `custom-pattern`; calc result v1 gains `reasons.notes` (both additive).

## Amendment 3 (2026-11-24, accepted 2026-11-26): Reviewed rewordings for item lines
Asked for by the maintainer on 2026-11-24 ("Draft the amendment"), from the open items survey (`dev/active/open-items/open-items-inventory.md`, S0; `dev/active/open-items/maintainer-follow-ups.md`, "Older item wordings"). Proposed in PR #168; **accepted by the maintainer on 2026-11-26**, with the two questions under "Not decided here" answered (see "Decided 2026-11-26" below). Implementation is agent work from then on.

### Context
Item text in PoB builds is often older than the game data the bundle is built from (build 26530219). PoB reads it with its own parser; the resolver reads it only through the current stat descriptions, so a line in an older wording has no template. In the open items survey this is the largest reason that needs a decision: `unresolved-line` `no-template` blocks 1,259 public fields in 17 builds (67 fields by it alone) and 757 corpus fields in 10 builds. Its lines are of four kinds (75 distinct public lines and 60 corpus lines, each run through PoB's `modLib.parseMod` with `tools/oracle/data/pob_parse_lines.lua`):

- **An older wording of a stat the game still has.** "+1 to Maximum Endurance Charge" (now "Charges"), "21% reduced Charges used" (now "Charges per use"), "+13% to Critical Strike Multiplier for Spells" (now "for Spell Damage"), "12% increased Attack Speed during Flask effect" (now "during Effect"). PoB parses each old and current wording into the same mods.
- **A mechanic that changed under similar words.** "7% reduced Mana Reserved" is now "Mana Reservation Efficiency": PoB parses them into different mods (`ManaReserved` INC against `ManaReservationEfficiency` INC), and the game changed the mechanic.
- **A mechanic the game removed**, which PoB still reads for legacy items: "10% chance to Dodge Spell Hits while moving", the pre-3.15 flask immunities ("Immunity to Freeze and Chill during Flask effect"). No current stat stands for them.
- **Not a modifier:** the author's notes ("Vendor Recipe. Higher Rarity Sash = Higher % Phys"), item properties ("Intangibility: 14%", "Recovers 374 Mana over 6.50 Seconds"), names ("Essence of Envy"). PoB parses none of them.

Of the public lines PoB applies 49 and reads none of 26; of the corpus lines, 9 and 51. The tree packs already read older node text through a reviewed table (`tools/tree/rewordings.json`, `docs/tree/tree-packs.md`, "Rewordings"), learned from two versions of GGG's own data. Items have no older GGG data to learn from, so they need another check.

### Decision
For PoE1 item lines (custom modifier lines keep Amendment 2):

- **A reviewed rewording table** (`tools/rewordings/items.json`, the tree table's format: `accepted` and `rejected` pairs of templates, numbers written as `#`, each rejection with its reason). A pair maps an older template to a current template of the bundle's stat descriptions.
- **A pair is accepted only when both hold:**
  1. PoB v2.67.2 parses the old and the current wording, at the same numbers, into identical mods (name, type, value, flags, keyword flags, tags) and with no text left over. This is checked by a tool, not by eye: `loom-parity-cli item-rewordings`, maintainer-run with the oracle as `custom-lines` is, reads `pob_parse_lines.lua`'s output and fails on any accepted pair whose parses differ.
  2. A review finds the change one of wording alone (the tree table's rule). PoB reading two texts the same is evidence, not proof: PoB may itself have kept an old mechanic under the new words.
- **Applied only as a tolerance.** A rewording is tried only when exact matching finds no template, after the existing tolerances; the line then resolves to the current template's stat IDs and carries `adapted` with the new tolerance `reworded` (Amendment 1), through import, the quarantine and display views, and the calculation's sources. An unlisted or rejected line stays `no-template`, a reason, as today.
- **Contract changes, additive:** `Adapted` gains `reworded` in the domain, in build v1 (`dto::v1::types::Adapted`), and wherever calc result v1 names a line's `adapted` flag. A reader of v1 that does not know `reworded` sees an unknown string, as for any additive kind before it. Export keeps the line's original text.
- **Unchanged.** Stat IDs stay first: a line that reads exactly is never reworded. Removed mechanics and lines that are not modifiers stay quarantined with their reason (but see "Not decided here").

### Consequences
- The largest maintainer-blocked reason becomes agent work: building the table from the survey's lines, one pair at a time with PoB's parse as evidence, and re-running the public and corpus calculations (each value that moves is compared with PoB, as for any mapping).
- The check makes a PoB/game mismatch visible rather than hidden: a pair PoB reads differently is rejected by the tool before review.
- The UI marks reworded lines as it marks other adapted lines, so the player sees that the text is not the game's.
- New leagues: a reworded line's current template must exist in the bundle; a later re-extraction that drops it turns the line back into `no-template`, never into a guess.

### Not decided here (for the maintainer)
1. **Removed mechanics** (Dodge, the old flask immunities). Options: (a) stay reasons (default: no stat-ID reading exists, and a current item cannot have them); (b) a curated `legacy-pattern` typed kind, as Amendment 2's `custom-pattern`, each pattern naming PoB's mod with a test, for legacy items in Standard; (c) the same, but only for mechanics the engine models anyway.
2. **Non-modifier lines in items** (notes, names). Options: (a) stay reasons (default); (b) extend Amendment 2's `custom-note` rule to item lines that PoB parses into nothing and that have no digit and no modifier's word, shown and kept for export, never a reason. Item properties ("Intangibility", a flask's recovery line) are a resolver gap under Amendment 1's `item-property` kind, agent work either way.

### Decided 2026-11-26
The maintainer accepted the amendment and answered both questions:
1. **Removed mechanics: option (a).** They stay reasons. No stat-ID reading exists for them, and a current item cannot have them.
2. **Non-modifier lines in items: option (b).** Amendment 2's `custom-note` rule extends to item lines that PoB parses into nothing and that have no digit and no modifier's word. They are shown and kept for export, and are never a reason. Item properties stay a resolver gap under `item-property`.

Implemented on PR #187 (2026-11-26): `tools/rewordings/items.json`, `loom_parser::import::poe1::reword`, `loom-parity-cli item-rewordings`; evidence in `docs/phase-6/item-rewordings.md`. The item note rule is checked against PoB's parse of every no-template item line of the public fixtures and the corpus (no note is a line PoB applies); the modifier words gained "apply", "applies", and "instead" for it.

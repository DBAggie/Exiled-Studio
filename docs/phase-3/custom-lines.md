# Custom modifier lines: notes and patterns (Phase 3 close-out)

ADR-015 Amendment 2 (maintainer, 2026-11-13; open question Q13, option (b)). A config set's custom modifier lines are the player's own text. Since P3-12c and P3-12d each is read alone through the game's stat descriptions; this step adds, for PoE1 data only, what happens to a line that no description reads as one stat.

## What PoB v2.67.2 does

`Classes/ConfigTab.lua` 1106 to 1145 (`BuildModList`): each line of an enabled block (and, with no block text, of the legacy `customMods` input) is stripped of colour codes and trimmed, then parsed with `modLib.parseMod`. The line is applied only when `mods and not extra` (1115 to 1116): the whole line parses. A line PoB cannot read in full does nothing.

## The rule (`loom_parser::import::poe1::custom`)

Tried in order, after the stat descriptions give no single reading:

1. **Curated pattern** (`PATTERNS`), when the descriptions give no template or several readings. Each pattern names the PoB mod it stands for:

| Pattern | Text | PoB mod | Engine effect (`poe1::mapping::CUSTOM_PATTERNS`) |
|---|---|---|---|
| `base-critical-strike-chance` | "+#% to critical strike chance" | `CritChance` BASE | `CritChance` base (the skill's base crit) |
| `more-block-chance` | "#% more/less block chance" | `BlockChance` MORE | `BlockChance` more |
| `more-damage` | "#% more/less damage" | `Damage` MORE | `Damage` more |
| `more-spell-block-chance` | "#% more/less spell block chance" | `SpellBlockChance` MORE | spell block more |

   "+1.5% to critical strike chance" is ambiguous in the descriptions (`additional_base_critical_strike_chance`, `local_critical_strike_chance`); "20% more Damage" renders three unique `_final` stats (Quill Rain, Berserker, Doedre's aura), none of which PoB reads.
2. **Custom note**, when the descriptions give no template: a line starting with a comment marker (`//`, `--`, `=`, `~`), or one with no digit and no word of a modifier's sentence (`MODIFIER_WORDS`: "always", "can", "cannot", "enemies", "gain", "have", "is", "while", "you", ...; added after review, so a flag line PoB applies, such as "Cannot be Stunned", stays a reason), that is not a keystone's name (PoB reads a keystone's name as the keystone). It has no effect; the calc result lists it under `reasons.notes` (`custom-note`), never as a reason for a state.
3. Anything else keeps its unresolved reason.

The typed kinds are `custom-note` and `custom-pattern` (build v1, additive; schema and TypeScript regenerated).

## The check against PoB

| Step | Command (maintainer-run) |
|---|---|
| Every custom line of the committed fixtures, as written | `python -I tools/oracle/data/custom_lines.py docs/phase-3/custom-lines/lines.txt "tools/oracle/fixtures/*.xml" "tools/oracle/fixtures/*/*.xml" "local/oracle/pob/spec/TestBuilds/3.13/*.xml"` (242 lines) |
| PoB's parse of each (`StripEscapes`, trimmed, `modLib.parseMod`) | `tools/oracle/data/pob_parse_lines.lua`, run as `pob_data.lua` (`tools/oracle/data/README.md`); output `docs/phase-3/custom-lines/pob-parse.json` (231 distinct lines; two runs byte-identical, the oracle checkout clean) |
| Our reading against PoB's | `loom-parity-cli custom-lines <bundle> docs/phase-3/custom-lines/pob-parse.json --out <dir>` |

Result on P3-8n's bundle: **46 notes, all lines PoB ignores; 7 patterns, each equal to PoB's one mod; 153 read by stat descriptions; 25 unresolved** (reasons; each in a build outside the gate). Exit 0: no note is a line PoB applies, and no pattern differs. The patterns found: "+1.5% to critical strike chance", "12% Less Damage", "20% more damage", "24% more block chance", "24% more spell block chance", "25% more damage", "40% more damage".

## Effect on the gate builds

- The four Smite Inquisitor builds: "Perfect Forest Warrior" is a note; with the other gems modelled every slice field is complete.
- The three Ice Nova Elementalist builds: "Ice Nova Buffs" and "Mana Infused Staff" are notes; "+1.5% to critical strike chance", "20% more damage", "24% more spell block chance", and "24% more block chance" are patterns (block and spell block now equal PoB's).

## Tests

`loom_parser::import::poe1::custom` (patterns, their less forms, near misses, the note rule); `crates/loom-parser/tests/resolve_build.rs` (a note, a pattern, and a line with a digit that stays unresolved, in blocks and in `customMods`); `loom-parity-cli`'s `custom_lines` (PoB's applied rule, a pattern's one mod, and the three pattern lists being the same names).


## A pattern with text before the number: Punishment's curse effect (the agent list, 2026-11-26)

**The line.** `rf-chieftain-league-start.xml`'s `customMods` holds "Punishment has 50% reduced Curse Effect", the older wording of the game's "#% reduced Punishment Curse Effect" (`punishment_curse_effect_+%`). No stat description reads it, so it was a `no-template` reason. It was the only reason on 77 of the build's fields, the only sole blocker in the public run.

**PoB** parses it whole: the "<skill> has" prefix gives the `SkillName` tag, and "curse effect" is `CurseEffect` (`ModParser.lua` 466). So it applies `CurseEffect` INC −50 to Punishment.

**The rule** (`loom_parser::import::poe1::custom`):
- A pattern may now have text before the number (`Pattern::prefix`; empty for the four earlier patterns).
- New pattern `punishment-curse-effect`: "punishment has #% increased curse effect" and its "reduced" form (negated), unsigned.

**The calculation:**
- `mapping::CUSTOM_PATTERNS` maps the pattern to `Poe1Quantity::NamedSkillEffect(6)` INC.
- That is the same named effect the game's stat maps to (`skill_modifiers::NAMED_SKILL_EFFECTS`, Punishment's curse, as Vulnerability's since P6-11gn). The item line in its current wording is read too.

**Fixture** `ice-nova-punishment-curse-effect-witch.xml` (original; Ice Nova, Punishment, the enemy on low life, the line): every field equals PoB's. PoB's DPS is 1,340.76 with the line, against 1,642.18 without it (a copy without the line, also equal to PoB's in every field, not kept). In the RF Chieftain build Punishment changes no compared field: its damage taken applies only to an enemy on low life, which the build's config does not set.

**Evidence** (`local/p6/pc` on `b-pb`, against `local/p6/rv`):
- **Public:** 0 unexpected; `Complete` 51,025 to 51,189 (the RF Chieftain build's 77 fields, the fixture's 87); no value moved.
- **Corpus:** 0 unexpected. Two RF builds carry the same line: one's average damage, ignite, and hit DPS now equal PoB's; both builds' `FullDPS` moved away (1.99 million to 1.65 million against PoB's 3.25 million, and 431 thousand to 360 thousand against 570 thousand), already far below PoB's from the skills and reasons they lack, indeterminate.
- **Tests:** the pattern's cases in `custom::tests`, `skill_modifiers`' named effect, the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32. The maintainer-run `custom-lines` check compares every fixture's custom lines with PoB's parse; this pattern is PoB's mod for the line (the fixture above is the evidence until that run).

# Item rewordings and item notes (ADR-015 Amendment 3)

ADR-015 Amendment 3 was accepted by the maintainer on 2026-11-26, with its two open questions answered: removed mechanics stay reasons, and item lines that are not modifiers become notes. This page records how it is built and the evidence for it. It is one of the 1.0 items (`docs/plan/ACTION-PLAN.md`, "Version 1.0").

## What changed

| Part | Where |
|---|---|
| The reviewed table: 11 accepted pairs, 5 rejected, each with its reason | `tools/rewordings/items.json` |
| The table compiled into the resolver; a line's template (each number or range `(a-b)` as `#`), the lookup, the numbers put back in order | `loom_parser::import::poe1::reword` |
| Step 6 of an item's resolution (PoE1): a line still without a template whose template the table accepts is read again in its current wording, alone, through the same mod filter as any line | `loom_parser::import::build`, `Resolver::reworded` |
| Step 7 (PoE1): a line still without a template that is a label (no digit, none of the modifier words, not a keystone's name) is a `custom-note`: no effect, kept for export, never a reason | `Resolver::item_notes`, `poe1::custom::is_label` |
| The tolerance `reworded`: domain `Adapted::Reworded`, build v1 and calc result v1 `adapted` (additive), the app's line status `Adapted` | `loom-domain`, `loom-serialization`, the schemas, `ui/src/generated/` |
| An item note in the calculation: a note, as a custom line's is, not an `unhandled-kind` reason | `loom_calc::ingest` |
| The item editor marks a note "Note, no effect", as the Configuration page does | `ui/src/screens/Items.tsx` |
| The check against PoB | `loom-parity-cli item-rewordings` (`tools/parity-cli/src/item_rewordings.rs`) |

PoE2 is unchanged: both steps run for PoE1 bundles only (`poe2_should_keep_an_item_label_a_reason`).

The modifier words gained "apply", "applies", and "instead". PoB applies the item line "Modifiers to number of Projectiles instead apply to the number of targets Projectiles Split towards", which has no digit and none of the earlier words, so the first check failed on it (`note-pob-applies`). The words make the rule stricter for custom lines too. `custom-lines` on its committed parse still gives 46 notes, 7 patterns, 153 stats, and 25 unresolved, the same as `docs/phase-3/custom-lines.md`; the five custom lines with "apply" were read as stats before and still are.

## The table

A pair is accepted only when both hold (the amendment's rule):

1. PoB v2.67.2 parses the old and the current wording, at the same numbers, into identical mods with no text left over. `item-rewordings check` tests this; an accepted pair that fails it fails the command.
2. A review finds the change one of wording alone.

The check also fails when an accepted pair's current wording reads through no description of the bundle.

| Old | Current | PoB's mod (both) |
|---|---|---|
| +# to Maximum Endurance / Frenzy / Power Charge | … Charges | `EnduranceChargesMax` / `FrenzyChargesMax` / `PowerChargesMax` BASE |
| #% increased / reduced Charges used | … Charges per use | `FlaskChargesUsed` INC |
| +#% to Critical Strike Multiplier for Spells | … for Spell Damage | `CritMultiplier` BASE, Spell |
| Chaos Damage does not bypass Energy Shield | Chaos Damage taken does not bypass Energy Shield | `ChaosNotBypassEnergyShield` FLAG |
| #% increased / reduced Attack Speed during Flask effect | … during Effect | `Speed` INC, Attack, while `UsingFlask` |
| #% increased / reduced Stun Recovery | … Stun and Block Recovery | `StunRecovery` INC |

Two descriptions print the current Chaos line (`accepted-ambiguous` in the report); an item's mods choose, as for any line, and a line they cannot choose for stays a reason.

Rejected:

| Old | Current | Why |
|---|---|---|
| #% reduced Mana Reserved | #% increased Mana Reservation Efficiency of Skills | A mechanic change (3.16); PoB: `ManaReserved` against `ManaReservationEfficiency` |
| #% increased Mana Cost Efficiency of Attacks | #% increased Cost Efficiency of Attacks | A mechanic change (every cost, not mana); PoB: `ManaCostEfficiency` against `CostEfficiency` |
| Projectiles cannot collide with Enemies at Close Range | … in Close Range | PoB parses neither wording, so there is no evidence |
| #% of Damage from Hits is taken from your Spectres' Life before you | … your Raised Spectres' Life … | PoB parses only the old wording |
| Wave of Conviction's Exposure applies -#% Elemental Resistance | … applies an extra -#% to Elemental Resistance | PoB reads both as one skill stat, but the game's current line is another stat: not shown to be the same mechanic |

The candidates came from the S0 survey's lines (`dev/active/open-items/open-items-inventory.md`) and from every no-template item line of the public fixtures and the private corpus on `dev` at `43d0847`, each looked up in the bundle's stat descriptions (build 26530219). Lines whose text differs from the game's only in where the game breaks a description over two lines ("… for each type of Elemental Ailment on Enemy", "Modifiers to number of Projectiles instead apply …"), or in letter case across a two-line description (Sacred Wisps), are resolver gaps, not wordings: open item S9.

## Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB `v2.67.2`, `b32759a`, checkout clean) and the evidence bundle `local/p6/packs/b-ct` (build 26530219, data schema 5).

**PoB's parse.** `item-rewordings lines` wrote each pair at the sample numbers and every no-template item line of the public fixtures (`lines.txt`, 54 lines); `tools/oracle/data/pob_parse_lines.lua` parsed them (`pob-parse.json`; a second run byte-identical); `item-rewordings check` compared (`report.json`). All three are in `docs/phase-6/item-rewordings/`.

| | Public | Corpus (21 builds, kept local) |
|---|---|---|
| `accepted` | 10 | 10 |
| `accepted-ambiguous` | 1 | 1 |
| `rejected-pob-differs` / `-agrees` | 4 / 1 | 4 / 1 |
| `reworded` lines | 1 | 2 |
| `note` lines | 4 | 20 |
| `note-pob-applies` | 0 | 0 |
| `unread` lines | 21 | 38 |
| Exit | 0 | 0 |

**The calculation.** `calc-run` with `--scope docs/phase-3/calc-scope-poe1.json` on the 536 public builds, `dev` (`43d0847`, built in a worktree) against this branch:

- Field states unchanged: 39,309 `Complete`, 1,379 partial, 1,420 unsupported, 679 indeterminate on both sides.
- `calc-parity` against the oracle run of `local/p6/oracle` (the same run as RV-21's record): 417 builds without a difference, 118 expected, **0 unexpected**, on both sides.
- Values: 31 fields of 2 builds moved, **all toward PoB's value, none away**; 10 now equal it (Boneshatter Juggernaut's armour, the four base damage reductions, and the physical damage over time multiplier, from "+1 to Maximum Endurance Charge"; Dual Wield Cospris CoC's crit multiplier 3.77 to 4.45 toward PoB's 6.3, from "for Spells"). Each field is still partial for its other reasons.
- Reasons: 9 builds lose 38 `unresolved-line` `no-template` reasons. 23 of those lines are notes now; 13 are reworded and counted; 2 are the reworded Chaos line (the item's mods chose its reading), now `unmapped-stat` `chaos_damage_does_not_bypass_energy_shield`, a stat the mapping does not have yet.

The private corpus (21 builds, `--anonymous`): field states unchanged; 5 builds changed, 18 fields moved, 20 `no-template` reasons gone, 18 notes added. The corpus has no oracle run on this workstation, so its moves are not compared with PoB.

## How to run it again

```sh
cargo build --release --locked -p loom-parity-cli
target/release/loom-parity-cli item-rewordings lines tools/rewordings/items.json --out lines.txt [<item-lines.txt>...]
# PoB's parse, as tools/oracle/data/README.md runs pob_data.lua:
#   luajit tools/oracle/data/pob_parse_lines.lua lines.txt pob-parse.json
target/release/loom-parity-cli item-rewordings check <bundle-dir> tools/rewordings/items.json pob-parse.json --out <dir>
```

The further lines are item lines as a build file holds them (their `{...}` prefixes are removed). `local/p6/tools/line_texts.py unresolved-line <calc-run-dir> <xml-dir>` lists a run's no-template lines.

## Left open

- The lines PoB applies that no accepted pair reads (the report's `unread` with PoB's mods): two-line descriptions written on one line, Sacred Wisps' case across two lines, the Impossible Escape and Intuitive Leap lines, Hungry Loop's lines, the old Trigger craft without its cost line. Resolver work (S9), not rewordings.
- `chaos_damage_does_not_bypass_energy_shield`: unmapped in the calculation (a stat mop-up item).
- A table pair for a new league's wording change: added by the same check.

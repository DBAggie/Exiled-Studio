# Phase 3: core defences (P3-7)

Plan step 7 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`): armour, evasion, energy shield with armour pieces, resistances, block, spell block, spell suppression. With P3-6's 8 fields, 17 of the 24 slice fields are computed; the main skill's 7 stay `not-computed`. The mechanic fixtures match PoB v2.67.2 on all 17 fields and on every breakdown PoB writes. Decisions: context file, "Decisions in P3-7".

## What is computed

The formulas follow PoB's (`crates/loom-calc/src/poe1/phases.rs`, each with its source):

| Field | Formula (PoB) | Source |
|---|---|---|
| Armour, evasion, energy shield | the sum of the rows, `max(round(sum), 0)`. A row is an armour piece's value or the global flat value, each when above 0, times `(1 + inc / 100) x more`; rows in PoB's order (Helmet, Gloves, Boots, Body Armour, Weapon 2, Global) | `Modules/CalcDefence.lua`, primary defences |
| An armour piece's value | `round((min + local flat + range x percentile) x (1 + local inc / 100) x (1 + quality / 100))`; the percentile is the item's `<Defence>BasePercentile`, else 1 | `Classes/Item.lua`, armour branch |
| Attribute bonuses | `floor(Int / 10)`% increased energy shield, `floor(Dex / 5)`% increased evasion, on every row | `Modules/CalcPerform.lua` |
| Resistances | `max = trunc(min(base max, 90))`, `total = trunc(base x max((1 + inc / 100) x more, 0))`, value `max(min(total, max), -200)`; the base includes the penalty | `Modules/CalcDefence.lua`, `calcs.resistances` |
| Resistance penalty | the config's `resistancePenalty` (a modelled input), else -60, on fire, cold, lightning and chaos | `Modules/CalcSetup.lua` |
| Block | `min(round((shield + base) x (1 + inc / 100) x more), min(max, 90))`; a shield's `floor((base block + local flat) x (1 + local inc / 100))` | `CalcDefence.lua`, `Item.lua` |
| Spell block | `min(round(base x (1 + inc / 100) x more), min(max, 90))` with attack block's increases and more | `CalcDefence.lua` |
| Spell suppression | `min(base, 100)` | `CalcDefence.lua` |

The base maximums (75% resistances, 75% block and spell block) and base evasion (15) are stats of the character template, read through the stat mapping. The resistance breakdown's terms are `step.min`, `step.max`, `step.total`; the defence rows are `step.slot.<slot>` (static names, `ArmourSlot::row`) and `step.slot.global`.

## What is collected, and what is a reason

Armour pieces (helmet, gloves, boots, body armour, a shield in the off hand, the swap set's when in use) act on their own quantities (`Poe1Quantity::Piece`): the base's ranges and block from the armour and shield type tables (`CalcData::item_base`), quality and base percentiles from spec lines, and local lines through `PieceMapping`. Their other lines are global. The host names each item's base (`CalcOptions::item_bases`, the legacy adapter's rules, as `resolve_build` takes them).

New reasons, each on every computed field:

| Reason | When |
|---|---|
| `unmodelled-item`, `no-base-name` | an armour piece the host named no base for |
| `unmodelled-item`, `not-armour` | a helmet, gloves, boots, or body armour whose base has no armour properties (only the off hand takes a quiver, whose lines are global) |
| `missing-data` (indeterminate), `shield_types` | a shield without a block row |
| `unmodelled-item`, `ambiguous-base` | bases of the name with different properties (Two-Toned Boots, three bases in game data) |
| `unmodelled-item`, `base-properties` | a weapon in the off hand (dual wielding: P3-10), or any weapon slot |
| `unmodelled-item`, `base-percentile` | a shown defence (`Armour: 120`) without its percentile line; PoB derives one, we do not yet (PoB writes both lines) |
| `missing-data` (indeterminate), `base_items` | a base name the data lacks |

A config input other than a numeric `resistancePenalty` is still `unmodelled-config`.

## Mechanic fixtures

Four new original builds in `tools/oracle/fixtures/mechanic/`; every mechanic fixture (23) now targets the 17 computed fields.

| Fixture | What it isolates |
|---|---|
| `armour-pieces-marauder` | four armour pieces: a percentile line (0.5), a shown value with percentile 0, no line (the top), quality 20 and 10, local flat and increased armour; global armour, evasion and resistances from jewellery |
| `shield-templar` | a spirit shield: local block, flat and increased energy shield, quality, a percentile; attack and spell block with their increase and both maximums raised |
| `resistances-duelist` | the act 5 penalty from the config (-30), fire over a raised maximum, pairs, all elemental, all maximums, chaos |
| `evasion-es-shadow` | a hybrid helmet with two percentiles, quality, local flat, hybrid flat and hybrid increased lines; energy shield and evasion bases; global evasion, energy shield and Global Defences; the attribute bonuses |

PoB's values (equal to ours on every field):

| Fixture | Armour | Evasion | EnergyShield | FireResist | ColdResist | LightningResist | ChaosResist | BlockChance | SpellBlockChance | SpellSuppressionChance | Life |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `armour-pieces-marauder` | 3371 | 52 | 0 | -10 | -35 | -38 | -40 | 0 | 0 | 0 | 1285 |
| `shield-templar` | 0 | 16 | 252 | -60 | -60 | -60 | -60 | 37 | 11 | 0 | 1040 |
| `resistances-duelist` | 0 | 16 | 0 | 76 | 28 | 16 | -17 | 0 | 0 | 0 | 925 |
| `evasion-es-shadow` | 0 | 1213 | 980 | -60 | -60 | -60 | -60 | 0 | 0 | 0 | 1096 |

The 19 P3-6 fixtures now check the defences too: a bare character has evasion 15 or 16 (15 base, `floor(Dex / 5)`% increased), -60% to every resistance, and no armour, block, or suppression.

Found while making them: "+15% chance to Suppress Spell Damage" resolves `ambiguous` (two game stats share the text: `base_spell_suppression_chance_%` and `base_spell_suppression_chance_150%_of_value`). The line was taken out of `shield-templar`; suppression is checked at 0 against PoB and above the cap in unit tests. Which stat a real item carries is a resolver question for P3-8 (the mod row knows).

## Evidence (2026-11-12, Windows)

Inputs: bundle `local/p3/p3-1c/evidence2/bundle-poe1-a` (Steam build 26530219, format 2); overlay `local/p3/p3-2/run4/overlay.json`; scope `docs/phase-3/calc-scope-poe1.json`. Outputs in `local/p3/p3-7/evidence/`, local; summaries in `docs/phase-3/defence-fields/`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, fixture manifest (the manifest committed in `5ba3bfd`; adapter protocol 4 as in P3-5 and P3-6, so the oracle tool's own commit does not change PoB's output) | 60 | Exit 0: every expectation met, 3/3 each; the 4 new fixtures `ok` |
| `calc-run`, public (fixtures, community, PoB's 3.13 builds), twice (clean `50a2b02`) | 60 | 59 documents, `malformed.xml` `IMPORT_XML` at `read` (exit 1, as expected); 425 fields complete, 68 partial, 510 indeterminate, 413 not computed; the two runs' documents and `report.json` byte-identical |
| `calc-run`, corpus, `--anonymous`, twice | 21 | 21 documents, all `indeterminate`: every corpus build uses an older tree (3_15 to 3_28); byte-identical |
| `calc-run --data-overlay`, public and corpus | 60, 21 | The P3-2 overlay has no row these fields read (no `bases` rows: P3-1c found them equal), so the same values |
| `calc-parity`, public, with the overlay run and the manifest | 60 | **No difference.** 425 fields match (the 23 mechanic fixtures and the 2 synthetic level 1 Scions, 17 each); 178 breakdown terms match, 47 absent where PoB writes none; every mechanic target met (391 of 391). Exit 5 (`incomplete`), expected: 34 real builds have no complete field to compare. Of them, 4 on the data's tree are `partial` (68 fields: gems, config, unmapped stats, excluded lines) and the rest `indeterminate` (older trees, unsettled values); `malformed.xml` is the expected failure (both sides fail to import) |
| `calc-parity`, corpus, against P3-5's protocol-4 oracle run (adapter unchanged) | 21 | No difference; exit 5 (`incomplete`): every corpus build is `indeterminate` for its tree version, so the corpus gives no parity evidence for these fields; the mechanic fixtures do |

The calc result v1 document gained a node kind (`trunc`) and a source (`item-base`) under its additive rule (`dto/calc/v1/mod.rs`): until the Phase 3 gate every reader is in this repository; a reader outside it must treat unknown node kinds and sources as open.

Tests: `crates/loom-calc/tests/poe1.rs` (31: the P3-6 set, plus bare defences, the penalty input and a non-numeric one, resistance caps and truncation, an armour piece at the top roll and at a percentile, the last quality line, attribute bonuses on every row, shield and spell block, a shield in the swap set in use, PoB's row order over every slot, each armour-slot reason); `tests/item_base.rs` (3: `Bundle::item_base` over the synthetic bundle: the table join, weapon, other, equal and ambiguous rows, no property tables); `poe1/mapping.rs` (a test per new row group and the piece mapping); `poe1/phases.rs` (`floor(n x 0.2)` equals integer division up to 100,000); `graph.rs` (`Trunc` towards zero); calc result v1 goldens with `trunc` and `item-base`; `calc_run` (17 computed fields on the synthetic build). CI's `round-trip` job checks the synthetic build's counts (7 not computed, 17 indeterminate: the synthetic bundle has no Scion).

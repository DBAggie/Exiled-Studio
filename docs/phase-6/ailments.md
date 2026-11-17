# P6-2b: ignite and the shared ailment base

Phase 6 plan (`dev/active/phase-6-mechanics/`), step P6-2b: the main skill's ignite, as PoB v2.67.2 computes it (`Modules/CalcOffence.lua` 4036 to 4287, 4866 to 5206). The base it builds on (`loom_calc::poe1::ailment`) is shared with bleed and poison (P6-2c).

## The field

`ignite-dps` (PoB's `IgniteDPS`; slice `applies: present`). The field exists only when the main skill's hit ignites: fire damage, and a chance on hit or a crit chance. A dual-wielding attack that would ignite has the field `NotComputed`: PoB combines the hands' ailments (`CHANCE_AILMENT`), which is not ported yet.

## What is computed

1. **Chance:** on a hit, `AilmentChance` (PoB's `EnemyIgniteChance`) at most 100; on a crit, 100. With Elemental Overload (`AilmentsAreNeverFromCrit`), the crit chance equals the hit chance and the critical DoT multiplier is dropped.
2. **The ignite's config:**
   - included: `Dot`, `Ailment`, `Ignite`, `FireDot`, the skill's keywords and its keyword `Attack` or `Spell`;
   - excluded: the hit's mod flags (`Spell`, `Attack`, `Hit`, `Area`, `Projectile`, `Melee`).
   - New flags `KEYWORD_ATTACK` and `KEYWORD_SPELL` stand for PoB's keyword flags. "with Attack Skills" and "with Spell Skills" rows now use them, as PoB does, so they reach an attack's or a spell's ailments.
3. **Rate and duration:**
   - `rateMod = mod(faster) / mod(slower)`;
   - `duration = 4 x mod(duration) x mod(enemy's duration) / rateMod`;
   - the enemy's duration comes from Elemental Weakness: PoB's `SelfElementalAilmentDuration`, `mult = -1`.
4. **Stacks and roll:**
   - `stacks = hitChance / 100 x chance x duration x rate`;
   - the roll is 50, or `stacks / (stacks + 1) x 100` above one stack.
5. **Source:**
   - the hit's fire damage recomputed in the ignite's config, from the hit's bases and conversion, times the share fire keeps;
   - then `(min + (max - min) x roll / 100) x (1 + DoT multiplier / 100)`;
   - from crits, the multiplier adds the character template's `critical_ailment_dot_multiplier_+` (50).
6. **Base:**
   - the two sources are weighted by the chances from non-crits and crits, at the crit chance over the stacks;
   - then times 0.9 (`IgnitePercentBase`).
7. **DPS:**
   - `base x mod(effect) x rateMod x min(stacks, 1) x effMult`, at most `DotDpsCap`;
   - `effMult` is the enemy's fire resistance (no penetration) and its damage taken.

All in PoB's order of operations. Not modelled, each a reason through its unmapped stat:
- ignites that stack, ignite to chaos, other types that can ignite;
- Ruthless Blow, Fist of War and warcry ailment effects;
- the enemy's ailment avoidance;
- burning ground from ignite.

## Stats mapped

| Stat | PoB (`SkillStatMap.lua`) | Effect |
|---|---|---|
| `base_chance_to_ignite_%` | 1030 | `AilmentChance(Ignite)` BASE |
| `always_ignite` | 1033 | the same, 100 |
| `chance_to_freeze_shock_ignite_%` | (ModParser) | the ignite part. It was no effect since Phase 3; freeze and shock chances reach no computed output. |
| `ignite_duration_+%` | 1129 | `AilmentDuration` INC, ignite |
| `base_all_ailment_duration_+%` | 1151 | `AilmentDuration` INC |
| `base_elemental_status_ailment_duration_+%` | 1148 | `AilmentDuration` INC, elemental ailments |
| `faster_burn_%` | 758 | `AilmentFaster` INC, ignite |
| `damaging_ailments_deal_damage_+%_faster` | 1187 | `AilmentFaster` INC, damaging ailments |
| `base_ailment_damage_+%` | 1017 | `Damage` INC, `Ailment` |
| `active_skill_ailment_damage_+%_final` | 1020 | `Damage` MORE, `Ailment` |
| `active_skill_ignite_damage_+%_final` | 1184 | `Damage` MORE, `Ignite` |

Two keystone and gem changes:
- **Elemental Overload's 40% more** now also applies to ailments; PoB's keyword flags are `Hit` or `Ailment`.
- **Elemental Weakness's "increased duration of elemental ailments"** now acts on ignites. It was a `NoSlice` buff stat before.

## Evidence

**Fixtures:**

| Fixture | Result |
|---|---|
| `ice-nova-ignite-witch.xml` | Ice Nova with added fire, 30% ignite chance, burning damage and fire DoT multiplier. Equal to PoB on every field. |
| `ice-nova-ignite-duration-witch.xml` | 80% ignite chance, longer ignites and ailments, faster burning, more crit; stacks above one. Equal to PoB on every field. |
| `smite-ignite-templar.xml` | An attack's ignite. Equal to PoB but for the weapon's rate (Q11, DIV-0075 to DIV-0078); the ignite reads the rate through its stacks. |

**Community fixtures:** the Smite builds (acts 2, 5, 10) and the conversion fixture now have `ignite-dps`. Each differs from PoB only through the Q11 rate (DIV-0079 to DIV-0082, under 0.05%). Act 10's matches PoB's 4.4 s duration once Elemental Weakness's duration is applied.

**Totals:**

| Check | Result |
|---|---|
| Public `calc-parity` | 145 builds, 2,401 match (+67), 82 expected differences (+8), 0 unexpected |
| `data-parity` (export format 10; the evidence bundle with packs) | `gem_effects` equal. The only uncategorized rows are `cluster_jewels`, which this bundle does not carry: the maintainer's extraction rerun. |

Corpus builds that ignite have no `ignite-dps` yet when their main skill is not a computed hit (totems, triggers, minions: later steps), and keep it `indeterminate` otherwise.

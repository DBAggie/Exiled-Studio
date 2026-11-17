# P6-2b and P6-2c: damaging ailments

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

**Community fixtures:** the Smite builds (acts 2, 5, 10) and the conversion fixture now have `ignite-dps`. Each differs from PoB only through the Q11 rate (DIV-0079 to DIV-0082). Below one ignite, the difference equals the build's Speed difference (act 2 +0.054%, the conversion fixture -0.339%). Above one, it is about a tenth of it, through the roll (acts 5 and 10). Act 10's matches PoB's 4.4 s duration once Elemental Weakness's duration is applied.

**Totals:**

| Check | Result |
|---|---|
| Public `calc-parity` | 145 builds, 2,401 match (+67), 82 expected differences (+8), 0 unexpected |
| `data-parity` (export format 10; the evidence bundle with packs) | `gem_effects` equal. The only uncategorized rows are `cluster_jewels`, which this bundle does not carry: the maintainer's extraction rerun. |

Corpus builds that ignite have no `ignite-dps` yet when their main skill is not a computed hit (totems, triggers, minions: later steps), and keep it `indeterminate` otherwise.

## P6-2c: bleed and poison

The same base (`ailment::damaging`), one spec per ailment. Bleed and poison differ from ignite as follows; each is PoB's (`CalcOffence.lua` 4288 to 4865):

| | Ignite | Bleed | Poison |
|---|---|---|---|
| Source types | fire | physical | chaos, then physical |
| Who inflicts it | hits | attacks only | hits |
| Chance on a crit | 100 | the chance on a hit | the chance on a hit |
| Duration base | 4 s | 5 s | 2 s |
| Source and roll | `(min + (max - min) x roll / 100) x mult` | `min x mult + (max x mult - min x mult) x roll / 100` | `(min + max) / 2 x mult` (no roll) |
| Crit chance the ailment sees | over its stacks | over its stacks | the hit's |
| Base | `x 0.9` | `x 70 / 100` | `x 0.30` |
| DPS | `x min(stacks, 1)` | `x min(stacks, 1)` | one poison's (capped), `x stacks` (capped): PoB's `TotalPoisonDPS` |

**Fields:**
- `bleed-dps` (PoB's `BleedDPS`);
- `poison-dps` (PoB's `TotalPoisonDPS`, every poison together).

Both are slice `applies: present`, and a dual-wielding attack that would inflict one has it `NotComputed`.

**New context pieces:**
- `CannotInflict` flags (`cannot_cause_bleeding`, PoB's `CannotBleed`);
- `MELEE_HIT` (PoB's `ModFlag.MeleeHit`, which an ailment's config gets from a melee hit).

**Rows** (`SkillStatMap.lua`):

| Stat | Line |
|---|---|
| `bleed_on_hit_with_attacks_%` | 995 |
| `global_bleed_on_hit` | 998 |
| `base_chance_to_poison_on_hit_%` | 1023 |
| `global_poison_on_hit` | 1026 |
| `bleeding_damage_+%` | 1154 |
| `base_poison_damage_+%` | 1163 |
| `base_bleed_duration_+%` | 709 |
| `base_poison_duration_+%` | 1123 |
| `faster_bleed_%` | 1011 |
| `faster_poison_%` | 761 |
| `poison_dot_multiplier_+` | 1169 |
| `cannot_cause_bleeding` | 1223 |

Two former no-effect stats now map:
- Melee Physical Damage support's more damage with bleed and poison from melee hits (`sup_str.lua` 3660);
- "Staff Attacks deal #% increased Damage with Ailments" (PoB's own parse: `Damage` INC with `ModFlag.Ailment` and the staff flag; a staff state row).

Not modelled, each a reason through its unmapped stat:
- Crimson Dance and other bleed stack limits;
- additional poisons and poison stack limits;
- chaos poison chance;
- the elemental types that can poison;
- aggravated bleeding;
- bleeding against moving enemies.

### Evidence

**Fixtures:**

| Fixture | Result |
|---|---|
| `smite-bleed-templar.xml` | Bleeding DPS equal to PoB; Speed and hit DPS Q11 (DIV-0083, DIV-0084) |
| `smite-poison-templar.xml` | The poisons' DPS differs only through the rate, -0.050%, as Speed (DIV-0085 to DIV-0087) |
| `ice-nova-ignite-overload-witch.xml` | Elemental Overload with ignite; equal to PoB on every field (P6-2b review) |

**Public `calc-parity`:** 148 builds, 2,466 match (+65), 87 expected differences, 0 unexpected.

### The P6-2b review (code-architecture-reviewer), applied here

- **Dual wielding:** either hand's ailment makes the field `NotComputed`.
- **Dependencies:** an attack's ailments depend on `HitChance` too.
- **`staff_ailment_damage_+%`:** no longer no effect.
- **The Q11 entries' evidence:** each is explained by its own numbers.
- **The Elemental Overload fixture:** added.

Not changed:
- **`always_ignite`:** its row's 0.01 is a divisor (`value x scale / divisor`, `ingest::EffectTemplate`), so the chance is 1 / 0.01 = 100, as PoB's.
- **The added-damage rows that PoB flags with keywords:** they feed only the hit's bases, which the ailments reuse.
- **Temporal Chains** (an enemy's debuffs expiring slower): it is not a modelled gem, so it is a reason.

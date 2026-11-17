# P6-2a: dual wielding

Phase 6 plan (`dev/active/phase-6-mechanics/`), step P6-2a. An attack with a weapon in each hand, as PoB v2.67.2 computes it. It comes before the ailments, which PoB computes per hand (`CalcOffence.lua` 5533).

## What is computed

- **The off hand's weapon.** A weapon in `Weapon 2` was a reason before (`base-properties`). Its base and local lines now act on the off hand's own quantities:
  - `Poe1Quantity::OffHandWeapon`, shown as `off-hand-weapon.<value>`;
  - `mapping::OffHandWeaponMapping`;
  - `collect::weapon` with a `Hand`.

  The main hand's quantity names are unchanged.
- **Which hands attack** (`hit::attack_skill`; `CalcActiveSkill.lua` 318 to 335):
  - the main hand must be a weapon the skill can use;
  - an off-hand weapon the skill can use attacks too, and one it cannot use does not;
  - an off-hand item whose base is not known leaves the attack not computed.
- **One pass per hand** (`hit::hand_hit`, PoB's `passList`): chance to hit, crit chance and multiplier, rate (each hand's weapon rate, Q11), and average damage.
- **Combined** (`hit::dual_wield`; `combineStat`, 2003 to 2108, 2441 to 2445, 3869 to 3875):

  | Output | Rule |
  |---|---|
  | Hit chance, crit chance, crit multiplier | `AVERAGE` |
  | Speed | Harmonic mean, `2 / (1 / main + 1 / off)`, 0 when either hand's is 0 |
  | Average damage, hit DPS | `DPS`: the sum, halved unless the skill hits with both weapons at once |

  A skill hits with both at once through its skill data `doubleHitsWhenDualWielding` (Swordstorm), or the stat `skill_double_hits_when_dual_wielding` of the skill or a support (`SkillStatMap.lua` 291). Each hand's DPS is its average damage times its rate.
- **Dual wielding's inherent bonuses** (`collect::dual_wielding`; `CalcSetup.lua` 533, 535), when both hands hold weapons:
  - attacks' `Speed` MORE by the character template's `dual_wield_inherent_attack_speed_+%_final`;
  - `BlockChance` BASE by `inherent_block_while_dual_wielding_%`.

  The doubled and removed variants come from stats not mapped yet (reasons).
- **The item card** shows an off-hand weapon's values (`item_values`).

A build with one weapon computes exactly as before: its documents are byte-identical.

## Not modelled yet

Each of these is a reason or out of the slice:
- per-hand weapon-type conditions (`Using<Type>` from the off hand, "with Axes" on one hand only);
- `MainHandAttack` and `OffHandAttack` conditions;
- a level's attack crit chance for the off hand (`skill-crit-chance`, as before);
- the `DualWielding` condition for "while dual wielding" stats (mapped in the mop-up).

## Evidence

**Fixture:** `smite-dual-wield-templar.xml`, Smite with a Rusted Hatchet and a Driftwood Club. It matches PoB on 21 fields, including:
- block (inherent dual-wield block);
- hit chance, crit chance and crit multiplier, averaged;
- average damage, halved.

Speed and hit DPS differ only through each hand's rate. That is the game's floored milliseconds (Q11), registered as DIV-0072 and DIV-0073 (`game-truth`).

PoB's `TotalDPS` breakdown prints the combined average times the combined speed, while its `TotalDPS` is the per-hand DPS combined. The engine computes the value as PoB does, so the breakdown's rate term has no node in our graph. This is registered as DIV-0074 (`harness`).

**Before and after** (`local/p6/p6-1` against `local/p6/p6-2a`, the evidence bundle with tree packs):

| Set | Result |
|---|---|
| Public `calc-parity` | 142 builds, 2,334 match (the fixture's 21 more), 74 expected differences (3 new), 0 unexpected. Changed documents: the fixture and the two public dual wielders (`Dual Savior`, `Dual Wield Cospris CoC`). |
| Corpus | No document changes; no corpus build attacks with two weapons in a computed skill. |

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
  - an off-hand weapon the skill can use attacks too;
  - an off-hand weapon the skill cannot use disables the skill, as in PoB ("Off Hand weapon is not usable with this skill"), so the attack is not computed;
  - an off-hand item whose base is not known leaves the attack not computed.
- **One pass per hand** (`hit::hand_hit`, PoB's `passList`): chance to hit, crit chance and multiplier, rate (each hand's weapon rate, Q11), and average damage.
- **A weapon's local accuracy is its own hand's.** PoB gives `+# to Accuracy Rating` on a weapon the `MainHandAttack` or `OffHandAttack` condition (`Classes/Item.lua` 2282 to 2288). Here it is the hand's weapon value `accuracy`, which only that hand's pass adds.
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
- per-hand weapon-type conditions (`Using<Type>` from the off hand, other than `UsingWand`); since P6-11b each hand's pass carries its weapon's flags, so "with Axes" acts on that hand only;
- `MainHandAttack` and `OffHandAttack` conditions;
- a level's attack crit chance for the off hand (`skill-crit-chance`, as before);
- the `DualWielding` condition for "while dual wielding" stats (settled since P6-11a, `docs/phase-6/stat-mop-up.md`);
- the skill types that require dual wielding or two different weapon types (`DualWieldOnly`, `DualWieldRequiresDifferentTypes`), supports' weapon types, `countsAsAll1H` and `countsAsDualWielding` (no vetted attack has them);
- an empty main hand with an off-hand weapon, which PoB counts as dual wielding for block and speed (the engine leaves `DualWielding` unsettled there, a reason; P6-11a review);
- `UseOffhandAttackSpeed` and the off hand's accuracy taken from the main hand, which stay `unmapped-stat` reasons.

## Evidence

**Fixture:** `smite-dual-wield-templar.xml`, Smite with a Rusted Hatchet and a Driftwood Club. It matches PoB on 21 fields, including:
- block (inherent dual-wield block);
- hit chance, crit chance and crit multiplier, averaged;
- average damage, halved.

Speed and hit DPS differ only through each hand's rate. That is the game's floored milliseconds (Q11), registered as DIV-0072, DIV-0073, and DIV-0107 (`game-truth`).

**Fixed in P6-11b.** Each hand's `TotalDPS` is its average damage times `globalOutput.Speed`, the combined speed (`CalcOffence.lua` 3712), so PoB's `TotalDPS` is the combined average times the combined speed. P6-2a multiplied each hand's own speed, which agrees only when the hands' speeds do. The fixture's hands are close, so the error was small (90.960 against 90.927 now). P6-2a had registered PoB's breakdown, which prints that product, as a display quirk (DIV-0074). DIV-0074 is retired: the breakdown is PoB's real formula, and the engine's graph now has its rate term. `smite-dual-wield-lines-templar.xml`, with a hatchet and a sword, showed the error at 0.39%.

**Before and after** (`local/p6/p6-1` against `local/p6/p6-2a`, the evidence bundle with tree packs):

| Set | Result |
|---|---|
| Public `calc-parity` | 142 builds, 2,334 match (the fixture's 21 more), 74 expected differences (3 new), 0 unexpected. Changed documents: the fixture and the two public dual wielders (`Dual Savior`, `Dual Wield Cospris CoC`). |
| Corpus | No document changes; no corpus build attacks with two weapons in a computed skill. |

## Review (code-architecture-reviewer)
Applied:
- a weapon's local accuracy is per hand;
- an off-hand weapon the skill cannot use disables it;
- an asymmetric test (`dual_wielding_should_combine_each_hands_own_pass`) checks each hand's own accuracy, the averaged hit chance and average damage, and the unusable off hand;
- DIV-0074's kind and test;
- stale comments.

Recorded above under "Not modelled yet": the skill types and weapon-type rules that remain.

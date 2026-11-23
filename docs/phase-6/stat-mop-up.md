# P6-11: the stat mop-up

Plan decision 9: P6-11 maps the remaining unmapped stats of the public builds and the corpus in batches, each row with PoB's reading and a test. A stat becomes `NoEffect` only when PoB reads it into no output. Batches are picked by how many builds a stat touches (from the latest evidence run).

The first batch of skill stats (P6-6a: `poe1::skill_no_output` and kin) is in `docs/phase-6/skill-data.md`.

## P6-11a: shields and dual wielding

**State:** `DualWielding` (`poe1::states::DUAL_WIELDING`), settled from both hands holding a weapon. A shield is not a weapon, as in PoB.

**Rows** (`mapping::STATE_ROWS`), each as PoB reads the game's text:

| Stat | Builds | PoB's reading |
|---|---|---|
| `shield_block_%` | 22 | `BlockChance` BASE, `UsingShield` |
| `resist_all_elements_+%_while_holding_shield` | 12 | `ElementalResist` BASE, `UsingShield` |
| `dual_wield_or_shield_block_%` | 12 | `BlockChance` BASE, `DualWielding` or `UsingShield` (two rows, never both) |
| `spell_block_chance_%_while_holding_staff_or_shield` | 8 | `SpellBlockChance` BASE, `UsingStaff` or `UsingShield` (two rows; a staff is two-handed) |
| `physical_attack_damage_+%_while_holding_a_shield` | 6 | `PhysicalDamage` INC, `ModFlag.Attack`, `UsingShield` |
| `attack_ailment_damage_+%_while_holding_shield` | 6 | `Damage` INC, `ModFlag.Ailment` with the attack keyword, `UsingShield` |

**Left as reasons:** two per-stat lines, which need a per-stat model:
- `spell_block_%_per_5_attack_block` ("per 5% Chance to Block Attack Damage");
- `chance_to_block_attack_damage_per_5%_chance_to_block_on_equipped_shield_+%`.

**Fixture:** `smite-shield-templar.xml`, Smite with a Rusted Hatchet and a Splintered Tower Shield, with a line for each row. Block, spell block, elemental resistances, and damage are equal to PoB. Speed and hit DPS differ only through the weapon's rate (Q11, DIV-0100, DIV-0101).

**Public and corpus:**
- distinct unmapped stats: 1,096 to 1,090;
- public `calc-parity`: 154 builds, 101 expected differences, 0 unexpected;
- corpus: no unexpected difference;
- both runs byte-identical.

## P6-11b: weapon types, and the P6-11a review

**The P6-11a review:**
- `DualWielding` has one definition, `collect::Hands::dual_wielding`, which the state and the inherent dual-wield bonuses both read. Those bonuses are conditional on the state, so an unsettled state is a reason where it is queried.
- An empty main hand with a weapon in the off hand is `DualWielding` in PoB, through its unarmed weapon data. The engine has no unarmed attack, so that case is left unsettled (a reason).
- Tests cover every case of the state.
- Three sibling shield lines are mapped (`STATE_ROWS`, `UsingShield`):
  - `attack_damage_+%_while_holding_a_shield`: `Damage` INC, `ModFlag.Attack`;
  - `attack_skills_damage_+%_while_holding_shield`: `Damage` INC, `KeywordFlag.Attack`;
  - `attack_speed_+%_while_holding_shield`: `Speed` INC, `ModFlag.Attack`.
- Necromantic Aegis's `UsingShield` is not modelled. That stat is a reason, so the shield rows never apply through it.

**Weapon flags.** PoB's `ModFlag` weapon bits are `Flags::{WEAPON, AXE, BOW, CLAW, DAGGER, MACE, STAFF, SWORD, WAND, UNARMED, FISHING, WEAPON_MELEE, WEAPON_RANGED, WEAPON_1H, WEAPON_2H}`. `KeywordFlag.Bow` is `Flags::KEYWORD_BOW`.
- `hit::weapon_type_flags` gives each game item class its flags, as PoB's `getWeaponFlags` with `data.weaponTypeInfo`. A class PoB has no type for leaves the attack not computed.
- Each hand's pass adds its weapon's flags (PoB's `weapon1Cfg` and `weapon2Cfg`). An attack whose main hand holds a bow also gets `KEYWORD_BOW`.
- An attack's ailments keep `AILMENT_WEAPON_MASK` and `KEYWORD_BOW`. The mask is PoB's `ModFlag.WeaponMask`, `0x2FFF0000`: weapon type, melee, ranged, and two-handed. One-handed (`0x10000000`) and `ModFlag.Weapon` are outside it, so PoB's "one-handed ... with Ailments" lines never reach an ailment.
- The test is `weapon_type_flags_should_follow_pobs_weapon_types`.

**Rows**, each as PoB reads the game's text (`Data/ModCache.lua`, or `ModParser.lua`'s "with swords", "with sword attacks", and "with bow skills"):
- **`HIT_ROWS`:**
  - attack speed with axes, bows, maces, swords, wands, and two-handed melee weapons;
  - accuracy with bows, swords (flat and increased), wands, and two-handed melee weapons;
  - crit chance with bows, staves, swords, wands, and two-handed melee weapons;
  - crit multiplier with bows, swords, wands, one- and two-handed melee weapons, and melee ("+#% to Melee Critical Strike Multiplier", `ModFlag.Melee`);
  - damage with bows and swords;
  - physical damage with axes, swords, and two-handed melee weapons;
  - elemental damage with wands;
  - ailment damage with axes, swords, melee weapons, and two-handed melee weapons;
  - hit and ailment damage with staves, wands, and two-handed melee weapons (`KeywordFlag.Hit` or `Ailment`: the `any` set);
  - damage and damage over time with bow skills (`KeywordFlag.Bow`);
  - added fire damage with sword attacks, and added fire and lightning damage with wand attacks.
- **`STATE_ROWS`:**
  - `additional_staff_block_%` (`UsingStaff`);
  - `attack_speed_while_dual_wielding_+%` (`DualWielding`);
  - `damage_+%_while_wielding_wand`, on the new `UsingWand` state, which reads either hand as PoB does.
- **`NO_EFFECT` (`area`, as `base_skill_area_of_effect_+%`):** `area_of_effect_+%_while_wielding_bow` and `area_of_effect_+%_while_wielding_staff`.
- **Left as reasons:**
  - weapons that count as every one-handed type (Varunastra);
  - `wand_damage_+%_per_power_charge`;
  - "fire an additional projectile" with wands;
  - socket-colour and per-socket lines;
  - unique `_final` lines;
  - two-handed impale lines;
  - the skills' "can only use" lines.

**Found: dual-wield hit DPS** (`docs/phase-6/dual-wield.md`, "Fixed in P6-11b"). Each hand's `TotalDPS` reads the combined speed, so the hit DPS is the combined average times the combined speed. P6-2a used each hand's own speed. DIV-0073 moved, and DIV-0074 is retired.

**Fixtures:**
- **`smite-axe-lines-templar.xml`:** a Rusted Hatchet. Axe, one-handed, and melee lines apply. Bow, sword, two-handed, and wand lines do not. Equal to PoB except Q11 (DIV-0102 to DIV-0104).
- **`smite-dual-wield-lines-templar.xml`:** a Rusted Hatchet and a Rusted Sword. The sword's damage, accuracy, and crit chance apply to the off hand only, and the axe's attack speed to the main hand only. The dual-wield speed and block also apply. Equal to PoB except Q11 (DIV-0105, DIV-0106).
- **`smite-dual-wield-templar.xml`:** its breakdown total, DIV-0107.

**Public and corpus** (`local/p6/p6-11b` against `local/p6/p6-11a`, bundle `local/p6/packs/b-flasks`):

| Measure | Before | After |
|---|---|---|
| Public `calc-parity` | 154 builds, 101 expected differences, 0 unexpected | 156 builds, 106 expected differences, 0 unexpected, no unused entry |
| Corpus | No unexpected difference | No unexpected difference |
| Distinct unmapped stats (public and corpus) | 1,090 | 1,039 |
| Public distinct unmapped stats | 934 | 887 |
| Corpus distinct unmapped stats | 431 | 412 |

- No field changed state, and both runs were byte-identical.
- 41 values of `Partial` public fields changed.
  - 38 moved toward PoB's value.
  - Three moved away:
    - the P6-2a dual-wield fixture's hit DPS, the fix above, registered;
    - Kinetic Fusillade's speed, a ballista totem's (its `totem-data` reason);
    - Perforate's hit chance (95 to 99, PoB 95): the sword accuracy lines now apply. PoB applies them to a sword too, as the dual-wield fixture shows; the build's other differences (dexterity, unmapped stats) keep the field `Indeterminate`.

## P6-11c: the P6-11b review

**Fixed:**
- **An attack's DoT keeps its weapon flags** (`dot::attack_weapon_flags`). PoB's `dotCfg.flags` is `skillCfg.flags` with `Dot` (`CalcOffence.lua` 5649). `skillCfg.flags` holds `weapon1Flags or weapon2Flags` (`CalcActiveSkill.lua` 455), and its keyword flags keep `KeywordFlag.Bow`. Without this, "Damage Over Time with Bow Skills" could never reach a bow attack's own DoT.
- **Each hand's ailments and impale read that hand's flags** when dual wielding (PoB: `band(cfg.flags, ModFlag.WeaponMask)` of the pass's config). P6-11b gave both hands the main hand's flags. These checks decide only whether the field is `NotComputed` (PoB's per-hand ailments are not combined yet).
- **Intimidating Cry's exert uptime reads `globalOutput.Speed`** (`CalcOffence.lua` 2598, after `combineStat("Speed")` at 2444). Each hand's pass is now split in two:
  - `hand_rate`: the weapon and the rate;
  - `hand_hit`: the hit chance, crit, and damage, with the global speed (the hands' combined speed when dual wielding).

  Single-hand values are unchanged; only the graph's node order moved.
- **`MainHandWeapon` is gone.** "Damage with Weapons Penetrates #% Fire Resistance" reads each hand's `Flags::WEAPON` (`ModFlag.Weapon`). Its sibling, "... #% Elemental Resistances" (`reduce_enemy_elemental_resistance_with_weapons_%`), is mapped the same way.
- **Two staff lines read `Flags::STAFF`.** `physical_staff_damage_+%` and `staff_ailment_damage_+%` are PoB's `ModFlag.Staff` lines ("with Staves", "Staff Attacks deal"). They now read the weapon flag instead of the `UsingStaff` state. The ailment line no longer stands in for the flag with the attack keyword: an ailment keeps `ModFlag.Staff` through `WeaponMask`.
- **`KeywordFlag.Arrow`** is `Flags::KEYWORD_ARROW`. It comes from a gem's `skill_can_fire_arrows` (`SkillStatMap.lua` 1341, `CalcActiveSkill.lua` 412). An attack's hit, ailments, and DoT keep it, and the stat itself has no other effect.
- **`wand_damage_+%`** ("#% increased Damage with Wands", `ModFlag.Wand` and `Hit`) is mapped.

**Noted, not changed:**
- If `DualWielding` is unsettled and the character data lacks a dual-wield constant, nothing is recorded. The constant matters only once the state settles, and the hand that left it unsettled already carries a reason.
- The engine's one `HIT` flag is PoB's `ModFlag.Hit`. PoB also strips `KeywordFlag.Hit` from a DoT that is a hit (`dotIsHit`), so a "Hits and Ailments" line would reach such a DoT here and not in PoB. No vetted skill has one (`dot::attack_weapon_flags` notes it).

**Fixture:** `burning-arrow-bow-lines-templar.xml`, Burning Arrow with a Short Bow.
- The bow and bow-skill lines apply: damage, attack speed, crit chance, crit multiplier, accuracy, and "Damage Over Time with Bow Skills" on the ignite.
- The wand, melee, and one-handed lines do not apply.
- Every field is `Complete` and equal to PoB, except the bow's floored attack time (Q11): Speed, hit DPS, and the ignite's DPS are each 0.050% lower (DIV-0108 to DIV-0110).

**Public and corpus** (`local/p6/p6-11c` against `local/p6/p6-11b`, bundle `local/p6/packs/b-flasks`):
- **Public `calc-parity`:** 157 builds, 109 expected differences, 0 unexpected, no unused entry.
- **Corpus:** no unexpected difference.
- **Determinism:** both runs were byte-identical.
- **Values:** the only changes are two `Partial` values of the Kinetic Fusillade build (its wand line, now mapped), both toward PoB.
- **Distinct unmapped stats:**
  - public: 887 to 885;
  - corpus: 412 to 411.

## P6-11d: the P6-11c review

- **`KeywordFlag.Arrow` comes from the active gem only.** PoB reads `skillFlag` entries from the active effect's own stat map (`CalcActiveSkill.lua` 341 to 346); a support adds flags only through `addFlags`. P6-11c read the stat from every gem of the skill. The DoT phase already read the active gem alone.
- **An attack's DoT takes its main hand's weapon flags only.** An empty main hand gives `Unarmed`, PoB's `weaponTypeInfo["None"]`, without `Weapon`. P6-11c fell back to the off hand, which PoB does only when the main hand gives no flags; that is a forced off hand, not modelled. PoB's `dotFromAttack` skills without the `Attack` type are not read.
- **Tests:**
  - `weapon_type_stats_should_map_as_pob_reads_them`: the bow, bow-skill DoT, two-handed, melee-weapon ailment, wand (hit and ailment, damage, added lightning), staff, and weapon penetration rows, each against `Data/ModCache.lua`'s flags.
  - `intimidating_cry_uptime_should_read_the_combined_speed_when_dual_wielding`: the uptime reads the hands' combined speed, so both hands' damage takes one factor.
- **Not tested end to end:** the elemental penetration line (`reduce_enemy_elemental_resistance_with_weapons_%`). It has a table test only.
- **Evidence** (`local/p6/p6-11d` against `local/p6/p6-11c`): no value or state changed in the public builds or the corpus. Public `calc-parity`: 157 builds, 109 expected differences, 0 unexpected, no unused entry. Both runs were byte-identical.

## P6-11e: the P6-11d review

**An attack DoT's main hand follows PoB's `getWeaponFlags`** (`dot::main_hand_dot_flags`, `CalcActiveSkill.lua` 192 to 223):

| Main hand | Flags |
|---|---|
| A weapon the skill can use | The weapon's flags; `KeywordFlag.Bow` for a bow |
| Empty | `Unarmed`, if the skill takes any weapon or lists `Unarmed` |
| A hand the skill cannot use | Reason `weapon-types` (PoB disables the skill unless the off hand serves, which is not modelled) |
| A base the engine does not know, or a class without PoB flags | Reason `weapon-flags` |
| A shield or another item | None (never PoB's `weaponData1`) |

P6-11d gave `Unarmed` and a weapon's flags without the skill's weapon check, and no flags, without a reason, for an unknown base.

**Tests:**
- `main_hand_dot_flags_should_follow_pobs_weapon_flags` covers each case.
- The dual-wield Intimidating Cry test also checks that each hand's own rate differs from the combined rate, and that no hand's uptime is capped. A per-hand uptime would then fail it.

**Evidence** (`local/p6/p6-11e` against `local/p6/p6-11d`):
- No value or state changed in the public builds or the corpus.
- Public `calc-parity`: 157 builds, 109 expected differences, 0 unexpected, no unused entry.
- Both runs were byte-identical.

## The P6-11e review (in P6-7b1's PR)

The #162 review's minor findings, applied with P6-7b1:
- **One weapon check:** `MainSkill::can_use` is now shared by the hit (`hit.rs`) and the attack DoT (`dot.rs`). Its docs name what is not modelled: a support's own `weaponTypes` (PoB disables the support, not the skill) and `countsAsAll1H` (Varunastra). The main hand's own class decides.
- **The impossible case is gone:** the DoT test's `Held::Weapon("Shield")` case is replaced by a weapon class without PoB flags. The hands read a shield as `Held::Shield` (an armour base with block), which gives no flags. Shield skills need a shield through their types, not the main hand's flags.
- **The field state, confirmed:** a `weapon-types` or `weapon-flags` reason makes the DoT field `Partial` through `Reasons::into_state`, or `Indeterminate` with the reason kept when a value is also unsettled.
- **Test order:** the DoT test module's `use super::*;` now comes first.
- **No value or state changed** (`docs/phase-6/minions.md`, "P6-7b1", Evidence).

## P6-11f: block per 5%, evasion per frenzy charge, and lines that reach no field

**Rows** (each as PoB reads the game's text; `local/p6/parse2` renders the line from the bundle's stat descriptions and runs PoB's `modLib.parseMod`):
- `chance_to_block_attack_damage_per_5%_chance_to_block_on_equipped_shield_+%`: `BlockChance` BASE, `PerStat` `ShieldBlockChance` `div = 5`;
- `spell_block_%_per_5_attack_block`: `SpellBlockChance` BASE, `PerStat` `BlockChance` `div = 5`;
- `evasion_rating_+%_per_frenzy_charge`: `Evasion` INC, `Multiplier` `FrenzyCharge` (`charges::PerCharge::Evasion`).

The two per-5% lines are `Poe1Quantity::BlockPer5`. `phases` reads them as PoB's `PerStat`: `floor(stat / 5) x value`, an extra base term.
- Attack block reads the shield's own block.
- Spell block reads the final attack block.
- The block chances are whole numbers, so PoB's `+ 0.0001` never changes the floor.

These two were P6-11a's "left as reasons".

**`NoEffect`** (no slice field reads them):
- stun threshold, projectile speed, block recovery, melee strike range;
- endurance charge duration (the count comes from the config);
- avoid elemental ailments, corrupted blood immunity;
- Unwavering Stance (the evade chance and stun).

**Left as reasons:**
- "+# Armour if you've Blocked Recently": three PoB inputs set `BlockedRecently`, a condition the engine does not settle yet.
- Glancing Blows: PoB applies its keystone node's mods.
- Additional physical damage reduction and the recovery stats: P6-10b.

**Evidence:**
- **Fixture:** `ice-nova-block-per-5-witch.xml`. Block is 36 (the shield's 24 plus 3 x 4), spell block 28 (4 x 7), and evasion 46 with three frenzy charges. All 22 computed fields are `Complete` and equal to PoB.
- **Public `calc-parity`:** 175 builds, 112 expected differences, 0 unexpected.
- **Corpus:** 13 values moved toward PoB, none away.

## P6-11g: armour if Blocked Recently, and the most common lines that reach no field

The batch takes the most common unmapped stats of the public builds and the corpus (`local/p6/unmapped_7c2.txt`), each checked against PoB's parse of its game text (`local/p6/parse2`).

**A condition several inputs set:** PoB sets `BlockedRecently` from any of three checks (`ConfigOptions.lua` 1410 to 1420): blocked recently, blocked an attack recently (which also sets `BlockedAttackRecently`), and blocked a spell recently (`BlockedSpellRecently`). `conditions::CHECKS` now lets one input set several conditions and several inputs set one. `settle` gives a condition `true` when any of its inputs is on, unsettled when an input is unreadable and none is on, else `false`. The three inputs left `config::NAMED_ONLY`.

**Row:** "+# Armour if you've Blocked Recently" (`X_armour_if_you_have_blocked_recently`): PoB's `Armour` BASE with `Condition` `BlockedRecently`, a state row. The block stats conditioned on not having blocked recently stay reasons (`config`'s guard list), as before.

**`NoEffect`** (each PoB parse named; no slice field reads them):
- regeneration: life and mana per second, life regeneration rate, life per uncapped fire resistance;
- leech: life and mana from attack damage, instant leech;
- recovery: life and mana per enemy hit, life per kill, life on kill, energy shield on spell block, mana on hit after a mana flask;
- energy shield recharge rate and delay;
- costs: skill costs, life instead of mana cost;
- additional physical damage reduction (the damage you take);
- Mind Over Matter and Divine Shield (their keystones change damage taken and recovery only);
- item rarity, avoid stun, sockets, movement speed per frenzy charge;
- two lines PoB does not parse: the Endurance Charge on kill with a shield, and the ignite spread radius.

The recovery, regeneration, and leech rows hold only until P6-10b adds those fields. That step must map them.

**Left as reasons:** Glancing Blows (block doubled), chance to ignore the enemy's physical damage reduction, skill effect duration (warcry uptime), additional projectiles, aura effect on self (a guarded word), damage per aura or herald, and the life thresholds (they move `LowLife` and `FullLife`).

**Evidence:**
- **Fixture:** `smite-blocked-recently-no-effect-templar.xml`. Armour is 300, from the line through "Blocked an Attack Recently", as in PoB. Every no-effect line leaves the fields equal to PoB, apart from the Q11 speed: DIV-0125 and DIV-0126.
- **Public `calc-parity`:** 185 builds, 125 expected differences, 0 unexpected; `Complete` fields 3,514. 3 armour values moved, all toward PoB (1 now equal).
- **Corpus:** 0 unexpected. 3 armour values moved, all toward PoB (2 now equal).
- Both runs are byte-identical.

## P6-11h: Glancing Blows, and a lone stat shown without a number

**Glancing Blows** (`keystone_glancing_blows`, the node's only stat): "Chance to Block Attack Damage is doubled" is PoB's `BlockChance` MORE 100 (`poe1::keystones::KEYSTONES`). PoB's spell block reads the attack block's increases and mores (`CalcDefence.lua` 709 to 713), so it doubles spell block too. Its spell-block line (`SpellBlockChance` MORE) scales only projectile spell block, and "You take 65% of Damage from Blocked Hits" (`BlockEffect`) reaches no slice field.

**The resolver's hidden value** (`loom_parser::resolve`, `hidden`): a description's stat shown without a placeholder used to be assumed 0 whenever its condition allowed 0. The game prints a description only when one of its stats is not 0, so a lone stat shown without a number is not 0. It is now assumed 1, the game's value of such a flag or keystone stat, when its condition allows 1. A custom "Glancing Blows" line had read 0, giving 0% more block, with no reason. A tree node's stat carries its value and was right already. Test: `resolve_should_fill_stats_without_a_placeholder`.

**On the way:**
- **`EnemyExplode` left the other-gems table.** It is PoB's own skill for the "enemies you kill explode" lines (an item's skill), not a granted effect of the game's data, so no gem names it. The real-bundle test `modelled_chilling_skills_should_be_chilling_areas` found it.
- **The maintainer-run tests (`--ignored`, `LOOM_BUNDLE_DIR`) all pass again.** The keystone, exposure, and reservation tests accept a stat with no effect by a rule of its own (`mapping::no_effect_by_rule`: a skill's stat with no output, a cluster jewel's or a gem display's prefix). The exposure test lists "#% chance to Freeze, Shock and Ignite" as a row that maps only its ignite part: the engine computes no shock from a hit. These rows came from earlier Phase 6 steps; the tests had not been run since.

**Evidence:**
- **Fixture:** `ice-nova-glancing-blows-witch.xml`, with block 58 and spell block 24. All 22 computed fields are equal to PoB.
- **Public `calc-parity`:** 186 builds, 125 expected differences, 0 unexpected; `Complete` fields 3,536. 2 values moved, both toward PoB.
- **Corpus:** 0 unexpected. 13 block and spell block values moved, all toward PoB; 10 are now equal.
- Both runs are byte-identical.

**The resolver change's reach** (the P6-11h review): every description with one stat and no placeholder now gives 1 where it gave 0. `resolve-corpus` on build 26530219 with Spike B's six PoB sources (`local/p6/rc-new`) gives the same rate: 3,356 of 3,366 lines, 99.70%, `uncategorized` 0. `data-parity` is unchanged in every table. More resolver cases: a lone stat whose condition excludes 1 keeps the old rule.

## P6-11i and P6-11j: flask lines; the chance to ignore physical damage reduction

**P6-11i:** six local flask lines reach no slice field (`flasks::NO_EFFECT_LOCAL`, each checked against PoB's parse):
- four PoB does not parse: immunity to bleeding on use, consecrated ground on use, charges when hit, use on full charges;
- movement speed during the effect;
- the effect of curses on you during the effect.

**P6-11j:** "Hits have #% chance to ignore Enemy Physical Damage Reduction" is PoB's `ChanceToIgnoreEnemyPhysicalDamageReduction` BASE (`Poe1Quantity::IgnorePdrChance`, a hit row). A hit's armour (`hit::ignore_reduction`, `CalcOffence.lua` 3423 to 3435) applies it as follows:
- the chance is capped at 100;
- with a chance between 0 and 100, the config's `ChanceToIgnoreEnemyPhysicalDamageReductionMode` counts it 100 under `MAX` and 0 under `MIN`; `AVERAGE`, the default, keeps the chance;
- at 100 the reduction is 0; otherwise a positive capped reduction is multiplied by `1 - chance / 100`.

Impale keeps its own reduction, as PoB's. Not modelled, so reasons where they appear: the partial ignore and the flag that ignores it outright (unmapped stats).

**Evidence:**
- **Fixtures:** `smite-ignore-pdr-average-templar.xml` and `smite-ignore-pdr-max-templar.xml`. The average damage matches PoB in both; the Q11 speed is DIV-0127 to DIV-0132.
- **Public `calc-parity`:** 188 builds, 131 expected differences, 0 unexpected; `Complete` fields 3,581. 2 values moved, both toward PoB (Cyclone).
- **Corpus:** 0 unexpected, no value moved.

## P6-11k: the equipped shield's defences, and the effect of your auras on you

Two kinds of line, each checked against PoB's parse (`local/p6/parse2`):

- **The equipped shield's defences:**
  - "#% increased Energy Shield from Equipped Shield" (`shield_maximum_energy_shield_+%`) is PoB's `EnergyShield` INC tagged `SlotName` Weapon 2.
  - "#% increased Defences from Equipped Shield" (`shield_armour_+%`) is PoB's `Defences` INC with the same tag.
  - Both map to `Poe1Quantity::SlotIncreased(slot, defence)`. The second gives one row for each of armour, evasion, and energy shield.
  - `phases::primary_defence` gives a piece with such an increase its own modifier: the global increase plus the piece's, times the more. Every other piece keeps the shared modifier, so builds without these lines keep their graphs.
- **"Auras from your Skills have #% increased Effect on you"** (`aura_effect_on_self_from_your_skills_+%`) is PoB's `SkillAuraEffectOnSelf` INC.
  - It has no skill type tag. PoB sums it for every aura buff on you (`CalcPerform.lua` 2413), and never for a buff or on a minion.
  - It is `Poe1Quantity::SkillAuraEffectOnSelf`, a `skill_modifiers::STATS` row, and `Totals::buff_scale` adds it for every aura.

**Left for later:** spell damage per 100 maximum mana (PoB's `PerStat` with a total limit, which every damage path would read after mana) stays an `unmapped-stat` reason. The life mastery line is P6-11l.

**Evidence** (`local/p6/p6-11k/`):
- **Fixture:** `ice-nova-shield-defences-aura-self-witch.xml`: a Twig Spirit Shield with both shield lines, and Discipline with 20% increased effect of your auras on you. EnergyShield 282 and its recharge 128, both equal to PoB's.
- **Public `calc-parity`:** 195 builds, 131 expected differences, 0 unexpected; `Complete` fields 5,020.
  - 9 values moved, all toward PoB: three builds' armour, energy shield, recharge, regeneration, and their hits.
  - No state changed.
- **Corpus:** 0 unexpected. 25 values moved, all toward PoB, 7 of them now equal (armour and evasion of shield builds). No state changed.
- **data-parity:** unchanged (uncategorized 353).
- **Tests:** a unit test of the aura on self in `buff_scale`; the real-bundle reservation and aura test lists the stat's text. Workspace tests, clippy, and the real-bundle tests pass.

## P6-11l: more maximum life with at least 6 Life Masteries

"#% more maximum Life if you have at least 6 Life Masteries allocated" (`life_mastery_count_maximum_life_+%_final`) is PoB's `Life` MORE with `MultiplierThreshold` `AllocatedLifeMastery` 6. PoB counts allocated nodes named "Life Mastery" (`CalcSetup.lua` 689).

- **The data:** `PassiveKind` gains `life_mastery`. From the bundle it is a node named "Life Mastery"; from a tree pack it is a mastery node of that name (`loom-pipeline`).
- **The state:** `states::SIX_LIFE_MASTERIES` counts the active spec's connected nodes.
  - 6 or more settle it true.
  - Fewer settle it false when every node's kind is known; otherwise it stays unsettled.
- **The row:** a state row, `Life` MORE.

**Evidence** (`local/p6/p6-11l/`):
- **Public `calc-parity`:** 195 builds, 0 unexpected, 5,020 `Complete`. 7 values moved, all toward PoB (two partial builds' life, unreserved life, regeneration, and DoT). No state changed.
- **Corpus:** 0 unexpected. 9 values moved, all toward PoB; three builds' Life and LifeUnreserved now equal PoB's. No state changed.
- **Tests:** the state rows' test lists the row. Workspace tests, clippy, and the real-bundle tests pass.
  - `loom-persistence`'s file round trip failed once on a Windows rename (access denied) and passed three reruns; it is unrelated to this step.


## P6-11m

The 21 most frequent globally unmapped stats that PoB reads into no slice field (each line rendered and parsed by PoB, `local/p6/parse2/m11_*`) are now `NoEffect` rows:
- **Minion counts:** zombies, skeletons.
- **Endurance charge gains:** PoB leaves them unparsed, or reads a partial maximum with leftover text. The count comes from the config.
- **Flasks:** a flask's amount recovered and recovery rate, charges on crit or on consuming a corpse, taunt on use.
- **Enemy debuffs and the enemy's damage:** Malediction on enemies ignited during a flask, the enemy's own conversion.
- **Pierce, reflect, a mark's transfer** (unparsed).
- **Recovery on block:** life, and energy shield from armour.
- **Lucky spell suppression:** PoB reads it only into the effective chance (`CalcDefence.lua` 1134).
- **Minion duration.**

**Left unmapped on purpose:**
- Lines that reach a slice field: Onslaught on kill, the attack and brand damage lines whose flags the parse does not show, the herald and aura multipliers, Wither's effect, the conversions, the hit's inverted resistances.
- Leech and the life thresholds (P6-10b's planned outputs).
- Timeless jewels (P6-10a).

**Evidence** (`local/p6/p6-11m/`):
- **Public and corpus:** no value moved and no state changed (those builds keep other reasons).
- **Reasons:** the global unmapped (build, stat) pairs went from 418 to 407.
- 0 unexpected.
- **Tests:** the charge guard lists the five charge rows.

## P6-11n

**The flask mapping honours `NoEffect` rows.** A flask's own line the game's mapping reads as `NoEffect` (P6-11m's amount and rate recovered, charges on crit, taunt, the ignited enemies' lines) was still `Unmapped` through the flask mapping. It is now `NoEffect` there too (`flasks::FlaskMapping`); any other local line stays a reason.

**Rows,** each as PoB parses its line (`local/p6/parse2`, the global unmapped stats of the public builds and the corpus):
- **Hit rows:**
  - physical attack damage (`PhysicalDamage` INC, attacks); attack crit chance and multiplier; the elemental skills' crit multiplier (`CritMultiplier` BASE, the elemental keywords);
  - physical damage converted to lightning or chaos (`Conversion`), the skill's own physical conversion to chaos (`SkillConversion`);
  - the Occultist's more chaos and cold damage, the Berserker's more damage, more elemental damage;
  - the Assassin's Mystical Infusion (30% less spell crit chance, `CritChance` MORE with spells);
  - melee attack speed.
- **A damage over time row:** the mastery's less damage taken over time (`SelfDamageTaken` over time, MORE).
- **`NoEffect`:** "Unaffected by Ignite" (ignite on you), flask life recovered, power charge duration.
- **`EhpReason` markers:** damage taken recouped as mana, and life gained on block (two lines).

**Left unmapped on purpose** (each reaches a field in a way the engine does not model yet):
- skill effect duration (ailments that are skill effects, ground effects);
- projectile counts;
- the life thresholds (they move PoB's automatic low life condition);
- the herald, mark, sigil, and brand lines (skill types);
- resistance conversion;
- the monster block line.

**Evidence** (`local/p6/p6-11n/`):
- **Public `calc-parity`:** 210 builds, 280 expected, 0 unexpected.
- **Values:** 60 changed in partial or indeterminate builds; 52 toward PoB, 3 the same.
  - Five moved away, all in `ethereal-knives-mines-assassin.xml`: the game's 30% less spell crit chance applies (PoB's parse reads it the same way). That build's distance from PoB is its other reasons (the forking crit among them).
- **Corpus:** 0 unexpected.
- **Tests:** the charge, damage taken, and flask guards list the new rows. The workspace's 1,766 tests and clippy pass.

## P6-11o: lines while an aura affects you

**PoB** sets `AffectedBy<Name>` while an aura affects the character (`CalcPerform.lua` 2417 to 2419; a Vaal aura's under its name without "Vaal "). Lines "while affected by <Aura>" read it.

**The engine:**
- **The settled conditions:** `gems::affected_by` lists the auras the gem table applies to the character, over the groups `arcane_surge_support` reads. `conditions::settle` settles `AffectedBy<Name>` for the seven auras the rows read (`conditions::AFFECTED_BY`): Anger, Determination, Grace, Hatred, Precision, Pride, Wrath.
- **Twelve state rows,** each as PoB parses its line (mostly cluster jewel notables):
  - Determination's armour;
  - Precision's attack damage, attack speed, and crit multiplier;
  - Wrath's crit chance and lightning penetration;
  - Anger's crit multiplier;
  - Hatred's added cold damage and cold penetration;
  - Pride's physical damage;
  - Grace's spell suppression.
- **Left unmapped:** Hatred's additional crit chance (a permyriad value; state rows carry no divisor), and Pride's impale chance and stacks.

**Evidence** (`local/p6/p6-11o/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected.
- **Values:** 49 changed in partial or indeterminate builds; 43 moved toward PoB.
- **The six that moved away** are `poisonous-concoction-of-bouncing-pathfinder.xml`'s suppression (87 to 100) and the hit fields and damage chance that follow it.
  - The line is Unholy Grace's, a cluster jewel notable, while Grace affects the character.
  - PoB's oracle run has 88; the build's own saved PoB stat is 100.
  - The build carries the cluster reason (the evidence bundle has no cluster tables).
- **Corpus:** 0 unexpected.
- **Tests:** the state rows test reads the new conditions. The workspace's 1,766 tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11p: Onslaught on kill

**PoB** parses "#% chance to gain Onslaught for 4 seconds on Kill" and "Gain Onslaught for # seconds on Kill" as `Condition:Onslaught` FLAG under `Condition` `KilledRecently`. The config sets `KilledRecently` from its check `conditionKilledRecently`, or from a kill count above 0 (`multiplierKilledRecently`, `ConfigOptions.lua`).

**The engine:**
- **The settled condition:** `conditions::KILLED_RECENTLY` is settled from the check and from the count (`KILLED_RECENTLY_COUNT_INPUT`), as hit recently is. The count's entry in `config::NAMED_ONLY` keeps only `Multiplier:EnemyKilledRecently`.
- **Onslaught:** `poe1::buffs` takes an Onslaught flag that holds: unconditional, or under settled conditions that all hold.
- **Two state rows:** `chance_to_gain_onslaught_on_kill_%` and `onslaught_time_granted_on_kill_ms`, each `Onslaught` FLAG under `KilledRecently`.

**Evidence** (`local/p6/p6-11p/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; Complete 13,701, unchanged.
- **Values:** none moved, public or corpus. Six builds lost the two stats' `unmapped-stat` reasons; their fields stay partial for other reasons. The minion's own line (`minion_chance_to_gain_onslaught_on_kill_for_4_seconds_%`) stays unmapped.
- **Corpus:** 0 unexpected.
- **Tests:** the state rows test reads the two rows. The workspace tests and clippy pass.

## P6-11q: socketed gem levels

**PoB** reads "+N to Level of Socketed <words> Gems" as a `GemProperty` LIST modifier `{ keyword = <the words>, key = "level", value = N }` with the tag `SocketedIn` for the item's slot (`ModParser.lua` 3178 to 3182; no words: `all`). A group tabulates its `GemProperty` modifiers with its slot, " Swap" dropped (`CalcSetup.lua` 1631 to 1638). The item's levels therefore reach every gem of a group in the item's slot whose gem matches the keyword (`ModStore.lua` 719 to 760; `applyGemMods`, `calcLib.gemIsType`).

**The engine** (`loom_calc::poe1::gem_levels`):
- **The stats:** `SOCKETED_STATS` lists 30 socketed level stats, each with PoB's keyword (all, skill, support, aoe, projectile, trap or mine, and so on).
- **The slot:** on an item in an equipment slot (`GemSlot`: the two weapons, helmet, body armour, gloves, boots), the stats map to `Poe1Quantity::SocketedGemLevel(stat, slot)`. The `InSlot` mapping wraps the slot's own mapping (piece, weapon, or the game's).
- **The bonus:** `GemLevels::bonus` takes the group's slot and adds the matching socketed totals to each gem of the group, actives and supports alike, after the global levels.
- **Left unmapped:** the three socketed stats that scale with a count or hold with one gem socketed ("per Socketed Gem", "per # Player Levels", "while there is a single Gem Socketed"). They stay reasons.

**PoB's levels checked:** a local probe (the oracle adapter with a print of each skill's effects, run on PoB v2.67.2; not committed) against the engine's levels:
- `penance-brand-ignite-elementalist.xml`: every gem matches (Penance Brand 27, its supports 18 from "-2 to Level of Socketed Support Gems", Ignite Proliferation 20 as an AoE gem). PoB's item-granted Burning Damage support is not modelled; its reason stands.
- `lightning-trap-of-sparking-elementalist.xml`: every support matches. The active gem is 25 here against PoB's 28; the other 3 levels are a named gem line ("+3 to Level of all <gem> Gems"), not modelled, a reason.
- `poisonous-concoction-of-bouncing-pathfinder.xml`: 20 of 22 gems match. The helmet's "+2 to Level of Socketed AoE Gems" is a `{custom}` line, which the import excludes as a reason.

**Evidence** (`local/p6/p6-11q/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; Complete 13,701, unchanged.
- **Values:** 36 moved, 32 toward PoB, 3 equal now.
- **The four that moved away** come from other gaps. Lightning Trap's and Penance Brand's mana costs: the levels match, the costs differ for the reasons above. The Pathfinder's mana unreserved and fire hit multiplier: the build's partial reservation.
- **Socketed level stats:** unmapped in 1 build, from 22.
- **Corpus:** 20 moved, 16 toward; 0 unexpected.
- **Tests:** the unit tests for the slot rule and the table. The maintainer-run bundle test checks each row's keyword against its English text and the gem tags. The workspace tests and clippy pass.

## P6-11r: supports an item grants

**PoB** reads "Socketed Gems are Supported by Level N <Support>" (and the "Socketed <kind> Gems" and "Socketed Spells" forms) as `ExtraSupport` LIST modifiers `{ skillId, level }` tagged `SocketedIn` for the item's slot (`ModParser.lua` 2071 to 2104, 3371, 3372):
- **The support:** the gem named "<name> Support", its granted effect and its secondary one when that is a support. A secondary effect that is a skill is an `ExtraSkill`.
- **Adding them:** `CalcSetup.lua` 1525 to 1555 adds them to each group in the slot before the group's own supports, at the line's level and quality 0. A group an item grants gets none. No gem level modifier changes them.
- **Joining:** the group's supports join by `addBestSupport` (338 to 367: the same granted effect is kept once, at the higher level, then the higher quality).
- **The exclusion:** an item's support does not support a skill an item grants (`CalcTools.lua` 94).

**The engine:**
- **The table:** `loom_calc::poe1::item_supports::ROWS` has 203 rows, PoB's own parse of each stat's English line. A local script runs `modLib.parseMod` on the bundle's texts.
  - 30 stats stay unmapped as reasons: 9 PoB does not parse, 19 that also grant a skill, and the two Chieftain lines that name a slot.
  - A maintainer-run test checks every row against the bundle's texts and gems.
- **Mapping and order:** on an item in an equipment slot, a row's stat maps to `Poe1Quantity::ItemSupport` for the slot (`gem_levels::InSlot`). `GemLevels` keeps them in order, and `skill`'s group effects put them first.
  - Each one's gem index is past the group's gems, and `GemEffectRef::from_item` marks it.
  - The group's supports join by PoB's `addBestSupport` (`skill::add_best`). PoB's `plusVersionOf` is not in the bundle.
- **Blasphemy is modelled** (`gems::SUPPORTS`), because Heretic's Veil gives it:
  - the hex it supports takes its types (an aura, so the aura curse priority, and a reservation);
  - Blasphemy's `CurseEffect` MORE (-25%) on the curse;
  - its level row's mana reservation share in place of the curse's (`CalcActiveSkill.lua` 516 and 517).
  - Data parity's `gem_effects` table allows its added types and reservation; uncategorized stays 353.

**PoB's supports checked:** the local probe on `penance-brand-ignite-elementalist.xml` lists PoB's main skill's supports with the item's Burning Damage at level 18. PoB's support cost multiplier, 2.8391, is 1.3 × 1.4 × 1.3 × 1.2: the item's support counts as the engine counts it.

**Evidence** (`local/p6/p6-11r/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; Complete 13,701, unchanged.
- **Values:** 10 moved, 3 toward PoB.
- **The seven that moved away:**
  - `penance-brand-ignite-elementalist.xml`'s mana cost (57 to 75; PoB 45). The item's Burning Damage ×1.3 applies, as in PoB; the base cost differed before.
  - `OccVortex.xml`'s evade chances and the EHP fields that follow them (8 to 6; PoB 13). Enfeeble now has Blasphemy's 25% less effect, as in PoB; the gap was there before.
  - The same build's mana unreserved moved toward PoB (927 to -46; PoB 167). The two Blasphemy curses reserve 35% each. Heretic's Veil's "reduced reservation of socketed curse gems" and the build's Enlighten (an old skill ID) are reasons.
- **Item support stats:** unmapped in 0 builds, from 10.
- **Corpus:** 4 moved, 1 toward; the three mana costs rise by the item supports' multipliers. 0 unexpected.
- **Tests:** the table's unit test; the maintainer-run rows test; the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11s: damage per aura or herald affecting you

**PoB** counts `Multiplier:Herald` and `Multiplier:AuraAffectingSelf` over the active skills (`CalcPerform.lua` 2112 to 2126):
- a skill whose types hold `Herald` counts once per skill name;
- otherwise a skill holding `Aura` counts once per name, but not when it also holds `AuraAffectsEnemies` or Generosity supports it (`auraCannotAffectSelf`).

"for each of your Aura or Herald Skills affecting you" reads both (`ModParser.lua` 1398).

**The engine:**
- **The count:** `gems::aura_or_herald_count` counts the same over the main group and the enabled groups in an enabled slot, from each skill's types with its supports' added ones.
- **The row:** `poe1::stacks` has a new kind, `AuraOrHerald`, whose per-count effect is `Damage` INC (`damage_+%_per_your_aura_or_herald_skill_affecting_you`). It is applied after the stacks.

**Evidence** (`local/p6/p6-11s/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; Complete 13,701; no value moved.
- **Corpus:** 7 values moved, all toward PoB; 0 unexpected.
- **The stat:** unmapped in 0 builds, from 6.
- **Tests:** the workspace tests and clippy pass.

## P6-11t: regeneration per charge and per stat, and channelling mana costs

**PoB** parses each line with a tag (`ModParser.lua`; checked by the local parse script on the bundle's texts):
- "Regenerate #% of Life per second per Endurance Charge": `LifeRegenPercent` BASE, `Multiplier` `EnduranceCharge`.
- "#% increased Mana Regeneration Rate per Power Charge": `ManaRegen` INC, `Multiplier` `PowerCharge`.
- "#% increased Mana Regeneration Rate per 1% Chance to Block Spell Damage": `ManaRegen` INC, `PerStat` `SpellBlockChance`.
- "Regenerate # Life per second for each 1% Uncapped Fire Resistance": `LifeRegen` BASE, `PerStat` `FireResistTotal`.
- "Non-Channelling Skills have +# to Total Mana Cost" and the channelling one: `ManaCost` BASE with a `SkillType` `Channel` tag (negated for the first).

A `PerStat` count is `floor(stat / div + 0.0001)` (`ModStore.lua` 458). `FireResistTotal` is the resistance's truncated total before its cap (`CalcDefence.lua` 611), and `SpellBlockChance` is the capped chance (714). Both are computed before regeneration (1237).

**The engine:**
- **Per charge:** the two charge stats are rows of `poe1::charges::MODIFIERS`, with new per-charge targets: life regeneration percent per minute, and increased mana regeneration.
- **Per stat:** two new quantities, `RegenPerFireResist` and `RegenIncPerSpellBlock`, are read by `poe1::regen`:
  - the first adds to the flat sum per point of the fire resistance's total (`phases::resistance_parts`, which also returns the uncapped total);
  - the second adds to the increase per point of the spell block chance field, which becomes a dependency of the regeneration field.
- **Channelling costs:** `ManaCostChannelling(bool)` joins mana's total in `poe1::costs` when the main skill's `Channel` type matches.
- **Scoping:** the regeneration quantities stay field-scoped (`elsewhere`), so a rolled value left unsettled is a reason on the regeneration fields only. The four stats are gone from `REGEN_REASONS`, and the two cost stats from `COST_REASONS`.

**Evidence** (`local/p6/p6-11t/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected. Match went from 13,399 to 13,403.
- **Values:** 26 moved, 25 toward PoB; 5 now equal PoB.
- **The one value that moved away:** `ethereal-knives-mines-assassin.xml`'s mana cost (20 to 14; PoB 22).
  - The item's "Non-Channelling Skills have -6 to Total Mana Cost" applies, as in PoB.
  - Our cost before the total is 20 where PoB's is 28; that gap was there before.
- **States:** 4 life regeneration fields became indeterminate, because they now read a per-charge line whose rolled value is unsettled. 4 cost fields became complete.
- **Corpus:** 17 moved, all toward PoB; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11u: durations, projectile counts, and chains with no slice field

These 14 stats are mapped `NoEffect` in `mapping::NO_EFFECT`, each with a category. They were unmapped in 59 build-occurrences across the public set and the corpus; now they are unmapped in none.

**PoB:**
- **Skill duration** (`Duration`, `CalcOffence.lua` 1462 to 1530) reaches a damage output in these places:
  - a skill function's stages or ticks (Blight's, Penance Brand of Dissipation's, Earthquake of Amplification's; `CalcPerform.lua` 2226 to 2252);
  - a brand's activation count;
  - `TotalDot` for stacking DoT;
  - Trauma;
  - an ailment that is a skill effect (`bleedIsSkillEffect` and kin).

  The engine leaves each of these as a reason or not computed:
  - an unvetted skill (`hit::pob_vetted`: functions, own stats, tagged base mods);
  - brands not computed;
  - `DotCanStack` not computed;
  - Trauma Support not modelled;
  - `poe1::ailment`'s `ailment-skill-data`.
- **Projectile count** (`ProjectileCount`) reaches DPS only with `SequentialProjectiles` (769 to 772). That flag comes either from a skill's base mod, which leaves the skill unvetted, or from Barrage's `projectiles_barrage`, which is unmapped and so a reason.
- **Chain count** (`ChainCountMax`) is read by the per-chain lines (`PerStat` `Chain` and `ChainRemaining`).
- **Charge duration:** the charges' count is the config's (`poe1::charges`).
- **Totem duration:** no slice field reads it.

**The rows:**
- **`skill-duration`:** `skill_effect_duration_+%`, `base_secondary_skill_effect_duration`, `sigil_duration_+%`, `base_trap_duration`, `mine_duration_+%`.
- **`totem-duration`:** `totem_duration_+%`, `base_totem_duration`.
- **`charge-duration`:** `charge_duration_+%`, `base_charge_duration_+%`. Both are also listed in the charges test's allowlist.
- **`projectile-count`:** `number_of_additional_projectiles`, `base_number_of_projectiles`, `attacks_number_of_additional_projectiles`, `wand_attacks_fire_an_additional_projectile`.
- **`chain-count`:** `number_of_chains`.

**Guards** (`NO_EFFECT_DEPENDS_ON`; a maintainer-run test checks that each stays unmapped):
- `projectiles_barrage` and `modifiers_to_number_of_projectiles_instead_apply_to_chaining` guard the projectile count.
- `projectile_damage_+%_per_chain`, `projectile_damage_+%_per_remaining_chain`, and `projectile_non_chaos_damage_to_add_as_chaos_damage_%_per_chain` guard the chains.

**Evidence** (`local/p6/p6-11u/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; no value or state moved.
- **Corpus:** no value or state moved; 0 unexpected.
- **The stats:** unmapped in 0 build-occurrences, from 59.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11v: nearby enemies

**PoB:**
- **The count** (`ConfigOptions.lua` 1223 to 1231): `Multiplier:NearbyEnemies` is the BASE sum of the config's `multiplierNearbyEnemies` and `multiplierNearbyRareOrUniqueEnemies` (a `countAllowZero`).
- **The condition:** `Condition:OnlyOneNearbyEnemy` holds when the first count is exactly 1.
- **The lines:** Lunaris's soul (`Data/Pantheons.lua` 69 to 102) gives three:
  - "#% additional Physical Damage Reduction for each nearby Enemy, up to 8%", and the same for movement speed. These carry the tag `{ Multiplier NearbyEnemies, limit, limitTotal }` (`ModParser.lua` 1913), so each line's total is `min(value x count, 8)`.
  - "#% additional Physical Damage Reduction while there is only one nearby Enemy" carries `limit = 1` and the condition (1774).

**The engine:**
- **The stack kind:** `poe1::stacks` has a new kind, `NearbyEnemies`, with three per-stack targets (physical damage reduction BASE, movement speed INC).
- **The total limit:** 8, the stat descriptions' fixed text.
- **The inputs:** `multiplierNearbyEnemies` becomes a `stacks` input and leaves `config::NAMED_ONLY`. `multiplierNearbyRareOrUniqueEnemies` keeps its other names.
- **The reason tables:** the three stats leave `MOVEMENT_REASONS` and `REDUCTION_REASONS`. Movement speed per nearby enemy stays scoped to the movement fields.

**Evidence** (`local/p6/p6-11v/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected. Match went from 13,403 to 13,533.
- **Values:** 72 moved, all toward PoB; 35 now equal PoB.
- **States:** 130 fields went from partial to complete.
- **Corpus:** 13 moved, all toward PoB; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11w: movement speed on burning ground

**PoB** reads "#% increased Movement Speed while on Burning Ground" as `MovementSpeed` INC with the `OnBurningGround` condition (`ModParser.lua` 1754). Only the config's check `conditionOnBurningGround` sets that condition (`ConfigOptions.lua` 1163). The check's implied `Burning` matters only to the UI.

**The engine:**
- `poe1::conditions` settles `OnBurningGround` from the check.
- The stat is a state row (`mapping::STATE_ROWS`) and leaves `MOVEMENT_REASONS`.

**Not done here, with why:** the other single-condition lines are not settled by a plain config check alone.
- **Full life** (`FullLife`): PoB also derives it from the life left unreserved (`CalcPerform.lua` 76 to 83). Corrected in P6-11ai: only Chaos Inoculation sets it there.
- **Life flask use** (`UsingLifeFlask`): PoB derives it from active flasks.
- **Minion skill used recently** (`UsedMinionSkillRecently`): PoB also derives it from the main skill.
- **Exposure inflicted recently** (`AppliedExposureRecently`): PoB derives it from exposure applied.

**Evidence** (`local/p6/p6-11w/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected. Match went from 13,533 to 13,535: 2 fields went from partial to complete, and no value moved.
- **Corpus:** unchanged; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11x: Flesh and Stone's Sand Stance, and fortification's rounding

**PoB** (`act_str.lua` 4380): in Sand Stance, Flesh and Stone's `flesh_stone_sand_stance_damage_taken_+%_final_from_distance_from_enemy_hits` is `DamageTakenWhenHit` MORE.
- Its tag is a `Buff` with `effectCond = "SandStance"`. The config's `bloodSandStance` set to `SAND` sets that condition (`ConfigOptions.lua` 643).
- As a `Buff`, it scales by the buff effect (`CalcPerform.lua` 2266 to 2276), not the aura effect, though the skill is an aura. It does not reach minions.

**The engine:**
- **`poe1::gems`:** a new `BuffStat::SandStance` applies its rows only in Sand Stance and never to minions. The rows take the buff scale: `add_rows` now gets the skill's own scale and the buff scale.
- **Flesh and Stone's Sand Stance stat:** was a `Defence` marker (an `unmapped-stat` reason on the enemy hit fields); it is now that row.
- **`data-parity` `gem_effects`:** reads the row as PoB's `DamageTakenWhenHit` with `effectCond` `SandStance`.

**Found on the way: fortification's rounding.**
- PoB's `MoreInternal` rounds each mod name's MORE product to two decimals. Fortification's MORE is a `DamageTakenWhenHit` mod (`CalcPerform.lua` 648), so PoB rounds it together with Sand Stance: with 20 stacks, `round(0.8 x 0.86, 2) = 0.69`.
- The engine multiplied fortification in after rounding (0.688). `enemy_hit::taken_modifiers` now multiplies it into `DamageTakenWhenHit`'s product before rounding.
- The first run, without this fix, gave smite-inquisitor-act5 15 unexpected differences; that run was not committed.

**Evidence** (`local/p6/p6-11x/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected.
  - Match went from 13,535 to 13,557.
  - 85 values moved, all toward PoB; 25 now equal it.
  - 22 fields went from partial to complete.
- **Corpus:** 56 values moved, all toward PoB; 7 now equal it. 0 unexpected.
- **Sand Stance stat:** no longer unmapped anywhere; it was in 15 builds.
- **`data-parity`:** `gem_effects` 1,611 values compared, 0 differences; uncategorized 353, unchanged.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11y: Momentum's Swiftness, and the main skill's support buffs

**PoB:** Momentum Support's `support_momentum_movement_speed_+%_per_stack_removed` is a `Buff`: `MovementSpeed` INC with a `Multiplier` tag (`sup_dex.lua` 3132).
- The multiplier is `MomentumStacksRemoved`. Only the config count `MomentumSwiftnessStacks` sets it (`ConfigOptions.lua` 543).

**The engine:**
- **`poe1::gems`:** a new `BuffStat::PerCount` takes the row's scaled value times the config count. It applies nothing without a count; a value that is not a number is an `unmodelled-config` reason.
- **Momentum's stat:** now that row, where it was a `MovementReason` marker. It leaves `mapping::MOVEMENT_REASONS`, and `MomentumSwiftnessStacks` is an input `poe1::gems` reads.
- **`data-parity` `gem_effects`:** reads the row as PoB's mod with its `Multiplier` tag.

**Found on the way: the main skill's supports.**
- The main skill's supports already apply their global effects through `poe1::gems` (`supports_global`). But `mapping::SkillMapping` also read their global stats as the skill's own, so a stat without a hit effect there was unmapped on every field.
- That showed in the three builds where Momentum supports the main skill. Before, the stat's movement marker hid it.
- `SkillMapping` now gives every modelled support's global stat (`gems::is_support_global_stat`) no effect on the skill. This also removes Infused Channelling's damage taken stat and Summon Phantasm's grants from those builds' main skill.

**Evidence** (`local/p6/p6-11y/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected.
  - Match went from 13,557 to 13,563.
  - No value moved: no fixture sets the count.
  - The movement fields of 3 builds went from partial to complete.
- **Corpus:** unchanged; 0 unexpected.
- **Momentum's stat:** no longer unmapped anywhere; it was in 11 builds.
- **`data-parity`:** `gem_effects` 1,612 values compared, 0 differences; uncategorized 353, unchanged.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.
- **Not committed:** the first run, without the `SkillMapping` fix, put an `unmapped-stat` reason on every field of 3 builds.

## P6-11z: Arctic Armour's damage taken while stationary

**PoB** (`act_dex.lua` 588 to 593) reads Arctic Armour's `new_arctic_armour_physical_damage_taken_when_hit_+%_final` and its fire counterpart as `PhysicalDamageTakenWhenHit` and `FireDamageTakenWhenHit` MORE.
- Each is a `Buff` with the `Stationary` condition. The config's `conditionStationary` count settles that condition (`poe1::conditions`, P6-10b).

**The engine:** the two stats, which were `Defence` markers, are `BuffStat::WhenState` rows on `conditions::STATIONARY`. `data-parity` names them `<Type>DamageTakenWhenHit`.

**Evidence** (`local/p6/p6-11z/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected.
  - Match went from 13,563 to 13,607.
  - 18 values moved, all toward PoB; 10 now equal it.
  - 44 fields went from partial to complete.
- **Corpus:** 17 values moved, all toward PoB; 1 now equals it. 0 unexpected.
- **The two stats:** no longer unmapped anywhere; they were in 7 builds.
- **`data-parity`:** `gem_effects` 0 differences; uncategorized 353, unchanged.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11aa: Infused Channelling

**PoB** (`sup_int.lua` 3892 to 3897) reads two Infused Channelling stats.
- **`support_storm_barrier_damage_taken_when_hit_+%_final_while_channelling`:** `DamageTakenWhenHit` MORE. It is a `Buff` whose `modCond` is `InfusionActive`. Only the config check `infusedChannellingInfusion` sets that condition (`ConfigOptions.lua` 495), and `CalcActiveSkill.lua` 806 drops a mod whose `modCond` fails.
- **`support_storm_barrier_damage_+%_final`:** the supported skill's `Damage` MORE.

**The engine:**
- **`poe1::gems`:** a new `BuffStat::CheckModCond` applies its rows only when its config check is on. The damage taken stat, which was a `Defence` marker, is that row.
- **`infusedChannellingInfusion`:** leaves `config::NAMED_ONLY` and is now an input `poe1::gems` reads.
- **Infused Channelling's damage stat:** a hit row (`mapping::HIT_ROWS`).
- **`data-parity` `gem_effects`:** reads the damage taken row with PoB's `modCond`.

**Evidence** (`local/p6/p6-11aa/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected.
  - Match went from 13,607 to 13,651.
  - 40 values moved, all toward PoB; 15 now equal it.
  - 44 fields went from partial to complete.
- **Corpus:** unchanged; 0 unexpected.
- **Infused Channelling's stats:** neither is unmapped anywhere now. The damage taken stat was in 4 builds and the damage stat in 4.
- **`data-parity`:** `gem_effects` 0 differences; uncategorized 353, unchanged.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ab: Rapid Decay and Swift Affliction

**PoB:**
- **Rapid Decay's `support_rapid_decay_damage_over_time_+%_final`:** `Damage` MORE with `ModFlag.Dot` (`sup_dex.lua` 3854).
- **Swift Affliction's `support_swift_affliction_skill_effect_and_damaging_ailment_duration_+%_final`:** `Duration` MORE and `DamagingAilmentDuration` MORE (`SkillStatMap.lua` 705).

**The engine:** both are hit rows (`mapping::HIT_ROWS`).
- **Rapid Decay:** its row is `Damage` MORE with `Flags::DOT`.
- **Swift Affliction:** its row is `AilmentDuration` MORE on the damaging ailments. Its `Duration` part is left out: nothing modelled reads a skill's duration (the `skill-duration` rows of `NO_EFFECT`), and an ailment that counts as a skill effect is already a reason (`poe1::ailment`).

**Evidence** (`local/p6/p6-11ab/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; match unchanged at 13,651.
  - 8 values moved, all toward PoB: the damage over time of essence-drain-contagion-occultist, lacerate-bleed-gladiator, and rf-chieftain-life-stack. Their fields stay partial for other reasons.
- **Corpus:** 4 values moved, all toward PoB; 0 unexpected.
- **The two stats:** no longer unmapped anywhere; each was in 6 builds.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ac: the eldritch block implicit

**The stat:** the eldritch implicit "#% Chance to Block Attack Damage" resolves to `monster_base_block_%`. PoB reads the line, and the stat, as `BlockChance` BASE (`SkillStatMap.lua` 434). It is now a row (`mapping::ROWS`).

**Looked at and left, with why:**
- **`maximum_physical_attack_damage_+%_final`:** PoB parses "maximum physical attack damage" as `MaxPhysicalDamage` (`ModParser.lua` 711), which no calculation reads. In PoB it has no effect, but the game applies it. It stays a reason until there is evidence for a divergence entry.
- **`withered_effect_+%`:** reaches a field only through `WitherEffectStack`, which every skill with `CanWither` adds (`CalcPerform.lua` 2704). That includes a skill Withering Touch supports, which `poe1::gems` models as a plain support. Settling it needs the wither sources first.
- **`recover_x_energy_shield_on_spell_block`:** one of `EHP_REASONS`, an intended reason until the EHP pools are modelled.

**Evidence** (`local/p6/p6-11ac/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected; match unchanged at 13,651.
  - 18 values moved, all toward PoB; 3 now equal it. The moved fields are block, spell block, and the EHP fields that read them.
- **Corpus:** 16 values moved, all toward PoB; 4 now equal it. 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ad: low life and low energy shield in the pools, and the life thresholds

**PoB** (`CalcDefence.lua` 1061 and 2215 to 2220):
- **Low life:** with the config's `conditionLowLife`, `LifeRecoverable` is `min(Life x LowLifePercentage / 100, LifeUnreserved)`. `LowLifePercentage` is 100 times the `LowLifePercentage` BASE sum when above 0, else `data.misc.LowPoolThreshold` (0.5; `CalcPerform.lua` 74).
- **Low energy shield:** with `conditionLowEnergyShield`, `EnergyShieldRecoveryCap` is `min(EnergyShield x 0.5, cap)`.
- **The pools:** maximum hit and the effective health pool read both.
- **The threshold lines:** "You count as on Low Life while at #% of maximum Life or below" is `LowLifePercentage` BASE, a fraction (`ModCache.lua` 12888). "You count as on Full Life while at #% of maximum Life or above" is `FullLifePercentage`, which PoB reads only for a config tooltip (`ConfigOptions.lua` 68).

**The engine:**
- **The pools** (`poe1::enemy_hit`): both checks now apply as PoB does. Before, either check turned maximum hit into a reason and left the effective health pool uncomputed.
  - A value that is not a boolean is still an `unmodelled-config` reason.
  - The new quantity `Poe1Quantity::LowLifePercentage` is read for the threshold.
- **`low_life_threshold_%_override`:** a row of that quantity at scale 0.01. Its other use in PoB, the `LowLife` condition from reservation (`CalcPerform.lua` 546), reaches nothing modelled: no modelled modifier reads the character's `LowLife`.
- **`full_life_threshold_%_override`:** a `NoEffect` row (`life-threshold`).
- **`conditionLowEnergyShield`:** joins `config::NAMED_ONLY` beside `conditionLowLife`. Each sets only its named condition, beyond the pool cap now modelled.

**New mechanic fixtures** (each run through PoB v2.67.2, 3 of 3 runs identical; `local/p6/fx-pools-oracle/`). Each adds "+300 to maximum Energy Shield" to the base Ice Nova Witch:
- **`pools-low-life-ice-nova-witch.xml`:** the low life check on.
- **`pools-low-life-threshold-ice-nova-witch.xml`:** the low life check with the 55% threshold line.
- **`pools-low-energy-shield-ice-nova-witch.xml`:** the low energy shield check on.

All three match PoB on every computed field (231), including each type's maximum hit, hits to die, and total EHP.

**Fixed on the way:** this step first placed the new row after `monster_base_block_%` in `mapping::ROWS`. Its sorted lookup then missed P6-11ac's block row, moving 18 values away from PoB. The row now sits in sorted order; that run was not committed.

**Evidence** (`local/p6/p6-11ad/`):
- **Public `calc-parity`:** 215 builds, 280 expected, 0 unexpected.
  - Match went from 13,651 to 13,882: the three fixtures' 231 fields.
  - No other value moved.
  - lightning-trap-of-sparking-elementalist sets the low life check. Its maximum hit and effective health pool are now computed, but they stay indeterminate because of the build's other reasons.
- **Corpus:** 12 fields of the one build with the check go from unsupported or not computed to partial. 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ae: guard skills in the defence estimate

**PoB** (`CalcDefence.lua` 2379 to 2395, 296 to 307, 3136 to 3146):
- **The guard's rows:** a guard skill's rows are `unscalable` `Guard` buffs (Molten Shell and Vaal Molten Shell, `act_str.lua` 7545 to 7554; Steelskin, 10952 to 10957).
  - `GuardAbsorbRate` BASE: the share of each hit the guard takes.
  - `GuardAbsorbLimit` BASE: Steelskin's flat limit. Molten Shell's is a share of armour (`PerStat` `Armour`, `floor(Armour) x pct / 100`), at most `Multiplier:GuardAbsorbMax` in total.
- **The effective health pool:** the shared guard takes its rate of each type's remaining damage, at most what is left of it, before energy shield. It is floored after each take, and what is left carries to the next hit.
- **Maximum hit:** each type's pool becomes `max(pool - protected, 0) + min(pool, protected) / (1 - rate)`, with `protected = limit / rate x (1 - rate)`, or `pool + limit` at a rate of 100%.

**The engine:**
- **`poe1::gems`:** a new `BuffStat::Unscalable` takes scale 1 for its rows. The two Molten Shells' and Steelskin's guard stats, which were `Defence("guard")` pool markers, are such rows of four new quantities:
  - `GuardAbsorbRate`;
  - `GuardAbsorbLimit`;
  - `GuardAbsorbPerArmour`;
  - `GuardAbsorbMax`.
- **The armour row:** Molten Shell's armour row keeps the guard's buff scale.
- **The pools:** `enemy_hit` reads the four for the shared guard. `ehp` carries the guard through `reduce` and `hits_to_die`.
- **Left as a reason:** a type's own guard has no modelled source. The Stone Golem's damage taken before you is an ally pool, which stays a pool marker.

**Evidence** (`local/p6/p6-11ae/`):
- **Public `calc-parity`:** 215 builds, 280 expected, 0 unexpected.
  - Match went from 13,882 to 13,894.
  - 30 values moved, all toward PoB; 10 now equal it.
  - 12 fields went from partial to complete.
- **Corpus:** 58 values moved, all toward PoB; 0 unexpected.
- **`data-parity`:** `gem_effects` 0 differences; uncategorized 353, unchanged.
- **Tests:** a unit test checks the guard's share in `reduce`. The workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11af: Efficacy, and damage per herald

**PoB:**
- **Efficacy** (`sup_int.lua` 1984 to 1989): "#% more Damage over Time" is `Damage` MORE with `ModFlag.Dot`, and its spell damage is the same with `ModFlag.Spell`.
- **Damage per herald:** "#% increased Damage for each Herald affecting you" is `Damage` INC with `Multiplier:Herald` (`ModParser.lua` 1397). That multiplier counts heralds alone, one per skill name (`CalcPerform.lua` 2112 to 2126).

**The engine:**
- **Efficacy's two stats:** hit rows (`mapping::HIT_ROWS`).
- **Damage per herald:** `poe1::stacks` has a new kind, `Herald`, whose per-count effect is `Damage` INC. `gems::herald_and_aura_counts` returns the two counts that P6-11s's `aura_or_herald_count` adds together.

**Left, with why:**
- **Trap and Mine Damage Support:** its damage is `Damage` MORE with the trap and mine keyword flags, which the engine does not have. A trap or mine skill is `Partial` already (`hit::TRAP_MINE_REASON`).
- **Multistrike's damage and speed:** with its repeats, which make the skill unvetted (`hit::pob_vetted`).

**Evidence** (`local/p6/p6-11af/`):
- **Public `calc-parity`:** 215 builds, 280 expected, 0 unexpected; match unchanged at 13,894.
  - 4 values moved, all toward PoB: OccVortex's damage over time and essence-drain-contagion-occultist's full DPS. Their fields stay partial for other reasons.
- **Corpus:** 2 values moved, both toward PoB; 0 unexpected.
- **The stats:** no longer unmapped anywhere.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ag: heralds' mana reservation efficiency

**PoB:** "#% increased Mana Reservation Efficiency of Herald Skills" is `ManaReservationEfficiency` INC tagged `SkillType.Herald` (`ModParser.lua` 1027, "of herald skills"). A herald's reservation sums it with the other mana and generic efficiencies, through the skill's own config (`CalcActiveSkill.lua`'s reservation terms).

**The engine:**
- **`poe1::skill_modifiers`:** a new row of `Poe1Quantity::HeraldManaReservationEfficiency` (`STATS`, eleven rows).
- **`Totals::reservation_for`:** adds the row to a herald's mana efficiency. `poe1::gems` reads each other gem's reservation through it.
- **Tests:** `tests/skill_modifiers.rs` carries the row's game text, and a unit test checks the herald-only sum.

**Evidence** (`local/p6/p6-11ag/`):
- **Public `calc-parity`:** 215 builds, 280 expected, 0 unexpected; match unchanged at 13,894.
  - 11 values moved, all toward PoB: penance-brand-ignite-elementalist's unreserved mana (-402 to 157, PoB 78) and the defence fields that read it. The build's fields stay partial for other reasons.
- **Corpus:** 13 values moved, all toward PoB; 0 unexpected.
- **The stat:** no longer unmapped anywhere.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ah: fire resistance modifiers applying to cold and lightning

**PoB** (`calcs.resistances`, `CalcDefence.lua` 513 to 556), before it computes the resistances:
- **Maximum:** "Modifiers to Maximum Fire Resistance also apply to Maximum Cold and Lightning Resistances" is `FireMaxResConvertTo<Cold|Lightning>` BASE 100. It adds the `FireResistMax` BASE mods not from the base to each target at that rate.
- **Resistance:** "Modifiers to Fire Resistance also apply to Cold and Lightning Resistances at #% of their Value" is `FireResConvertTo<...>` BASE. It adds the `FireResist` BASE mods not from the base (the resistance penalty), and every `FireResist` INC and MORE mod, at that rate.
- **Which mods count:** only mods named `FireResist`, so `ElementalResist` and `ElementalResistMax` mods do not. They are tabulated with no skill config.

**The engine:**
- **The pass:** `poe1::resist_conversion` runs after every source of resistance modifiers, buffs included. The two stats are rows of a new quantity, `Poe1Quantity::ResistConvert(maximum, to)`. The maximum stat has no placeholder, so the resolver reads it as 1, and its row scales by 100.
- **All-elemental lines:** the engine reads an elemental line as three effects. A fire effect is therefore left out when the same source gives cold and lightning the same op, value, and conditions: an all-elemental line, Purity of Elements, the bandit's 15.
- **Left out as PoB does:** effects that need query flags, the character's base maximum, and the penalty.
- **A conditional rate:** an `unmodelled-skill` reason (`resistance-conversion`) on every field.
- **Tests:** a unit test checks the conversion and these exclusions.

**Evidence** (`local/p6/p6-11ah/`):
- **Public `calc-parity`:** 215 builds, 280 expected, 0 unexpected; match unchanged at 13,894.
  - 13 values moved, all toward PoB: rf-chieftain-life-stack's cold resistance (31 to 80, PoB 90) and lightning resistance (-9 to 80, PoB 90), and the defence fields that read them.
  - The rest of that gap is the build's own fire maximum (80, PoB 90): a reason the build already carried.
- **Corpus:** 37 values moved, all toward PoB; 28 now equal it. 0 unexpected.
- **The two stats:** no longer unmapped.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ai: on full life

**PoB** sets `FullLife` from the config's `conditionFullLife` (`ConfigOptions.lua` 174). The only other source is Chaos Inoculation (`CalcPerform.lua` 79 to 83). P6-11w's note said it was also derived from the life left unreserved; that was wrong, and `FullLifePercentage` is a tooltip value alone (P6-11ad).
- **Spell suppression:** "+#% chance to Suppress Spell Damage while on Full Life" is `SpellSuppressionChance` BASE with `FullLife`.
- **Attack speed:** "#% increased Attack Speed when on Full Life" is `Speed` INC with attacks and `FullLife`.

**The engine:**
- **`poe1::conditions`:** settles `FullLife` from the check. The check leaves `config::NAMED_ONLY`.
- **Chaos Inoculation:** its keystone stat is unmapped, so such a build is `Partial` everywhere.
- **The two lines:** state rows (`mapping::STATE_ROWS`).

**Evidence** (`local/p6/p6-11ai/`):
- **Public `calc-parity`:** 215 builds, 280 expected, 0 unexpected; match unchanged at 13,894; no value moved.
- **Corpus:** 5 values moved, 0 unexpected.
  - **Toward PoB:** spell suppression from 54 to 64 (PoB 65), and the damage chance with it.
  - **Away from PoB:** three of the same build's estimate fields, hits and total EHP. They were already above PoB's for other reasons, and the extra suppression raises them further. The build is `Partial`.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11aj: repeats (Multistrike and Spell Echo)

**PoB** (`CalcOffence.lua` 774 to 870, 2359, 2367, 5803 to 5819):
- **The count:** `Repeats = 1 + RepeatCount` BASE in the skill's config, for an attack or a spell that is not `Instant`, `Channel`, `Triggered`, `Retaliation`, or `NonRepeatable`.
  - Multistrike's count needs a melee weapon or no weapon (`ModFlagOr`); Spell Echo's needs a cast.
  - An attack's config carries the main hand's weapon flags (`CalcActiveSkill.lua` 455).
- **The repeat mode:** the config's `repeatMode` is `AVERAGE` by default; every new config set has it (`ConfigTab.lua` 1237). In that mode:
  - **Per repeat** (Greater Spell Echo): `Damage` MORE `value x (Repeats - 1) / 2`.
  - **The final repeat's double damage** (Awakened Spell Echo): `DoubleDamageChance` `value / Repeats`.
  - **Each repeat's own damage** (Multistrike's first to third): one `Damage` MORE of their sum over `Repeats`. That is 22% for Multistrike and 33% for Greater Multistrike.
- **The rate:** each repeat is a use. The cooldown's and the server tick's caps are times `Repeats`, and the costs per second are `cost x Speed / Repeats`.

**The engine:**
- **`poe1::repeats`** settles the count at the start of each spell and attack hit and adds the derived effects to the actor's store, as PoB's `NewMod` does. This needs a new `Calc::add_effect`.
  - The averaged damage is a quantity of its own (`RepeatAverageDamage`). The hit's MORE multiplies it in as a separate group.
  - **Evidence for the separate group:** a PoB probe of the Multistrike fixture with `repeatMode` set to `NONE` gives lightning 281.75 against `AVERAGE`'s 343.5, a ratio of 1.22. So PoB does not round the 22% together with Multistrike's 20% less (0.976, not 0.98).
- **`hit::rate`:** multiplies both caps by the count.
- **`costs::per_second`:** divides `Speed` by it before the cost, in PoB's order.
- **New rows:**
  - Multistrike's and Spell Echo's stats are hit rows: the repeat counts, each repeat's damage, Greater Spell Echo's damage per repeat, Awakened Spell Echo's double damage, Multistrike's less damage and more speed, Spell Echo's more damage and cast speed.
  - The repeats' target choice and Greater Spell Echo's area are `NoEffect`.
- **Not modelled, each a reason:** the `FINAL` and `FINAL_DPS` modes (`unmodelled-config`); Multistrike's `RequiresShield` alternative (no such skill is a modelled attack); a trap's or a mine's repeats (those skills are `Partial` already).

**New mechanic fixtures** (each run through PoB v2.67.2, 3 of 3 runs identical; `local/p6/fx-rep-oracle/`):
- **Smite:** with Multistrike, and with Greater Multistrike.
- **Ice Nova:** with Spell Echo, and with Greater Spell Echo.

The two spells match PoB on every field. The two attacks match on everything except the rate's Q11 floor (attack time floored to whole milliseconds, answered in game) and the fields that read it: registered as DIV-0282 to DIV-0293 (`game-truth`).

**Evidence** (`local/p6/p6-11aj/`):
- **Public `calc-parity`:** 219 builds, 292 expected, 0 unexpected.
  - Match went from 13,894 to 14,194: the four fixtures.
  - 8 values moved, 7 toward PoB: shock-nova-archmage-hierophant's speed (4.30 to 6.62, PoB 8.16), DPS, and leech, and animate-guardian's full DPS.
  - Away from PoB: shock nova's mana cost per second (202 to 156, PoB 2,951), by the division PoB makes. Its gap is the cost per cast, a reason the build already carried.
- **Corpus:** 23 values moved, 19 toward PoB; 0 unexpected. The 4 away are in two partial builds:
  - One build's attack speed: its skill is unvetted (`pob-skill-data`: PoB computes it from data the engine does not read).
  - The average damage and mana cost per second of builds already far from PoB's.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ak: life and energy shield gained on block in the effective health pool

**PoB** (`CalcDefence.lua` 1525 to 1532, 2569 to 2730, 2747 to 2840):
- **The gains:** `LifeOnBlock`, `EnergyShieldOnBlock`, and `EnergyShieldOnSpellBlock` BASE. A percent line ("Recover #% of Life when you Block", "Recover Energy Shield equal to #% of Armour when you Block") is `PercentStat`: `ceil(stat x percent / 100)` per mod.
- **When they count:** with more than one damaging hit, the mitigated estimate gains each hit's worth times the average block chance; spell block's energy shield is halved for `Average`.
  - Each pool is capped at its maximum after each hit.
  - An accelerated iteration first gains `gainMult - 1` hits' worth, capped at `gainMult` times the maximum.
- **Two rules a PoB oracle probe showed:**
  - **The speed-up:** with gains it is limited to 4 (`LimitEHPSpeedup`, 2837).
  - **The top-level clamp:** the mitigated run sets `MissingLifeBeforeEnemyHit` and `MissingManaBeforeEnemyHit` to 0, a true value in Lua. So at its top level life and mana go back to at most their caps before each hit, which drops most of an accelerated life gain. The recursion does not carry the fields, and energy shield is never clamped.
  - **How the probe was made:** a copy of the oracle adapter (local, not committed) wrapped `calcs.reducePoolsByDamage` to log each call. A model with both rules gives PoB's numbers to the last digit.

**The engine:**
- **The mapping:** the five on-block stats, which were `EhpReason` markers, are rows of `Poe1Quantity::BlockGain` (flat life, percent of life, energy shield on spell block, energy shield as a percent of armour).
- **`enemy_hit`:** sums them per effect.
- **`ehp::hits_to_die`:** applies the gains, the limited speed-up, and the clamp.

**New mechanic fixtures** (each run through PoB v2.67.2, 3 of 3 runs identical; `local/p6/fx-block-oracle/`). Each is an Ice Nova Witch with life, energy shield, armour, and 40% block against a non-boss enemy:
- **`ehp-block-none-ice-nova-witch.xml`:** no gain (the reference).
- **`ehp-life-on-block-ice-nova-witch.xml`:** life on block.
- **`ehp-energy-shield-on-block-ice-nova-witch.xml`:** energy shield on block.

All three match PoB on every computed field (231). "+100 Life gained when you Block" does not resolve, since the game's text has no sign, so the fixture uses "100 Life gained when you Block", which PoB reads the same.

**Evidence** (`local/p6/p6-11ak/`):
- **Public `calc-parity`:** 222 builds, 292 expected, 0 unexpected.
  - Match went from 14,194 to 14,425: the fixtures' 231 fields.
  - 21 values moved, all toward PoB: four builds' estimates (kinetic-fusillade-ballista, penance-brand-ignite, poisonous-concoction, shock-nova).
- **Corpus:** 21 values moved, all toward PoB; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11al: lucky spell suppression

**PoB** (`CalcDefence.lua` 1131 to 1157): "Chance to Suppress Spell Damage is Lucky" is `SpellSuppressionChanceIsLucky`.
- **The effective chance:** in the effective mode, `EffectiveSpellSuppressionChance` becomes `(1 - (1 - chance)^2) x 100`.
- **Who reads it:** the hit's suppression at 100% (2028) and the effective health pool's suppression share. The chance field itself is unchanged.

**The engine:**
- **The mapping:** the stat, which was an `EhpReason` marker, is a row of a new flag, `Poe1Quantity::SuppressionLucky`.
- **`enemy_hit::effective_suppression`:** applies it at both reads.
- **Left as reasons:** unlucky suppression, extreme luck, and Unexciting are unmapped stats.

**New mechanic fixtures** (each run through PoB v2.67.2, 3 of 3 runs identical): `ehp-suppression-ice-nova-witch.xml` (40% suppression on a body armour) and `ehp-suppression-lucky-ice-nova-witch.xml` (the same, lucky).
- Both match PoB on every computed field. With the lucky line, the damage chance goes from 92 to 87.2.
- **Why a body armour:** "+#% chance to Suppress Spell Damage" is ambiguous as a custom line or on a ring. The game's descriptions give the same text to `base_spell_suppression_chance_%` and `base_spell_suppression_chance_150%_of_value`, and only a base whose mods roll the stat settles it.

**Evidence** (`local/p6/p6-11al/`):
- **Public `calc-parity`:** 224 builds, 292 expected, 0 unexpected.
  - Match went from 14,425 to 14,579: the fixtures.
  - 4 values moved, all toward PoB (kinetic-blast-of-clustering-necromancer's estimate).
- **Corpus:** 12 values moved, all toward PoB; 3 now equal it. 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11am: Divine Shield

**PoB:** the keystone is `Keystone` LIST, so its node's lines apply.
- **"Cannot Recover Energy Shield to above Armour"** is `ArmourESRecoveryCap`. With armour below energy shield, `EnergyShieldRecoveryCap` is the armour (`CalcDefence.lua` 1055 to 1063), and maximum hit and the effective health pool read that cap.
- **"3% of Physical Damage prevented from Hits Recently is Regenerated as Energy Shield per second"** is `PhysicalDamageMitigatedEnergyShieldPseudoRecoup` BASE 12. It reaches no field we compute, but it makes `anyRecoup` above 0. The mitigated estimate then tracks recoupable damage: it is always rerun, at the limited speed-up of 4 (2828 to 2840).

**The engine:**
- **The keystone row:** `keystone_divine_shield` moves from `POOL_REASONS` to a `keystones` row with two effects, the flag `ArmourEsRecoveryCap` and `PseudoRecoup` BASE 12.
- **`enemy_hit`:** caps the energy shield pool at armour, before the low energy shield check (P6-11ad), and passes on whether any recoup is present.
- **`ehp`:** limits the mitigated run's speed-up when there is recoup.
- **Tests:** `tests/keystones_chill.rs` carries the keystone's passive text.

**New mechanic fixtures** (each run through PoB v2.67.2, 3 of 3 runs identical; the custom line "Divine Shield" resolves to the keystone stat):
- **`ehp-divine-shield-capped-ice-nova-witch.xml`:** armour 500, below energy shield, so the cap applies.
- **`ehp-divine-shield-ice-nova-witch.xml`:** armour above energy shield, so only the limited speed-up applies. The mitigated hits move from 6.88245 to 6.88379, as PoB's.

Both match PoB on every computed field.

**Evidence** (`local/p6/p6-11am/`):
- **Public `calc-parity`:** 226 builds, 292 expected, 0 unexpected; match 14,579 to 14,733 (the fixtures).
  - No other value moved: the public Divine Shield builds' estimate fields carry other reasons.
- **Corpus:** 3 values moved, all toward PoB; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11an: Blood Magic

**PoB** reads the keystone's lines:
- "Removes all mana" is `Mana` MORE -100.
- "10% more maximum Life" is `Life` MORE 10.
- "Skills Cost Life instead of Mana" is `CostLifeInsteadOfMana`.
- "Skills Reserve Life instead of Mana" is `BloodMagicReserved`: a skill's mana reservation goes to life (`CalcPerform.lua` 1955 to 1964).

**The engine:**
- **The keystone row:** a `keystones` row with the four effects. The cost flag is the one P6-10b part 8 reads.
- **The reservation flag:** a new `Poe1Quantity::ReserveLifeInsteadOfMana` that `skill_modifiers::Totals` reads. `poe1::gems` then reserves life for every other skill's mana reservation, as it already did for Arrogance.
- **Tests:** `tests/keystones_chill.rs` carries the keystone's text. `tests/skill_modifiers.rs` accepts the flag among the stats that name reservation.

**New mechanic fixture:** `blood-magic-ice-nova-witch.xml`, Ice Nova with Anger and the line "Blood Magic", run through PoB v2.67.2 (3 of 3 runs identical). It matches PoB on every compared field: life 1,238, mana 0, unreserved life 619, the life cost 23.

**Evidence** (`local/p6/p6-11an/`):
- **Public `calc-parity`:** 227 builds, 292 expected, 0 unexpected; match 14,733 to 14,810.
- **Moved values:** 41 moved, 36 toward PoB, 9 now equal.
  - The 5 away are lacerate-bleed-gladiator's life and the fields that read it (3,865 to 4,252, PoB 3,834).
  - PoB's breakdown has the same ×1.10 more: 1,734 base × 2.01 increased × 1.10.
  - The build's life base was already over PoB's by 189 (1,923, PoB 1,734, its strength bonus included), which the missing more had hidden. The build is `Partial`.
- **Corpus:** 21 values moved, all toward PoB; 5 now equal it. 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ao: Eternal Youth, and keystones a Timeless jewel replaces

**PoB, Eternal Youth:** the keystone's lines are:
- "50% less Life Regeneration Rate" (`LifeRegen` MORE -50);
- "50% less maximum Total Life Recovery per Second from Leech" (`MaxLifeLeechRate` MORE -50);
- "Energy Shield Recharge instead applies to Life" (`EnergyShieldRechargeAppliesToLife`). PoB then computes `LifeRecharge`, which is no slice field, and sets no `EnergyShieldRecharge`. The delay stays (`CalcDefence.lua` 1325 to 1371).

**The engine, Eternal Youth:**
- **The keystone row:** a `keystones` row with the three effects, as a new flag `Poe1Quantity::EsRechargeAppliesToLife`.
- **`regen::recharge`:** sets no energy shield recharge when the flag is on. It checks with `phases::has` first, so a build without the flag gains no graph node.
- **Tests:** `tests/keystones_chill.rs` carries the keystone's text.

**Found on the way: Timeless jewels.**
- Mapping the keystone moved four public builds away from PoB (Eternal Youth applied, PoB's leech still 20%). Each has a Timeless jewel (Glorious Vanity or Elegant Hubris) whose radius holds the keystone.
- A Timeless jewel replaces every keystone in its radius with its conqueror's keystone, whatever its seed. Those builds allocate Eternal Youth for the replacement, and PoB never applies Eternal Youth's own lines.
- **The fix:** `loom_pipeline::timeless_keystones` finds the keystones in the radius of each socketed Timeless jewel: its base is "Timeless Jewel", or a line resolved to a `local_unique_jewel_alternate_tree_*` stat, and its `Radius:` line names the ring.
  - A connected keystone there is a new `PassiveState::TimelessKeystone`. `collect` turns it into an `unmodelled-passive` reason (`timeless-keystone`) instead of applying its lines.
  - **Why the base check:** one build's Glorious Vanity resolved to no such stat, so the stat check alone missed it.
  - The replacement keystones wait for the Timeless data run (a maintainer item).
  - **Superseded by P6-10a part 3** (`docs/phase-6/timeless.md`): `timeless_keystones` and `PassiveState::TimelessKeystone` are gone; the keystone becomes its conqueror's, applied.
- **Confirmed in PoB's own output:**
  - Eternal Youth dropped in those four builds leaves their leech at PoB's 20%.
  - In penance-brand-ignite-elementalist, the replaced keystone is Mind over Matter. PoB's `sharedMindOverMatter` is 0 there too.

**New mechanic fixture:** `eternal-youth-ice-nova-witch.xml` (the line "Eternal Youth", energy shield, and life regeneration), run through PoB v2.67.2 (3 of 3 runs identical). It matches PoB on every compared field: life regeneration 11.3, the leech rate at 10%, the recharge delay 1.6, and no energy shield recharge on either side.

**Evidence** (`local/p6/p6-11ao/`):
- **Public `calc-parity`:** 228 builds, 292 expected, 0 unexpected; match 14,810 to 14,886 (the fixture).
- **Moved values:** 10 moved away from PoB, all the penance-brand build's estimate (its maximum hit and effective health pool lose Mind over Matter, which PoB does not apply either).
  - The build stays far below PoB's for other reasons (maximum hit 37,014 against 106,161).
- **Corpus:** the same build moves the same way; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ap: Eldritch Battery

**PoB:** the keystone's lines are:
- "Spend Energy Shield before Mana for Skill Mana Costs": PoB parses it to nothing (`ModParser.lua` 2310). The cost moves with the next line's flag.
- "Energy Shield protects Mana instead of Life" (`EnergyShieldProtectsMana`):
  - **Pools:** the shield leaves the hit pool (`CalcDefence.lua` 2536 to 2552) and enters mind over matter's pool, before the mana (`calcMoMEBPool`, 2309 to 2321).
  - **Simulated hits:** in `reducePoolsByDamage`, the shield takes mind over matter's share before mana does (254 to 256, 319 to 339).
  - **Cost per second:** the mana cost per second becomes `ESPerSecondCost` (`CalcOffence.lua` 5793 to 5825).
- "50% less Energy Shield Recharge Rate" (`EnergyShieldRecharge` MORE -50).

**The engine:**
- **Mapping:** a `keystones` row with a new flag `Poe1Quantity::EsProtectsMana` and the recharge MORE -50. The standalone stat `energy_shield_protects_mana` is a row with the same flag.
- **`enemy_hit`:** `mom_hit_pool` is PoB's `calcMoMEBPool` in full. With the flag, the shield of a type that does not bypass it (all but chaos) is in that pool and not added to the hit pool. The flag is read only when present (`phases::has`), so no other build gains a graph node.
- **`ehp`:** `Character.eb`. The pools start as PoB's, and the shield takes the mana share first.
- **`costs`:** under the flag, the mana cost per second is set as `ESPerSecondCost`.
  - **The energy shield cost's own per second:** PoB then skips it.
  - **Mana cost also moved to energy shield:** PoB adds both, which the engine leaves as a reason (`eldritch-battery-cost-per-second`).
- **Tests:** `tests/keystones_chill.rs` carries the keystone's text.

**New mechanic fixtures** (run through PoB v2.67.2, 3 of 3 runs identical), each matching PoB on every compared field:
- `eldritch-battery-ice-nova-witch.xml`: the keystone without mind over matter. The shield is in no pool (maximum hit 1,125 physical), recharge is 134, and energy shield per second is 32.86.
- `eldritch-battery-mom-ice-nova-witch.xml`: with 40% mind over matter. The shield fills the mana share first (maximum hit 1,875 physical, total effective health pool 1,448.3).

**Evidence** (`local/p6/p6-11ap/`):
- **Public `calc-parity`:** 230 builds, 292 expected, 0 unexpected; match 14,886 to 15,040 (the fixtures).
- **Moved values:** 1 toward PoB (animate-guardian-guardian's recharge, 880 to 440 against PoB's 412).
- **Corpus:** 0 unexpected; 9 values toward PoB, 7 away.
  - The 7 away are one build's maximum hit and effective health pool. PoB applies the keystone there too: its `ESPerSecondCost` is set and the recharge moved toward it.
  - That build already had less energy shield than PoB (457 against 680) and negative unreserved mana (-286 against 80). With the shield in mind over matter's pool, those gaps now show in the maximum hit.
- **Data parity:** `uncategorized` 353, unchanged.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11aq: Withered on the enemy

**PoB:**
- **Can wither:** any active skill whose mod list has `Condition:CanWither` lets the enemy be withered (`CalcPerform.lua` 2704 to 2707). The flag comes from:
  - `withered_on_hit_chance_%` and `withered_on_hit_for_2_seconds_%_chance` (`SkillStatMap.lua` 2202 to 2210; Withering Touch, Poisonous Concoction's quality, items);
  - one skill's own stat map: Wither, Withering Step, Toxic Rain of Withering, Summon Void Spawn.
- **Effect per stack:** each such skill gives `WitherEffectStack` MAX `floor(6 x (1 + WitherEffect / 100))`. A minion skill uses `MinionWitherEffect` instead.
- **On the enemy** (812 to 815): `ChaosDamageTaken` INC of that effect per stack. The stacks are the `multiplierWitheredStackCount` input (`ConfigOptions.lua` 1813), at most 15.

**The engine:**
- **New quantities:** `Poe1Quantity::CanWither` (a flag) and `WitherEffect` (increased).
- **Mapping:**
  - `withered_effect_+%` becomes `WitherEffect` INC.
  - The two chance stats set the `CanWither` flag. A gem's copy belongs to its own skill, and `gems::can_wither` reads it.
  - New: "Adds # to # Chaos Damage to Spells" (`spell_minimum_added_chaos_damage`, `spell_maximum_added_chaos_damage`), as the other elements' rows (PoB's `ChaosMin`/`ChaosMax` BASE, spell keyword).
- **`gems::can_wither`:** goes over the skills `affected_by` reads. A skill can wither when:
  - the character has the flag, or
  - its gem's or a support's stats at their levels hold one of the stats above.

  It returns whether a player skill can and whether a minion skill can.
- **`damage_taken::withered`:**
  - **Effect per stack:** the larger of the player's `floor(6 x (1 + WitherEffect / 100))` and a minion skill's 6 (no mapping gives `MinionWitherEffect`).
  - **Enemy damage taken:** it adds `EnemyTypeDamageTaken(Chaos)` INC of that effect times the stacks.
  - **Unread conditions:** a conditional `WitherEffect` is a reason.
  - **Bad stack values:** a stack count that is not a number is a reason only when a skill can wither.
- **The input:** `multiplierWitheredStackCount` leaves `config::NAMED_ONLY`; `damage_taken` handles it.
- **Withering Step:** now in `gems::SKILLS` as a buff whose two global effects are Elusive's (`Condition:CanBeElusive`, `ElusiveEffect`).
  - Neither reaches a slice field: Elusive itself is the `buffElusive` named condition.
  - **Data parity:** the two names join `gem_effects`' `NO_SLICE_NAMES`; `uncategorized` stays 353.

**New mechanic fixtures** (run through PoB v2.67.2, 3 of 3 runs identical), each matching PoB on every compared field:
- `withered-step-ice-nova-witch.xml`: Withering Step in a second group, 20% increased Effect of Withered, 15 stacks. Chaos taken is 7% per stack: average hit 1,006.70, hit DPS 1,438.14.
- `withered-touch-ice-nova-witch.xml`: Withering Touch on the main skill, 8 stacks. Average hit 863.72.

**Evidence** (`local/p6/p6-11aq/`):
- **Public `calc-parity`:** 232 builds, 292 expected, 0 unexpected; match 15,040 to 15,194 (the fixtures).
- **Moved values:** 3 toward PoB, none away. essence-drain-contagion-occultist's `TotalDot` went from 157,676 to 346,888 against PoB's 2,141,187.
- **Corpus:** 0 unexpected; 5 values toward PoB, none away.
- **Data parity:** `uncategorized` 353.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11ar: Summon Stone Golem of Safeguarding's life before yours

**PoB:**
- **The stat:** the golem's buff stat `stone_golem_grants_melee_damage_removed_from_stone_golem_before_life_or_es_%` is `takenFromStoneGolemBeforeYou` BASE (`SkillStatMap.lua` 2087), scaled by buff effect.
- **The golem's life** (`CalcPerform.lua` 3597 to 3677): with any such redirect, PoB computes the life of the first active skill's golem as a minion:
  - `initMinionModDB`;
  - the skill's and the buffs' minion modifiers;
  - then `doActorAttribsConditions`, which runs `doActorLifeMana`.

  The config input `TotalStoneGolemLife` overrides it.
- **The pool** (`CalcDefence.lua` 2459 to 2473): the share is melee only.
  - **Share by damage category:** all of the redirect under `Melee`, a quarter under `Average`, none otherwise.
  - **Pool:** `buildAllyLifePools` makes a pool of the life with that share, at most 100%.
- **Simulated hits** (`reducePoolsByDamage`, 266 to 277): the ally takes its share of each type's damage before aegis, guard, and energy shield, at most what is left, floored after each take. What is left carries to the next hit, as the guard.
- **Maximum hit** (3147 to 3160): the pools are undone after the guard, in reverse order.

**The engine:**
- **The stat:** the golem's row is now `Poe1Quantity::TakenFromStoneGolem` BASE, a buff row scaled as the others. Before, it was a pool marker, so a reason.
- **`enemy_hit::stone_golem_life`:**
  - **Life:** the override when set (now an applied input). Otherwise, the build is computed with the first active Summon Stone Golem of Safeguarding as the main skill (nested, as `full_dps` does), and its minion `Life` is read with its reasons.
  - **Inside a nested calculation:** no golem life is computed; an `ally-life` reason takes its place.
- **Share:** a quarter of the sum, since the engine computes the estimate only under `Average`; another damage category is already a reason.
- **`maximum_hit`:** adds the pool after the guard.
- **`ehp`:** `Character.golem_rate` and `golem`, and `Pools.golem`, drained before the guard. Unit test `an_ally_should_take_its_share_before_the_guard`.
- **Data parity:** `gem_effects` names the row `takenFromStoneGolemBeforeYou`; `uncategorized` stays 353.

**New mechanic fixtures** (run through PoB v2.67.2, 3 of 3 runs identical), each matching PoB on every compared field:
- `stone-golem-ice-nova-witch.xml`: the golem in a second group, 90% reduced minion life, and 20,000 more life. The computed golem life limits the pool: physical maximum hit is 21,661, and total effective health pool is 14,373.96.
- `stone-golem-override-ice-nova-witch.xml`: `TotalStoneGolemLife` 20. Physical maximum hit is 1,145.

**Evidence** (`local/p6/p6-11ar/`):
- **Public `calc-parity`:** 234 builds, 292 expected, 0 unexpected; match 15,194 to 15,360.
- **Moved values:** 28 toward PoB, none away, 18 of them now equal to PoB; 12 fields went from partial to complete. The minion fixture `minion-summon-stone-golem-alt-y-witch.xml` now matches PoB's maximum hit and effective health pool.
- **Corpus:** 0 unexpected; 30 values toward PoB, none away.
- **Data parity:** `uncategorized` 353.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11as: minion added damage and double damage, mana recoup, four rows of no effect

**PoB and the engine, row by row:**

| Stat | PoB | The engine |
|---|---|---|
| `minion_global_minimum_added_cold_damage`, `minion_global_maximum_added_cold_damage` ("Minions deal # to # additional Cold Damage") | `MinionModifier` `ColdMin`/`ColdMax` BASE, no flags | New `MinionValue::AddedDamage(type, bound)`, the minion's `AddedDamage` |
| `minion_chance_to_deal_double_damage_%` | `MinionModifier` `DoubleDamageChance` BASE | New `MinionValue::DoubleDamageChance`, the minion's `DoubleDamageChance` |
| `damage_taken_goes_to_mana_%` ("#% of Damage taken Recouped as Mana") | `ManaRecoup` BASE; with the mana recovery rate it counts in `anyRecoup` (`CalcDefence.lua` 1385 to 1400), which turns on the estimate's recoup tracking | New `Poe1Quantity::ManaRecoup` (was an `EhpReason` marker). `enemy_hit` adds it, times the mana recovery rate, to the pseudo recoup for `Character.recoup` |
| `area_of_effect_+%_per_endurance_charge` | `AreaOfEffect` INC per charge | `NoEffect` `area`, as `base_skill_area_of_effect_+%` |
| `base_avoid_chill_%` | chill on you | `NoEffect` `ailment-on-self` |
| `gain_X_life_on_stun` | the line is left unparsed | `NoEffect` `recovery` |
| `stun_duration_+%_per_endurance_charge` | `EnemyStunDuration` INC | `NoEffect` `stun` |

**Tests:**
- `tests/minions.rs` carries the three new rows' texts. Its real-bundle check now reads descriptions of two stats too.
- The charges and damage-taken tests list the new rows they meet.
- `tests/keystones_chill.rs` lists `base_avoid_chill_%`.

**New mechanic fixtures** (run through PoB v2.67.2, 3 of 3 runs identical), each matching PoB on every compared field:
- `minion-added-cold-double-damage-witch.xml`: Raise Zombie's hit with both lines. Minion average hit 201.15, minion hit DPS 171.93.
- `mana-recoup-ice-nova-witch.xml`: mana recoup with more life and mana. Total effective health pool 2,736.81.

**Evidence** (`local/p6/p6-11as/`):
- **Public `calc-parity`:** 236 builds, 292 expected, 0 unexpected; match 15,360 to 15,523.
- **Moved values:** 5 toward PoB, none away. Two minion builds' full DPS moved, and boneshatter-juggernaut's effective health pool.
- **Corpus:** 0 unexpected; 1 value toward PoB.
- **Data parity:** `uncategorized` 353.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11at: herald buff effect, three rows of no effect

**PoB and the engine, row by row:**

| Stat | PoB | The engine |
|---|---|---|
| `herald_effect_on_self_+%` ("#% increased Effect of Herald Buffs on you") | `BuffEffect` INC tagged `SkillType.Herald` | New `Poe1Quantity::HeraldBuffEffect`, a `skill_modifiers` row (`Totals.herald_buff_effect`). `buff_scale` adds it for a skill typed `Herald`, as the golems' buff effect for a golem. It leaves `gems::DEPENDS_ON` |
| `area_of_effect_+%_with_herald_skills` | `AreaOfEffect` INC, `SkillType.Herald` | `NoEffect` `area` |
| `base_stun_threshold_reduction_+%` | `EnemyStunThreshold` INC, `mult = -1` | `NoEffect` `stun` |
| `chance_to_unnerve_on_hit_%` | no stat map row and no parse, so no modifier; the enemy's Unnerve is the config's | `NoEffect` `ailment-chance`, listed in `damage_taken`'s test |

**Tests:**
- A unit assertion in `skill_modifiers`: a herald's scale 1.4 with 40%, none for a golem.
- `tests/skill_modifiers.rs` carries the text.

**New mechanic fixture:** `herald-effect-ice-nova-witch.xml` (Herald of Ice in a second group, 40% increased Effect of Herald Buffs on you). It was run through PoB v2.67.2 (3 of 3 runs identical) and matches PoB on every compared field: average hit 812.62, hit DPS 1,160.89.

**Evidence** (`local/p6/p6-11at/`):
- **Public `calc-parity`:** 237 builds, 292 expected, 0 unexpected; match 15,523 to 15,600 (the fixture).
- **Moved values:** none; the builds with these stats are partial for other reasons.
- **Corpus:** 0 unexpected, no value moved.
- **Data parity:** `uncategorized` 353.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11au: mana regeneration after inflicting exposure, two inert gems

### Mana regeneration after inflicting exposure

**PoB:** "Regenerate #% of Mana per second if you've inflicted Exposure Recently" is `ManaRegenPercent` BASE with `Condition:AppliedExposureRecently`. PoB sets that condition when any exposure is on the enemy (`CalcPerform.lua` 3570 to 3586). The stat comes from Mastermind of Discord (Elementalist).

**The engine:**
- **The row:** the stat is now a row of `exposure::STATS`, `Poe1Quantity::RegenIfExposureApplied(Mana)`, its value per minute. It is no longer a regeneration reason (`REGEN_REASONS`).
- **`exposure()`:** notes whether it put an exposure on the enemy. If so, each such line becomes `RegenPercent(Mana)` at its value per second; otherwise it does nothing. A conditional line is a reason.
- **Tests:** `tests/exposure_shock.rs` carries the text.

**Why no mechanic fixture:**
- The text's handler (`per_minute_to_per_second`) rounds to whole percents, so a typed custom line resolves to a range of values, an `unsettled-value` reason. The tree's notable has the exact value.
- **The community Elementalists check both cases.** Three builds with an exposure now match PoB's mana regeneration (114.1, 170.1, 103.1). `ice-nova-elementalist-campaign-lab1`, without one, still matches (75.1).

### Two inert gems

`ElementalHitAltX` and `SupportMoreDuration` join `gems::SKILLS` and `gems::SUPPORTS` as inert: no global effect in PoB's skill data.
- **Rejected:** the other candidates tried (Bear Trap, Blink Arrow, Mana Infused Staff, Pyroclast Mine of Sabotage, Vaal Blight, Hexpass, Living Lightning, Manaforged Arrows, Mark On Hit) have global effects, so `gem_effects` refused them; they stay `unmodelled-gem`.
- **EnemyExplode** is PoB's own skill ID with no granted effect in the game's data; it stays out (`tests/enemy.rs`).

**Evidence** (`local/p6/p6-11au/`):
- **Public `calc-parity`:** 237 builds, 292 expected, 0 unexpected; match 15,600 to 15,602.
- **Moved values:** 4 toward PoB, none away, 3 of them now equal to PoB; 2 fields went from partial to complete.
  - The fourth, `lightning-trap-of-sparking-elementalist`, is 115.0 against PoB's 116.6 for its other reasons.
- **Corpus:** 0 unexpected; 1 value toward PoB, now equal.
- **Data parity:** `uncategorized` 353.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

## P6-11aw: devotion and ward, the Timeless conquests' stats

The stats Militant Faith's and Heroic Tragedy's conquests give (`docs/phase-6/timeless.md`, "Part 3", left them as reasons).

**Devotion:**
- `base_devotion` ("+# to Devotion", PoB's `Devotion` BASE) is `NoEffect`. Devotion reaches a slice field only through the 30 stats that read it: per 10 Devotion (`..._per_10_devotion`) or at a threshold (`..._at_devotion_threshold`).
- Each of the 30 is a guard of the row in `NO_EFFECT_DEPENDS_ON`. They stay `unmapped-stat`, so a build with one is partial, and mapping one (even as `NoEffect`) fails the guard test until the row is revisited.
- Militant Faith's own threshold notables (exposure on hit, mana as energy shield at 150 Devotion) remain reasons.
- *Superseded by P6-11az:* `base_devotion` is a row, and its readers are modelled where the stat they mean is mapped.

**Ward** (`EHP_REASONS`, `Poe1Quantity::EhpReason`, the effective health pool fields' reason alone):
- `ward_+%` and `ward_from_armour_item_+%`: PoB's `Ward` INC, which its effective health pool reads through `output.Ward` (`CalcDefence.lua` 223). The engine has no ward pool.
- `restore_ward_on_block_%`: PoB reads "#% chance to Restore Ward when you Block" only in part (`Ward` BASE with text left over), and its tree adds no mod from such a line (`PassiveTree.lua` 816 to 836).
- The armour pieces' local ward stays as P3-8c left it (`LOCAL_ROWS`, no effect). The ward pool is an open item (tasks file, "ward and aegis").

**Fixtures** (`docs/phase-6/timeless.md`, "Part 3"):
- `timeless-militant-faith-ice-nova-witch.xml` (seed 3191): its attribute passives become the devotion node and the other small passives gain +5 Devotion. All 77 computed fields equal PoB's. The seed was picked so its notables carry no threshold stat.
- `timeless-heroic-tragedy-ice-nova-witch.xml` (seed 5000): its small passives gain increased Ward. 71 fields equal PoB's; the 6 effective health pool fields are partial (the ward reason).

**Evidence** (b-tt, against P6-11av's run):
- **Public and corpus:** 0 unexpected, no value moved; no build has these stats outside the fixtures.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle (the guards are stats of the real bundle), and clippy pass.

## P6-11ax: catalyst quality

Found while tracing the Strength gaps tattoos made visible (`docs/phase-6/tattoos.md`): Lightning Trap's Mageblood has `+(25-35) to Strength` at range 1 with Intrinsic catalyst quality 20%. PoB counts 42; the engine counted nothing, because the line was a `scaled-line` reason since P3-8b (`docs/phase-3/rolled-values.md`).

**The data:**
- The bundle's stats carry the game's `IsScalable` (`loom_data::bundle::Stat::scalable`, written by `bundle`). PoB's `data.modScalability` comes from the same column (`Export/statdesc.lua` `describeScalability`).
- The field is optional: `None` in a bundle written before, whose catalysed lines stay reasons. 16,159 of 23,362 stats are scalable.

**The rule** (`loom_serialization::legacy::pob_item::line_values_with`, PoB's `Item.lua` `getCatalystScalar` and `ItemTools.lua` `formatValue`):
- **Which lines:** a catalysed one-line reading whose `{tags}`, or `prefix` or `suffix` flag, meets the catalyst's tags, with each stat's scalability known.
- **The values:** each stat's raw value is the roll of a ranged one (as before), else its exact value. A scalable stat becomes `floorSymmetric(raw x (100 + quality) / 100)` in PoB's order of float operations; an unscalable one keeps its value.
- **Still reasons** (`scaled-line` `catalyst`):
  - a stat whose scalability is not known;
  - a multi-line reading;
  - an untagged ranged line (the game scales it; PoB does not);
  - the unnamed `Quality (... Modifiers)` line.
- **The new `LineValues::Catalysed`** carries the values: unlike `Rolled`, a value may lie outside the line's range (24 becomes 28), and an exact stat's value is replaced too.

**Tests:**
- `pob_item::values::tests::known_scalability_should_settle_catalysed_lines`;
- `ingest::tests::the_hosts_catalysed_values_should_replace_the_stats_values`.

**Evidence** (evidence bundle `local/p6/packs/b-ct`, `b-tt` with the new `data/stats.json` from extract `26530219-tt`, identical but the new field; against P6-11aw's run):
- **Public:** 0 unexpected; 196 values moved, 181 toward PoB, 15 away, 24 now equal; no state changed.
  - Lightning Trap's Strength gains Mageblood's 42 and now equals PoB's 131 (89 since its tattoos).
  - Poisonous Concoction's Dexterity now equals PoB's (376).
  - **The away moves** are in builds with larger gaps. Essence Drain's life is 1 in PoB, under Chaos Inoculation. Mana reserved differs in several builds. Poisonous Concoction's Intelligence now overshoots PoB's by 10 from another source, and its energy shield follows.
- **Corpus:** 0 unexpected; 31 moved, 30 toward PoB, 1 away, 11 now equal.
- **Tests:** the workspace tests, the maintainer-run tests on `b-ct`, clippy, and the wasm32 check pass.

**Not compared:** the `scalable` flags against PoB's `modScalability` (a `data-parity` table could; PoB keys its data by description text, not by stat).

## P6-11ay: added damage with a weapon type, avoid shock and ignite

The Abyss jewels' additions (`docs/phase-6/timeless.md`, "Part 4") and quivers' implicits carry these.

**Rows:**
- **64 `HIT_ROWS`:** "# to # Added <Type> Damage with <Weapon> Attacks" (`attack_<minimum|maximum>_added_<type>_damage_with_<weapon>`) for every damage type and weapon the bundle names: axes, bows, claws, daggers, maces (and sceptres), staves, swords, wands.
  - Each is `added(<type>, <bound>)` BASE on the weapon's flag and `Hit`, PoB's `["with <weapon> attacks"] = { flags = bor(ModFlag.<Weapon>, ModFlag.Hit) }` (`ModParser.lua` 905 to 945). P6-11b had the sword and wand ones.
  - Left out: the rage, per-trauma, and "with weapons" variants (other mechanics).
- **`NoEffect` (`ailment-on-self`):** `base_avoid_ignite_%` and `base_avoid_shock_%`. Avoid shock is listed with the shock rows `poe1::damage_taken` checks: it is shock on you, not the enemy's.
- **Test:** `mapping::tests` checks six of the new rows' flags.

**Fixture** `burning-arrow-weapon-added-damage-templar.xml`: Burning Arrow with a Short Bow and custom lines adding damage with each weapon type. The bow lines reach the hit and the others do not.
- 73 fields equal PoB's.
- Six differ as every bow fixture does: the attack rate floored to whole milliseconds (Q11), registered as DIV-0296 to DIV-0301 (`game-truth`).

**Evidence** (`b-ct`, against P6-11ax's run): public and corpus 0 unexpected, no value moved; no build has these stats outside the fixture.

## P6-11az: devotion and the lines that read it

`loom_calc::poe1::devotion`.

**How PoB reads devotion** (`CalcPerform.lua` 490 to 491, `ModParser.lua`):
- `base_devotion` ("+# to Devotion") sums as `Poe1Quantity::Devotion`.
- A reader per 10 Devotion counts `floor(Devotion / 10 + 0.0001)` (PoB's `PerStat`, `div = 10`).
- A reader at 150 Devotion counts 1 when the total is at least 150 (PoB's `StatThreshold`).

**How the engine reads it:** each modelled reader (`devotion::READERS`) means one of the game's other stats.
- The reader maps to `Poe1Quantity::DevotionReader`.
- After every other source, `devotion::devotion` ingests the stat it means, with the reader's value times the count (and a factor), from the reader's source. The existing rows apply.

**Modelled readers:**
- **Per 10 Devotion:**
  - area damage (`area_damage_+%`);
  - elemental damage (`elemental_damage_+%`);
  - all elemental resistances (`base_resist_all_elements_%`);
  - mana cost efficiency;
  - reduced mana cost (`base_mana_cost_-%`, factor -1).
- **At 150 Devotion:**
  - all maximum resistances (`additional_maximum_all_resistances_%`, PoB's elemental and chaos maximums);
  - additional physical damage reduction;
  - physical converted to fire or lightning.

**Still reasons:**
- **The other readers:** totem, brand, and channelled damage, minion speed and accuracy, non-curse aura effect, non-damaging ailment effect, durations on you, shield defences, cold conversion, the threshold exposures, mana as energy shield, minimum charges, Arcane Surge on hit, and ailment immunity on consecrated ground. The stats they would mean are not mapped.
- **"Regenerate # Mana per Second per 10 Devotion":** the game prints its per-minute value to one decimal, so the text does not settle it. Mapped, it would make Essence Drain's fields indeterminate rather than partial.
- **The guards gone:** `base_devotion` is no longer `NoEffect`, so its 30 guards in `NO_EFFECT_DEPENDS_ON` are gone.

**Fixtures** on the Militant Faith fixture's tree (seed 3191, 205 Devotion in PoB):
- `militant-faith-elemental-damage-per-devotion-witch.xml`;
- `militant-faith-mana-per-devotion-witch.xml`: resistances, mana cost, mana cost efficiency;
- `militant-faith-devotion-threshold-witch.xml`: maximum resistances, physical damage reduction.

Every computed field equals PoB's.

**Also fixed:** the `exposure_shock` maintainer-run test lists `base_avoid_shock_%` (P6-11ay's row; shock on you).

**Evidence** (`b-ct`, against P6-11ay's run):
- **Public and corpus:** 0 unexpected, no value moved.
- **Tests:** the workspace tests, every maintainer-run test but the extractor's (which needs a game install and its decoder), and clippy pass.

## P6-11ba: the Abyss jewels' other additions

Each line was checked with PoB's own parser (`tools/oracle/data/pob_parse_lines.lua`).

**`NoEffect` (`on-hit-debuff`):** chances to blind, taunt, or hinder on hit, yours and your minions' (7 stats).
- PoB parses none of these lines, and the enemy is Blinded only by the config's condition.
- The three blind chances are listed in `damage_taken`'s Blind guard test, as P6-11at's unnerve chance is.

**`STATE_ROWS`, each on a condition the build or the config settles:**
- "# to # Added Spell <Type> Damage while Dual Wielding" (10 stats) and "#% increased Spell Damage while Dual Wielding": `KeywordFlag.Spell` or `ModFlag.Spell`, `DualWielding`.
- "#% increased Damage over Time while Dual Wielding", and "... while holding a Shield" (PoB's "while holding a (%w+)", `UsingShield`).
- "+#% Chance to Block Spell Damage if you were Damaged by a Hit Recently" (`BeenHitRecently`).
- "#% increased Evasion Rating while moving" (`Moving`).

**Still reasons:**
- **PoB reads only in part** (text left over, so it applies none of it): "... additional Physical Damage Reduction if you weren't Damaged by a Hit Recently", "... Minion Attack and Cast Speed if you or your Minions have Killed Recently". The game applies them, so they are not `NoEffect`.
- **PoB does not parse:** "Enemies you Wither have #% to all Resistances".
- **A condition the engine does not settle:** the two-handed variants (`UsingTwoHandedWeapon`), movement speed if you haven't taken damage recently, cast speed if a minion has been killed recently.
- **Not modelled yet:** the minion chances to ignite and bleed, and the minion's added chaos damage.

**Fixture** `ice-nova-moving-hit-recently-lines-witch.xml`: moving and damaged by a hit recently by the config. Evasion while moving and spell block apply; the shield and dual wielding lines do not. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11az's run):
- **Public:** 0 unexpected; 6 values toward PoB, none away, 2 now equal (Arakaali's spell block is now PoB's 78).
- **Corpus:** no value moved.

## P6-11bb: charge stats of several kinds

`charges::MULTI`: a stat that is several of the charge module's effects at once, as PoB reads its text.
- **"+# to Maximum Power Charges and Maximum Endurance Charges":** `PowerChargesMax` and `EnduranceChargesMax` BASE.
- **"#% increased Damage per Endurance, Frenzy or Power Charge":** `Damage` INC per `TotalCharges` (`PerStat`). PoB sums `TotalCharges` from the three kinds (`CalcPerform.lua` 1081), so the stat is a per-charge damage effect of each kind.

**Fixture** `ice-nova-total-charges-lines-witch.xml`: every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-9g's run):
- **Public:** 0 unexpected; 80 values toward PoB, none away, 41 now equal.
- **Corpus:** unchanged.

## P6-11bc: rage on attack hit, minion area, avoid shock for all ailments

- **"Gain # Rage on Attack Hit":** PoB's `Condition:CanGainRage`, as on melee hit (`stacks::MODIFIERS`). It leaves `config::DEPENDS_ON`, whose other rage stats PoB reads otherwise.
- **"Minions have #% increased Area of Effect":** `NoEffect` (`area`). The minion's `AreaOfEffect` feeds only radius outputs, as the character's does.
- **"Modifiers to Chance to Avoid being Shocked apply to all Elemental Ailments":** `NoEffect` (`ailment-on-self`). It is listed with the shock rows `damage_taken` and the `exposure_shock` test check (shock on you).

**Evidence** (`b-ct`, against P6-11bb's run):
- **Public:** 0 unexpected; 11 values toward PoB, none away (Elemental Hit's and Kinetic Blast's damage).
- **Corpus:** unchanged.

**Recorded, not done:**
- "Socketed Skill Gems get a #% Cost & Reservation Multiplier" (PoB's `ExtraSkillMod` `SupportManaMultiplier` for the item's gems);
- "#% increased Damage while Leeching" (four config checks, three implying `Leeching`, and PoB's automatic case: leech not removed at full life).

## P6-11bd: bleeding's DoT multiplier, stun, minion aggression

- **"+#% to Damage over Time Multiplier for Bleeding":** `DotMultiplier` BASE with `KeywordFlag.Bleed` (`HIT_ROWS`, as poison's).
- **`NoEffect`:**
  - stun duration with two-handed melee weapons, and Occultist's "Cannot Be Stunned while you have Energy Shield" (`stun`);
  - "Minions are Aggressive" (`minion-behaviour`: PoB parses none of it).

**Evidence** (`b-ct`, against P6-11bc's run):
- **Public:** 0 unexpected; 4 values toward PoB (Boneshatter's and Lacerate's bleed), none away.
- **Corpus:** unchanged.

**Recorded, not done:** damage with hits against chilled enemies (the enemy's `Chilled` condition is no engine state).

## P6-11be: reflect, a minion recovery on death, explode, hexproof

**`NoEffect`, each checked against PoB's parse:**
- reflected lightning damage (unparsed);
- "Minions Recover #% of Life on Minion Death" (unparsed);
- the cursed enemies' explosion (PoB's `ExplodeMod`, read only by its own `EnemyExplode` skill, which no modelled build includes);
- "Your Hexes can affect Hexproof Enemies" (no input or modelled modifier makes the enemy hexproof).

**Left as reasons:** Crush on full-life enemies and aggravated bleeding. PoB parses neither, but in the game they act on the enemy's physical damage reduction and the bleed.

**Evidence:** public and corpus 0 unexpected, no value moved (those builds have other reasons).

## P6-11bf: damage with hits against chilled enemies

"#% increased Damage with Hits against Chilled Enemies" is PoB's `Damage` INC with `KeywordFlag.Hit` and the enemy's `Chilled` condition. It maps to `Poe1Quantity::DamageVsChilled`.
- `damage_taken::enemy_states` turns it into hit `Damage` INC when the enemy is chilled (`damage_taken::chilled`: the config's check, or a skill's guaranteed chill, `CalcPerform.lua` 1396).
- This is how it already applies "Enemies Ignited or Chilled by you have #% to Elemental Resistances".

**Fixture** `ice-nova-damage-vs-chilled-witch.xml`: every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11be's run):
- **Public:** 0 unexpected; 7 values toward PoB, none away.
- **Corpus:** unchanged.

## P6-11bg: the frenzy charges' maximum from the power charges'

"Your Maximum Frenzy Charges is equal to your Maximum Power Charges" (Badge of the Brotherhood) is `ChargeValue::MaximumFromPower`.
- With it, `charges::charges` computes the frenzy charges' maximum as the power charges' (the power constant plus the power maximum modifiers), as PoB's `FrenzyChargesMax` override (`CalcPerform.lua` 955 to 960).
- **Found:** PoB fails the calculation when the line comes from the config's custom modifiers. Its `modDB.mods[...][1].source` has no such entry there (the fixture's first try, `ORACLE_CALCULATION`). The fixture carries it on the amulet.

**Fixture** `ice-nova-frenzy-max-from-power-witch.xml`: every computed field equals PoB's.

**Evidence:**
- **Public:** 0 unexpected; 8 values toward PoB (a Cospris build's speed and damage), none away.
- **Corpus:** unchanged.

## P6-11bh: the minion's added damage of every type

"Minions deal # to # additional <Type> Damage" for physical, fire, lightning, and chaos, as P6-11as did for cold (`minions::STATS`, `MinionValue::AddedDamage`: PoB's `MinionModifier` `<Type>Min` and `<Type>Max` BASE, no flags). The minion test checks each row against its game text.

**Also fixed:** the `keystones_chill` maintainer test lists P6-11bf's row (`damage_+%_vs_chilled_enemies`).

**Evidence:**
- **Public:** 0 unexpected; 1 value toward PoB (Animate Guardian's full DPS), none away.
- **Corpus:** unchanged.

## P6-11bi: the socketed skills' cost and reservation multiplier

"Socketed Skill Gems get a #% Cost & Reservation Multiplier" is PoB's `ExtraSkillMod` `SupportManaMultiplier` MORE of the value less 100 for the skills socketed in the item.
- It maps, on an item in a gem slot, to `Poe1Quantity::SocketedCostMultiplier(slot)` (`gem_levels::InSlot`, as P6-11q's socketed gem levels).
- `GemLevels` collects it, and each `MainSkill` and `GroupSkill` carries the multipliers of its group's slot (`cost_mores`).
- `gems::support_multiplier` multiplies them in with the supports' before PoB's two floors, for the cost (`costs`) and the reservation (`gems`).

**Fixture** `ice-nova-socketed-cost-multiplier-witch.xml`: Ice Nova and Precision in the armour. Every computed field equals PoB's, the mana cost and unreserved mana included.

**Evidence** (`b-ct`, against P6-11bh's run):
- **Public:** 0 unexpected; Flicker Strike's unreserved mana toward PoB.
- **Away:** two values (Elemental Hit's and Ethereal Knives' unreserved mana). Their unreserved mana was already far above PoB's, with many reasons on the field.
- **Corpus:** unchanged.

## P6-11bj: life and energy shield recovered on block while wielding a staff

"Recover #% of Life when you Block Attack Damage while wielding a Staff" and "Recover #% of Energy Shield when you Block Spell Damage while wielding a Staff" are PoB's `LifeOnBlock` and `EnergyShieldOnSpellBlock` BASE with `PercentStat` (`Life`, `EnergyShield`) and the condition `UsingStaff`.
- The life line is a `STATE_ROWS` row: `BlockGain::LifePercent` (P6-11ak's) under `states::USING_STAFF`.
- The energy shield line needs a new gain, `BlockGain::EnergyShieldSpellPercent`. `enemy_hit` reads it as a percent of maximum energy shield, rounded up as the other percent gains, and adds it to the flat energy shield on spell block.

**Fixture** `ehp-staff-recover-on-block-ice-nova-witch.xml`: a Quarterstaff, block chance, and both lines. Every computed field equals PoB's (77 match).

**Evidence** (`b-ct`, against P6-11bi's run):
- **Public:** 0 unexpected; 6 values toward PoB (two staff builds' effective health pool and hits to die), none away.
- **Corpus:** unchanged.

## P6-11bk: totem, trap, and mine skills; totem and mine damage

PoB gives a skill `KeywordFlag.Totem` when it is a totem, else `KeywordFlag.Trap` when it is a trap, else `KeywordFlag.Mine` when it is a mine (`CalcActiveSkill.lua` 415 to 420). Its skill flags come from the granted effect's base flags and the supports' added flags.
- The engine reads them from the skill's types, the supports' added types included: `SummonsTotem`, `Trapped`, `RemoteMined` (`skill::keyword_flags`, `Flags::KEYWORD_TOTEM`, `KEYWORD_TRAP`, `KEYWORD_MINE`). A test checks the three names against the real bundle's skill types.
- An ailment keeps them, as PoB's ailment config keeps every keyword but `Hit`.
- "#% increased Totem Damage" and "#% increased Mine Damage" are `HIT_ROWS` rows: `Damage` INC with the keyword.
- Totem, trap, and mine skills themselves are not modelled yet (their supports and their own data are reasons). The rows matter now for every other skill, which they do not reach, as in PoB.

**Fixture** `ice-nova-totem-mine-damage-lines-witch.xml`: Ice Nova with both lines and a spell damage line. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11bj's run):
- **Public:** 0 unexpected; 15 values toward PoB, none away: the two totem builds' hits (Holy Flame Totem's and Kinetic Fusillade's through a ballista), their ailments and DPS. Their totem skills are still partial for other reasons.
- **Corpus:** unchanged.

## P6-11bl: energy shield from the equipped helmet; counting as having maximum charges

- "#% increased Energy Shield from Equipped Helmet" is PoB's `EnergyShield` INC with `SlotName` Helmet: a `ROWS` row on `SlotIncreased(Helmet, EnergyShield)`, as P6-11k's shield lines.
- "Count as having maximum number of <Kind> Charges" (power, frenzy, endurance) is PoB's `HaveMaximum<Kind>Charges` flag (`CalcPerform.lua` 1072 to 1080). The count is the maximum whatever the config set says. In `charges`, a new `ChargeValue::CountAsMaximum` sets the count to the maximum, the line as the source, with or without the `use` input.

**Fixtures:**
- `ice-nova-count-as-max-charges-witch.xml`: no charges in use, both lines, one more maximum power charge.
- `ice-nova-helmet-energy-shield-witch.xml`: a Hubris Circlet and a Vaal Regalia, the helmet's line.

Every computed field of both equals PoB's.

**Evidence** (`b-ct`, against P6-11bk's run):
- **Public:** 0 unexpected; 25 values toward PoB (a build's helmet line: its energy shield, recharge, regeneration, and hits to die).
- **Away:** one value, a trap build's energy shield leech rate. It was already 549.7 against PoB's 0, a partial field, and rises with the energy shield the line now adds.
- **Corpus:** unchanged.

## P6-11bm: the other rage lines PoB reads as "can gain rage"

PoB parses seven more rage lines as `Condition:CanGainRage`, as P6-11bc's "Gain # Rage on Attack Hit":
- on hit with axes;
- on hit with axes or swords;
- on melee weapon hit (the tinctured weapons stat);
- after spending 200 mana;
- when hit;
- on using a life flask;
- on losing Temporal Chains.

They join `stacks::MODIFIERS` (`StackValue::CanGain`) and leave `config::DEPENDS_ON`. The rest of that list stays: PoB reads those lines as nothing, or not at all.

**Fixture** `smite-rage-on-hit-with-axes-templar.xml`: Smite with a Siege Axe, the axes line, the config's rage at 20. The rage's damage equals PoB's. The differences are the weapon's attack rate floored to whole milliseconds, Q11's game truth (`DIV-0302` to `DIV-0307`).

**Evidence** (`b-ct`, against P6-11bl's run):
- **Public:** 0 unexpected; 4 values toward PoB (a Gladiator's bleed with rage), none away.
- **Corpus:** unchanged.

## P6-11bn: nearby enemies' resistances

"Nearby Enemies have #% to <Type> Resistance" (fire, cold, lightning, chaos) and "Nearby Enemies have #% to all Resistances" are PoB's `EnemyModifier` `<Type>Resist` BASE; all resistances are `ElementalResist` and `ChaosResist`.
- They are `ROWS` rows on `Poe1Quantity::EnemyResistance`, the quantity curses and exposure use.
- The hit and damage over time add it to the enemy's base resistance before its maximum.
- Exposure picks its strongest from its own candidates, so these lines add beside it, as a curse's do.

**Fixture** `ice-nova-nearby-enemy-resistances-witch.xml`: the cold and the all-resistances lines. Every computed field equals PoB's; the hit's enemy resistance sums both.

**Evidence** (`b-ct`, against P6-11bm's run):
- **Public:** 0 unexpected; 6 values toward PoB (a build's chaos damage over time and DPS), none away.
- **Corpus:** unchanged.

## P6-11bo: the ends of the damage range (Volatility, Ryslatha's Clutches)

PoB's `calcDamage` (`CalcOffence.lua` 113 to 138) rolls each end of a type's range as `round(((base x inc x more) x genericMore + converted) x more<Type> x inc<Type>)`:
- `genericMore` is `MinDamage` or `MaxDamage` MORE.
- `more<Type>` and `inc<Type>` are `Min<Type>Damage` or `Max<Type>Damage`.

The engine adds `Poe1Quantity::BoundDamage(type, bound)`, with no type for the generic one. `hit::calc_damage` reads it in that order. It scans for the quantity only for a type with base damage, and reads it only when the store has one, so other hits keep their graphs.
- Volatility's two stats (`MinDamage` and `MaxDamage` MORE, `SkillType` Attack) are `HIT_ROWS` rows with no type.
- "#% more Maximum Physical Attack Damage" and Ryslatha's Clutches' maximum and minimum lines have the physical type.
- The skill mapping reads the quantity as a hit quantity (`is_hit_quantity`), so a support's stats apply.

**Fixture** `smite-volatility-min-max-physical-templar.xml`: Smite with Volatility, a Siege Axe, and both physical lines. The damage equals PoB's. The differences are the weapon's attack rate floored to whole milliseconds, Q11's game truth (`DIV-0308` to `DIV-0313`).

**Evidence** (`b-ct`, against P6-11bn's run):
- **Public:** 0 unexpected; 12 values toward PoB in three attack builds, none away.
- **Corpus:** 0 unexpected; 2 values toward PoB in one build, none away.

## P6-11bp: gain as extra damage

"Gain #% of <Type> Damage as Extra <Type> Damage" is PoB's `<From>DamageGainAs<To>` BASE.
- "Elemental" is `ElementalDamageGainAs<To>`, which PoB sums into each elemental type's gain (`CalcOffence.lua` 1893).
- The eleven plain stats are `ROWS` rows on `Poe1Quantity::GainAs`, which the hit's conversion table already read for the heralds and Hatred.
- PoB's `SkillStatMap.lua` (925 to 945) maps seven of them for a skill's own stats too, so the skill mapping now reads `GainAs` as a hit quantity, as it reads the conversions.

**Fixture** `smite-gain-as-extra-templar.xml`: Smite with a Siege Axe, physical gained as cold, elemental gained as chaos. The damage equals PoB's. The differences are Q11's floored attack rate (`DIV-0314` to `DIV-0319`).

**Evidence** (`b-ct`, against P6-11bo's run):
- **Public:** 0 unexpected; 14 values toward PoB in three builds, none away.
- **Corpus:** unchanged.

## P6-11bq: life regeneration per endurance charge

"Regenerate # Life per second per Endurance Charge" is PoB's `LifeRegen` BASE per `Multiplier:EnduranceCharge`, the stat per minute.
- A new `PerCharge::LifeRegenPerMinute` adds `Regen(Life)` BASE: the stat's value over 60 (`PerCharge::scale`), times the count.
- The regeneration phase reads it among life's quantities (`regen::reads`), as P6-11t's per-charge regeneration. An unsettled value is therefore life regeneration's reason.

**No fixture:** the game prints `per_minute_to_per_second` stats to one decimal, so a line read from text leaves the value unsettled (`docs/phase-6/recovery.md`, "A display limit"). A typed line and an item's line are both text. A test pins the arithmetic: `life_regeneration_per_endurance_charge_should_follow_the_count` (6000 per minute and three charges: 300 per second; 0 without the charges). The test data gains three endurance charges and their reductions.

While writing this step, a fault showed: without the `reads` entry, the unsettled value's reason reached no field, and life regeneration showed a complete 0. The entry fixes it.

**Evidence** (`b-ct`, against P6-11bp's run):
- **Public:** 0 unexpected; no value moved. Two builds' life regeneration went from partial (the stat unmapped) to indeterminate: their item's line is text, so its value is unsettled.
- **Corpus:** unchanged.

## P6-11br: five NoEffect rows

Five stats that change no slice field are `NoEffect` rows. PoB parses none of them:
- stun immunity while using a chaos skill (`stun`);
- gem experience (`gem-experience`);
- projectiles fired in random directions (`targeting`);
- magic utility flask effects that cannot be removed, and a flask that gains no charges during its effect (`flask`). The effects apply while the flask is on, as the config sets. The charge row is listed in the charges guard (`NO_EFFECT_CHARGE_ROWS`).

"Magic Utility Flasks cannot be Used" stays a reason. PoB ignores it, but the game does not, and the engine applies a magic utility flask's effects.

**Evidence** (`b-ct`, against P6-11bq's run): public and corpus 0 unexpected; no value or state moved (those builds' fields have other reasons).

## P6-11bu: life regeneration per fortification

"Regenerate #% of Life per second per Fortification" is PoB's `LifeRegenPercent` BASE per `PerStat` `FortificationStacks`.
- **The stacks** (`CalcPerform.lua` 632 to 653): while the config's `buffFortification` is on, `min(overrideFortification or the maximum, the maximum)`. The maximum is `base_max_fortification` plus `MaximumFortification` BASE.
- **Shared:** the enemy's hit (P6-10b part 12) already computed them. That code is now `enemy_hit::fortification_stacks`, shared with regeneration.
- **The new term:** `Poe1Quantity::RegenPerFortification` (a `ROWS` row) is the stat per minute. `regen` reads it per second times the stacks, and nothing when not fortified.
- **Guards:** the stat leaves `mapping::REGEN_REASONS`. The fortification guard (`config::tests::no_mapped_stat_should_read_fortification`) names it beside the maximum.

**No fixture:** the stat's text handler prints one decimal, so a typed line is unsettled. The public builds take it from a passive (node 13922), whose value is exact. A test pins the term: `life_regeneration_per_fortification_should_follow_the_stacks` (2% of life at 20 stacks, nothing when not fortified).

**Evidence** (`b-ct`, against P6-11bt's run):
- **Public:** 0 unexpected; 3 values toward PoB, none away. The three builds with the passive's life regeneration; one (an Inquisitor) now equals PoB's 553 and is complete.
- **Corpus:** unchanged.

## P6-11bv: while Leeching

PoB's `Leeching` condition (`ConfigOptions.lua` 1068 to 1081):
- The config's "Are you Leeching?" sets it. So does each of the life, energy shield, and mana leeching checks, which also set their own condition (`implyCond`).
- PoB also sets it automatically for "Life Leech effects are not removed at Full Life" (unless Ghost Reaver) and the energy shield equivalent (`CalcPerform.lua` 873 to 880).

What the engine does:
- `conditions::CHECKS` gains the four checks, each setting `conditions::LEECHING` and its own condition. `conditionLeeching` leaves `NAMED_ONLY`.
- The four stats that set it automatically join `config::DEPENDS_ON`, which keeps them reasons: the engine settles the condition from the config alone.
- Seven `STATE_ROWS` read it as PoB reads their text: damage, attack damage and attack speed (`ModFlag.Attack`), attack and cast speed, evasion, and armour and evasion while Leeching, and attack and cast speed while Leeching Energy Shield.
- Stun and bleeding immunity and damage taken while Leeching stay reasons.

**Fixture** `ice-nova-leeching-lines-witch.xml`: the energy shield check, with damage, armour and evasion, and attack and cast speed lines. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11bu's run):
- **Public:** 0 unexpected; 18 values toward PoB in a Deadeye's Elemental Hit, 6 now equal, none away.
- **Corpus:** unchanged.

## P6-11bw: spell damage per 100 maximum mana

"#% increased Spell Damage per 100 Maximum Mana, up to 40%":
- **PoB's reading:** `Damage` INC with `ModFlag.Spell` and a `PerStat` tag on `Mana`, `div` 100, `limit` 40, `limitTotal` (`ModParser.lua` 1487). The tag reads `min(floor(mana / 100 + 0.0001) x value, limit)`.
- **The cap:** the game's text says 40%, though the stat ID keeps an older 60. PoB reads the text, and the engine takes 40.
- **The design:** this is the tasks file's "stats read per a computed value" item, solved as `phases::per_5` solves block. A computed term in the hit uses the maximum mana field's node, so the graph keeps the provenance.
- **The engine:** `Poe1Quantity::SpellDamagePer100Mana` holds the value per step (a `HIT_ROWS` row with `Flags::SPELL`). `hit::spell_damage_per_mana` computes the term for a context with the spell flag. `calc_damage` and `damage_modifiers` (damage over time's) add it to the damage increase's one sum, as the melee physical bonus is added. A context without the spell flag, or a store without the stat, keeps its graph.

**Fixtures:**
- `ice-nova-spell-damage-per-mana-below-witch.xml`: +500 maximum mana, 2% per 100, 20%.
- `ice-nova-spell-damage-per-mana-capped-witch.xml`: +3000, capped at 40%.

Every computed field of both equals PoB's.

**Evidence** (`b-ct`, against P6-11bv's run):
- **Public:** 0 unexpected; 9 values toward PoB (two spell builds' damage and DPS), none away.
- **Corpus:** unchanged.

## P6-11bx: elemental resistances treated as inverted

"Hits have #% chance to treat Enemy Monster Elemental Resistance values as inverted" is PoB's `HitsInvertEleResChance` CHANCE, the stat over 100 (`SkillStatMap.lua` 821).

PoB's hit (`CalcOffence.lua` 3478 to 3500), for an elemental type: the chance `c` is clamped to [0, 1], and the effective resistance is `(resist - pen) x (1 - c) + (-resist - pen) x c`.

What the engine does:
- `Poe1Quantity::InvertElementalResistChance` is a `HIT_ROWS` row (divisor 100), read by the skill mapping as a hit quantity too.
- `hit::effective_resistance` takes that form when the store has the chance. Other hits keep their graph.
- Chaos's own chance (`HitsInvertChaosResChance`) and PoB's `CannotElePenIgnore` branch have no source in the engine.

**Fixture** `ice-nova-inverted-resistance-witch.xml`: a 25% chance and 10% cold penetration against the default Pinnacle boss. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11bw's run):
- **Public:** 0 unexpected; 9 values toward PoB, none away.
- **Corpus:** 0 unexpected; 11 values toward PoB, none away.

## P6-11by: the body armour's defences with hybrid helmet, gloves, and boots

"#% increased Armour and Energy Shield from Equipped Body Armour if Equipped Helmet, Gloves and Boots all have Armour and Energy Shield":
- **PoB's reading:** `Body ArmourESAndArmour` INC with six `StatThreshold` tags: `ArmourOn<slot>` and `EnergyShieldOn<slot>` at least 1 for the three pieces (`ModParser.lua` 2478). Its defence code reads the name in the body armour's armour and energy shield rows (`CalcDefence.lua` 900, 908).
- **The engine:** `Poe1Quantity::BodyArmourIfHybridPieces` (a `ROWS` row) joins the body armour's own increase in `phases::primary_defence` for armour and energy shield. It applies only when `hybrid_pieces` finds the helmet, gloves, and boots equipped, each with armour and energy shield of at least 1 (each piece's own value, as PoB's `<Defence>On<slot>`).

**No fixture:** the game prints the line on two lines, so a typed line does not resolve, and PoB parses only the first line of an item's text. The public builds take it from a passive.

**Evidence** (`b-ct`, against P6-11bx's run):
- **Public:** 0 unexpected; 40 values toward PoB, 1 now equal (a Penance Brand build's armour, 25,366).
- **Away:** 4 values, all on fields with other reasons. One build's energy shield recharge rose with its energy shield, which is still below PoB's. Three of another build's defence estimate values moved by under 0.01%.
- **Corpus:** 0 unexpected; 43 toward PoB, 19 now equal, 3 away (the same estimate, by under 0.01%).

## P6-11bz: a flask's enemy damage taken on consecrated ground and while ignited

Two flask lines are PoB's `EnemyModifier` `DamageTaken` INC:
- "Consecrated Ground created during Effect applies #% increased Damage taken to Enemies", with the enemy's `OnConsecratedGround`;
- "Enemies Ignited by you during Effect take #% increased Damage", with the enemy's `Ignited`.

The consecrated ground line is not scaled by the ground's effect, unlike the global "Consecrated Ground you create applies ..." (P3-8n). So it is a quantity of its own and not an alias.

What the engine does:
- `Poe1Quantity::ConsecratedEnemyDamageTaken` and `IgnitedEnemyDamageTaken` are `ROWS` rows.
- `flasks::FLASK_ROWS` lets the flask mapping read them, as other local lines are reasons.
- `damage_taken::enemy_states` adds each as the enemy's damage taken under the config's check (`conditionEnemyOnConsecratedGround`, `conditionEnemyIgnited`). The damage-taken guard lists both.
- "Consecrated Ground created by this Flask has Tripled Radius" is `NoEffect` (`area`; PoB does not parse it).

**Fixture** `ice-nova-flask-enemy-damage-taken-witch.xml`: a magic Sulphur Flask and Ruby Flask, both active, with both checks on. Every computed field equals PoB's; the hit is 1.2 times the same build without the lines.

**Evidence** (`b-ct`, against P6-11by's run):
- **Public:** 0 unexpected; 12 values toward PoB, none away.
- **Unsettled rolls:** two builds carry the consecrated ground line on a flask whose roll the item does not settle. Their 141 fields went from partial (the stat unmapped) to indeterminate (its unsettled value), as P6-11bq's.
- **Corpus:** unchanged.

## P6-11ca: Pulverise, attacks' additional crit chance, a flask's crit on consecrated ground

- **Pulverise** (`Data/Skills/sup_str.lua` 4289 to 4294): `Damage` MORE with `ModFlag.Melee` and `ModFlag.Area`, and `Speed` MORE with `ModFlag.Attack` (its constant -15). Both are `HIT_ROWS` rows.
- **"Attacks have +#% to Critical Strike Chance":** PoB's parser gives `CritChance` BASE with `ModFlag.Attack`. The stat is in hundredths (`divide_by_one_hundred`), so the row's divisor is 100.
- **A flask's "+#% to Critical Strike Chance against Enemies on Consecrated Ground during Effect":** `CritChance` BASE with the enemy's `OnConsecratedGround`, also in hundredths.
  - `Poe1Quantity::ConsecratedCritChance` (scale 0.01) is in `flasks::FLASK_ROWS`.
  - `damage_taken::enemy_states` adds it to the crit chance under the config's consecrated ground check.

**Fixtures:**
- `smite-pulverise-attack-crit-templar.xml`: the damage and crit equal PoB's. The differences are Q11's floored attack rate (`DIV-0327` to `DIV-0332`).
- `ice-nova-flask-consecrated-crit-witch.xml`: every computed field equals PoB's.

**data-parity:** `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11bz's run):
- **Public:** 0 unexpected; 16 values toward PoB.
- **Away:** 4 values, two builds' speed (and a cost and leech rate that follow it), now with Pulverise's 15% less. A check with PoB itself confirms PoB applies it: the Cyclone build at 12.59 without Pulverise, 10.81 with, a factor of 0.858; the engine's is 0.858 too. The engine's speed before was near PoB's by chance: it lacked Pulverise, and the field misses other sources (a partial field).
- **Corpus:** 0 unexpected; 14 toward PoB, none away.

## P6-11cb: energy shield increased by spell block chance or chaos resistance

"Maximum Energy Shield is increased by Chance to Block Spell Damage" and "... by Chaos Resistance":
- **PoB's reading:** flags (`ModParser.lua` 4805, 4806). PoB's defence code adds an `EnergyShield` INC of the final spell block chance or chaos resistance (`CalcDefence.lua` 812 to 824), after computing both.
- **The engine:** `Poe1Quantity::EnergyShieldFromSpellBlock` and `EnergyShieldFromChaosResistance` (`Flag` rows). `phases::primary_defences` adds the field's value as one more term of energy shield's increase, beside the intelligence bonus. The block and resistance phases run first, and energy shield depends on the field.
- **Negative resistance:** a negative chaos resistance lowers energy shield, as in PoB.

**Fixtures:**
- `ice-nova-energy-shield-from-spell-block-witch.xml`: 1,000 energy shield, 30% spell block, 1,330.
- `ice-nova-energy-shield-from-chaos-resistance-witch.xml`: chaos resistance -20, 830.

Every computed field of both equals PoB's.

**Evidence** (`b-ct`, against P6-11ca's run):
- **Public:** 0 unexpected; 25 values toward PoB (two Occultists' energy shield and the values that follow it), none away.
- **Corpus:** unchanged.

## P6-11cc: block if hit recently; a weapon's poison chance

- **"+#% Chance to Block Attack Damage if you were Damaged by a Hit Recently":** PoB's `BlockChance` BASE with `BeenHitRecently` (its parser), a `STATE_ROWS` row on the settled check.
- **A weapon's "#% chance to Poison on Hit"** (`local_poison_on_hit_%`): PoB parses it as the global `PoisonChance` BASE, with no hand condition. A `WEAPON_ROWS` row reads it the same way.
  - For one weapon, the game's local chance and PoB's global one agree.
  - For a dual wielder with one such weapon, PoB gives both hands the chance and the engine does too. The difference from the game there is PoB's, kept as the oracle has it.

**Fixtures:**
- `ehp-block-if-hit-recently-ice-nova-witch.xml`: every computed field equals PoB's.
- `smite-weapon-poison-chance-templar.xml`: the poison equals PoB's apart from Q11's floored attack rate (`DIV-0333` to `DIV-0339`).

**Evidence** (`b-ct`, against P6-11cb's run):
- **Public:** 0 unexpected; 13 values toward PoB, 2 now equal, none away.
- **Corpus:** 0 unexpected; 2 toward PoB. One build's poison DPS is now computed, indeterminate for the build's older tree version.

## P6-11cd: the spectre limit

"+# to maximum number of Spectres" is PoB's `ActiveSpectreLimit`. As the zombie and skeleton limits (P6-11m), it is a count no slice field reads, so it is a `NoEffect` row (`minion-count`).

**Evidence** (`b-ct`, against P6-11cc's run): public and corpus 0 unexpected; no value or state moved (those builds' fields have other reasons).

## P6-11cf: projectile damage against nearby enemies; trap and totem lines

Each as PoB's parser reads the game's text:
- "Projectiles deal #% increased Damage with Hits against nearby Enemies": `Damage` INC with `ModFlag.Projectile` and `KeywordFlag.Hit`. PoB keeps no condition for "nearby".
- "#% increased Trap Damage": `Damage` INC, `KeywordFlag.Trap`.
- "+#% to Critical Strike Multiplier with Totem Skills": `CritMultiplier` BASE, `KeywordFlag.Totem`.
- "Attacks used by Totems have #% increased Attack Speed": `Speed` INC, `ModFlag.Attack` and `KeywordFlag.Totem`.
- "Damage cannot be Reflected" is `NoEffect` (`reflect`).

All are `HIT_ROWS` rows on P6-11bk's keywords.

**Fixture** `burning-arrow-projectile-trap-totem-lines-templar.xml`: the projectile line raises the hit; the trap and totem lines do not reach it. The differences are Q11's floored attack rate (`DIV-0340` to `DIV-0345`).

**Evidence** (`b-ct`, against P6-11ce's run):
- **Public:** 0 unexpected; 17 values toward PoB, 1 now equal.
- **Away:** 1 value, the Kinetic Fusillade ballista's speed (4.2 to 4.36, PoB 2.7), with the totem attack speed line PoB also applies. The engine does not model totems yet, so that build's totem speed is a partial field far from PoB's either way.
- **Corpus:** unchanged.

## P6-11cg: physical damage reduction per hit taken recently

"#% additional Physical Damage Reduction for each Hit you've taken Recently up to a maximum of 5%" is PoB's `PhysicalDamageReduction` BASE per `Multiplier:BeenHitRecently`, `limit` 5, `limitTotal`.
- **The count** is the config's `multiplierBeenHitRecently`. The "hit recently" check alone sets no count.
- **The engine:** a new stack kind `stacks::StackKind::HitsTakenRecently` reads that input (with `conditions::HIT_RECENTLY_COUNT_INPUT`, which also settles the condition). `PerStack::PhysicalReductionUpTo5` caps the total at 5.
- **Reasons:** the stat leaves `mapping::REDUCTION_REASONS`, now empty.

**Fixture** `ehp-reduction-per-hit-taken-ice-nova-witch.xml`: 3 hits and 2% each, 6% capped at 5%. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cf's run):
- **Public:** 0 unexpected; 8 values toward PoB, none away. 32 fields went from partial (the field's reason) to complete.
- **Corpus:** unchanged.

## P6-11ch: Arcane Surge's effect and its more spell damage

PoB's Arcane Surge (`CalcPerform.lua` 686 to 699):
- **The effect:** `1 + (ArcaneSurgeEffect + BuffEffectOnSelf) / 100`, plus a flask share.
- **What it scales:** the mana regeneration (`ArcaneSurgeManaRegen` or 30) and the cast speed (`ArcaneSurgeCastSpeed` or 20).
- **The more spell damage:** the largest `ArcaneSurgeDamage` (the Hierophant's "Arcane Surge also grants #% more Spell Damage to you") times the effect.

What the engine does:
- "#% increased Effect of Arcane Surge on you" is `Poe1Quantity::ArcaneSurgeEffect`, and the Hierophant's line is `ArcaneSurgeDamage`, both `ROWS` rows.
- `buffs::arcane_surge` scales the regeneration and the cast speed by the effect, and adds the more spell damage.
- The generic buff effect on self and the flask share stay reasons (`ARCANE_SURGE_DEPENDS_ON`, which the effect stat leaves).

**Fixture** `ice-nova-arcane-surge-effect-witch.xml`: 30% effect and 10% more spell damage. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cg's run):
- **Public:** 0 unexpected; 9 values toward PoB (a Hierophant's Shock Nova), none away.
- **Corpus:** unchanged.

## P6-11cm: minion damage if you've used a minion skill recently

**PoB** reads "Minions deal #% increased Damage if you've used a Minion Skill Recently" as a `MinionModifier` LIST of `Damage` INC, with the condition `UsedMinionSkillRecently` on the outer mod (`ModParser.lua` 4381). Two things set the condition:
- **The config's check** `conditionUsedMinionSkillRecently` (`ConfigOptions.lua` 1526). Its implied `UsedSkillRecently` matters only to the UI (`implyCond`, `ConfigTab.lua` 193).
- **The main skill** (`doActorMisc`, `CalcPerform.lua` 240 to 251): a skill with PoB's `minion` flag and not `permanentMinion`, not triggered, not a trap, mine, or totem.

PoB's minion gets the line in both cases. Summon Raging Spirit without the check gets it (fixture below), although `addMinionModifiers` (1918) runs before `doActorMisc` (3425) in the source.

**The engine:**
- `poe1::conditions` settles `UsedMinionSkillRecently` from the check, or from the main skill (`conditions::used_minion_skill`: PoB's flags from `pob_skills`, the trigger from `triggers::kind`, and the `Trapped`, `RemoteMined`, `SummonsTotem` types). The check leaves `config::NAMED_ONLY`.
- The stat is a state row on the minion's damage (`mapping::STATE_ROWS`).
- `minion_actor::minion_modifiers` now evaluates a minion modifier whose only conditions are the character's states against those settled states: it applies, does not, or (unsettled) stays a reason as before. A modifier with flag conditions is a reason as before.
- Spiritual Aid's redirect (`minions::minions`) keeps the condition, which the character's store evaluates.

**Fixtures:**
- `minion-raise-zombie-used-minion-skill-witch.xml`: Raise Zombie, the line at 40%, and the check on.
- `minion-raise-zombie-used-minion-skill-off-witch.xml`: the same with the check off; Raise Zombie is a permanent minion, so nothing.
- `minion-summon-raging-spirit-used-minion-skill-witch.xml`: Summon Raging Spirit with the check off; the main skill sets the condition.

Every computed field of the three equals PoB's.

**Evidence** (`b-ct`, against P6-11cl's run):
- **Public:** 0 unexpected. Match went from 19,673 to 19,932, the three fixtures' fields; 2 values moved toward PoB (two community builds' full DPS), none away.
- **Corpus:** 0 unexpected; 1 value toward PoB, none away.
- **Tests:** the workspace tests and clippy pass. The app's config test used this check as an input with no effect; it now uses `conditionCastSpellRecently`.

## P6-11cn: armour and damage reduction per endurance charge

**PoB** reads each line with the tag `{ type = "Multiplier", var = "EnduranceCharge" }`:
- "#% increased Armour per Endurance Charge": `Armour` INC.
- "#% additional Physical Damage Reduction per Endurance Charge": `PhysicalDamageReduction` BASE.
- "#% additional Elemental Damage Reduction per Endurance Charge": `ElementalDamageReduction` BASE.

**The engine:** `charges::PerCharge` gains `Armour`, `PhysicalDamageReduction`, and `ElementalDamageReduction`, each a `charges::MODIFIERS` row on endurance charges. `charges` applies each times the settled count, as the other per-charge values.

**Fixture** `ice-nova-endurance-reduction-lines-witch.xml`: endurance charges in use, +500 armour, and the three lines. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cm's run):
- **Public:** 0 unexpected; 22 values toward PoB (a Juggernaut's EHP, hits taken, and damage taken multipliers among them), none away.
- **Corpus:** 0 unexpected; 48 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass. `loom-persistence`'s `an_empty_build_should_round_trip_through_a_file` failed once in the workspace run and passed on its own; it is recorded for review.

## P6-11co: projectile attack skills' critical strikes

**PoB** reads "Projectile Attack Skills have #% increased Critical Strike Chance" as `CritChance` INC and "Projectile Attack Skills have +#% to Critical Strike Multiplier" as `CritMultiplier` BASE. Each has the tag `{ type = "SkillType", skillType = SkillType.RangedAttack }`, which matches the skill's types after its supports added theirs.

**The engine:**
- `Flags::SKILL_RANGED_ATTACK` (bit 50): `skill::keyword_flags` sets it from the `RangedAttack` type (`skill::RANGED_ATTACK_TYPE`). The ailments keep it, as PoB's ailment config keeps `skillTypes`.
- The two stats are `HIT_ROWS` rows with that flag.
- **Test:** `keyword_flags` of an attack with `RangedAttack`.

**Fixture** `burning-arrow-projectile-attack-crit-templar.xml`: Burning Arrow with a Short Bow and both lines (40% and +30%). Crit chance and multiplier equal PoB's. The attack rate's Q11 rounding is registered as DIV-0360 to DIV-0365 (`game-truth`).

**Evidence** (`b-ct`, against P6-11cn's run):
- **Public:** 0 unexpected; 6 values toward PoB (a Deadeye's Elemental Hit: crit chance, multiplier, damage), none away.
- **Corpus:** 0 unexpected; 18 values toward PoB and 1 now equal, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11cp: ignoring the armour movement penalties

**PoB** gives an armour base with a `MovementPenalty` (body armours, shields) `MovementSpeed` INC of minus the penalty. The mod carries the negated condition `IgnoreMovementPenalties` (`Item.lua` 2362). "Ignore all Movement Penalties from Armour" is the flag `Condition:IgnoreMovementPenalties` (`ModParser.lua` 4814).

**The engine:**
- The stat is a `ROWS` flag on the new `Poe1Quantity::IgnoreMovementPenalties`, and leaves `MOVEMENT_REASONS`.
- `collect` gives the base's penalty the state `IgnoreMovementPenalties` = false.
- `poe1::movement` queries the flag, then queries movement speed with the state settled from it. Only the movement phase reads `MovementSpeed`.
- **Tests:** the field-marker test now uses another movement reason stat (per summoned phantasm).

**Goldens:** `synthetic-poe1.calc.json` and `poe1-empty.json` regenerated. The only change in each is the new flag query's node and the node numbers after it.

**Fixtures:**
- `ice-nova-plate-vest-movement-penalty-witch.xml`: a Plate Vest (3% penalty) and 20% increased movement speed, 1.17.
- `ice-nova-ignore-movement-penalties-witch.xml`: the Plate Vest and the line, 1.0.

Every computed field of both equals PoB's.

**Evidence** (`b-ct`, against P6-11co's run):
- **Public:** 0 unexpected; 2 values toward PoB (one build's two movement speed fields), none away.
- **Corpus:** 0 unexpected; 4 values toward PoB and 2 now equal, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11cq: life regeneration while using a life flask

**PoB** reads "Regenerate #% of Life per Second if you've used a Life Flask in the past 10 seconds" (Heart of Oak) as `LifeRegenPercent` BASE with the condition `UsingLifeFlask` (`ModParser.lua` 1744). No config check sets the condition. `CalcPerform.lua` sets it when all of these hold (1729 to 1748, 1783):
- an active flask's base recovers life;
- its recovery is not all instant;
- the character can recover life outside leech (not Vaal Pact);
- flasks apply to the character.

**The engine:**
- `flasks::using_life_flask` settles the condition from the active flasks' bases, sharing the active-slot and base lookups with `flasks::flasks`.
- The other inputs are unmapped stats, each already a reason on every field wherever it appears: the flask's own instant recovery, `LifeFlaskInstantRecovery`, Vaal Pact's `CannotRecoverLifeOutsideLeech`, and "Flasks do not apply to you".
- `poe1::conditions` inserts it, and the stat is a state row on life's percent regeneration per minute.

**Fixtures:** each is a Ranger with the path to Heart of Oak. A custom line with the game's per-second text leaves the per-minute value unsettled (as in P6-11bq); the notable's stat is typed.
- `ice-nova-life-flask-regen-ranger.xml`: an active Divine Life Flask, 92.4 life per second.
- `ice-nova-utility-flask-no-life-regen-ranger.xml`: an active Basalt Flask only, 0.

Every computed field of both equals PoB's.

**Evidence** (`b-ct`, against P6-11cp's run):
- **Public:** 0 unexpected; no value moved (the builds with the notable have other reasons on the field).
- **Corpus:** 0 unexpected; 2 values toward PoB and 1 now equal, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11cr: Cyclone's movement speed as the main skill

**PoB** gives Cyclone's `cyclone_movement_speed_+%_final` as `MovementSpeed` MORE with `Condition:ChannellingCyclone`, a `GlobalEffect` buff of any active Cyclone (`act_dex.lua` 4719), the main skill included. `poe1::gems` applied it for a Cyclone that is not the main skill (P6-8a). As the main skill, the stat went through `SkillMapping`, which keeps only what the hit reads, so it was an unmapped stat.

**The engine:** `SkillMapping` maps the stat (`mapping::CYCLONE_MOVEMENT_STAT`) to that MORE with the `ChannellingCyclone` state, which the config's check settles.

**Evidence** (`b-ct`, against P6-11cq's run):
- **Public:** 0 unexpected. The two Cast while Channelling fixtures are unchanged (already complete and equal to PoB, through `poe1::gems`), so nothing is counted twice.
- **The one move away:** the community Cyclone build's two movement speed fields (partial) moved away from PoB's value, 2.11 to 1.688 against PoB's 2.008. PoB gives the same 20% less: with the check turned off, PoB's value is 2.51 (2.008 = 2.51 × 0.8; local run `local/p6/cr-xo`). The gap is the 40% increased movement speed that the build's unmapped stats give in PoB; those stats are the fields' reasons.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.

## P6-11cs: minion more damage and minion crit multiplier

**PoB** reads both lines as `MinionModifier` LISTs:
- The Necromancer's "Minions deal #% more Damage" (`necromancer_minion_damage_+%_final`) holds `Damage` MORE.
- "Minions have +#% to Critical Strike Multiplier" (`minion_critical_strike_multiplier_+`) holds `CritMultiplier` BASE.

**The engine:**
- `minions::MinionValue` gains `CritMultiplier`, which reaches the minion's `CritMultiplier`.
- Both stats are `minions::STATS` rows. The more damage is not redirected: Spiritual Aid's redirect copies increases only, as PoB's.
- **Tests:** the rows' texts (`tests/minions.rs`). The gem mapping test's unmapped example is now the minions' penetration against cursed enemies.

**Fixture** `minion-raise-zombie-more-damage-crit-multiplier-witch.xml`: Raise Zombie with 10% more minion damage and +30% minion crit multiplier. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cr's run):
- **Public:** 0 unexpected; 2 values toward PoB (two community builds' full DPS), none away.
- **Corpus:** 0 unexpected; 1 value toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11ct: minion damage while you are affected by a herald

**PoB** reads "Minions deal #% increased Damage while you are affected by a Herald" as a `MinionModifier` LIST of `Damage` INC. The inner mod carries `ActorCondition` `parent` `AffectedByHerald`. PoB sets that condition for each herald among the active skills (`CalcPerform.lua` 2117 to 2120), the same pass that counts `Multiplier:Herald`.

**The engine:**
- `poe1::conditions` settles `AffectedByHerald` as `gems::herald_and_aura_counts`' herald count above 0 (P6-11af's count).
- The stat is a state row on the minion's damage, which `minion_actor::minion_modifiers` evaluates against the character's settled states (P6-11cm).

**Fixture** `minion-raise-zombie-herald-damage-witch.xml`: Raise Zombie with Herald of Ash in a second group and the line at 30%. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cs's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.

## P6-11cu: minion penetration against cursed enemies

**PoB** reads "Minions Penetrate #% of Cursed Enemies' Elemental Resistances" as a `MinionModifier` LIST of `ElementalPenetration` BASE. The inner mod carries `ActorCondition` `enemy` `Cursed`. Two things make the enemy `Cursed`:
- **A filled curse slot** (`CalcPerform.lua` 3301 to 3303).
- **The config check** `conditionEnemyCursed` (`ConfigOptions.lua` 1780).

**The engine:**
- `gems::curses_enemy` is whether the gem table applies a curse or mark (a `Kind::Curse` skill in the groups `affected_by` reads). PoB's curse limit starts at 1, so such a skill fills a slot. A curse is never a main skill whose hit the engine computes.
- `poe1::conditions` settles `Enemy:Cursed` from the check or `curses_enemy`; the check leaves `config::NAMED_ONLY`.
- `minions::MinionValue::Penetration(type)` reaches the minion's `Penetration`. The stat is a state row of fire, cold, and lightning penetration on `Enemy:Cursed`, which `minion_actor::minion_modifiers` evaluates against the character's settled states.

**Fixtures** (Summon Flame Golem, a fire spell, with the line at 15%):
- `minion-summon-flame-golem-cursed-penetration-witch.xml`: with Punishment in a second group, 483.3 average damage.
- `minion-summon-flame-golem-uncursed-penetration-witch.xml`: no curse, 371.7.

Every computed field of both equals PoB's.

**Found on the way** (recorded for review): with Temporal Chains instead of Punishment, PoB's minion ignite lasts 5.33 s instead of 4 and its `IgniteDPS` rises from 37.7 to 50.3. The engine reads Temporal Chains' `buff_time_passed_+%_other_than_temporal_chains` as `NoSlice` (`gems::SKILLS`). It is not a reason, so the minion's ignite is wrong, unflagged, for a build with Temporal Chains. No public or corpus build has both.

**Evidence** (`b-ct`, against P6-11ct's run):
- **Public:** 0 unexpected; 2 values toward PoB (two community builds' full DPS), none away.
- **Corpus:** 0 unexpected; 1 value toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11cv: Temporal Chains lengthens the ailments

**Found in P6-11cu.** PoB reads Temporal Chains' `buff_time_passed_+%_other_than_temporal_chains` as the enemy's `BuffExpireFaster` MORE (`act_dex.lua` 15947). The ailment durations are multiplied by `debuffDurationMult = 1 / max(BuffExpirationSlowCap, mod(enemy, BuffExpireFaster))` (`CalcOffence.lua` 1457 to 1459; ignite, bleed, and poison at 4904, 4317, 4588), and `BuffExpirationSlowCap` is 0.25 (`Data.lua` 235). The gem table had the stat as `NoSlice`, so the ailments were wrong, unflagged, for a build with Temporal Chains.

**The engine:**
- `Poe1Quantity::EnemyBuffExpireFaster` is an enemy quantity, so the minion's hits read it too.
- Temporal Chains' stat is a curse row on it (MORE), scaled by curse effect as the curse's other rows.
- `poe1::ailment` divides the duration by `max(0.25, mod)` when the quantity has effects (`ailment::BUFF_EXPIRATION_SLOW_CAP`). The stacks follow the duration, as in PoB.

**Fixtures:**
- `minion-summon-flame-golem-temporal-chains-witch.xml`: the Flame Golem's ignite, 4 s to 5.33 s.
- `burning-arrow-temporal-chains-templar.xml`: the character's ignite. Its attack rate's Q11 rounding is registered as DIV-0366 to DIV-0371.

Every other computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cu's run):
- **Public:** 0 unexpected; 3 values toward PoB (a build's ignite and combined DPS, another's full DPS), none away.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.
- **data-parity** (`gem_effects`, run at P6-11dl): the new row first showed one uncategorized difference, `<poe1.enemy-buff-expire-faster>` against PoB's `BuffExpireFaster`: the table's naming did not know the quantity. It now names it (`tools/parity-cli/src/data_parity/gem_effects.rs`), and `uncategorized` is 0.

## P6-11cw: attack damage per 450 armour

**PoB** reads "#% increased Attack Damage per 450 Armour" as `Damage` INC with `ModFlag.Attack` and a `PerStat` tag of `Armour`, `div` 450, no limit (`ModCache`). `PerStat` reads the actor's computed `Armour`, so the value is `floor(Armour / 450 + 0.0001) × value`, one term of the hit's increased sum.

**The engine:**
- `hit::DAMAGE_PER_STAT` generalizes P6-11bw's spell damage per 100 maximum mana. Each entry gives a quantity, the flag the hit needs, the field, the step, and the limit. The new entry is `Poe1Quantity::AttackDamagePer450Armour` on the armour field (computed in `primary-defences`, before the hits).
- `hit::per_stat` builds the term for each entry; both of the hit's damage sums take every term that applies.
- The graph's step names for the mana term are now `per-stat-*`. Its value is unchanged: the P6-11bw fixture still equals PoB's.

**Fixture** `smite-attack-damage-per-armour-templar.xml`: Smite with +5000 armour and the line at 2%. Its average damage equals PoB's. The attack rate's Q11 rounding is registered as DIV-0372 to DIV-0377.

**Evidence** (`b-ct`, against P6-11cv's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected; 17 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11cx: brand and mark skills' cast speed

**PoB** reads "#% increased Cast Speed with Brand Skills" and "Mark Skills have #% increased Cast Speed" as `Speed` INC with `ModFlag.Cast` and a `SkillType` tag (`Brand`, `Mark`).

**The engine:**
- `skill::SKILL_TYPE_FLAGS` names the skill types whose tag a row reads, each with its flag: P6-11co's `RangedAttack`, and now `Brand` (`Flags::SKILL_BRAND`, bit 51) and `Mark` (`Flags::SKILL_MARK`, bit 52). `skill::keyword_flags` sets them from the skill's types, and the ailments keep them.
- The two stats are `HIT_ROWS` rows. A brand or a mark is not a main skill whose hit the engine computes (`hit::NOT_SELF_CAST`), so today the rows only keep these lines from being reasons on other skills, as in PoB.
- **Tests:** `keyword_flags` of a brand and a mark. The maintainer-run test checks the type names against the real bundle; it passes on `b-ct`.

**Fixture** `ice-nova-brand-mark-cast-speed-witch.xml`: Ice Nova with both lines at 30%. Its cast rate equals PoB's.

**Evidence** (`b-ct`, against P6-11cw's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.

## P6-11cy: trap throwing and mine laying speed; Trap and Mine Damage

**PoB** (`CalcOffence.lua` 1223 to 1337) sets a trap's rate to `1 / TrapThrowingTime × calcLib.mod(TrapThrowingSpeed) × ActionSpeedMod`, at most the server tick rate, and a mine's alike with `MineLayingSpeed`. The engine had the base time and the action speed (P6-6e); the speed modifiers were unmapped stats.

**The engine:**
- `Poe1Quantity::TrapThrowingSpeed` and `MineLayingSpeed` (hit quantities). The trap or mine rate multiplies by their `Increased` and `More` when the store has any.
- **Rows** (`HIT_ROWS`):
  - "#% increased Trap Throwing Speed" and "#% increased Mine Throwing Speed" (INC);
  - Trap and Mine Damage's less throwing and laying speed (MORE);
  - its more damage (`Damage` MORE with either `KeywordFlag.Trap` or `Mine`);
  - Trap Support's more trap damage (`support_trap_damage_+%_final`, `SkillStatMap.lua` 1777).
- **`NoEffect`:** the supports' player-versus-player scaling (`pvp`), and "cannot support melee attacks" (`skill type marker`).
- **Not done:** mines. Blastchain Mine's own stats stay reasons: its costs as reservation (`remote_mined_by_support`, PoB's `ManaCostGainAsReservation`), its more mine damage, and its detonation. So a mine skill's fields stay partial (tasks file).

**Fixture** `ice-nova-trap-throwing-speed-witch.xml`: Ice Nova with Trap and Trap and Mine Damage, and 25% increased trap throwing speed. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11cx's run):
- **Public:** 0 unexpected; 9 values toward PoB (a trap build's and a mine build's rate and damage), none away.
- **Corpus:** 0 unexpected; 10 values toward PoB and 2 now equal, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11cz: life flask charges on suppressing a spell

**PoB** leaves "Life Flasks gain # Charges when you Suppress Spell Damage" unparsed, so it applies nothing. Flask charges reach no slice field.

**The engine:** a `NoEffect` row (`flask`). The charges guard (`charges::tests`) lists it among the `NoEffect` rows that name a charge: these are flask charges, not power, frenzy, or endurance charges.

**Evidence** (`b-ct`, against P6-11cy's run): public and corpus 0 unexpected; no value moved (the builds with the line have other reasons). The workspace tests and clippy pass.

## P6-11da: a trap skill's visual variant

`trap_variation` is a trap skill's own stat (Bear Trap and others, `act_dex.lua` 1656 and on). It picks the trap's look; PoB lists it in the skill's stats with no stat map entry, so it applies nothing. A `NoEffect` row (`display`).

**Evidence** (`b-ct`, against P6-11cz's run): public and corpus 0 unexpected; no value moved. The workspace tests pass.

## P6-11db: attack speed per frenzy charge

**PoB** reads "#% increased Attack Speed per Frenzy Charge" as `Speed` INC with `ModFlag.Attack` per `Multiplier:FrenzyCharge`. Two stats print that text: `base_attack_speed_+%_per_frenzy_charge` (a passive, `frenzy_charges2420`) and `attack_speed_+%_per_frenzy_charge`. No item mod carries either, so an item line with the text stays ambiguous; the fixture takes the passive.

**The engine:** `charges::PerCharge::AttackSpeed` (`Speed` INC with `Flags::ATTACK`) and a `MODIFIERS` row for the base stat.

**Fixture** `smite-attack-speed-per-frenzy-duelist.xml`: Smite as a Duelist with three frenzy charges and the passive's path. Every computed field equals PoB's but Q11's attack rate (DIV-0378 to DIV-0383).

**Evidence** (`b-ct`, against P6-11da's run):
- **Public:** 0 unexpected; no value moved (the builds with the passive have other reasons).
- **Corpus:** 0 unexpected; 8 values moved, 6 toward PoB. Two moved away, both in one Shield Crush build: its attack rate and mana cost per second. That build's rate was already above PoB's (2.536 against 2.387) from stats still unmapped (off-hand added damage, attack speed per 200 accuracy rating); the frenzy charges' speed applies there in PoB too.
- **Tests:** the workspace tests and clippy pass.

## P6-11dc: ailment damage from attack skills; Burning Damage Support

**PoB:**
- "#% increased Damage with Ailments from Attack Skills" (`attack_ailment_damage_+%`) is `Damage` INC with `ModFlag.Ailment` and `KeywordFlag.Attack`. An attack's ailments keep the attack keyword (P6-2b), so the row reaches its bleeding, ignite, and poison.
- Burning Damage Support's `support_burning_damage_+%_final` is `FireDamage` MORE with `KeywordFlag.FireDot` (`sup_str.lua` 915), as the burning damage row (`burn_damage_+%`) but more.

**The engine:** two `HIT_ROWS` rows. The support was already in `gems::SUPPORTS` with no global effect; its stat was the reason.

**Fixtures:**
- `smite-attack-ailment-damage-templar.xml`: Smite with a chance to bleed and 25% increased damage with ailments from attack skills. Every computed field equals PoB's but Q11's attack rate (DIV-0384 to DIV-0389).
- `ice-nova-burning-damage-support-witch.xml`: Ice Nova with a chance to ignite and Burning Damage Support. Every computed field equals PoB's.

**Not done:** "#% increased Damage per Summoned Golem" (`damage_+%_per_active_golem`, 5 public builds). PoB reads it per `PerStat` `ActiveGolemLimit`, which `GetStat` takes from `output.ActiveGolemLimit` (never set) or the skill's stats, so it applies 0. The game applies it per golem. It stays a reason until an in-game check (tasks file).

**Evidence** (`b-ct`, against P6-11db's run):
- **Public:** 0 unexpected; 8 values toward PoB (a ballista build's bleeding, ignite, and poison, and a burning build's damage over time), none away.
- **Corpus:** 0 unexpected; 3 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11dd: maximum life gained as extra maximum energy shield

**PoB** (`CalcDefence.lua` 1023 to 1036): "Gain #% of Maximum Life as Extra Maximum Energy Shield" is `LifeGainAsEnergyShield` BASE. Energy shield gains one more row, after the pieces' and the global row: `Life BASE x percent / 100`, times `calcLib.mod(modDB, nil, "Life", "EnergyShield", "Defences")`. The breakdown names it "Conversion", "Life to Energy Shield". Under Chaos Inoculation the row's total is 1.

**The engine:**
- `Poe1Quantity::LifeAsEnergyShield` and a `ROWS` row.
- `phases::life_as_energy_shield` builds the row (`step.slot.conversion.life-to-energy-shield`):
  - life's base is its BASE effects and the strength bonus, as the life field's;
  - the increases of life and energy shield are summed with energy shield's extra increases (the intelligence bonus among them, an `EnergyShield` INC in PoB's store);
  - the more multipliers are life's and energy shield's, each rounded, as PoB rounds per mod name.
- `primary_defence` adds the row last, before the rounding. The energy shield field now also depends on strength.
- Chaos Inoculation's 1 is not read: its keystone is a reason of the life field.

**Tests:** `life_gained_as_energy_shield_should_be_a_row_of_energy_shield` (a row of 918 from life's base 1200, 50%, and 53% increased).

**Fixture** `ice-nova-life-as-energy-shield-witch.xml`: Ice Nova with 20% of maximum life as energy shield, flat and increased life, strength, and increased and flat energy shield. Every computed field and the energy shield breakdown equal PoB's.

**Evidence** (`b-ct`, against P6-11dc's run):
- **Public:** 0 unexpected; 12 values toward PoB (a build's energy shield 1,236 to 1,468 against PoB's 1,611, its recharge, leech, maximum hits, and EHP), none away.
- **Corpus:** 0 unexpected; 17 values toward PoB, none away. One build's energy shield went from 457 to 677 against PoB's 680.
- **Tests:** the workspace tests and clippy pass.

## P6-11de: lines against the marked enemy

**PoB** sets the enemy's `Marked` only for a mark that holds a curse slot (`CalcPerform.lua` 3304). Three lines read it:
- "#% increased Damage with Hits and Ailments against Marked Enemy": `Damage` INC with `KeywordFlag.Hit` or `Ailment`;
- "Marked Enemy takes #% increased Damage": the enemy's `DamageTaken` INC;
- "Marked Enemy has #% increased Accuracy Rating": the enemy's `Accuracy` INC.

**The engine:**
- `conditions::ENEMY_MARKED` from `gems::marks_enemy`, which shares `curses_enemy`'s list of applied curse skills:
  - false with no mark among them;
  - true when every one is a mark, since the first takes a slot (the limit starts at 1, and a mark never loses its slot to another mark);
  - unsettled, so a reason, when hexes and a mark compete: the slots' priorities decide, and `gems::curse_slots` runs only while the gems are applied.
- Three `STATE_ROWS` entries. A hit's query and an ailment's never carry both flags (an ailment keeps every keyword but `Hit`), so the damage line is one row with `HIT` and one with `AILMENT`: PoB's either-flag match.

**Fixture** `burning-arrow-marked-enemy-templar.xml`: Burning Arrow with Sniper's Mark and the three lines. Every computed field equals PoB's but Q11's attack rate (DIV-0390 to DIV-0395).

**Evidence** (`b-ct`, against P6-11dd's run):
- **Public:** 0 unexpected; 14 values moved, 13 toward PoB, 5 now equal (a mark build's evade chances and hits).
- **Corpus:** 0 unexpected; 29 values moved, 28 toward PoB.
- **Away:** one `NumberOfMitigatedDamagingHits` in each run, by under 0.02% (2.3633 to 2.3631; 0.68615 to 0.68608). It follows the evade chance, which moved toward PoB (equal to it in the public build); the field was already partial.
- **Tests:** the workspace tests and clippy pass.

## P6-11df: the effect of marks

**PoB** reads "#% increased Effect of your Marks" (`mark_skills_curse_effect_+%`, on passives such as Focal Point and Expert Hunter) as `CurseEffect` INC tagged `SkillType.Mark`. It scales a mark's effect on the enemy as the global curse effect does.

**The engine:**
- `Poe1Quantity::MarkCurseEffect`, a `skill_modifiers::STATS` row, and `Totals::mark_curse_effect`.
- `gems` adds it to a mark's own increase before the curse's scale (`Totals::curse_scale_with`), as Mark On Hit's increase (P6-9g). Mark On Hit's own stat is still read from the support, not through the mapping.
- The texts test lists the row's text; the maintainer-run test passes on `b-ct`.

**Fixture** `burning-arrow-mark-effect-templar.xml`: Burning Arrow with Sniper's Mark and 40% increased effect of marks. PoB's total DPS goes from 6.89 (P6-9e's fixture without the line) to 7.48. Every computed field equals PoB's but Q11's attack rate (DIV-0396 to DIV-0401).

**Not done:** the marks' effect per maximum power charge (Deathmarked) and while Elusive (The Vivid Cat) stay reasons.

**Evidence** (`b-ct`, against P6-11de's run):
- **Public:** 0 unexpected; 4 values toward PoB (a Deadeye build's damage), none away.
- **Corpus:** 0 unexpected; 10 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11dg: the bows' additional arrows

**PoB** reads "Bow Attacks fire an additional Arrow" (`number_of_additional_arrows`) as `ProjectileCount` BASE with `ModFlag.Bow`. P6-11u found that the projectile count reaches a slice field only with `SequentialProjectiles`, which Barrage's `projectiles_barrage` sets.

**The engine:**
- a `NoEffect` row (`projectile-count`), guarded by `projectiles_barrage` in `NO_EFFECT_DEPENDS_ON`, as `number_of_additional_projectiles` is;
- the guard table now allows one stat to guard several rows: it is sorted by stat, then row, and only tests read it.

**Also fixed:** six earlier `NO_EFFECT` comments (P6-11br, P6-11cd, P6-11cf, P6-11cj to P6-11cl) held a literal `\n` where a line break was meant.

**Evidence** (`b-ct`, against P6-11df's run): public and corpus 0 unexpected; no value moved. The workspace tests and clippy pass.

**Not done:** "#% more Damage taken from other Enemies near your Marked Enemy" (Focal Point, 4 builds). PoB leaves it unparsed, so it applies nothing; the game applies it. It stays a reason (tasks file).

## P6-11dh: resistances capped by the highest maximum

**PoB** (`CalcDefence.lua` 561 to 576): with "Elemental Resistances are capped by your highest Maximum Elemental Resistance instead" (`ElementalResistMaxIsHighestResistMax`, Melding of the Flesh), each elemental maximum is `min(cap, sum)`. The highest of them, starting from 0, overrides every elemental maximum. Chaos keeps its own.

**The engine:**
- `Poe1Quantity::ElementalResistMaxIsHighest`, a `Flag` row.
- `phases::highest_maximum_resistance` builds that maximum for fire, cold, and lightning when the flag is present. Without it the graph is unchanged: the node order is kept, so the stored calculation results stay equal.

**Tests:** `elemental_resistances_should_be_capped_by_the_highest_maximum` (fire's 79 caps all three; chaos unchanged).

**Fixture** `ice-nova-highest-maximum-resistance-witch.xml`: Ice Nova with the line, +4% maximum fire and +1% maximum cold resistance, and +160% to all elemental resistances. Each elemental resistance is 79, as in PoB.

**Evidence** (`b-ct`, against P6-11dg's run):
- **Public:** 0 unexpected; 12 values toward PoB, 6 now equal (an Elementalist's fire and lightning resistances 74 to 90, their taken multipliers, maximum hits, and EHP).
- **Corpus:** 0 unexpected; 12 values toward PoB, 6 now equal.
- **Tests:** the workspace tests and clippy pass.

## P6-11di: brand damage

**PoB** reads "#% increased Brand Damage" (`sigil_damage_+%`) as `Damage` INC with `KeywordFlag.Brand`, which a skill gets from its `Brand` type: P6-11cx's `Flags::SKILL_BRAND`.

**The engine:** a `HIT_ROWS` row. A brand is not a main skill whose hit the engine computes (`hit::NOT_SELF_CAST`), so the row only keeps the line from being a reason on other skills, as in PoB.

**Not done:**
- "Enemies take #% increased Damage for each of your Brands Attached to them" needs PoB's attached brand count (`CalcPerform.lua` 1320 to 1332).
- The travel skills' cooldown recovery needs a per-skill-type cooldown.
- The totem lines (damage per summoned totem, the totem limit) need PoB's `TotemsSummoned`. Totem skills are partial on `totem-data` meanwhile.

All stay reasons.

**Evidence** (`b-ct`, against P6-11dh's run): public and corpus 0 unexpected; no value moved (the brand builds have other reasons). The workspace tests and clippy pass.

## P6-11dj: three lines with no slice field

`NoEffect` rows:
- **"Reflects # Fire Damage to Melee Attackers"** (`reflect`): unparsed in PoB; damage to attackers, which no slice field reads, as the physical one (P6-11m).
- **"Can be Anointed"** (`display`): an item property PoB reads only to offer the anoint. The anoint's passive is the item's own line (P3-8n).
- **The on-kill explosion** (`explosion`): PoB's `ExplodeMod` and `CanExplode` are read only by its own "On Kill Monster Explosion" skill (`EnemyExplode`, in a group PoB adds with source "Explode"). No game gem grants that skill. The gem table skips a group with a source, and the gem is `unmodelled-gem` as a main skill or in full DPS.

**Kept as reasons:**
- "Magic Utility Flasks cannot be Used": PoB leaves it unparsed, so its flasks stay active there; the game applies it.
- Phasing on kill: it sets a condition other lines read.
- "Take no Extra Damage from Critical Strikes if you have taken a Critical Strike Recently": unparsed in PoB; the game applies it to hits taken.

**Evidence** (`b-ct`, against P6-11di's run): public and corpus 0 unexpected; no value moved. The workspace tests and clippy pass.

## P6-11dk: added fire damage against burning enemies

**PoB:**
- Immolate Support's "# to # added Fire Damage against Burning Enemies" is `FireMin` and `FireMax` BASE with the enemy's `Burning` (`SkillStatMap.lua` 831 to 836).
- The enemy burns when the config checks it as burning (`conditionEnemyBurning`, `ConfigOptions.lua` 1849) or as ignited. `conditionEnemyIgnited` implies `Burning` (1852), and `CalcSetup.lua` 72 gives `Burning` from `Ignited`.

**The engine:**
- `conditions::ENEMY_BURNING` ("Enemy:Burning"), set by either check through `CHECKS`. `conditionEnemyBurning` left `config::NAMED_ONLY`.
- Two `STATE_ROWS` rows for the added damage.

**Fixture** `ice-nova-immolate-ignited-enemy-witch.xml`: Ice Nova with Immolate Support against an enemy checked as ignited. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11dj's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected; 5 values toward PoB, none away. One build's average damage went from 29,141 to 52,813 against PoB's 84,459.
- **Tests:** the workspace tests and clippy pass.

## P6-11dl: Pride's impale stacks; two `NoEffect` rows

**PoB** reads "Impales you inflict last # additional Hits while using Pride" as `ImpaleStacksMax` BASE while `AffectedByPride`. The engine's impale (P6-2d) reads `ImpaleStacksMax`, and `AffectedByPride` is settled from the auras (P6-11o).

**The engine:**
- a `STATE_ROWS` row for Pride's line;
- `NoEffect` rows for "Projectile Barrages have no spread" (unparsed in PoB; the projectiles' geometry, `projectile-spread`) and Cyclone's area per additional melee range (`area`).

**Fixture** `smite-pride-impale-stacks-templar.xml`: Smite with a chance to impale, Pride in its own group, and the line at 2. Every computed field equals PoB's but Q11's attack rate (DIV-0402 to DIV-0407).

**Evidence** (`b-ct`, against P6-11dk's run):
- **Public:** 0 unexpected; 3 values toward PoB (a Slayer's impale DPS), none away.
- **Corpus:** 0 unexpected; 2 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11dm: supports under old skill IDs, found by name

**Found:** builds made before a support's granted effect was renamed carry the old `skillId` with the game's `gemId` and no `variantId`:
- Enlighten as `SupportAdditionalXP`;
- Second Wind as `SupportAdditionalCooldown`;
- Faster Attacks as `SupportFasterAttack`;
- Void Manipulation, Vicious Projectiles, and the Awakened supports, among others.

PoB's `LoadSkill` misses them by `skillId`, and `ProcessSocketGroup` then finds the gem by its name (`FindSkillGem`, `SkillsTab.lua` 1076). The engine's fallback by name (Phase 6) took a default variant's name from its active skill. A support has none, so these gems were `unmodelled-gem`: the public builds now have 34 fewer such reasons (129 to 95), among them Enlighten (3 builds), Second Wind (4), and Faster Attacks (2), already in `gems::SUPPORTS`.

**The engine:**
- `CalcData::item_base_name` (defaulted, `None`): the bundle gives a base item's name by its ID, and `OverlayData` (`calc-run --data-overlay`) forwards it.
- `skill::resolve`'s fallback: a gem effect without a name of its own and without an active skill takes the gem's base item name without " Support", as PoB names its gems.
- "Increased Duration" (the game's More Duration now) stays unknown, as PoB's `FindSkillGem` does not find it either.

**Tests:** `real_supports_under_old_skill_ids_should_be_found_by_name` (maintainer-run, passes on `b-ct`): Enlighten found, Increased Duration unknown. The synthetic bundle's support has a name of its own, so it cannot show the case.

**Evidence** (`b-ct`, against P6-11dl's run):
- **Public:** 0 unexpected; 36 values moved, 30 toward PoB, 5 now equal (mana costs 25 to 53 and 49 to 90, equal to PoB's). 6 moved away, each checked:
  - **A Berserker's attack rate** (1.81 to 1.64 against PoB's 2.20): its main group's Awakened Melee Physical Damage now applies, with a less attack speed multiplier PoB applies too; the rate stays partial on other modifiers.
  - **Its unreserved mana** (141 to 205 against PoB's 138): Enlighten now applies to its auras. PoB reserves 599 without the gem and 534 with it, a drop of 65; the engine reserves 531 and 467, a drop of 64. The engine was already about 67 short before this step.
  - **A Cast On Critical Strike build's rate and DPS:** its triggered skill's supports now apply, and the trigger rate stays partial.
  - **A build's life regeneration** (462 to 455.8 against PoB's 724.5): Second Wind now supports Enduring Cry and lowers the warcry's uptime. PoB goes from 732 without the gem to 724.5 with it.
- **Corpus:** 0 unexpected; 64 values moved, 58 toward PoB, 7 now equal, 5 away. The away moves were checked on local copies of the builds:
  - **An unreserved mana** (-118 to 209 against PoB's 18): Divine Blessing now applies to an aura. PoB's reservation drops by 376 with it, the engine's by 393. Without it the engine reserves 121% of mana against PoB's 141%, so other reservations are short.
  - **A mana cost** (10 to 20 against PoB's 15): each newly found support moves both the same way (Vicious Projectiles: PoB 11 to 15, engine 15 to 20). The engine's base cost is about a third higher.
  - **One build's hit damage** (57 to 46 against PoB's 5,443, with its leech) is open. Its life reserved changed too (901 to 630 unreserved). No single newly found support explains the hit's drop: without Vicious Projectiles it is 43.9, and 45.9 without each other one. Vicious Projectiles' own stats are unmapped (next step). Tasks file.
- **Tests:** the workspace tests and clippy pass.

## P6-11dn: Vicious Projectiles

**PoB** (`sup_dex.lua` 4360 to 4372) maps four stats:
- more projectile attack physical damage: `PhysicalDamage` MORE with `ModFlag.Attack` and `Projectile`;
- less projectile attack speed: `Speed` MORE with the same flags;
- more physical damage over time: `PhysicalDamage` MORE with `KeywordFlag.PhysicalDot`;
- more chaos damage over time: `ChaosDamage` MORE with `KeywordFlag.ChaosDot`.

The support was in `gems::SUPPORTS` with no global effect, but its stats were reasons; P6-11dm made the builds' old-ID copies known.

**The engine:** four `HIT_ROWS` rows.

**Fixture** `burning-arrow-vicious-projectiles-templar.xml`: Burning Arrow with Vicious Projectiles. Every computed field equals PoB's but Q11's attack rate (DIV-0408 to DIV-0413).

**Evidence** (`b-ct`, against P6-11dm's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected; 17 values moved, 15 toward PoB.
  - P6-11dm's open build gains the support's damage: its hit's average damage goes from 46 to 68, above the 57 it had before P6-11dm.
  - Two of its values moved away: the attack rate (2.15 to 1.93 against PoB's 2.18) and the life cost per second that follows it. They carry the support's less attack speed, which PoB applies too; the rate is partial on other modifiers.
- **Tests:** the workspace tests and clippy pass.

## P6-11do: Void Manipulation and Power Charge On Critical

P6-11dm made these supports known in more builds; their stats were reasons.

**PoB:**
- Void Manipulation's `support_void_manipulation_chaos_damage_+%_final` is `ChaosDamage` MORE.
- Power Charge On Critical's `support_power_charge_on_crit_damage_+%_final_per_power_charge` is `Damage` MORE per `Multiplier:PowerCharge` (`sup_int.lua` 5604).
- Its chance to gain a power charge on crit is never read by PoB's calculation; the count is the config's.

**The engine:**
- **Void Manipulation:** a `HIT_ROWS` row (`Damage` MORE with `Flags::CHAOS`).
- **The per-charge damage:**
  - `charges::PerCharge::MoreDamage` (`Damage` MORE), and a `MODIFIERS` row for the support's stat.
  - The main skill's mapping (`SkillMapping`) now keeps a per-charge row whose effect is a hit quantity. The charge phase turns it into that effect times the count, so it stays the main skill's, as PoB's `skillModList` entry.
- **The chance:** a `NoEffect` row (`charge-gain-chance`), listed in the charges test's `NoEffect` rows that name a charge.

**Fixtures:**
- `smite-void-manipulation-templar.xml`: Smite with Void Manipulation and added chaos damage to attacks.
- `smite-power-charge-on-crit-templar.xml`: Smite with Power Charge On Critical and three power charges.

Every computed field equals PoB's but Q11's attack rate (DIV-0414 to DIV-0424).

**Evidence** (`b-ct`, against P6-11dn's run):
- **Public:** 0 unexpected; 12 values toward PoB, none away.
- **Corpus:** 0 unexpected; 5 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11dp: impales, endurance charges on stun, crits against bleeding enemies

**PoB:**
- "#% chance to Impale Enemies on Hit with Attacks while using Pride": `ImpaleChance` BASE with `KeywordFlag.Attack`, `AffectedByPride`.
- "#% chance on Hitting an Enemy for all Impales on that Enemy to last for an additional Hit": `ImpaleAdditionalDurationChance` BASE. The impale's stacks are `ImpaleStacksMax x (1 + chance / 100)` (`CalcOffence.lua` 5477).
- Endurance Charge on Melee Stun's `endurance_charge_on_melee_stun_damage_+%_final_per_endurance_charge`: `Damage` MORE per `Multiplier:EnduranceCharge` (`sup_str.lua` 2005).
- "+#% to Critical Strike Multiplier against Bleeding Enemies": `CritMultiplier` BASE with the enemy's `Bleeding`, which only its config check `conditionEnemyBleeding` sets.
- A flask's "#% increased Area of Effect during Effect": `AreaOfEffect` INC, which no slice field reads.

**The engine:**
- **Two `STATE_ROWS` rows:** Pride's impale chance, and the crit multiplier with `conditions::ENEMY_BLEEDING` (the check through `CHECKS`; it left `config::NAMED_ONLY`).
- **Impale stacks:** `Poe1Quantity::ImpaleAdditionalDurationChance` (a `ROWS` row) scales the impale's stacks when present.
- **The support:** a `charges::MODIFIERS` row with P6-11do's `PerCharge::MoreDamage` for the endurance charges.
- **The flask line:** `flasks::NO_EFFECT_LOCAL`.

**Fixtures:**
- `smite-pride-impale-chance-templar.xml`: Pride's impale chance and the additional duration chance.
- `smite-endurance-charge-on-stun-templar.xml`: the support with three endurance charges.
- `smite-crit-multiplier-bleeding-enemy-templar.xml`: the crit multiplier, with the enemy checked as bleeding.

Every computed field equals PoB's but Q11's attack rate (DIV-0425 to DIV-0443).

**Evidence** (`b-ct`, against P6-11do's run):
- **Public:** 0 unexpected; 13 values toward PoB (a Juggernaut's, a Slayer's, and a Deadeye's damage), none away.
- **Corpus:** 0 unexpected; 6 values toward PoB, none away.
- **Tests:** the workspace tests and clippy pass.

## P6-11dq: Versatile Combatant

**PoB** reads the keystone's lines (`ModCache.lua` 675):
- -10% to maximum chance to block attack damage and -10% to maximum chance to block spell damage (`BlockChanceMax`, `SpellBlockChanceMax` BASE);
- "+2% Chance to Block Spell Damage for each 1% Overcapped Chance to Block Attack Damage": `SpellBlockChance` BASE per `PerStat` `BlockChanceOverCap`, which is `max(0, total - maximum)` of attack block (`CalcDefence.lua` 694).

**The engine:**
- `keystone_versatile_combatant` is a `ROWS` row with the three effects; the third is the new `Poe1Quantity::SpellBlockPerOvercappedBlock`.
- `phases::attack_block` builds the overcap only when that quantity is present, so other graphs are unchanged; `spell_block` adds it times the rate to its base.

**Fixture** `ice-nova-versatile-combatant-witch.xml`: Ice Nova with the keystone (as a custom line), +80% attack block, and +5% spell block. Block is 65 and spell block 5 + 2 x 15 = 35, as in PoB.

**Evidence** (`b-ct`, against P6-11dp's run):
- **Public:** 0 unexpected; 6 values toward PoB, 1 now equal. A Slayer's attack block went from 69 to 65, equal to PoB's, and its spell block from 17 to 25 against PoB's 64; its EHP followed.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.

## P6-11dr: two lines with no slice field

`NoEffect` lines:
- "Minions Leech #% of Damage as Life" (`minion-leech`): a `MinionModifier` of `DamageLifeLeech`; no slice field reads a minion's leech.
- A flask's "Recover #% of Energy Shield when you Kill an Enemy during Effect" (`flasks::NO_EFFECT_LOCAL`): `EnergyShieldOnKill`, which no slice field reads.

**Not done:** "Auras from your Skills which affect Allies also affect Enemies" (`AurasAffectEnemies`, 3 builds). PoB copies each aura's modifiers onto the enemy as a debuff (`CalcPerform.lua` 2451 to 2464): Determination's armour and Grace's evasion, for example. The engine would need each aura row's enemy counterpart. It stays a reason (tasks file).

**Evidence** (`b-ct`, against P6-11dq's run): public and corpus 0 unexpected; no value moved. The workspace tests and clippy pass.

## P6-11ds: Hextouch (P6-11dm's open corpus drop)

**The cause of P6-11dm's open item:**
- A corpus build's Vulnerability is supported by Hextouch, whose old `skillId` (`SupportCurseOnHit`) P6-11dm now finds by name.
- Hextouch's gem also grants `SupportCurseOnHitCurse`, the part that supports the hex. The gem table did not have it, so the hex had an unmodelled support and was not applied: its physical damage taken and bleed chance went missing.
- Before P6-11dm, Hextouch was unknown and the hex applied at its full effect, which was wrong too.

**PoB** (`sup_int.lua` 1484): `SupportCurseOnHitCurse` triggers the hex on hit (`triggeredByCurseOnHit`, no cost) at `support_hextouch_curse_effect_+%_final`, `CurseEffect` MORE (-35% at every level). Its curse duration reaches no slice field. Hextouch's own `apply_linked_curses_on_hit_%` is display only.

**The engine:**
- `SupportCurseOnHitCurse` joins `gems::SUPPORTS`, and its less curse effect joins the curse's more multiplier with Blasphemy's (`gems::HEXTOUCH_CURSE_EFFECT_STAT`).
- `apply_linked_curses_on_hit_%` is `NoEffect` (`trigger`).
- **data-parity `gem_effects`:** allowances for the trigger, as Mark On Hit's (P6-9g): `triggeredByCurseOnHit` in `ALLOWED_BLOCKED`, `Triggered` in `ALLOWED_ADD_TYPES`. `uncategorized` 0.

**Fixture** `smite-hextouch-vulnerability-templar.xml`: Smite with Hextouch and Vulnerability in one group. Every computed field equals PoB's but Q11's attack rate (DIV-0444 to DIV-0450).

**Evidence** (`b-ct`, against P6-11dr's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected; 6 values toward PoB. The open build's hit damage is now 81, against 57 before P6-11dm.
- **Tests:** the workspace tests and clippy pass.

## P6-11dt: the rage effect and Berserk

**PoB** (`CalcPerform.lua` 881 to 897):
- **The rage effect:** `floor(stacks x calcLib.mod(RageEffect))`, attack `Damage` MORE by that amount.
- **"Every Rage also grants #% increased Attack Speed":** `Speed` INC with `ModFlag.Attack` per `Multiplier:RageEffect`, the effect.
- **Berserk** (`act_str.lua` 934): its buff is `RageEffect` INC, a `GlobalEffect` marked unscalable. PoB does not map its minimum rage or rage loss.

**The engine** (`poe1::stacks`):
- `StackValue::Effect` sums the rage effect's increases: "#% increased Rage Effect" (a `MODIFIERS` row) and Berserk's buff (`Increased`).
- The rage's damage is now `floor(stacks x (1 + inc / 100))`, and per-rage effects apply times that effect. "Every Rage also grants #% increased Attack Speed" is a `MODIFIERS` row; no per-rage row existed before, so none changed.
- **Berserk** joins `gems::SKILLS` as a buff with a `BuffStat::Unscalable` row. The data-parity `gem_effects` table names the quantity `RageEffect`; `uncategorized` 0.

**Fixture** `smite-berserk-rage-effect-templar.xml`: Smite with rage on melee hit, 20 rage, Berserk in its own group, 25% increased rage effect, and 1% attack speed per rage. PoB's rage effect is 36 (20 x 1.80). Every computed field equals PoB's but Q11's attack rate (DIV-0451 to DIV-0456).

**Evidence** (`b-ct`, against P6-11ds's run): public and corpus 0 unexpected; no value moved (the Berserker builds have other reasons). The workspace tests and clippy pass.

## P6-11du: Vaal Haste and Vaal Discipline

**PoB:**
- **Vaal Haste** (`act_dex.lua` 9171): Haste's three aura stats (attack speed, cast speed, movement speed), applied whenever the skill is in an enabled group. PoB merges buffs by name, so it stacks with Haste.
- **Vaal Discipline's** only global mod, `EnergyShieldRechargeNotDelayedByDamage`, is a `DUMMY` mod PoB's calculation never reads.

**The engine:**
- Both join `gems::SKILLS`: Vaal Haste as a buff with Haste's rows, Vaal Discipline with a `NoSlice` row.
- **data-parity `gem_effects`** allowances: the Discipline mod in `NO_SLICE_NAMES`, and both skills' soul gain prevention skill data in `ALLOWED_BLOCKED`. `uncategorized` 0.

**Fixture** `ice-nova-vaal-haste-witch.xml`: Ice Nova with Vaal Haste and Vaal Discipline in their own group. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11dt's run): public and corpus 0 unexpected; no value moved (the builds with them have other reasons). The workspace tests and clippy pass.

## P6-11dv: Immortal Call

**PoB** (`act_str.lua` 6700 to 6716): Immortal Call is a guard with three rows:
- less elemental damage taken (`ElementalDamageTaken` MORE);
- less physical damage taken (`PhysicalDamageTaken` MORE);
- less physical damage taken per removable endurance charge (`Multiplier:RemovableEnduranceCharge`, at most 5, `div = 100`). That is 0 without endurance charges in use.

PoB's guard slots (`CalcPerform.lua` 3262 to 3281) let one guard act, which the engine's guards follow (P6-9e).

**The engine:**
- Immortal Call joins `gems::SKILLS` as a guard. Its first two rows are `SelfDamageTaken(Always, ...)` MORE, so they reach both hits and damage over time, as in PoB.
- The per-charge row is the new `BuffStat::ReasonWhenInput`. It is a reason only while `useEnduranceCharges` is on: the removable charges are counted after the gem table.
- **data-parity `gem_effects`** names the character's `<Type>DamageTaken` and `ElementalDamageTaken`. `uncategorized` 0.

**Fixture** `ice-nova-immortal-call-witch.xml`: Ice Nova with Immortal Call in its own group, without endurance charges. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11du's run):
- **Public:** 0 unexpected. 33 values moved; 31 toward PoB, 2 now equal (a Toxic Rain build's taken multipliers, maximum hits, and EHP).
- **Two moved away:** a Deadeye's cold and lightning damage-over-time multipliers (cold 0.25 to 0.1875, against PoB's 0.2625). Its physical one is now 0.75, equal to PoB's, so Immortal Call applies as in PoB. PoB's cold value, 0.75 x 0.35, implies 65% cold resistance against the engine's 75%: a resistance gap that was already there.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.

## P6-11dw: Blood and Sand

**PoB** (`act_str.lua` 1227): Blood and Sand is a reserving buff.
- **Damage:** in each stance (the `bloodSandStance` config, Blood unless it says Sand), melee skills get more area damage: `Damage` MORE with `ModFlag.Area` and the `SkillType.Melee` tag, under `effectCond` `BloodStance` or `SandStance`.
- **Area of effect:** its area rows reach no slice field.

**The engine:**
- **The melee flag:** `Flags::SKILL_MELEE` (bit 53) for the `Melee` skill type, in `skill::SKILL_TYPE_FLAGS` (the real-bundle test checks the name; Smite is now `SKILL_LIGHTNING | SKILL_MELEE`). Ailments keep it, as the other skill-type flags.
- **The gem:** Blood and Sand joins `gems::SKILLS` with `BloodStance` and `SandStance` rows (as Flesh and Stone's, P6-11x) and `NoSlice` area rows. Its reservation is the skill's own, as for any reserving skill.
- **data-parity `gem_effects`:**
  - the two renderers know `ModFlag.Area` and the `Melee` skill type tag (`SkillType.Melee = 20`);
  - PoB's two buffs, "Blood" and "Sand", are `EXTRA_BUFFS`;
  - `uncategorized` 0.

**Fixture** `smite-blood-and-sand-templar.xml`: Smite's Area Hit with Blood and Sand in Blood Stance. Every computed field equals PoB's but Q11's attack rate (DIV-0457 to DIV-0460). Smite is left out of full DPS: with Blood and Sand present, PoB's full DPS pass gives Smite another value than its main calculation. Blood Stance gives 128.11 against 128.02; Sand Stance gives 95.85 against 96.27; without Blood and Sand the two agree. The cause is not found yet (tasks file).

**Evidence** (`b-ct`, against P6-11dv's run):
- **Public:** 0 unexpected; 12 values toward PoB, 2 now equal. P6-11dm's Berserker's unreserved mana is now PoB's 138: Blood and Sand's reservation was the 67 it lacked.
- **Corpus:** 0 unexpected; 8 values toward PoB, 1 now equal.
- **Tests:** the workspace tests, clippy, and the maintainer-run type test pass.

## P6-11dx: Battlemage's Cry

**PoB** (`act_int.lua`): Battlemage's Cry's buff is base critical strike chance per 5 warcry power, up to 25 (`CritChance` BASE, a `Warcry` effect with `div = 5`, `limit = 25`, the stat over 100). Its exerted attacks trigger the spells it supports; the stat is the skill's own, not global, and no slice field reads it.

**The engine:**
- Battlemage's Cry joins `gems::SKILLS` as a warcry with a `PerPower` row, scaled by power and uptime as P6-11bs's warcries.
- **data-parity `gem_effects`:** PoB's "Buff" and "Warcry" entries of the skill are `EXTRA_BUFFS`, as for Intimidating Cry. `uncategorized` 0.

**Fixture** `smite-battlemages-cry-templar.xml`: Smite with Battlemage's Cry in its own group. Every computed field equals PoB's (crit chance included) but Q11's attack rate (DIV-0461 to DIV-0465).

**Evidence** (`b-ct`, against P6-11dw's run): public and corpus 0 unexpected; no value moved (the builds with it have other reasons). The workspace tests and clippy pass.

## P6-11dy: Autoexertion

**Found:** Autoexertion's gem grants a second support, `SupportAutoexertion`, which the gem table lacked. A warcry it supports therefore had an unmodelled support and was skipped (Hextouch's case, P6-11ds).

**PoB** (`sup_str.lua`):
- `SupportAutoexertion` uses the warcry automatically (`triggeredByAutoexertion`).
- `CannotShareWarcryBuffs`: the warcry's buff does not apply to you (`CalcPerform.lua` 2326). Its exerts are counted before that check.
- It reserves its level's share of mana (20%) and gives the warcry a fixed mana cost.

**The engine:**
- `SupportAutoexertion` joins `gems::SUPPORTS` (`gems::AUTOEXERTION_SUPPORT`).
- A warcry it supports applies no rows to you. Its supports' global effects and its reservation are read as for any skill; the reservation takes the support's share, as Blasphemy's (P6-11r).
- **data-parity `gem_effects`:** allowances for the trigger and the added types, and the support in `RESERVING_SUPPORTS`. `uncategorized` 0.

**Fixture** `smite-autoexertion-templar.xml`: Smite with Battlemage's Cry and Autoexertion in their own group. The crit buff is gone and the unreserved mana equals PoB's; every computed field equals PoB's but Q11's attack rate (DIV-0466 to DIV-0471).

**Evidence** (`b-ct`, against P6-11dx's run):
- **Public:** 0 unexpected; 3 values moved, 2 toward PoB.
- **One moved away:** a Slayer's unreserved mana (41 to -85, against PoB's -18). Autoexertion's reservation for its General's Cry, checked by removing the gem on a local copy:
  - PoB reserves 118 for it: 20% of 714 mana at 21% efficiency.
  - The engine reserves 126: 20% at the build's 11%, the factor of its other skills.
  - PoB's extra 10% efficiency for this warcry comes from no stat the engine sees. Open (tasks file).
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests and clippy pass.

## P6-11dz: during any flask effect; small lines

**PoB:**
- `UsingFlask` is set by an active flask whose recovery is not all instant (`CalcPerform.lua` 1735), or by the config check `conditionUsingFlask`.
- "#% increased Elemental Damage during any Flask Effect" is `ElementalDamage` INC under it.
- "#% increased Critical Strike Chance against Bleeding Enemies" is `CritChance` INC with the enemy's `Bleeding` (P6-11dp's condition).
- "+#% to Damage Over Time Multiplier with Spell Skills" is `DotMultiplier` BASE with `KeywordFlag.Spell`.

**The engine:**
- `conditions::USING_FLASK`, from the check through `CHECKS` (it left `config::NAMED_ONLY`) or `flasks::using_flask`: any active flask. The lines that make recovery instant are reasons wherever they appear.
- **The elemental damage line:** one `STATE_ROWS` row per element. A query carries one damage type, so this reads as PoB's elemental match.
- **The other two:** a `STATE_ROWS` row for the crit chance, a `HIT_ROWS` row for the multiplier.
- **`NoEffect`:**
  - "#% increased Duration of Ailments on You" (`ailment-on-self`);
  - two flask lines (`flasks::NO_EFFECT_LOCAL`): avoid shock during the effect, and "Removes Curses on use".

**Fixture** `ice-nova-elemental-damage-during-flask-witch.xml`: Ice Nova with an active Quicksilver Flask and the elemental damage line. Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11dy's run):
- **Public:** 0 unexpected; 8 values toward PoB, none away.
- **Corpus:** 0 unexpected; 7 values moved, 6 toward PoB. One build's ignite DPS moved away (1,571 to 1,656, against PoB's 1,482): it was already above PoB's, and the line's fire increase applies there in PoB too.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11ea: allocation and radius stats

`NoEffect` lines:
- **Radius jewels' allocation stats** (`tree-allocation`): Intuitive Leap's and its keystone form, Thread of Hope's ring, and Impossible Escape's keystone. The pipeline reads them when it resolves the passives (`loom_pipeline`, `LEAP_STAT`, `RING_STAT`, `ESCAPE_STAT`). They act through the nodes they let you allocate, not as modifiers.
- **Skills' radius stats** (`area`): Earthquake's and Lightning Arrow's radius, the trap trigger radius, Caustic Arrow's explosion, and Kinetic Fusillade's targeting radius.
- **A flask's smoke ground on use** (`flasks::NO_EFFECT_LOCAL`).

**Not done:** "Passives in Radius grant ..." lines (the jewels that change the nodes in their radius) stay reasons; they need the radius's nodes in the calculation.

**Evidence** (`b-ct`, against P6-11dz's run): public and corpus 0 unexpected; no value moved. The workspace tests, clippy, and rustfmt pass.

## P6-11eb: eighteen lines with no slice field

`NoEffect` lines, each checked against PoB's reading of its text:
- **Chances to gain a charge** (`charge-gain-chance`): on a trap triggered or a mine detonated (frenzy and power), on a melee crit, and on stunning (endurance). PoB's calculation never reads them; the counts are the config's. The charges test lists them, with the frenzy charges' duration.
- **Immunities on you** (`ailment-on-self`): to chill, freeze, and stun.
- **Other lines no slice field reads** (`no-slice`):
  - knockback chance, frenzy charge duration, mana gained on kill;
  - chill duration on enemies (the chill's effect is its own);
  - pierce and chaining range (unparsed in PoB);
  - the atlas's wisps and a mercenary follower.
- **"Projectiles have #% chance for an additional Projectile when Forking"** (`projectile-count`): PoB reads it as `ProjectileCount`, which reaches a slice field only with Barrage's `SequentialProjectiles`; that stat guards it, as the other projectile counts.

**Evidence** (`b-ct`, against P6-11ea's run): public and corpus 0 unexpected; no value moved. The workspace tests, clippy, and rustfmt pass.

## P6-11ec: after a crit; against bleeding and maimed enemies

**PoB:**
- "#% increased Attack Speed if you've dealt a Critical Strike Recently" (`Speed` INC with `ModFlag.Attack`) and "+#% Chance to Block Attack Damage if you've dealt a Critical Strike Recently" (`BlockChance` BASE), under `CritRecently`.
- "#% increased Attack Damage against Bleeding Enemies" and "... against Maimed Enemies": `Damage` INC with `ModFlag.Attack` and the enemy's `Bleeding` or `Maimed`. Only the config checks set them (`conditionEnemyMaimed`, `ConfigOptions.lua` 1825).
- "#% increased Accuracy Rating per Frenzy Charge": `Accuracy` INC per `Multiplier:FrenzyCharge`.
- "#% increased Chaos Damage over Time": `ChaosDamage` INC with `KeywordFlag.ChaosDot`.

**The engine:**
- **The enemy's `Maimed`:** `conditions::ENEMY_MAIMED` from its check through `CHECKS`; it left `config::NAMED_ONLY`.
- **Rows:** four `STATE_ROWS` rows, `charges::PerCharge::Accuracy` with its `MODIFIERS` row, and a `HIT_ROWS` row for the chaos damage over time, as the burning damage row.

**Fixture** `smite-crit-recently-bleeding-maimed-templar.xml`: Smite with two frenzy charges, the three checks on, and the five lines. Every computed field equals PoB's but Q11's attack rate (DIV-0472 to DIV-0477).

**Evidence** (`b-ct`, against P6-11eb's run):
- **Public:** 0 unexpected. 17 values moved, 16 toward PoB: a Berserker's and a Slayer's damage, and the Slayer's spell block (25 to 35, against PoB's 64), attack rate, and EHP.
- **One moved away:** the Slayer's mana cost per second follows its attack rate, which moved toward PoB. PoB's cost there is 0, which the engine does not model yet.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11ed: two-handed damage; crit multiplier near a rare or unique enemy

**PoB:**
- "#% increased Damage with Two Handed Weapons": `Damage` INC with `ModFlag.Weapon2H` and `Hit`.
- "+#% Critical Strike Multiplier while a Rare or Unique Enemy is Nearby": `CritMultiplier` BASE with the enemy's `NearbyRareOrUniqueEnemy` or `RareOrUnique`, the condition P6-10b settles.

**The engine:**
- **Rows:** a `HIT_ROWS` row and a `STATE_ROWS` row.
- **`NoEffect` (`display`):** gem experience (unparsed in PoB), extra gore, ignite's visual variant, and a dummy display stat.

**Not done:** "#% increased Damage taken while Leeching" is unparsed in PoB, so it applies nothing there; the game applies it. It stays a reason (tasks file).

**Fixture** `smite-two-handed-rare-nearby-templar.xml`: Smite with its two-handed Siege Axe and both lines; PoB's default enemy is a boss, so rare or unique. Every computed field equals PoB's but Q11's attack rate (DIV-0478 to DIV-0483).

**Evidence** (`b-ct`, against P6-11ec's run):
- **Public:** 0 unexpected; 5 values toward PoB (a mine Assassin's crit multiplier and damage), none away.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11ee: mines' crits and penetration; movement speed; fourteen `NoEffect` lines

**PoB:**
- **Mines:** "#% increased Critical Strike Chance with Mines", "+#% to Critical Strike Multiplier with Mines", and "Mine Damage Penetrates #% Elemental Resistances" are `CritChance` INC, `CritMultiplier` BASE, and `ElementalPenetration` BASE with `KeywordFlag.Mine`.
- **Movement speed:** "... Movement Speed if you've dealt a Critical Strike Recently" and "... while holding a Shield" are `MovementSpeed` INC under `CritRecently` and `UsingShield`.
- "#% increased maximum Life, Mana and Global Energy Shield" is `Life`, `Mana`, and `EnergyShield` INC.

**The engine:**
- **Rows:** three `HIT_ROWS` rows, two `STATE_ROWS` rows, and a `ROWS` row.
- **`NoEffect`** (`no-slice`): an item's attribute requirements, flask automation and on-use effects, maim's effect, a mercenary's unique slots, a corpse's life, minions' taunt and defensive stance.

**Evidence** (`b-ct`, against P6-11ed's run):
- **Public:** 0 unexpected. 32 values moved, 31 toward PoB: a mine Assassin's crits and damage, a Berserker's movement speed, and a Flicker Strike Berserker's life, mana, energy shield, and defences.
- **One moved away:** that Berserker's energy shield recharge (90 to 106, against PoB's 0) follows its energy shield, which moved toward PoB. PoB's recharge is 0 for a reason the engine does not model yet.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11ef: cold conversion, penetration, added fire and cold to spells and attacks, throwing and totem cast speed, wand damage per power charge

**PoB:**
- "#% of Physical Damage Converted to Cold Damage" is `PhysicalDamageConvertToCold` BASE, as the fire and lightning ones.
- **Penetration:** "Attack Damage Penetrates #% of Enemy Elemental Resistances" and "Damage Penetrates #% of Enemy Elemental Resistances" are `ElementalPenetration` BASE, the first with `ModFlag.Attack`.
- **Added damage:** "Adds # to # Fire Damage to Spells and Attacks" and its cold twin are `FireMin`/`FireMax` (`ColdMin`/`ColdMax`) BASE with `KeywordFlag.Attack` or `Spell`. The engine gives each line an attack row and a spell row; no skill is both, so no skill gets it twice.
- **Speeds:** "#% increased Trap and Mine Throwing Speed" is `TrapThrowingSpeed` and `MineLayingSpeed` INC. "Spells Cast by Totems have #% increased Cast Speed" is `Speed` INC with `ModFlag.Spell`, `Cast`, and `KeywordFlag.Totem`.
- "#% increased Wand Damage per Power Charge" is `Damage` INC with `ModFlag.Wand` and `Hit` per `Multiplier:PowerCharge`.

**The engine:**
- **Rows:** nine `HIT_ROWS` rows; `poe1::charges` gains `PerCharge::WandDamage` and its modifier row.
- **`NoEffect`:**
  - `area`: area of effect for melee skills, warcries, while wielding a two handed melee weapon, and per power charge. The per-charge line is listed in `charges`' checked `NoEffect` rows.
  - `no-slice`: the trap and mine limits (`ActiveTrapLimit`, `ActiveMineLimit`). PoB shows them, and only its "for each trap" and "for each mine" lines read them; no slice field does.
- **Left as reasons:**
  - Flask duration, local and global. A recovery flask's recovery reads it (`poe1::flasks`, `DURATION_STATS`).
  - "#% increased Area Damage while wielding a Two Handed Melee Weapon". It needs two conditions at once (`UsingTwoHandedWeapon` and `UsingMeleeWeapon`), which a state row does not hold yet.
  - Curse cast speed and travel skills' cooldown. The engine has no curse or travel skill flag yet.
  - "#% increased Damage per Summoned Golem". Found on the way: PoB's `PerStat` reads `output.ActiveGolemLimit` (`ModStore.lua` 316), and no PoB module sets that output, so the line adds nothing in PoB. It stays guarded by `NO_EFFECT_DEPENDS_ON` (P3-8n).

**Evidence** (`b-ct`, against P6-11ee's run):
- **Public:** 0 unexpected. 26 values moved, all toward PoB: the speed and damage of a mine Assassin and a Holy Flame Totem Hierophant, and the damage of a ballista Hierophant, a Shock Nova Hierophant, a Kinetic Blast Necromancer, and a cast-on-crit build.
- **Corpus:** 0 unexpected. 13 values moved, 12 toward PoB.
- **One moved away:** a corpus build's poison DPS (179 to 143, against PoB's 548). Its physical damage is now partly converted to cold, which leaves less physical damage to poison; PoB converts it the same way. PoB's higher poison was a gap before this step, from another cause.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11eg: crits by whether you have blocked recently; movement speed during a flask effect; nineteen `NoEffect` lines

**PoB:**
- "#% increased Critical Strike Chance if you haven't Blocked Recently" is `CritChance` INC under `BlockedRecently` negated. "+#% to Critical Strike Multiplier if you have Blocked Recently" is `CritMultiplier` BASE under `BlockedRecently`.
- "#% increased Movement Speed during any Flask Effect" is `MovementSpeed` INC under `UsingFlask`.

**The engine:**
- **Condition:** `conditions::NOT_BLOCKED_RECENTLY`, the negation of `BLOCKED_RECENTLY` (the config's three blocked checks), settled only when that is. It is built as `NOT_HIT_RECENTLY` is (P6-10b part 13).
- **Rows:** three `STATE_ROWS` rows. The movement speed row took effect only in P6-11ei: the stat was also in `MOVEMENT_REASONS`, a field marker the mapping reads first ("P6-11ei").
- **`NoEffect`:**
  - `charge-gain-chance`: chances to gain an endurance or power charge on block, and a power charge on a non-critical strike. PoB does not parse them; the counts are the config's.
  - `flask`: flask charges gained, used, and generated every 3 seconds. No slice field reads a flask's charges.
  - `recovery`: mana and energy shield recovered on kill.
  - `no-slice`: taunt duration, Desecrate's and Unearth's corpse limit, totems' life and elemental resistances, attribute requirements (global, and a local Intelligence requirement), reflected damage taken.
  - `display`: the sell price, modification while corrupted, the implicit limit, and a wand base's crafting rule.
- The six lines that name a charge are listed in `charges`' checked `NoEffect` rows.
- **Left as reasons:** PoB does not parse "#% increased Spell Damage during any Flask Effect" or "+# to Accuracy Rating per Frenzy Charge". Each needs a game check before a row. "#% increased Movement Speed while Ignited" needs the character's own `Ignited` condition, which no settled condition gives yet.

**Evidence** (`b-ct`, against P6-11ef's run):
- **Public:** 0 unexpected. 5 values moved, all toward PoB: a General's Cry Perforate Berserker's crit multiplier (5.44 to 5.74, PoB 6.25) and the damage that follows.
- **Corpus:** 0 unexpected. 7 values moved, all toward PoB: another Berserker's crit multiplier, damage, and leech.
- **Not exercised:** no build has the crit chance line with its blocked check settled false, so `NOT_BLOCKED_RECENTLY` moved no value. The state rows test checks the row and its condition.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11eh: traps' and totems' crits; traps and mines per charge; eighteen `NoEffect` lines

**PoB:**
- **Crits:** "#% increased Critical Strike Chance with Traps" and "+#% to Critical Strike Multiplier with Traps" are `CritChance` INC and `CritMultiplier` BASE with `KeywordFlag.Trap`. "#% increased Critical Strike Chance with Totem Skills" is `CritChance` INC with `KeywordFlag.Totem`.
- **Per charge** (`SkillStatMap.lua` 1792 to 1843): trap throwing speed and mine throwing speed per frenzy charge (`TrapThrowingSpeed`, `MineLayingSpeed` INC); trap crit multiplier and mine crit chance per power charge (`CritMultiplier` BASE with `KeywordFlag.Trap`, `CritChance` INC with `KeywordFlag.Mine`).

**The engine:**
- **Rows:** three `HIT_ROWS` rows. `poe1::charges` gains four `PerCharge` targets (`TrapThrowingSpeed`, `MineLayingSpeed`, `TrapCritMultiplier`, `MineCritChance`) and their modifier rows.
- **`NoEffect`:**
  - `charge-gain-chance`: chances to gain a charge on casting a curse, on hitting a unique, rare, or marked enemy, and on blocking a spell; the chance to lose a frenzy charge on a travel skill. PoB parses none of them; the counts are the config's. All six are listed in `charges`' checked `NoEffect` rows.
  - `recovery`: energy shield recovered on block.
  - `no-slice`: totems' range and placement speed (`TotemPlacementSpeed`, which only PoB's own output reads), stun avoidance on kill, a rare monster's mods on kill, blight towers' range, strike skills' targeting distance, a projectile's contact position, reflected cold damage, and totems' taunt.
  - `display`: Blight's stack limit.
  - `projectile-count`: "#% increased Projectile Fork angle". PoB parses it as `ProjectileCount` INC (from "Projectile" in its text), which no slice field reads either.
- **Left as reasons:** the keystones Iron Reflexes, Chaos Inoculation, Pain Attunement, Acrobatics, and Lightning Purist (one build each). Each changes defences or needs a condition (`LowLife`) the engine does not settle yet; each is its own step. Crit chance against shocked enemies and against enemies with lightning exposure needs the enemy's shock and exposure as conditions.

**Evidence** (`b-ct`, against P6-11eg's run):
- **Public:** 0 unexpected. 26 values moved, all toward PoB, and one is now equal: a Lightning Trap Elementalist's crit multiplier (3.53 to 4.79, equal to PoB). The others are that build's, a mine Assassin's, a Holy Flame Totem Hierophant's, and a ballista Hierophant's crit chance, speed, and damage.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, clippy, and rustfmt pass.

## P6-11ei: Lightning Purist and Pain Attunement; low life and ignited; field markers checked first

**PoB:**
- **Lightning Purist** (the game's stat name for Roiling Tempest): "25% more Maximum Lightning Damage" (`MaxLightningDamage` MORE 25), "50% less Minimum Lightning Damage" (`MinLightningDamage` MORE -50), and "Cannot deal non-Lightning Damage" (`DealNoPhysical`, `DealNoCold`, `DealNoFire`, `DealNoChaos`; `ModCache.lua` 5945, 8049).
- **Pain Attunement:** "30% more Spell Damage when on Low Life" is `Damage` MORE 30 with `ModFlag.Spell` under `Condition:LowLife`.
- **Conditions:** PoB sets `LowLife` only from the `conditionLowLife` check (`ConfigOptions.lua` 177), and `Ignited` only from `conditionIgnited` (1181). Life reserved past the threshold (`CalcPerform.lua` 546) reads `LowLifePercentage` for display only.
- "#% increased Movement Speed when on Low Life" and "... while Ignited" are `MovementSpeed` INC under those conditions.

**The engine:**
- **Conditions:** `conditions::LOW_LIFE` and `conditions::IGNITED` are settled from their checks (`CHECKS`). The two inputs leave `config::NAMED_ONLY`, as `conditionEnemyMaimed` did in P6-11ec.
- **Keystones:** `keystone_lightning_purist` (`BoundDamage` of lightning, max and min, and the four `DealNo` flags) and `keystone_pain_attunement` (spell `Damage` MORE under `LOW_LIFE`) in `poe1::keystones`. Their texts are in `tests/keystones_chill.rs`, checked against the real bundle.
- **Rows:** two `STATE_ROWS` rows, movement speed on low life and while ignited.
- **A fix to P6-11eg:** its row for "#% increased Movement Speed during any Flask Effect" never applied. The stat was also in `MOVEMENT_REASONS`, and the mapping reads field markers before any table. The three movement stats now leave `MOVEMENT_REASONS`. A test now checks that no `STATE_ROWS`, `HIT_ROWS`, or `ROWS` stat is a field marker.
- **A test fixed:** the real-bundle test `real_keystone_and_chill_stats_should_be_covered` failed since P6-11eb on two `NoEffect` lines whose text names chill: chill immunity on you and the chill's duration. Neither changes the chill's effect; both are now listed in its checked `NoEffect` list.
- **Left as reasons:** Iron Reflexes, Chaos Inoculation, and Acrobatics, which change defences.

**Evidence** (`b-ct`, against P6-11eh's run):
- **Public:** 0 unexpected. 6 values moved, 5 toward PoB: a Lightning Trap Elementalist's damage (Lightning Purist) and a mine Assassin's effective movement speed.
- **One moved away:** that Assassin's `MovementSpeedMod` (2.93 to 3.01, against PoB's 2.79). It now carries The Arkhon's Tools' "8% increased Movement Speed during any Flask Effect": the build's flasks are active, and PoB applies the line too (`UsingFlask`). The engine was 0.14 above PoB before this step, for another reason.
- **Not exercised:** no build sets `conditionLowLife` or `conditionIgnited`, so Pain Attunement and the two movement rows moved no value. The keystone and state rows tests check them.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ej: Chaos Inoculation

**PoB:**
- The tree's "Maximum Life becomes 1, Immune to Chaos Damage" is `ChaosInoculation` FLAG and `ChaosDamageTaken` MORE -100 (`ModParser.lua` 2242).
- With the flag, `output.Life = 1` and `condList["FullLife"] = true` (`CalcPerform.lua` 79 to 83).
- Life gained as energy shield or armour adds 1 (`CalcDefence.lua` 1013, 1027).
- The stun threshold reads life before the keystone (2121), which is no slice field.

**The engine:**
- **Keystone:** `keystone_chaos_inoculation` in `poe1::keystones`: the new flag `Poe1Quantity::ChaosInoculation` and `SelfDamageTaken(Always, Chaos)` MORE -100, which damage taken over time and the incoming hit already read.
- **Life:** with the flag on, life is the flag's node times 1. The row of life gained as energy shield is 1 too (`phases::life_as_energy_shield`). The engine does not model life gained as armour.
- **Full life:** `states` sets `FullLife` when an allocated, connected node carries the keystone's stat. The game grants the keystone from the tree alone, so from another source `FullLife` stays the config's.
- The keystone's text is checked against the real bundle (`tests/keystones_chill.rs`).

**Evidence** (`b-ct`, against P6-11ei's run): an Essence Drain Occultist with the keystone.
- **Public:** 0 unexpected. 14 values moved, 6 toward PoB and now equal: life 1, unreserved life 1, the life leech instance and rate (0.1, 0.2), and chaos damage taken from hits and over time (0).
- **Eight moved away:** the build's defence estimates (maximum hits taken, effective hit pool, number of hits). They were already far below PoB's before this step: its energy shield is 3,915 against PoB's 10,165, from its other unresolved lines (Quiet Might's keystone among them). Losing the 1,509 life the keystone removes lowers them further. Every one of them is a partial field.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ek: Iron Reflexes

**PoB:**
- The tree's "Converts all Evasion Rating to Armour. Dexterity provides no bonus to Evasion Rating" is `IronReflexes` and `NoDexBonusToEvasion` FLAG (`ModParser.lua` 2230).
- With `IronReflexes`, each armour piece's evasion and the global evasion base go to armour, times `calcLib.mod(modDB, slotCfg, "Armour", "Evasion", "ArmourAndEvasion", "Defences")` (`CalcDefence.lua` 919 to 923, 980 to 984). Evasion keeps nothing from them.
- With `NoDexBonusToEvasion`, Dexterity adds no evasion increase (`CalcPerform.lua` 509).

**The engine:**
- **Keystone:** `keystone_iron_reflexes` in `poe1::keystones`, two new flags (`Poe1Quantity::IronReflexes`, `NoDexBonusToEvasion`).
- **Defences** (`phases::primary_defence`): with the flag, evasion is 0 (the flag's node times 0). Armour gains a row per piece's evasion and one for the global evasion base, after its own rows as in PoB (`iron_reflexes_rows`). Dexterity's evasion bonus is skipped with the second flag.
- **The modifier counts each increase once.** The engine maps PoB's `ArmourAndEvasion` and `Defences` increases to both `Armour` and `Evasion`, so adding the two would count them twice. Each such row now also adds to a marker, `Poe1Quantity::ArmourAndEvasionShared`, and the modifier is armour's increase plus evasion's less the marker's. The rows: "#% increased Evasion Rating and Armour", "#% increased Global Defences", armour and evasion while leeching, defences while wielding a staff, and the Stone Golem's buff. The more product is armour's times evasion's.
- **Guard:** a mapping test checks that every row of `ROWS`, `STATE_ROWS`, and `HIT_ROWS` that increases both armour and evasion carries the marker, and that no row has a more of both. The gem tables' rows are checked by reading (the Stone Golem's is the only one).
- A piece's own increases (PoB's `SlotName` tag) of armour and of evasion are both in that piece's modifier.
- The keystone's text is checked against the real bundle (`tests/keystones_chill.rs`).

**Evidence** (`b-ct`, against P6-11ej's run): a Lacerate Gladiator with the keystone.
- **Public:** 0 unexpected. 8 values moved, all toward PoB, one now equal. Evasion 8,571 to 0 (PoB 0); armour 5,080 to 20,393 (PoB 37,055); physical damage reduction 32 to 66 (PoB 80); the physical hit multiplier and the defence estimates follow. The armour still differs because of the build's other unresolved lines (Supreme Prodigy's and the tinctures' keystones among them).
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11el: Acrobatics

**PoB:**
- The current tree's "Modifiers to Chance to Suppress Spell Damage instead apply to Chance to Dodge Spell Hits at 50% of their value" is `ConvertSpellSuppressionToSpellDodge` FLAG and `SpellSuppressionChance` OVERRIDE 0. "Maximum Chance to Dodge Spell Hits is 75%" is `SpellDodgeChanceMax` OVERRIDE 75 (`ModCache.lua` 9946).
- With the flag, spell dodge is half the suppression chance's `BASE` sum (`CalcDefence.lua` 1124). The effective spell dodge is that, less the enemy's `reduceEnemyDodge`, between 0 and the maximum, and 0 against `CannotBeDodged` (1164 to 1172). The chance not to be hit by a spell reads it (1650).

**The engine:**
- **Keystone:** `keystone_acrobatics` in `poe1::keystones`, the flag `Poe1Quantity::SuppressionToSpellDodge`.
- **Spell suppression** (`phases`): 0 with the flag.
- **Spell dodge** (`enemy_hit::spell_dodge`): half the suppression `BASE` sum, between 0 and 75, passed to `ehp::Mitigation`'s `spell_dodge`, which was 0 until now. The estimate already ported PoB's chance not to be hit with it. PoB's `CannotBeDodged` comes from a boss skill preset, already a reason of the estimate (`UNMODELLED`). Nothing the engine maps sets `reduceEnemyDodge`.
- The keystone's text is checked against the real bundle (`tests/keystones_chill.rs`).

**Evidence** (`b-ct`, against P6-11ek's run): a Mirage Archer Toxic Rain build (PoB's 3.13 test build) with the keystone.
- **Public:** 0 unexpected. 6 values moved, 3 toward PoB: the chance not to be hit (33.5 to 37, PoB 50.45), the effective hit pool, and the number of hits.
- **Three moved away:** spell suppression (14 to 0, PoB 14), the configured damage chance, and the mitigated hits. **A tree version difference:** the build's spec is on tree 3_13, before the keystone's 3.16 rework. PoB reads that tree's node: "30% Chance to Dodge Attack Hits. 50% less Armour, 30% less Energy Shield, 30% less Chance to Block Spell and Attack Damage" (`TreeData/3_13`). The engine calculates a spec with no tree pack on the data's tree (`docs/tree/tree-packs.md`), whose keystone is today's. Each is a partial field.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11em: attacked and cast a spell recently; spell block; five `NoEffect` lines

**PoB:**
- `AttackedRecently` and `CastSpellRecently` come from their checks (`ConfigOptions.lua` 1485, 1489). The main skill also sets them (`CalcPerform.lua` 240 to 245): a skill not triggered and not a trap, mine, or totem sets `AttackedRecently` with its `attack` flag, else `CastSpellRecently` with its `spell` flag. `UsedMinionSkillRecently` (P6-11cm) comes from the same block.
- "+#% Chance to Block Spell Damage if you've Cast a Spell Recently" is `SpellBlockChance` BASE under `CastSpellRecently`.

**The engine:**
- **Conditions:** `conditions::ATTACKED_RECENTLY` and `CAST_SPELL_RECENTLY` are settled from their checks, or set by the main skill. `used_minion_skill`'s reading of the main skill is now `main_skill_recently`, which gives the three together. The two inputs leave `config::NAMED_ONLY`. Their implied `UsedSkillRecently` is no row's.
- **Row:** one `STATE_ROWS` row, spell block after casting a spell.
- **`NoEffect`:**
  - `ailment-on-self`: ignite duration on you, and the chance to be frozen, shocked, and ignited. PoB parses neither. The second is listed in the shock tests' checked `NoEffect` rows (`damage_taken`, `tests/exposure_shock.rs`).
  - `recovery`: energy shield recovered when you take physical damage from a hit (PoB does not parse it).
  - `stun`: a flask's stun recovery while healing.
  - `charge-gain-chance`: losing a power charge on gaining Elusive.
- **Left as reasons:** the "you and nearby allies" lines under these conditions (PoB's `ExtraAura`, scaled by the aura effect on you), and attack speed per enemy in close range (a condition with a count).
- `loom-app`'s workflow test used `conditionCastSpellRecently` as an input with no effect; it now uses `conditionImpaledRecently`.

**Evidence** (`b-ct`, against P6-11el's run):
- **Public:** 0 unexpected. 5 values moved, all toward PoB: a Penance Brand Elementalist's spell block (71 to 78, equal to PoB) and configured damage chance (equal to PoB), and its defence estimates. Its main skill is a spell, so it has cast a spell recently, as in PoB.
- **Corpus:** 0 unexpected. 5 values moved, all toward PoB, two now equal: another Penance Brand build, the same fields.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11en: the hands' weapon types; two-handed area damage and impales

**PoB:** each hand's weapon type sets `UsingMeleeWeapon`, `UsingOneHandedWeapon`, or `UsingTwoHandedWeapon` from `data.weaponTypeInfo` (`CalcPerform.lua` 150 to 214; `Modules/Data.lua` 550 to 566). An empty main hand is the unarmed `None` type: melee and one-handed. The off hand counts only with a weapon in it.
- "#% increased Area Damage while wielding a Two Handed Melee Weapon" is `Damage` INC with `ModFlag.Area` under `UsingTwoHandedWeapon` and `UsingMeleeWeapon`.
- "#% chance to Impale Enemies on Hit with Two Handed Weapons" and "#% increased Effect of Impales you inflict with Two Handed Weapons" are `ImpaleChance` BASE and `ImpaleEffect` INC with `ModFlag.Weapon2H` and `Hit`.

**The engine:**
- **States** (`poe1::states`): `USING_MELEE_WEAPON`, `USING_ONE_HANDED_WEAPON`, and `USING_TWO_HANDED_WEAPON` from both hands, each hand typed as PoB's table (`hand_kinds`, with `skill::pob_weapon_type`). A hand whose item is not known, or a class PoB does not type, leaves all three unsettled. `USING_TWO_HANDED_MELEE_WEAPON` is both at once: the engine's key for PoB's two conditions on one modifier, since a state row reads one.
- **Rows:** a `STATE_ROWS` row for the area damage, two `HIT_ROWS` rows for the impales. The impale effect row is listed in `gems`' checked rows, as the plain impale effect is.
- **Left as a reason:** "#% increased Effect of Impales you inflict with Two Handed Weapons on Non-Impaled Enemies". PoB leaves "on Non-Impaled Enemies" unparsed and so skips the line (`Item.lua` 2504).

**Evidence** (`b-ct`, against P6-11em's run):
- **Public:** 0 unexpected. 3 values moved, all toward PoB: a Cyclone Slayer's impale DPS and the DPS totals on it.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** a unit test of the hand kinds; the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11eo: the curse keyword; curse skills' cast speed

**PoB:** a skill of `SkillType.AppliesCurse` gets `KeywordFlag.Curse` (`CalcActiveSkill.lua` 379). "Curse Skills have #% increased Cast Speed" is `Speed` INC with `ModFlag.Cast` and `KeywordFlag.Curse`.

**The engine:**
- **Flag:** `Flags::KEYWORD_CURSE`, set from the main skill's `AppliesCurse` type (`skill::SKILL_TYPE_FLAGS`; the real-bundle test checks the type name).
- **Row:** one `HIT_ROWS` row.
- **Left as a reason:** "#% increased Cooldown Recovery Rate of Travel Skills". The engine has no cooldown recovery quantity for the character's skills yet.

**Evidence** (`b-ct`, against P6-11en's run):
- **Public and corpus:** 0 unexpected; no value moved. No build's main skill is a curse, so the line only stops being a reason.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ep: totems summoned

**PoB:** when any active skill is a totem, `TotemsSummoned` is the config's count, else the main skill's `ActiveTotemLimit` and `ActiveBallistaLimit` summed (`CalcPerform.lua` 1334 to 1338). The limit is the character's `base_number_of_totems_allowed` (1, `CalcSetup.lua` 60), plus "+# to maximum number of Summoned Totems", plus the ballista limit with `SkillType.TotemsAreBallistae` and the attack keyword. Modifiers per summoned totem read it as a `PerStat`; without a totem skill it is unset, so 0.

**The engine:**
- **Stacks:** a new kind, `StackKind::Totems`, counted by `stacks::totems` after the aura and herald count and before the buffs. The count is the config's `TotemsSummoned`, else, with a totem skill (`gems::any_totem_skill`: the main group and the enabled groups, supports' types included), the limit above. The ballista limit counts only for a main skill of both types.
- **Per totem:** "#% more Damage per Summoned Totem" (Hierophant), mana and life regeneration per totem (life: the character's share), and Arcane Surge's effect per totem.
- **Limits:** "+# to maximum number of Summoned Totems" and "Attack Skills have +# to maximum number of Summoned Ballista Totems".
- The totem's own hit keeps its `totem-data` reason (P6-6d), so its DPS stays `Partial`.

**Evidence** (`b-ct`, against P6-11eo's run):
- **Public:** 0 unexpected. 13 values moved, all toward PoB. A ballista Hierophant's life regeneration went from 43.1 to 239.2 (PoB 315.6) and its mana regeneration from 42.5 to 103.2 (PoB 115.4); its damage and a Holy Flame Totem Hierophant's moved too.
- **Corpus:** 0 unexpected. 5 values moved, 2 toward PoB: one build's life and mana regeneration. **Three moved away:** that build's DPS, where PoB has 0: PoB computes no damage for its main skill, a gap from before this step.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11eq: "you and nearby allies" lines (PoB's `ExtraAura`)

**PoB:** a line "you and nearby Allies ..." is an `ExtraAura` list mod (`CalcPerform.lua` 3352 to 3362). Unless it is for allies only, its mod applies to you, scaled by `(1 + (BuffEffectOnSelf + AuraEffectOnSelf) INC / 100) x MORE`. A condition on the line is evaluated on you.

**The engine:**
- **Rows:** the mod on you at scale 1:
  - "You and nearby Allies deal #% increased Damage" (`Damage` INC), a `HIT_ROWS` row;
  - "You and nearby Allies have +#% to Elemental Resistances" (each elemental resistance BASE), a `ROWS` row;
  - "If you've Attacked Recently, you and nearby Allies have +#% Chance to Block Attack Damage" and "If you've Cast a Spell Recently, ... Spell Damage" (`BlockChance`, `SpellBlockChance` BASE under P6-11em's conditions), two `STATE_ROWS` rows.
- **The scale:** no stat the engine maps is `BuffEffectOnSelf` or `AuraEffectOnSelf`. "#% increased Effect of Buffs on you" stays a reason (`gems::DEPENDS_ON`, P6-11k), as does any other such stat. So the scale is 1 wherever the fields are complete, as for the gems' auras.
- **Left as reasons:** the Necromancer's lines with a nearby corpse (PoB's `NearbyCorpse` multiplier, unsettled) and Onslaught on warcry (`UsedWarcryRecently`, unsettled).

**Evidence** (`b-ct`, against P6-11ep's run):
- **Public:** 0 unexpected. 23 values moved, all toward PoB, 13 now equal:
  - an Animate Guardian Guardian's attack block (46 to 71) and spell block (51 to 78), its configured damage chance, and its mana regeneration;
  - a Forged Frostbearer Necromancer's fire, cold, and lightning resistances (59 or 60 to 80) and their hit and over-time damage taken;
  - both builds' defence estimates moved toward PoB.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11er: auras that also affect enemies

**PoB:** with `AurasAffectEnemies` ("Auras from your Skills which affect Allies also affect Enemies"), each aura buff that affects allies is also a debuff on the enemy (`CalcPerform.lua` 2451 to 2464). Its mods are scaled by the skill's `AuraEffect` and `BuffEffect` INC and MORE, without the terms for the effect on you. The enemy's resistance reads the debuff's `<Type>Resist`. Its maximum reads only the config and the constant (`calcResistForType`), so a maximum resistance mod does nothing there.

**The engine:**
- **Flag:** `Poe1Quantity::AurasAffectEnemies`, mapped from `base_aura_skills_affecting_allies_also_affect_enemies`.
- **`gems::auras_on_enemy`:** each aura of `EffectType::Aura` that does not itself affect enemies (`AuraAffectsEnemies`) is recorded with its first skill. With the flag, its merged rows go on the enemy: a resistance row as `EnemyResistance`, which the hit adds to the enemy's base; a maximum resistance row as nothing; any other row as an `aura-on-enemy` reason.
- **The scale:** the merged rows carry the character's scale. It equals the debuff's when the aura effect on you (`SkillAuraEffectOnSelf`) is 0; otherwise each aura is an `aura-on-enemy-scale` reason.
- **Unit test:** `ally_auras_should_go_on_the_enemy_with_the_flag` covers the flag, the resistance, the maximum, another row, and the effect on you.

**Evidence** (`b-ct`, against P6-11eq's run):
- **Public and corpus:** 0 unexpected; no value moved.
- **The three builds with the flag:**
  - A Penance Brand Elementalist now has Purity of Ice's cold resistance on the enemy, but deals no cold damage, so nothing moved.
  - A Facebreaker Luminary has no active ally aura: its Precision's group is not active in its item set.
  - The flag stat is no longer a reason in any of the three, a corpus build among them.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11es: Forbidden Flame and Forbidden Flesh

**PoB:** "Allocates <notable> if you have the matching modifier on Forbidden Flesh" (on Forbidden Flame) and "... on Forbidden Flame" (on Forbidden Flesh) are `GrantedAscendancyNode` with their side (`ModParser.lua` 5484). Two lines of opposite sides naming the same ascendancy notable allocate it when both `ForbiddenFlesh` and `ForbiddenFlame` equal the character's class (`CalcSetup.lua` 1299 to 1315). Each is the class restriction ("Requires Class") of the item of that title (1170).

**The engine:**
- **Lines:** `unique_jewel_grants_notable_hash_part_1` (naming Forbidden Flesh) and `_part_2`, read by their `passive_hash` handler's lookup, as "Allocates <notable>" is (`collect::hashes_of`). Each candidate hash is a `Poe1Quantity::ForbiddenNode(side)` effect.
- **`collect::forbidden_pairs`,** after the granted passives:
  - each line pairs with the first unpaired line of the other side whose candidates meet its own;
  - when both lines' items carry a selected "Requires Class <the character's class>" line, the one ascendancy notable among the shared candidates is allocated, its stats with the node as their source;
  - a node already allocated or granted counts once; an ambiguous name is indeterminate.

**Evidence** (`b-ct`, against P6-11er's run):
- **Public:** 0 unexpected. 11 values moved, 9 toward PoB, 5 now equal. A Poisonous Concoction Pathfinder's evasion went from 25,181 to 25,807, equal to PoB, and so did its evade chances (79 to 80) and its configured chance not to be hit. Its movement speed moved toward PoB.
- **Two moved away,** by about 1e-6: that build's number of damaging hits, an estimate already far below PoB's.
- The hash lines are no longer reasons in any of the five builds with the jewels.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11et: mines' cost as reservation; the main skill's reservation

**PoB:** every active skill that reserves, the main skill included, reserves in one loop (`CalcPerform.lua` 1940 on). A skill supported by Blastchain Mine, High-Impact Mine, or Locus Mine has `remote_mined_by_support` (`SkillStatMap.lua` 1838: `ManaCostGainAsReservation`, `LifeCostGainAsReservation`), so its flat reservation is `ManaCostBase` plus its level's mana cost, and the same for life (1946 to 1953). The game's own mine skills carry their reservation in their level data, equal to their cost.

**The engine:**
- **Mine supports:** `gems::COST_AS_RESERVATION_SUPPORTS`. With one of them, the level's mana and life costs replace the flat reservation of each pool. No row the engine maps gives `ManaCostBase` or `LifeCostBase`.
- **The main skill:** reserves as any skill (`gems::reservation`). Its `reservation` reason (P3-12h) is gone.
- **Test:** the hit test `a_main_skill_that_reserves_should_reserve` (was `..._should_be_a_reason`) checks that a reserving main skill has no reason and that its flat amount comes off the unreserved mana.

**Evidence** (`b-ct`, against P6-11es's run):
- **Public:** 0 unexpected. A mine Assassin's unreserved mana moved toward PoB (578 to 552, PoB 174), and its `reservation` reason, on 77 fields, is gone. The rest of the gap is the build's other reservation lines.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11eu: PoB-only skills; two investigations

**PoB-only skills.** PoB adds "On Kill Monster Explosion" (`EnemyExplode`, `other.lua` 6063: types `Damage` and `Area` only) as a granted group (`source="Explode"`) from the explode-on-kill modifiers. No game granted effect has the ID. It has no buff, reservation, or cost, so it reaches a field only as the main skill or in full DPS.
- **The engine:** `gems::POB_ONLY_INERT`. A granted group whose enabled gems are all such skills, not the main group and not in full DPS, is no `unmodelled-gem` reason; otherwise it stays one.
- **Evidence:** 0 unexpected, no value moved. In the six public builds with the group, the `unmodelled-gem` reasons on their fields fell (for example 316 to 237 in a Raise Spider Occultist).

**Autoexertion's reservation** (the community Slayer, PoB 118, the engine 126):
- The build's helmet (item 43) has a `{custom}` line, "(9-10)% increased Mana Reservation Efficiency of Skills", beside its 11% implicit.
- PoB applies `{custom}` item lines; the engine excludes them (`Exclusion::Custom`). So PoB's 21% efficiency follows, and the reservation code is right.
- It waits on the maintainer's decision on `{custom}` item lines (tasks file).

**PoB's full DPS with Blood and Sand** (Smite's area hit differs slightly between PoB's main calculation and its full DPS pass):
- `calcs.calcFullDPS` makes one environment and calls `calcs.perform` for each included skill without resetting `env.modDB`, so a later pass sees mods an earlier one added with `NewMod`.
- Blood and Sand's stance buffs (`effectCond` `BloodStance`) are the likely carriers. Naming the mod needs an instrumented PoB run.
- It stays open: no compared build has both.

**Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ev: nearby corpses; used a warcry recently

**PoB:**
- `Multiplier:NearbyCorpse` is the config's `multiplierNearbyCorpse` (`ConfigOptions.lua` 1125). The Necromancer's lines read it:
  - "With at least one nearby corpse, you and nearby Allies deal #% more Damage": `ExtraAura` `Damage` MORE, `MultiplierThreshold` 1;
  - "... nearby Enemies deal #% reduced Damage": the enemy's `Damage` INC, threshold 1;
  - "For each nearby corpse, you and nearby Allies Regenerate #% of Energy Shield per second, up to 2.0% per second": `ExtraAura` `EnergyShieldRegenPercent` BASE per corpse, `limit` 2, `limitTotal`.
- `UsedWarcryRecently` comes from `conditionUsedWarcryRecently`, or from a `multiplierWarcryUsedRecently` above 0 (1558, 1612). "When you Warcry, you and nearby Allies gain Onslaught for 4 seconds" is `ExtraAura` `Onslaught` FLAG under it.

**The engine:**
- **Corpses:** `StackKind::NearbyCorpses`, counted by `stacks::nearby_corpses` from the config's count. The threshold lines apply once from 1 corpse; the regeneration totals at most 120% per minute. Its config input leaves `config::NAMED_ONLY` for `stacks::INPUTS`. The regeneration line leaves `mapping::REGEN_REASONS`.
- **Warcry:** `conditions::USED_WARCRY_RECENTLY`, from its check or the warcry count, as "killed recently" (P6-11p). The check leaves `NAMED_ONLY`. The count input keeps only the names no row reads.
- **Row:** Onslaught under it, a `STATE_ROWS` row that `poe1::buffs` applies.
- **Left as a reason:** "For each nearby corpse, you and nearby Allies Regenerate # Mana" (PoB does not parse it).

**Evidence** (`b-ct`, against P6-11eu's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected. 12 values moved, all toward PoB, 2 now equal. One build's physical damage reduction went from 78 to 80 (equal to PoB), and its maximum hits taken and hit counts moved toward PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ew: brands attached to the enemy

**PoB:** for each brand skill, the brands attached are its attach limit (`BrandsAttachedLimit`, from its gems' `base_number_of_sigils_allowed_per_target`) at most its active brand limit (`ActiveBrandLimit`: 3, `CalcSetup.lua` 529, plus "You can Cast an additional Brand"). The config's `BrandsAttachedToEnemy`, when set, replaces it, at most that limit. The enemy's `Multiplier:BrandsAttached` is the largest over the brand skills (`CalcPerform.lua` 1320 to 1332). "Enemies take #% increased Damage for each of your Brands Attached to them" reads it as the enemy's `DamageTaken` INC.

**The engine:**
- **Stacks:** `StackKind::BrandsAttached`, counted by `stacks::brands`, with the attach limit from `gems::brand_attach_limit` (the main group and the enabled groups' `Brand` skills). The active brand limit's extra is "You can Cast an additional Brand". A skill's own `ActiveBrandLimit` modifiers (a support's) are not read.
- **Per brand:** the enemy's damage taken (`EnemyDamageTaken` INC).
- **Config:** `BrandsAttachedToEnemy` joins `stacks::INPUTS`.
- **Left as a reason:** the config's `ActiveBrands` (PoB's `Multiplier:ConfigActiveBrands`), which no mapped line reads yet.

**Evidence** (`b-ct`, against P6-11ev's run):
- **Public and corpus:** 0 unexpected; no value moved.
- A Penance Brand Elementalist's line is no longer a reason. Its count, checked with a debug run, is 1: the config's 2, at most Penance Brand's attach limit of 1. The build's own damage fields are not computed (`unsupported`), so nothing moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ex: cooldown recovery on the main skill's cooldown; travel skills

**PoB:** `calcSkillCooldown` (`CalcOffence.lua` 274 to 288) divides a skill's cooldown by `calcLib.mod(skillModList, skillCfg, "CooldownRecovery")` before it rounds it up to a server tick. The rate is then at most `1 / cooldown`. "#% increased Cooldown Recovery Rate of Travel Skills" is `CooldownRecovery` INC with `SkillType.Travel`.

**The engine:**
- **A fix:** the main spell's cooldown cap (`hit::cooldown_limit`, P6-7c3b) read the level's cooldown alone. Its docs said no cooldown recovery stat was mapped, which stopped being true in P6-11bs, when "#% increased Cooldown Recovery Rate" became `SkillTiming::CooldownRecovery` for the warcries. A cooldown spell with that line was capped at the unrecovered cooldown, with no reason. The cap now divides by `1 + inc / 100`: the global recovery plus the typed one. A recovery of -100% or less is a `cooldown-recovery` reason.
- **Travel skills:** `Flags::SKILL_TRAVEL` from the main skill's `Travel` type (`skill::SKILL_TYPE_FLAGS`). The travel line is `Poe1Quantity::TypedCooldownRecovery` with that flag. It is a quantity of its own because `skill_modifiers::Totals` counts any conditional `SkillTiming` effect as unread.
- **Test:** `cooldown_recovery_should_shorten_the_cooldown_cap`. Ice Nova at a 0.5 s cast with a 2 s cooldown is capped at the tick-rounded 1/2.013 per second; with 100% increased cooldown recovery, at 1/1.023 per second.

**Evidence** (`b-ct`, against P6-11ew's run):
- **Public and corpus:** 0 unexpected; no value moved. No build's computed main skill is a cooldown spell with a recovery line.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ey: mines' reservation efficiency; High-Impact Mine's damage

**PoB:**
- "#% increased Mana Reservation Efficiency of Skills that throw Mines" is `ManaReservationEfficiency` INC with `KeywordFlag.Mine`, which a mined skill has.
- High-Impact Mine (`sup_int.lua`, `SupportHighImpactMine`) maps `support_remote_mine_2_damage_+%_final` to `Damage` MORE: -55% at level 20 in PoB's table and the game's.
- Its double damage near mines is an `AuraDebuff` on the enemy, stacked per active mine (`effectStackVar` `ActiveMineCount`), at most 100%.

**The engine:**
- **Reservation:** `Poe1Quantity::MineManaReservationEfficiency`, a `skill_modifiers::STATS` row. `Totals::reservation_for` adds it for a `RemoteMined` skill, as it adds a herald's for a `Herald` skill (P6-11ag).
- **Damage:** a `HIT_ROWS` row for High-Impact Mine's more damage.
- **`NoEffect` (`no-slice`):** the mine supports' own stats: thrown as a mine (the types carry it), the cost as reservation (read by the support's ID, P6-11et), the mine's duration, detonation time, and PvP scaling. No PoB calc module reads them.
- **Texts:** the efficiency line is checked against the real bundle (`tests/skill_modifiers.rs`), and the cost-as-reservation line is listed there.
- **Left as reasons:**
  - the double damage near mines (the enemy's debuff per active mine);
  - "Mines have #% increased Detonation Speed" and "... a #% chance to be Detonated an Additional Time" (PoB leaves both unparsed);
  - the crits per mine detonated (PoB leaves "for each Mine Detonated" unparsed and skips the line);
  - the effect of auras from mines.

**Evidence** (`b-ct`, against P6-11ex's run):
- **Public:** 0 unexpected. **5 values moved away,** all in a mine Assassin.
  - Its damage (240,503 to 108,224 average, PoB 3,074,566) now carries High-Impact Mine's 55% less damage, which PoB applies too. The engine was already far below PoB; the cause of the gap is elsewhere: the double damage near mines adds nothing in PoB for this build (P6-11ez).
  - Its unreserved mana (552 to 560, PoB 174) now has the mine efficiency, which PoB applies too. The gap is its other reservations.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ez: High-Impact Mine's double damage near mines

**PoB:** the double damage near mines is an `AuraDebuff` on the enemy whose value is multiplied by the active mine count (`effectStackVar` `ActiveMineCount`, `CalcPerform.lua` 3800 to 3830). That count is the gem's own `skillMineCount`, set in PoB's skill tab (`CalcActiveSkill.lua` 612 to 617). Without it the stack count is 0 and PoB removes the debuff, so the line gives nothing.

**The engine:**
- **`NoEffect` (`no-slice`):** `support_remote_mine_2_chance_to_deal_double_damage_%_against_enemies_near_mines`.
- **Reason:** in `gems`, a main skill supported by High-Impact Mine whose active gem or a support carries a `skillMineCount` above 0 is an `active-mine-count` reason. The count is kept with the gem's other attributes on import.
- **A correction to P6-11ey:** its evidence said the double damage and mine count were the bulk of the mine Assassin's gap to PoB. That build has no `skillMineCount`, so PoB adds nothing from them; the gap's cause is elsewhere (tasks file).

**Evidence** (`b-ct`, against P6-11ey's run):
- **Public and corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fa: trap and mine throw counts; the trap and mine reason retired

**PoB** (`CalcOffence.lua` 1230 to 1237, 1319 to 1336):
- A trap's count is `calcLib.val(TrapThrowCount)`, the base 1 (`CalcSetup.lua` 528) plus "+# Traps thrown" and Swift Assembly's expected extra. The config's `trapsPerThrow` overrides it. The time is the throwing time divided by the count.
- A mine's count works the same way (`minesPerThrow`). The laying speed is first divided by `1 + (count - 1) x 0.1`, then capped at the tick rate; a trap thrown by a mine divides by both counts.
- A skill with a cooldown throws one, but none of the engine's computed traps or mines has one (P6-6e), and no skill of PoB's data has a `trapCooldown`.

**The engine:**
- **Counts:** `Poe1Quantity::MineThrowCount` and `TrapThrowCount` (BASE). Rows: "+# Mines thrown", "+# Traps thrown", Swift Assembly's three chances (scaled 1/100, 2/100, 3/100, PoB's `div`), and "Throw # additional Traps while Dual Wielding" (a state row). The config's two inputs are read by `hit::throw_count` and classed as applied.
- **The rate** (`hit::cast_rate`): the capped throwing speed times the counts, a mine's speed divided by the extra-mine penalty first.
- **The reason retired:** `trap-mine-data` (P6-6e) stood for the throw count, the throwing speed, the cooldown, and the keyword mods. The speed (P6-11cy) and the keywords were read since, and the cooldown cannot occur. With the counts read, no trap's or mine's hit carries the reason.

**Evidence** (`b-ct`, against P6-11ez's run):
- **Public:** 0 unexpected. **7 fields went from partial to complete, each equal to PoB:** the Ice Nova trap fixture's average damage, speed (2.16), crit chance and multiplier, and DPS (2,200.10).
- A mine Assassin's speed went from 5.9 to 21.07 (PoB 18.61) with its five mines per throw, and its DPS moved toward PoB. The speed's remaining difference is its other mine laying speed lines, still reasons.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fb: the timeless additions' remaining lines

Every stat of the timeless data (`poe1/timeless.json`: 395 stats, the conquests' and the Abyss additions') was run through the mapping. Each unmapped one was rendered and parsed by PoB's own `ModParser` (`local/p6/parse2`), as P6-11ba did.

**Mapped:**
- **Spell added damage while holding a Shield or wielding a Two Handed Weapon**, each type's minimum and maximum (20 `STATE_ROWS` rows): `<Type>Min`/`Max` BASE with `KeywordFlag.Spell`, under `UsingShield` (PoB's "while holding a (%w+)", a `varList`) or `UsingTwoHandedWeapon` (settled since P6-11en). "Adds # to # Physical Damage to Spells", unconditional (two `HIT_ROWS` rows).
- **Conditional lines** (`STATE_ROWS`):
  - damage over time with a two-handed weapon;
  - crit chance if you haven't crit recently, under the new `conditions::NOT_CRIT_RECENTLY`;
  - crit multiplier and damage if you've killed recently;
  - elemental penetration if you haven't killed recently, under the new `NOT_KILLED_RECENTLY` (each the negation of its check, settled when it is, as `NOT_BLOCKED_RECENTLY`);
  - life regeneration and mana regeneration rate while moving;
  - movement speed if you haven't taken damage recently (PoB's `BeenHitRecently` negated);
  - Onslaught on kill (`KilledRecently`, as P6-11p's chance).
- **Others:** "#% increased Projectile Attack Damage" (`ModFlag.Attack` and `Projectile`); "#% increased Energy Shield per Power Charge" (`PerCharge::EnergyShield`).
- **Devotion readers at 150:** the three minimum charges and physical damage converted to cold (`poe1::devotion::READERS`).
- **`NoEffect`:** curse skills' duration, Phasing on kill (no row reads `Phasing`), elemental ailment immunity on consecrated ground.

**Left as reasons**, each for its own cause:
- the ward lines (no ward pool);
- the minions' ailment chances, life regeneration, and speed (next);
- the enemy-side lines (blinded, hindered, intimidated, maimed, withered);
- "additional Physical Damage Reduction if you weren't Damaged by a Hit Recently" (PoB leaves "if you weren't Damaged by a Hit Recently" unparsed and skips the line);
- the plain exposure chances and mana as energy shield at 150 Devotion (no target stat yet);
- the per-10-Devotion aura effect and mana (P6-11az's reasons);
- flask duration (P6-11ef);
- the Abyss and other keystones.

**Evidence** (`b-ct`, against P6-11fa's run):
- **Public and corpus:** 0 unexpected; no value moved. No build has an Abyss jewel, and the conquests' lines are not on any build's nodes. The state rows test checks every new row and its condition.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fc: minions' ailment chances

**PoB:** "Minions have #% chance to Ignite", "... to cause Bleeding with Attacks", and "... to Poison Enemies on Hit" are `MinionModifier` lists of `EnemyIgniteChance`, `BleedChance` (with `KeywordFlag.Attack`), and `PoisonChance` BASE: the minion's own chances. "Minions Regenerate # Life per second" is a `MinionModifier` of `LifeRegen` BASE.

**The engine:**
- **`MinionValue::AilmentChance(ailment)`:** on the minion, its `AilmentChance` (bleed with the attack keyword), which the minion's ailments read since P6-7c3d. Three rows in `minions::STATS`, each text checked by `tests/minions.rs` against the game's.
- **`NoEffect` (`minion-regen`):** the minions' flat life regeneration. No minion field reads regeneration, as for the percent form.
- **Left as a reason:** "Minions have #% increased Attack and Cast Speed if you or your Minions have Killed Recently" (a condition on the minion).

**Evidence** (`b-ct`, against P6-11fb's run):
- **Public and corpus:** 0 unexpected; no value moved. No build's minions carry these lines.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fd: Seismic, Ancestral, and Infernal Cry; two `data-parity` fixes

**PoB** (`act_str.lua`): each is a warcry whose global mods are `Warcry` effects scaled by warcry power:
- **Seismic Cry:** `Armour` MORE per 5 power up to 25; `StunThreshold` INC.
- **Ancestral Cry:** `ElementalResist` BASE per 5 power up to 30; `ElementalResistMax` BASE per 10 power up to 30.
- **Infernal Cry:** `PhysicalDamageGainAsFire` BASE per 5 power up to 25.

Their exerted attacks' mods (`SeismicExertedAttacks`, `AncestralExertedAttacks`, `InfernalExertedAttacks`, Seismic's slam area) act on the skill's own list, not as global mods, and reach no slice field.

**The engine:**
- **Gems:** three `gems::Kind::Warcry` entries with `BuffStat::PerPower` rows, as Battlemage's Cry (P6-11dx). Seismic's stun threshold and the three warcries' power from enemies are `NoSlice`.
- **`data-parity` (`gem_effects`):** the three warcries' `Buff` and `Warcry` identities join `EXTRA_BUFFS`, as the other warcries'. `StunThreshold` joins `NO_SLICE_NAMES`. `collapse` writes the three maximum resistances as `ElementalResistMax`.
- **A fix to P6-11ek:** `collapse` now skips the shared armour and evasion marker. The Stone Golem's buff row has carried it since P6-11ek, and P6-11ek did not run `data-parity`, so its `Defences` row showed as a difference. `data-parity`: `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fc's run):
- **Public:** 0 unexpected. A Boneshatter Juggernaut's unreserved mana went from 198 to 66, equal to PoB: its Autoexertion now reserves for the newly modelled warcries.
- **One moved away:** a Facebreaker Luminary's unreserved mana (14 to -80, PoB 42) carries its warcries' reservation too. Its helmet's `{custom}` lines, a 9-10% mana reservation efficiency and more maximum mana, are applied by PoB and excluded by the engine, as for the Slayer (P6-11eu). The `{custom}` decision settles it.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests (the `data-parity` tool's included), loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fe: skills an item grants

**PoB** (`CalcSetup.lua` 1346 to 1402, 1686 to 1710):
- A skill an item grants ("Grants Level # <skill>", "Curse Enemies with <curse> on Hit", an enchantment's trigger) gets a socket group rebuilt from its grant. That is one gem, the active skill, at the granted level, with quality 0 and `enableGlobal1` on.
- Unless the grant says `noSupports`, every support socketed in the item's slot, in any of that slot's other groups, joins it.
- The build file keeps the group with its `source` (`Item:<id>:<name>`), but not the grant's `noSupports`.

**The engine:**
- **`gems::granted_view`:** an item's group whose slot has no support gem in another group is now that rebuilt group, processed as any other (`one_skill`): its buffs, curses, and reservation apply. With no support in the slot, PoB's support question cannot arise.
- **Still reasons (`unmodelled-gem`):** an item's group whose slot holds supports (the build file does not say whether they join it), and every other source (the tree, PoB's own groups, P6-11eu's rule aside).
- **Test:** `a_group_with_a_source_should_be_modelled_or_a_reason` (was `..._should_be_a_reason`): an item's Precision alone in its slot is modelled with no reason; with a support socketed in the slot, or from the tree, it stays a reason.

**Evidence** (`b-ct`, against P6-11fd's run):
- **Public:** 0 unexpected. 24 values moved, 21 toward PoB:
  - an Elemental Hit Deadeye's hit chance (62 to 90, PoB 100), crit chance, damage, and unreserved mana (424 to 252, PoB 152), from its item's Precision;
  - a Juggernaut's life regeneration;
  - a Cyclone Slayer's and a Lightning Trap Elementalist's damage.
- **Three moved away:** a cast-on-crit build's average damage and DPS (its crit chance, multiplier, and speed moved toward PoB). Its ring's Assassin's Mark now applies and takes part in the curse slots by PoB's rule (P6-9e) with the build's Frostbite. PoB's damage is ten times the engine's from other gaps, so this build does not settle that choice.
- **Reasons:** item-granted Precision and Vulnerability no longer appear among the unmodelled gems.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ff: an item's skills with the slot's supports

**PoB** (`ModParser.lua` 2046 to 2062, 3275 to 3367): the grant's wording sets `noSupports`.
- "Grants (Level #) <skill>", "Trigger(s) / Cast(s) Level # <skill> ...", and "Use Level # ..." take the slot's supports.
- "Curse Enemies with ..." and the triggered grants without a level ("Cast / Trigger <skill> on <event>", "... when Hit", "... when your Skills or Minions Kill", "... when you take a Critical Strike") are `noSupports`.

With supports, every support socketed in the slot's other groups joins the skill, the best of each kind (`CalcSetup.lua` 1690 to 1710).

**The engine:**
- **`gems::grant_supports`** reads the grant lines of the item the group's `source` names (`Item:<id>:...`), through `grant_kind`. When all agree, the group is built: `granted_view` alone for `noSupports`, `granted_view_with_supports` otherwise, with the slot's other gems after the active one; `skill::group_skills` keeps the supports that fit.
- **Still reasons:** an item with both kinds of grant line, or none, or one not found.
- **Test:** `grant_lines_should_say_whether_supports_join`.

**Evidence** (`b-ct`, against P6-11fe's run):
- **Public:** 0 unexpected. 8 values moved, all toward PoB: a General's Cry Berserker's damage (average 521,698 to 587,458, PoB 604,979) and a mine Assassin's, each from an item's skill now with its slot's supports.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fg: Pyroclast Mine of Sabotage's exposure

**PoB** (`PyroclastMineAltX`): a mine skill with an aura on enemies near its mines, `pyroclast_mine_aura_fire_exposure_%_to_apply`, `FireExposure` BASE -25. The aura is an `AuraDebuff` named "Pyroclast Mine Fire Exposure", scaled by the aura effects that apply to it. The enemy takes the strongest fire exposure (`CalcPerform.lua` 3563 to 3589). The mine reserves mana as every mine does.

**The engine:**
- **Gems:** a `Kind::Debuff` entry whose row is `SkillExposure(Fire)`. It takes part in `poe1::exposure`'s strongest exposure, as Frost Bomb's does (Phase 3 close-out). Its reservation is the level's, `gems::reservation` running for every kind.
- **A debuff at scale 1:** PoB scales this one by the non-curse aura effect, which the engine reads (`Totals::aura_effect`). With that effect present the debuff is an `aura-debuff-effect` reason, and its reservation still applies.
- **`data-parity` (`gem_effects`):** its `Aura` and `HasReservation` types are allowed for this debuff (`ALLOWED_TYPES`, with why); `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11ff's run):
- **Public:** 0 unexpected. A Penance Brand Elementalist's unreserved mana went from 240 to 221 (PoB 78) with the mine's reservation. Its damage fields are not computed, so the exposure moves nothing there.
- **Corpus:** 0 unexpected. The same move in its corpus build.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fh: item `{custom}` lines applied as PoB applies them

**Maintainer decision** (2026-11-20): "Yes, follow how PoB handles custom". Spike B's plan kept an item's `{custom}` lines (the player's text, not the game's) out, as `Exclusion::Custom`. ADR-015 Amendment 2 covered config custom lines only, so no amendment is needed: an item line read through the stat descriptions is stat-ID first as ADR-015 asks.

**PoB** (`Classes/Item.lua`): `{custom}` only marks the line (`modLine.custom`). `BuildModList` takes every mod line, custom or not, and a line its `ModParser` does not read whole adds nothing, as for any item line.

**The engine:**
- **`Resolver::resolve_item_ranged`** turns an `Excluded(Custom)` line into `Pending` before it reads the item, so the line is resolved, ranged, and applied as any line of the item. One that does not resolve stays a reason (`unresolved-line`) instead of `excluded-line`.
- **The import keeps the exclusion** (`legacy::pob_item::domain`), and the `{custom}` prefix stays on the line for export, so a build document saved before reads the same.
- **Tests:** the parser's ring fixture (its `{custom}` life line resolves), the synthetic build's outcome counts (`excluded` 0, `resolved` 10), the pipeline's reset test (a line a variant leaves out keeps its exclusion), the synthetic golden regenerated (each `excluded-line` reason of that line became its `unmapped-stat`: the synthetic bundle maps no stat). CI's pinned synthetic field counts are unchanged (10 unsupported, 41 indeterminate).

**Evidence** (`b-ct`, against P6-11fg's run):
- **Public:** 0 unexpected. 147 values moved, 145 toward PoB, 15 now equal:
  - a Cyclone Slayer's Strength, Intelligence, mana, and unreserved mana (-85 to -18, PoB -18; the helmet's reservation efficiency line that explained Autoexertion in P6-11eu) and crit chance (100, PoB 100);
  - a Boneshatter Juggernaut's life (5,579 to 5,890, PoB 5,890) and its leech caps;
  - an Occultist's armour and block, a Flicker Strike Berserker's and a Deadeye's movement speed, a spectre Necromancer, a ballista Hierophant, a Lightning Trap Elementalist.
- **Two moved away**, each where the engine was already past PoB for another cause: a Pathfinder's unreserved mana (220 to 269, PoB 91: its two `{custom}` reservation efficiency lines now apply, as in PoB) and a Flicker Strike Berserker's energy shield recharge (106 to 107, PoB 0; it follows the energy shield, 317 to 320, PoB 3,199).
- **Corpus:** 0 unexpected. 39 moved, 36 toward PoB, 17 now equal; the three away again past PoB already (unreserved mana, attack speed and its ignite).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fi: supports' weapon restrictions from the game's stats

**Maintainer decision** (2026-11-20): "Game data is king unless PoB seems to have a logical reason to differ mathematically."

**The game:** a support's stat set carries a flag stat for its restriction: `supported_skill_can_only_use_axe_and_sword` (Close Combat), `_axe_mace_and_staff` (Fissure, Trauma, Void Shockwave), `_dagger_and_claw` (Nightblade), `_mace_and_staff` (Shockwave), `_wand` (Sacred Wisps). Each one's description names the weapons ("Supported Skills can only be used with Maces and Staves").

**PoB** (`CalcActiveSkill.lua` 192 to 223, 287 to 291): each applied support's own `weaponTypes` list is added to the active skill's, and the held weapon's type must be in every list, else the skill is disabled.
- **Where a support has the stat,** PoB's list names the same types (Maces with Sceptres, Staves with Warstaves).
- **Fissure Support** carries the stat and has no list in PoB, so PoB does not restrict it. The engine follows the game. No fixture holds Fissure Support, so no difference is registered yet; one found later is a `game-truth` entry.
- **PoB lists types for supports with no such stat** (Arrow Nova, Barrage, Kinetic Instability, Voidstorm, Ballista Totem, Manaforged Arrows). No mathematical reason sets them apart, and the game matches those supports by skill types, which the engine already checks, so the engine does not follow them.

**The engine:**
- **`skill::SUPPORT_WEAPON_STATS`** maps each stat to PoB's weapon types.
- **`skill::support_weapons`** reads the applied supports' implicit and constant stats into `MainSkill::support_weapons`.
- **`MainSkill::can_use` and `disabled_by_hands`** need the held type in every list, as PoB does. A minion's own skills have none.
- **The five stats are `NoEffect`** (`weapon-restriction`): they change no modifier, and the code above reads them.
- **Test:** `can_use_should_need_every_supports_weapon_types`.

**Evidence** (`b-ct`, against P6-11fh's run):
- **Public and corpus:** 0 unexpected; no value moved. The fixtures' Shockwave builds hold maces or staves, and no restriction stat is unmapped now (5 before).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fj: Bear Trap's debuff

**PoB** (`act_dex.lua`, `BearTrap`): `bear_trap_damage_taken_+%_from_traps_and_mines` maps to `TrapMineDamageTaken` INC on the enemy, a `GlobalEffect` `Debuff`. A trap or mine skill's hit adds it to the enemy's `takenInc`, read with no skill config (`CalcOffence.lua` 3488 to 3490). Damage over time never reads it.

**The engine:**
- **New `Poe1Quantity::EnemyTrapMineDamageTaken`.**
- **`hit::projectile_taken` sums what the hit's kind adds:** a projectile hit's `ProjectileDamageTaken` (Sniper's Mark) and now a trap or mine skill's (`KEYWORD_TRAP`, `KEYWORD_MINE`) trap and mine damage taken. It is one term of `takenInc`, as in PoB.
- **`gems::SKILLS`:** Bear Trap as a `Kind::Debuff` with that row.
- **`data-parity` (`gem_effects`):** the quantity is PoB's `TrapMineDamageTaken`; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fi's run):
- **Public:** 0 unexpected. 8 values moved, all toward PoB: a mine Assassin's and a Lightning Trap Elementalist's damage (average, total, combined, and full DPS), each with Bear Trap in the build.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fk: Frost Shield

**PoB** (`act_int.lua`, `FrostShield`): every value of its buff scales by `Multiplier:FrostShieldStage`, which only the config's `frostShieldStages` sets (a count with no default, `ConfigOptions.lua` 455):
- the spell crit chance per stage;
- the shield's life per stage;
- the energy shield drain, from one stage;
- the share of damage the shield takes. It matters only with the shield's life: `CalcDefence.lua` 47 to 52 makes no pool without life, 2444 to 2447.

With no stage, every value is 0. As a `ChillingArea` skill it also chills the enemy, at the default chill times `calcLib.mod(skillModList, "EnemyChillEffect")`, which includes the gem's quality (`cold_ailment_effect_+%`, `CalcPerform.lua` 1393 to 1397).

**The engine:**
- **`gems::SKILLS`:** Frost Shield as a buff whose staged values are `ReasonWhenInput` on `frostShieldStages` (`frost-shield-stages`). The stage multiplier and the shield's pool are not modelled, so with stages they are reasons; without, nothing, as in PoB. `ReasonWhenInput` now reads a count of 0 as off. The stage limit is `NoSlice`.
- **Its guaranteed chill applies** as Arctic Armour's and Frostblink's do (`gems::guaranteed_chill`; the real-bundle test's list of chilling skills now names it).
- **The skill's own chill effect:** a chilling skill whose quality gives it, at a quality above 0, is a `skill-chill-effect` reason (`SKILL_CHILL_QUALITY`); the chill stays at the character's effect.
- **`data-parity` (`gem_effects`):** Frost Shield's own `EnemyChillEffect` is allowed with that reason, and its stage limit is a no-slice name; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fj's run):
- **Public:** 0 unexpected; no value moved. Frost Shield is no longer an unmodelled gem in the three builds that hold it, none of which sets stages. The two with quality 20 carry the `skill-chill-effect` reason.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fl: Hexpass

**PoB** (`sup_int.lua`, `SupportHexpass`): `hexed_enemies_cannot_deal_critical_strikes` gives the enemy `NeverCrit` and `Condition:NeverCrit`, each a `GlobalEffect` of type `Curse`. So it rides on the hex it supports and applies only while that hex holds a curse slot. The defence estimate's enemy crit chance is then 0 (`CalcDefence.lua` 1694). Its hexproof and curse area stats reach no slice field.

**The engine:**
- **New `Poe1Quantity::EnemyNeverCrit`** (a flag), read by `enemy_hit`'s enemy crit.
- **`gems::CURSE_SUPPORTS`:** supports whose rows join the curse's rows in its `CurseApplied`, scaled as the curse's are (a flag is not scaled), so they go through PoB's curse slots. Hexpass is the first. It is also in `SUPPORTS`, with no global stats. Test: `curse_supports_should_be_sorted_modelled_supports`.
- **`data-parity` (`gem_effects`):**
  - a support in `CURSE_SUPPORTS` checks those rows;
  - `POB_ROW_COMPANIONS` leaves out PoB's `Condition:NeverCrit`, a companion of the flag that only conditioned mods read;
  - a flag's own `true` is no longer a tag.
  - `uncategorized` 0, the totals otherwise unchanged.

**Evidence** (`b-ct`):
- **New mechanic fixture** `minion-summon-flame-golem-hexpass-witch.xml`: the cursed Flame Golem fixture with Hexpass on its Punishment. In PoB the enemy crit chance goes from 5 to 0 and the total EHP from 745.2 to 757.4. All 86 compared fields equal PoB's.
- **Public:** 0 unexpected; no other value moved. The two community builds with Hexpass apply no hex the engine models: one is cast through Bane, the other through Eldritch Blasphemy.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fm: Battlemage's Cry's support effect

**PoB** (`act_str.lua` 868, `BattlemagesCrySupport`): the Battlemage's Cry gem also grants a support (`secondaryGrantedEffectId`). It triggers the spells it supports (`triggeredByBattleMageCry`) with less damage (`support_divine_cry_damage_+%_final`, a local stat), and has no global stat. A curse it triggers applies as any.

**The engine:** the support was not in `gems::SUPPORTS`, so a group with Battlemage's Cry and a spell was an `unmodelled-gem` reason, and its spells (a Vulnerability) did nothing. It is now in `SUPPORTS` with no global stats. `data-parity` (`gem_effects`) allows its `Triggered` type and its trigger key, as Automation's; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fl's run):
- **Public:** 0 unexpected. 4 values moved, all toward PoB: a Boneshatter Juggernaut's damage (average 88,871 to 115,531, PoB 582,277) with its Vulnerability now applied.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fn: Ensnaring Arrow's debuff

**PoB** (`act_dex.lua`, `EnsnaringArrow`):
- `tethered_enemies_take_attack_projectile_damage_taken_+%` maps to `ProjectileAttackDamageTaken` INC on the enemy, a `Debuff` named "Ensnared".
- Its `GlobalEffect` tag carries `effectStackVar = "EnsnareStackCount"` and `effectStackLimit = 1`. The stack count is the config's `multiplierEnsnaredStackCount`, which has no default.
- PoB scales the debuff by `DebuffEffect` times `min(stacks, limit)`; with no stack it does nothing (`CalcPerform.lua` 2534 to 2600).
- A projectile attack's hit adds it to the enemy's `takenInc` (`CalcOffence.lua` 3485).
- The gem's quality (`BuffEffect`) does not reach a `Debuff`.

**The engine:**
- **New `Poe1Quantity::EnemyProjectileAttackDamageTaken`,** one more term of `hit::projectile_taken`, for a hit with both the projectile and attack flags.
- **New `BuffStat::PerStack { input, var, limit, rows }`:** PoB's stack variable and limit, with the rows times `min(count, limit)`. Ensnaring Arrow is a `Kind::Debuff` with it.
- **Config inputs read:** `multiplierEnsnaredStackCount`, and Frost Shield's `frostShieldStages` (P6-11fk), are now among the inputs `poe1::gems` reads, so neither is an `unmodelled-config` reason.
- **`data-parity` (`gem_effects`):** the stack tag is compared as PoB writes it, and the quality's `BuffEffect` is allowed with why; `uncategorized` 0.

**Evidence** (`b-ct`):
- **New mechanic fixture** `burning-arrow-ensnaring-arrow-templar.xml`: Burning Arrow with Ensnaring Arrow at one stack.
  - Average damage is 6.105, equal to PoB's; with 0 stacks the engine gives 5.088.
  - 73 fields match. The 6 differences are the Q11 attack-time rounding of the base fixture, registered as `game-truth` (DIV-0484 to DIV-0489).
- **Public:** 0 unexpected; no other value moved. The Deadeye with Ensnaring Arrow attacks with Elemental Hit, which is not a projectile skill.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fo: Phase Run

**PoB** (`act_dex.lua`, `PhaseRun`): a `Buff` with `base_movement_velocity_+%` (`MovementSpeed` INC) and `phase_run_melee_physical_damage_+%_final`. The second is `PhysicalDamage` MORE with `ModFlag.Melee` and a negated `SkillType.SummonsTotem` tag: it does not apply "to your melee skills used by totems". Its stealth and phasing reach no slice field.

**The engine:**
- **New `Flags::SKILL_NOT_TOTEM`:** `skill::keyword_flags` sets it on every skill whose types (supports' added types included) lack `SummonsTotem`. It is PoB's negated tag as a flag the rows can require; the tests' expected keyword flags carry it.
- **`gems::SKILLS`:** Phase Run as a buff with both rows, the damage row requiring physical, melee, and `SKILL_NOT_TOTEM`.
- **`data-parity` (`gem_effects`):** `ModFlag.Melee` and the negated `SummonsTotem` tag are named on both sides; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fn's run):
- **Public:** 0 unexpected. 4 values moved, all toward PoB, 2 now equal: an Occultist's movement speed (2.14 to 2.69, PoB 2.69) and a Mirage Archer Toxic Rain build's.
- **Corpus:** 0 unexpected. 2 values moved toward PoB, both now equal (movement speed 1.62 to 2.17).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fp: the enemy's Debilitated and Malignant Madness in its damage

**PoB** (`CalcSetup.lua` 68 to 71, `initModDB`): every actor gets `Damage` MORE -10 under its condition `Debilitated` and under `MalignantMadness`, each a `Debuff` (movement and action speed too, which reach no slice field). The config checks `conditionEnemyDebilitated` and `conditionEnemyMalignantMadness` (`ConfigOptions.lua` 528) set the enemy's. Since P6-10b part 12 the enemy's damage reaches the slice: its hit on you reads `calcLib.mod(enemyDB, "Damage")`. Both inputs were still in `config::NAMED_ONLY`, as read only outside the slice.

**The engine:** `damage_taken::ENEMY_LESS_DAMAGE_INPUTS`: each check on gives the enemy's `EnemyDamage` MORE -10 (`damage_taken::damage_taken`). Both left `NAMED_ONLY`, and the stale part of its doc comment is gone. Test: `the_inputs_should_not_be_named_only` lists them.

**Evidence** (`b-ct`, against P6-11fo's run):
- **Public:** 0 unexpected. 28 values moved, all toward PoB, in three builds with Eldritch Blasphemy's Malignant Madness checked: the maximum hits taken, the physical damage reduction, the hits to die, and the total EHP (an Occultist's 251,664 to 287,753, PoB 364,354).
- **Corpus:** 0 unexpected. 10 values moved, all toward PoB, in one build.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fq: Eldritch Blasphemy, and "<Skill> has no Reservation"

**PoB:**
- **Eldritch Blasphemy** (`sup_int.lua`, `SupportEldritchBlasphemy`, `plusVersionOf = "SupportBlasphemy"`) is Blasphemy with its own reservation share. Its Malignant Madness minion damage needs the player condition `enemyMalignantMadness`, which nothing sets; the enemy's own condition is P6-11fp's.
- **"<Skill> has no Reservation" and "<Curse> has no Reservation if Cast as an Aura"** (`ModParser.lua` 5317 to 5328) set that skill's reservation keys to 0: by a `SkillId` from its name, never a Blessing, the second only while the skill is an `Aura`.

**The engine:**
- **`SupportEldritchBlasphemy` in `gems::SUPPORTS`,** as Blasphemy. Its types and reservation share are allowed in `data-parity` as Blasphemy's are.
- **`skill_modifiers::NO_RESERVATION_STATS`:** the game's 33 stats with the skill's name and whether the aura type is needed.
  - Each maps to the flag `Poe1Quantity::SkillNoReservation(i)`, collected in `Totals::no_reservation`.
  - `gems::reservation` gives such a skill no reservation, matched by name as PoB's lookup is.
  - Tests: `no_reservation_stats_should_be_sorted_flags`; the real-bundle test checks each row's text ("Despair has no Reservation if Cast as an Aura").
- **Without the second part** Eldritch Blasphemy alone gave an Occultist 10,863 life reserved (unreserved -5,377, PoB 5,486): its two hexes have no reservation from its body armour's lines, which the engine now reads.

**Evidence** (`b-ct`, against P6-11fp's run):
- **Public:** 0 unexpected. 7 values moved, all toward PoB:
  - the Occultist's defence estimate, its Eldritch Blasphemy hexes now applied (total EHP 287,753 to 293,684, PoB 364,354; its unreserved life stays at PoB's 5,486);
  - a Luminary's unreserved mana (-80 to 154, PoB 42) by its own "has no Reservation" line.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fr: the Vaal gems' base skills, Vaal Blight's Hinder, and four skills with no global effect

A Vaal gem grants its base skill too (`gemId` with two granted effects): a build with Vaal Earthquake, Vaal Flicker Strike, or Vaal Blight had the base skill, not the Vaal one, unmodelled. The engine's skill table missed both.

**PoB** (`Data/Skills`):
- Earthquake, Blight, Vaal Earthquake, Primal Aegis (an item's triggered spell), Bloodrend (another), and Charged Mines Support have no global effect.
- Flicker Strike's buff gives `MovementSpeed` INC ("Flick Strike").
- Vaal Blight's Hinder gives the enemy `ChaosDamageTaken` INC, a `Debuff`.

**The engine (`gems::SKILLS`, `SUPPORTS`):**
- **Inert:** Blight, Earthquake, Vaal Earthquake, Primal Aegis, Bloodrend.
- **Buff:** Flicker Strike.
- **Debuff:** Vaal Blight's Hinder (`EnemyTypeDamageTaken(Chaos)`).
- **Support:** Charged Mines.
- **`data-parity` (`gem_effects`):** the Vaal skills' soul key and the two item spells' trigger allowed with why; `uncategorized` 0.
- **New helper** `local/p6/tools/skills_insert.py` places an entry at its sorted place (one-line entries included).

**Evidence** (`b-ct`, against P6-11fq's run):
- **Public:** 0 unexpected. 14 values moved, 13 toward PoB: an Essence Drain Occultist's defence estimate (total EHP 75,636 to 89,654, PoB 433,211) and full DPS, with Vaal Blight's Hinder and the base skills now modelled.
- **One away:** a Holy Flame Totem Hierophant's unreserved mana (130 to 98, PoB 181): its Pyroclast Mine of Sabotage with Charged Mines now reserves. The engine was already below PoB's unreserved mana before this, so another cause still reserves more than PoB does.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fs: skills with no global effect, Vaal Righteous Fire's buff, Wither's debuff

Found by counting, across the public and corpus runs, the granted effect IDs the skill table misses (a local instrumented run). A gem's skill ID is not always the effect the table needs: a Vaal gem grants its base skill too (P6-11fr).

**PoB** (`Data/Skills`):
- **No global effect:** Armageddon Brand, Penance Brand, Combust, Cremation, Dark Pact, Detonate Dead, and the Frost enchantment's triggered spell (`EnchantmentOfFrostOnKill4`).
- **Vaal Righteous Fire's buff:** `Damage` MORE with spells (`vaal_righteous_fire_spell_damage_+%_final`).
- **Wither's "Withered" debuff:** only the enemy's movement speed.

**The engine (`gems::SKILLS`):** the seven as `Kind::Inert`, Vaal Righteous Fire as a buff with its row, Wither as a debuff whose stat is `NoSlice`. `data-parity` (`gem_effects`) allows each skill's own data, the brands' type, and the triggered spells, with why; `uncategorized` 0.

**Left for their own steps:**
- **The banners:** PoB's `AuraEffect` MORE per resource (Valour).
- **Corrupting Fever:** a buff skill with no stat map, whose effect PoB reads elsewhere.
- **Herald of the Breach, Flame Surge, Scorching Ray, Wave of Conviction, Sigil of Power, Plague Bearer, Petrified Blood:** each needs a stage, a config condition, or a pool.

**Evidence** (`b-ct`, against P6-11fr's run):
- **Public:** 0 unexpected. 4 values moved, all toward PoB: a dual-wield Cast on Crit build's damage with Vaal Righteous Fire's buff, and a Righteous Fire Chieftain's full DPS.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ft: Manaforged Arrows, Mirage Archer beside the main skill, and Generosity

**PoB** (`Data/Skills`):
- **Manaforged Arrows and Mirage Archer** have no global stat. The first triggers the bow skill it supports; the second's mirage uses it, and Mirage Archer on the main skill is P6-8b's.
- **Generosity** sets `auraCannotAffectSelf`: the aura it supports affects your allies and minions, not you (`CalcPerform.lua` 2409). Its `non_curse_aura_effect_+%` is the aura's `AuraEffect` INC.

**The engine:**
- **`gems::SUPPORTS`:** the three, with no global stats.
- **Generosity:** an aura it supports adds no rows to you (`AURA_NOT_ON_SELF_SUPPORTS`). The minion still gets it (`minion_buffs`), scaled with the support's increase: `non_curse_aura_effect_+%` joins `OWN_EFFECT_STATS`. Test: `aura_not_on_self_supports_should_be_modelled`.
- **`data-parity` (`gem_effects`):** the trigger keys and Manaforged's `Triggered` type, and Generosity's self flag, allowed with why; `uncategorized` 0.
- **Left out on purpose, `data-parity` showed why:**
  - Ballista Totem and Spell Totem make the skill a totem's (`SummonsTotem`, `ReservationBecomesCost`), so a buff under them would wrongly reach you.
  - Unbound Ailments raises the skill's own chill effect.

**Evidence** (`b-ct`, against P6-11fs's run):
- **Public:** 0 unexpected. 4 values moved, all toward PoB: an Elemental Hit Deadeye's damage, with its supported skills now modelled.
- **Corpus:** 0 unexpected. 1 value moved toward PoB: a build's full DPS with its Generosity aura on the minion (195,296 to 480,518, PoB 3,261,033).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fu: Maim Support's debuff

**PoB** (`sup_str.lua` 3585 to 3592, `SupportMaim`): `support_maimed_enemies_physical_damage_taken_+%` gives the enemy `PhysicalDamageTaken` INC under its `Maimed` condition (the config's `conditionEnemyMaimed`), a `GlobalEffect` `Debuff` named "Maim". Whatever skill it supports, the enemy takes it. Its physical damage MORE is the supported skill's own.

**The engine:**
- **`SupportMaim` in `gems::SUPPORTS`:** a `WhenState` row on `conditions::ENEMY_MAIMED`, `EnemyTypeDamageTaken(Physical)` INC, merged under "Maim".
- **`DEBUFF_SUPPORTS`:** a support's debuff takes `DebuffEffect` alone, a reason (`DEPENDS_ON`), so `supports_global` adds it at scale 1, not the buff scale. Test: `aura_not_on_self_supports_should_be_modelled` checks it.
- **`data-parity` (`gem_effects`):** the row equals PoB's; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11ft's run):
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected. 6 values moved, all toward PoB, in one build with Maim and a maimed enemy (its damage, bleed, and leech).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fv: Bonechill

**PoB** (`sup_int.lua` 542, `SupportBonechill`; `CalcPerform.lua`):
- **`HasBonechill`:** a skill with `supportBonechill` (its base mod) sets it, unless the skill cannot chill (1357, 1358). A chilling area with it also sets `hasGuaranteedBonechill` (1397, 1398).
- **The chill as damage taken:** the enemy's chill becomes `ColdDamageTaken` INC equal to it, last in the chill's order, only with the guarantee or a configured chill above 0 (`ChillVal`, 3444).
- **Bonechill's `chill_effect_+%`** (every level) is the supported skill's own `EnemyChillEffect`. Its cold damage MORE is that skill's own too.

**The engine:**
- **New `Poe1Quantity::Bonechill(guaranteed)`:** `gems::guaranteed_chill` sets it for every skill Bonechill supports, guaranteed for a chilling area. `damage_taken::chill_damage_taken` reads it after Bitter Frost's flags, with PoB's condition.
- **`SupportBonechill` in `SUPPORTS`;** the socketed-item Bonechill stat leaves `CHILL_DEPENDS_ON`, since `item_supports` joins it as the support.
- **A chilling area with Bonechill:** its chill effect is a `skill-chill-effect` reason, as a chilling skill's own quality is (P6-11fk).
- **`data-parity` (`gem_effects`):** Bonechill's flag and chill effect allowed with why; `uncategorized` 0.

**Evidence** (`b-ct`):
- **New mechanic fixture** `ice-nova-bonechill-witch.xml`: Ice Nova with Arc and Bonechill beside it, the enemy configured as chilled (8).
  - Average damage is 805.20, equal to PoB's; without Bonechill the engine gives 747.0.
  - All 78 compared fields equal PoB's.
- **Public and corpus:** 0 unexpected; no other value moved. The builds with Bonechill neither configure a chill nor have a chilling area with it.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fw: Defiance Banner and War Banner

**PoB** (`act_str.lua`, `DefianceBanner`, `WarBanner`):
- **Unplanted:** a banner's base mods give its aura `AuraEffect` MORE -100 unless the config's `bannerPlanted` is on, so it does nothing.
- **Planted:**
  - the aura scales per Valour consumed (`banner_buff_effect_+%_final_per_resource` times `Multiplier:BannerValour`, the config's `bannerValour` up to `MaximumValour`, `CalcPerform.lua` 1299 to 1302);
  - a `Buff` of its name sets `AffectedByPlacedBanner`.
- **The aura's rows:** Defiance Banner's movement speed and spell suppression; War Banner's accuracy and physical damage MORE with melee skills.

**The engine:** both in `gems::SKILLS` as auras whose stats, the base mods (`""`) included, are `ReasonWhenInput` on `bannerPlanted` (`banner-valour`). Unplanted they do nothing, as in PoB; planted they are reasons, the Valour scale not being modelled. `bannerPlanted` is among the inputs `poe1::gems` reads. `data-parity` (`gem_effects`) lists both buff identities and allows the `AuraEffect` MORE with why; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fv's run):
- **Public and corpus:** 0 unexpected; no value moved. The banner builds do not plant theirs, so the banner is no longer an unmodelled gem there and still does nothing, as in PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fx: Wave of Conviction and Sigil of Power

**PoB** (`Data/Skills`, `ConfigOptions.lua`):
- **Wave of Conviction:** its stat gives fire, cold, or lightning exposure, each a `Debuff` with an `effectCond` that only the config list `waveOfConvictionExposureType` sets (no default; 736 to 744). Without a choice it does nothing.
- **Sigil of Power:** added lightning damage per stage up to its maximum, and the enemy's less damage at the maximum. The stages are the config's `sigilOfPowerStages` (`countAllowZero` with `defaultPlaceholderState = 1`, 620), so it acts at stage 1 without any input.

**The engine (`gems::SKILLS`):**
- **Wave of Conviction:** a debuff whose stat is `ReasonWhenInput` on the exposure type (`wave-of-conviction-exposure`): nothing without one, as PoB; a reason with one.
- **Sigil of Power:** a buff whose staged stats are `Reason` (`sigil-of-power-stages`), since PoB applies them by default and the stage multiplier is not modelled. Its stage limit is `NoSlice`.
- **`data-parity` (`gem_effects`):** both skills' buff identities, and the stage limit as a no-slice name; `uncategorized` 0.

**Left for their own steps:**
- **Plague Bearer:** PoB's default list value, "Incubating", turns its poison damage on without input.
- **Scorching Ray:** its exposure's condition is not a config input.
- **Herald of the Breach:** damage per herald.

**Evidence** (`b-ct`, against P6-11fw's run):
- **Public and corpus:** 0 unexpected; no value moved. The builds holding them have them unmodelled before and a reason or nothing now, as above.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fy: Plague Bearer

**PoB** (`act_int.lua`, `PlagueBearer`; `ConfigOptions.lua` 545 to 552): its buff gives `Damage` MORE with `KeywordFlag.Poison` under `modCond = "PlagueBearerIncubating"`. The config list `plagueBearerState` sets that condition for "INC", its default (`defaultIndex = 1`), and "INF" sets the other. So without input the multiplier applies; at level 20 it is a less multiplier on poison.

**The engine:**
- **`gems::SKILLS`:** Plague Bearer as a buff whose row is `ConfigIs { value: "INF", equal: false }`: it applies unless the state is "Infecting". `plagueBearerState` is among the inputs `poe1::gems` reads.
- **`data-parity` (`gem_effects`):** `KeywordFlag.Poison` (`Flags::POISON`) is named on both sides. No other row's form changed (the totals are equal); `uncategorized` 0.

**Evidence** (`b-ct`):
- **New mechanic fixture** `smite-poison-plague-bearer-templar.xml`: the Smite poison fixture with Plague Bearer beside it and no state configured.
  - PoB's poison DPS goes from 27.28 to 21.88 with it; the engine's total poison DPS is 8.4655 against PoB's 8.4698.
  - 73 fields match. The 6 differences are the Q11 attack-time rounding the base fixture has too, registered as `game-truth` (DIV-0490 to DIV-0495).
- **Public and corpus:** 0 unexpected; no other value moved. The public build with Plague Bearer sets "Infecting".
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11fz: Herald of the Breach

**PoB** (`other.lua`, `HeraldOfTheBreach`, named "Herald of the Hive", a skill items grant): its buff gives `Damage` MORE per herald skill affecting you (`Multiplier` `Herald`, a `GlobalEffect` buff), 3% at every level, and it reserves 25% of mana. `Multiplier:Herald` is the count `CalcPerform.lua` 2112 to 2126 takes, the herald itself included. Its pulses (Otherworldly Pressure, the pulse delay) are its own hit, not a global effect.

**The engine:**
- **`gems::SKILLS`:** Herald of the Breach as a buff whose row is the stack quantity `Stack(Herald, Per(DamageMore))`, which `poe1::stacks::aura_or_herald` multiplies by the herald count `gems::herald_and_aura_counts` already gives. Being in the table, its reservation now counts.
- **`data-parity` (`gem_effects`):** an effect per herald is printed as its target with PoB's `Multiplier` tag; the buff's name is PoB's. `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11fy's run):
- **New mechanic fixture** `ice-nova-herald-of-the-breach-witch.xml`: the Ice Nova witch with Herald of the Hive in a group of its own. PoB's average damage goes from 727.55 to 749.33 and unreserved mana from 590 to 442; all 77 compared fields match.
- **Public and corpus:** 0 unexpected. Unreserved mana moves toward PoB in the 3 builds holding the herald (2 now equal; the Penance Brand community build 221 to 88, PoB 78, its remaining gap from elsewhere).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ga: Flame Surge's burning ground (slice revision 23)

**PoB:**
- **The buff** (`act_int.lua` 6620): Flame Surge gives `IgniteDpsAsBurningGround` MAX 30, an unscalable `GlobalEffect` buff.
- **The ignite** (`CalcOffence.lua` 5098 to 5108): the share makes `BurningGroundDPS = min(baseVal x effectMod x rateMod x fireEffMult x share / 100, DotDpsCap)`, without the active ignites.
- **The totals:** `TotalDotDPS` adds it after the ignite (6126), and the full DPS keeps the largest of all skills (`Calcs.lua` 303, 355).

**The engine:**
- **`Poe1Quantity::IgniteDpsAsBurningGround`:** `Base`. One buff per name applies, so the sum is PoB's largest.
- **`Poe1Field::BurningGroundDps`** (`poe1.burning-ground-dps`): set by the ignite (`ailment::damaging`) from its own parts, with fire's enemy effect (the ignite's `dot_type`). When a dual-wielding attack's ignite is not computed, neither is the ground.
- **`poe1::combined` and `poe1::full_dps`:** the ground enters `TotalDotDPS` after the ignite, and the full DPS takes the largest.
- **`gems::SKILLS`:** Flame Surge as a buff with that row (`Unscalable`).
- **`data-parity` (`gem_effects`):** the row is printed as PoB's MAX mod; `uncategorized` 0.
- **Slice revision 23:** `BurningGroundDPS`, applies present.
- **UI:** the field's label, with the offence fields.

**Not modelled:**
- Alchemist's Mark's share, a curse on the enemy (`enemyDB`).
- A skill's own burning ground (`dotIsBurningGround`, 5620 to 5643).
- A mirage's burning ground.
- Flame Surge of Combusting's share, which applies only when it is the main skill.

**Evidence** (`b-ct`, against P6-11fz's run):
- **New mechanic fixture** `ice-nova-ignite-flame-surge-witch.xml`: the Ice Nova ignite fixture with Flame Surge in a group of its own. All 79 compared fields match, among them `BurningGroundDPS` 41.83, the ignite 139.43, and the combined and full DPS 1699.34.
- **Public and corpus:** 0 unexpected, and no value moved. The new field is not computed in the 7 builds where PoB has it (1 public, 6 corpus): none of them has a computed ignite.
- **Tests:** the workspace tests, the UI tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gb: Scorching Ray's fire exposure

**PoB** (`act_int.lua` 15712 to 15770; `CalcActiveSkill.lua` 635 to 643; `CalcPerform.lua` 2241 to 2245, 2260):
- **The debuff:** Scorching Ray's `base_fire_damage_resistance_%` (-25) is `FireExposure` BASE, a `Debuff` named "Fire Exposure" whose `effectCond` is `ScorchingRayMaxStages`.
- **The condition:** the skill sets it on its own mods by a `MultiplierThreshold` of 7 on `ScorchingRayStageAfterFirst`, which is `min(7, stage count - 1)`.
- **The stage count:** the gem's `skillStageCount`, defaulting to the maximum, 8. In part 2 ("Maximum Stages") it is always 7.
- **So:** the exposure applies by default, and not below 8 stages in part 1.

**The engine:**
- **`gems::BuffStat::AtMaxStages`:** applies its rows when the gem's `skillPart` is the maximum-stages part, or its `skillStageCount` (missing: the maximum; at least 1, as PoB's `m_max`) reaches the maximum.
- **`gems::SKILLS`:** Scorching Ray as a debuff with that row, `SkillExposure(Fire)`; the strongest exposure wins as for every skill (`poe1::exposure`).
- **`data-parity` (`gem_effects`):** the row is printed with PoB's `effectCond`, and it matches; `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11ga's run):
- **Two new mechanic fixtures** from the Ice Nova ignite fixture, with Scorching Ray in a group of its own. All 78 compared fields match in each:
  - `ice-nova-ignite-scorching-ray-witch.xml`, at the default stage count: the exposure applies (average damage 1121.02, ignite 209.14);
  - `ice-nova-ignite-scorching-ray-stages-witch.xml`, at 4 stages: none (ignite 139.43, the base fixture's).
- **Public and corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gc: summoned golems; each golem's buff effect

**PoB:**
- **The golem count** (`CalcPerform.lua` 1401 to 1413): for each active skill whose minion list has a minion limited by `ActiveGolemLimit`, `floor(calcLib.val(skillModList, "ActiveGolemLimit") x More("ActiveMinionLimit"))`, and `output.ActiveGolemLimit` keeps the largest. The skill's list holds its gem's `base_number_of_golems_allowed` (1) and the character's "+# to maximum number of Summoned Golems".
- **"#% increased Damage per Summoned Golem"** (`damage_+%_per_active_golem`): `Damage` INC with `PerStat` `ActiveGolemLimit`, which `GetStat` reads from that output.
  - **Correction:** P6-11dc said nothing sets the output, so PoB applies 0. That was wrong: line 1412 sets it, and PoB applies the line per golem, as the game does. The maintainer's game-truth item for it is closed.
- **"#% increased Effect of the Buff granted by your <Kind> Golems"** (`base_<kind>_golem_granted_buff_effect_+%`, `ModParser.lua` 479 to 484): `BuffEffect` INC tagged with the golem skill's name, transfigured skills included.

**The engine:**
- **`poe1::stacks::golems`:** the count from `gems::golem_limit`. It takes the skills of the main group and of the enabled groups in an enabled slot with a golem minion (PoB's minion data). Each skill's limit is its gems' `base_number_of_golems_allowed` plus the character's (`StackKind::Golems`, `Maximum`), times its supports' `ActiveMinionLimit` MORE, floored. The count is the largest of them, and 0 without a golem skill. The damage per golem applies times the count.
  - "Maximum number of Summoned Golems is Doubled" and the Primordial jewels' line stay unmapped, so they are reasons.
- **`Poe1Quantity::NamedGolemBuffEffect`** (`skill_modifiers::Golem`): five `skill_modifiers::STATS` rows. `Totals::named_golem` adds the matching golem's sum to a buff's own effect, matched by the active skill's ID with `_alt_x` or `_alt_y` stripped.
- **Mapping:** the character's `base_number_of_golems_allowed` is now that stack row; it was a no-slice row. On a golem gem, the stat has no effect (`SkillMapping`), because the count reads the gems.

**Evidence** (`b-ct`, against P6-11gb's run):
- **Two new mechanic fixtures** from the Ice Nova fixture, with Summon Lightning Golem and custom lines (31% increased effect of its buff, 10% increased damage per summoned golem). All compared fields match in each:
  - `ice-nova-lightning-golem-witch.xml`, with one more golem allowed: two golems (average damage 873.06);
  - `ice-nova-lightning-golem-one-witch.xml`: one golem (800.17).
- **Public:** 0 unexpected. One value moved, toward PoB: the Penance Brand build's mana regeneration, from the Lightning Golem's buff, 77.4 to 84.3 (PoB 81.9). The build has other reasons (timeless jewels).
- **Corpus:** 0 unexpected. Eight values moved, all toward PoB, and one now equals PoB.
  - Three builds' mana regeneration, one now equal (81.9).
  - One build's average damage, ignite, combined DPS, total DPS, and life leech from the damage per golem.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gd: Trinity Support's resonance

**PoB** (`sup_int.lua` 6592 to 6597; `ConfigOptions.lua` 708):
- **The count:** the config's `configResonanceCount` ("Lowest Resonance Count", shown with Trinity) sets `Multiplier:ResonanceCount` to the input clamped to 0 to 50. Nothing is set at 0 or without the input.
- **The damage:** `elemental_damage_+%_final_per_5_lowest_resonance` (3 per 5) is `ElementalDamage` MORE per `ResonanceCount` with `div` 5, so `floor(n / 5 + 0.0001)` times.
- **The penetration:** `damage_penetrates_%_elemental_resistances_while_all_resonance_is_25` is `ElementalPenetration` BASE with a `MultiplierThreshold` of 25.
- Both are the supported skill's, so the ignite takes the more damage too.

**The engine:**
- **`poe1::stacks`:** `StackKind::Resonance` with the two rows, read from the config's input (`stacks::RESONANCE_INPUT`, moved out of `config::NAMED_ONLY`). `stacks::resonance` applies the more damage per 5, and from 25 the penetration of fire, cold, and lightning (`PerStack::ElementalPenetrationIfAll25`).
- An input that is not a number is a reason under its input, as the other counts.

**Evidence** (`b-ct`, against P6-11gc's run):
- **Two new mechanic fixtures** from the Ice Nova ignite fixture, with Trinity Support on Ice Nova. All compared fields match in each:
  - `ice-nova-ignite-trinity-witch.xml`, at a resonance of 50: average damage 1823.15, ignite 181.35;
  - `ice-nova-ignite-trinity-20-witch.xml`, at 20: average damage 1190.01, ignite 156.16. No penetration.
- **Public:** 0 unexpected. Four values moved toward PoB (the Elemental Hit Deadeye build's damage), none away.
- **Corpus:** 0 unexpected. Ten values moved, eight toward PoB.
  - The two moved away are two builds' ignite, which was above PoB's before this step (1658.6 against 1482.3, and 1951.0 against 2175.0, which it now passes).
  - The fixtures show PoB's ignite takes the more damage, so both builds' remaining gap is from something else.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ge: Elemental Focus cannot inflict elemental ailments

**PoB:**
- **The stat:** Elemental Focus's `cannot_inflict_status_ailments` sets seven flags on the supported skill's list (`SkillStatMap.lua` 1062): `CannotShock`, `CannotChill`, `CannotFreeze`, `CannotIgnite`, `CannotScorch`, `CannotBrittle`, `CannotSap`.
- **Where PoB reads them:**
  - **The hit's chances** (`CalcOffence.lua` 4119): no ignite, and no chance of the others. The others' chances reach no slice field.
  - **A guaranteed ailment's effect:** read from the main skill's list only with `ChecksHighestDamage` (`CalcPerform.lua` 3517). The stats that set that flag are `damage_taken`'s reasons.
  - **Bonechill** (1357): `HasBonechill` only for a chilling area, a non-hit chill, or a skill without `CannotChill`.
  - **A non-hit chill** (1393): only without `CannotChill`.

**The engine:**
- **Mapping:** the stat is `CannotInflict(Ignite)`, a flag `poe1::ailment` reads, so the supported skill deals no ignite.
- **`poe1::gems`:** a skill whose gems hold the stat (`gems::CANNOT_CHILL_STATS`) gets no Bonechill unless it is a chilling area or a non-hit chill, and its non-hit chill is no reason (it does not chill).
- The other flags reach no slice field, as the mapping's comment records.

**Evidence** (`b-ct`, against P6-11gd's run):
- **Two new mechanic fixtures.** All compared fields match in each:
  - `ice-nova-ignite-elemental-focus-witch.xml`: the Ice Nova ignite fixture with Elemental Focus on Ice Nova. No ignite, and average damage 1423.97.
  - `ice-nova-bonechill-elemental-focus-witch.xml`: the Bonechill fixture with Elemental Focus beside Bonechill on Arc. Bonechill gives nothing: average damage 747.00, against 805.20 without the support.
- **Public:** 0 unexpected. Two values of the Holy Flame Totem build moved: its combined and full DPS lost the ignite (10898.47).
  - Numerically they moved away from PoB, whose values are far higher for other reasons.
  - PoB gives the build no ignite (its combined DPS equals its total), so the change is PoB's.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gf: 57 more supports an item grants; Concentrated Effect

**Found:** `item_supports::ROWS` (P6-11r, 203 rows) lacked 84 stats whose game text is PoB's item support line, "Socketed Gems are Supported by Level N <Support>".
- Among them are the older `local_display_socketed_gems_get_*_level` stats, and those written "supported" in lowercase.
- They were unmapped, so reasons. Concentrated Effect (2 builds), Increased Area of Effect, Minion Damage (2 builds), and Hypothermia were among the build lines.

**PoB:** PoB's own `modLib.parseMod` on each line at level 20 (`tools/oracle/data/pob_parse_lines.lua`, output kept locally):
- **57 stats** give `ExtraSupport` alone. "Cast On Critical Strike" gives two supports.
- **19** also give an `ExtraSkill`, and **8** PoB does not parse ("Blessed Calling", "Call To Arms", "Cluster Trap", "Edify", "Harrowing Throng", "Increased Duration", "Magnetism", "Ward"). These stay reasons, as in P6-11r.

**The engine:**
- **`item_supports::ROWS`:** the 57 rows, so 260 in all. Each is the stat's value as the level, with PoB's support IDs.
- **The bundle test** (`real_item_support_rows_should_name_their_gems`) now reads the line in lowercase, as PoB matches it. It checks every row's text and gem against the bundle, and all 260 pass.
- **Concentrated Effect's own stats:**
  - `support_area_concentrate_area_damage_+%_final` is a hit row: `Damage` MORE with `ModFlag.Area`, `sup_int.lua` 1104.
  - `support_concentrated_effect_skill_area_of_effect_+%_final` is a `NoEffect` "area" row, as `active_skill_area_of_effect_+%_final`. Area of effect reaches no slice field (P3-8e). A P3-10 test had kept it unmapped and now expects `NoEffect`.

**Evidence** (`b-ct`, against P6-11ge's run):
- **Two new mechanic fixtures:** Ice Nova alone from a magic body armour. All compared fields match in each:
  - `ice-nova-item-concentrated-effect-witch.xml`: "Socketed Gems are Supported by Level 20 Concentrated Effect";
  - `ice-nova-item-increased-critical-damage-witch.xml`: "... supported by Level 20 Increased Critical Damage", in lowercase as the game writes it.
- **Public:** 0 unexpected. Four values of the Vortex build moved, all toward PoB.
- **Corpus:** 0 unexpected. Six values moved, all toward PoB; one build's life cost now equals PoB's (194).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`, the item support test among them), clippy, and rustfmt pass.

## P6-11gg: the offerings' effect, and offerings that also affect you

**Found:** three stats were unmapped, so reasons:
- `offering_spells_effect_+%` ("Your Offerings have #% increased Effect"), from the minion offence mastery, in 3 builds;
- the Necromancer's Mistress of Sacrifice: `offerings_also_buff_you` ("Your Offering Skills also affect you") and `self_offering_effect_+%` ("Your Offerings have #% reduced Effect on you"), in 3 builds.

Until now the offerings' buffs reached minions alone (P6-9e, `BuffStat::MinionOnly`).

**PoB:**
- **The effect.** PoB's tree text "#% increased effect of Offerings" is `BuffEffect` INC tagged with the four offerings' names (`ModParser.lua` 486). It scales the buff on you and on minions (`CalcPerform.lua` 2274, 2284).
- **"Your Offering Skills also affect you"** is an `ExtraSkillMod` that sets the offerings' `buffNotPlayer` to false (2923). Their buff then reaches you as any `Buff` does (2270 to 2280).
- **"Your Offerings have #% reduced Effect on you"** is `BuffEffectOnPlayer` INC on the offerings (2924). Only the scale on you reads it, not the minion's.

**The engine:**
- **`skill_modifiers`:**
  - Two `STATS` rows, 28 in all: `Poe1Quantity::OfferingBuffEffect` and `OfferingEffectOnYou`.
  - The flag `OFFERINGS_AFFECT_YOU_STAT`, a `Flag` of `Poe1Quantity::OfferingsAffectYou`.
  - `OFFERING_SKILLS` (Blood, Bone, Flesh, and Spirit Offering).
  - `Totals::offering` and `Totals::offering_on_you`.
- **`gems`:**
  - An offering's buff scale on minions adds the offerings' effect.
  - With the flag, the buff's `MinionOnly` rows apply to you too (`add_rows`' new `minion_only_on_you`), at the offerings' effect plus the effect on you.
  - Flesh Offering's movement speed was `NoSlice` "movement". It is now a `MinionOnly` `MovementSpeed` row, since movement speed became a slice field in P6-10b.
- **The texts test** (`tests/skill_modifiers.rs`) checks both rows' game texts against the bundle.
- **Not modelled:** Spirit Offering and Blood Offering are not in the gem table. With them, the gem stays an `unmodelled-gem` reason, as before.

**Evidence** (`b-ct`, against P6-11gf's run):
- **Two new mechanic fixtures** match PoB on every compared field:
  - `minion-raise-zombie-offerings-on-you-witch.xml`: P6-9e's zombie fixture with both of Mistress of Sacrifice's lines as custom lines. The offerings act on you at half effect (block 17, spell block 34), and the zombie's buffs are unchanged.
  - `ice-nova-flesh-offering-on-you-witch.xml`: Ice Nova with Flesh Offering in another group and "Your Offering Skills also affect you". Flesh Offering's cast speed and movement speed act on you.
- **Public:** 0 unexpected. Eight values moved, all toward PoB:
  - the Animate Guardian build's full DPS (the offering effect on its minion);
  - the Forged Frostbearer build's block, spell block, mana regeneration, and effective health pool.
- **Corpus:** 0 unexpected. Seven values of one Necromancer build moved, all toward PoB. Its spell block (61) and mana regeneration (33.6) now equal PoB's.
- **Data-parity:** `uncategorized` 0.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gh: a weapon's attack speed per 8% quality

**Found:** `local_attack_speed_+%_per_8%_quality` ("#% increased Attack Speed per 8% Quality", an alternate quality's line) was unmapped, so a reason. It sits on a weapon in 3 builds.

**PoB:**
- `ModParser.lua` 5756 reads the line as `AlternateQualityLocalAttackSpeedPer8Quality` INC.
- `Item.lua` 2246 adds it to the weapon's local attack speed as `m_floor(quality / 8 x sum)`, the sum of the weapon's lines. The product is floored once, not the quality steps.

**The engine:**
- `WeaponValue::AttackSpeedPer8Quality` (`Base`) holds the line, mapped in `mapping.rs`.
- `collect` turns the weapon's lines into its local `WeaponValue::AttackSpeed` INC, `floor(quality / 8 x sum)`, beside P3-8n's elemental damage per 2% quality.

**Evidence** (`b-ct`, against P6-11gg's run):
- **Two new mechanic fixtures:** Smite with a magic Rusted Hatchet.
  - `smite-attack-speed-per-quality-templar.xml`: quality 20 and 3% per 8% quality, so 7.5 floored to 7%.
  - `smite-attack-speed-per-quality-12-templar.xml`: quality 12 and 2%, so 3%.
  - Every field matches PoB except those of Q11's rounding. PoB rounds 1.50 x 1.07 = 1.605 to 1.61 (1.55 for 3%). The game floors the attack time to 623 ms (647 ms), so 1.6051 (1.5456).
  - These are registered as Q11 `game-truth` entries DIV-0496 to DIV-0508, as the base fixture `smite-axe-lines-templar.xml`.
- **Public:** 0 unexpected. Five values of the Elemental Hit of the Spectrum build moved toward PoB.
- **Corpus:** 0 unexpected. Four values of one build moved: its speed and DPS toward PoB.
  - Its energy shield cost per second moved away (138.6 to 140.8, PoB 100.9). It is the cost per attack times the speed, and that build's cost per attack was already above PoB's (40.0 against 28.0) for another reason, not traced here.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gi: the effect of socketed abyss jewels

**Found:** `local_socketed_abyss_jewel_effect_+%` ("#% increased Effect of Socketed Abyss Jewels", Darkness Enthroned) was a reason. Since the close-out, an abyssal slot whose parent carried it was left out whole, an `unmodelled-item` `abyssal-socket` reason. This affected 3 builds.

**PoB:**
- The line is a local `SocketedJewelEffect` INC (`ModParser.lua` 846).
- The parent item keeps `socketedJewelEffectModifier = 1 + inc / 100` (`Item.lua` 2639).
- An abyss jewel in that item's abyssal slot is added with `ScaleAddList` at that scale (`CalcSetup.lua` 956, 1167). `ScaleAddMod` keeps the integer part of a whole value scaled and rounded to two decimals, and floors a value that is not whole to one decimal.

**The engine:**
- `collect::abyssal_active` returns the scale beside whether the socket is active: the parent's lines summed.
- The jewel's lines go in through `item_lines_in`, and the effects they add are rescaled by `skill_modifiers::scale`, PoB's `ScaleAddMod`, as an active flask's are (P6-4b).
- The parent's own line is `NoEffect` "abyss-jewel-effect".
- The close-out unit test `an_abyssal_socket_should_count_by_its_parents_sockets` now expects the scale: 16 Strength at 50% gives 24.

**Evidence** (`b-ct`, against P6-11gh's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is Ice Nova with a rare belt of one abyssal socket and a rare abyss jewel (+16 Intelligence, 10 to 20 added cold damage to spells, +30 life):
  - `ice-nova-abyss-jewel-effect-25-witch.xml`: 25%, so 20 Intelligence, 12 to 25, and 37 life (12.5 and 37.5 truncated);
  - `ice-nova-abyss-jewel-effect-75-witch.xml`: 75%, so 28, 17 to 35, and 52.
- **A PoB check, kept local:** the same jewel in both abyssal sockets of a belt counts twice in PoB too (Intelligence 60 to 88), as the engine counts it (close-out).
- **Public:** 0 unexpected.
  - 17 values of the Animate Guardian build moved toward PoB. Its life (4258) and life regeneration now equal PoB's.
  - Its energy shield recharge moved away (570 to 604, PoB 412). The recharge was already above PoB's for another reason and scales with energy shield, which moved toward PoB (2673 to 2832, PoB 3218).
- **Corpus:** 0 unexpected. One Necromancer build has Darkness Enthroned with the same jewel in both sockets. 12 of its values moved toward PoB and 4 away.
  - Its life moved from 3997 to 4282, past PoB's 4142, so its maximum hits taken passed PoB's too.
  - Its belt also has "gain no maximum life from Strength" (`gain_no_maximum_life_from_strength`), still unmapped and a reason on the life field. The engine still counts Strength's 80 life (Strength 160), which PoB drops. Without it, the base life would be 1750 against PoB's 1770.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gj: Strength provides no bonus to Maximum Life

**Found:** `gain_no_maximum_life_from_strength` ("Strength provides no bonus to Maximum Life") was unmapped, a reason. One corpus Necromancer build has it on its belt.
- P6-11gi counted that build's abyss jewels. Its life then passed PoB's (4282, PoB 4142), since the engine still added Strength's 80 life that PoB drops.

**PoB:**
- `ModParser.lua` 5430 reads the line as the flag `NoStrBonusToLife`.
- `CalcPerform.lua` 496 then adds no `Life` BASE `floor(Str / 2)` from "Strength".

**The engine:**
- `Poe1Quantity::NoStrBonusToLife`, a `Flag` mapped from the stat.
- `phases::strength_life` gives no strength bonus while the flag is on. It is read by both places that add Strength's life: life itself, and the life base of life gained as energy shield (P6-11dd).

**Evidence** (`b-ct`, against P6-11gi's run):
- **Two new mechanic fixtures** match PoB on every compared field:
  - `ice-nova-no-life-from-strength-witch.xml`: Ice Nova with "+100 to Strength" and the line;
  - `ice-nova-life-as-energy-shield-no-strength-witch.xml`: P6-11dd's life-as-energy-shield fixture with the line.
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected.
  - The Necromancer build's life moved from 4282 to 4095 (PoB 4142). Its base life is now 1750 against PoB's 1770: the gap left after P6-11gi, not traced here.
  - Eleven of its values moved toward PoB, among them its maximum hits taken and leech maximums.
  - Four moved away (life regeneration, the effective health pool, and its hit counts). They were already below PoB's and fall with life.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gk: attack and cast speed while Focused

**Found:** `attack_and_cast_speed_+%_while_focused` ("#% increased Attack and Cast Speed while Focused") was unmapped, a reason, in 3 builds. The config's "Are you Focused?" (`conditionFocused`) set only a named condition, so it sat in `config::NAMED_ONLY`.

**PoB:**
- `ModParser.lua` 1731 reads "while focused" as the condition `Focused`, and the line as `Speed` INC with it.
- The condition comes only from the config check (`ConfigOptions.lua` 992). No skill or modifier PoB reads sets it.

**The engine:**
- `conditions::FOCUSED`, settled from `conditionFocused` in `conditions::CHECKS`. The input left `config::NAMED_ONLY`.
- The stat is a `STATE_ROWS` row: `Speed` INC with that condition.

**Evidence** (`b-ct`, against P6-11gj's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is Ice Nova with "20% increased Attack and Cast Speed while Focused":
  - `ice-nova-speed-while-focused-witch.xml`: the check on, cast rate 1.71;
  - `ice-nova-speed-while-focused-off-witch.xml`: the check off, 1.43.
- **Public:** 0 unexpected. Eight values of the Shock Nova build moved, all toward PoB; its speed is 7.90, PoB 8.16.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gl: Socketed Gems deal more Elemental Damage

**Found:** `local_display_socketed_gems_elemental_damage_+%_final` ("Socketed Gems deal #% more Elemental Damage") was unmapped, a reason, in 4 builds. No line yet acted on the main skill alone by the slot its gems are socketed in.

**PoB:**
- `ModParser.lua` 1237 reads the prefix "socketed gems" as a `SocketedIn` tag with the item's slot, so the line is `ElementalDamage` MORE on the skills socketed in that item.
- `ElementalDamage` scales the skill's fire, cold, and lightning damage, its hits and its damage over time.

**The engine:**
- **The state.** `GemSlot::main_skill_state` gives one state per gem slot, "the main skill's group is socketed here". `states::states` settles it from PoB's main socket group and its slot: true for that slot alone, false for the others. It stays unsettled while the skill groups are not certain.
- **The line.** `InSlot` maps the stat (`gem_levels::SOCKETED_ELEMENTAL_DAMAGE_STAT`) to `Damage` MORE, `Flags::ELEMENTAL` any, with the item's slot state. That is the condition the global "more elemental damage" rows use.

**Evidence** (`b-ct`, against P6-11gk's run):
- **Two new mechanic fixtures** match PoB on every compared field:
  - `ice-nova-socketed-elemental-damage-witch.xml`: the line on the body armour Ice Nova is socketed in, average damage 1346.27;
  - `ice-nova-socketed-elemental-damage-elsewhere-witch.xml`: the line on a helmet, which does not apply, 1035.63.
- **Public:** 0 unexpected. Three values of the Righteous Fire build moved toward PoB (its burning damage). Its Righteous Fire is socketed with the line.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gm: damage per 100 Strength; accuracy equal to twice Strength

**Found:** PoB's own parser was run on the game text of every stat still unmapped in two or more builds (local, `local/p6/tools/gm_scan.py` with `tools/oracle/data/pob_parse_lines.lua`). 60 of 123 parse; this step takes two of them, each unmapped in 2 builds:
- `damage_+%_per_100_strength` ("#% increased Damage per 100 Strength");
- `gain_accuracy_rating_equal_to_2_times_strength` ("Gain Accuracy Rating equal to twice your Strength").

**PoB:**
- **Damage.** `Damage` INC with `PerStat` `Str` `div` 100 (`ModParser.lua` 1468): `floor(Str / 100 + 0.0001)` steps. It is untagged, so it reaches hits and ailments.
- **Accuracy.** `Accuracy` BASE 2 with `PerStat` `Str` (2921), so twice Strength.

**The engine:**
- **Damage.** `Poe1Quantity::DamagePer100Strength` (`Base`) is one more `hit::DAMAGE_PER_STAT` entry: no flag, `Field::Strength`, step 100, no limit. It sits beside P6-11cw's attack damage per 450 armour, a term of the damage increase sum for hits and, through `hit::damage_modifiers`, for ailments.
- **Accuracy.** `Poe1Quantity::AccuracyFromStrength` (`Flag`) makes `hit::accuracy` add `Str x 2` to the accuracy base, beside Dexterity's term.

**Evidence** (`b-ct`, against P6-11gl's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is P3-12h's Smite fixture with "+200 to Strength" and the line:
  - `smite-damage-per-strength-templar.xml`;
  - `smite-accuracy-from-strength-templar.xml`: the hit chance is 51%, against 22% without it.
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected. Four values of one build moved toward PoB: its average damage, ignite, and DPS.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gn: Pride's aura effect and Vulnerability's curse effect

**Found** (P6-11gm's scan of the stats PoB parses): two stats were unmapped, so reasons, each in 2 builds:
- `pride_aura_effect_+%` ("Pride has #% increased Aura Effect");
- `vulnerability_curse_effect_+%` ("#% increased Vulnerability Curse Effect").

The scan's output now goes to `local/p6/scan/`, which a step's evidence run does not clear.

**PoB:**
- The first line is `AuraEffect` INC, the second `CurseEffect` INC, each with a `SkillName` tag of the skill (`includeTransfigured`).
- So each enters only that skill's aura scale or curse scale (`CalcPerform.lua` 2392 onward for an aura, 2619 onward for a curse).

**The engine:**
- **`skill_modifiers`:**
  - `NAMED_SKILL_EFFECTS` holds each stat, the skill's granted effect ID, and what it modifies (`NamedEffect::Aura` or `Curse`).
  - Two `STATS` rows, 30 in all, on `Poe1Quantity::NamedSkillEffect(i)`.
  - `Totals::named_skill` reads them.
- **`gems`:**
  - An aura's scale adds its named aura effect (`named_aura`): on you, and on the minion in `minion_buffs`.
  - A curse's scale adds its named curse effect, beside a mark's.
- **The texts test** (`tests/skill_modifiers.rs`) checks both game texts against the bundle.

**Evidence** (`b-ct`, against P6-11gm's run):
- **Two new mechanic fixtures.** All fields match PoB except the Q11 floored attack time, registered as `game-truth` DIV-0509 to DIV-0519 as the base fixtures' are:
  - `smite-pride-aura-effect-templar.xml`: P6-9e's Pride fixture with "Pride has 40% increased Aura Effect", DPS 110.87 to 111.54 in PoB;
  - `smite-vulnerability-curse-effect-templar.xml`: P6-11ds's Hextouch fixture with "40% increased Vulnerability Curse Effect", bleeding 29.01 to 42.56 in PoB.
- **Public:** 0 unexpected. Eleven values of the Cyclone and Lacerate builds moved, all toward PoB.
- **Corpus:** 0 unexpected. Six values of one build moved, all toward PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11go: life per red socket and mana per blue socket on a staff

**Found** (P6-11gm's scan): two passives' stats were unmapped, so reasons, each in 2 builds:
- `maximum_life_+%_per_red_socket_on_staff` ("#% increased maximum Life per Red Socket on equipped Staff");
- `maximum_mana_+%_per_blue_socket_on_staff`, its Mana per Blue Socket.

**PoB:**
- `ModParser.lua` 1435 reads "per red socket on equipped staff" as the `Multiplier` `RedSocketInWeapon 1` with the condition `UsingStaff`, the blue one alike.
- `Item.lua` 2207 to 2219 adds one `Multiplier:<Colour>SocketIn<slot>` per socket of that colour on the item in the slot. A swap slot counts as its set's slot.

**The engine:**
- `Poe1Quantity::PerStaffSocket` (`Base`) holds each line.
- `collect::staff_sockets` runs after the passives, items, and the config's custom lines. It turns each line into `Life` (or `Mana`) INC, the value times the main hand's sockets of that colour, with the `UsingStaff` state.
- `sockets::colour_count` counts the sockets as `sockets` reads them for socket colour quality (P3-12i). An item whose sockets it cannot count is a reason.
- **Found on the way:** a Driftwood Wand is an ambiguous base in the bundle (`unmodelled-item` `ambiguous-base`), so the wand fixture holds a Carved Wand. Not traced here.

**Evidence** (`b-ct`, against P6-11gn's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is Ice Nova with "10% increased maximum Life per Red Socket on equipped Staff" and "15% increased maximum Mana per Blue Socket on equipped Staff":
  - `ice-nova-staff-sockets-witch.xml`: a Quarterstaff with `R-R-B-G`, so life x 1.20 and mana x 1.15;
  - `ice-nova-staff-sockets-wand-witch.xml`: a Carved Wand with `R-B-B`, so neither applies.
- **Public:** 0 unexpected. Nineteen values of the Cyclone and Shock Nova builds moved, all toward PoB; nine now equal it. The Cyclone build's life is 5234 and the Shock Nova build's mana 8373, as PoB.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gp: flask lines during the effect; conditions from config checks

**Found** (P6-11gm's scan): nine more stats PoB parses, each unmapped (a reason) in 2 to 4 builds.
- **Four flask lines "during Effect":** life regeneration, mana cost, avoid stun, the effect of shock on you, and life leeched from damage taken.
- **Lines with a condition** only a config check sets:
  - consumed a corpse recently (life regeneration, and area of effect for you and your minions);
  - on caustic ground (chaos damage taken over time);
  - Ngamahu's Flames' Magmatic Strikes (physical damage as extra fire);
  - "while not Ignited, Frozen or Shocked" (armour).

**PoB:**
- **The flask lines** are the global mods with `UsingFlask`.
  - The engine's flask lines act only while the flask is active (P6-4b), so life regeneration is an alias of `life_regeneration_rate_per_minute_%` and the mana cost a row of its own.
  - Avoid stun reaches no slice field.
  - `SelfShockEffect` is read only for a shock on you (`CalcPerform.lua` 849), which only the checks `conditionShocked` and `conditionOnShockedGround` give. Both stay reasons (no engine module reads them).
  - No PoB module reads `DamageTakenWhenHitLifeLeech`. These three are `NoEffect`.
- **The conditions.** Each is a `Condition` flag set only by its check:
  - `conditionConsumedCorpseRecently`, and by implication `conditionConsumedCorpseInPast2Sec` and the count `multiplierCorpseConsumedRecently` (`ConfigOptions.lua` 1599 to 1608);
  - `conditionOnCausticGround` (1157);
  - `buffNgamahuFlamesAdvance` (1469);
  - `Ignited`, `Frozen`, `Shocked` from `conditionIgnited`, `conditionFrozen`, `conditionShocked`, `conditionOnShockedGround`. The armour line's tag is their negation.

**The engine:**
- **Flasks:** `flasks::ALIASES`, `FLASK_ROWS`, and `NO_EFFECT_LOCAL` gain the flask lines. The mana cost line left `mapping::COST_REASONS`.
- **`conditions`:**
  - `CHECKS` gains the corpse, caustic ground, and Magmatic Strikes checks, which left `config::NAMED_ONLY`.
  - The corpse count settles `CONSUMED_CORPSE_RECENTLY` too.
  - `NOT_IGNITED_FROZEN_SHOCKED` is settled true when every one of the four checks is off, false when any is on.
- **`STATE_ROWS`** gains four rows. The caustic ground row left `DOT_TAKEN_REASONS`, which is now empty.
- **`NO_EFFECT`** gains the area row.
- **The real-bundle exposure test** lists the armour row among the config-condition rows (it names shock).
- **Left for later:**
  - Ngamahu's line in a custom modifier block is unresolved on our side. The game's template has two lines, while PoB (and its unique data) writes it as one.
  - A one-line reading of a two-line template would be a new import tolerance (a new `Adapted` kind, ADR-015 Amendment 1's list), not part of a stat step.
  - The build that carries the stat gets it from a passive, by stat ID, so the row acts there.

**Evidence** (`b-ct`, against P6-11go's run):
- **Three new mechanic fixtures** match PoB on every compared field:
  - `ice-nova-config-conditions-on-witch.xml`: the corpse and caustic ground checks on, armour 35 and chaos damage taken over time x 2;
  - `ice-nova-config-conditions-off-witch.xml`: a corpse count of 2, the ignited check on, caustic ground off, armour 27 and x 1.6;
  - `ice-nova-flask-regen-mana-cost-witch.xml`: an active flask with both lines.
- **Public:** 0 unexpected; five fields went from partial to complete.
- **Corpus:** 0 unexpected.
  - Eleven values moved, all toward PoB; nine now equal it (one build's armour 3953 and its effective health pool, another's mana cost 15).
  - Two builds' life regeneration went from partial to indeterminate: their flask's regeneration line has a range the corpus run does not settle (P3-8b's rolled values).
- **Tasks:** the staff socket task was premised on PoB never setting the socket multiplier. It does, and P6-11go applies the lines as PoB does, so the task is ticked. The task on attack speed per 8% quality and the offerings' effect is ticked with what is left of it (the game's item wording of the offerings' line).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gq: armour from body armour doubled; armour against chaos; Blind's effect; Herald damage; guard timing and explosions

**Found** (P6-11gm's scan): eight stats PoB parses, each unmapped (a reason) in 2 builds:
- "Armour from Equipped Body Armour is doubled" (`damage_reduction_rating_from_body_armour_doubled`);
- "#% of Armour also applies to Chaos Damage taken from Hits" (`base_armour_%_applies_to_chaos_damage`);
- "#% increased Blind Effect" (`blind_effect_+%`, P3-12g's `BLIND_DEPENDS_ON`);
- "Herald Skills deal #% increased Damage";
- guard skills' duration and cooldown recovery;
- two explosions on kill.

**PoB:**
- **Body armour doubled.** `Armour` MORE 100 with the `SlotName` Body Armour, at most once (`globalLimit` 100). `CalcDefence.lua`'s slot loop reads it in the body armour's armour row and, under Iron Reflexes, in its evasion row counted as armour (907 to 921).
- **Armour against chaos.** `ArmourAppliesToChaosDamageTaken` BASE, the valued twin of P6-10b part 12's row.
- **Blind's effect.** `BlindEffect` INC on the enemy: Blind is `floor(-20 x (1 + inc / 100))`% less evasion and accuracy (`CalcPerform.lua` 824 to 835).
- **Herald damage.** `Damage` INC with `SkillType.Herald`.
- **Guard timing.** `Duration` and `CooldownRecovery` INC with `SkillType.Guard`. A guard's buff applies without its uptime (2298 to 2310), so neither reaches a slice field.
- **Explosions.** `ExplodeMod` and `CanExplode` make an "On Kill Monster Explosion" group of PoB's own (`CalcSetup.lua` 1405 to 1440). It is never the main skill and never in full DPS: no slice field.

**The engine:**
- **Body armour doubled.**
  - `Poe1Quantity::BodyArmourArmourDoubled` (`Flag`). `phases::primary_defence` puts the flag times 2 into the body armour row's more (`defence_modifier_with`), so its slot breakdown's "more" is PoB's 2.00.
  - Under Iron Reflexes the body armour's evasion row is doubled too.
  - That piece row is now `piece-evasion-to-armour.<slot>`, outside the `slot.` rows. PoB's armour breakdown shows only the global evasion's conversion; before, the piece row and the global row shared one name.
  - The global conversion row has a `base` node, as PoB's breakdown has.
- **Armour against chaos:** a row on P6-10b's `ArmourAppliesTo(Chaos)`.
- **Blind's effect:** `Poe1Quantity::EnemyBlindEffect` (`Increased`), read by `damage_taken`'s Blind. `BLIND_DEPENDS_ON` is empty.
- **Herald damage:** `Flags::SKILL_HERALD` from the `Herald` skill type (`skill::SKILL_TYPE_FLAGS`), and a hit row.
- **`NoEffect`:** guard timing (`guard-timing`) and the explosions (`explosion`).
- **Register:** PoB's evasion breakdown prints a piece's evasion as an evasion row under Iron Reflexes (914 to 918) though evasion is 0 and the value goes to armour. Two `pob-bug` entries, DIV-0520 and DIV-0521, with the test `iron_reflexes_should_count_a_doubled_body_armours_evasion_as_armour`.

**Evidence** (`b-ct`, against P6-11gp's run):
- **Three new mechanic fixtures:**
  - `ice-nova-body-armour-doubled-witch.xml`: a Plate Vest doubled, flat armour not, armour against chaos. Armour 554, matching PoB.
  - `ice-nova-body-armour-doubled-iron-reflexes-witch.xml`: a Scale Vest under Iron Reflexes. Armour 667 = 76 + 76 + 515, matching PoB, with the two `pob-bug` breakdown rows.
  - `smite-blind-effect-templar.xml`: P3-12g's Blind fixture with "40% increased Blind Effect", hit chance 28 as PoB's. Q11 DIV-0522 to DIV-0527.
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected. 69 values moved: 58 toward PoB (16 now equal), 11 away.
  - Two builds' hit and evade chances now equal PoB's (Blind's effect).
  - A Juggernaut build's armour went from 23938 to 30499 (PoB 30838) and its chaos damage reduction is PoB's 80%.
  - Away: that build's elemental and physical maximum hits, already above PoB's (22971 against 20364 cold), grow with its armour; and two ignites, already above PoB's, grow 0.1% with the hit chance. Neither cause is traced here.
- **Tests:** a new test `iron_reflexes_should_count_a_doubled_body_armours_evasion_as_armour`; the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gr: attack speed per enemy in close range; armour per second stationary; the curse limit from power charges

**Found** (P6-11gm's scan): three stats PoB parses, each unmapped (a reason) in 2 builds:
- "#% increased Attack Speed per Enemy in Close Range";
- "#% increased Armour per second you've been stationary, up to a maximum of 100%";
- "Your Curse Limit is equal to your maximum Power Charges".

**PoB:**
- **Attack speed.** `Speed` INC with `ModFlag.Attack`, the `Multiplier` `NearbyEnemies`, and `AtCloseRange`.
- **Armour.** `Armour` INC with the `Multiplier` `StationarySeconds` (`limit` 100 / value) and `Stationary`. Both come from the config's `conditionStationary` count.
- **The curse limit.** `CurseLimitIsMaximumPowerCharges`: the enemy's curse limit is `PowerChargesMax` instead of the `EnemyCurseLimit` sum (`CalcPerform.lua` 3156).

**The engine:**
- **`stacks`:**
  - `PerStack::AttackSpeedAtCloseRange` on P6-11v's nearby enemy count, with the `AtCloseRange` state.
  - `StackKind::StationarySeconds` from the stationary count, with `PerStack::ArmourUpTo100` (`min(value x seconds, 100)`). The input joins `stacks::INPUTS`.
- **The curse limit:** `Poe1Quantity::CurseLimitIsMaximumPowerCharges` (`Flag`). `gems`' curse slots take `charges::power_maximum` (the character's `max_power_charges` and the maximum modifiers) as the limit; a conditional flag is a reason.
- **The charges guard test** allows the row, which reads the maximum and changes no count.

**Evidence** (`b-ct`, against P6-11gq's run):
- **Four new mechanic fixtures** match PoB on every compared field:
  - `smite-attack-speed-per-close-enemy-templar.xml`: 3 enemies at close range, 30%;
  - `ice-nova-armour-per-stationary-second-witch.xml`: 4 seconds, 40%;
  - `ice-nova-armour-per-stationary-second-capped-witch.xml`: 15 seconds, 100%;
  - `ice-nova-curse-limit-power-charges-witch.xml`: P6-9b's three curses, all three applied with 3 power charges.
- **Public:** 0 unexpected. Twelve values of the Holy Flame Totem and Shock Nova Hierophant builds moved, all toward PoB (their curses).
- **Corpus:** 0 unexpected. Fifteen values moved, all toward PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gs: Arcane Surge on kill

**Found** (P6-11gm's scan): "#% chance to gain Arcane Surge when you Kill an Enemy" (`gain_arcane_surge_on_kill_chance_%`, a Hierophant passive) was unmapped, so a reason, in 2 builds.

**PoB:** `Condition:ArcaneSurge` FLAG with `KilledRecently`, as the config's `buffArcaneSurge` sets it. `CalcPerform.lua` 686 to 705 then gives Arcane Surge's cast speed and mana regeneration and sets `AffectedByArcaneSurge`.

**The engine:**
- `Poe1Quantity::ArcaneSurgeOnKill` (`Flag`).
- **`buffs`:** Arcane Surge is on with its check, an Arcane Surge Support, or the flag while killed recently. The settled conditions now reach `buffs::arcane_surge`.
- **`conditions`:** the condition `AffectedByArcaneSurge` (`conditions::ARCANE_SURGE`) is settled before the modifiers are ingested, so `conditions::settle` sets it when killed recently holds and the build has the stat.
- **`states::build_has_stat`** finds the stat among the allocated passives and mastery effects, the lines of the items in use, and the lines of the enabled custom modifier blocks.

**Evidence** (`b-ct`, against P6-11gr's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is Ice Nova with the line as a custom line:
  - `ice-nova-arcane-surge-on-kill-witch.xml`: killed recently on, cast rate 1.71;
  - `ice-nova-arcane-surge-on-kill-off-witch.xml`: killed recently off, 1.43.
- **Public and corpus:** 0 unexpected; no value moved. The two Hierophant builds do not set killed recently.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gt: lucky block if blocked recently

**Found** (P6-11gm's scan): "Chance to Block Attack or Spell Damage is Lucky if you've Blocked Recently" (`spell_and_attack_block_chance_is_lucky_if_blocked_recently`, a Gladiator passive) was unmapped, so a reason, in 2 builds.

**PoB:**
- The line is the four `<Type>BlockChanceIsLucky` FLAGs with `BlockedRecently`.
- `CalcDefence.lua` 737 to 760 rolls each effective block chance twice when its flag is on, `1 - (1 - p)^2`, for the effective health pool.

**The engine:**
- `Poe1Quantity::BlockLucky` (`Flag`), a `STATE_ROWS` row with P6-11g's `BlockedRecently`.
- The enemy hit's mitigation takes the attack and spell block chances through `lucky`, the roll P6-11al wrote for lucky suppression, now shared by both.

**Evidence** (`b-ct`, against P6-11gs's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is Ice Nova with 30% attack and 20% spell block and the line:
  - `ice-nova-lucky-block-witch.xml`: blocked recently on, effective health pool 1319;
  - `ice-nova-lucky-block-off-witch.xml`: off, 994.
- **Public:** 0 unexpected. Eight values of the Cyclone and Lacerate builds moved, all toward PoB (their effective health pools).
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gu: auras that grant attack and cast speed

**Found** (P6-11gm's scan): "Auras from your Skills grant #% increased Attack and Cast Speed to you and Allies" (`auras_grant_attack_and_cast_speed_+%_to_you_and_your_allies`, the Necromancer's Commander of Darkness) was unmapped, so a reason, in 2 builds.

**PoB:** the line is an `ExtraAuraEffect` of `Speed` INC. `CalcPerform.lua` 2394 to 2420 adds every such mod to each aura buff, scaled by that aura's multiplier and merged per aura name. It reaches you, and the minion, through each aura that affects them.

**The engine:**
- `Poe1Quantity::ExtraAuraSpeed` (`Base`).
- `gems::extra_aura_rows` adds, for each aura buff on you (not one that affects enemies, not Generosity's), a `Speed` INC row of the unconditional sum, scaled as the aura with `skill_modifiers::scale`.
- The row is merged per aura name in the buffs' map. `minion_buffs` does the same for the minion.
- A conditional sum is a reason.
- **Found on the way:** the game's text of this stat, like Ngamahu's (P6-11gp), has two lines. A one-line custom line does not resolve on our side, so the fixture allocates the notable instead.

**Evidence** (`b-ct`, against P6-11gt's run):
- **A new mechanic fixture** matches PoB on every compared field. `ice-nova-commander-of-darkness-witch.xml` is a Necromancer reserving Precision with Commander of Darkness and its path (60791, 5415, 36017; `local/p6/tools/asc_path.py`).
  - The cast rate is 1.4714 = 1.4286 x 1.03: the 3% at Precision's 5% increased aura effect (node 5415) is 3.15, truncated to 3 by PoB's `ScaleAddMod`.
- **Public and corpus:** 0 unexpected; no value moved. The two Necromancer builds' compared fields do not take it.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gv: radius jewels that change the passives in their radius

**Found** (P6-11gm's scan): three radius jewel stats were unmapped, so reasons:
- "Passive Skills in Radius also grant #% increased Physical Damage" (5 builds);
- Unnatural Instinct's "Allocated Small Passive Skills in Radius grant nothing" (3 builds);
- "Grants all bonuses of Unallocated Small Passive Skills in Radius" (3 builds).

The game has 13 "also grant" stats: `local_unique_jewel_passives_in_radius_grant_<stat>`, each the global `<stat>` at the jewel's value.

**PoB:** its jewel functions (`ModParser.lua` 6093 onward), run per node by `CalcSetup.lua` `buildModListForNode` (117 to 165):
- **"Also grant"** (`jewelOtherFuncs`, 6217 to 6282): every allocated node in the radius but keystones, sockets, class starts, and masteries gains the stat. A node a Timeless jewel conquers is skipped (6496 to 6511).
- **"Grant nothing"** (6192): an allocated small passive (`Normal`) in the radius loses all its mods, additions from other jewels included (the wipe follows the first pass, 131).
- **"Grants all bonuses"** (`jewelSelfUnallocFuncs`, 6362): the unallocated small passives in the radius (`extraRadiusNodeList`, 794 to 800) give their mods to the jewel. A tattooed one gives its tattoo's.
- **The radius** is `nodesInRadius` (`PassiveTree.lua` 630 to 660) with the version's ring.

**The engine:**
- **`loom_pipeline::radius_jewels`** (new, the host's): for each jewel in an allocated socket, the ring as for Timeless jewels (`loom_tree::poe1::radius::in_ring`), and the changes as `Conquest` entries:
  - added stats on the allocated nodes;
  - an emptied node for "grant nothing";
  - the unallocated small nodes' stats (a tattoo's when the spec overrides the node) added to the socket node.
- A value that does not settle, a copied node a Timeless jewel conquers, or a tattoo the data does not know is a `radius-jewel` reason on the socket.
- **`resolve_passives_with`** adds the changes after the Timeless jewels' conquests, and gives the module the spec's tattoos.
- **The 15 jewel stats** are `NoEffect` "radius-jewel" in the engine's mapping.

**Evidence** (`b-ct`, against P6-11gu's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is the P6-10a Abyss fixture's tree (socket 41263) with a rare Crimson Jewel:
  - `ice-nova-radius-also-grant-witch.xml`: Medium, +5 life and 10% increased cold damage per allocated passive, life 1346;
  - `ice-nova-unnatural-instinct-witch.xml`: Small, with 55247 and 63799 left unallocated so their stats are copied, life 1198 (1130 without the copy line, in PoB too).
- **PoB checks, kept local:** PoB with and without each Unnatural Instinct line on three public builds. The copy adds the same in PoB as in the engine: 2% spell block, 10 Dexterity, 9% movement speed.
  - A tattooed unallocated node (7112 in the Pathfinder build, Tattoo of the Tasalio Warrior) first gave its own +10 Dexterity. It now gives the tattoo's line, as PoB does.
- **Public:** 0 unexpected.
  - 98 values moved: 93 toward PoB (17 now equal), 5 away.
  - The away ones are older base gaps that the copy no longer hides. The Hierophant's spell block is 70 in PoB and 72 here; it is 68 in PoB without the copy and 70 here. The Facebreaker build's movement speed is already 6% above PoB's without the copy. Mana unreserved is far from PoB's in two builds (154 against 42). None is traced here.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gw: flask duration and instant recovery

**Found** (P6-11gm's scan): three flask stats were reasons, each in 2 builds:
- "#% increased Flask Effect Duration" (`flask_duration_+%`);
- a recovery flask's own "#% increased Duration" (`local_flask_duration_+%`, which P6-4b left a reason "until recovery is computed");
- "Instant Recovery" (`local_flask_recovers_instantly`).

**PoB:**
- **The durations** are read only by a recovery flask's recovery (`CalcPerform.lua` 1576 to 1643): the `LifeRecovery` and `ManaRecovery` rates, which no slice field reads.
- **Instant recovery** is the flask's `instantPerc`. A flask that recovers all at once sets no `UsingFlask`, `UsingLifeFlask`, or `Using<base>` condition (1733 to 1748).

**The engine:**
- **The durations** are `NoEffect` on every flask (`flasks::DURATION_STATS`), and the global stat is `NoEffect` "flask-duration".
- **Instant recovery** (`flasks::INSTANT_STAT`) is `NoEffect` as a line. `flasks::using_flask` and `using_life_flask` now leave out an active flask that has it (`using_slots`).
- **The test** `active_flasks_should_scale_and_merge_their_buffs` now expects a life flask's duration to leave life complete.

**Evidence** (`b-ct`, against P6-11gv's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is P6-11dz's "during any Flask Effect" fixture with an active Divine Life Flask:
  - `ice-nova-instant-life-flask-witch.xml`: "Instant Recovery", not in use, average damage 1062.66;
  - `ice-nova-life-flask-in-use-witch.xml`: in use, 1263.93.
- **Public and corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gx: Arcane Surge per mana spent; the off hand's more attack speed

**Found** (P6-11gm's scan): two stats PoB parses were reasons:
- "#% increased Effect of Arcane Surge on you per 200 Mana spent Recently, up to 50%" (Arcane Capacitor, 2 builds);
- the runegraft's "#% more Attack speed with Offhand" (3 builds).

**PoB:**
- **Arcane Surge.** `ArcaneSurgeEffect` INC with the `Multiplier` `ManaSpentRecently` (`div` 200, `limit` 50, `limitTotal`), from the config's `multiplierManaSpentRecently` (`ConfigOptions.lua` 1358).
- **Off-hand speed.** `Speed` MORE with `ModFlag.Attack`, `OffHandAttack`, and the `Attack` skill type: the off hand's pass alone, in its speed's more product before PoB rounds the modifier.

**The engine:**
- **Arcane Surge.** `stacks::StackKind::ManaSpentRecently` from the config's count (it left `config::NAMED_ONLY`): `PerStack::ArcaneSurgeEffect` per 200, at most 50, which `buffs`' Arcane Surge reads (P6-11ch).
- **Off-hand speed.** `Poe1Quantity::OffHandSpeedMore` (`More`). `hit::attack_rate` takes the hand, and for the off hand `rate` multiplies its more product by it before the rounding.
- **Magmatic Strikes.** P6-11gp's row now has a fixture, through an anoint ("Allocates Magmatic Strikes"). The game's two-line text does not fit a custom line on our side.

**Evidence** (`b-ct`, against P6-11gw's run):
- **Four new mechanic fixtures.** All fields match PoB but the dual wielding one's Q11 floored attack times (DIV-0528 to DIV-0534, as its base fixture's):
  - `ice-nova-arcane-capacitor-witch.xml`: 700 mana spent, 30%;
  - `ice-nova-arcane-capacitor-capped-witch.xml`: 2000, the 50% cap;
  - `smite-dual-wield-offhand-speed-templar.xml`;
  - `smite-magmatic-strikes-templar.xml`.
- **Public:** 0 unexpected. Ten values of the Shock Nova build moved, all toward PoB; its speed is now PoB's 8.164.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gy: Covered in Ash and Covered in Frost

**Found** (P6-11gm's scan): the rings' "Left Ring slot: Cover Enemies in Ash for # seconds when you Ignite them" and "Right Ring slot: Cover Enemies in Frost ... when you Freeze them" were reasons (2 builds each). The config's "Is the enemy covered in Ash/Frost?" checks were unmodelled inputs.

**PoB:**
- **The lines.** `CoveredInAshEffect` or `CoveredInFrostEffect` BASE 20 (the line's number is its duration), with `SlotNumber` 1 or 2 (the item in `Ring 1` or `Ring 2`) and the enemy's `Ignited` or `Frozen`.
- **The checks.** `conditionEnemyCoveredInAsh` and `...Frost` give BASE 20 (`ConfigOptions.lua` 1980, 1983). The enemy's `Ignited` and `Frozen` come only from `conditionEnemyIgnited` and `conditionEnemyFrozen` (1852, 1898).
- **The effect.** `CalcPerform.lua` 916 to 923 gives the enemy `FireDamageTaken` (`ColdDamageTaken`) INC `min(effect, 20)`.

**The engine:**
- `Poe1Quantity::CoveredInRing` (`Base`).
- **`damage_taken::covered`:** the check's 20, plus 20 for each line in its ring (`collect::items_in_use`) while its ailment's check is on, capped at 20, as `EnemyTypeDamageTaken` INC.
- **Inputs:** the module now handles the two covering checks and `conditionEnemyFrozen`. That check sets only the enemy's `Frozen`, which no mapped row reads.

**Evidence** (`b-ct`, against P6-11gx's run):
- **Two new mechanic fixtures** match PoB on every compared field. Each is Ice Nova with a frost ring and the enemy frozen:
  - `ice-nova-covered-in-frost-witch.xml`: right ring, average damage 1242.75;
  - `ice-nova-covered-in-frost-left-ring-witch.xml`: left ring, which does not apply, 1035.63.
- **Public:** 0 unexpected; no value moved.
- **Corpus:** 0 unexpected. Five values of one build moved, all toward PoB (its average damage and ignite).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11gz: curses cast by you; physical damage as a random element; exposure if a curse was cast

**Found** (P6-11gm's scan): reasons, in 2 or 3 builds each:
- "Gain #% of Physical Damage as a Random Element if you've cast Elemental Weakness in the past 10 seconds";
- "Inflict Fire Exposure on Hit if you've cast Flammability in the past 10 seconds", and its Frostbite and Conductivity twins.

**PoB:**
- **Cast by you.** `SelfCast<Curse>` is set for every curse skill in the list that is not `Triggered` or `Aura`, applied or not (`CalcPerform.lua` 2603).
- **The random element** is `PhysicalDamageGainAsRandom` BASE. In the default `physMode` `AVERAGE` it is a third each as fire, cold, and lightning; in `FIRE`, `COLD`, or `LIGHTNING` it is all one type (`CalcOffence.lua` 944 to 965).
- **The exposure** is the enemy's `<Type>Exposure` BASE -10 with `SelfCast<Curse>` (`ModParser.lua` 4236 to 4239).

**The engine:**
- **The conditions.** `gems::self_cast_curses` (the gem table's curses in the enabled groups whose types lack `Triggered` and `Aura`) settles `conditions::self_cast(<curse>)` for every curse of the table, in `conditions::settle`.
- **The random element.** `Poe1Quantity::GainAsRandom` (a `STATE_ROWS` row with `SelfCastElementalWeakness`). `collect::random_gains` splits it, with its conditions, into `GainAs(Physical, <type>)` by the config's `physMode`; another value is a reason.
- **The exposure.** `Poe1Quantity::ExposureIfSelfCast(<type>)`, three `STATE_ROWS` rows. `exposure` takes the settled states and adds -10 as a candidate while the row's state holds. The three left `exposure::DEPENDS_ON`; the real-bundle exposure test lists them among the condition rows.
- **Found on the way:** the game's Frostbite line reads "Cold Exposure on Hit if you've cast Frostbite in the past 10 seconds" (no "Inflict"), which PoB's pattern does not match. No build has it, so no fixture of it either side would read alike.

**Evidence** (`b-ct`, against P6-11gy's run):
- **Two new mechanic fixtures:**
  - `smite-random-element-elemental-weakness-templar.xml`: P3-12h's Smite with Elemental Weakness, 30% as a random element. It matches PoB on every field.
  - `burning-arrow-flammability-exposure-templar.xml`: Burning Arrow with Flammability and the fire exposure line. It matches PoB but the base fixture's Q11 floored attack time (DIV-0535 to DIV-0540).
- **Public:** 0 unexpected. Four values of the mine Assassin build moved toward PoB (its random element). Its ignite went from not computed to partial: the fire share lets it ignite, under its other reasons.
- **Corpus:** 0 unexpected; no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ha: projectile damage as the distance travelled increases

**Found** (P6-11gm's scan): reasons, 3 builds each, kept unmapped by `config::DEPENDS_ON` since P3-12:
- Longshot's "Projectiles gain Damage as they travel farther, dealing up to #% increased Damage with Hits to targets";
- the Deadeye's Far Shot, "... up to #% more Damage with Hits and Ailments".

**PoB:**
- **The lines.** `Damage` INC with `ModFlag.Hit` and `ModFlag.Projectile`, and `Damage` MORE on hits and ailments. Each has the `DistanceRamp` tag `{35, 0}, {70, 1}` (`ModParser.lua` 2499, 2644).
- **The ramp** multiplies the value by 0 up to 35 units, 1 from 70, linear between (`Classes/ModStore.lua` 555 to 570).
- **The distance** is the skill's `skillDist`: for a melee skill the config's `meleeDistance` (placeholder 15), for any other its `projectileDistance` (placeholder 40), the input or else the placeholder (`CalcActiveSkill.lua` 446 to 451).

**The engine:**
- `Poe1Quantity::DistanceRampDamage` (`Base`).
- `collect::distance_ramps` turns each line into `Damage` at the ramp of the main skill's distance: the increased line with `HIT` and `PROJECTILE`, the more line on hits and ailments. A distance of another type is a reason.
- The two stats left `config::DEPENDS_ON`.
- Both lines' game texts have two lines, so the fixtures allocate Far Shot (a Ranger Deadeye, path 56856, 28995, 45313) and anoint Longshot.

**Evidence** (`b-ct`, against P6-11gz's run):
- **Two new mechanic fixtures.** Both match PoB but the base fixture's Q11 floored attack time (DIV-0541 to DIV-0552):
  - `burning-arrow-far-shot-ranger.xml`: distance 56, so 0.6 of 30% more;
  - `burning-arrow-longshot-templar.xml`: distance 70, the full 60% increased.
- **Public:** 0 unexpected. Four values of the Elemental Hit Deadeye build moved toward PoB.
- **Corpus:** 0 unexpected. Eleven values moved, 8 toward PoB and 3 away.
  - The away ones are the "Lightning Arrow" build's average damage and two ignites. That build's tree is 3_23, where Far Shot was the old mechanic (PoB's `FarShot` flag: 20% less to 60% more damage over distance), while the engine, with no 3_23 tree pack, reads the 3_29 node.
  - PoB's own run of that build at distances 35, 40, and 70 shows the gap is the tree version's: ours gains 42% from 35 to 70, PoB's 52%.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hb: projectile speed also applies to bow damage

**Found** (P6-11gm's scan): the bow mastery's "Increases and Reductions to Projectile Speed also apply to Damage with Bows" (`arrow_speed_additive_modifiers_also_apply_to_bow_damage`) was a reason in 3 builds. "#% increased Projectile Speed" was `NoEffect` (P6-11f: no slice field read it).

**PoB:** `ProjectileSpeedAppliesToBowDamage` (`CalcOffence.lua` 645 to 651) copies each `ProjectileSpeed` INC, tabulated with no skill flags, as `Damage` INC with `ModFlag.Bow` and `ModFlag.Hit`.

**The engine:**
- "#% increased Projectile Speed" is `Poe1Quantity::ProjectileSpeed` (`Increased`), which only this flag reads.
- The line is `Poe1Quantity::ProjectileSpeedToBowDamage` (`Flag`).
- `hit::damage_per_stat` adds the projectile speed's sum, read with no flags, as one more term of the damage increase of a bow hit while the flag is on. Ailments (no `HIT`) do not take it, as in PoB.

**Evidence** (`b-ct`, against P6-11ha's run):
- **A new mechanic fixture**, `burning-arrow-projectile-speed-damage-templar.xml`: Burning Arrow with the line and "30% increased Projectile Speed". PoB's DPS is 6.97, 5.34 in the base fixture. It matches PoB but the base fixture's Q11 floored attack time (DIV-0553 to DIV-0558).
- **Public:** 0 unexpected. Four values of the Elemental Hit Deadeye build moved toward PoB.
- **Corpus:** 0 unexpected. Eight values moved: 6 toward PoB, 2 away (the two Light Arrow builds' average damage).
  - PoB's run of "Example Light Arrow" with and without its bow mastery: the mastery adds 24.8% there and 25.0% here.
  - The away moves are that build's base gap: 62701 here against 52284 in PoB without the mastery, its tree 3_22.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hc: the enemy's fire resistance against damage over time while stationary

**Found** (P6-11gm's scan): the Chieftain's Ramako, Sun's Light, "Nearby Enemy Monsters' Fire Resistance against Damage over Time is #% while you are Stationary" (`nearby_enemies_fire_dot_resistance_is_%_while_you_are_stationary`), was a reason in 3 builds.

**PoB:** the enemy's `FireResist` OVERRIDE with `ModFlag.Dot` and the player's `Stationary` condition. `calcResistForType` reads `enemyDB:Override(cfg, "FireResist")` in place of the enemy's sum; the maximum and the floor still apply.

**The engine:**
- `Poe1Quantity::EnemyResistOverride(t)` (`Base`, the value) and `EnemyResistOverrideOn(t)` (`Flag`), both with `Flags::DOT` and `conditions::STATIONARY`, as one `STATE_ROWS` row.
- `hit::enemy_resistance`: where the flag is on in the context, the override takes the sum's place (exposure included), before the maximum and the floor.

**A data gap found** (for the maintainer): in build 26530219's `PassiveSkills`, Ramako has no stats, nor do Void Beacon, Augury of Penitence, and Unwavering Crusade (the "nearby enemies" ascendancy auras; the game probably grants them as buffs), while tree packs 3_25 to 3_28 and PoB's 3_29 export carry them. A build on 3_29 with these nodes keeps their lines out of the calculation. The fixtures use tree 3_28.

**Evidence** (`b-ct`, against P6-11hb's run):
- **Two new mechanic fixtures**, both matching PoB on every field:
  - `ice-nova-ignite-ramako-marauder.xml`: stationary for 2 seconds; PoB's ignite DPS 357.70;
  - `ice-nova-ignite-ramako-moving-marauder.xml`: not stationary; 149.04.
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. Three values moved, all toward PoB (two Chieftain builds' full and ignite DPS).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hd: attack speed per 150 accuracy rating

**Found** (P6-11gm's scan): the notable's "1% increased Attack Speed per 150 Accuracy Rating" (`attack_speed_+%_per_200_accuracy_rating`; the stat ID keeps an older 200) was a reason in 2 builds.

**PoB:** `Speed` INC with `ModFlag.Attack` and `PerStat` `Accuracy` 150 (`ModParser.lua` 1490). `GetStat` falls back to the pass's `cfg.skillStats`, which is the hand's output (`CalcOffence.lua` 1943, 1960), so the hand's own final accuracy is read: `floor(accuracy / 150 + 0.0001) x value`.

**The engine:**
- `Poe1Quantity::AttackSpeedPerAccuracy` (`Base`).
- `hit::attack_rate` computes the hand's accuracy when the store has it and adds the per-step term to the speed's increase (`hit::per_value`, split out of `per_stat`; `rate` takes its computed extras as `RateExtras`).

**Evidence** (`b-ct`, against P6-11hc's run):
- **Two new mechanic fixtures**, both matching PoB on every field: `smite-attack-speed-per-accuracy-templar.xml` (806 accuracy, so 5%; PoB's speed 1.1475 against 1.105) and its base `smite-accuracy-no-speed-templar.xml`.
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. Twelve values moved: 10 toward PoB, 2 away (one build's speed and mana cost per second).
  - That build attacks with its off hand and has 5037 accuracy here against PoB's 4212, so 33 steps against 28. Its speed was already above PoB's (2.57 against 2.0 without the notable, PoB's run of the build with the node removed).
  - PoB's run with and without the node: the speed gains ×1.194 there and ×1.225 here, the difference being the accuracy gap.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11he: shield attacks

**Found** (P6-11gm's scan): the shield attack skills' own stats were reasons in 4 builds (`off_hand_local_<minimum|maximum>_added_physical_damage`, and the added damage per 15 of the shield's armour and evasion). The engine computed those skills with the main hand's weapon.

**PoB:**
- **The skill** (`CalcActiveSkill.lua` 269 to 279): a `shieldAttack` skill needs a shield, attacks with the off hand only (`weapon2Attack`), its weapon flags 0, and the main hand does not decide between melee and projectile.
- **The weapon** (`CalcOffence.lua` 1957 to 1983): a shield has no weapon data (`Item.lua` 2488), so the off hand's source is the skill's `setOffHand<Type>Min` and `Max`, the level's `critChance`, and `1000 / attackTime`. No local increase or quality reaches it.
- **The added damage** (`SkillStatMap.lua` 1675 to 1697): `<Type>Min` and `Max` BASE with `PerStat` `ArmourOnWeapon 2`, `EvasionOnWeapon 2`, or both, div 15. Those are the shield's own armour and evasion before global modifiers (`CalcDefence.lua` 496, 500), and the damage is added damage (times damage effectiveness).
- **Multistrike and Greater Multistrike** (`SkillStatMap.lua` 297; `sup_str.lua` 3960 to 3974, 4044 to 4060, 4113 to 4130): the repeat count, the more attack speed, and each repeat's damage each have a second entry for `SkillType.RequiresShield`.

**The engine:**
- `hit::attack_skill` takes a shield attack (`shieldAttack` in PoB's skill flags) with the shield in the off hand (`collect::shield`); its part comes from `attack_part`, split out of `attack_skill`.
- `hit::shield_weapon_data` is the weapon: `Poe1Quantity::ShieldAttackDamage`, the level's crit (no `skill-crit-chance` reason), and `1000 / attackTime`.
- `hit::shield_added` adds `Poe1Quantity::ShieldAddedDamage` per 15 of `phases::piece` (the shield's own armour, evasion, or both) to the added damage.
- `Flags::SKILL_REQUIRES_SHIELD` (from the `RequiresShield` type) carries the Multistrike entries.
- `attack_is_melee_override` is `NoEffect` (PoB's stat map has no entry; the melee flag is its skill data's).
- The shield skills stay `Partial` (`pob-skill-data`): Shield Crush's distance damage (`MeleeProximity`), Spectral Shield Throw's shards and chains.

**Evidence** (`b-ct`, against P6-11hd's run):
- **Three new mechanic fixtures**, all matching PoB on every field:
  - `shield-crush-templar.xml`: a Splendid Round Shield of quality 20;
  - `shield-crush-defences-templar.xml`: the shield's own armour and evasion raised, while a global armour line does not reach the added damage;
  - `shield-crush-multistrike-templar.xml`: Multistrike's count, speed, and repeat damage.
- Spectral Shield Throw with the same shield matches PoB too (`local`, not a fixture: none of its fields is `Complete`).
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. 32 values moved, 26 toward PoB and 6 away.
  - The away ones are two Shield Crush builds' speed, mana per second, and one build's crit. Their old values came from the main-hand weapon.
  - A PoB probe of one build's off-hand `Speed` mods (`local/p6/tools/pob_tab.lua`) shows the mores equal here (1.5 x 0.9 x 1.1). The increase lacks Soul Eater's 210% (its item stats are reasons) and 5% of the per-accuracy line (P6-11hd's accuracy gap).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hf: Soul Eater

**Found** (P6-11he's probe of a Shield Crush build): PoB's off-hand speed had 210% increased from Soul Eater. The build's "Eat a Soul when you Hit a Rare or Unique Enemy, no more than once every 0.5 seconds" and "+# to maximum number of Eaten Souls" were reasons (2 builds).

**PoB:**
- **Can have Soul Eater:** `Condition:CanHaveSoulEater` from "Soul Eater", "Eat a Soul when you Hit a Rare or Unique Enemy, no more than once every # seconds", and Double Strike's or Dual Strike's chance (`ModParser.lua` 3975, 4305, 4306).
- **The souls:** the config's `multiplierSoulEater` (`ConfigOptions.lua` 986: `Multiplier:SoulEaterStack` in combat).
- **The effect** (`CalcPerform.lua` 935 to 938): `Speed` INC 5 per soul with `ModFlag.Attack`, and again with `ModFlag.Cast`. The multiplier's `limit` is `SoulEaterMax`: the character's `soul_eater_maximum_stacks` (`CalcSetup.lua` 49) plus "+# to maximum number of Eaten Souls".

**The engine:**
- `stacks::StackKind::SoulEater`: the three lines as `CanGain`, "+# to maximum number of Eaten Souls" as `Maximum`.
- `stacks::stacks` reads `stacks::SOUL_EATER_INPUT` (no longer in `config::NAMED_ONLY`), and adds `5 x min(floor(souls), maximum)` as attack and cast `Speed` increases.
- Not mapped, so still reasons:
  - the timed lines PoB reads with a condition (during a flask effect, after a Vaal skill, on killing blow);
  - "Maximum # Eaten Soul";
  - "Eat a Soul when you Kill a Rare or Unique Enemy with this Weapon".
  - PoB leaves the last two unparsed, as it does the hit line at exactly one second ("once every second").

**Evidence** (`b-ct`, against P6-11he's run):
- **Two new mechanic fixtures**, both matching PoB on every field, each with "Soul Eater", "+10 to maximum number of Eaten Souls", and 60 souls in the config (so the maximum of 55 applies): `smite-soul-eater-templar.xml` (attack speed) and `ice-nova-soul-eater-witch.xml` (cast speed).
- **Public:** 0 unexpected, no value moved. The oracle record of P6-11he's `shield-crush-multistrike-templar.xml` was rerun for its corrected comment.
- **Corpus:** 0 unexpected. Nine values moved, all toward PoB (the two Shield Crush builds' speed, now 7.80 against 8.16 and 8.77 against 8.87, their mana per second and DPS).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hg: crit per 10 maximum energy shield on the shield

**Found** (P6-11gm's scan): reasons in 3 builds each:
- "+#% to Off Hand Critical Strike Chance per 10 Maximum Energy Shield on Shield";
- "+#% to Off Hand Critical Strike Multiplier per 10 ...";
- "+#% to Critical Strike Multiplier per 10 Maximum Energy Shield on Shield".

**PoB** (`ModParser.lua` 1503): `CritChance` and `CritMultiplier` BASE with `PerStat` `EnergyShieldOnWeapon 2`, div 10, the shield's own maximum energy shield before global modifiers (`CalcDefence.lua` 492). The off-hand lines add `Condition:OffHandAttack` (the off hand's `weapon2Cfg`, `CalcActiveSkill.lua` 476) and `SkillType.Attack`. The crit chance stat is in hundredths.

**The engine:**
- `Flags::OFF_HAND_ATTACK`, PoB's `OffHandAttack`, is set in the off hand's context of a dual-wielding attack and in a shield attack's context.
- `Poe1Quantity::CritPerShieldEnergyShield` (`Base`): the off-hand lines with `ATTACK` and `OFF_HAND_ATTACK`, the other without conditions.
- `hit::crit_chance` and `hit::crit_multiplier` add `floor(es / 10 + 0.0001) x value` of `phases::piece` (the shield's energy shield) to their bases (`hit::per_shield_energy_shield`).

**Evidence** (`b-ct`, against P6-11hf's run):
- **Two new mechanic fixtures**, each with an Archon Kite Shield of quality 20 and the three lines:
  - `shield-crush-crit-per-shield-es-templar.xml`: all three apply to the off-hand attack. It matches PoB on every field.
  - `smite-crit-per-shield-es-templar.xml`: only the line without "Off Hand" reaches the main hand. It matches PoB but the base fixture's Q11 floored attack time (DIV-0559 to DIV-0564).
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. 18 values moved, all toward PoB, 4 now equal (three Shield Crush builds' crit multiplier and one crit chance).
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hh: attack damage per 75 armour or evasion rating on the shield

**Found** (P6-11he's ShieldDB build): "#% increased Attack Damage per 75 Armour or Evasion Rating on Shield" (`attack_damage_+%_per_75_armour_or_evasion_on_shield`) was a reason in 1 build.

**PoB** (`ModParser.lua` 1518): `Damage` INC with `ModFlag.Attack` and `PerStat` `statList` `ArmourOnWeapon 2`, `EvasionOnWeapon 2`, div 75: the shield's own armour and evasion summed.

**The engine:**
- `Poe1Quantity::DamagePerShieldDefences` (`Base`, `ATTACK`).
- `hit::damage_per_stat` adds `floor((armour + evasion) / 75 + 0.0001) x value` of `phases::piece` to the hit's damage increase.

**Evidence** (`b-ct`, against P6-11hg's run):
- **A new mechanic fixture**, `smite-damage-per-shield-defences-templar.xml`: Smite with a Splendid Round Shield of quality 20 and the line at 2%. It matches PoB but the base fixture's Q11 floored attack time (DIV-0565 to DIV-0570).
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. Five values of the ShieldDB build moved, all toward PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hi: brutal charges

**Found** (P6-11gm's scan): Arn's Anguish's three brutal charge lines were reasons in 2 builds.

**PoB:**
- **The conversion** (`CalcPerform.lua` 974, 975, 1042 to 1046): with `EnduranceChargesConvertToBrutalCharges`, the brutal charges are `max(endurance charges, min(brutal maximum, brutal minimum))`, and the endurance charges are 0.
  - The brutal maximum is the endurance maximum with `MaximumEnduranceChargesEqualsMaximumBrutalCharges`, else 0.
  - The brutal minimum is the endurance minimum with `MinimumEnduranceChargesEqualsMinimumBrutalCharges`, else 0.
  - `HaveMaximumEnduranceCharges` then sets the endurance charges back to their maximum (1078 to 1080).
- **Triple damage** (3069 to 3073): with the charges used, `TripleDamageChance` BASE is the character's `chance_to_deal_triple_damage_%_per_brutal_charge` per brutal charge.
- **In the hit** (`CalcOffence.lua` 3192 to 3208): `T = min(T, 100)`, the double damage chance becomes `max(D - T x D / 100, 0)`, and the scaled damage effect is `1 + D / 100 + 2 x T / 100`.

**The engine:**
- `charges::ChargeValue::ToBrutal`, `BrutalMaximumIsEndurance`, and `BrutalMinimumIsEndurance` on endurance charges.
- `charges::charges` counts the brutal charges, adds `Poe1Quantity::TripleDamageChance` when they are used, and gives the endurance charges no effect unless the character counts as having the maximum.
- `hit::double_damage` applies PoB's triple damage terms.

**Evidence** (`b-ct`, against P6-11hh's run):
- **Three new mechanic fixtures**, all matching PoB on every field:
  - `smite-brutal-charges-templar.xml`: three brutal charges used, with a 20% double damage chance;
  - `smite-brutal-charges-minimum-templar.xml`: a minimum endurance charge, unused, so no endurance charge and no triple damage;
  - `smite-brutal-charges-maximum-templar.xml`: counting as maximum endurance charges, so they stay.
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. Seven values moved, all toward PoB: two Shield Crush builds' damage.
  - A first version without the maximum's reset moved 54 defence values away. PoB's probe of the build (`local/p6/tools/pob_out.lua`) showed 8 endurance and 8 brutal charges, which led to the reset.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hj: Transfiguration of Body, Mind, and Soul

**Found** (P6-11gm's scan): "Transfiguration of Mind" (`additive_mana_modifiers_apply_to_damage_at_30%_value`) was a reason in 2 builds. Body and Soul have the same form.

**PoB** (`CalcOffence.lua` 511 to 519): with `TransfigurationOfBody`, `Mind`, or `Soul`, `Damage` INC `floor(<Life|Mana|EnergyShield> INC x 0.3)` (`data.misc.Transfiguration`), the pool's increases with no skill flags. Body's is for attacks, Mind's for any hit, Soul's for spells. Energy shield's increases include intelligence's `floor(Int / 10)` (`CalcPerform.lua` 518).

**The engine:**
- `Poe1Quantity::Transfiguration` (`Flag`).
- `hit::damage_per_stat` adds each Transfiguration's floored share to the hit's damage increase. Soul's sum adds the intelligence bonus (`phases::bonus`), which the engine applies in the defence phase rather than through the store.

**Evidence** (`b-ct`, against P6-11hi's run):
- **Two new mechanic fixtures**, both matching PoB on every field:
  - `ice-nova-transfiguration-witch.xml`: Mind and Soul. A first version without the intelligence bonus was one percent short.
  - `smite-transfiguration-templar.xml`: Body and Mind, and Soul's not applied to the attack.
- **Public:** 0 unexpected. Three values of `shock-nova-archmage-hierophant.xml` moved toward PoB.
- **Corpus:** 0 unexpected, no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hk: Arcane Might (spell damage increases on attacks)

**Found** (P6-11gm's scan): Kinetic Blast of Clustering's and Kinetic Fusillade's "#% Arcane Might" (`active_skill_additive_spell_damage_modifiers_apply_to_attack_damage_at_%_value`) was a reason in 2 public builds.

**PoB** (`CalcOffence.lua` 618 to 631; `SkillStatMap.lua` 910; `ModParser.lua` 3413 to 3416): with `SpellDamageAppliesToAttacks`, each `Damage` INC whose flags include `ModFlag.Spell` is copied as an attack's at the largest `ImprovedSpellDamageAppliesToAttacks` (`MAX`, 100 without one) over 100.

**The engine:**
- `Poe1Quantity::SpellDamageToAttacks` (`Base`, the percent). It comes from the skill stat, "Attacks have 100% Arcane Might", and "Attacks have 150% Arcane Might".
- `hit::spell_damage_to_attacks` adds the spell-only increases at the largest share to an attack hit's damage increase.
  - The spell-only increases are the hit's context with `ATTACK` swapped for `SPELL`, minus the context without either.
  - Approximation: the engine's `SPELL` is PoB's `ModFlag.Spell` and `KeywordFlag.Spell` both, so an increase that needs only the spell keyword is copied here and not in PoB. None of the fixtures or builds that moved has one.
- Not mapped: the wand and retaliation variants (conditions), and "Modifiers to Spell Damage also apply to Attack Damage" (PoB leaves the text unparsed).

**Evidence** (`b-ct`, against P6-11hj's run):
- **Two new mechanic fixtures**, both matching PoB on every field:
  - `smite-arcane-might-templar.xml`: 150% with spell, generic, and attack increases;
  - `smite-arcane-might-largest-templar.xml`: 100% and 150%, so the largest counts.
- **Public:** 0 unexpected. Ten values of the two Kinetic builds moved toward PoB.
- **Corpus:** 0 unexpected, no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hl: Primal Aegis

**Found** (P6-11gm's scan): the Elementalist's Bastion of Elements had two reasons in 155 corpus documents: "Triggers Level # Primal Aegis when Allocated" (`local_display_cast_primal_aegis_on_gain_skill`) and "Other Aegis Skills are Disabled".

**PoB:**
- **The value:** "Primal Aegis can take 75 Elemental Damage per Allocated Notable Passive Skill" is `ElementalAegisValue` MAX 75 per `Multiplier:AllocatedNotable` (`ModParser.lua` 2828; `CalcSetup.lua` 677: the spec's allocated nodes of type `Notable`). It becomes `sharedElementalAegis` (`CalcDefence.lua` 2419).
- **The hit pools** (3135): an elemental type's total hit pool adds it.
- **The EHP** (`reducePoolsByDamage`, 285 to 290): an elemental hit drains it before guard, all of the hit up to what is left.
- "Other Aegis Skills are Disabled" disables `SkillType.Aegis` but Primal Aegis.

**The engine:**
- `Poe1Quantity::PrimalAegis` (`Flag`). `collect::primal_aegis` turns it into `Poe1Quantity::ElementalAegis`: 75 per allocated, connected notable. The 75 is the game's description text; the stat's value is the level. A node of unknown kind is a reason.
- `enemy_hit::maximum_hit`'s pools and `ehp::reduce` add the shared elemental aegis for elemental types.
- "Other Aegis Skills are Disabled" is `NoEffect`: no other aegis skill is modelled, and each keeps its own reason.
- A type's own aegis and the shared aegis have no modelled source.

**Evidence** (`b-ct`, against P6-11hk's run):
- **A new mechanic fixture**, `ice-nova-primal-aegis-witch.xml`: an Elementalist with Bastion of Elements (path 16023, 6052, 4917; one notable). It matches PoB on every field. PoB's probe gives `sharedElementalAegis` 75 and fire's total hit pool 1200 against chaos's 1125.
- **Public:** 0 unexpected, no value moved.
- **Corpus:** 0 unexpected. Seven EHP and maximum hit values of one Elementalist build moved toward PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hm: missing unreserved life gained before an enemy hit

**Found** (P6-11gm's scan): "Gain #% of Missing Unreserved Life before being Hit by an Enemy" (`missing_unreserved_life_%_gained_as_life_before_hit`) was a reason in 3 builds.

**PoB** (`CalcDefence.lua` 2647 to 2666, 2793 to 2798): with more than one damaging hit, `DamageIn.MissingLifeBeforeEnemyHit` is the `MissingLifeBeforeEnemyHit` BASE sum. In `numberOfHitsToDie`'s mitigated run:
- before each hit, life is `min(life + X x (LifeUnreserved - life) / 100, LifeRecoverable)` (at X = 0 that is the clamp the engine already did);
- for the accelerated hits, the same times `gainMult - 1`, at most `gainMult x LifeRecoverable`;
- a value other than 0 sets `GainWhenHit`, which limits the speed-up to 4, also in the recursive run, which does not carry the value itself.

**The engine:**
- `Poe1Quantity::MissingLifeBeforeHit` (`Base`), read by `enemy_hit` into `ehp::Character::missing_life`.
- `ehp::Gains::missing_life` carries it on the mitigated run's top level; `ehp::hits_to_die` applies both steps, and `Gains::when_hit` is PoB's `GainWhenHit`.
- The tests: `gems`'s reservation-word guard and `skill_modifiers`'s coverage test know the stat is not a reservation's.

**Evidence** (`b-ct`, against P6-11hl's run):
- **Two new mechanic fixtures**, both matching PoB on every field. Each has the line at 20%, "+1000 to maximum Life", and the enemy at level 40, not a boss, so there is more than one damaging hit:
  - `ice-nova-missing-life-before-hit-witch.xml`: 58 mitigated hits against 16.6 damaging;
  - `ice-nova-missing-life-before-hit-block-witch.xml`: with block as well.
- **Public:** 0 unexpected. Three EHP values of `rf-chieftain-life-stack.xml` moved toward PoB.
- **Corpus:** 0 unexpected. Six EHP values of two builds moved toward PoB.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hn: ward

**Found** (the open task "A ward pool"): ward had no pool. Its local stats on armour (`local_ward`, `local_ward_+%`) were `NoEffect`. Its global lines (`ward_+%`, `ward_from_armour_item_+%`, Heroic Tragedy's) were EHP reasons in two public builds.

**PoB:**
- **The value** (`CalcDefence.lua` 829 to 948, 1052): `output.Ward` is the sum of two parts, then `max(floor(sum), 0)`:
  - each armour piece's ward above 0 times `calcLib.mod(modDB, slotCfg, "Ward", "Defences")`;
  - the global `Ward` BASE times the same modifier without a slot.
- **The piece's ward** (`Item.lua` 2308 to 2343) is computed as its armour: the base's ward range and `WardBasePercentile`, local `Ward` and local `Defences` increases, and quality.
- **The lines:** "#% increased Ward from Equipped Armour Items" is `Ward` INC with the armour slots' `SlotName`. "#% increased Global Defences" is `Defences` INC, so it reaches ward too.
- **The EHP** (`reducePoolsByDamage`, 309 to 315):
  - Ward takes all of a hit, up to what is left, after the guards and before energy shield.
  - A hit that takes any ward breaks it unless a chance to avoid the break applies (409).
  - `numberOfHitsToDie` leaves ward out of an accelerated run (2590 to 2593).
- **The maximum hit** (3124 to 3133): each type's total hit pool adds ward.

**The engine:**
- `Defence::Ward`, outside `Defence::ALL` (the slice's defences): the piece's ward is `phases::piece`'s, from `ArmourBase::ward` (the armour types table).
- `Poe1Quantity::Ward`: rows for `base_ward`, `ward_+%`, and `ward_from_armour_item_+%` (each slot's `SlotIncreased`). It is also a row of `global_defences_+%`, `defences_+%_while_wielding_staff`, the shield's `Defences` line, and the local triple hybrid.
- `phases::ward`, read by `enemy_hit` into `ehp::Character::ward` and the maximum hit's pools.
- `ehp::reduce` drains ward and breaks it. `hits_to_die` starts an accelerated run without it.
- Still reasons:
  - "#% chance to Restore Ward when you Block";
  - the bypass lines;
  - Wicked Ward;
  - Ghostwrithe's energy shield increases applied to ward;
  - armour and evasion converted to ward.

**Evidence** (`b-ct`, against P6-11hm's run):
- **Two new mechanic fixtures**, both matching PoB on every field (P6-11hm's binary differed from PoB on 12 fields of the two):
  - `ice-nova-ward-witch.xml`: a Runic Crown and Runic Gauntlets against several hits. PoB's probe gives `Ward` 652.
  - `ice-nova-ward-boss-witch.xml`: against the default Pinnacle boss.
- **Public:** 0 unexpected. Ten EHP and maximum hit values of `poisonous-concoction-of-bouncing-pathfinder.xml` moved toward PoB.
- **Corpus:** 0 unexpected, no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ho: Frost Shield

**Found** (the open task "Frost Shield (stages, the enemy in its globe)"): with the config's `frostShieldStages` set, each of Frost Shield's values was an `unmodelled-skill` reason (P6-11fk).

**PoB** (`act_int.lua` 8014 to 8075, `CalcDefence.lua` 31 to 52, 262 to 277, 2441 to 2447, 3147 to 3160):
- **Per stage:** the spell crit chance and `FrostGlobeHealth` are counted per `Multiplier:FrostShieldStage`, at most `FrostShieldMaxStages` (the gem's 4).
- **The share:** `FrostGlobeDamageMitigation` is 20% with the enemy in the area (the config's `conditionEnemyInFrostGlobe`, an enemy `ActorCondition`), else 60%.
- **The pool:** `buildAllyLifePools` makes an ally pool when both the life and the share are above 0. It drains after the Stone Golem's: the share of each hit, up to the life left, floored.
- **The maximum hit:** the ally pools are undone after the guard in reverse order, Frost Shield first.
- **The drain:** `EnergyShieldDegen` from one stage reaches only the net regeneration (1288 to 1292, 3397), which is not a slice field.

**The engine:**
- `poe1::gems`: Frost Shield's per-stage rows are `BuffStat::PerCount` with a new `limit` (PoB's `limitVar`). The two shares are `BuffStat::EnemyCheck`, a new variant: the config check, negated for outside. The drain is `NoSlice`.
- `Poe1Quantity::FrostShieldLife` and `FrostShieldShare`.
- `enemy_hit::frost_shield` reads them into `ehp::Character` and into the maximum hit's pools. `ehp::reduce` drains the shield after the golem.
- `data-parity`'s `gem_effects` checks both new variants against PoB's mods: the `Multiplier` tag, and the enemy's `ActorCondition`. `uncategorized` stays 0.

**Evidence** (`b-ct`, against P6-11hn's run):
- **Two new mechanic fixtures**, both matching PoB on every field (P6-11hn's binary left all of their fields partial):
  - `ice-nova-frost-shield-witch.xml`: 4 stages, the enemy outside. PoB's probe gives `FrostShieldLife` 3640, mitigation 60, crit chance 9.02.
  - `ice-nova-frost-shield-in-area-witch.xml`: 6 stages counted as 4, the enemy inside (20%), against several hits.
- **Public and corpus:** 0 unexpected, no value moved. The community builds with Frost Shield set no stages.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hp: Bear Trap (a trap with a cooldown)

**Found** (the open task "... Bear Trap ..."): Bear Trap as the main skill left its hit fields not computed. `hit::spell_skill` refused a trap or a mine with a cooldown. Once it was let through, its damage was 0: Bear Trap's base damage is its `secondary_*` stats, which were unmapped.

**PoB** (`CalcOffence.lua` 1218 to 1267, 333, 5983; `CalcActiveSkill.lua` 353 to 359, 427; `SkillStatMap.lua` 30 to 89):
- **The rate:** a trap's cooldown is `TrapCooldown`, which caps nothing, so the rate is the throwing rate. With a cooldown the throw count is 1, unless the config overrides it. A mine that is not a trap sets `output.Cooldown`, which would cap its laying rate.
- **The damage:** `secondary_minimum/maximum_base_<type>_damage` are `skill("<Type>Min/Max")`, as the `spell_*` ones.
- **The flags:** `ModFlag.Spell` needs the base flag `spell`, and the spell keyword needs the Spell type without the base flag `cast`. Bear Trap's base flags are cast, duration, and trap, so spell modifiers do not reach it.
- **The debuff:** its own debuff, `TrapMineDamageTaken` INC, reaches its own hit (3488).
- **`showAverage`:** kept for a trap with a cooldown (Bear Trap's `base_skill_show_average_damage_instead_of_dps`), so `CombinedDPS` starts from `AverageDamage`.

**The engine:**
- `hit::spell_skill` lets a trap with a cooldown through. A mine-only skill with a cooldown stays not computed.
- `hit::throw_count` is 1 with a cooldown.
- The hit's spell flags follow PoB's base flags when the skill has PoB's data.
- `HIT_ROWS`:
  - the ten `secondary_*` base damage stats;
  - Bear Trap's `bear_trap_damage_taken_+%_from_traps_and_mines` as `EnemyTrapMineDamageTaken`, now a hit quantity.
- `combined::shows_average` handles a trap's or a mine's `showAverage` with a cooldown.
- Its other stats PoB reads into no output: five more in `skill_no_output::STATS`.

**Evidence** (`b-ct`, against P6-11ho's run):
- **Two new mechanic fixtures**, both matching PoB on every computed field:
  - `bear-trap-witch.xml`: PoB's probe gives `AverageDamage` 558.94, `Speed` 1.667, `TrapCooldown` 4.026, `TotalDPS` 931.56, `CombinedDPS` 558.94.
  - `bear-trap-modifiers-witch.xml`: spell damage, trap damage and throwing speed, cooldown recovery, and an additional trap.
- **Public and corpus:** 0 unexpected, no value moved. The spell flag rule moved no other skill.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hq: Fortify and Hypothermia supports

**Found** (the unmapped stats left in single builds, `unmapped.py`): Fortify Support's two stats were unmapped in three builds, and Hypothermia's two in two builds. A support's unmapped stat is a reason on every field of the skill it supports.

**PoB** (`Data/Skills/sup_str.lua` 2631 to 2636, `sup_dex.lua` 2346 to 2357, `SkillStatMap.lua` 1056):
- **Fortify:** `support_fortify_melee_damage_+%_final` is `Damage` MORE with `ModFlag.Melee`. `support_fortify_ailment_damage_+%_final_from_melee_hits` is `Damage` MORE with `ModFlag.MeleeHit` and `ModFlag.Ailment`: an ailment from a melee hit. Neither reads or changes fortification.
- **Hypothermia:**
  - `support_hypothermia_damage_+%_vs_chilled_enemies_final` is `Damage` MORE with `KeywordFlag.Hit` or `Ailment`, while the enemy is `Chilled`.
  - `support_hypothermia_cold_damage_over_time_+%_final` is `ColdDamage` MORE with `KeywordFlag.ColdDot`.
  - Its quality and constant stat `additional_chance_to_freeze_chilled_enemies_%` is `EnemyFreezeChance` BASE against a chilled enemy. No slice field reads the freeze.

**The engine:**
- `HIT_ROWS`: Fortify's two rows (`Flags::MELEE`; `Flags::MELEE_HIT` with `Flags::AILMENT`, which `poe1::ailment` gives an ailment from a melee hit) and Hypothermia's cold damage over time (as Burning Damage's row).
- `ROWS`: Hypothermia's more damage against a chilled enemy as `DamageVsChilled` MORE. `DamageVsChilled` is now a hit quantity, so the main skill's gem may carry it.
- `damage_taken`: P6-11bf's chilled gate applies the MORE too, with `KeywordFlag.Hit` or `Ailment` (the increase keeps `Hit` alone).
- `NO_EFFECT`: the freeze chance (`freeze`).
- Tests: the fortification guard (`config`) lists Fortify's two rows; the chill coverage test (`keystones_chill`, real bundle) lists Hypothermia's two stats.

**Evidence** (`b-ct`, against P6-11hp's run):
- **Three new mechanic fixtures:**
  - `ice-nova-hypothermia-chilled-witch.xml`: Ice Nova with Hypothermia against an enemy the config chills. It matches PoB on every computed field (average hit 938.42).
  - `ice-nova-hypothermia-witch.xml`: the same without the chill. It matches PoB (727.55).
  - `smite-fortify-templar.xml`: Smite with Fortify, a Rusted Hatchet, and a chance to bleed. Its only differences from PoB are the attack rate's Q11 rounding, registered as DIV-0571 to DIV-0576.
- **Public:** 0 unexpected, and 6 values move toward PoB: OccVortex's hit and damage over time, and smite-inquisitor-act5's full DPS (Fortify on its Static Strike).
- **Corpus:** 0 unexpected; 9 values move toward PoB, and 3 move away in one Vortex build where PoB gives 0 damage. Those 3 fields were already nonzero and stay partial.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hr: Combustion, against a blinded enemy, after consuming a corpse

**Found** (the unmapped stats left in single builds): Combustion Support's two stats, "#% increased Critical Strike Chance against Blinded Enemies", "#% increased Damage if you have Consumed a corpse Recently", and projectile speed on a gem were unmapped, each in two builds.

**PoB** (`Data/Skills/sup_int.lua` 928 to 941; `CalcPerform.lua` 2254, 2709 to 2720; `SkillStatMap.lua` 663, 1062, 1071, 1211, 1344, 2199; `ConfigOptions.lua` 1599, 1835):
- **Combustion's more fire damage:** `support_chance_to_ignite_fire_damage_+%_final` is `FireDamage` MORE.
- **Combustion's fire resistance:**
  - `ignites_apply_fire_resistance_+` is `CombustionFireResist` BASE on the supported skill, -10 at every level and quality.
  - `CalcPerform` reads it in its loop over every active skill. While the enemy is `Ignited` (the config), the first skill with Damage or Attack type that has Combustion and no `CannotIgnite` adds its value to the enemy's `FireResist` (a debuff named Combustion, not scaled by debuff effect). Then `appliedCombustion` stops the loop.
  - `CannotIgnite` comes from a gem's `cannot_inflict_status_ailments` (Elemental Focus), `never_any_ailment`, or `never_ignite` (Flame Surge's own stat).
- **Against blinded enemies:** `CritChance` INC with the enemy's `Blinded`, set by the config check `conditionEnemyBlinded`.
- **After consuming a corpse:** `Damage` INC with `ConsumedCorpseRecently`, from its config checks.
- **Projectile speed:** `ProjectileSpeed` INC.

**The engine:**
- `HIT_ROWS`: Combustion's more fire damage.
- `poe1::gems`:
  - `SupportCombustion`'s support table has its fire resistance as a `BuffStat::EnemyCheck` on `conditionEnemyIgnited` (the enemy's fire resistance, BASE).
  - Combustion is a debuff support (scale 1), named Combustion.
  - `supports_global` skips it on a skill that cannot ignite (`cannot_ignite`, `CANNOT_IGNITE_STATS`).
  - Every Combustion is -10, so the merged value is PoB's first.
  - Another source of `CannotIgnite` is an unmapped stat, so a reason.
- `config::DEPENDS_ON` no longer lists `ignites_apply_fire_resistance_+`.
- `conditions`: `ENEMY_BLINDED` from `conditionEnemyBlinded`.
- `STATE_ROWS`: crit chance against a blinded enemy, damage after consuming a corpse.
- `ProjectileSpeed` is a hit quantity, so a gem's projectile speed reaches the hit's bow damage flag (P6-11hb) and no longer leaves a reason.
- **Data-parity (`gem_effects`):** the new rule `gem-effects-combustion-calc-perform` (`representation`, 2 differences). PoB's stat map has the skill's mod, and the engine's table has the debuff `CalcPerform` makes of it. `uncategorized` is 0.

**Evidence** (`b-ct`, against P6-11hq's run):
- **Six new mechanic fixtures**, each matching PoB on every computed field:
  - `ice-nova-combustion-ignited-witch.xml`: Ice Nova supported by Combustion, with added fire damage, against an ignited enemy.
  - `ice-nova-combustion-other-group-witch.xml`: Arc supported by Combustion in another group lowers the fire resistance for Ice Nova (PoB's fire hit 90 against 75 without).
  - `ice-nova-combustion-never-ignite-witch.xml`: Flame Surge's Combustion applies nothing, because Flame Surge never ignites.
  - `ice-nova-combustion-elemental-focus-witch.xml`: Elemental Focus stops Combustion's resistance.
  - `ice-nova-corpse-blinded-lines-witch.xml` and `ice-nova-corpse-blinded-lines-off-witch.xml`: the two lines with their checks on and off.
- **Public:** 0 unexpected, and 4 values move toward PoB (holy-flame-totem-hierophant.xml's hit).
- **Corpus:** 0 unexpected, and 12 values move toward PoB; one build's crit chance now equals PoB's.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hs: the totem supports

**Found** (the unmapped stats left in single builds): the stats of Ballista Totem, Multiple Totems, and Focused Ballista were unmapped in two builds (kinetic-fusillade-ballista-hierophant.xml and one corpus build). Spell Totem's are the same kind, with no build using it yet.

**PoB** (`Data/Skills/sup_str.lua` 3883 to 3906, 4513 to 4560, 5034 to 5066; `sup_dex.lua` 1918 to 1950; `SkillStatMap.lua` 188, 1883, 2368):
- **Damage:** `support_totem_damage_+%_final` (Spell Totem, Ballista Totem), `support_additional_totem_damage_+%_final` (Multiple Totems), and `support_focused_ballista_totem_damage_+%_final` are `Damage` MORE.
- **Speed:** `support_attack_totem_attack_speed_+%_final` (Ballista Totem) and `support_focused_ballista_totem_attack_speed_+%_final` are `Speed` MORE with `ModFlag.Attack`; `support_spell_totem_cast_speed_+%_final` is `Speed` MORE with `ModFlag.Cast`.
- **Totem limit:** Multiple Totems' constant `base_number_of_totems_allowed` (2) is `ActiveTotemLimit` BASE on the supported skill.
- **Not read:** `is_totem` is display only; `base_skill_is_totemified`, `is_ranged_attack_totem`, `summon_2_totems`, and `ranged_attack_totem_only_attacks_when_owner_attacks` have no stat map entry. `totem_support_gem_level` is `totemLevel`, which only the totem's life reads.

**The engine:**
- `HIT_ROWS`: the six MORE rows.
- `poe1::stacks::MODIFIERS`: `base_number_of_totems_allowed` is the totems' maximum, so the main skill's limit (P6-11ep) counts Multiple Totems' two.
- `NO_EFFECT`: the four markers (`skill type marker`) and the other two (`no-slice`). The gems test that guards level stats lists `totem_support_gem_level` as checked.

**Evidence** (`b-ct`, against P6-11hr's run):
- **Four new mechanic fixtures**, each matching PoB on every computed field:
  - `ice-nova-spell-totem-witch.xml`: Ice Nova supported by Spell Totem, with "3% more Damage per Summoned Totem" at one totem.
  - `ice-nova-spell-totem-multiple-totems-witch.xml`: the same with Multiple Totems, three totems.
  - `burning-arrow-ballista-templar.xml` and `burning-arrow-ballista-focused-templar.xml`: Burning Arrow with a Short Bow, supported by Ballista Totem, then with Focused Ballista too.
- **Public:** 0 unexpected. 8 values move: the attack rate of kinetic-fusillade-ballista-hierophant.xml toward PoB, and its damage away, because that build was already far below PoB for its own unmapped Kinetic Fusillade stats; the less damage is PoB's too.
- **Corpus:** 0 unexpected; one build's full DPS moves away, the same Ballista build kind with other reasons left.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11ht: melee proximity damage and accuracy per level

**Found** (the unmapped stats left in single builds): Shield Crush's more damage from distance (three corpus builds), "+# to Accuracy Rating per Level" (two builds), and Impact's area per enemy killed recently. Impact's "Deal up to #% more Melee Damage to Enemies, based on proximity" and Close Combat's were listed in `config::DEPENDS_ON` (P3-12b) until the engine read the distance.

**PoB** (`Data/Skills/act_str.lua` 9874, 9889; `sup_dex.lua` 1175; `ModParser.lua` 2995; `Classes/ModStore.lua` 575 to 587; `CalcActiveSkill.lua` 446 to 464):
- **Melee proximity:** `shield_crush_damage_+%_final_from_distance` is `Damage` MORE with `ModFlag.Hit`; Impact's `slayer_damage_+%_final_from_distance` and Close Combat's `support_slashing_damage_+%_final_from_distance` are `Damage` MORE with `ModFlag.Attack` and `ModFlag.Melee`. Each carries the `MeleeProximity` tag, `ramp = {1, 0}`: the whole value up to a distance of 15, falling by a 25th per unit from 16 to 39, 0 from 40 (between 15 and 16 and between 39 and 40 no branch applies, so the value stays whole).
- **The distance** is the skill's `skillDist`: `meleeDistance` for a melee skill, else `projectileDistance`, the input or its placeholder (15 and 40).
- **Shield Crush's stat** comes only from its quality (0.5 per 1%), so a gem at quality 0 has none.
- **Accuracy per level:** `Accuracy` BASE per `Multiplier:Level`, the character's level.
- **Impact's area** per enemy killed recently reaches no slice field.

**The engine:**
- `Poe1Quantity::MeleeProximityDamage` (`HIT_ROWS`, a hit quantity for the gems' stats) and `Poe1Quantity::AccuracyPerLevel` (`ROWS`).
- `collect`: `melee_proximity`, after the gems (whose stats carry Shield Crush's and Close Combat's), makes each a `Damage` MORE at PoB's ramp of the main skill's distance with its row's flags; `accuracy_per_level` makes `Accuracy` BASE times the level. The distance read is shared with P6-11ha's ramps (`skill_distance`).
- `config::DEPENDS_ON` no longer lists the three proximity stats.
- `NO_EFFECT`: `slayer_area_of_effect_+%_per_enemy_killed_recently_up_to_50%` (`area`).
- A custom line "Deal up to #% more Melee Damage to Enemies, based on proximity" stays ambiguous (Impact's and Close Combat's stats share the text, and no item mod carries either), so the fixtures allocate Impact on a Slayer's tree. Close Combat's other stats (Combat Rush, damage at close range) are still unmapped, so it has no fixture of its own.

**Evidence** (`b-ct`, against P6-11hs's run):
- **Six new mechanic fixtures**, each matching PoB on every computed field:
  - `shield-crush-proximity-templar.xml`, `-distance-` (25), `-far-` (45): Shield Crush at quality 20 with a Driftwood Club and a Splintered Tower Shield; its 10% more damage whole, at 60%, and none.
  - `smite-impact-slayer.xml` (distance 30, 40% of Impact's 15%) and `smite-impact-close-slayer.xml` (default distance, whole).
  - `smite-accuracy-per-level-templar.xml`: "+3 to Accuracy Rating per Level" at level 90, the hit chance.
- **Public:** 0 unexpected; 5 values of cyclone-shockwave-slayer.xml (Impact) move toward PoB.
- **Corpus:** 0 unexpected; 16 values move toward PoB, 4 of them now equal.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hu: Onslaught after summoning a totem; maim, intimidate, and crush chances

**Found** (the unmapped stats left in single builds): "You have Onslaught if you've Summoned a Totem Recently" and "Skills that Summon a Totem have #% chance to Summon two Totems instead of one" (two Hierophant totem builds each), the chances to maim on hit and on crit with attacks, to intimidate on hit, and "Crush Enemies for # seconds when you Hit them while they are on Full Life" (two builds each).

**PoB** (`ModParser.lua` 1872; `CalcPerform.lua` 265 to 267; `ConfigOptions.lua` 1109):
- **Onslaught:** the line is `Condition:Onslaught` FLAG with the `SummonedTotemRecently` condition. That condition is set by its config check, and by `CalcPerform` whenever the main skill is a totem (`skillFlags.totem`).
- **Not read:** no stat map or parse reads the maim, intimidate, or crush chances (PoB's enemy maim, intimidation, and crush are config checks), and PoB leaves the two-totems chance unparsed.

**The engine:**
- `conditions::SUMMONED_TOTEM_RECENTLY` from `conditionSummonedTotemRecently` (moved from `config::NAMED_ONLY` to `CHECKS`), else `conditions::summoned_totem`: the main skill's types with its supports' added types hold `SummonsTotem`; a main skill that is not certain leaves it unsettled.
- `STATE_ROWS`: the Onslaught line on that condition (`poe1::buffs`).
- `NO_EFFECT`: `maim_on_hit_%`, `maim_on_crit_%_with_attacks`, `chance_to_intimidate_on_hit_%`, `crush_on_hit_ms_vs_full_life_enemies` (`on-hit-debuff`); `chance_to_summon_two_totems_%` (`no-slice`). The damage-taken guard test lists the intimidate chance as checked.
- **Left:** Ancestral Bond and Runebinder (each one keystone stat that is two effects, among them the totem limit), for a later step.

**Evidence** (`b-ct`, against P6-11ht's run):
- **Four new mechanic fixtures**, each matching PoB on every computed field:
  - `ice-nova-totem-onslaught-witch.xml`: Ice Nova supported by Spell Totem with the line: Onslaught from the totem main skill alone.
  - `ice-nova-totem-onslaught-check-witch.xml` and `-off-`: Ice Nova (not a totem) with the line, with and without the config check.
  - `smite-maim-crush-lines-templar.xml`: the maim-on-crit and crush lines change nothing.
- **Public:** 0 unexpected; 15 values move toward PoB, 4 of them now equal (holy-flame-totem-hierophant.xml's and kinetic-fusillade-ballista-hierophant.xml's movement speed; their speed and DPS move toward).
- **Corpus:** 0 unexpected, no value moved.
- **Tests:** the workspace tests, loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

## P6-11hv: Ancestral Bond and Runebinder

**Found** (the unmapped stats left in single builds): the keystones Ancestral Bond (two Hierophant totem builds) and Runebinder (two builds). Each is one stat, `keystone_ancestral_bond` and `keystone_runebinder`, whose description is two lines.

**PoB** (`ModParser.lua` 2334, 5252; `CalcPerform.lua` 1320 to 1338; `CalcOffence.lua` 1874):
- **Ancestral Bond:** "You can't deal Damage with Skills yourself" is `DealNoDamage` FLAG with a negated `SkillType` list (`SummonsTotem`, `RemoteMined`, `Trapped`) and a negated `usedByMirage` condition; "+1 to maximum number of Summoned Totems" is `ActiveTotemLimit` BASE 1.
- **Runebinder:** "-1 to maximum number of Summoned Totems" is `ActiveTotemLimit` BASE -1; "You can have an additional Brand Attached to an Enemy" is `BrandsAttachedLimit` BASE 1, which each brand skill sums with its own gems' limit (`CalcPerform`'s attach limit, at most the active brand limit).
- A keystone node allocated without a path to the class start is not counted (PoB's tree, and the engine's `unconnected-node` reason), so the fixtures name each keystone in a custom line, which PoB reads as the keystone (`Keystone` LIST).

**The engine:**
- `poe1::keystones::KEYSTONES`: Ancestral Bond's `DealNo` of every damage type under the new state `conditions::SKILL_USED_BY_YOU` and the totems' maximum +1; Runebinder's totems' maximum -1 and `StackValue::AttachMaximum` +1 for brands attached.
- `conditions::skill_used_by_you`: false for the mirage's calculation and a main skill of the totem, mine, or trap type (with its supports' added types); unsettled when the main skill is not certain.
- `stacks::brands` adds the character's `AttachMaximum` to each brand skill's limit.
- **Left:** Call to Arms (its "Your Warcries do not grant Buffs or Charges to You" needs the warcry buffs to read it) and Supreme Prodigy (no inherent bonuses from attributes) stay reasons.

**Evidence** (`b-ct`, against P6-11hu's run):
- **Five new mechanic fixtures**, each matching PoB on every computed field:
  - `ice-nova-totem-ancestral-bond-witch.xml`: Ice Nova with Spell Totem: the totem deals its damage, two totems (3% more damage per totem).
  - `ice-nova-ancestral-bond-witch.xml`: self-cast Ice Nova deals no damage.
  - `ice-nova-totem-runebinder-witch.xml`: Spell Totem with Runebinder: no totem.
  - `ice-nova-armageddon-brand-runebinder-witch.xml` and `ice-nova-armageddon-brand-witch.xml`: damage taken per brand attached, two brands with Runebinder, one without.
- **Public:** 0 unexpected; 9 values of kinetic-fusillade-ballista-hierophant.xml move toward PoB, its mana regeneration now equal.
- **Corpus:** 0 unexpected, no value moved.
- **Tests:** the workspace tests (the keystone text test lists both), loom-calc's real-bundle tests (`b-ct`), clippy, and rustfmt pass.

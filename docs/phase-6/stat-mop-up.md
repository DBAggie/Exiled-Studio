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

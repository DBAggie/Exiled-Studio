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
- **Full life** (`FullLife`): PoB also derives it from the life left unreserved (`CalcPerform.lua` 76 to 83).
- **Life flask use** (`UsingLifeFlask`): PoB derives it from active flasks.
- **Minion skill used recently** (`UsedMinionSkillRecently`): PoB also derives it from the main skill.
- **Exposure inflicted recently** (`AppliedExposureRecently`): PoB derives it from exposure applied.

**Evidence** (`local/p6/p6-11w/`):
- **Public `calc-parity`:** 212 builds, 280 expected, 0 unexpected. Match went from 13,533 to 13,535: 2 fields went from partial to complete, and no value moved.
- **Corpus:** unchanged; 0 unexpected.
- **Tests:** the workspace tests, the maintainer-run tests on the evidence bundle, and clippy pass.

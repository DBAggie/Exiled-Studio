# P6-10b: recovery and defence fields

P6-10b adds the character's recovery and defence outputs PoB computes in `CalcDefence.lua` and `CalcOffence.lua` (plan, "P6-10b"). It lands in parts, each a commit on PR #167's branch with its evidence.

## Part 1: life and mana regeneration

Two new fields: `life-regen` and `mana-regen` (PoB's `LifeRegen` and `ManaRegen`, per second). The slice is now at revision 6: group `recovery`, `PRESENT_NAMES` with both.

**The calculation** (`loom_calc::poe1::regen`, phase `regeneration` after `unreserved`; `CalcDefence.lua` 1237 to 1285), for each pool:
- **Base:** flat regeneration, plus the pool's maximum times its percent regeneration over 100.
  - Mana's base is the character constant `mana_regeneration_rate_per_minute_%` over 60, as PoB's `ManaRegenBase` (`CalcSetup.lua` 497, `Data.lua` 247).
  - The game's per-minute stats are rows scaled by 1/60. On a state row, which has no scale, they use `RegenPercentPerMinute`, read divided by 60.
- **Scaling:** the base times `(1 + increased / 100)` times the more multipliers of `<Pool>Regen`.
- **Recovery rate:** the result times the pool's recovery rate modifier (`<Pool>RecoveryRate`, 1194 to 1198), rounded to one decimal.

**Sources now read:**
- **Rows:** about 30 regeneration and recovery rate stats from items, the tree, and gems. These include Clarity's and Vitality's regeneration and the golems' buffs (gem table rows, divisor 60).
  - The P6-11g `NoEffect` rows for regeneration are gone: PoB reads them into these fields.
- **Arcane Surge:** its increased mana regeneration, the gem's value or PoB's default 30 (`buffs::arcane_surge`, `ArcaneSurgeManaRegen` from "Arcane Surge grants more mana regeneration" sources).
- **Consecrated ground under you** (`buffs::consecrated_ground`, `CalcPerform.lua` 770 to 775): 5% of life per second, times its effect. It is on in either case:
  - with the config's `conditionOnConsecratedGround`;
  - with "You have Consecrated Ground around you while stationary" (`ConsecratedWhileStationary`, a flag row) and a `conditionStationary` count above 0.
  - Both inputs left `config::NAMED_ONLY`.
- **State rows:**
  - the Soul of Shakari's "Regenerate #% of Life per second while stationary" (`conditions::STATIONARY`, settled from the stationary count);
  - the Soul of Arakaali's recovery rate "if you've stopped taking Damage Over Time Recently" (`conditions::STOPPED_TAKING_DOT_RECENTLY`, a config check; it left `NAMED_ONLY`).
- **Enduring Cry:** its regeneration depends on warcry power, which the engine does not model yet. It stays a reason (`warcry-power`) on the life field alone.

**Reasons stay on the regeneration fields.** A regeneration stat the engine does not model is `mapping::REGEN_REASONS`, a marker only `regen` reads. It is an `unmapped-stat` reason on that pool's field, so the character's other fields keep their state. The stats are:
- per fortification stack (fortification is named only);
- per uncapped fire resistance, per endurance charge, per level, per power charge, per spell block chance;
- mana with exposure inflicted recently;
- Zealot's Oath.

A regeneration value the engine cannot settle (an unsettled condition) goes through `StatMapping::elsewhere`, for item, weapon, off-hand, and flask lines too. It makes only the regeneration fields indeterminate. The config test that forbids mapped rows naming fortification is unchanged: `REGEN_REASONS` is not a row table.

**Not modelled:** no regeneration and cannot gain (`No<Pool>Regen`, `CannotGain<Pool>`), and life regeneration moved to another pool; degeneration (Righteous Fire's own degeneration stays as decided in D7 until a later part).

**Evidence** (`local/p6/p6-10b1/`):
- **Fixtures:**
  - `ice-nova-regeneration-witch.xml`: flat and percent life regeneration, increased mana regeneration, Clarity, Vitality, and the recovery rate. LifeRegen 258.6, ManaRegen 52, both equal to PoB.
  - `ice-nova-arcane-surge-regeneration-witch.xml`: Arcane Surge's mana regeneration. ManaRegen 13.4, equal to PoB.
  - `smite-blocked-recently-no-effect-templar.xml` changed: its life regeneration line, a `NoEffect` row in P6-11g, now reaches LifeRegen (26, equal to PoB). It is re-pinned.
- **Public `calc-parity`:** 190 builds, 131 expected differences, 0 unexpected; `Complete` fields 3,945 (3,581 after P6-11j).
  - Every computed regeneration value equals PoB's: life and mana in 159 builds each, including the Inquisitor's acts and the stationary golems.
  - No other value moved, and no field changed state.
- **Corpus:** 0 unexpected, no value moved, no state changed.
- **data-parity:** `gem_effects` 1,475 values, 0 differences (Enduring Cry's regeneration row is a reason now); uncategorized 353, unchanged.
- **Real-bundle tests** pass. The workspace tests and clippy pass.

## Part 2: energy shield regeneration and recharge

Three new fields (slice revision 7, applies present):
- `energy-shield-regen`: PoB's `EnergyShieldRegen`, per second.
- `energy-shield-recharge`: PoB's `EnergyShieldRecharge`, per second.
- `energy-shield-recharge-delay`: PoB's `EnergyShieldRechargeDelay`, in seconds.

**The calculation** (`loom_calc::poe1::regen`; `CalcDefence.lua` 1237 to 1383):
- **Energy shield regeneration** is part 1's calculation for a third pool, `Pool::EnergyShield`, after mana and life.
- **Zealot's Oath** (`ZealotsOath`, from `regenerate_energy_shield_instead_of_life`; 1250 to 1259): life regenerates nothing. Life's flat sum and percent sum, each when above 0, are added to energy shield's.
- **Recharge** is energy shield times the base, times `(1 + increased / 100)` and the more multipliers of `EnergyShieldRecharge`, times energy shield's recovery rate modifier, rounded to a whole number.
  - The base is the character constant `energy_shield_recharge_rate_per_minute_%` (2000) over 60 and 100, so 33.3% per second.
  - PoB's `Data.lua` has two lines for the base: 248 derives it from that constant, 249 sets 0.33. PoB's outputs are the constant's (980 energy shield recharges 327, not 323), so there is no divergence.
- **Delay** is 2 seconds (`data.misc.EnergyShieldRechargeDelay`) over `(1 + increased / 100)` of `EnergyShieldRechargeFaster`.

**Sources now read:**
- **Rows:**
  - flat and percent energy shield regeneration (per minute, scaled 1/60), increased regeneration, recovery rate, recharge rate, and faster start of recharge;
  - P6-11g's two recharge `NoEffect` rows are gone;
  - Discipline's recharge rate is a gem row now, not `NoSlice`.
- **State rows:**
  - the Soul of Arakaali's recovery rate now raises energy shield's too;
  - "Regenerate # Energy Shield per second while a Rare or Unique Enemy is Nearby" is a state row under `conditions::ENEMY_RARE_OR_UNIQUE_NEARBY`. That condition is PoB's `varList` (`ModParser.lua` 1776): the enemy is rare or unique (a boss kind or the config's check), or the config's count of nearby rare or unique enemies is at least 1. Its value is per minute, so it adds the new `RegenPerMinute` quantity, read divided by 60.
- **`REGEN_REASONS`:** gains energy shield regeneration per nearby corpse (PoB's `ExtraAura`) and loses Zealot's Oath. A test now checks the table is sorted, since a binary search reads it.

**A display limit:** the game prints `per_minute_to_per_second` stats to one decimal, so their printed text covers several stat values. Two cases:
- A custom line such as "Regenerate 2% of Energy Shield per second" leaves the value unsettled.
- A unique's "Regenerate # Energy Shield per second while a Rare or Unique Enemy is Nearby" (Elementalist, mapping) leaves its field indeterminate.

PoB takes the printed number. Life's percent stat uses the `2dp_if_required` handler and is exact.

**Evidence** (`local/p6/p6-10b2/`):
- **Fixtures:**
  - `ice-nova-energy-shield-recovery-witch.xml`: +300 energy shield, recovery rate, recharge rate, faster start. Recharge 147 and delay 1.6, both equal to PoB.
  - `ice-nova-zealots-oath-witch.xml`: Zealot's Oath with Vitality and life regeneration lines. LifeRegen 0, EnergyShieldRegen 215.9, recharge 103, all equal to PoB.
- **Public `calc-parity`:** 192 builds, 131 expected differences, 0 unexpected; `Complete` fields 4,484 (3,945 after part 1).
  - Every computed energy shield value equals PoB's: regeneration in 163 builds, recharge and delay in 164 each.
  - Three values moved, all to PoB's: the Zealot's Oath builds' LifeRegen, now 0. No field changed state.
- **Corpus:** 0 unexpected. One value moved, to PoB's (a Zealot's Oath build's LifeRegen); no state changed.
- **data-parity:** `gem_effects` 1,475 values, 0 differences (Discipline's recharge compared as `EnergyShieldRecharge`); uncategorized 353, unchanged.
- **Real-bundle tests** pass. The workspace tests and clippy pass.

## Part 3: movement speed

Two new fields (slice revision 8, applies present, group `movement`). The engine always computes them:
- `movement-speed`: PoB's `MovementSpeedMod`.
- `effective-movement-speed`: PoB's `EffectiveMovementSpeedMod`.

**The calculation** (`loom_calc::poe1::movement`, phase `movement` after `regeneration`; `CalcDefence.lua` 1504 to 1508):
- **Movement speed modifier:** `(1 + increased / 100)` times the more multipliers of `MovementSpeed`.
- **Effective modifier:** the movement speed modifier times the action speed modifier, `max(0, 1 + ActionSpeed increased / 100)` (`calcs.actionSpeedMod`, as the hit's).

**Sources now read:**
- **Rows:**
  - "#% increased Movement Speed" (`base_movement_velocity_+%`, a `NoEffect` row before);
  - Haste's movement speed (a gem row, not `NoSlice`);
  - a flask's "#% increased Movement Speed during Effect", now an alias of the global stat (`flasks::ALIASES`), out of `NO_EFFECT_LOCAL`.
- **Per charge:** movement speed per frenzy charge (`charges::PerCharge::MovementSpeed`).
- **Item bases:** the armour base's movement penalty, the armour types table's `movement_speed` (`ArmourBase::movement_speed`; PoB's `MovementPenalty`, `Item.lua` 2361). It is -3 on shields and lighter body armours and -5 on the heavier ones.
- **Buffs:**
  - Onslaught's movement speed (`CalcPerform.lua` 684), with Silver Flask's extra;
  - Adrenaline's 25 (745).
- **`MOVEMENT_REASONS`:** stats that change movement speed in a way the engine does not model map to `Poe1Quantity::MovementReason`, a marker only `movement` reads, so they are these fields' `unmapped-stat` reason and no other field's. The table is sorted, and a test checks it. It covers:
  - on kill, during a flask's effect, on burning ground, on low life, while ignited, and with no gems in the boots: config conditions the engine does not settle;
  - per summoned phantasm;
  - per nearby enemy: Lunaris's soul, PoB's `Multiplier:NearbyEnemies` with a limit, a `NoEffect` row before;
  - the armour penalties ignored;
  - movement speed that cannot go below its base;
  - Momentum's Swiftness, per the config's `MomentumSwiftnessStacks`, also on another group's Momentum through `gems::SUPPORTS`.
- **Warcries:** Intimidating Cry's movement speed per warcry power uses the same marker. Enduring Cry's regeneration now uses `RegenReason` in place of `BuffStat::Reason`, which put its reason on every field. `SkillMapping` keeps both markers on the main skill's gems.

**Found while checking:** "Smite Inquisitor, act 1" was 1.45 against PoB's 1.5. PoB runs on two variants of the build found the cause:
- without the body armour, 1.55;
- without the Onslaught line, 1.3.

So the gap is Lunaris's soul with the build's 5 nearby enemies, a stat the table had as `NoEffect`. It is a movement reason now.

**Evidence** (`local/p6/p6-10b3/`):
- **Fixture:** `ice-nova-movement-speed-witch.xml`, with Haste, "25% increased Movement Speed", and Onslaught, Adrenaline, and Tailwind from the config. MovementSpeedMod 1.86 and EffectiveMovementSpeedMod 2.0088, both equal to PoB's.
- **Public `calc-parity`:** 193 builds, 131 expected differences, 0 unexpected; `Complete` fields 4,823 (4,484 after part 2).
  - Every computed movement value equals PoB's: 156 builds each.
  - No other value moved, and no field changed state.
- **Corpus:** 0 unexpected, no value moved, no state changed.
- **data-parity:** `gem_effects` 1,473 values, 0 differences (Haste's movement speed compared as `MovementSpeed`; the warcries' markers are not compared); uncategorized 353, unchanged.
- **Real-bundle tests** pass. The workspace tests and clippy pass.

## Part 4: the main skill's mana cost

One new field: `mana-cost`, PoB's `ManaCost` (slice revision 9, applies present, group `cost`).
- The engine always emits it.
- When the build has no main skill it is `not-computed`. The bare characters and the minimal builds count one more `not-computed` field (tests updated).

**The calculation** (`loom_calc::poe1::costs`, phase `costs` after `dot`; `CalcOffence.lua` 1677 to 1800). It follows PoB's order of floors and ceilings:
- The base is the skill level's `Mana` cost (the granted effect level's costs).
- Times the supports' cost multiplier `floor(More(SupportManaMultiplier), 4)` (`gems::support_multiplier`, as reservation's), floored, plus `ManaCostNoMult`.
- Times `(1 + inc / 100)` of `ManaCost` and `Cost` `INC`, rounded up when the sum is below 0 and down otherwise.
- The `ManaCost` more, then the `Cost` more, each rounded up below 1 and down otherwise.
- Divided by the cost efficiency, `1 + (ManaCostEfficiency + CostEfficiency) / 100`.
- Plus the total, the `BASE` sums of `ManaCost` and `Cost`; never below 0.
- The modifiers are queried in the skill's context: attack or spell, and its keyword flags.

**Sources now read:**
- **Rows:**
  - increased cost (`base_cost_+%`) and reduced mana cost (`base_mana_cost_-%`, negated);
  - "Skills Cost +# Mana" (`ManaCostNoMult`);
  - cost and mana cost efficiency;
  - Hierophant's more mana cost and Inspiration Support's (its `statMap`);
  - attacks' mana cost (a hit row on `Flags::ATTACK`).
- **Retired `NoEffect` rows:** five P3-8n and P6-11g rows for costs. Two `NO_EFFECT_DEPENDS_ON` guards went with them, since the stats they guarded are rows now. The stats that add damage from a cost stay unmapped.
- **`COST_REASONS`:** the cost field's markers (`Poe1Quantity::CostReason`), sorted and tested. Each is a reason on that field alone:
  - costs paid in life or energy shield instead;
  - socketed attacks' total cost and Manaweave's cost from unreserved mana;
  - the flask's and full energy shield's conditions;
  - channelling and non-channelling skills' totals;
  - Focused Channelling's more cost.
- **The skill's own data:** a cost in another resource and a skill whose reservation becomes its cost are the skill's reasons.

**Evidence** (`local/p6/p6-10b4/`):
- **Fixture:** `ice-nova-mana-cost-witch.xml`, with Controlled Destruction's multiplier, 12% reduced and 4% increased cost (a net reduction, rounded up), "Skills Cost +3 Mana", and 20% increased mana cost efficiency. ManaCost 27.5, equal to PoB's.
- **Public `calc-parity`:** 194 builds, 131 expected differences, 0 unexpected; `Complete` fields 4,990 (4,823 after part 3).
  - Every computed mana cost equals PoB's: 138 builds.
  - No other value moved, and no field changed state.
- **Corpus:** 0 unexpected, no value moved, no state changed.
- **data-parity:** `gem_effects` 1,473 values, 0 differences; uncategorized 353, unchanged.
- **Real-bundle tests** pass; the reservation stats test accepts a field's reason marker. The workspace tests and clippy pass.

## The review of parts 1 to 4

The code-architecture-reviewer read the four parts against PoB's Lua. It found nothing critical. Its findings and what changed:

- **Cost efficiency read its MORE multipliers.** PoB's `calcLib.mod` is `(1 + inc / 100) x more`; the cost read only the increase. It now multiplies both names' more.
- **The cost's clamp order.** PoB holds `cost / efficiency` at 0 or above before it adds the total; the engine clamped only at the end. It now clamps after the division, as PoB. An efficiency at or below 0 leaves the field unsupported (`cost-efficiency`), where PoB would divide by it.
- **Zealot's Oath kept life's reasons.** Life's flat and percent sums moved to energy shield without their reasons, so a life regeneration stat the engine does not model left energy shield's regeneration looking complete (ADR-015). They now travel together.
- **Cost inputs with no stat.** PoB's `<Resource>CostBase` (Archmage, Frostmage), `Base<Resource>CostOverride`, and `HasNoCost` have no mapped stat, so their stats stay `unmapped-stat` reasons on every field; the module docs say so. The movement speed override and the recharge override are named likewise.
- **Shared code:**
  - `phases::marker_reasons` reads every field's markers and unsettled values, used by `regen`, `movement`, and `costs`;
  - `mapping::field_marker` maps a stat to its marker, with a test that the three tables are disjoint;
  - `hit::action_speed_modifier` is the one action speed modifier, which the hit's rate and the effective movement speed share.
- **The recharge delay's reasons** are the faster start's alone, not energy shield's recovery rate.
- **`Data.lua`'s two recharge bases.** Lines 248 and 249 set one key. Under LuaJIT a table constructor's constant entries fill a template first and its computed ones are stored after, so line 248's constant is the value PoB uses. Its outputs show it: 980 energy shield recharges 327, where 0.33 would give 323. The docs now say this.
- **Tests:**
  - Zealot's Oath moving life's regeneration and its reasons;
  - a field marker being that field's reason alone;
  - movement speed with action speed and its floor at 0;
  - the cost's rounding direction;
  - which quantities each field reads.

**Evidence** (`local/p6/p6-10b-rv/`):
- Public and corpus `calc-parity` are unchanged: 194 builds, 0 unexpected, 4,990 `Complete`; no value moved, no state changed.
- The synthetic golden's graph nodes moved (the efficiency's more, the clamp); its values, states, and reasons are unchanged.
- Workspace tests, clippy, and the real-bundle tests pass.

## A disabled skill's cost; PoB's weapon types (P6-12 follow-up)

**Found by the minion sweep** (`local/p6/minion-fx`, 52 builds): `minion-holy-strike-witch.xml` gave a mana cost of 10 where PoB writes none, the sweep's one unexpected difference.

**Why PoB writes no cost:**
- PoB disables a skill the hands cannot use (`CalcActiveSkill.lua` 269 to 334):
  - it requires a shield and the off hand holds none;
  - the main hand's weapon type is not in the skill's `weaponTypes`, an empty hand being the class's unarmed data, type `None`;
  - the off hand holds a weapon it cannot use (an empty off hand is `{}` and never disables);
  - it is `DualWieldOnly` without two weapons, or `DualWieldRequiresDifferentTypes` with one type twice.
- Its offence then returns at once (`CalcOffence.lua` 339), so it has no cost and no other offence output.
- Holy Strike takes maces, staves, and sceptres, and the witch is unarmed. Blink Arrow and Mirror Arrow need a bow.

**The engine:**
- `MainSkill::disabled_by_hands` follows those checks. The cost phase leaves `mana-cost` not computed for a disabled skill. A hand whose base is not known leaves it unsupported with a reason.
- The hit already returned nothing for a skill its main hand cannot use.

**Found on the way: PoB's weapon types.** PoB's exporter keeps only the item classes its `weaponClassMap` names (`Export/Scripts/skills.lua` 15 to 31, 234 to 246).
- Shield Crush lists only `Shield` in the game, so PoB gives it no `weaponTypes`: it takes any weapon.
- A held Warstaff is PoB's `Staff`, and a Rune Dagger its `Dagger`.
- `can_use` now compares by PoB's type (`skill::pob_weapon_type`). Before this, Shield Crush with a sword was never computed.

**Evidence** (`local/p6/p6-12c/`):
- **Minion sweep:** 0 unexpected (1 before). Eight mana costs became not computed, each where PoB writes none (Holy Strike, one Animate Weapon variant, Blink Arrow and Mirror Arrow and their variants).
- **Public `calc-parity`:** documents byte-identical to before. 195 builds, 172 expected, 0 unexpected, 5,158 `Complete`.
- **Corpus:** four Shield Crush builds' hits are now computed. Every new value is `partial` or `indeterminate`, with Shield Crush's added damage per 15 shield armour and evasion among the reasons. No mana cost moved; 0 unexpected.
- **Tests:** `disabled_by_hands_should_follow_pobs_weapon_checks` and the extended `can_use` test; 1,756 workspace tests and clippy pass.


## Part 5: base damage reduction

**What PoB does** (`CalcDefence.lua` 1496 to 1501): `Base<Type>DamageReduction = min(max(0, <Type>DamageReduction BASE (+ ElementalDamageReduction BASE for fire, cold, and lightning)), DamageReductionMax)`.
- `DamageReductionMax` is the character constant `maximum_physical_damage_reduction_%` (90), unless a modifier overrides it.
- Endurance charges give `PhysicalDamageReduction` and `ElementalDamageReduction` BASE each (`CalcSetup.lua` 515 and 516).
- Adrenaline gives `PhysicalDamageReduction` BASE `floor(10 x effect)` (`CalcPerform.lua` 746).

**The engine** (`poe1::reduction`, phase `damage-reduction`; slice revision 12):
- **Fields:** `base-<type>-damage-reduction` for the five types (`Poe1Field::BaseDamageReduction`), always present.
- **Sources:**
  - `base_additional_physical_damage_reduction_%`, a row (it was `NoEffect`, reaching no slice field);
  - the stationary physical reduction, a state row on `Stationary`;
  - the endurance charges' per-charge effects (`charges::CHARGES`);
  - Adrenaline's 10 (`buffs`).
- **Reasons on these fields only** (`mapping::REDUCTION_REASONS`, field markers): physical damage reduction per nearby enemy, per hit taken recently, and with one enemy nearby. These were `NoEffect` rows. Under plan decision 9 they can no longer be, now that a slice field reads them.
- **Not computed:** the reduction against a hit (PoB's `<Type>DamageReduction` with armour, the enemy's hit, and overwhelm; the EHP part), and the `WhenHit` variants.

### Evidence

- **Public `calc-parity` on `b-ot`:** 205 builds, 187 expected, 0 unexpected; `Complete` fields 6,356 (5,506 before; the 850 new reduction fields that are `Complete` equal PoB's).
  - Of the reduction fields, 24 `partial` ones differ from PoB, each with its reasons.
  - No other value or state moved.
- **Corpus:** 0 unexpected; no other value moved.
- **Minion sweep:** 0 unexpected. **Skill sweep:** the same 32 unexpected as before.
- **Goldens:** `synthetic-poe1.calc.json` and `poe1-empty.json` regenerated. Only the five new fields were added (`indeterminate` in the synthetic bundle, which has no Scion).
- **Tests:** the field counts (`engine`, `calc_run`, the DTO) and the test data's charge and maximum constants; the workspace's 1,763 tests and clippy pass.

## Part 6: the mana cost per second

**What PoB does** (`CalcOffence.lua` 5794 to 5825): for an upfront cost above 0, `<Resource>PerSecondCost = <Resource>Cost x useSpeed`.
- `useSpeed` is the skill's `Speed` (`1 / Cooldown` when the speed is 0) divided by its repeats.
- A trap's, a mine's, or a totem's is its throwing, laying, or placement speed.
- Eldritch Battery moves the mana cost to energy shield.

**The engine** (`costs::per_second`; slice revision 13):
- **The field:** `mana-per-second-cost` is the mana cost times `Speed`, with both as its dependencies. It exists only when there is a cost and a computed speed.
- **Repeats** are 1 (the hit's module docs).
- **Traps, mines, and totems:** the field is unsupported (`cost-per-second`).
- **Eldritch Battery** is an unmapped stat, so a reason.
- **Not yet ported:** the life and energy shield costs, and the costs per second of channelled skills, which PoB writes for a few builds (Boneshatter's, Cyclone's, Lacerate's life costs).

### Evidence

- **Public `calc-parity` on `b-ot`:** 205 builds, 231 expected, 0 unexpected; `Complete` fields 6,485.
  - The field equals PoB's in 85 builds.
  - In 44 builds it carries the registered Q11 attack rate, as the cost equals PoB's and `Speed` is registered: DIV-0189 to DIV-0232, `game-truth`. The register tool adds an entry only where the build's `Speed` entry exists.
  - 11 are `partial` or `indeterminate` with their reasons, 22 not computed.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,763 tests and clippy pass.

## Part 7: the leech caps

**What PoB does** (`CalcDefence.lua` 1201 to 1233): for life, energy shield, and mana:
- **The rate cap's percent** is `calcLib.val(Max<Pool>LeechRate)`: the `Base` sum times `(1 + inc / 100) x more`.
  - The base is the character's (`CalcSetup.lua` 46, 47, 51): life's and mana's constants `maximum_<pool>_leech_rate_%_per_minute` (1,200) / 60, so 20%; energy shield's 10, which equals its constant (600) / 60.
- **The caps:** `Max<Pool>LeechRate = pool x percent / 100`; life's percent is its own output, `MaxLifeLeechRatePercent`.
- **The instance cap** works the same from `Max<Pool>LeechInstance`, whose base is the constant `maximum_<pool>_leech_amount_per_leech_%_max_<pool>` (10).

**The engine** (`poe1::leech`, phase `leech` after regeneration; slice revision 15):
- **The fields:** `max-life-leech-rate`, `max-life-leech-instance`, and the same for mana and energy shield. Each is present with its pool and depends on it. `max-life-leech-rate-percent` is always present.
- **Arithmetic:** each is computed in PoB's order, `pool x percent / 100`, so its floating point matches PoB's.
- **The stats:** seven rows (`LeechRateMax`, `LeechInstanceMax`, `Increased`):
  - the three pools' "Maximum total Recovery per second from Leech";
  - the three pools' "Maximum Recovery per Leech";
  - the line for all three rates.
- **Skill stat map:** PoB's skill stat map reads the life and energy shield instance stats as the rate (`SkillStatMap.lua` 482 to 487), but its item lines read them as the instance (`ModParser.lua` 563 to 565). No public build has the gem form.
- **Not modelled:** the conditioned caps (a crit or savage hit recently, Defiance, Zealotry, maximum rage), the doubled caps, and a parent's or party member's percent. Each is an unmapped stat, so a reason.
- **The leech itself** is part 9.

### Evidence

- **Public `calc-parity` on `b-ot`:** 205 builds, 271 expected, 0 unexpected; `Complete` fields 7,861.
  - Each of the seven fields equals PoB's in 177 builds.
  - 21 are `partial` and 6 `indeterminate`. In every partial build that PoB also computes, the cap divided by its pool equals PoB's: the differences are the pools' own.
  - No other field's value or state moved.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,763 tests and clippy pass. The bare Witch's test checks all seven values.

## Part 8: the life and energy shield costs

**What PoB does** (`CalcOffence.lua` 1677 to 1800): the upfront costs run in the order mana, life, energy shield, each as the mana cost did in part 4.
- **The base:** each resource's base is the skill level's cost of that resource (`Life`, `ES`). Its final base is `floor(base x multiplier)`, plus mana's `ManaCostNoMult` for mana.
- **The conversions:** in the first pass, before any modifier:
  - With `CostLifeInsteadOfMana` (Lifetap, "Skills Cost Life instead of Mana"), life's final base gains mana's, and mana's becomes 0.
  - Otherwise, with `HybridManaAndLifeCost_Life` ("Skills Cost Life instead of #% of Mana Cost", the attack form too) or `ManaCostAsLifeCost` above 0, life's gains `round(mana's x share / 100)`.
  - Energy shield's works the same, with `CostESInsteadOfManaOrLife` or `ManaCostAsEnergyShieldCost`.
- **The modifiers:** each resource's increased (its own and `Cost`), more, efficiency, and total, rounded as mana's.
- **The hybrid's last step:** mana's cost then becomes `floor((1 - share / 100) x cost)`.
- **Per second:** `<Resource>PerSecondCost` is the cost times the skill's speed.

**The engine** (`costs`; slice revision 16):
- **The fields:** `life-cost` and `energy-shield-cost` are present with a main skill, as `mana-cost` is. `life-per-second-cost` and `energy-shield-per-second-cost` are present with a cost and a computed speed.
- **The conversion quantities:** `CostLifeInsteadOfMana`, `CostEnergyShieldInsteadOfManaOrLife`, `HybridLifeCost`, `ManaCostAsLifeCost`, and `ManaCostAsEnergyShieldCost`. With the life and energy shield `Cost` and `CostEfficiency`, they are skill quantities too, so Lifetap's stat acts on the skill it supports.
- **The rows:**
  - Lifetap's `base_skill_cost_life_instead_of_mana` and the mastery's `base_skill_cost_life_instead_of_mana_%` (both were `COST_REASONS`).
  - The attack form `base_attack_skill_cost_life_instead_of_mana_%` (a hit row with attacks).
  - `base_life_cost_+%`.
  - Lifetap's "more Damage while you have the Lifetap buff", under the config's `buffLifetap` (`conditions::LIFETAP`; it left `NAMED_ONLY`, whose other name, the flask life recovery, reaches no slice field).
- **Not modelled:**
  - **Data the bundle lacks:** soul costs (PoB's `VaalSouls` column) wait for a data run.
  - **Reasons:** rage and percent costs, and the Blood Magic keystone (which also removes mana and reserves life).
  - **Partial in a community build:** Petrified Blood's life cost is an aura gem's stat.

### Evidence

- **Fixtures:** three new mechanic fixtures, each cost equal to PoB's:
  - `cost-lifetap-ice-nova-witch.xml`: 69 life, 0 mana.
  - `cost-life-share-ice-nova-witch.xml`: 6 life, 16 mana.
  - `cost-attack-life-share-smite-templar.xml`: 5 life, 7 mana. Its Speed, DPS, and per-second costs carry Smite's Q11 rate: DIV-0273 to DIV-0278, `game-truth`.
- **Public `calc-parity` on `b-ot`:** 208 builds, 277 expected, 0 unexpected; `Complete` fields 8,303.
  - The life and energy shield costs equal PoB's in 152 builds.
  - Boneshatter's, Cyclone's, and Flicker Strike's life costs (Lifetap or the mastery) equal PoB's in their partial builds.
  - Holy Flame Totem's mana cost already differed before this part, from its unmodelled totem stats.
- **Moved in partial builds:**
  - RF Chieftain's damage over time and Elemental Hit's full DPS gain Lifetap's 20% more: both builds configure the buff.
  - Four partial builds' mana costs moved toward PoB's.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,764 tests and clippy pass.

## Part 9: the leech rates

**What PoB does** (`CalcOffence.lua` 3296 to 3667, 3981 to 4005), for life, energy shield, and mana:
- **Per pass:** in each hit pass (crits and non-crits), each damage type's average against the enemy (after `effMult`) times its leech percent / 100.
  - The percent is `Sum(BASE, "DamageLeech", "Damage<Pool>Leech", "<Type>Damage<Pool>Leech", "ElementalDamage<Pool>Leech")`; `DamageLeech` is life's and mana's alone.
  - Nothing is leeched with `CannotLeech<Pool>`.
  - Each pass's total is held to the pool's instance cap, then weighted by the crit chance.
- **The instant share:** `Instant<Pool>Leech`, held between 0 and 100, is taken out of the leech per hit.
- **Instances:** `duration = leech / pool / LeechRateBase` (0.02), and `instances = duration x hitRate`, where `hitRate = HitChance / 100 x Speed`.
- **The rate:** `instanceRate = pool x 0.02 x (1 + inc / 100) x more` of `<Pool>LeechRate`, and `rate = instances x instanceRate`.
- **The result:** `<Pool>LeechRate = instant x hitRate + min(rate, Max<Pool>LeechRate) x <Pool>RecoveryRateMod`.

**The engine** (`leech::rates`, from the hit phase; slice revision 17):
- **The per-type values:** the hit's passes keep each type's value against the enemy (`HitSources::against`).
- **Which hits:** a spell's or a one-handed attack's hit gets a rate for each pool it leeches (`life-leech-rate`, `energy-shield-leech-rate`, `mana-leech-rate`).
  - A dual wielder's leech is `NotComputed`: PoB combines the hands (`combineStat`). Computed since open items S7 (below, "Open items S7, part 2").
  - A minion's hit sets none.
- **The quantities:** `DamageLeech(pool, source)` (`LeechSource` is any damage, a type, or elemental), `LeechRate`, `InstantLeech`, and `CannotLeech`, all skill quantities.
- **The rows:** 31 rows generated from PoB's own parse of each leech stat's line (`ModParser`, with its flags: attack or spell hits are hit rows), plus "#% of Leech is Instant".
  - Three of them were `NoEffect` rows: the attack and spell damage leech lines and the instant line.
  - Blood Rage's leech is now its buff's row (`gems`, checked by `gem_effects`, whose names gained `Damage<Pool>Leech`).
- **Not modelled:** Ghost Reaver, Immortal Ambition, leech conditioned on a crit or on an enemy's state, the enemy's own leech lines, and totems', traps', and mines' leech to you. Each is an unmapped stat, so a reason.

### Evidence

- **Fixtures:** two new mechanic fixtures, each rate equal to PoB's:
  - `leech-ice-nova-witch.xml`: life 27.02, mana 10.39.
  - `leech-instant-cap-ice-nova-witch.xml`: life 123.25, the crit pass held to the instance cap.
- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected; `Complete` fields 8,405.
  - Ice Nova Elementalist's energy shield leech and Blood Rage's Smite leech equal PoB's.
  - Three Smite builds' life leech carries their registered Q11 attack rate: DIV-0279 to DIV-0281, `game-truth`.
  - Builds whose hit is not computed, or whose leech comes from an unmapped line, have no field (`not-computed` against PoB's value).
  - No other field's value or state moved.
- **Data parity:** `uncategorized` 353, `gem_effects` 0 differences.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,764 tests and clippy pass.

## Part 10: damage taken over time

**What PoB does** (`CalcDefence.lua` 1880 to 1940), for each damage type:
- **The base:** the `INC` sum and the more product of `DamageTaken` and `<Type>DamageTaken` (and `ElementalDamageTaken` for fire, cold, and lightning).
- **Over time:** the base with the `DamageTakenOverTime` forms of the same names.
- **The multiplier:** `<Type>TakenDotMult = max((1 - resist / 100) x (1 - reduction / 100) x (1 + inc / 100) x more, 0)`.
  - `resist` is `<Type>ResistOverTime`: the resistance with its base summed with `ModFlag.Dot` (`CalcDefence.lua` 589), so "+#% Chaos Resistance against Damage Over Time" counts there alone. Physical's is 0.
  - `reduction` is the base damage reduction (part 5).
- **The hit's multiplier:** PoB writes a first `<Type>TakenHitMult` here, then overwrites it in its incoming hit (1942 on) with the whole hit's mitigation against the enemy's damage. That field waits for the enemy's hit (next part).

**The engine** (`poe1::taken`, phase `damage-taken` after `damage-reduction`; slice revision 18):
- **The fields:** `<type>-taken-dot-mult` for the five types, always present.
- **The over-time resistance:** `phases::resistance_in` sums the resistance's base in a context with the flags given (`DOT` here), the rest as `resistance`.
- **The quantity:** `SelfDamageTaken(when, of)` (`TakenWhen`: always, hit, over time; `TakenOf`: all, a type, elemental).
- **The rows** (PoB's parse of each line):
  - nine damage taken lines;
  - "#% increased Damage taken from Damage Over Time" (`degen_effect_+%`, was `NoEffect`);
  - "#% increased Physical Damage taken over time";
  - the four "Resistance against Damage Over Time" lines (`Resistance` BASE with `DOT`).
- **The pantheon:** `chaos_damage_taken_+%` (Shakari's soul) left `NoEffect` and the guard test's list (`damage_taken::tests`).
- **Rounding:** PoB rounds its more products per `ModDB:More` call; the engine rounds each quantity's, which is the same while each call has at most one source (the only mapped more is the hidden Marauder ascendancy's).
- **Not modelled:** conditional damage taken, damage taken from a hit's source, reflected damage, and ignoring a resistance or reduction. Each is an unmapped stat, so a reason.

### Evidence

- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected; `Complete` fields 9,280.
  - Each of the five fields equals PoB's in 175 builds, Arakaali's and Shakari's souls included (`ice-nova-pantheon-arakaali-shakari-witch.xml`: fire 1.44, chaos 1.02).
  - No other field's value or state moved.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,764 tests and clippy pass.

## Part 11: the chance to evade

**What PoB does** (`CalcDefence.lua` 1049 to 1096):
- **The evasion of each kind:** `MeleeEvasion` and `ProjectileEvasion` are `max(round(evasion x calcLib.mod(<Kind>Evasion)), 0)`.
- **The special cases:** `CannotEvade` (Unwavering Stance) makes every chance 0; `AlwaysEvade` makes it 100.
- **The enemy's accuracy:** `round(calcLib.val(enemyDB, "Accuracy"))`, the monster accuracy at the enemy's level (`CalcSetup.lua` 575) times its modifiers. Blind gives 20% less (`CalcPerform.lua` 831).
- **Each kind's chance:** `max(0, min(95, (100 - (hitChance(evasion, accuracy) - EvadeChance BASE) x the enemy's hit chance mod) x calcLib.mod(EvadeChance, <Kind>EvadeChance)))`, where `hitChance` is `calcs.hitChance`.
- **`EvadeChance`:** the melee chance when the two are equal.

**The engine** (`poe1::evade`, phase `evade` after `primary-defences`; slice revision 19):
- **The fields:** `evade-chance`, `melee-evade-chance`, `projectile-evade-chance`, present with evasion and the monster level.
- **The shared formula:** `hit::pob_hit_chance` is `calcs.hitChance`, factored out of the character's own accuracy against the enemy.
- **The quantities:**
  - `EnemyAccuracy`: the configured Blind adds its less accuracy beside its less evasion (`damage_taken::conditions`).
  - `CannotEvade`: Unwavering Stance's row; the stat was `NoEffect`, and its "Cannot be Stunned" reaches no field.
- **Not modelled:** no stat of the kinds' evasion, of `EvadeChance`, or of the enemy's hit chance is mapped yet. Each is an unmapped stat, so a reason, and their values are PoB's defaults (1 and 0).

### Evidence

- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected; `Complete` fields 9,826.
  - The three fields equal PoB's in 182 builds; 21 are `partial` and 6 `indeterminate`.
  - No other field's value or state moved.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,764 tests and clippy pass.

## Part 12: the enemy's hit

**What PoB does** (`calcs.buildDefenceEstimations`, `CalcDefence.lua` 1635 to 2105), for the config's default damage category, `Average`:
- **The enemy's crit:**
  - The chance is the config's `enemyCritChance` or its placeholder (5), times `(1 + EnemyCritChance INC / 100 + the enemy's CritChance INC / 100)`, times `(1 - ConfiguredEvadeChance / 100)`, held between 0 and 100.
  - `ConfiguredEvadeChance` for `Average` is `(MeleeEvadeChance + ProjectileEvadeChance) / 4`, PoB's own sum over four.
  - The effect is `1 + chance / 100 x damage / 100 x (1 - CritExtraDamageReduction / 100)`. `damage` is the config's `enemyCritDamage`, or the monster constant `base_critical_strike_multiplier` - 100, plus the enemy's `CritMultiplier` BASE.
- **The enemy's damage of each type:** the config's `enemy<Type>Damage`, or the placeholder `enemyIsBoss` sets.
  - The placeholder is `round(monster damage x 1.5 x mult)` at the boss kind's level, physical alone for a non-boss.
  - `mult` is 1 for a non-boss, 4/4.4 for a boss, 8/4.4 for a Pinnacle boss (84), and 10/4.25 for an Uber boss (85).
  - Chaos is the rounded value over 2.5, or over 4 for the Uber boss.
  - It is multiplied by the enemy's damage modifier and the crit effect.
- **The enemy's penetration:** the config's `enemy<Type>Pen` or the placeholder: 3 for a Pinnacle boss's elements, 8 for an Uber boss's.
- **Taken as:** each type's shares moved to another type (hits and over time), the rest `max(100 - moved, 0)`.
- **The hit,** for each type:
  - `resMult = 1 - (resist - pen) / 100`.
  - The armour's reduction is `calcs.armourReduction(armour x applies / 100, damage x resMult)`; physical's `applies` is 100 (`CalcDefence.lua` 1497). It is held to the maximum, added to the base reduction, and held to the maximum again.
  - `reductMult = 1 - max(min(max, total - overwhelm), 0) / 100`, and `<Type>DamageReduction = 100 - reductMult x 100`.
  - `takenMult` is the mean of the type's spell and attack multipliers from hits. Fortification's `DamageTakenWhenHit` MORE `-stacks` applies while Fortified (`CalcPerform.lua` 632 to 653).
  - `TakenHit = max(damage x (resMult x reductMult) + flat, 0) x takenMult x suppression`, and `<Type>TakenHitMult = TakenHit / damage` (0 without damage).
  - `suppression` is `1 - effect / 100 / 2` at a 100% suppression chance, else 1.

**The engine** (`poe1::enemy_hit`, phase `enemy-hit`; slice revision 20):
- **The fields:** `<type>-taken-hit-mult` and `<type>-damage-reduction`, present with armour and the evade chances.
- **The monster damage table:** `CalcData::monster_damage` is the bundle's `monster_levels` damage (f32, as PoB's table).
- **The quantities:** `EnemyCritChance`, `CritExtraDamageReduction`, `EnemyDamage`, `DamageTakenAs(from, to, hits)`, `ArmourAppliesTo`, `SpellSuppressionEffect`, `MaximumFortification`, and `SelfDamageTaken`'s attack, projectile attack, spell, and spell projectile forms.
- **The rows:** 31 from PoB's parse, three of them former `NoEffect` rows:
  - "You take #% reduced Extra Damage from Critical Strikes";
  - physical damage taken as fire;
  - physical damage taken as lightning.
- **Fortification:** `base_max_fortification` is now a row, so its `NO_EFFECT_DEPENDS_ON` guards are gone; the per-fortification lines stay unmapped.
- **Delirium** adds its increased enemy damage (6 to 30).
- **Enfeeble:**
  - Its accuracy line is the enemy's `EnemyAccuracy` INC, which also moves the evade chance.
  - Its two damage lines are `BuffStat::Defence` markers: PoB conditions them on the enemy being rare or unique or not.
- **`BuffStat::Defence`:** a gem stat that reaches only the enemy's hit, a marker those fields read as an `unmapped-stat` reason. As `NoSlice` elsewhere, and in `gem_effects`. Arctic Armour, Flesh and Stone, the Stone Golem, and Storm Barrier use it too.
- **Not modelled:** another damage category, an unlucky estimate, a boss skill preset, the enemy's impale, armour based on ward, ignored resistances or reductions, and the enemy's modifiers the engine does not carry. The config inputs are `unmodelled-config`; the stats are reasons.

### Evidence

- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected; `Complete` fields 11,516.
  - Each of the ten fields equals PoB's in 169 builds; `smite-charges-templar.xml` has 10 fortification stacks.
  - OccVortex's evade chance moved toward PoB's with Enfeeble's accuracy (2 to 8; PoB 13, the build `indeterminate`).
  - No other field moved.
- **Data parity:** `uncategorized` 353.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,765 tests and clippy pass.

## Part 13: maximum hit taken

**What PoB does** (`CalcDefence.lua` 2214 to 2556 for the pools, 3120 to 3325 for the hit):
- **The life pool:** `LifeRecoverable = max(LifeUnreserved, 1)`. With no life loss prevented, its hit pool is `max(life - half, 0) + min(life, half)`, where `half` is half of maximum life.
- **Each type's total hit pool:** the life pool (Mind over Matter's mana aside), plus energy shield (`EnergyShieldRecoveryCap`) unless the type bypasses it (chaos does, 100%), plus ward, aegis, and guard.
- **Each type's maximum hit:** for each type the hit is taken as:
  - with no armour applied, `max(pool / share / (resMult x (1 - DamageReduction / 100)) - flat, 0) / takenMult`;
  - with armour, PoB's quadratic in the raw hit, `floor(max(min(RAW, maxDR), noDR))`.
- **The result:** `<Type>MaximumHitTaken = round(smallest / enemy damage multiplier)`. A hit split over types needs PoB's smoothing passes.
- **`SecondMinimalMaximumHitTaken`:** the second smallest of the five, in PoB's order.

**The engine** (`enemy_hit::maximum_hit`; slice revision 21):
- **The fields:** `<type>-maximum-hit-taken` and `second-minimal-maximum-hit-taken`. A split hit was `NotComputed`; open items S7 ports the smoothing passes (below, "Open items S7, part 1").
- **Not modelled config:** `conditionLowLife` and `conditionLowEnergyShield` are `unmodelled-config`.
- **The markers,** split by the fields they change:
  - **`EnemyHitReason`:** the hit's fields and maximum hit (Arctic Armour, Storm Barrier, Flesh and Stone, Enfeeble's damage, no extra crit damage if crit recently).
  - **`PoolReason`:** maximum hit alone. The guard skills are `BuffStat::Defence("guard")` (eight entries, Stone Golem's damage removed among them), Mind over Matter and Divine Shield (former `NoEffect`), and half of area damage taken.
  - **`DotTakenReason`:** damage taken over time alone (the caustic cloud line).
- **Settled conditions:** "if you've been Hit Recently" (`conditionBeenHitRecently`, or `multiplierBeenHitRecently` above 0), its negation, and "while moving" (`conditionMoving`) are now settled conditions (`poe1::conditions`). This gives three state rows: elemental damage taken if hit or not hit recently, and physical damage over time taken while moving. Before, these were `NoEffect`.
- **`NoEffect` again:** "no extra bleed damage while moving" (no field reads it).
- **The math module** gains `sqrt` (correctly rounded).

### Evidence

- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected; `Complete` fields 12,539.
  - Each maximum hit equals PoB's in 168 builds (167 for physical and the second smallest).
  - The hit recently and moving rows bring nine partial builds' taken multipliers to or toward PoB's (Inquisitor, Solaris's soul, Animate Guardian).
  - One build's over-time multipliers became partial with Shakari's caustic cloud line.
  - Two builds' hit fields became complete again (the Stone Golem's line is a pool marker).
- **Data parity:** `uncategorized` 353.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,765 tests and clippy pass.

## Part 14: the effective health pool

**What PoB does** (`CalcDefence.lua` 188 to 427, 1645 to 1667, 2568 to 2913), for `Average`:
- **Hits to die** (`numberOfHitsToDie`): with the pools full, each enemy hit of `<Type>TakenHit` (each part rounded up) drains energy shield first, unless the type bypasses it, then life (`calcs.reducePoolsByDamage`).
  - It runs until life is 0 or for 50 iterations.
  - After the first hit it runs itself on eight times the damage and counts `max((hits - 1) x 8 - 1, 1)` hits at once.
  - The last hit's overkill over its damage is taken off; 10^8 damage survived is infinite.
- **Mitigated hits:** each type's hit times `blockEffect x suppressionEffect x (1 - avoid / 100)`.
  - `blockEffect = 1 - block x BlockEffect / 100`, where `block` is the four effective block chances averaged over 100.
  - `suppressionEffect = 1 - chance / 2 x SpellSuppressionEffect / 100` below a 100% chance.
  - `ConfiguredDamageChance` is 100 times the product; the hits are counted again unless it is 100.
- **Not hit:** the four categories' chances not to be hit (evade, dodge, avoiding all damage, avoiding projectiles), averaged, give `ConfiguredNotHitChance`.
  - `TotalNumberOfHits = mitigated / (1 - not hit / 100)`.
  - `TotalEHP = TotalNumberOfHits x` the enemy's damage before its modifiers.

**The engine** (`poe1::ehp`, from the enemy's hit; slice revision 22):
- **The fields:** `number-of-damaging-hits`, `number-of-mitigated-damaging-hits`, `total-number-of-hits`, `configured-damage-chance`, `configured-not-hit-chance`, and `total-ehp`, present with every type's hit. An infinite estimate has no value.
- **The pools:** life and energy shield (chaos bypasses it).
- **The rows:**
  - Glancing Blows' "You take 65% of Damage from Blocked Hits" is `BlockEffect` BASE 65 (`keystones`).
  - "#% chance to avoid Projectiles" is `AvoidProjectiles` (was `NoEffect`).
- **`EhpReason`:** a new marker, for life and energy shield gained on block and spell block, and lucky suppression (were `NoEffect`).
- **Not modelled:** mind over matter, guard, ward, aegis, allies, life loss prevented, recoup, gains on block or suppression, dodge, avoiding all damage or a type's damage, lucky block, and the enemy reducing block. These are reasons: the markers of parts 12 to 14, or unmapped stats.

### Evidence

- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected; `Complete` fields 13,547.
  - Each of the six fields equals PoB's in 168 builds, Glancing Blows' fixture included.
  - No other field moved.
- **Data parity:** `uncategorized` 353.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,766 tests and clippy pass.

## Part 15: mind over matter

**What PoB does** (`CalcDefence.lua` 188 to 427, 2303 to 2377):
- **The share:** `sharedMindOverMatter = min(DamageTakenFromManaBeforeLife BASE, 100)`, and each type's own share `min(<Type>DamageTakenFromManaBeforeLife BASE, 100 - shared)`. Mind over Matter's keystone is the tree's "40% of Damage is taken from Mana before Life".
- **Maximum hit:** the life pool gains the mana mind over matter can take (`calcMoMEBPool` without Eldritch Battery).
  - `max = life / (1 - share) - life`;
  - `usable = floor(min(mana unreserved, max))`;
  - the used mana is `floor(min(max, usable))`.
- **Hits to die:** after energy shield, `ceil(remainder x share)` of each hit comes from mana, held to the type's pool (`floor(min(max, mana))`), and every type's pool drops by it.

**The engine** (slice unchanged):
- **The quantity** is `ManaBeforeLife(TakenOf)`.
- **The keystone:** `keystone_mana_shield` is a keystone row, BASE 40 (`keystones::MIND_OVER_MATTER`). It was a `PoolReason` marker.
- **Three rows:** "#% of Damage is taken from Mana before Life", and its lightning and physical forms.
- **Where it is read:** `enemy_hit::mom_hit_pool` for maximum hit, and the `ehp` pools (`Character::mana` and `mom`).
- **Not modelled:** Eldritch Battery (energy shield protecting mana, an unmapped stat), the chaos form (PoB misreads it as `ChaosDamage`), and the shares conditioned on Clarity or being Focused (unmapped).

### Evidence

- **Fixtures:** two new mechanic fixtures, every field equal to PoB's:
  - `mom-ice-nova-witch.xml`: 40%, +400 mana; maximum hits 1,875 and 1,150, EHP 1,243.13;
  - `mom-physical-ice-nova-witch.xml`: 30% of physical damage; physical maximum hit 1,607, the others unchanged at 690.
- **Public `calc-parity` on `b-ot`:** 212 builds, 280 expected, 0 unexpected; `Complete` fields 13,701. Eight values in partial builds moved toward PoB, none away.
- **Data parity:** `uncategorized` 353.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,766 tests and clippy pass. The maintainer-run tests on the evidence bundle pass too: the pantheon's soul stats list those parts 5 and 10 to 15 read, the keystone texts list Mind over Matter's, and Enemy Explode left P6-9f's inert skills (PoB's id is not one of the game's granted effects).

## P6-11ce: life loss prevented

A flask's "When Hit during effect, #% of Life loss from Damage taken occurs over 4 seconds instead" is PoB's `LifeLossPrevented` BASE with `UsingFlask` (`ModParser.lua` 5082). PoB parses no other source of it. Petrified Blood's below-half share comes from a skill the engine does not model.

What PoB does with `preventedLifeLoss` (at most 100) and nothing prevented below half:
- **The hit pool** (`calcLifeHitPoolWithLossPrevention`, `CalcDefence.lua` 177 to 181): `aboveLow / (1 - p / 100) + min(life, half) / (1 - p / 100)`. Maximum hit taken and mind over matter read it (2245, 2326).
- **Each hit** (`reducePoolsByDamage`, 348 to 384):
  - the damage life can still take caps the hit, the rest overkill;
  - the prevented share of what is left is lost over time instead (`LifeLossLostOverTime`, which only the breakdown shows);
  - life takes the rest.
- **The speed-up:** both hits-to-die runs limit it to 4 (`LimitEHPSpeedup`, 2742, 2837), and the mitigated run is always made (2840).

What the engine does:
- `Poe1Quantity::LifeLossPrevented` is a `ROWS` row, in `flasks::FLASK_ROWS`.
- `enemy_hit` sums it, at most 100, into the maximum hit's life pool (`ehp::life_hit_pool`) and the defence estimate's `Character::prevented`.
- `ehp::reduce` applies the cap and the prevented share before life takes damage.
- `ehp::estimate` limits the speed-up of both runs, and makes the mitigated one.

**Fixture** `ehp-life-loss-prevented-flask-ice-nova-witch.xml`: an active Basalt Flask preventing 25%. Every computed field equals PoB's, the hits to die and `TotalEHP` among them.

**Evidence** (`b-ct`, against P6-11cd's run):
- **Public:** 0 unexpected; 28 values toward PoB (three builds' maximum hits and defence estimate), none away.
- **Corpus:** unchanged.


## Open items S7, part 1: maximum hit's smoothing passes

**What PoB does** (`CalcDefence.lua` 3164 to 3258): for a hit of one type partly taken as others (`PhysicalDamageFromHitsTakenAsFire` and kin) with armour applied, each part's quadratic gives an approximate maximum hit; PoB takes the smallest and runs up to 8 smoothing passes (`data.misc.maxHitSmoothingPasses`):
- each pass takes the hit from full pools: `calcs.takenHitFromDamage` (149 to 175: per type, the converted share times resistance and `1 - max(min(DRmax, min(DRmax, armourReductionF + base reduction) - overwhelm), 0) / 100`, plus the flat damage taken, times the multiplier after reduction, rounded), then `calcs.reducePoolsByDamage(nil, ...)` (188 to 421);
- the pass's overkill is `OverkillDamage - hitPoolRemaining` (`hitPoolRemaining`: the life hit pool left, plus the smallest mind over matter pool and the smallest energy shield pool left over the hit's types);
- the incoming damage moves by the overkill, scaled by a step from the previous pass's overkill, and is divided by the square root of the largest `(overkill + pool) / pool` over the parts; it stops when the overkill is within 1;
- `<Type>MaximumHitTaken = round(incoming / enemy damage multiplier)`.

**The engine:** `enemy_hit::smoothed` and `enemy_hit::taken_hit_from_damage`, with `ehp::overkill_from_full` (`ehp::reduce` now also gives `hitPoolRemaining`). The split hit's field is computed instead of `NotComputed`. Vaal Arctic Armour's mitigation stays out of maximum hit, as before.

**Fixtures:** `max-hit-taken-as-armour-marauder.xml` (armour, 20% of physical taken as fire and 20% as cold) and `max-hit-taken-as-energy-shield-marauder.xml` (an energy shield helmet, 30% as lightning, 10% as chaos): every maximum hit, `SecondMinimalMaximumHitTaken`, and `TotalEHP` equal PoB's; 66 targets each.

**Evidence** (`local/p6/s7a`, `b-ct`, against S4's run):
- **Public:** 0 unexpected; `Complete` 41,654 to 41,786 (the fixtures). 20 fields in 10 builds computed for the first time (physical and the second smallest), partial with their builds' reasons; in two (`ice-nova-elementalist-mapping.xml`, `kinetic-blast-of-clustering-necromancer.xml`) they equal PoB's; in the other eight every type's maximum hit already differs from PoB's for those reasons (energy shield, armour, or the taken multipliers). No other value moved.
- **Corpus:** 0 unexpected; 10 fields in 5 builds computed (partial or indeterminate); no value moved.
- **Tests:** `ehp::tests::the_hit_pool_left_should_be_pobs`; the workspace tests, the real-bundle tests, clippy, and rustfmt pass.


## Open items S7, part 2: a dual wielder's leech

**What PoB does:** each hand's pass computes its leech per hit, its instances (`duration x hitRate`), and its instant rate, with `hitRate = HitChance / 100 x globalOutput.Speed` (`CalcOffence.lua` 3211, 3632 to 3667): the hand's own chance to hit and the hands' combined speed. `combineStat` `DPS` then joins each value (3877 to 3891; 2099 to 2103): `(main + off) / 2`, or the sum for a skill that hits with both weapons (`doubleHitsWhenDualWielding`). The rate follows from the joined values as for one hand (3981 to 4005).

**The engine:** `leech::rates` is split into the pass's part (`hand_leech`: the leech per hit, the instant share, the instances and instant rate) and the rate (`finish`); the one-hand path makes the same graph as before (its documents are byte-identical). `leech::rates_dual` runs each hand's part at the combined speed and joins them, a hand that does not leech adding 0; the instance rate is read in the main hand's context (PoB's `skillCfg`, which no modelled leech rate modifier tells from a hand's). A dual-wielding attack's leech fields were `NotComputed`.

**Fixture:** `smite-dual-wield-leech-templar.xml` (the dual-wield lines fixture with 0.4% of attack damage leeched as life, 0.2% of physical attack damage as life, 0.3% of attack damage as mana, and 8% of leech instant). Its life and mana leech rates are PoB's times the attack rate's Q11 ratio exactly (0.999893497594129 for both and for `Speed`): DIV-0720 to DIV-0726, Q11. A Dual Strike copy (the sum) gives the same ratio, 0.999893497594129, but stays partial while Dual Strike's PoB skill data is not ported (S1), so it is not a fixture.

**Evidence** (`local/p6/s7b`, `b-ct`, against part 1's run):
- **Public:** 0 unexpected; `Complete` 41,786 to 41,866 (the fixture). No other value or state moved; the three dual-wield fixtures' documents changed in graph nodes only.
- **Corpus:** 0 unexpected; documents byte-identical.
- **Tests:** the workspace tests, the real-bundle tests, clippy, and rustfmt pass.


## Open items S7, part 3: a dual wielder's damaging ailments

**What PoB does:** each hand's pass computes its ailments with its own crit and chance to hit, the stacks at the hands' combined speed (`globalOutput.Speed`, `CalcOffence.lua` 4323, 4607). `combineStat` then joins them (5528 to 5545; 2002 to 2104):
- **Bleeding and ignite** (`CHANCE_AILMENT`): one hand's alone when only it inflicts the ailment; otherwise `max x r + min x (1 - r)`, `r = min(1, <Ailment>Stacks / <Ailment>StacksMax)` of the shared output. For bleeding that is the stacks of the last pass that bled, the off hand's (4476), over its maximum. Ignite's stacks are never written, so `r = 1` and the larger hand's ignite counts.
- **Poison** (`TotalPoisonDPS`, `DPS`): `(main + off) / 2`, 0 for a hand without it, the sum for a skill that hits with both weapons.

**The engine:** `ailment::damaging` returns a `Damaging` with its stacks; the dual-wield path runs it per hand at the combined speed (with the skill's PoB data, as one hand does) and joins the hands in `hit::combine_ailment`, the bleed maximum the 1 the engine models. A dual-wielding attack's ailment fields were `NotComputed`. Burning ground stays not computed for a dual wielder (PoB does not combine it), and so does impale (PoB combines its inputs, not its DPS).

**Fixtures** (both from the dual-wield lines fixture): `smite-dual-wield-bleed-poison-templar.xml` (both hands bleed and poison) and `smite-dual-wield-ignite-sword-templar.xml` (fire on the off hand's sword only, so its ignite alone; both poison). Poison and ignite are PoB's times the attack rate's Q11 ratio exactly (0.9998934975941292). Bleeding is not linear in the rate (the off hand's stacks weight it); the engine's larger hand (9.7236), smaller hand (6.7177), and share (0.8572), each rescaled to PoB's rate, recombine to PoB's 9.29565303191029 exactly. Registered as Q11: DIV-0727 to DIV-0740.

**Evidence** (`local/p6/s7c`, `b-ct`, against part 2's run):
- **Public:** 0 unexpected; `Complete` 41,866 to 42,026 (the fixtures). No other document changed.
- **Corpus:** 0 unexpected; documents byte-identical.
- **Tests:** the workspace tests, the real-bundle tests, clippy, and rustfmt pass.


## Open items S7, part 4: a dual wielder's impale

**What PoB does** (`CalcOffence.lua` 5472 to 5520 per pass, 5569 to 5571 and 3901 to combine, 6027 to 6047): each hand's pass computes its stored hit (`impaleStoredHitAvg`), `ImpaleChance` (`min(100, sum)`), and `ImpaleModifier`. For a skill that hits with both weapons, `ImpaleDPS` is the sum of each hand's `stored x (modifier - 1) x HitChance / 100`; otherwise it reads the combined values: the stored hit and the hit chance averaged, `ImpaleModifier` by `CHANCE` (each hand's weighted by `ImpaleChance x HitChance` over both). Then times the combined speed.

**The engine:** `ailment::impale` is split into the pass's `impale_parts` and the DPS (one-hand documents unchanged); `hit::combine_impale` joins the hands as PoB. When only one hand impales, PoB still averages the other hand's stored hit in, which the parts do not carry: that case stays `NotComputed`. Mapped on the way: `impale_on_hit_%_chance_with_axes_swords` ("#% chance to Impale Enemies on Hit with Axes or Swords"), PoB's `ImpaleChance` BASE with `Hit` and Axe or Sword (`ModParser.lua` 917).

**Fixture:** `smite-dual-wield-impale-templar.xml` (20% chance to impale with attacks, 10% more with axes or swords, so the two hands' chances weight the modifier): `ImpaleDPS` is PoB's times the attack rate's Q11 ratio exactly (0.999893497594129); DIV-0741 to DIV-0746, Q11. The summed branch (a skill that hits with both weapons) has no fixture: Cleave's and Viper Strike's PoB skill data are not ported (S1), so their fields stay partial or not computed; the same holds for parts 2 and 3's summed branches.

**Evidence** (`local/p6/s7d`, `b-ct`, against part 3's run):
- **Public:** 0 unexpected; `Complete` 42,026 to 42,105 (the fixture). No other document changed.
- **Corpus:** 0 unexpected; documents byte-identical.
- **Tests:** the workspace tests, the real-bundle tests, clippy, and rustfmt pass.

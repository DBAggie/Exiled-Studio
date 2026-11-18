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


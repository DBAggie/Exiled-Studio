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

**Not modelled:** energy shield regeneration (next part); no regeneration and cannot gain (`No<Pool>Regen`, `CannotGain<Pool>`), and life regeneration moved to another pool; degeneration (Righteous Fire's own degeneration stays as decided in D7 until a later part).

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

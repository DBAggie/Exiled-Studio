//! Config inputs (Phase 3 plan, step 12; P3-12a): which of PoB's config
//! inputs can change a computed field.
//!
//! The engine models two inputs (`resistancePenalty` and the enemy
//! model's numeric inputs, [`super::collect`]); every other input is
//! `unmodelled-config` on every computed field, except the ones this
//! module lists: inputs whose only effect is a named condition or
//! multiplier that modifiers read ([`NAMED_ONLY`]).
//!
//! Why such an input changes no computed field: in PoB v2.67.2 the input
//! adds `Condition:<name>` or `Multiplier:<name>` mods
//! (`Modules/ConfigOptions.lua`), and only modifiers read them. Every
//! modifier comes from a stat, and every stat the engine ingests is one
//! of three kinds:
//! - mapped: no mapping reads a named condition (a test in
//!   [`super::mapping`] checks the global and hit rows; local and weapon
//!   rows are built with no condition);
//! - `NoEffect`: the stat changes no slice field whatever the config
//!   (P3-8a's criterion, `docs/phase-3/stat-mapping.md`);
//! - a reason (`unmapped-stat`, `unmodelled-gem`, and the others), which
//!   already makes every computed field `Partial`.
//!
//! And where an effect reads a named condition, the store leaves it out
//! with `unmodelled-condition` ([`crate::condition`]): the engine never
//! sets a named condition from the config, so a future mapping that reads
//! one is a reason on its own, never a silent value.
//!
//! An input stays here only if PoB's calc modules (`CalcSetup.lua`,
//! `CalcPerform.lua`, `CalcOffence.lua`, `CalcDefence.lua`) read its names
//! only through modifiers or flags a modifier sets, or add from them only
//! effects outside the slice; each row's comment gives what was checked.
//! The stats behind those modifiers are [`DEPENDS_ON`]: they must stay
//! reasons (`unmapped-stat`), and mapping one (even as `NoEffect`) must
//! revisit this table (a test checks). Inputs with their
//! own effects (charges, rage, fortification, shock and its effect,
//! exposure, a minion's aura, custom modifiers, bandit and pantheon) stay
//! reasons.

/// Config inputs whose only effect is named conditions or multipliers
/// that modifiers read, with those names (PoB v2.67.2,
/// `Modules/ConfigOptions.lua`), sorted by input (a test checks the
/// order). Matched by name whatever the value's type, unlike the enemy
/// inputs: PoB's `apply` keys only on the value (a list input sets a
/// condition per value, a count a multiplier, a check a condition), so
/// every value only sets names. The names are documentation; no code
/// reads them.
///
/// - `OverloadedIntensity`, `intensifyIntensity`: `Multiplier:
///   OverloadedIntensity` (at most 3), `Multiplier:Intensity` (lines 514, 517); read
///   by the skill stat map of the gems that grant intensity.
/// - `bloodSandStance`: `Condition:SandStance` for Sand (line 654);
///   `CalcSetup.lua` (652) adds `Condition:BloodStance` when not Sand;
///   read by the stance skills' stats.
/// - `conditionBeenHitRecently`, `conditionBlockedRecently`,
///   `conditionChilledEnemyRecently`, `conditionShockedEnemyRecently`,
///   `conditionFullLife`: the condition of that name (lines 1375, 1421,
///   1339, 1351, 174); the calc modules read them only through modifiers
///   or flags a modifier sets (`CalcDefence.lua` 707, 723: maximum block
///   if not blocked recently, from a stat); `FullLife` is set by Chaos
///   Inoculation (`CalcPerform.lua` 86), not read.
/// - `conditionConvergence`: `Condition:Convergence`, itself conditioned
///   on `Condition:CanGainConvergence` (line 1441), which only a modifier
///   sets ("Gain Convergence when you Hit a Unique Enemy", `ModParser.lua`
///   2680). `CalcSetup.lua` (129) turns the condition into the
///   `Convergence` flag, and `CalcPerform.lua` (789) into more elemental
///   damage: without that modifier, nothing.
/// - `conditionCritRecently`: `Condition:CritRecently` and
///   `Condition:SkillCritRecently` (line 1251).
/// - `conditionEnemyChilled`, `conditionEnemyChilledEffect`: the enemy's
///   `Condition:Chilled`, `Condition:ChilledConfig`, `ChillVal`, and
///   `DesiredChillVal` (never read) (lines 1899, 1907). `CalcPerform.lua` (3527) gives a chilled enemy less
///   action speed (not a slice field); damage taken only with a modifier's
///   flag (`ChillEffectIncDamageTaken` and the others, `ModParser.lua`
///   3877) or Bonechill's skill data (a gem).
/// - `conditionEnemyOnConsecratedGround`: the enemy's
///   `Condition:OnConsecratedGround` (line 2012); `CalcPerform.lua`
///   (3688) adds damage taken from `DamageTakenConsecratedGround`, which
///   only a modifier gives.
/// - `conditionOnConsecratedGround`: `Condition:OnConsecratedGround`, also
///   for minions (line 1148); `CalcPerform.lua` adds life regeneration
///   and less curse effect on you (804, 3171; not slice fields) and
///   accuracy from `ConsecratedGroundAlsoAccuracy` (a modifier).
/// - `conditionStationary`: `Multiplier:StationarySeconds`, and
///   `Condition:Stationary` when above 0 (line 158).
/// - `multiplierNearbyEnemies`: `Multiplier:NearbyEnemies` and
///   `Condition:OnlyOneNearbyEnemy` (line 1234); the calc modules never
///   read them.
pub const NAMED_ONLY: &[(&str, &[&str])] = &[
    ("OverloadedIntensity", &["Multiplier:OverloadedIntensity"]),
    (
        "bloodSandStance",
        &["Condition:SandStance", "Condition:BloodStance"],
    ),
    ("conditionBeenHitRecently", &["Condition:BeenHitRecently"]),
    ("conditionBlockedRecently", &["Condition:BlockedRecently"]),
    (
        "conditionChilledEnemyRecently",
        &["Condition:ChilledEnemyRecently"],
    ),
    ("conditionConvergence", &["Condition:Convergence"]),
    (
        "conditionCritRecently",
        &["Condition:CritRecently", "Condition:SkillCritRecently"],
    ),
    (
        "conditionEnemyChilled",
        &["enemy Condition:Chilled", "enemy Condition:ChilledConfig"],
    ),
    (
        "conditionEnemyChilledEffect",
        &["enemy ChillVal", "enemy DesiredChillVal"],
    ),
    (
        "conditionEnemyOnConsecratedGround",
        &["enemy Condition:OnConsecratedGround"],
    ),
    ("conditionFullLife", &["Condition:FullLife"]),
    (
        "conditionOnConsecratedGround",
        &["Condition:OnConsecratedGround"],
    ),
    (
        "conditionShockedEnemyRecently",
        &["Condition:ShockedEnemyRecently"],
    ),
    (
        "conditionStationary",
        &["Multiplier:StationarySeconds", "Condition:Stationary"],
    ),
    ("intensifyIntensity", &["Multiplier:Intensity"]),
    (
        "multiplierNearbyEnemies",
        &["Multiplier:NearbyEnemies", "Condition:OnlyOneNearbyEnemy"],
    ),
];

/// The game stats through which PoB turns a [`NAMED_ONLY`] input into a
/// slice field, sorted (a test checks the order, and that each is
/// `Unmapped` through every mapping): the argument holds only while each
/// is a reason. PoB's own text for each (`ModParser.lua`) or skill data:
///
/// - `additional_spell_block_%_if_havent_blocked_recently`,
///   `chance_to_block_attack_damage_if_not_blocked_recently_%`,
///   `max_chance_to_block_attacks_if_not_blocked_recently`,
///   `spell_block_is_maximum_if_not_blocked_recently`: block conditioned
///   on not `BlockedRecently` (4146, 4147; `CalcDefence.lua` 707, 723).
/// - `consecrated_ground_enemy_damage_taken_+%`: the enemy's
///   `DamageTakenConsecratedGround`; `your_consecrated_ground_grants_
///   accuracy_rating_+%`: `ConsecratedGroundAlsoAccuracy` (4176).
/// - `enemies_chilled_by_hits_take_damage_increased_by_chill_effect`,
///   `keystone_cold_purist`: chill's damage taken flags (3877 to 3879).
/// - `gain_convergence_on_hitting_unique_enemy`:
///   `Condition:CanGainConvergence` (2680).
/// - `support_chilling_areas_also_grant_cold_damage_taken_+%_equal_to_slow_amount`,
///   `support_chills_also_grant_cold_damage_taken_+%_equal_to_slow_amount`:
///   Bonechill's stats. PoB's effect is the gem's skill data
///   (`supportBonechill`, `Data/Skills/sup_int.lua` 560; `CalcPerform.lua`
///   3536: cold damage taken equal to the configured chill), and its stat
///   map ignores these two stats, so they are the only trace the engine
///   sees: classifying them as `NoEffect` would make
///   `conditionEnemyChilled` silent.
pub const DEPENDS_ON: &[&str] = &[
    "additional_spell_block_%_if_havent_blocked_recently",
    "chance_to_block_attack_damage_if_not_blocked_recently_%",
    "consecrated_ground_enemy_damage_taken_+%",
    "enemies_chilled_by_hits_take_damage_increased_by_chill_effect",
    "gain_convergence_on_hitting_unique_enemy",
    "keystone_cold_purist",
    "max_chance_to_block_attacks_if_not_blocked_recently",
    "spell_block_is_maximum_if_not_blocked_recently",
    "support_chilling_areas_also_grant_cold_damage_taken_+%_equal_to_slow_amount",
    "support_chills_also_grant_cold_damage_taken_+%_equal_to_slow_amount",
    "your_consecrated_ground_grants_accuracy_rating_+%",
];

/// Whether `input` only sets named conditions or multipliers that
/// modifiers read ([`NAMED_ONLY`]).
#[must_use]
pub fn named_only(input: &str) -> bool {
    NAMED_ONLY
        .binary_search_by(|(name, _)| (*name).cmp(input))
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_should_be_sorted_and_unique() {
        assert!(NAMED_ONLY.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(NAMED_ONLY.iter().all(|(_, names)| !names.is_empty()));
    }

    #[test]
    fn named_only_should_find_listed_inputs_alone() {
        for (input, _) in NAMED_ONLY {
            assert!(named_only(input), "{input}");
        }
        // Inputs with their own effects stay reasons.
        for input in [
            "useFrenzyCharges",
            "usePowerCharges",
            "useEnduranceCharges",
            "multiplierRage",
            "buffFortification",
            "conditionEnemyShocked",
            "conditionShockEffect",
            "conditionEnemyColdExposure",
            "summonLightningGolemEnableWrath",
            "customMods",
            "bandit",
            "pantheonMajorGod",
            "pantheonMinorGod",
            "enemyIsBoss",
            "resistancePenalty",
        ] {
            assert!(!named_only(input), "{input}");
        }
    }

    #[test]
    fn the_stats_the_rule_depends_on_should_stay_reasons() {
        use crate::ingest::{StatMap, StatMapping};
        use crate::poe1::ArmourSlot;
        use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
        use loom_domain::StatId;

        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in DEPENDS_ON {
            for m in mappings {
                assert_eq!(
                    m.map(&StatId::new(*stat)),
                    StatMap::Unmapped,
                    "{stat}: mapping it must revisit NAMED_ONLY"
                );
            }
        }
    }

    #[test]
    fn the_table_should_not_overlap_the_modelled_inputs() {
        for (input, _) in NAMED_ONLY {
            assert_ne!(*input, super::super::collect::RESISTANCE_PENALTY_INPUT);
            assert!(
                !super::super::enemy::ENEMY_INPUTS.contains(input),
                "{input}"
            );
        }
    }
}

//! Config inputs (Phase 3 plan, step 12; P3-12a, P3-12b): which of PoB's
//! config inputs and placeholders can change a computed field.
//!
//! The engine models two inputs (`resistancePenalty` and the enemy
//! model's numeric inputs, [`super::collect`]); every other input is
//! `unmodelled-config` on every computed field, except the ones this
//! module lists: inputs whose only effect is a named condition,
//! multiplier, or distance that modifiers read ([`NAMED_ONLY`]), and the
//! enemy's damage, which only PoB's damage-taken estimates read
//! ([`ENEMY_DAMAGE`]).
//!
//! **Placeholders** (P3-12b). PoB applies a numeric input's placeholder
//! when the input is unset, or 0 unless the option is `countAllowZero`
//! (`Classes/ConfigTab.lua`,
//! `BuildModList`, line 1147), through the input's own `apply`, and its
//! calc modules read a few placeholders directly (`env.configPlaceholder`:
//! the enemy's damage, the distances). So a placeholder is classified as
//! its input is: [`NAMED_ONLY`] and [`ENEMY_DAMAGE`] hold for both. The
//! enemy's level, resistances, armour, and evasion placeholders are
//! [`super::enemy::RESET_PLACEHOLDERS`]: PoB's `enemyIsBoss` sets them on
//! every run, so a build's saved ones are not read. Any other placeholder
//! is `unmodelled-config` under its input's name.
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
//! A distance (`meleeDistance`, `projectileDistance`) is the same case:
//! PoB reads it as the skill's `skillDist`, and only the `DistanceRamp`
//! and `MeleeProximity` tags of modifiers read that
//! (`Classes/ModStore.lua` 599, 619).
//! The stats behind those modifiers are [`DEPENDS_ON`]: they must stay
//! reasons (`unmapped-stat`), and mapping one (even as `NoEffect`) must
//! revisit this table (a test checks). Inputs with their
//! own effects (rage, shock and its effect, exposure, a minion's aura,
//! bandit and pantheon) stay reasons; charges have their effects
//! ([`super::charges`], P3-12e). Custom
//! modifier blocks are not inputs: their lines act through their
//! resolutions (`collect::config`, P3-12c), and so do the lines of PoB's
//! legacy `customMods` input where PoB moves them into a block
//! (`collect::legacy_custom_mods`, P3-12d).

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
///
/// Added in P3-12b, the inputs whose placeholders PoB writes into a build
/// (each has a `defaultPlaceholderState`):
///
/// - `ScorchStacks`, `ShockStacks`: the enemy's `Multiplier:ScorchStacks`,
///   `Multiplier:ShockStacks` (lines 1892, 1951). `CalcPerform.lua` (3550,
///   3566) reads them in the shock and scorch effects, only with the
///   `ShockCanStack` or `ScorchCanStack` flag, which only a modifier sets
///   (`ModParser.lua` 3045, 3047).
/// - `aspectOfLunarisLunarProtection`, `aspectOfLunarisLunarResistance`,
///   `aspectOfSolarisSunscaldStacks`, `configSpectralTigerCount`,
///   `multiplierHallowingFlameStacksRemovedByAlly`, `sigilOfPowerStages`:
///   `Multiplier:LunarProtection`, `LunarResistance`, `SunscaldStack`,
///   `SpectralTigerConfig` and `SpectralTigerCount`,
///   `HallowingFlameStacksRemovedByAlly`, `SigilOfPowerStage` (lines 262,
///   265, 269, 644, 1994, 631); the calc modules never read them; the
///   skills' stat maps and modifiers do (`HallowingFlameStacksRemovedByAlly`:
///   extra lightning damage per flame removed, `ModParser.lua` 2898, its
///   stat in [`DEPENDS_ON`]).
/// - `meleeDistance`, `projectileDistance`: no `apply` (lines 1789,
///   1790); `CalcActiveSkill.lua` (457) makes the input or placeholder the
///   skill's `skillDist`, which only modifier tags read (module docs).
///   `CalcOffence.lua` (1083, 1086) adds such modifiers for the
///   `PointBlank` and `FarShot` flags, which only modifiers set.
/// - `multiplierEnemyHallowingFlame`: the enemy's
///   `Multiplier:HallowingFlame` (line 1991); the calc modules never read
///   it (`CalcPerform.lua` 3440 reads the condition, not the multiplier,
///   and only with `CanInflictHallowingFlame`, a modifier's flag).
/// - `multiplierWitheredStackCountSelf`: `Multiplier:WitheredStack` (line
///   1231); `CalcPerform.lua` (852) reads it for chaos damage taken by
///   you (not a slice field), only with `CanBeWithered`, a skill's or
///   map's flag.
///
/// Added in P3-12e:
///
/// - `buffFortification`, `overrideFortification`: `Condition:Fortified`
///   and `FortificationStacks` OVERRIDE in combat (lines 1039, 1042).
///   `CalcSetup.lua` (94) turns the condition into the `Fortified` flag;
///   `CalcPerform.lua` (659 to 680) then sets the stacks (the override or
///   the maximum), `DamageTakenWhenHit` MORE per stack (damage taken, not a
///   slice field), the `HaveMaximumFortification` flag, and one more
///   `BuffOnSelf`; 3889 exports the stacks to a party (not a slice
///   field). Every other reader is a modifier (per fortification, while
///   fortified, at maximum fortification, per buff on you).
pub const NAMED_ONLY: &[(&str, &[&str])] = &[
    ("OverloadedIntensity", &["Multiplier:OverloadedIntensity"]),
    ("ScorchStacks", &["enemy Multiplier:ScorchStacks"]),
    ("ShockStacks", &["enemy Multiplier:ShockStacks"]),
    (
        "aspectOfLunarisLunarProtection",
        &["Multiplier:LunarProtection"],
    ),
    (
        "aspectOfLunarisLunarResistance",
        &["Multiplier:LunarResistance"],
    ),
    (
        "aspectOfSolarisSunscaldStacks",
        &["Multiplier:SunscaldStack"],
    ),
    (
        "bloodSandStance",
        &["Condition:SandStance", "Condition:BloodStance"],
    ),
    (
        "buffFortification",
        &["Condition:Fortified", "Fortified", "FortificationStacks"],
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
    (
        "configSpectralTigerCount",
        &[
            "Multiplier:SpectralTigerConfig",
            "Multiplier:SpectralTigerCount",
        ],
    ),
    ("intensifyIntensity", &["Multiplier:Intensity"]),
    ("meleeDistance", &["skillDist"]),
    (
        "multiplierEnemyHallowingFlame",
        &["enemy Multiplier:HallowingFlame"],
    ),
    (
        "multiplierHallowingFlameStacksRemovedByAlly",
        &["Multiplier:HallowingFlameStacksRemovedByAlly"],
    ),
    (
        "multiplierNearbyEnemies",
        &["Multiplier:NearbyEnemies", "Condition:OnlyOneNearbyEnemy"],
    ),
    (
        "multiplierWitheredStackCountSelf",
        &["Multiplier:WitheredStack"],
    ),
    ("overrideFortification", &["FortificationStacks"]),
    ("projectileDistance", &["skillDist"]),
    ("sigilOfPowerStages", &["Multiplier:SigilOfPowerStage"]),
];

/// The enemy's damage (PoB v2.67.2, `Modules/ConfigOptions.lua` 2352 to
/// 2376), sorted (a test checks): inputs with no `apply`, which only
/// `calcs.buildDefenceEstimations` reads (`CalcDefence.lua` 1737 to 1757,
/// 2965, 3552: damage taken, effective life, maximum hit; no slice field)
/// as an input or placeholder. `enemyDamageRollRange` is also read by
/// `presetBossSkills`'s `apply` (2286), which only sets the damage
/// placeholders. No effect for an input or a placeholder.
pub const ENEMY_DAMAGE: &[&str] = &[
    "enemyChaosDamage",
    "enemyColdDamage",
    "enemyColdPen",
    "enemyCritChance",
    "enemyCritDamage",
    "enemyDamageRollRange",
    "enemyFireDamage",
    "enemyFirePen",
    "enemyLightningDamage",
    "enemyLightningPen",
    "enemyPhysicalDamage",
    "enemyPhysicalOverwhelm",
    "enemySpeed",
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
///
/// Added in P3-12b:
///
/// - `can_apply_additional_scorch`, `raider_shocks_you_apply_can_stack_up_to_50_times`:
///   `ScorchCanStack`, `ShockCanStack` (`ModParser.lua` 3045, 3047).
/// - The distance modifiers (`DistanceRamp`, `MeleeProximity`):
///   `arrow_critical_strike_chance_+%_max_as_distance_travelled_increases`,
///   `arrow_damage_+%_max_as_distance_travelled_increases`,
///   `deadeye_projectile_damage_+%_final_max_as_distance_travelled_increases`,
///   `impale_effect_+%_max_as_distance_travelled_increases`,
///   `projectile_damage_+%_max_as_distance_travelled_increases`,
///   `projectile_damage_+%_max_before_distance_increase`,
///   `slayer_damage_+%_final_from_distance` (`ModParser.lua` 2511, 2662,
///   3013, 3917, 4638 to 4640); `keystone_point_blank`, `player_far_shot`,
///   `gain_player_far_shot_while_do_not_have_iron_reflexes`,
///   `local_display_socketed_gems_supported_by_level_x_point_blank` (the
///   `PointBlank` and `FarShot` flags, 2317, 2661, 5489; the last through
///   the Point Blank support, `sup_dex.lua`); and the skill
///   stats `lancing_steel_damage_+%_at_close_range`,
///   `shattering_steel_hit_damage_+%_final_scaled_by_projectile_distance_per_ammo_consumed`,
///   `shield_crush_damage_+%_final_from_distance`,
///   `support_slashing_damage_+%_final_from_distance` (the skills' stat
///   maps, `Data/Skills/act_dex.lua`, `act_str.lua`, `sup_dex.lua`).
///   Freezing Pulse and Eye of Winter add theirs in their own skill code
///   (`act_int.lua`), outside any stat: those skills are not modelled.
/// - `physical_damage_%_to_add_as_lightning_damage_per_hallowing_flame_consumed_by_ally_up_to_80%`:
///   reads `Multiplier:HallowingFlameStacksRemovedByAlly` (`ModParser.lua`
///   2898).
///
/// Each is a stat of the real bundle (a maintainer-run test,
/// `tests/enemy.rs`), so a misspelt ID cannot pass as `Unmapped`.
pub const DEPENDS_ON: &[&str] = &[
    "additional_spell_block_%_if_havent_blocked_recently",
    "arrow_critical_strike_chance_+%_max_as_distance_travelled_increases",
    "arrow_damage_+%_max_as_distance_travelled_increases",
    "can_apply_additional_scorch",
    "chance_to_block_attack_damage_if_not_blocked_recently_%",
    "consecrated_ground_enemy_damage_taken_+%",
    "deadeye_projectile_damage_+%_final_max_as_distance_travelled_increases",
    "enemies_chilled_by_hits_take_damage_increased_by_chill_effect",
    "gain_convergence_on_hitting_unique_enemy",
    "gain_player_far_shot_while_do_not_have_iron_reflexes",
    "impale_effect_+%_max_as_distance_travelled_increases",
    "keystone_cold_purist",
    "keystone_point_blank",
    "lancing_steel_damage_+%_at_close_range",
    "local_display_socketed_gems_supported_by_level_x_point_blank",
    "max_chance_to_block_attacks_if_not_blocked_recently",
    "physical_damage_%_to_add_as_lightning_damage_per_hallowing_flame_consumed_by_ally_up_to_80%",
    "player_far_shot",
    "projectile_damage_+%_max_as_distance_travelled_increases",
    "projectile_damage_+%_max_before_distance_increase",
    "raider_shocks_you_apply_can_stack_up_to_50_times",
    "shattering_steel_hit_damage_+%_final_scaled_by_projectile_distance_per_ammo_consumed",
    "shield_crush_damage_+%_final_from_distance",
    "slayer_damage_+%_final_from_distance",
    "spell_block_is_maximum_if_not_blocked_recently",
    "support_chilling_areas_also_grant_cold_damage_taken_+%_equal_to_slow_amount",
    "support_chills_also_grant_cold_damage_taken_+%_equal_to_slow_amount",
    "support_slashing_damage_+%_final_from_distance",
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

/// Whether `input` is the enemy's damage ([`ENEMY_DAMAGE`]).
#[must_use]
pub fn enemy_damage(input: &str) -> bool {
    ENEMY_DAMAGE.binary_search(&input).is_ok()
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

    /// Fortification (P3-12e): every reader of its names is a modifier,
    /// and no mapped stat reads or changes it (the `NoEffect` rows that
    /// name it are listed and reach no slice field).
    #[test]
    fn no_mapped_stat_should_read_fortification() {
        use crate::ingest::StatMapping;
        use crate::poe1::mapping::{HIT_ROWS, LOCAL_ROWS, ROWS, WEAPON_ROWS};

        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        for id in ids {
            assert!(!id.contains("fortif"), "{id}: revisit NAMED_ONLY");
        }
        // `NoEffect` rows naming fortification, each checked: a chance to
        // fortify, its duration, regeneration per stack; none reaches a
        // slice field.
        let checked = [
            "chance_to_fortify_on_melee_hit_+%",
            "fortify_duration_+%",
            "life_regeneration_per_minute_%_per_fortification",
        ];
        for (id, _) in crate::poe1::mapping::NO_EFFECT {
            assert!(
                !id.contains("fortif") || checked.contains(id),
                "{id}: a NoEffect row naming fortification; check it and list it"
            );
        }
        // The maximum stays a reason (PoB scales stats per stack).
        assert_eq!(
            crate::poe1::mapping::Poe1Mapping
                .map(&loom_domain::StatId::new("base_max_fortification")),
            crate::ingest::StatMap::Unmapped
        );
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

    #[test]
    fn the_enemy_damage_should_be_sorted_and_apart_from_the_other_tables() {
        use super::super::enemy::{ENEMY_INPUTS, RESET_PLACEHOLDERS};

        assert!(ENEMY_DAMAGE.windows(2).all(|w| w[0] < w[1]));
        for input in ENEMY_DAMAGE {
            assert!(enemy_damage(input), "{input}");
            assert!(!named_only(input), "{input}");
            assert!(!ENEMY_INPUTS.contains(input), "{input}");
            assert!(!RESET_PLACEHOLDERS.contains(input), "{input}");
        }
        for input in RESET_PLACEHOLDERS {
            assert!(!named_only(input), "{input}");
            assert!(!enemy_damage(input), "{input}");
        }
        assert!(!enemy_damage("enemyIsBoss"));
        assert!(!enemy_damage("enemyArmour"));
    }
}

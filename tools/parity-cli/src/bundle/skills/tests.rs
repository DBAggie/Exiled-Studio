//! The skill converter on made-up extracts in each game's column shape.

use std::collections::BTreeMap;

use loom_data::bundle::skills::{Cost, Reservation, Resource};
use serde_json::{Value, json};

use super::{SkillSummary, convert};
use crate::bundle::Counts;

/// One table file of an extract.
fn table(name: &str, columns: &[&str], rows: &Value) -> (String, Vec<u8>) {
    let columns: Vec<Value> = columns
        .iter()
        .map(|c| json!({"name": c, "references": null}))
        .collect();
    let body = json!({"table": name, "columns": columns, "rows": rows});
    (format!("tables/{name}.json"), body.to_string().into_bytes())
}

/// The tables both shapes share.
fn shared() -> BTreeMap<String, Vec<u8>> {
    [
        table(
            "Stats",
            &["Id"],
            &json!([["s_a"], ["s_b"], ["s_c"], ["s_flag"]]),
        ),
        table(
            "ActiveSkillType",
            &["Id", "FlagStat"],
            &json!([["Spell", 3], ["Attack", null]]),
        ),
        table(
            "EffectivenessCostConstants",
            &["Id", "Multiplier"],
            &json!([["DamageValue", 1.0], ["PerMinute", 0.06]]),
        ),
        table("CostTypes", &["Id"], &json!([["Mana"], ["Life"]])),
        table("BaseItemTypes", &["Id"], &json!([["Gems/Bolt"]])),
    ]
    .into_iter()
    .collect()
}

/// A PoE1-shaped extract: cost types per level, mana and life
/// reservations, `DamageEffectiveness`, `GrantedEffect2`, `Tag`.
fn poe1() -> BTreeMap<String, Vec<u8>> {
    let mut files = shared();
    files.extend([
        table(
            "ActiveSkills",
            &[
                "Id",
                "DisplayedName",
                "ActiveSkillTypes",
                "MinionActiveSkillTypes",
                "WeaponRestriction_ItemClassesKeys",
                "IsManuallyCasted",
            ],
            &json!([["bolt", "Bolt", [0], [], [0], true]]),
        ),
        table(
            "GrantedEffects",
            &[
                "Id",
                "IsSupport",
                "AllowedActiveSkillTypes",
                "AddedActiveSkillTypes",
                "ExcludedActiveSkillTypes",
                "SupportsGemsOnly",
                "CannotBeSupported",
                "CastTime",
                "ActiveSkill",
                "AddedMinionActiveSkillTypes",
                "StatSet1",
                "StatSet2",
            ],
            &json!([["Bolt", false, [], [], [], false, false, 750, 0, [], 0, []]]),
        ),
        table(
            "GrantedEffectsPerLevel",
            &[
                "GrantedEffect",
                "Level",
                "CostMultiplier",
                "StoredUses",
                "Cooldown",
                "AttackSpeedMultiplier",
                "AttackTime",
                "CostAmounts",
                "CostTypes",
                "ManaReservationFlat",
                "ManaReservationPercent",
                "LifeReservationFlat",
                "LifeReservationPercent",
            ],
            // Level 2 before level 1; a cost list longer than its types; a
            // row whose effect is past the table's end.
            &json!([
                [0, 2, 100, 0, 0, 0, 0, [6, 9], [0], 0, 2500, 0, 0],
                [0, 1, 100, 0, 0, 0, 0, [5], [0], 0, 0, 0, 0],
                [7, 1, 100, 0, 0, 0, 0, [], [], 0, 0, 0, 0]
            ]),
        ),
        table(
            "GrantedEffectQualityStats",
            &["GrantedEffectsKey", "StatsKeys", "StatsValuesPermille", ""],
            &json!([[0, [1], [500], []]]),
        ),
        table(
            "GrantedEffectStatSets",
            &[
                "Id",
                "ImplicitStats",
                "ConstantStats",
                "ConstantStatsValues",
                "BaseEffectiveness",
                "IncrementalEffectiveness",
            ],
            &json!([["Bolt", [3], [2], [25], 2.921, 0.0495]]),
        ),
        table(
            "GrantedEffectStatSetsPerLevel",
            &[
                "StatSet",
                "GemLevel",
                "SpellCritChance",
                "AttackCritChance",
                "BaseMultiplier",
                "DamageEffectiveness",
                "BaseResolvedValues",
                "AdditionalStatsValues",
                "GrantedEffects",
                "AdditionalFlags",
                "FloatStats",
                "InterpolationBases",
                "AdditionalStats",
                "StatInterpolations",
                "FloatStatsValues",
            ],
            &json!([[
                0,
                1,
                500,
                0,
                0,
                20000,
                [1],
                [10],
                [],
                [3],
                [0],
                [0],
                [2],
                [3, 1],
                [0.8]
            ]]),
        ),
        table(
            "SkillGems",
            &[
                "BaseItemTypesKey",
                "StrengthRequirementPercent",
                "DexterityRequirementPercent",
                "IntelligenceRequirementPercent",
                "IsVaalVariant",
                "GemVariants",
            ],
            // The second gem's base is past the table's end.
            &json!([[0, 0, 0, 100, false, [0]], [9, 0, 0, 0, false, []]]),
        ),
        table(
            "GemEffects",
            &["Id", "Name", "GrantedEffect", "GrantedEffect2", "GemTags"],
            &json!([["Bolt", "Bolt", 0, null, [0]]]),
        ),
        table("GemTags", &["Id", "Tag"], &json!([["spell", "Spell"]])),
    ]);
    files
}

/// A PoE2-shaped extract: cost types per effect, `Reservation` (spirit),
/// `DamageIncrementalEffectiveness`, `AdditionalGrantedEffects`, `Name`.
fn poe2() -> BTreeMap<String, Vec<u8>> {
    let mut files = shared();
    files.extend([
        table(
            "ActiveSkills",
            &[
                "Id",
                "DisplayedName",
                "ActiveSkillTypes",
                "MinionActiveSkillTypes",
                "IsManuallyCasted",
            ],
            &json!([["bolt", "Bolt", [0], [1], true]]),
        ),
        table(
            "GrantedEffects",
            &[
                "Id",
                "IsSupport",
                "AllowedActiveSkillTypes",
                "AddedActiveSkillTypes",
                "ExcludedActiveSkillTypes",
                "SupportsGemsOnly",
                "CannotBeSupported",
                "CastTime",
                "ActiveSkill",
                "AddedMinionActiveSkillTypes",
                "StatSet",
                "AdditionalStatSets",
                "CostTypes",
            ],
            &json!([
                [
                    "Bolt",
                    false,
                    [],
                    [],
                    [],
                    false,
                    false,
                    1000,
                    0,
                    [],
                    0,
                    [],
                    [1]
                ],
                ["Aura", false, [], [], [], false, true, 0, 0, [], 0, [], []]
            ]),
        ),
        table(
            "GrantedEffectsPerLevel",
            &[
                "GrantedEffect",
                "Level",
                "CostMultiplier",
                "StoredUses",
                "Cooldown",
                "AttackSpeedMultiplier",
                "AttackTime",
                "Reservation",
                "CostAmounts",
            ],
            &json!([
                [0, 1, 100, 0, 0, 0, 0, 0, [8]],
                [1, 1, 100, 0, 0, 0, 0, 30, []]
            ]),
        ),
        table(
            "GrantedEffectQualityStats",
            &["GrantedEffect", "Stats", "StatsValuesPermille"],
            &json!([[0, [1], [250]]]),
        ),
        table(
            "GrantedEffectStatSets",
            &[
                "Id",
                "ImplicitStats",
                "ConstantStats",
                "ConstantStatsValues",
                "BaseEffectiveness",
                "IncrementalEffectiveness",
                "DamageIncrementalEffectiveness",
            ],
            &json!([["Bolt", [], [], [], 1.0, 0.055, 0.01]]),
        ),
        table(
            "GrantedEffectStatSetsPerLevel",
            &[
                "StatSet",
                "GemLevel",
                "SpellCritChance",
                "AttackCritChance",
                "BaseMultiplier",
                "BaseResolvedValues",
                "AdditionalStatsValues",
                "GrantedEffects",
                "AdditionalFlags",
                "FloatStats",
                "InterpolationBases",
                "AdditionalStats",
                "StatInterpolations",
                "FloatStatsValues",
            ],
            &json!([[0, 1, 700, 0, 0, [], [], [1], [], [], [], [], [], []]]),
        ),
        table(
            "SkillGems",
            &[
                "BaseItemType",
                "IntelligenceRequirementPercent",
                "GemEffects",
            ],
            &json!([[0, 100, [0]]]),
        ),
        table(
            "GemEffects",
            &[
                "Id",
                "Name",
                "GrantedEffect",
                "GemTags",
                "AdditionalGrantedEffects",
            ],
            &json!([["Bolt", "Bolt", 0, [0], [1]]]),
        ),
        table("GemTags", &["Id", "Name"], &json!([["spell", "Spell"]])),
    ]);
    files
}

fn run(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(loom_data::bundle::skills::SkillTables, SkillSummary, usize), String> {
    let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: none"));
    let counts = Counts::default();
    let (tables, summary) = convert(&read, &["Wand".to_owned()], &counts)?;
    Ok((tables, summary, counts.dangling.get()))
}

#[test]
fn poe1_shape_should_convert_with_its_columns() {
    let (t, s, dangling) = run(&poe1()).unwrap();
    let effect = &t.granted_effects[0];
    let set = &t.stat_sets[0];
    let level = &set.levels[0];
    assert_eq!(
        (
            t.skill_types[0].flag_stat.as_deref(),
            t.active_skills[0].weapon_classes.clone(),
            effect.levels.iter().map(|l| l.level).collect::<Vec<_>>(),
            effect.levels[1].costs.clone(),
            effect.levels[1].reservations.clone(),
            effect.quality[0].stat.as_str(),
            effect.stat_sets.clone(),
        ),
        (
            Some("s_flag"),
            vec!["Wand".to_owned()],
            vec![1, 2],
            vec![Cost {
                resource: "Mana".to_owned(),
                amount: 6
            }],
            vec![Reservation {
                resource: Resource::Mana,
                percent: true,
                amount: 2500
            }],
            "s_b",
            vec!["Bolt".to_owned()],
        )
    );
    assert_eq!(
        (
            set.base_effectiveness,
            set.damage_incremental_effectiveness,
            set.constant_stats.clone(),
            level.damage_effectiveness,
            (
                level.float_stats[0].stat.as_str(),
                level.float_stats[0].value,
                level.float_stats[0].resolved,
                level.float_stats[0].interpolation,
                level.float_stats[0].interpolation_base,
            ),
            (
                level.stats[0].stat.as_str(),
                level.stats[0].value,
                level.stats[0].interpolation
            ),
            level.flags.clone(),
        ),
        (
            2.921,
            None,
            vec![("s_c".to_owned(), 25)],
            Some(20000),
            ("s_a", 0.8, 1, 3, 1.0),
            ("s_c", 10, Some(1)),
            vec!["s_flag".to_owned()],
        )
    );
    assert_eq!(
        (
            t.gems.len(),
            t.gems[0].intelligence,
            t.gem_tags[0].name.as_str()
        ),
        (1, 100, "Spell")
    );
    assert_eq!(
        (
            s.effect_levels_sorted,
            s.parallel_lists_unequal,
            s.rows_without_owner,
            dangling
        ),
        (1, 1, 1, 1),
        "{s:?}"
    );
}

#[test]
fn poe2_shape_should_convert_with_its_columns() {
    let (t, s, dangling) = run(&poe2()).unwrap();
    assert_eq!(
        (
            t.active_skills[0].weapon_classes.len(),
            t.active_skills[0].minion_types.clone(),
            t.granted_effects[0].levels[0].costs.clone(),
            t.granted_effects[1].levels[0].reservations.clone(),
            t.granted_effects[1].cannot_be_supported,
            t.granted_effects[0].quality[0].per_mille,
        ),
        (
            0,
            vec!["Attack".to_owned()],
            vec![Cost {
                resource: "Life".to_owned(),
                amount: 8
            }],
            vec![Reservation {
                resource: Resource::Spirit,
                percent: false,
                amount: 30
            }],
            true,
            250,
        )
    );
    let set = &t.stat_sets[0];
    assert_eq!(
        (
            set.damage_incremental_effectiveness,
            set.levels[0].damage_effectiveness,
            set.levels[0].spell_crit_chance,
            set.levels[0].granted_effects.clone(),
            t.gem_effects[0].additional_granted_effects.clone(),
            t.gem_tags[0].name.as_str(),
        ),
        (
            Some(0.01),
            None,
            700,
            vec!["Aura".to_owned()],
            vec!["Aura".to_owned()],
            "Spell"
        )
    );
    assert_eq!(
        (
            s.effect_levels_sorted,
            s.parallel_lists_unequal,
            s.rows_without_owner,
            dangling
        ),
        (0, 0, 0, 0),
        "{s:?}"
    );
}

#[test]
fn non_finite_float_should_fail_the_conversion() {
    let mut files = poe1();
    files.insert(
        "tables/GrantedEffectStatSets.json".to_owned(),
        table(
            "GrantedEffectStatSets",
            &[
                "Id",
                "ImplicitStats",
                "ConstantStats",
                "ConstantStatsValues",
                "BaseEffectiveness",
                "IncrementalEffectiveness",
            ],
            &json!([["Bolt", [], [], [], "NaN", 0.0]]),
        )
        .1,
    );
    assert!(run(&files).is_err());
}

//! The main skill's hit as a self-cast spell (Phase 3 plan, step 9b):
//! base damage, added damage times effectiveness, increased and more
//! damage (PoB's more rounding), crit, cast rate, the enemy's resistance
//! and penetration, and which skills and gems the engine models.
//!
//! The data is made up (a nova with Ice Nova's shape under the game's stat
//! IDs); the expected values follow PoB v2.67.2's formulas (module docs of
//! `loom_calc::poe1::hit`). The mechanic fixtures check the same against
//! the oracle (`docs/phase-3/spell-hits.md`).

use loom_calc::poe1::hit::spell_skill;
use loom_calc::{
    ActiveSkill, CalcData, CalcInput, CalcOptions, CalcResult, CharacterBase, EffectLevel, Field,
    FieldState, GemEffect, GrantedEffect, PassiveLinks, SkillGem, Source, StatAmount, StatSet,
    StatSetLevel, StateKind, UnsupportedReason, calculate,
};
use loom_data::bundle::skills::{FloatStat, LevelStat};
use loom_domain::{
    Build, ClassId, ConfigInput, ConfigSet, ConfigSetId, ConfigValue, DataVersion, Game, Gem,
    GemId, Ordered, SkillId, SkillSet, SkillSetId, SocketGroup, Sourced,
};

// ---------------------------------------------------------------- data

fn version() -> DataVersion {
    DataVersion {
        game: Game::Poe1,
        schema_version: 2,
        steam_build: Some(1),
    }
}

fn level(n: i64, cooldown: i64) -> EffectLevel {
    EffectLevel {
        level: n,
        costs: Vec::new(),
        cost_multiplier: 100,
        cooldown,
        stored_uses: 0,
        attack_speed_multiplier: 0,
        attack_time: 0,
        reservations: Vec::new(),
    }
}

fn effect(id: &str, active: Option<&str>, support: bool, cooldown: i64) -> GrantedEffect {
    GrantedEffect {
        id: id.to_owned(),
        support,
        active_skill: active.map(str::to_owned),
        cast_time: if support { 0 } else { 500 },
        allowed_types: if support {
            vec!["Damage".to_owned()]
        } else {
            Vec::new()
        },
        added_types: Vec::new(),
        excluded_types: Vec::new(),
        added_minion_types: Vec::new(),
        supports_gems_only: false,
        cannot_be_supported: false,
        stat_sets: vec![id.to_owned()],
        levels: vec![level(1, cooldown)],
        quality: Vec::new(),
    }
}

fn float(stat: &str, resolved: i64) -> FloatStat {
    FloatStat {
        stat: stat.to_owned(),
        #[expect(clippy::cast_precision_loss, reason = "small test values")]
        value: resolved as f32,
        resolved,
        interpolation: 3,
        interpolation_base: 1.0,
    }
}

fn int(stat: &str, value: i64) -> LevelStat {
    LevelStat {
        stat: stat.to_owned(),
        value,
        interpolation: Some(1),
    }
}

fn stat_set(id: &str, float_stats: Vec<FloatStat>, stats: Vec<LevelStat>) -> StatSet {
    StatSet {
        id: id.to_owned(),
        implicit_stats: if id.starts_with("Support") {
            Vec::new()
        } else {
            vec!["is_area_damage".to_owned()]
        },
        constant_stats: Vec::new(),
        base_effectiveness: 1.0,
        incremental_effectiveness: 0.0,
        damage_incremental_effectiveness: None,
        levels: vec![StatSetLevel {
            level: 1,
            // 5.00% base crit; damage effectiveness 150%.
            spell_crit_chance: 500,
            attack_crit_chance: 0,
            base_multiplier: 0,
            damage_effectiveness: Some(5000),
            float_stats,
            stats,
            flags: Vec::new(),
            granted_effects: Vec::new(),
        }],
    }
}

fn active(id: &str, types: &[&str]) -> ActiveSkill {
    ActiveSkill {
        id: id.to_owned(),
        name: id.to_owned(),
        types: types.iter().map(|t| (*t).to_owned()).collect(),
        minion_types: Vec::new(),
        weapon_classes: Vec::new(),
        manually_cast: true,
    }
}

fn gem_row(id: &str) -> (SkillGem, GemEffect) {
    (
        SkillGem {
            base_item: format!("Test/Gems/{id}"),
            strength: 0,
            dexterity: 0,
            intelligence: 100,
            vaal: false,
            effects: vec![id.to_owned()],
        },
        GemEffect {
            id: id.to_owned(),
            name: id.to_owned(),
            granted_effect: Some(id.to_owned()),
            additional_granted_effects: Vec::new(),
            tags: Vec::new(),
        },
    )
}

/// The Witch, the constants the hit reads, and these skills:
/// - `IceNova`: a cold area spell, 100 to 200 base damage, 0.5 s cast;
/// - `TestCooldownNova`: the same with a cooldown;
/// - `TestStrike`: an attack;
/// - `SupportTestAdded`: adds 10 to 20 cold damage, 50% more spell damage;
/// - `SupportTestLessCrit`: 80% less critical strike chance (PoB rounds
///   the more factor 0.19999999999999996 to 0.2);
/// - `SupportTestUnknown`: a stat the mapping does not know;
/// - `TestSpark`: `IceNova`'s data under an ID the oracle has not checked;
/// - `TestPhysicalNova`: physical base damage;
/// - `SupportTestMore`: 25% more spell damage, 10% increased cold damage;
/// - `SupportTestPen`: 30% cold penetration.
struct Data {
    effects: Vec<GrantedEffect>,
    sets: Vec<StatSet>,
    actives: Vec<ActiveSkill>,
    gems: Vec<SkillGem>,
    gem_effects: Vec<GemEffect>,
    /// Whether the hit's constants (crit multiplier, the enemy's maximum
    /// resistance) are in the data.
    constants: bool,
}

impl Data {
    fn new() -> Self {
        let cold = || {
            vec![
                float("spell_minimum_base_cold_damage", 100),
                float("spell_maximum_base_cold_damage", 200),
            ]
        };
        let ids = [
            "IceNova",
            "TestCooldownNova",
            "TestStrike",
            "SupportTestAdded",
            "SupportTestLessCrit",
            "SupportTestUnknown",
            "TestSpark",
            "TestPhysicalNova",
            "SupportTestMore",
            "SupportTestPen",
        ];
        let (gems, gem_effects) = ids.iter().map(|id| gem_row(id)).unzip();
        Self {
            effects: vec![
                effect("IceNova", Some("nova"), false, 0),
                effect("TestCooldownNova", Some("nova"), false, 3000),
                effect("TestStrike", Some("strike"), false, 0),
                effect("SupportTestAdded", None, true, 0),
                effect("SupportTestLessCrit", None, true, 0),
                effect("SupportTestUnknown", None, true, 0),
                effect("TestSpark", Some("nova"), false, 0),
                effect("TestPhysicalNova", Some("nova"), false, 0),
                effect("SupportTestMore", None, true, 0),
                effect("SupportTestPen", None, true, 0),
            ],
            sets: vec![
                stat_set("IceNova", cold(), Vec::new()),
                stat_set("TestCooldownNova", cold(), Vec::new()),
                stat_set("TestStrike", Vec::new(), Vec::new()),
                stat_set(
                    "SupportTestAdded",
                    vec![
                        float("global_minimum_added_cold_damage", 10),
                        float("global_maximum_added_cold_damage", 20),
                    ],
                    vec![int(
                        "support_controlled_destruction_spell_damage_+%_final",
                        50,
                    )],
                ),
                stat_set(
                    "SupportTestLessCrit",
                    Vec::new(),
                    vec![int(
                        "support_controlled_destruction_critical_strike_chance_+%_final",
                        -80,
                    )],
                ),
                stat_set(
                    "SupportTestUnknown",
                    Vec::new(),
                    vec![int("a_stat_the_mapping_does_not_know", 1)],
                ),
                stat_set("TestSpark", cold(), Vec::new()),
                stat_set(
                    "TestPhysicalNova",
                    vec![
                        float("spell_minimum_base_physical_damage", 100),
                        float("spell_maximum_base_physical_damage", 200),
                    ],
                    Vec::new(),
                ),
                // 25% more spell damage and 20% more cold damage: two mod
                // names, each rounded alone (PoB's rule).
                stat_set(
                    "SupportTestMore",
                    Vec::new(),
                    vec![
                        int("support_controlled_destruction_spell_damage_+%_final", 25),
                        int("cold_damage_+%", 10),
                    ],
                ),
                stat_set(
                    "SupportTestPen",
                    Vec::new(),
                    vec![int("base_reduce_enemy_cold_resistance_%", 30)],
                ),
            ],
            actives: vec![
                active("nova", &["Spell", "Damage", "Area", "Cold"]),
                active("strike", &["Attack", "Damage", "Melee"]),
            ],
            gems,
            gem_effects,
            constants: true,
        }
    }
}

impl CalcData for Data {
    fn data_version(&self) -> DataVersion {
        version()
    }
    fn character(&self, class: &str) -> Option<CharacterBase> {
        (class == "Witch").then_some(CharacterBase {
            base_life: 38,
            base_mana: 34,
            strength: 14,
            dexterity: 14,
            intelligence: 32,
        })
    }
    fn character_constant(&self, stat: &str) -> Option<i64> {
        match stat {
            "life_per_level" => Some(12),
            "mana_per_level" => Some(6),
            "base_maximum_all_resistances_%" | "maximum_block_%" | "base_maximum_spell_block_%" => {
                Some(75)
            }
            "base_evasion_rating" => Some(15),
            "base_critical_strike_multiplier" if self.constants => Some(150),
            _ => None,
        }
    }
    fn monster_constant(&self, stat: &str) -> Option<i64> {
        (self.constants && stat == "base_maximum_all_resistances_%").then_some(75)
    }
    fn item_base(&self, _: &str) -> Option<loom_calc::ItemBase> {
        None
    }
    fn passive_stats(&self, _: u32) -> Option<&[StatAmount]> {
        None
    }
    fn mastery_effect_stats(&self, _: u32) -> Option<&[StatAmount]> {
        None
    }
    fn passive_links(&self, _: u32) -> Option<PassiveLinks> {
        None
    }
    fn skill_gem(&self, base_item: &str) -> Option<&SkillGem> {
        self.gems.iter().find(|g| g.base_item == base_item)
    }
    fn gem_effect(&self, id: &str) -> Option<&GemEffect> {
        self.gem_effects.iter().find(|e| e.id == id)
    }
    fn granted_effect(&self, id: &str) -> Option<&GrantedEffect> {
        self.effects.iter().find(|e| e.id == id)
    }
    fn active_skill(&self, id: &str) -> Option<&ActiveSkill> {
        self.actives.iter().find(|a| a.id == id)
    }
    fn stat_set(&self, id: &str) -> Option<&StatSet> {
        self.sets.iter().find(|s| s.id == id)
    }
    fn monster_level(&self, _: i64) -> Option<loom_calc::MonsterStats> {
        None
    }
    fn monster_type(&self, _: &str) -> Option<loom_calc::MonsterDefences> {
        None
    }
    fn game_constant(&self, _: &str) -> Option<(i64, i64)> {
        None
    }
}

// ---------------------------------------------------------------- builds

fn gem(id: &str) -> Gem {
    Gem {
        gem: Some(GemId::new(format!("Test/Gems/{id}"))),
        skill: Some(SkillId::new(id)),
        name: None,
        level: Sourced::new(1),
        quality: Sourced::new(0),
        enabled: true,
        attributes: Vec::new(),
    }
}

fn group(ids: &[&str]) -> SocketGroup {
    let mut g = SocketGroup {
        label: None,
        enabled: true,
        slot: None,
        main_active_skill: None,
        gems: Ordered::new(),
        attributes: Vec::new(),
    };
    for id in ids {
        g.gems.push(gem(id));
    }
    g
}

/// A level 90 Witch with these groups, the first the main one.
fn build(groups: Vec<SocketGroup>) -> Build {
    let mut b = Build::new(Game::Poe1);
    b.data = Some(version());
    b.character.class = Some(ClassId::new("Witch"));
    b.character.level = Some(Sourced::new(90));
    b.character.main_socket_group = Some(Sourced::new(1));
    let mut set = SkillSet {
        id: SkillSetId(1),
        title: None,
        groups: Ordered::new(),
        attributes: Vec::new(),
    };
    for g in groups {
        set.groups.push(g);
    }
    b.skills.sets.push(set);
    b
}

fn with_config(mut b: Build, inputs: &[(&str, f64)]) -> Build {
    let mut set = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        attributes: Vec::new(),
    };
    for (name, v) in inputs {
        set.inputs.push(ConfigInput {
            name: (*name).to_owned(),
            value: ConfigValue::Number(Sourced::new(loom_domain::Number(*v))),
            attributes: Vec::new(),
        });
    }
    b.config.sets.push(set);
    b
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn run(b: &Build) -> CalcResult {
    let data = Data::new();
    let options = CalcOptions::default();
    let input = CalcInput {
        build: b,
        data: &data,
        options: &options,
    };
    let r = calculate(&input).unwrap();
    r.graph.check().unwrap();
    r
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn value(r: &CalcResult, field: Field) -> f64 {
    r.fields[&field].value.unwrap()
}

const HIT: [Field; 5] = [
    Field::AverageDamage,
    Field::Speed,
    Field::CritChance,
    Field::CritMultiplier,
    Field::HitDps,
];

/// PoB's average hit of a min-max hit against a resistance, the crit
/// chance and multiplier given (each pass `min / 2 + max / 2`, times `1 -
/// resist / 100`).
fn average(min: f64, max: f64, crit: f64, multiplier: f64, resist: f64) -> f64 {
    let effect = 1.0 - resist / 100.0;
    let hit = (min / 2.0 + max / 2.0) * effect;
    let crit_hit = ((min * multiplier) / 2.0 + (max * multiplier) / 2.0) * effect;
    let average_hit = hit * (1.0 - crit / 100.0) + crit_hit * crit / 100.0;
    average_hit * 100.0 / 100.0
}

// ---------------------------------------------------------------- tests

#[test]
fn a_self_cast_spell_alone_should_hit_the_default_enemy() {
    let r = run(&build(vec![group(&["IceNova"])]));
    for f in HIT {
        assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
    }
    // Crit 5% (hundredths / 100, times the hit chance / 100); multiplier
    // 1 + round(50 / 100, 2).
    assert_eq!(value(&r, Field::CritChance), 5.0);
    assert_eq!(value(&r, Field::CritMultiplier), 1.5);
    // The Pinnacle boss's 50% cold resistance.
    let expected = average(100.0, 200.0, 5.0, 1.5, 50.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
    assert_eq!(value(&r, Field::AverageDamage), 76.875);
    // 1 / (0.5 / round(1 x 1, 2)).
    assert_eq!(value(&r, Field::Speed), 2.0);
    assert_eq!(value(&r, Field::HitDps), 76.875 * 2.0);
    // The modelled gem leaves the character's fields complete.
    assert_eq!(r.fields[&Field::Life].state, FieldState::Complete);
    // Neither hit chance nor hit rate is a spell's.
    for f in [Field::HitChance, Field::HitSpeed] {
        assert_eq!(r.fields[&f].state.kind(), StateKind::Unsupported, "{f}");
    }
}

#[test]
fn a_support_should_add_damage_scaled_by_effectiveness_and_more_damage() {
    let r = run(&build(vec![group(&["IceNova", "SupportTestAdded"])]));
    assert_eq!(r.fields[&Field::AverageDamage].state, FieldState::Complete);
    // 100 + 10 x 1.5 = 115, 200 + 20 x 1.5 = 230; 50% more:
    // round(172.5) = 173, round(345) = 345.
    let expected = average(173.0, 345.0, 5.0, 1.5, 50.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
}

#[test]
fn a_more_factor_should_be_rounded_to_two_decimals_as_pob_does() {
    let r = run(&build(vec![group(&["IceNova", "SupportTestLessCrit"])]));
    // 5 x round(1 - 0.8, 2) = 5 x 0.2 = 1, not 5 x 0.19999999999999996.
    assert_eq!(value(&r, Field::CritChance), 1.0);
    assert_ne!(
        5.0 * (1.0 + -80.0 / 100.0),
        1.0,
        "the unrounded factor differs"
    );
}

#[test]
fn a_configured_resistance_above_the_maximum_should_raise_it() {
    let r = run(&with_config(
        build(vec![group(&["IceNova"])]),
        &[("enemyColdResist", 80.0)],
    ));
    // `min(max(80, 75), 90)` = 80: the full 80% applies.
    let expected = average(100.0, 200.0, 5.0, 1.5, 80.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
    assert_eq!(r.fields[&Field::AverageDamage].state, FieldState::Complete);
    // A configured resistance above 90 stops at 90.
    let r = run(&with_config(
        build(vec![group(&["IceNova"])]),
        &[("enemyColdResist", 95.0)],
    ));
    let expected = average(100.0, 200.0, 5.0, 1.5, 90.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
}

#[test]
fn only_a_self_cast_spell_without_cooldown_should_be_computed() {
    for main in ["TestStrike", "TestCooldownNova"] {
        let b = build(vec![group(&[main])]);
        assert!(spell_skill(&b, &Data::new()).is_none(), "{main}");
        let r = run(&b);
        for f in HIT {
            assert_eq!(
                r.fields[&f].state,
                FieldState::Unsupported(UnsupportedReason::NotComputed),
                "{main} {f}"
            );
        }
        // Its gem is not modelled: a reason on the character's fields.
        assert_eq!(r.fields[&Field::Life].state.kind(), StateKind::Partial);
    }
    // A disabled main group is not cast.
    let mut g = group(&["IceNova"]);
    g.enabled = false;
    assert!(spell_skill(&build(vec![g]), &Data::new()).is_none());
}

#[test]
fn other_gems_and_unmapped_gem_stats_should_be_reasons() {
    // Another group's gem: not modelled, on every field.
    let r = run(&build(vec![group(&["IceNova"]), group(&["TestStrike"])]));
    let reasons = |f: Field| match &r.fields[&f].state {
        FieldState::Partial(p) => p.iter().cloned().collect::<Vec<_>>(),
        other => panic!("{f}: {other:?}"),
    };
    let other = UnsupportedReason::UnmodelledGem {
        source: Source::Gem { group: 1, gem: 0 },
    };
    assert_eq!(reasons(Field::Life), std::slice::from_ref(&other));
    assert_eq!(reasons(Field::AverageDamage), [other]);
    // A support stat the mapping does not know: unmapped, on every field
    // (it might be a buff).
    let r = run(&build(vec![group(&["IceNova", "SupportTestUnknown"])]));
    for f in [Field::Life, Field::AverageDamage] {
        assert!(
            matches!(&r.fields[&f].state, FieldState::Partial(p)
                if p.iter().any(|x| x.name() == "unmapped-stat")),
            "{f}"
        );
    }
}

#[test]
fn a_spell_the_oracle_has_not_checked_should_be_partial() {
    let r = run(&build(vec![group(&["TestSpark"])]));
    for f in HIT {
        assert!(
            matches!(&r.fields[&f].state, FieldState::Partial(p)
            if p.contains(&UnsupportedReason::UnmodelledSkill {
                source: Source::Gem { group: 0, gem: 0 },
                what: "pob-skill-data",
            })),
            "{f}"
        );
    }
    // Its value is the vetted spell's (same data).
    let vetted = run(&build(vec![group(&["IceNova"])]));
    assert_eq!(
        value(&r, Field::AverageDamage),
        value(&vetted, Field::AverageDamage)
    );
}

#[test]
fn base_damage_should_be_the_games_resolved_value() {
    // The stored float (0.4) is not what PoB's recomputation would round;
    // the game's resolved value (here 1, as the game keeps Added Lightning
    // Damage's level 1 minimum where PoB gives 0) is used.
    let mut data = Data::new();
    let set = data.sets.iter_mut().find(|s| s.id == "IceNova").unwrap();
    set.levels[0].float_stats[0].value = 0.4;
    set.levels[0].float_stats[0].resolved = 1;
    let b = build(vec![group(&["IceNova"])]);
    let options = CalcOptions::default();
    let r = calculate(&CalcInput {
        build: &b,
        data: &data,
        options: &options,
    })
    .unwrap();
    let expected = average(1.0, 200.0, 5.0, 1.5, 50.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
}

#[test]
fn physical_hit_damage_should_read_the_enemys_armour() {
    let r = run(&build(vec![group(&["TestPhysicalNova"])]));
    // The enemy's armour is modelled (P3-10): no armour reason. This data
    // has no monster tables and no reduction cap, so the enemy's armour
    // rows and the cap are missing data, which count with physical damage.
    let FieldState::Indeterminate { reasons, partial } = &r.fields[&Field::AverageDamage].state
    else {
        panic!("not indeterminate");
    };
    assert!(
        !partial
            .iter()
            .any(|x| format!("{x:?}").contains("physical"))
    );
    assert!(reasons.iter().all(|x| x.name() == "missing-data"));
    assert!(
        reasons
            .iter()
            .any(|x| format!("{x:?}").contains("maximum_physical_damage_reduction_%"))
    );
    // Without armour rows the enemy's armour is 0: the hit itself.
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 5.0, 1.5, 0.0).to_bits()
    );
    // The rate does not read the enemy: only the unvetted spell's reason.
    assert!(
        matches!(&r.fields[&Field::Speed].state, FieldState::Partial(p)
        if p.iter().all(|x| x.name() == "unmodelled-skill" && !format!("{x:?}").contains("physical")))
    );
}

#[test]
fn penetration_should_lower_the_enemys_resistance() {
    let r = run(&build(vec![group(&["IceNova", "SupportTestPen"])]));
    // 50 - 30 = 20.
    let expected = average(100.0, 200.0, 5.0, 1.5, 20.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
}

#[test]
fn more_and_increased_damage_should_combine_as_pob_does() {
    let r = run(&build(vec![group(&["IceNova", "SupportTestMore"])]));
    // round(100 x 1.1 x 1.25) = round(137.5) = 138, round(275) = 275.
    let expected = average(138.0, 275.0, 5.0, 1.5, 50.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected.to_bits()
    );
}

#[test]
fn enemy_inputs_of_another_type_and_missing_constants_should_be_reasons() {
    let mut b = build(vec![group(&["IceNova"])]);
    let mut set = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        attributes: Vec::new(),
    };
    set.inputs.push(ConfigInput {
        name: "enemyColdResist".to_owned(),
        value: ConfigValue::Boolean(true),
        attributes: Vec::new(),
    });
    b.config.sets.push(set);
    let r = run(&b);
    assert!(
        matches!(&r.fields[&Field::AverageDamage].state, FieldState::Partial(p)
        if p.contains(&UnsupportedReason::UnmodelledConfig { input: "enemyColdResist".to_owned() }))
    );

    // Data without the crit multiplier constant or the enemy's maximum
    // resistance: indeterminate, never a silent value.
    let mut data = Data::new();
    data.constants = false;
    let b = build(vec![group(&["IceNova"])]);
    let options = CalcOptions::default();
    let r = calculate(&CalcInput {
        build: &b,
        data: &data,
        options: &options,
    })
    .unwrap();
    for f in [Field::CritMultiplier, Field::AverageDamage, Field::HitDps] {
        assert_eq!(r.fields[&f].state.kind(), StateKind::Indeterminate, "{f}");
    }
    assert_eq!(r.fields[&Field::Speed].state, FieldState::Complete);
}

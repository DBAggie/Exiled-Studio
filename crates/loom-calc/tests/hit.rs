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
        let ids = ids.into_iter().chain(OTHER_GEMS);
        let (gems, gem_effects) = ids.map(gem_row).unzip();
        let mut sets = Vec::new();
        let mut effects = other_gems(&mut sets);
        let mut s = Self {
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
                active("aura", &["Spell", "Aura", "Buff", "HasReservation"]),
                active("slam", &["Attack", "Damage", "Area", "Melee"]),
            ],
            gems,
            gem_effects,
            constants: true,
        };
        s.effects.append(&mut effects);
        s.sets.append(&mut sets);
        s
    }
}

/// The other gems' made-up rows (P3-12h), under the IDs `poe1::gems`
/// models:
/// - `Precision`: an aura, 100 accuracy, 20% increased crit chance,
///   reserving 30 mana;
/// - `Haste`: an aura, 10% increased attack and cast speed, reserving
///   25.5% of mana;
/// - `Discipline`: an aura reserving spirit (PoE2's pool, a reason);
/// - `GroundSlam`: an inert attack;
/// - `SupportFasterAttacks`: a support of every skill, cost multiplier
///   150, modelled;
/// - `SupportTestMore` (above) supports a damaging skill: not modelled.
const OTHER_GEMS: [&str; 5] = [
    "Precision",
    "Haste",
    "Discipline",
    "GroundSlam",
    "SupportFasterAttacks",
];

fn other_gems(sets: &mut Vec<StatSet>) -> Vec<GrantedEffect> {
    use loom_data::bundle::skills::{Reservation, Resource};
    let reserving = |id: &str, r: Reservation| {
        let mut e = effect(id, Some("aura"), false, 0);
        e.levels[0].reservations = vec![r];
        e
    };
    let mana = |percent, amount| Reservation {
        resource: Resource::Mana,
        percent,
        amount,
    };
    sets.push(stat_set(
        "Precision",
        Vec::new(),
        vec![
            int("accuracy_rating", 100),
            int("skill_buff_grants_critical_strike_chance_+%", 20),
        ],
    ));
    sets.push(stat_set(
        "Haste",
        Vec::new(),
        vec![
            int("attack_speed_+%_granted_from_skill", 10),
            int("cast_speed_+%_granted_from_skill", 10),
        ],
    ));
    sets.push(stat_set("Discipline", Vec::new(), Vec::new()));
    sets.push(stat_set("GroundSlam", Vec::new(), Vec::new()));
    sets.push(stat_set("SupportFasterAttacks", Vec::new(), Vec::new()));
    let mut support = effect("SupportFasterAttacks", None, true, 0);
    support.allowed_types = Vec::new();
    support.levels[0].cost_multiplier = 150;
    vec![
        reserving("Precision", mana(false, 30)),
        reserving("Haste", mana(true, 2550)),
        reserving(
            "Discipline",
            Reservation {
                resource: Resource::Spirit,
                percent: false,
                amount: 10,
            },
        ),
        effect("GroundSlam", Some("slam"), false, 0),
        support,
    ]
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
            "max_power_charges" | "max_frenzy_charges" | "max_endurance_charges" => Some(3),
            "critical_strike_chance_+%_per_power_charge" => Some(50),
            "base_attack_speed_+%_per_frenzy_charge"
            | "base_cast_speed_+%_per_frenzy_charge"
            | "object_inherent_damage_+%_final_per_frenzy_charge" => Some(4),
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
        custom_modifiers: Vec::new(),
        attributes: Vec::new(),
    };
    for (name, v) in inputs {
        set.inputs.push(ConfigInput {
            name: (*name).to_owned(),
            value: ConfigValue::Number(Sourced::new(loom_domain::Number(*v))),
            kind: loom_domain::ConfigInputKind::Input,
            lines: Vec::new(),
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
        custom_modifiers: Vec::new(),
        attributes: Vec::new(),
    };
    set.inputs.push(ConfigInput {
        name: "enemyColdResist".to_owned(),
        value: ConfigValue::Boolean(true),
        kind: loom_domain::ConfigInputKind::Input,
        lines: Vec::new(),
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

/// Power and frenzy charges (P3-12e, `poe1::charges`): with `use` set,
/// the maximum (3 here) or the override, else its placeholder; endurance
/// charges change no computed field.
#[test]
fn charges_should_act_as_pob_counts_them() {
    use loom_domain::ConfigInputKind;

    let with = |inputs: &[(&str, ConfigValue, ConfigInputKind)]| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: Vec::new(),
            attributes: Vec::new(),
        };
        for (name, value, kind) in inputs {
            set.inputs.push(ConfigInput {
                name: (*name).to_owned(),
                value: value.clone(),
                kind: *kind,
                lines: Vec::new(),
                attributes: Vec::new(),
            });
        }
        b.config.sets.push(set);
        run(&b)
    };
    let on = || ConfigValue::Boolean(true);
    let n = |v: f64| ConfigValue::Number(Sourced::new(loom_domain::Number(v)));
    let input = ConfigInputKind::Input;
    let placeholder = ConfigInputKind::Placeholder;
    let bare = run(&build(vec![group(&["IceNova"])]));

    // Three power charges: 5% x (1 + 150 / 100).
    let r = with(&[("usePowerCharges", on(), input)]);
    assert_eq!(r.fields[&Field::CritChance].state, FieldState::Complete);
    assert_eq!(value(&r, Field::CritChance), 12.5);
    // The override, else its placeholder; 0 is unset.
    for (inputs, crit) in [
        (vec![("overridePowerCharges", n(1.0), input)], 7.5),
        (vec![("overridePowerCharges", n(2.0), placeholder)], 10.0),
        (
            vec![
                ("overridePowerCharges", n(0.0), input),
                ("overridePowerCharges", n(1.0), placeholder),
            ],
            7.5,
        ),
    ] {
        let mut all = vec![("usePowerCharges", on(), input)];
        all.extend(inputs);
        assert_eq!(value(&with(&all), Field::CritChance), crit);
    }
    // Without `use`, an override and `false` do nothing.
    for inputs in [
        vec![("overridePowerCharges", n(2.0), input)],
        vec![("usePowerCharges", ConfigValue::Boolean(false), input)],
        vec![("useEnduranceCharges", on(), input)],
    ] {
        let r = with(&inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
            assert_eq!(value(&r, f), value(&bare, f), "{f}");
        }
    }
    // Three frenzy charges: 12% increased cast speed, 12% more damage.
    let r = with(&[("useFrenzyCharges", on(), input)]);
    assert_eq!(r.fields[&Field::Speed].state, FieldState::Complete);
    assert!((value(&r, Field::Speed) - 2.0 * 1.12).abs() < 1e-12);
    assert!(
        (value(&r, Field::AverageDamage) - value(&bare, Field::AverageDamage) * 1.12).abs() < 1e-9
    );
    // A `use` that is not a flag, or an override that is not a number, is
    // a reason under the `use` input.
    for inputs in [
        vec![("usePowerCharges", n(1.0), input)],
        vec![
            ("usePowerCharges", on(), input),
            (
                "overridePowerCharges",
                ConfigValue::String("x".to_owned()),
                input,
            ),
        ],
    ] {
        let r = with(&inputs);
        let FieldState::Partial(p) = &r.fields[&Field::CritChance].state else {
            panic!("not partial");
        };
        assert!(p.contains(&UnsupportedReason::UnmodelledConfig {
            input: "usePowerCharges".to_owned(),
        }));
        assert_eq!(value(&r, Field::CritChance), 5.0);
    }
    // Counts as PoB keeps them: not clamped to the maximum, fractional
    // kept, negative floored at the minimum (0), the last input counts.
    for (inputs, crit) in [
        (vec![("overridePowerCharges", n(5.0), input)], 5.0 * 3.5),
        (vec![("overridePowerCharges", n(1.5), input)], 5.0 * 1.75),
        (vec![("overridePowerCharges", n(-2.0), input)], 5.0),
        (
            vec![
                ("overridePowerCharges", n(2.0), input),
                ("overridePowerCharges", n(1.0), input),
            ],
            7.5,
        ),
    ] {
        let mut all = vec![("usePowerCharges", on(), input)];
        all.extend(inputs);
        let r = with(&all);
        assert_eq!(r.fields[&Field::CritChance].state, FieldState::Complete);
        assert!(
            (value(&r, Field::CritChance) - crit).abs() < 1e-12,
            "{crit}"
        );
    }
    // A placeholder named like a `use` input does not turn it on (PoB
    // applies placeholders to count options only).
    let r = with(&[("usePowerCharges", on(), placeholder)]);
    assert_eq!(value(&r, Field::CritChance), 5.0);
    assert_eq!(r.fields[&Field::CritChance].state, FieldState::Complete);
    // A non-finite override, a `use` given as text, and an override
    // given as a flag are reasons, never a silent count.
    for inputs in [
        vec![
            ("usePowerCharges", on(), input),
            ("overridePowerCharges", n(f64::NAN), input),
        ],
        vec![
            ("usePowerCharges", on(), input),
            ("overridePowerCharges", n(f64::INFINITY), input),
        ],
        vec![(
            "usePowerCharges",
            ConfigValue::String("true".to_owned()),
            input,
        )],
        vec![
            ("usePowerCharges", on(), input),
            ("overridePowerCharges", ConfigValue::Boolean(false), input),
        ],
    ] {
        let r = with(&inputs);
        let FieldState::Partial(p) = &r.fields[&Field::CritChance].state else {
            panic!("not partial");
        };
        assert!(p.contains(&UnsupportedReason::UnmodelledConfig {
            input: "usePowerCharges".to_owned(),
        }));
    }
}

/// The enemy's damage taken from the config (P3-12f,
/// `poe1::damage_taken`): shock, the Uber boss kind, delirium. Each hit's
/// enemy effect is `(1 + inc / 100) x more x (1 - resist / 100)`.
#[test]
fn the_enemys_damage_taken_should_scale_each_hit_as_pob_does() {
    use loom_domain::ConfigInputKind;

    let with = |inputs: &[(&str, ConfigValue, ConfigInputKind)]| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: Vec::new(),
            attributes: Vec::new(),
        };
        for (name, value, kind) in inputs {
            set.inputs.push(ConfigInput {
                name: (*name).to_owned(),
                value: value.clone(),
                kind: *kind,
                lines: Vec::new(),
                attributes: Vec::new(),
            });
        }
        b.config.sets.push(set);
        run(&b)
    };
    let on = || ConfigValue::Boolean(true);
    let n = |v: f64| ConfigValue::Number(Sourced::new(loom_domain::Number(v)));
    let text = |v: &str| ConfigValue::String(v.to_owned());
    let input = ConfigInputKind::Input;
    let placeholder = ConfigInputKind::Placeholder;
    let bare = run(&build(vec![group(&["IceNova"])]));
    // PoB's order: `effMult = (1 + inc / 100) x more`, then `x (1 -
    // resist / 100)`; each pass's average times it.
    let expected = |taken: f64| {
        let effect = taken * (1.0 - 50.0 / 100.0);
        let hit = (100.0 / 2.0 + 200.0 / 2.0) * effect;
        let crit_hit = ((100.0 * 1.5) / 2.0 + (200.0 * 1.5) / 2.0) * effect;
        let average_hit = hit * (1.0 - 5.0 / 100.0) + crit_hit * 5.0 / 100.0;
        average_hit * 100.0 / 100.0
    };
    let shocked = |effect: ConfigValue, kind| {
        vec![
            ("conditionEnemyShocked", on(), input),
            ("conditionShockEffect", effect, kind),
        ]
    };
    // A shock: its effect, at most 50, floored; the input, else its
    // placeholder (0 is unset).
    for (inputs, taken) in [
        (shocked(n(20.0), input), 1.0 * (1.0 + 20.0 / 100.0)),
        (shocked(n(70.0), input), 1.0 * (1.0 + 50.0 / 100.0)),
        (shocked(n(20.7), input), 1.0 * (1.0 + 20.0 / 100.0)),
        (shocked(n(30.0), placeholder), 1.0 * (1.0 + 30.0 / 100.0)),
        (
            vec![
                ("conditionEnemyShocked", on(), input),
                ("conditionShockEffect", n(0.0), input),
                ("conditionShockEffect", n(30.0), placeholder),
            ],
            1.0 * (1.0 + 30.0 / 100.0),
        ),
        // The Uber boss: 70% less (one name, rounded to two decimals).
        (vec![("enemyIsBoss", text("Uber"), input)], 1.0 * 0.3),
        // Both: increased times more.
        (
            vec![
                ("conditionEnemyShocked", on(), input),
                ("conditionShockEffect", n(20.0), input),
                ("enemyIsBoss", text("Uber"), input),
            ],
            1.0 * (1.0 + 20.0 / 100.0) * 0.3,
        ),
        // Delirium; with the Uber boss, one name: round(0.3 x 0.2, 2).
        (
            vec![("deliriousPercentage", text("40Percent"), input)],
            1.0 * 0.68,
        ),
        (
            vec![
                ("enemyIsBoss", text("Uber"), input),
                ("deliriousPercentage", text("100Percent"), input),
            ],
            1.0 * 0.06,
        ),
    ] {
        let r = with(&inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f} {inputs:?}");
        }
        assert_eq!(
            value(&r, Field::AverageDamage).to_bits(),
            expected(taken).to_bits(),
            "{inputs:?}"
        );
        assert_eq!(
            value(&r, Field::CritChance),
            value(&bare, Field::CritChance)
        );
    }
    // No shock: not shocked, no effect, a negative effect; the Pinnacle
    // kind and delirium's "None" give nothing either.
    for inputs in [
        vec![("conditionShockEffect", n(20.0), input)],
        vec![("conditionEnemyShocked", on(), input)],
        shocked(n(-10.0), input),
        vec![
            ("conditionEnemyShocked", ConfigValue::Boolean(false), input),
            ("conditionShockEffect", n(20.0), input),
        ],
        // The default kind given (the others' resistances differ).
        vec![("enemyIsBoss", text("Pinnacle"), input)],
        vec![("deliriousPercentage", n(0.0), input)],
        // A check's placeholder is not applied (PoB applies placeholders
        // to count options only).
        vec![
            ("conditionEnemyShocked", on(), placeholder),
            ("conditionShockEffect", n(20.0), input),
        ],
    ] {
        let r = with(&inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f} {inputs:?}");
            assert_eq!(value(&r, f).to_bits(), value(&bare, f).to_bits(), "{f}");
        }
    }
    // Values PoB would read otherwise: a reason under their input.
    for (inputs, reason) in [
        (
            vec![("conditionEnemyShocked", n(1.0), input)],
            "conditionEnemyShocked",
        ),
        (shocked(text("20"), input), "conditionShockEffect"),
        (shocked(n(f64::NAN), input), "conditionShockEffect"),
        (
            vec![("deliriousPercentage", text("50Percent"), input)],
            "deliriousPercentage",
        ),
        (
            vec![("deliriousPercentage", on(), input)],
            "deliriousPercentage",
        ),
    ] {
        let r = with(&inputs);
        let FieldState::Partial(p) = &r.fields[&Field::AverageDamage].state else {
            panic!("{inputs:?}: not partial");
        };
        assert!(
            p.contains(&UnsupportedReason::UnmodelledConfig {
                input: reason.to_owned(),
            }),
            "{inputs:?}"
        );
    }
    // PoB's spelling rule (`ConfigTab.lua` 853 to 855): any case, and
    // the legacy names.
    for (kind, taken) in [
        ("uber", Some(0.3)),
        ("UBER", Some(0.3)),
        ("Shaper", None),
        ("sirus", None),
    ] {
        let r = with(&[("enemyIsBoss", text(kind), input)]);
        assert_eq!(
            r.fields[&Field::AverageDamage].state,
            FieldState::Complete,
            "{kind}"
        );
        let expect = taken.map_or(value(&bare, Field::AverageDamage), |t| expected(1.0 * t));
        assert_eq!(
            value(&r, Field::AverageDamage).to_bits(),
            expect.to_bits(),
            "{kind}"
        );
    }
    // The last input of a name counts (PoB's input table), for the boss
    // kind and the shock alike.
    let r = with(&[
        ("enemyIsBoss", text("None"), input),
        ("enemyIsBoss", text("Uber"), input),
    ]);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(1.0 * 0.3).to_bits()
    );
    let r = with(&[
        ("conditionEnemyShocked", on(), input),
        ("conditionShockEffect", n(30.0), input),
        ("conditionShockEffect", n(20.0), input),
    ]);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(1.0 * (1.0 + 20.0 / 100.0)).to_bits()
    );
    // A kind PoB does not know: the enemy model's reason on the hit, read
    // as the default kind (no damage taken).
    let r = with(&[("enemyIsBoss", text("Eramir"), input)]);
    let FieldState::Partial(p) = &r.fields[&Field::AverageDamage].state else {
        panic!("not partial");
    };
    assert!(p.contains(&UnsupportedReason::UnmodelledConfig {
        input: "enemyIsBoss".to_owned(),
    }));
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        value(&bare, Field::AverageDamage).to_bits()
    );
}

/// Intimidate, Unnerve, and Blind (P3-12g, `poe1::damage_taken`): an
/// unnerved enemy takes 10% increased spell damage, an intimidated one 10%
/// increased attack damage (none on a spell), and Blind changes no spell
/// field; a value other than a boolean is a reason.
#[test]
fn unnerve_should_raise_a_spells_damage_taken_and_intimidate_not() {
    let with = |inputs: &[(&str, ConfigValue)]| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: Vec::new(),
            attributes: Vec::new(),
        };
        for (name, value) in inputs {
            set.inputs.push(ConfigInput {
                name: (*name).to_owned(),
                value: value.clone(),
                kind: loom_domain::ConfigInputKind::Input,
                lines: Vec::new(),
                attributes: Vec::new(),
            });
        }
        b.config.sets.push(set);
        run(&b)
    };
    let on = || ConfigValue::Boolean(true);
    let bare = run(&build(vec![group(&["IceNova"])]));
    let expected = |taken: f64| {
        let effect = taken * (1.0 - 50.0 / 100.0);
        let hit = (100.0 / 2.0 + 200.0 / 2.0) * effect;
        let crit_hit = ((100.0 * 1.5) / 2.0 + (200.0 * 1.5) / 2.0) * effect;
        hit * (1.0 - 5.0 / 100.0) + crit_hit * 5.0 / 100.0
    };
    let r = with(&[("conditionEnemyUnnerved", on())]);
    assert_eq!(r.fields[&Field::AverageDamage].state, FieldState::Complete);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(1.0 + 10.0 / 100.0).to_bits()
    );
    // With a shock, one sum of increases: `1 + (10 + 20) / 100`.
    let r = with(&[
        ("conditionEnemyUnnerved", on()),
        ("conditionEnemyShocked", on()),
        (
            "conditionShockEffect",
            ConfigValue::Number(Sourced::new(loom_domain::Number(20.0))),
        ),
    ]);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(1.0 + 30.0 / 100.0).to_bits()
    );
    for inputs in [
        vec![("conditionEnemyIntimidated", on())],
        vec![("conditionEnemyBlinded", on())],
        vec![("conditionEnemyUnnerved", ConfigValue::Boolean(false))],
    ] {
        let r = with(&inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
            assert_eq!(value(&r, f).to_bits(), value(&bare, f).to_bits(), "{f}");
        }
    }
    for input in [
        "conditionEnemyUnnerved",
        "conditionEnemyIntimidated",
        "conditionEnemyBlinded",
    ] {
        let r = with(&[(input, ConfigValue::String("yes".to_owned()))]);
        let FieldState::Partial(p) = &r.fields[&Field::AverageDamage].state else {
            panic!("{input}: not partial");
        };
        assert!(p.contains(&UnsupportedReason::UnmodelledConfig {
            input: input.to_owned(),
        }));
    }
}

/// Onslaught (P3-12f, `poe1::buffs`): 20% increased attack and cast
/// speed; any value other than a boolean is a reason.
#[test]
fn onslaught_should_grant_increased_cast_speed() {
    let with = |value: ConfigValue| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: Vec::new(),
            attributes: Vec::new(),
        };
        set.inputs.push(ConfigInput {
            name: "buffOnslaught".to_owned(),
            value,
            kind: loom_domain::ConfigInputKind::Input,
            lines: Vec::new(),
            attributes: Vec::new(),
        });
        b.config.sets.push(set);
        run(&b)
    };
    let bare = run(&build(vec![group(&["IceNova"])]));
    // 1 / (0.5 / round(1 + 20 / 100, 2)).
    let r = with(ConfigValue::Boolean(true));
    assert_eq!(r.fields[&Field::Speed].state, FieldState::Complete);
    assert_eq!(value(&r, Field::Speed), 1.0 / (0.5 / 1.2));
    assert_eq!(
        value(&r, Field::AverageDamage),
        value(&bare, Field::AverageDamage)
    );
    let r = with(ConfigValue::Boolean(false));
    assert_eq!(value(&r, Field::Speed), value(&bare, Field::Speed));
    let r = with(ConfigValue::String("true".to_owned()));
    let FieldState::Partial(p) = &r.fields[&Field::Speed].state else {
        panic!("not partial");
    };
    assert!(p.contains(&UnsupportedReason::UnmodelledConfig {
        input: "buffOnslaught".to_owned(),
    }));
}

/// Lines on a charge input (a build document can carry them) are a reason
/// under its name, as on any input other than `customMods` (P3-12e
/// review).
#[test]
fn lines_on_a_charge_input_should_be_a_reason() {
    let mut b = build(vec![group(&["IceNova"])]);
    let mut set = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        custom_modifiers: Vec::new(),
        attributes: Vec::new(),
    };
    set.inputs.push(ConfigInput {
        name: "usePowerCharges".to_owned(),
        value: ConfigValue::Boolean(true),
        kind: loom_domain::ConfigInputKind::Input,
        lines: vec![loom_domain::CustomModifierLine {
            text: "+10 to maximum Life".to_owned(),
            resolution: loom_domain::LineResolution::Pending,
        }],
        attributes: Vec::new(),
    });
    b.config.sets.push(set);
    let r = run(&b);
    let FieldState::Partial(p) = &r.fields[&Field::CritChance].state else {
        panic!("not partial");
    };
    assert!(p.contains(&UnsupportedReason::UnmodelledConfig {
        input: "usePowerCharges".to_owned(),
    }));
}

// ------------------------------------------------- other gems (P3-12h)

fn gem_reasons(r: &CalcResult) -> Vec<String> {
    let reasons = match &r.fields[&Field::Life].state {
        FieldState::Partial(p) | FieldState::Indeterminate { partial: p, .. } => p.clone(),
        _ => std::collections::BTreeSet::new(),
    };
    let mut out: Vec<String> = reasons
        .iter()
        .filter_map(|x| match x {
            UnsupportedReason::UnmodelledGem { source } => Some(format!("gem {source:?}")),
            UnsupportedReason::UnmodelledSkill { source, what } => {
                Some(format!("skill {what} {source:?}"))
            }
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn mana(r: &CalcResult) -> f64 {
    value(r, Field::Mana)
}

#[test]
fn an_aura_should_buff_and_reserve_and_inert_gems_should_do_nothing() {
    let bare = run(&build(vec![group(&["IceNova"])]));
    let r = run(&build(vec![
        group(&["IceNova"]),
        group(&["Precision"]),
        group(&["Haste"]),
        group(&["GroundSlam", "SupportFasterAttacks"]),
    ]));
    assert_eq!(gem_reasons(&r), Vec::<String>::new());
    for f in [
        Field::ManaUnreserved,
        Field::CritChance,
        Field::Speed,
        Field::Life,
    ] {
        assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
    }
    // 30 flat, and ceil(mana x 25.5 / 100) from the percent.
    let m = mana(&r);
    assert_eq!(
        value(&r, Field::ManaUnreserved),
        m - (30.0 + (m * 25.5 / 100.0).ceil())
    );
    // 20% increased crit chance; 10% increased cast speed.
    assert_eq!(
        value(&r, Field::CritChance),
        value(&bare, Field::CritChance) * 1.2
    );
    assert_eq!(value(&r, Field::Speed), value(&bare, Field::Speed) * 1.1);
    // The life pool is not reserved: one term, its own node.
    assert_eq!(value(&r, Field::LifeUnreserved), value(&r, Field::Life));
}

#[test]
fn a_supports_cost_multiplier_should_scale_the_reservation() {
    let r = run(&build(vec![
        group(&["IceNova"]),
        group(&["Precision", "SupportFasterAttacks"]),
    ]));
    // floor(30 x 1.5) = 45.
    assert_eq!(value(&r, Field::ManaUnreserved), mana(&r) - 45.0);
}

#[test]
fn what_the_tables_do_not_model_should_be_reasons() {
    // An active skill not in the table; a support not in the table on an
    // inert skill; spirit reservation.
    let r = run(&build(vec![
        group(&["IceNova"]),
        group(&["TestStrike"]),
        group(&["GroundSlam", "SupportTestMore"]),
        group(&["Discipline"]),
    ]));
    assert_eq!(
        gem_reasons(&r),
        [
            "gem Gem { group: 1, gem: 0 }",
            "gem Gem { group: 2, gem: 1 }",
            "skill spirit-reservation Gem { group: 3, gem: 0 }",
        ]
    );
    assert_eq!(
        r.fields[&Field::ManaUnreserved].state.kind(),
        StateKind::Partial
    );
}

#[test]
fn groups_pob_does_not_create_should_change_nothing() {
    let bare = run(&build(vec![group(&["IceNova"])]));
    let mut disabled = group(&["Precision"]);
    disabled.enabled = false;
    let mut swap = group(&["Precision"]);
    swap.slot = Some(loom_domain::SlotId::new("Weapon 1 Swap"));
    let mut off = group(&["Haste"]);
    let mut g = gem("Haste");
    g.attributes.push(loom_domain::Attribute {
        name: "enableGlobal1".to_owned(),
        value: "false".to_owned(),
    });
    off.gems = Ordered::new();
    off.gems.push(g);
    let r = run(&build(vec![group(&["IceNova"]), disabled, swap, off]));
    assert_eq!(gem_reasons(&r), Vec::<String>::new());
    for f in [Field::ManaUnreserved, Field::CritChance, Field::Speed] {
        assert_eq!(value(&r, f), value(&bare, f), "{f}");
        assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
    }
}

#[test]
fn a_group_with_a_source_should_be_a_reason() {
    let mut granted = group(&["Precision"]);
    granted.attributes.push(loom_domain::Attribute {
        name: "source".to_owned(),
        value: "Item:3".to_owned(),
    });
    let r = run(&build(vec![group(&["IceNova"]), granted]));
    assert_eq!(gem_reasons(&r), ["gem Gem { group: 1, gem: 0 }"]);
}

#[test]
fn a_main_skill_that_reserves_should_be_a_reason() {
    let mut data = Data::new();
    for e in &mut data.effects {
        if e.id == "IceNova" {
            e.levels[0].reservations = vec![loom_data::bundle::skills::Reservation {
                resource: loom_data::bundle::skills::Resource::Mana,
                percent: false,
                amount: 5,
            }];
        }
    }
    for a in &mut data.actives {
        if a.id == "nova" {
            a.types.push("HasReservation".to_owned());
        }
    }
    let b = build(vec![group(&["IceNova"])]);
    let options = CalcOptions::default();
    let input = CalcInput {
        build: &b,
        data: &data,
        options: &options,
    };
    let r = calculate(&input).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        gem_reasons(&r),
        ["skill reservation Gem { group: 0, gem: 0 }"]
    );
}

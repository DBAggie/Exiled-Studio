//! The main skill's hit as a self-cast spell (Phase 3 plan, step 9b):
//! base damage, added damage times effectiveness, increased and more
//! damage (PoB's more rounding), crit, cast rate, the enemy's resistance
//! and penetration, and which skills and gems the engine models.
//!
//! The data is made up (a nova with Ice Nova's shape under the game's stat
//! IDs); the expected values follow PoB v2.67.2's formulas (module docs of
//! `loom_calc::poe1::hit`). The mechanic fixtures check the same against
//! the oracle (`docs/phase-3/spell-hits.md`).

use loom_calc::poe1::gem_levels::GemLevels;
use loom_calc::poe1::hit::spell_skill;
use loom_calc::poe1::sockets::{ALWAYS_MATCHES, SocketMatch, socket_matches};
use loom_calc::{
    ActiveSkill, CalcData, CalcInput, CalcOptions, CalcResult, CharacterBase, EffectLevel, Field,
    FieldState, GemEffect, GrantedEffect, PassiveLinks, SkillGem, Source, StatAmount, StatSet,
    StatSetLevel, StateKind, UnsupportedReason, calculate,
};
use loom_data::bundle::skills::{FloatStat, LevelStat};
use loom_domain::{
    Build, ClassId, ConfigInput, ConfigSet, ConfigSetId, ConfigValue, DataVersion, Game, Gem,
    GemId, Item, ItemBody, ItemId, ItemLine, ItemSet, ItemSetId, ItemsEntry, LineKind,
    LineResolution, ModSection, ModifierLine, Ordered, Resolved, SkillId, SkillSet, SkillSetId,
    SlotEntry, SlotId, SocketGroup, Sourced, StatId, StatLine, StatValue,
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
        attribute: None,
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
                active("chilling", &["Spell", "Area", "Cold", "ChillingArea"]),
                active("hex", &["Spell", "Area", "Hex", "AppliesCurse", "Cold"]),
                active("bomb", &["Spell", "Area", "Cold", "Duration"]),
                active("herald", &["Spell", "Buff", "Herald", "Fire"]),
                active("stance", &["Spell", "Buff", "Aura", "Physical"]),
                active("golem", &["Spell", "Minion", "Golem", "CreatesMinion"]),
            ],
            gems,
            gem_effects,
            constants: true,
        };
        s.effects.append(&mut effects);
        s.sets.append(&mut sets);
        // The gems' colours (P3-12i): Ice Nova blue, Precision and Faster
        // Attacks green; the others have none (a bundle without the field).
        for e in &mut s.effects {
            e.attribute = match e.id.as_str() {
                "IceNova" => Some(3),
                "Precision" | "SupportFasterAttacks" => Some(2),
                _ => None,
            };
        }
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
/// - `SupportTestMore` (above) supports a damaging skill: not modelled;
/// - `Frostblink`: an inert spell typed `ChillingArea` (P3-8m): a
///   guaranteed chill.
/// - `Frostbite`: a hex, -20 to the enemy's cold resistance (P3-close);
/// - `FrostBomb`: a debuff, -15 cold exposure;
/// - `HeraldOfAsh`: 15% of physical damage gained as extra fire;
/// - `FleshAndStone`: Blood Stance, 10% more physical damage taken by the
///   enemy from hits;
/// - `SummonFlameGolem`: a golem's buff, 20% increased damage.
const OTHER_GEMS: [&str; 11] = [
    "Precision",
    "Haste",
    "Discipline",
    "GroundSlam",
    "SupportFasterAttacks",
    "Frostblink",
    "Frostbite",
    "FrostBomb",
    "HeraldOfAsh",
    "FleshAndStone",
    "SummonFlameGolem",
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
    sets.push(stat_set("Frostblink", Vec::new(), Vec::new()));
    sets.push(stat_set(
        "Frostbite",
        Vec::new(),
        vec![int("base_cold_damage_resistance_%", -20)],
    ));
    sets.push(stat_set(
        "FrostBomb",
        Vec::new(),
        vec![int("base_cold_damage_resistance_%", -15)],
    ));
    sets.push(stat_set(
        "HeraldOfAsh",
        Vec::new(),
        vec![int("physical_damage_%_to_add_as_fire", 15)],
    ));
    sets.push(stat_set(
        "FleshAndStone",
        Vec::new(),
        vec![int(
            "flesh_stone_blood_stance_enemies_physical_damage_taken_when_hit_+%_final_from_player_distance",
            10,
        )],
    ));
    sets.push(stat_set(
        "SummonFlameGolem",
        Vec::new(),
        vec![int("fire_golem_grants_damage_+%", 20)],
    ));
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
        effect("Frostblink", Some("chilling"), false, 0),
        effect("Frostbite", Some("hex"), false, 0),
        effect("FrostBomb", Some("bomb"), false, 0),
        effect("HeraldOfAsh", Some("herald"), false, 0),
        effect("FleshAndStone", Some("stance"), false, 0),
        effect("SummonFlameGolem", Some("golem"), false, 0),
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
        assert!(
            spell_skill(&b, &Data::new(), &GemLevels::none()).is_none(),
            "{main}"
        );
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
    assert!(spell_skill(&build(vec![g]), &Data::new(), &GemLevels::none()).is_none());
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

/// A modifier "Onslaught" (a custom line, `base_should_have_onslaught_from_stat`)
/// gives Onslaught as the check does; any number of sources give it once,
/// as PoB's flag (P3-12j).
#[test]
fn an_onslaught_modifier_should_grant_onslaught_once() {
    let onslaught_line = || loom_domain::CustomModifierLine {
        text: "Onslaught".to_owned(),
        resolution: LineResolution::Resolved {
            result: Resolved::Stats(vec![StatLine {
                stat: StatId::new("base_should_have_onslaught_from_stat"),
                value: StatValue::Exact { min: 1, max: 1 },
            }]),
            adapted: None,
        },
    };
    let with = |lines: usize, check: bool| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: vec![loom_domain::CustomModifierBlock {
                title: None,
                enabled: true,
                text: "Onslaught\n".repeat(lines),
                lines: (0..lines).map(|_| onslaught_line()).collect(),
                attributes: Vec::new(),
            }],
            attributes: Vec::new(),
        };
        if check {
            set.inputs.push(ConfigInput {
                name: "buffOnslaught".to_owned(),
                value: ConfigValue::Boolean(true),
                kind: loom_domain::ConfigInputKind::Input,
                lines: Vec::new(),
                attributes: Vec::new(),
            });
        }
        b.config.sets.push(set);
        run(&b)
    };
    let bare = run(&build(vec![group(&["IceNova"])]));
    assert_eq!(
        value(&with(0, false), Field::Speed),
        value(&bare, Field::Speed)
    );
    // A disabled block gives nothing.
    let mut b = build(vec![group(&["IceNova"])]);
    b.config.sets.push(ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        custom_modifiers: vec![loom_domain::CustomModifierBlock {
            title: None,
            enabled: false,
            text: "Onslaught".to_owned(),
            lines: vec![onslaught_line()],
            attributes: Vec::new(),
        }],
        attributes: Vec::new(),
    });
    assert_eq!(value(&run(&b), Field::Speed), value(&bare, Field::Speed));
    // 1 / (0.5 / round(1 + 20 / 100, 2)), whatever the sources.
    for (lines, check) in [(1, false), (2, false), (1, true), (0, true)] {
        let r = with(lines, check);
        assert_eq!(
            r.fields[&Field::Speed].state,
            FieldState::Complete,
            "{lines} {check}"
        );
        assert_eq!(
            value(&r, Field::Speed),
            1.0 / (0.5 / 1.2),
            "{lines} {check}"
        );
    }
}

/// A custom line resolved to one stat with `value`.
fn custom_stat_line(stat: &str, value: i64) -> loom_domain::CustomModifierLine {
    loom_domain::CustomModifierLine {
        text: stat.to_owned(),
        resolution: LineResolution::Resolved {
            result: Resolved::Stats(vec![StatLine {
                stat: StatId::new(stat),
                value: StatValue::Exact {
                    min: value,
                    max: value,
                },
            }]),
            adapted: None,
        },
    }
}

/// Ice Nova with custom lines and config inputs.
fn ice_nova_with(lines: &[(&str, i64)], inputs: &[(&str, ConfigValue)]) -> CalcResult {
    groups_with(vec![group(&["IceNova"])], lines, inputs)
}

/// These groups (the first the main one) with custom lines and config
/// inputs.
fn groups_with(
    groups: Vec<SocketGroup>,
    lines: &[(&str, i64)],
    inputs: &[(&str, ConfigValue)],
) -> CalcResult {
    let mut b = build(groups);
    let mut set = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        custom_modifiers: vec![loom_domain::CustomModifierBlock {
            title: None,
            enabled: true,
            text: String::new(),
            lines: lines.iter().map(|&(s, v)| custom_stat_line(s, v)).collect(),
            attributes: Vec::new(),
        }],
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
}

/// The enemy's exposure (P3-8k, `poe1::exposure`): the check gives -10
/// to the enemy's resistance only with a chance of its type above 0, plus
/// every extra of all types or of that type, before the resistance's
/// maximum (PoB's `CalcPerform.lua` 375 to 385, 3563 to 3589).
#[test]
fn exposure_should_lower_the_enemys_resistance_as_pob_does() {
    let on = || ConfigValue::Boolean(true);
    let text = |v: &str| ConfigValue::String(v.to_owned());
    let cold = ("conditionEnemyColdExposure", on());
    let chance = (
        "chance_to_inflict_cold_exposure_on_hit_with_cold_damage_%",
        10,
    );
    let all = (
        "exposure_you_inflict_applies_extra_%_to_affected_resistance",
        -25,
    );
    let extra_cold = (
        "cold_exposure_you_inflict_applies_extra_cold_resistance_+%",
        -5,
    );
    let extra_fire = (
        "fire_exposure_you_inflict_applies_extra_fire_resistance_+%",
        -7,
    );
    let bare = run(&build(vec![group(&["IceNova"])]));
    for (lines, inputs, resist) in [
        (vec![chance], vec![cold.clone()], 40.0),
        (vec![chance, all], vec![cold.clone()], 15.0),
        (
            vec![chance, all, extra_cold, extra_fire],
            vec![cold.clone()],
            10.0,
        ),
        // Inflict all three; cold skills' chance.
        (
            vec![("inflict_all_exposure_on_hit", 1)],
            vec![cold.clone()],
            40.0,
        ),
        (
            vec![("cold_skill_chance_to_inflict_cold_exposure_%", 25)],
            vec![cold.clone()],
            40.0,
        ),
        // Elemental Army's exposure type gives the chance.
        (
            vec![all],
            vec![cold.clone(), ("elementalArmyExposureType", text("Cold"))],
            15.0,
        ),
    ] {
        let r = ice_nova_with(&lines, &inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f} {lines:?}");
        }
        assert_eq!(
            value(&r, Field::AverageDamage).to_bits(),
            average(100.0, 200.0, 5.0, 1.5, resist).to_bits(),
            "{lines:?} {inputs:?}"
        );
    }
    // Nothing: the check without a chance of its type, a chance without
    // the check, another type's check, Elemental Army of another type or
    // none, the check off.
    for (lines, inputs) in [
        (vec![all], vec![cold.clone()]),
        (vec![chance, all], vec![]),
        (
            vec![("fire_skill_chance_to_inflict_fire_exposure_%", 25), all],
            vec![("conditionEnemyFireExposure", on()), cold.clone()],
        ),
        (
            vec![all],
            vec![cold.clone(), ("elementalArmyExposureType", text("Fire"))],
        ),
        (
            vec![all],
            vec![
                cold.clone(),
                (
                    "elementalArmyExposureType",
                    ConfigValue::Number(Sourced::new(loom_domain::Number(0.0))),
                ),
            ],
        ),
        (
            vec![chance, all],
            vec![("conditionEnemyColdExposure", ConfigValue::Boolean(false))],
        ),
    ] {
        let r = ice_nova_with(&lines, &inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f} {lines:?}");
            assert_eq!(
                value(&r, f).to_bits(),
                value(&bare, f).to_bits(),
                "{f} {lines:?}"
            );
        }
    }
    // Values PoB would read otherwise: a reason under their input.
    for (inputs, reason) in [
        (
            vec![("conditionEnemyColdExposure", text("yes"))],
            "conditionEnemyColdExposure",
        ),
        (
            vec![("elementalArmyExposureType", text("Water"))],
            "elementalArmyExposureType",
        ),
    ] {
        let r = ice_nova_with(&[chance], &inputs);
        let FieldState::Partial(p) = &r.fields[&Field::AverageDamage].state else {
            panic!("not partial: {inputs:?}");
        };
        assert!(
            p.contains(&UnsupportedReason::UnmodelledConfig {
                input: reason.to_owned(),
            }),
            "{inputs:?}"
        );
    }
    // A stat that would change the exposure otherwise stays a reason.
    let r = ice_nova_with(
        &[
            chance,
            ("exposure_you_inflict_has_minimum_resistance_%", -18),
        ],
        &[cold],
    );
    assert!(matches!(
        r.fields[&Field::AverageDamage].state,
        FieldState::Partial(_)
    ));
}

/// The guaranteed shock (P3-8k, `poe1::damage_taken`): with the enemy
/// shocked, the shock is `floor(min(max(minimums x (1 + inc / 100),
/// configured), 50))`, the minimums summed (PoB's `CalcPerform.lua` 3499
/// to 3546); without the check, nothing.
#[test]
fn a_guaranteed_shock_should_raise_the_shock_as_pob_does() {
    let on = || ConfigValue::Boolean(true);
    let n = |v: f64| ConfigValue::Number(Sourced::new(loom_domain::Number(v)));
    let shocked = |effect: f64| {
        vec![
            ("conditionEnemyShocked", on()),
            ("conditionShockEffect", n(effect)),
        ]
    };
    let minimum = |v| ("shock_minimum_damage_taken_increase_%", v);
    let lightning = |v| ("lightning_ailment_effect_+%", v);
    let expected = |shock: f64| {
        let taken = 1.0 * (1.0 + shock / 100.0);
        let effect = taken * (1.0 - 50.0 / 100.0);
        let hit = (100.0 / 2.0 + 200.0 / 2.0) * effect;
        let crit_hit = ((100.0 * 1.5) / 2.0 + (200.0 * 1.5) / 2.0) * effect;
        let average_hit = hit * (1.0 - 5.0 / 100.0) + crit_hit * 5.0 / 100.0;
        average_hit * 100.0 / 100.0
    };
    for (lines, inputs, shock) in [
        (vec![minimum(25)], shocked(10.0), 25.0),
        // 25 x 1.5 = 37.5, floored.
        (vec![minimum(25), lightning(50)], shocked(10.0), 37.0),
        // The configured shock is stronger.
        (vec![minimum(25), lightning(50)], shocked(40.0), 40.0),
        // No configured effect: the minimum alone.
        (
            vec![minimum(25)],
            vec![("conditionEnemyShocked", on())],
            25.0,
        ),
        // Minimums add up; the three effect stats add up: 35 x 1.3.
        (
            vec![
                minimum(25),
                minimum(10),
                lightning(10),
                ("non_damaging_ailment_effect_+%", 10),
                ("shock_effect_+%", 10),
            ],
            shocked(5.0),
            45.0,
        ),
        // At most 50.
        (vec![minimum(40), lightning(50)], shocked(10.0), 50.0),
    ] {
        let r = ice_nova_with(&lines, &inputs);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f} {lines:?}");
        }
        assert_eq!(
            value(&r, Field::AverageDamage).to_bits(),
            expected(shock).to_bits(),
            "{lines:?}"
        );
    }
    // Without the check a minimum shocks nothing; "Hits always Shock" and
    // "All Damage can Shock" reach no field.
    let bare = run(&build(vec![group(&["IceNova"])]));
    for lines in [
        vec![minimum(25), lightning(50)],
        vec![
            ("always_shock", 1),
            ("all_damage_can_shock", 1),
            minimum(25),
        ],
    ] {
        let r = ice_nova_with(&lines, &[]);
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f} {lines:?}");
            assert_eq!(value(&r, f).to_bits(), value(&bare, f).to_bits(), "{f}");
        }
    }
    // A stat that would change the shock otherwise stays a reason.
    let r = ice_nova_with(
        &[minimum(25), ("shock_maximum_magnitude_+", 10)],
        &shocked(10.0),
    );
    assert!(matches!(
        r.fields[&Field::AverageDamage].state,
        FieldState::Partial(_)
    ));
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

/// The other gems' reservation and aura effect modifiers (P3-8l,
/// `poe1::skill_modifiers`): efficiency divides each reservation, rounded
/// as PoB rounds it; "Reservation of Skills" multiplies it; the effect of
/// non-curse auras scales each aura's mods, each to PoB's
/// `modf(round(value x scale, 2))`.
#[test]
fn reservation_and_aura_effect_modifiers_should_apply_as_pob() {
    let auras = || {
        build(vec![
            group(&["IceNova"]),
            group(&["Precision"]),
            group(&["Haste"]),
        ])
    };
    let bare = run(&build(vec![group(&["IceNova"])]));
    let plain = run(&auras());
    let with = |lines: &[(&str, i64)]| {
        let mut b = auras();
        b.config.sets.push(ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: vec![loom_domain::CustomModifierBlock {
                title: None,
                enabled: true,
                text: String::new(),
                lines: lines.iter().map(|&(s, v)| custom_stat_line(s, v)).collect(),
                attributes: Vec::new(),
            }],
            attributes: Vec::new(),
        });
        run(&b)
    };
    // 13% mana and 7% generic efficiency, 8% reduced reservation:
    // flat round(30 x 0.92 / 1.2) = round(23.0) = 23; percent
    // round(25.5 x 0.92 / 1.2, 2) = 19.55.
    let r = with(&[
        ("base_mana_reservation_efficiency_+%", 13),
        ("base_reservation_efficiency_+%", 7),
        ("base_reservation_+%", -8),
    ]);
    assert_eq!(gem_reasons(&r), Vec::<String>::new());
    assert_eq!(r.fields[&Field::ManaUnreserved].state, FieldState::Complete);
    let m = mana(&r);
    assert_eq!(
        value(&r, Field::ManaUnreserved),
        m - (23.0 + (m * 19.55 / 100.0).ceil())
    );
    // The doubled stat: 3 is 6% reduced efficiency, 30 / 0.94 = 31.9 to 32.
    let r = with(&[("mana_reservation_efficiency_-2%_per_1", 3)]);
    let m = mana(&r);
    let percent = (25.5f64 / 0.94 * 100.0 + 0.5).floor() / 100.0;
    assert_eq!(
        value(&r, Field::ManaUnreserved),
        m - (32.0 + (m * percent / 100.0).ceil())
    );
    // Life efficiency reserves no mana differently.
    let r = with(&[("base_life_reservation_efficiency_+%", 40)]);
    assert_eq!(
        value(&r, Field::ManaUnreserved),
        value(&plain, Field::ManaUnreserved)
    );
    // 25% increased effect of non-curse auras: 20% crit becomes 25%, 10%
    // cast speed 12% (12.5 rounds to 12.5, its integer part 12).
    let r = with(&[("non_curse_aura_effect_+%", 25)]);
    assert_eq!(gem_reasons(&r), Vec::<String>::new());
    assert_eq!(
        value(&r, Field::CritChance),
        value(&bare, Field::CritChance) * 1.25
    );
    assert_eq!(value(&r, Field::Speed), value(&bare, Field::Speed) * 1.12);
    // Its reservation does not change.
    assert_eq!(
        value(&r, Field::ManaUnreserved),
        value(&plain, Field::ManaUnreserved)
    );
    // An efficiency of -100 or below: PoB divides by 0, a reason.
    let r = with(&[("base_reservation_efficiency_+%", -100)]);
    assert_eq!(
        gem_reasons(&r),
        [
            "skill reservation-efficiency Gem { group: 1, gem: 0 }",
            "skill reservation-efficiency Gem { group: 2, gem: 0 }",
        ]
    );
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

// ------------------------------------------------- socket colour (P3-12i)

fn spec(key: &str, value: &str) -> ItemLine {
    ItemLine {
        source: format!("{key}: {value}"),
        kind: LineKind::Spec {
            key: key.to_owned(),
            value: value.to_owned(),
            known: true,
        },
    }
}

fn stat_line(id: &str) -> ItemLine {
    ItemLine {
        source: id.to_owned(),
        kind: LineKind::Modifier(ModifierLine {
            text: id.to_owned(),
            prefixes: Vec::new(),
            section: ModSection::Explicit,
            selected: true,
            resolution: LineResolution::Resolved {
                result: Resolved::Stats(vec![StatLine {
                    stat: StatId::new(id),
                    value: StatValue::Exact { min: 1, max: 1 },
                }]),
                adapted: None,
            },
        }),
    }
}

/// `b` with an item of these lines in `slot` of one item set.
fn with_item(mut b: Build, slot: &str, lines: Vec<ItemLine>) -> Build {
    b.items.entries.push(ItemsEntry::Item(Item {
        id: ItemId(1),
        body: ItemBody {
            lines,
            ..ItemBody::default()
        },
        layout: Ordered::new(),
        text_spacing: Vec::new(),
        attributes: Vec::new(),
    }));
    let mut set = ItemSet {
        id: ItemSetId(1),
        title: None,
        slots: Ordered::new(),
        attributes: Vec::new(),
    };
    set.slots.push(SlotEntry {
        slot: SlotId::new(slot),
        item: Some(ItemId(1)),
        attributes: Vec::new(),
    });
    b.items.entries.push(ItemsEntry::Set(set));
    b
}

fn in_slot(mut g: SocketGroup, slot: &str) -> SocketGroup {
    g.slot = Some(SlotId::new(slot));
    g
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn matches(b: &Build) -> Vec<Vec<SocketMatch>> {
    let groups = loom_calc::poe1::skill::active_groups(b).unwrap();
    socket_matches(b, &groups, &Data::new())
}

#[test]
fn a_gem_in_a_socket_of_its_colour_should_gain_quality() {
    use SocketMatch::{No, Yes};
    // Ice Nova (blue) in socket 1, Precision (green) in socket 2.
    let gloves = |sockets: &str| {
        with_item(
            build(vec![in_slot(group(&["IceNova", "Precision"]), "Gloves")]),
            "Gloves",
            vec![spec("Sockets", sockets)],
        )
    };
    assert_eq!(matches(&gloves("B-G")), [vec![Yes, Yes]]);
    assert_eq!(matches(&gloves("G-B")), [vec![No, No]]);
    // A white socket gives none; a missing socket none.
    assert_eq!(matches(&gloves("W-W")), [vec![No, No]]);
    assert_eq!(matches(&gloves("B")), [vec![Yes, No]]);
    // Abyssal sockets count in the order.
    assert_eq!(matches(&gloves("A-B-G")), [vec![No, No]]);
    let data = Data::new();
    let main =
        loom_calc::poe1::skill::main_skill(&gloves("B-G"), &data, &GemLevels::none()).unwrap();
    assert_eq!((main.active_gem.quality, main.active_gem.socket), (10, Yes));
    let main =
        loom_calc::poe1::skill::main_skill(&gloves("R-G"), &data, &GemLevels::none()).unwrap();
    assert_eq!(main.active_gem.quality, 0);
}

#[test]
fn earlier_groups_of_the_slot_should_count_every_gem() {
    use SocketMatch::{No, Yes};
    // Group 1 (Ice Nova, a disabled gem) takes sockets 1 and 2; group 2's
    // Precision sits in socket 3; group 3 is in another slot, group 4 in
    // none.
    let mut first = group(&["IceNova", "Precision"]);
    first.gems.typed_mut().nth(1).unwrap().enabled = false;
    let b = with_item(
        build(vec![
            in_slot(first, "Gloves"),
            in_slot(group(&["Precision"]), "Gloves"),
            in_slot(group(&["Precision"]), "Helmet"),
            group(&["Precision"]),
        ]),
        "Gloves",
        vec![spec("Sockets", "B-R G")],
    );
    assert_eq!(matches(&b), [vec![Yes, No], vec![Yes], vec![No], vec![No]]);
}

#[test]
fn items_whose_sockets_are_not_known_should_be_a_reason() {
    use SocketMatch::{Unknown, Yes};
    let b = |lines| {
        with_item(
            build(vec![
                group(&["IceNova"]),
                in_slot(group(&["Precision", "GroundSlam"]), "Gloves"),
            ]),
            "Gloves",
            lines,
        )
    };
    // "Gems Socketed always have the Quality bonus": every gem matches,
    // whatever the colours.
    let always = b(vec![spec("Sockets", "R-R"), stat_line(ALWAYS_MATCHES)]);
    assert_eq!(matches(&always)[1], [Yes, Yes]);
    assert_eq!(gem_reasons(&run(&always)), Vec::<String>::new());
    for (lines, why) in [
        (vec![], "no-sockets-line"),
        (
            vec![spec("Sockets", "G"), spec("Sockets", "G")],
            "several-sockets-lines",
        ),
        (
            vec![spec("Sockets", "G"), stat_line("local_has_X_sockets")],
            "socket-count-line",
        ),
    ] {
        let build = b(lines);
        assert_eq!(matches(&build)[1], [Unknown(why), Unknown(why)], "{why}");
        // Precision's quality is a reason; the inert Ground Slam's is not.
        assert_eq!(
            gem_reasons(&run(&build)),
            ["skill socket-colour-quality Gem { group: 1, gem: 0 }"],
            "{why}"
        );
    }
    // A gem whose colour the data lacks (Haste) is not known in a socket.
    let haste = with_item(
        build(vec![
            group(&["IceNova"]),
            in_slot(group(&["Haste"]), "Gloves"),
        ]),
        "Gloves",
        vec![spec("Sockets", "G")],
    );
    assert_eq!(matches(&haste)[1], [Unknown("no-colour")]);
    // No item in the slot: nothing matches, and nothing is a reason.
    let empty = build(vec![
        group(&["IceNova"]),
        in_slot(group(&["Precision"]), "Gloves"),
    ]);
    assert_eq!(matches(&empty)[1], [SocketMatch::No]);
    assert_eq!(gem_reasons(&run(&empty)), Vec::<String>::new());
}

/// The review's cases (P3-12i): a disabled earlier group and a gem the
/// data does not have still take their sockets; untyped gem content makes
/// the rest of the slot unknown; the last entry of a slot name holds the
/// item; an empty slot, an item the build does not have, and a white
/// socket for a gem without a colour.
#[test]
fn the_socket_index_should_follow_pobs_count() {
    use SocketMatch::{No, Unknown, Yes};
    let gloves = |groups: Vec<SocketGroup>, sockets: &str| {
        with_item(build(groups), "Gloves", vec![spec("Sockets", sockets)])
    };
    // A disabled group counts its gems.
    let mut off = in_slot(group(&["IceNova"]), "Gloves");
    off.enabled = false;
    let b = gloves(
        vec![
            group(&["IceNova"]),
            off,
            in_slot(group(&["Precision"]), "Gloves"),
        ],
        "B-G",
    );
    assert_eq!(matches(&b), [vec![No], vec![Yes], vec![Yes]]);
    // A gem the data does not have never matches but takes its socket.
    let b = gloves(
        vec![
            group(&["IceNova"]),
            in_slot(group(&["NotAGem", "Precision"]), "Gloves"),
        ],
        "G-G",
    );
    assert_eq!(matches(&b)[1], [No, Yes]);
    // Untyped gem content: this group's and later groups' gems of the slot
    // are not known.
    let mut untyped = in_slot(group(&["Precision"]), "Gloves");
    untyped
        .gems
        .push_opaque(loom_domain::OpaqueNode::element("Gem"));
    let b = gloves(
        vec![
            group(&["IceNova"]),
            untyped,
            in_slot(group(&["Precision"]), "Gloves"),
        ],
        "G-G-G",
    );
    assert_eq!(
        matches(&b)[1..],
        [vec![Unknown("untyped-gem")], vec![Unknown("untyped-gem")]]
    );
    // A gem without a colour in a white socket, or past the last socket:
    // no match whatever its colour, so no reason.
    let b = gloves(
        vec![
            group(&["IceNova"]),
            in_slot(group(&["Haste", "Haste"]), "Gloves"),
        ],
        "W",
    );
    assert_eq!(matches(&b)[1], [No, No]);
    // The last entry of a slot name holds the item; `itemId="0"` none.
    let mut b = gloves(
        vec![
            group(&["IceNova"]),
            in_slot(group(&["Precision"]), "Gloves"),
        ],
        "G",
    );
    let set = b
        .items
        .entries
        .typed_mut()
        .find_map(|e| match e {
            ItemsEntry::Set(s) => Some(s),
            ItemsEntry::Item(_) => None,
        })
        .unwrap();
    set.slots.push(SlotEntry {
        slot: SlotId::new("Gloves"),
        item: None,
        attributes: Vec::new(),
    });
    assert_eq!(matches(&b)[1], [No]);
    // A slot naming an item the build does not have.
    let set = b
        .items
        .entries
        .typed_mut()
        .find_map(|e| match e {
            ItemsEntry::Set(s) => Some(s),
            ItemsEntry::Item(_) => None,
        })
        .unwrap();
    set.slots.push(SlotEntry {
        slot: SlotId::new("Gloves"),
        item: Some(ItemId(9)),
        attributes: Vec::new(),
    });
    assert_eq!(matches(&b)[1], [Unknown("untyped-item")]);
}

/// Lines of an unselected variant are not part of the item; every
/// rebuilding stat is unknown; the always-matching line wins over a
/// socket rebuild, even "Has no Sockets" (PoB sets the flag after the
/// rebuild).
#[test]
fn socket_lines_should_count_only_when_part_of_the_item() {
    use SocketMatch::{Unknown, Yes};
    let b = |lines| {
        with_item(
            build(vec![
                group(&["IceNova"]),
                in_slot(group(&["Precision"]), "Gloves"),
            ]),
            "Gloves",
            lines,
        )
    };
    let mut unselected = stat_line(ALWAYS_MATCHES);
    if let LineKind::Modifier(m) = &mut unselected.kind {
        m.resolution = LineResolution::Excluded(loom_domain::Exclusion::NotSelected);
    }
    assert_eq!(
        matches(&b(vec![spec("Sockets", "G"), unselected]))[1],
        [Yes]
    );
    for stat in loom_calc::poe1::sockets::REBUILDS {
        assert_eq!(
            matches(&b(vec![spec("Sockets", "G"), stat_line(stat)]))[1],
            [Unknown("socket-count-line")],
            "{stat}"
        );
    }
    assert_eq!(
        matches(&b(vec![
            spec("Sockets", "R"),
            stat_line("local_has_no_sockets"),
            stat_line(ALWAYS_MATCHES),
        ]))[1],
        [Yes]
    );
}

/// Charge modifiers (P3-8d): the maximum and minimum add to PoB's count,
/// `max(use ? (override or max) : 0, min(max, min))`, and an effect per
/// charge acts times the count. The data's constants: 3 charges, 50%
/// increased crit chance each; Ice Nova's base crit 5%.
#[test]
fn charge_modifiers_should_count_as_pob_does() {
    let line = |stat: &str, v: i64| loom_domain::CustomModifierLine {
        text: stat.to_owned(),
        resolution: LineResolution::Resolved {
            result: Resolved::Stats(vec![StatLine {
                stat: StatId::new(stat),
                value: StatValue::Exact { min: v, max: v },
            }]),
            adapted: None,
        },
    };
    let with = |inputs: &[(&str, f64)], use_on: bool, lines: &[(&str, i64)]| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: vec![loom_domain::CustomModifierBlock {
                title: None,
                enabled: true,
                text: String::new(),
                lines: lines.iter().map(|(s, v)| line(s, *v)).collect(),
                attributes: Vec::new(),
            }],
            attributes: Vec::new(),
        };
        let mut push = |name: &str, value: ConfigValue| {
            set.inputs.push(ConfigInput {
                name: name.to_owned(),
                value,
                kind: loom_domain::ConfigInputKind::Input,
                lines: Vec::new(),
                attributes: Vec::new(),
            });
        };
        if use_on {
            push("usePowerCharges", ConfigValue::Boolean(true));
        }
        for (name, v) in inputs {
            push(
                name,
                ConfigValue::Number(Sourced::new(loom_domain::Number(*v))),
            );
        }
        b.config.sets.push(set);
        run(&b)
    };
    let crit = |r: &CalcResult| {
        assert_eq!(r.fields[&Field::CritChance].state, FieldState::Complete);
        value(r, Field::CritChance)
    };
    // One more maximum: 4 charges, 5% x (1 + 200 / 100).
    assert_eq!(crit(&with(&[], true, &[("max_power_charges", 1)])), 15.0);
    // 10% more per charge: 3 x 60.
    assert_eq!(
        crit(&with(
            &[],
            true,
            &[("critical_strike_chance_+%_per_power_charge", 10)]
        )),
        14.0
    );
    // A minimum without `use`: 2 charges; capped by the maximum (3).
    assert_eq!(
        crit(&with(&[], false, &[("base_minimum_power_charges", 2)])),
        10.0
    );
    assert_eq!(
        crit(&with(&[], false, &[("base_minimum_power_charges", 5)])),
        12.5
    );
    // The override below the minimum: the minimum.
    assert_eq!(
        crit(&with(
            &[("overridePowerCharges", 1.0)],
            true,
            &[("base_minimum_power_charges", 2)]
        )),
        10.0
    );
    // Per-charge modifiers without charges do nothing.
    let bare = run(&build(vec![group(&["IceNova"])]));
    let r = with(&[], false, &[("spell_damage_+%_per_power_charge", 5)]);
    assert_eq!(
        value(&r, Field::AverageDamage),
        value(&bare, Field::AverageDamage)
    );
    // 5% spell damage per charge with 3 charges is 15% increased spell
    // damage.
    let per = with(&[], true, &[("spell_damage_+%_per_power_charge", 5)]);
    let flat = with(&[], true, &[("spell_damage_+%", 15)]);
    assert_eq!(
        value(&per, Field::AverageDamage),
        value(&flat, Field::AverageDamage)
    );
    assert_eq!(
        per.fields[&Field::AverageDamage].state,
        FieldState::Complete
    );
}

/// More charge modifier cases (P3-8d review): frenzy and endurance, a
/// negative maximum, crit multiplier per charge. Constants: 3 charges of
/// each kind; a frenzy charge gives 4% increased cast speed and 4% more
/// damage.
#[test]
fn charge_modifiers_of_every_kind_should_count() {
    let line = |stat: &str, v: i64| loom_domain::CustomModifierLine {
        text: stat.to_owned(),
        resolution: LineResolution::Resolved {
            result: Resolved::Stats(vec![StatLine {
                stat: StatId::new(stat),
                value: StatValue::Exact { min: v, max: v },
            }]),
            adapted: None,
        },
    };
    let with = |checks: &[&str], lines: &[(&str, i64)]| {
        let mut b = build(vec![group(&["IceNova"])]);
        let mut set = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            custom_modifiers: vec![loom_domain::CustomModifierBlock {
                title: None,
                enabled: true,
                text: String::new(),
                lines: lines.iter().map(|(s, v)| line(s, *v)).collect(),
                attributes: Vec::new(),
            }],
            attributes: Vec::new(),
        };
        for name in checks {
            set.inputs.push(ConfigInput {
                name: (*name).to_owned(),
                value: ConfigValue::Boolean(true),
                kind: loom_domain::ConfigInputKind::Input,
                lines: Vec::new(),
                attributes: Vec::new(),
            });
        }
        b.config.sets.push(set);
        run(&b)
    };
    let bare = run(&build(vec![group(&["IceNova"])]));
    // Two frenzy charges from a minimum, no check: 8% increased cast
    // speed, 1 / (0.5 / round(1.08, 2)).
    let r = with(&[], &[("base_minimum_frenzy_charges", 2)]);
    assert_eq!(value(&r, Field::Speed), 1.0 / (0.5 / 1.08));
    // Endurance charges have no effect of their own on a slice field; a
    // modifier per charge does: 3 x 5% = 15% increased damage.
    let per = with(
        &["useEnduranceCharges"],
        &[("damage_+%_per_endurance_charge", 5)],
    );
    let flat = with(&[], &[("damage_+%", 15)]);
    assert_eq!(
        value(&per, Field::AverageDamage),
        value(&flat, Field::AverageDamage)
    );
    // A negative maximum stops at 0 charges.
    let r = with(&["usePowerCharges"], &[("max_power_charges", -5)]);
    assert_eq!(
        value(&r, Field::CritChance),
        value(&bare, Field::CritChance)
    );
    // +10% crit multiplier per power charge, 3 charges: 150 + 30.
    let r = with(
        &["usePowerCharges"],
        &[("critical_strike_multiplier_+_per_power_charge", 10)],
    );
    assert_eq!(value(&r, Field::CritMultiplier), 1.8);
}

/// The main skill alone with character-wide resolved lines (a custom
/// modifier block).
fn with_lines(skill: &str, lines: &[(&str, i64)]) -> CalcResult {
    let line = |stat: &str, v: i64| loom_domain::CustomModifierLine {
        text: stat.to_owned(),
        resolution: LineResolution::Resolved {
            result: Resolved::Stats(vec![StatLine {
                stat: StatId::new(stat),
                value: StatValue::Exact { min: v, max: v },
            }]),
            adapted: None,
        },
    };
    let mut b = build(vec![group(&[skill])]);
    b.config.sets.push(ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        custom_modifiers: vec![loom_domain::CustomModifierBlock {
            title: None,
            enabled: true,
            text: String::new(),
            lines: lines.iter().map(|(s, v)| line(s, *v)).collect(),
            attributes: Vec::new(),
        }],
        attributes: Vec::new(),
    });
    run(&b)
}

/// "with Cold Skills" needs the skill's keyword (its `Cold` type), not
/// cold damage; "Cold Damage with Spell Skills" needs the hit's cold
/// damage and a spell (P3-8h).
#[test]
fn skill_keyword_lines_should_need_the_skills_type() {
    // Every line is mapped: the fields keep the bare skill's state
    // (Ice Nova's complete; the physical nova's waits on armour data).
    let as_bare = |skill: &str, r: &CalcResult| {
        let bare = with_lines(skill, &[]);
        for f in HIT {
            assert_eq!(r.fields[&f].state, bare.fields[&f].state, "{f:?}");
        }
    };
    let same = |a: &CalcResult, b: &CalcResult| {
        for f in HIT {
            assert_eq!(value(a, f), value(b, f), "{f:?}");
        }
    };
    assert_eq!(
        with_lines("IceNova", &[]).fields[&Field::AverageDamage].state,
        FieldState::Complete
    );
    for skill in ["IceNova", "TestPhysicalNova"] {
        let bare = with_lines(skill, &[]);
        // Both novas are cold skills: 20% increased cast speed,
        // 1 / (0.5 / round(1.2, 2)); with elemental skills too.
        for stat in [
            "cast_speed_for_cold_skills_+%",
            "cast_speed_for_elemental_skills_+%",
        ] {
            let r = with_lines(skill, &[(stat, 20)]);
            as_bare(skill, &r);
            assert_eq!(value(&r, Field::Speed), 1.0 / (0.5 / 1.2), "{stat}");
        }
        // As the global lines, the keyword's crit chance and multiplier.
        for (keyword, global) in [
            (
                "cold_critical_strike_multiplier_+",
                "base_critical_strike_multiplier_+",
            ),
            (
                "cold_critical_strike_chance_+%",
                "critical_strike_chance_+%",
            ),
        ] {
            let r = with_lines(skill, &[(keyword, 30)]);
            as_bare(skill, &r);
            same(&r, &with_lines(skill, &[(global, 30)]));
            assert_ne!(
                value(&r, Field::AverageDamage),
                value(&bare, Field::AverageDamage)
            );
        }
        // Other keywords, and attack skills' damage, do nothing.
        for stat in [
            "cast_speed_for_fire_skills_+%",
            "cast_speed_for_lightning_skills_+%",
            "cast_speed_for_chaos_skills_+%",
            "fire_critical_strike_multiplier_+",
            "lightning_critical_strike_chance_+%",
            "chaos_critical_strike_multiplier_+",
            "cold_damage_with_attack_skills_+%",
            "physical_damage_with_attack_skills_+%",
        ] {
            let r = with_lines(skill, &[(stat, 50)]);
            as_bare(skill, &r);
            same(&r, &bare);
        }
    }
    // A spell's damage of its hit's type, as the plain line.
    for (skill, keyword, global) in [
        (
            "IceNova",
            "cold_damage_with_spell_skills_+%",
            "cold_damage_+%",
        ),
        (
            "TestPhysicalNova",
            "physical_damage_with_spell_skills_+%",
            "physical_damage_+%",
        ),
    ] {
        let r = with_lines(skill, &[(keyword, 40)]);
        as_bare(skill, &r);
        same(&r, &with_lines(skill, &[(global, 40)]));
        assert_ne!(
            value(&r, Field::AverageDamage),
            value(&with_lines(skill, &[]), Field::AverageDamage)
        );
    }
    // The physical nova is a cold skill but deals no cold damage.
    same(
        &with_lines(
            "TestPhysicalNova",
            &[("cold_damage_with_spell_skills_+%", 40)],
        ),
        &with_lines("TestPhysicalNova", &[]),
    );
}

// ---------------------------------------------------------------- P3-8m

/// Resolute Technique and Elemental Overload on a spell (P3-8m,
/// `poe1::keystones`): never crit gives a crit chance and multiplier of 0
/// (PoB's `NeverCrit`); no extra crit damage a multiplier of 1
/// (`NoCritMultiplier`); Elemental Overload's 40% more elemental damage
/// needs a crit chance above 0 (`CritInPast8Sec`, set before the hit).
#[test]
fn keystones_should_change_the_spells_hit_as_pob_does() {
    let complete = |r: &CalcResult| {
        for f in HIT {
            assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
        }
    };
    let r = ice_nova_with(&[("resolute_technique", 1)], &[]);
    complete(&r);
    assert_eq!(value(&r, Field::CritChance), 0.0);
    assert_eq!(value(&r, Field::CritMultiplier), 0.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 0.0, 0.0, 50.0).to_bits()
    );
    let r = ice_nova_with(&[("keystone_elemental_overload", 1)], &[]);
    complete(&r);
    assert_eq!(value(&r, Field::CritChance), 5.0);
    assert_eq!(value(&r, Field::CritMultiplier), 1.0);
    // round(100 x 1.4), round(200 x 1.4).
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(140.0, 280.0, 5.0, 1.0, 50.0).to_bits()
    );
    // Never crit: no crit in the past 8 seconds, so no more damage; the
    // multiplier is 0 (`NeverCrit` first).
    let r = ice_nova_with(
        &[
            ("keystone_elemental_overload", 1),
            ("resolute_technique", 1),
        ],
        &[],
    );
    complete(&r);
    assert_eq!(value(&r, Field::CritMultiplier), 0.0);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 0.0, 0.0, 50.0).to_bits()
    );
}

/// Groups, custom lines, config inputs, and the cold damage taken
/// expected.
type ChillCase<'a> = (
    Vec<SocketGroup>,
    Vec<(&'a str, i64)>,
    Vec<(&'a str, ConfigValue)>,
    f64,
);

/// Bitter Frost (P3-8m): the hit deals no non-cold damage (added fire
/// damage is skipped), and a chilled enemy takes cold damage increased by
/// the chill: the configured effect, or a chilling skill's guaranteed
/// chill (10 times the character's chill effect), the larger, at most 30,
/// floored (PoB's `CalcPerform.lua` 1393 to 1399, 3438 to 3546).
#[test]
fn bitter_frost_should_chill_and_deal_only_cold_damage_as_pob_does() {
    let on = || ConfigValue::Boolean(true);
    let n = |v: f64| ConfigValue::Number(Sourced::new(loom_domain::Number(v)));
    let chilled = |effect: f64| {
        vec![
            ("conditionEnemyChilled", on()),
            ("conditionEnemyChilledEffect", n(effect)),
        ]
    };
    let keystone = ("keystone_cold_purist", 1);
    // Cold damage taken increased by `taken`.
    let expected = |taken: f64| {
        let effect = (1.0 + taken / 100.0) * (1.0 - 50.0 / 100.0);
        let hit = (100.0 / 2.0 + 200.0 / 2.0) * effect;
        let crit_hit = ((100.0 * 1.5) / 2.0 + (200.0 * 1.5) / 2.0) * effect;
        let average_hit = hit * (1.0 - 5.0 / 100.0) + crit_hit * 5.0 / 100.0;
        average_hit * 100.0 / 100.0
    };
    let nova = || group(&["IceNova"]);
    let frostblink = || group(&["Frostblink"]);
    let cases: Vec<ChillCase<'_>> = vec![
        (vec![nova()], vec![keystone], chilled(20.0), 20.0),
        // At most 30.
        (vec![nova()], vec![keystone], chilled(40.0), 30.0),
        // Chilled with no effect: 0.
        (
            vec![nova()],
            vec![keystone],
            vec![("conditionEnemyChilled", on())],
            0.0,
        ),
        // The effect counts only with the check.
        (
            vec![nova()],
            vec![keystone],
            vec![("conditionEnemyChilledEffect", n(20.0))],
            0.0,
        ),
        // Without the keystone the chill only slows.
        (vec![nova()], vec![], chilled(20.0), 0.0),
        // A chilling skill chills without the config: 10.
        (vec![nova(), frostblink()], vec![keystone], vec![], 10.0),
        // Scaled by the character's chill effect: 10 x 1.25, floored.
        (
            vec![nova(), frostblink()],
            vec![keystone, ("chill_effect_+%", 25)],
            vec![],
            12.0,
        ),
        // The three effect stats add up: 10 x 1.65.
        (
            vec![nova(), frostblink()],
            vec![
                keystone,
                ("chill_effect_+%", 5),
                ("cold_ailment_effect_+%", 10),
                ("non_damaging_ailment_effect_+%", 50),
            ],
            vec![],
            16.0,
        ),
        // The larger of the configured and the guaranteed chill.
        (
            vec![nova(), frostblink()],
            vec![keystone],
            chilled(20.0),
            20.0,
        ),
        (
            vec![nova(), frostblink()],
            vec![keystone],
            chilled(5.0),
            10.0,
        ),
        // A disabled group creates no skill.
        (
            vec![nova(), {
                let mut g = frostblink();
                g.enabled = false;
                g
            }],
            vec![keystone],
            vec![],
            0.0,
        ),
        // "Damage taken increased by Chill Effect" acts on cold damage too.
        (
            vec![nova()],
            vec![(
                "enemies_chilled_by_hits_take_damage_increased_by_chill_effect",
                1,
            )],
            chilled(20.0),
            20.0,
        ),
    ];
    for (groups, lines, inputs, taken) in cases {
        let r = groups_with(groups, &lines, &inputs);
        for f in HIT {
            assert_eq!(
                r.fields[&f].state,
                FieldState::Complete,
                "{f} {lines:?} {inputs:?}"
            );
        }
        assert_eq!(
            value(&r, Field::AverageDamage).to_bits(),
            expected(taken).to_bits(),
            "{lines:?} {inputs:?}"
        );
    }
    // No non-cold damage: added fire damage is skipped.
    let fire = [
        ("spell_minimum_added_fire_damage", 10),
        ("spell_maximum_added_fire_damage", 20),
    ];
    let with_fire = ice_nova_with(&fire, &[]);
    assert!(value(&with_fire, Field::AverageDamage) > expected(0.0));
    let r = ice_nova_with(&[fire[0], fire[1], keystone], &[]);
    assert_eq!(r.fields[&Field::AverageDamage].state, FieldState::Complete);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(0.0).to_bits()
    );
    // Every type's damage taken: the fire damage takes it too.
    let every = (
        "enemies_chilled_by_hits_take_damage_increased_by_chill_effect",
        1,
    );
    let r = ice_nova_with(&[fire[0], fire[1], every], &chilled(20.0));
    let ratio = value(&r, Field::AverageDamage) / value(&with_fire, Field::AverageDamage);
    assert!((ratio - 1.2).abs() < 1e-12, "{ratio}");
    // Both flags: every type's first (PoB's order), so the fire damage
    // is not dealt (Bitter Frost) and the cold takes the chill once.
    let r = ice_nova_with(&[fire[0], fire[1], every, keystone], &chilled(20.0));
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(20.0).to_bits()
    );
    // A tie between the configured and a guaranteed chill: the chill once.
    let r = groups_with(vec![nova(), frostblink()], &[keystone], &chilled(10.0));
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        expected(10.0).to_bits()
    );
    // A stat that would change the chill otherwise stays a reason.
    let r = ice_nova_with(&[keystone, ("chill_minimum_slow_%", 20)], &chilled(10.0));
    assert!(matches!(
        r.fields[&Field::AverageDamage].state,
        FieldState::Partial(_)
    ));
}

// ------------------------------------------------- other gems (P3-close)

fn complete(r: &CalcResult) {
    for f in HIT {
        assert_eq!(r.fields[&f].state, FieldState::Complete, "{f}");
    }
}

/// A hex lowers the enemy's resistance, scaled by curse effect (PoB's
/// `CalcPerform.lua` 2601 to 2632).
#[test]
fn a_curse_should_lower_the_enemys_resistance_scaled_by_curse_effect() {
    let r = run(&build(vec![group(&["IceNova"]), group(&["Frostbite"])]));
    complete(&r);
    // 50 - 20.
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 5.0, 1.5, 30.0).to_bits()
    );
    // 50% increased curse effect: -20 x 1.5 = -30.
    let r = groups_with(
        vec![group(&["IceNova"]), group(&["Frostbite"])],
        &[("curse_effect_+%", 50)],
        &[],
    );
    complete(&r);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 5.0, 1.5, 20.0).to_bits()
    );
}

/// A debuff's exposure lowers the enemy's resistance; the strongest
/// exposure wins over the config's (PoB's `CalcPerform.lua` 3563 to 3588).
#[test]
fn a_debuffs_exposure_should_win_when_strongest() {
    let r = run(&build(vec![group(&["IceNova"]), group(&["FrostBomb"])]));
    complete(&r);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 5.0, 1.5, 35.0).to_bits()
    );
    // The config's -10 exposure is weaker: still -15.
    let r = groups_with(
        vec![group(&["IceNova"]), group(&["FrostBomb"])],
        &[(
            "chance_to_inflict_cold_exposure_on_hit_with_cold_damage_%",
            10,
        )],
        &[("conditionEnemyColdExposure", ConfigValue::Boolean(true))],
    );
    complete(&r);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(100.0, 200.0, 5.0, 1.5, 35.0).to_bits()
    );
}

/// A golem's buff reaches the character, scaled by the golems' buff
/// effect (PoB's `CalcPerform.lua` 2266 to 2297).
#[test]
fn a_golems_buff_should_scale_with_golem_buff_effect() {
    let r = run(&build(vec![
        group(&["IceNova"]),
        group(&["SummonFlameGolem"]),
    ]));
    complete(&r);
    // 20% increased damage.
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(120.0, 240.0, 5.0, 1.5, 50.0).to_bits()
    );
    // 50% increased golem buff effect: 30%.
    let r = groups_with(
        vec![group(&["IceNova"]), group(&["SummonFlameGolem"])],
        &[("golem_buff_effect_+%", 50)],
        &[],
    );
    complete(&r);
    assert_eq!(
        value(&r, Field::AverageDamage).to_bits(),
        average(130.0, 260.0, 5.0, 1.5, 50.0).to_bits()
    );
    // The buff's source is the golem's gem.
    assert!(r.graph.iter().any(|(_, n, _)| matches!(
        n,
        loom_calc::Node::Contribution { source: Source::Gem { group: 1, .. }, value, .. }
            if *value == 30.0
    )));
}

/// Blood Stance's more physical damage taken applies unless the config's
/// stance is Sand (PoB's `CalcSetup.lua` 538).
#[test]
fn blood_stance_should_raise_physical_damage_taken_unless_sand() {
    let bare = run(&build(vec![group(&["TestPhysicalNova"])]));
    let blood = run(&build(vec![
        group(&["TestPhysicalNova"]),
        group(&["FleshAndStone"]),
    ]));
    let sand = groups_with(
        vec![group(&["TestPhysicalNova"]), group(&["FleshAndStone"])],
        &[],
        &[("bloodSandStance", ConfigValue::String("SAND".to_owned()))],
    );
    let (b, x, s) = (
        value(&bare, Field::AverageDamage),
        value(&blood, Field::AverageDamage),
        value(&sand, Field::AverageDamage),
    );
    assert!(((x / b) - 1.1).abs() < 1e-12, "{b} {x}");
    assert_eq!(s.to_bits(), b.to_bits());
    assert_eq!(
        blood.fields[&Field::AverageDamage].state.kind(),
        bare.fields[&Field::AverageDamage].state.kind()
    );
}

/// Herald of Ash's physical damage gained as extra fire is added to the
/// hit as fire, against the enemy's fire resistance (PoB's
/// `CalcOffence.lua` 1961 to 2010).
#[test]
fn a_heralds_gain_as_extra_should_add_fire() {
    let bare = run(&build(vec![group(&["TestPhysicalNova"])]));
    let herald = run(&build(vec![
        group(&["TestPhysicalNova"]),
        group(&["HeraldOfAsh"]),
    ]));
    let gained = value(&herald, Field::AverageDamage) - value(&bare, Field::AverageDamage);
    // 15% of 100 to 200: 15 to 30 fire, at the boss's 50% fire resistance.
    let fire = average(15.0, 30.0, 5.0, 1.5, 50.0);
    assert!((gained - fire).abs() < 1e-9, "{gained} {fire}");
}

//! The main skill's hit as an attack with the weapon in the main hand
//! (Phase 3 plan, step 10): the weapon's values, PoB's skill parts,
//! strength's melee physical damage, conversion, accuracy and the chance
//! to hit, the enemy's armour, and which attacks the engine computes.
//!
//! The data is made up (an attack with Smite's shape under its ID, so its
//! PoB parts apply; a one-handed axe); the expected values follow PoB
//! v2.67.2's formulas (module docs of `loom_calc::poe1::hit`). The mechanic
//! fixtures check the same against the oracle (`docs/phase-3/attack-hits.md`).

use loom_calc::{
    ActiveSkill, BaseName, CalcData, CalcInput, CalcOptions, CalcResult, CharacterBase,
    EffectLevel, Field, FieldState, GemEffect, GrantedEffect, ItemBase, PassiveLinks, Range,
    SkillGem, StatAmount, StatSet, StatSetLevel, StateKind, UnsupportedReason, WeaponBase,
    calculate, math,
};
use loom_data::bundle::skills::{ConstantStat, LevelStat};
use loom_domain::{
    Attribute, Build, ClassId, ConfigInput, ConfigSet, ConfigSetId, ConfigValue, DataVersion, Game,
    Gem, GemId, Item, ItemBody, ItemId, ItemLine, ItemSet, ItemSetId, ItemsEntry, LineKind,
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

/// An attack effect: no cooldown, an attack speed multiplier of -20%.
fn effect(id: &str, active: &str) -> GrantedEffect {
    GrantedEffect {
        id: id.to_owned(),
        support: false,
        active_skill: Some(active.to_owned()),
        cast_time: 1000,
        allowed_types: Vec::new(),
        added_types: Vec::new(),
        excluded_types: Vec::new(),
        added_minion_types: Vec::new(),
        supports_gems_only: false,
        cannot_be_supported: false,
        stat_sets: vec![id.to_owned()],
        levels: vec![EffectLevel {
            level: 1,
            costs: Vec::new(),
            cost_multiplier: 100,
            cooldown: 0,
            stored_uses: 0,
            attack_speed_multiplier: -20,
            attack_time: 0,
            reservations: Vec::new(),
        }],
        quality: Vec::new(),
    }
}

/// Smite's shape: 50% of physical converted to lightning, 20% less area
/// damage; base multiplier and effectiveness 100%.
fn stat_set(id: &str) -> StatSet {
    // `TestOverStrike` converts 150%, which PoB scales to 100%.
    let conversion = if id == "TestOverStrike" { 150 } else { 50 };
    StatSet {
        id: id.to_owned(),
        implicit_stats: Vec::new(),
        constant_stats: vec![ConstantStat {
            stat: "skill_physical_damage_%_to_convert_to_lightning".to_owned(),
            value: conversion,
        }],
        base_effectiveness: 1.0,
        incremental_effectiveness: 0.0,
        damage_incremental_effectiveness: None,
        levels: vec![StatSetLevel {
            level: 1,
            spell_crit_chance: 0,
            attack_crit_chance: 0,
            base_multiplier: 0,
            damage_effectiveness: Some(0),
            float_stats: Vec::new(),
            stats: vec![LevelStat {
                stat: "active_skill_area_damage_+%_final".to_owned(),
                value: -20,
                interpolation: Some(1),
            }],
            flags: Vec::new(),
            granted_effects: Vec::new(),
        }],
    }
}

fn active(id: &str, classes: &[&str]) -> ActiveSkill {
    ActiveSkill {
        id: id.to_owned(),
        name: id.to_owned(),
        types: ["Attack", "Damage", "Melee", "Area"]
            .iter()
            .map(|t| (*t).to_owned())
            .collect(),
        minion_types: Vec::new(),
        weapon_classes: classes.iter().map(|c| (*c).to_owned()).collect(),
        manually_cast: true,
    }
}

fn gem_row(id: &str) -> (SkillGem, GemEffect) {
    (
        SkillGem {
            base_item: format!("Test/Gems/{id}"),
            strength: 100,
            dexterity: 0,
            intelligence: 0,
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

/// A Templar (20 strength, 10 dexterity), the constants the hit reads, and
/// these skills: `Smite` (PoB's parts apply) and `TestStrike` (the same
/// data, not vetted), both for one-handed axes; `TestMaceStrike` for maces
/// only. Bases: `Test Axe` (one-handed axe, 10 to 20, 800 ms, 5%) and
/// `Test Shield`.
struct Data {
    effects: Vec<GrantedEffect>,
    sets: Vec<StatSet>,
    actives: Vec<ActiveSkill>,
    gems: Vec<SkillGem>,
    gem_effects: Vec<GemEffect>,
}

impl Data {
    fn new() -> Self {
        let ids = ["Smite", "TestStrike", "TestMaceStrike", "TestOverStrike"];
        let (gems, gem_effects) = ids.iter().map(|id| gem_row(id)).unzip();
        Self {
            effects: vec![
                effect("Smite", "smite"),
                effect("TestStrike", "smite"),
                effect("TestMaceStrike", "mace-strike"),
                effect("TestOverStrike", "smite"),
            ],
            sets: ids.iter().map(|id| stat_set(id)).collect(),
            actives: vec![
                active("smite", &["One Hand Axe"]),
                active("mace-strike", &["One Hand Mace"]),
            ],
            gems,
            gem_effects,
        }
    }
}

impl CalcData for Data {
    fn data_version(&self) -> DataVersion {
        version()
    }
    fn character(&self, class: &str) -> Option<CharacterBase> {
        (class == "Templar").then_some(CharacterBase {
            base_life: 38,
            base_mana: 34,
            strength: 20,
            dexterity: 10,
            intelligence: 10,
        })
    }
    fn character_constant(&self, stat: &str) -> Option<i64> {
        match stat {
            "life_per_level" => Some(12),
            "mana_per_level" => Some(6),
            "accuracy_rating_per_level" => Some(2),
            "base_maximum_all_resistances_%" | "maximum_block_%" | "base_maximum_spell_block_%" => {
                Some(75)
            }
            "base_evasion_rating" => Some(15),
            "base_critical_strike_multiplier" => Some(150),
            _ => None,
        }
    }
    fn monster_constant(&self, stat: &str) -> Option<i64> {
        match stat {
            "base_maximum_all_resistances_%" | "maximum_physical_damage_reduction_%" => Some(75),
            _ => None,
        }
    }
    fn item_base(&self, name: &str) -> Option<ItemBase> {
        match name {
            "Test Axe" => Some(ItemBase::Weapon(WeaponBase {
                class: "One Hand Axe".to_owned(),
                physical: Range { min: 10, max: 20 },
                attack_time: 800,
                crit_chance: 500,
            })),
            "Test Mace" => Some(ItemBase::Weapon(WeaponBase {
                class: "One Hand Mace".to_owned(),
                physical: Range { min: 10, max: 20 },
                attack_time: 800,
                crit_chance: 500,
            })),
            // The Q11 check's attack time (Siege Axe, 667 ms).
            "Test Siege Axe" => Some(ItemBase::Weapon(WeaponBase {
                class: "One Hand Axe".to_owned(),
                physical: Range { min: 10, max: 20 },
                attack_time: 667,
                crit_chance: 500,
            })),
            "Broken Axe" => Some(ItemBase::Weapon(WeaponBase {
                class: "One Hand Axe".to_owned(),
                physical: Range { min: 10, max: 20 },
                attack_time: 0,
                crit_chance: 500,
            })),
            "Test Shield" => Some(ItemBase::Armour(loom_calc::ArmourBase {
                block: Some(20),
                ..loom_calc::ArmourBase::default()
            })),
            _ => None,
        }
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

/// A level 1 Templar attacking with `skill` (its `skillPart` when given),
/// holding the items of `slots` (`(slot, base)`), against an enemy
/// configured with `enemy` (armour, evasion, and resistances given, so no
/// monster table is read).
fn build(skill: &str, part: Option<&str>, slots: &[(&str, &str)], enemy: &[(&str, f64)]) -> Build {
    let mut b = Build::new(Game::Poe1);
    b.data = Some(version());
    b.character.class = Some(ClassId::new("Templar"));
    b.character.level = Some(Sourced::new(1));
    b.character.main_socket_group = Some(Sourced::new(1));
    let mut group = SocketGroup {
        label: None,
        enabled: true,
        slot: None,
        main_active_skill: None,
        gems: Ordered::new(),
        attributes: Vec::new(),
    };
    group.gems.push(Gem {
        gem: Some(GemId::new(format!("Test/Gems/{skill}"))),
        skill: Some(SkillId::new(skill)),
        name: None,
        level: Sourced::new(1),
        quality: Sourced::new(0),
        enabled: true,
        attributes: part
            .map(|p| vec![Attribute::new("skillPart", p)])
            .unwrap_or_default(),
    });
    let mut set = SkillSet {
        id: SkillSetId(1),
        title: None,
        groups: Ordered::new(),
        attributes: Vec::new(),
    };
    set.groups.push(group);
    b.skills.sets.push(set);
    let mut items = ItemSet {
        id: ItemSetId(1),
        title: None,
        slots: Ordered::new(),
        attributes: Vec::new(),
    };
    for (i, (slot, _)) in slots.iter().enumerate() {
        let id = ItemId(u32::try_from(i).unwrap_or(0) + 1);
        b.items.entries.push(ItemsEntry::Item(Item {
            id,
            body: ItemBody::default(),
            layout: Ordered::new(),
            text_spacing: Vec::new(),
            attributes: Vec::new(),
        }));
        items.slots.push(SlotEntry {
            slot: SlotId::new(*slot),
            item: Some(id),
            attributes: Vec::new(),
        });
    }
    b.items.entries.push(ItemsEntry::Set(items));
    let mut config = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        attributes: Vec::new(),
    };
    for (name, v) in enemy {
        config.inputs.push(ConfigInput {
            name: (*name).to_owned(),
            value: ConfigValue::Number(Sourced::new(loom_domain::Number(*v))),
            kind: loom_domain::ConfigInputKind::Input,
            attributes: Vec::new(),
        });
    }
    b.config.sets.push(config);
    b
}

/// Gives item `n` (1-based) these resolved stat lines.
fn with_lines(mut b: Build, n: u32, stats: &[(&str, i64)]) -> Build {
    for entry in b.items.entries.typed_mut() {
        if let ItemsEntry::Item(item) = entry
            && item.id == ItemId(n)
        {
            for (stat, v) in stats {
                item.body.lines.push(ItemLine {
                    source: "a line".to_owned(),
                    kind: LineKind::Modifier(ModifierLine {
                        text: "a line".to_owned(),
                        prefixes: Vec::new(),
                        section: ModSection::Explicit,
                        selected: true,
                        resolution: LineResolution::Resolved {
                            result: Resolved::Stats(vec![StatLine {
                                stat: StatId::new(*stat),
                                value: StatValue::Exact { min: *v, max: *v },
                            }]),
                            adapted: None,
                        },
                    }),
                });
            }
        }
    }
    b
}

/// Sets the item set's second weapon set in use.
fn second_weapon_set(mut b: Build) -> Build {
    for entry in b.items.entries.typed_mut() {
        if let ItemsEntry::Set(set) = entry {
            set.attributes
                .push(Attribute::new("useSecondWeaponSet", "true"));
        }
    }
    b
}

/// An enemy without armour or evasion and no lightning resistance.
const OPEN: [(&str, f64); 3] = [
    ("enemyArmour", 0.0),
    ("enemyEvasion", 0.0),
    ("enemyLightningResist", 0.0),
];

const AXE: [(&str, &str); 1] = [("Weapon 1", "Test Axe")];

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn run(b: &Build, slots: &[(&str, &str)]) -> CalcResult {
    let data = Data::new();
    let mut options = CalcOptions::default();
    for (i, (_, base)) in slots.iter().enumerate() {
        options.item_bases.insert(
            ItemId(u32::try_from(i).unwrap_or(0) + 1),
            BaseName {
                name: (*base).to_owned(),
                fallback: None,
            },
        );
    }
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

/// The reasons that block `Complete` beyond the monster tables this
/// data leaves out (the enemy's placeholders read them even when the
/// config sets the values): none expected.
fn partial(r: &CalcResult, field: Field) -> Vec<&'static str> {
    match &r.fields[&field].state {
        FieldState::Complete => Vec::new(),
        FieldState::Partial(p) => p.iter().map(UnsupportedReason::name).collect(),
        FieldState::Indeterminate { reasons, partial } => {
            assert!(
                reasons.iter().all(|x| x.name() == "missing-data"),
                "{field}: {reasons:?}"
            );
            partial.iter().map(UnsupportedReason::name).collect()
        }
        other @ FieldState::Unsupported(_) => panic!("{field}: {other:?}"),
    }
}

/// A pass's average (`min / 2 + max / 2`) of a type's damage times `all`.
fn pass(min: f64, max: f64, all: f64) -> f64 {
    (min * all) / 2.0 + (max * all) / 2.0
}

// ---------------------------------------------------------------- tests

#[test]
fn a_melee_hit_should_convert_and_take_strengths_bonus() {
    let r = run(&build("Smite", Some("1"), &AXE, &OPEN), &AXE);
    // Accuracy 2 x 1 - 2 + 10 x 2 = 20 against no evasion: 125%, capped.
    assert_eq!(value(&r, Field::HitChance), 100.0);
    assert_eq!(value(&r, Field::CritChance), 5.0);
    assert_eq!(value(&r, Field::CritMultiplier), 1.5);
    // Physical 10 to 20 with floor(20 / 5) = 4% increased melee physical
    // damage: round(10.4) = 10, round(20.8) = 21; half kept. Lightning: the
    // other half of the physical damage (with lightning's modifiers too,
    // none), rounded: round(5) = 5, round(10.5) = 11. No armour, no
    // resistance.
    let hit = pass(10.0, 21.0, 0.5) * 1.0 + pass(5.0, 11.0, 1.0) * 1.0;
    let crit = pass(10.0, 21.0, 0.5 * 1.5) * 1.0 + pass(5.0, 11.0, 1.5) * 1.0;
    let average = (hit * (1.0 - 5.0 / 100.0) + crit * 5.0 / 100.0) * 100.0 / 100.0;
    assert_eq!(value(&r, Field::AverageDamage).to_bits(), average.to_bits());
    // 1 / (1 / (1000 / floor(800 x 100 / 100) x (1 - 20 / 100)) / 1) = 1.
    assert_eq!(value(&r, Field::Speed), 1.0);
    assert_eq!(value(&r, Field::HitDps), average);
    for f in [
        Field::HitChance,
        Field::CritChance,
        Field::CritMultiplier,
        Field::AverageDamage,
        Field::Speed,
        Field::HitDps,
    ] {
        assert!(partial(&r, f).is_empty(), "{f}");
    }
    assert_eq!(
        r.fields[&Field::HitSpeed].state.kind(),
        StateKind::Unsupported
    );
}

#[test]
fn an_area_hit_should_take_the_area_more_and_not_the_melee_bonus() {
    let r = run(&build("Smite", Some("2"), &AXE, &OPEN), &AXE);
    // 20% less area damage: 8 to 16, half kept; lightning 4 to 8.
    let hit = pass(8.0, 16.0, 0.5) + pass(4.0, 8.0, 1.0);
    let crit = pass(8.0, 16.0, 0.5 * 1.5) + pass(4.0, 8.0, 1.5);
    let average = (hit * (1.0 - 5.0 / 100.0) + crit * 5.0 / 100.0) * 100.0 / 100.0;
    assert_eq!(value(&r, Field::AverageDamage).to_bits(), average.to_bits());
    // A part above the count is the last (PoB's `m_min`); none is part 1.
    let last = run(&build("Smite", Some("5"), &AXE, &OPEN), &AXE);
    assert_eq!(
        value(&last, Field::AverageDamage),
        value(&r, Field::AverageDamage)
    );
    let first = run(&build("Smite", None, &AXE, &OPEN), &AXE);
    let melee = run(&build("Smite", Some("1"), &AXE, &OPEN), &AXE);
    assert_eq!(
        value(&first, Field::AverageDamage),
        value(&melee, Field::AverageDamage)
    );
}

#[test]
fn the_hit_chance_should_follow_the_enemys_evasion() {
    let enemy = [
        ("enemyArmour", 0.0),
        ("enemyEvasion", 1000.0),
        ("enemyLightningResist", 0.0),
    ];
    let r = run(&build("Smite", Some("1"), &AXE, &enemy), &AXE);
    // round(20 / (20 + (1000 / 5) ^ 0.9) x 125), at least 5.
    let chance = (20.0 / (20.0 + math::pow(1000.0 / 5.0, 0.9)) * 125.0 + 0.5)
        .floor()
        .clamp(5.0, 100.0);
    assert_eq!(value(&r, Field::HitChance), chance);
    assert_eq!(value(&r, Field::HitChance), 18.0);
    // Crit chance and average damage both scale by it.
    assert_eq!(value(&r, Field::CritChance), 5.0 * 18.0 / 100.0);
    let open = run(&build("Smite", Some("1"), &AXE, &OPEN), &AXE);
    assert!(value(&r, Field::AverageDamage) < value(&open, Field::AverageDamage) * 0.19);
}

#[test]
fn the_enemys_armour_should_reduce_physical_damage_per_pass() {
    let enemy = [
        ("enemyArmour", 100.0),
        ("enemyEvasion", 0.0),
        ("enemyLightningResist", 0.0),
    ];
    let r = run(&build("Smite", Some("1"), &AXE, &enemy), &AXE);
    // `armour / (armour + hit x 5) x 100`, at most 75, per pass.
    let effect = |hit: f64| {
        let reduction = (100.0 / (100.0 + hit * 5.0) * 100.0).clamp(0.0, 75.0);
        1.0 - reduction / 100.0
    };
    let physical = pass(10.0, 21.0, 0.5);
    let physical_crit = pass(10.0, 21.0, 0.5 * 1.5);
    let hit = physical * effect(physical) + pass(5.0, 11.0, 1.0);
    let crit = physical_crit * effect(physical_crit) + pass(5.0, 11.0, 1.5);
    let average = (hit * (1.0 - 5.0 / 100.0) + crit * 5.0 / 100.0) * 100.0 / 100.0;
    assert_eq!(value(&r, Field::AverageDamage).to_bits(), average.to_bits());
}

#[test]
fn only_attacks_with_a_usable_main_hand_weapon_should_be_computed() {
    let not_computed = |r: &CalcResult| {
        [Field::HitChance, Field::AverageDamage, Field::Speed]
            .iter()
            .all(|f| r.fields[f].state.kind() == StateKind::Unsupported)
    };
    // No weapon (unarmed is not modelled).
    assert!(not_computed(&run(&build("Smite", None, &[], &OPEN), &[])));
    // Dual wielding.
    let two = [("Weapon 1", "Test Axe"), ("Weapon 2", "Test Axe")];
    assert!(not_computed(&run(&build("Smite", None, &two, &OPEN), &two)));
    // A weapon the skill cannot use.
    assert!(not_computed(&run(
        &build("TestMaceStrike", None, &AXE, &OPEN),
        &AXE
    )));
    // A shield in the off hand is fine.
    let shield = [("Weapon 1", "Test Axe"), ("Weapon 2", "Test Shield")];
    let r = run(&build("Smite", None, &shield, &OPEN), &shield);
    assert!(partial(&r, Field::AverageDamage).is_empty());
}

#[test]
fn an_attack_the_oracle_has_not_checked_should_be_partial() {
    let r = run(&build("TestStrike", None, &AXE, &OPEN), &AXE);
    assert_eq!(partial(&r, Field::AverageDamage), ["unmodelled-skill"]);
    assert!(format!("{:?}", r.fields[&Field::AverageDamage].state).contains("pob-skill-data"));
    // Without PoB's parts its flags are the game's (melee, and no
    // `is_area_damage`): the melee hit's value.
    let smite = run(&build("Smite", Some("1"), &AXE, &OPEN), &AXE);
    assert_eq!(
        value(&r, Field::AverageDamage),
        value(&smite, Field::AverageDamage)
    );
}

#[test]
fn the_weapons_local_lines_should_act_on_it() {
    // 50% increased physical damage (its "No Physical Damage" stat at 0, as
    // the canonical line carries it): the weapon round(10 x 1.5) = 15,
    // round(20 x 1.5) = 30; the hit with strength's 4%: round(15.6) = 16,
    // round(31.2) = 31; half converted: round(8) = 8, round(15.5) = 16.
    let lines = [
        ("local_physical_damage_+%", 50),
        ("local_weapon_no_physical_damage", 0),
    ];
    let b = with_lines(build("Smite", Some("1"), &AXE, &OPEN), 1, &lines);
    let r = run(&b, &AXE);
    assert!(partial(&r, Field::AverageDamage).is_empty());
    let hit = pass(16.0, 31.0, 0.5) + pass(8.0, 16.0, 1.0);
    let crit = pass(16.0, 31.0, 0.5 * 1.5) + pass(8.0, 16.0, 1.5);
    let average = (hit * (1.0 - 5.0 / 100.0) + crit * 5.0 / 100.0) * 100.0 / 100.0;
    assert_eq!(value(&r, Field::AverageDamage).to_bits(), average.to_bits());
    // "No Physical Damage" itself is not modelled: a reason.
    let b = with_lines(
        build("Smite", Some("1"), &AXE, &OPEN),
        1,
        &[("local_weapon_no_physical_damage", 1)],
    );
    let r = run(&b, &AXE);
    assert_eq!(partial(&r, Field::AverageDamage), ["unmodelled-item"]);
    assert!(format!("{:?}", r.fields[&Field::AverageDamage].state).contains("no-physical-damage"));
}

#[test]
fn the_weapons_attack_time_should_be_floored_to_whole_milliseconds() {
    // The game's model (Q11, a `game-truth` divergence): 800 ms with 15%
    // increased local attack speed is floor(80000 / 115) = floor(695.65) =
    // 695 ms, 1000 / 695 = 1.4388 attacks per second. PoB's round(1.25 x
    // 1.15, 2) = 1.44 is not taken.
    let b = with_lines(
        build("Smite", Some("1"), &AXE, &OPEN),
        1,
        &[("local_attack_speed_+%", 15)],
    );
    let r = run(&b, &AXE);
    assert!(partial(&r, Field::Speed).is_empty());
    let weapon: f64 = 1000.0 / 695.0;
    // Smite's 20% less attack speed (its multiplier), then no global
    // modifier: round(1 x 1, 2) = 1.
    let speed = 1.0 / (1.0 / (weapon * (1.0 + -20.0 / 100.0)) / 1.0);
    assert_eq!(value(&r, Field::Speed).to_bits(), speed.to_bits());
    let pob = 1.0 / (1.0 / (1.44 * (1.0 + -20.0 / 100.0)) / 1.0);
    assert!((value(&r, Field::Speed) - pob).abs() > 1e-4);
    // An exact quotient stays exact: 800 ms with 25% is 640 ms.
    let b = with_lines(
        build("Smite", Some("1"), &AXE, &OPEN),
        1,
        &[("local_attack_speed_+%", 25)],
    );
    let r = run(&b, &AXE);
    let speed: f64 = 1.0 / (1.0 / (1000.0 / 640.0 * (1.0 + -20.0 / 100.0)) / 1.0);
    assert_eq!(value(&r, Field::Speed).to_bits(), speed.to_bits());

    // The in-game check's weapon: 667 ms with 15% is 66700 / 115 = 580 ms,
    // 1000 / 580 = 1.7241 (the tooltip's 1.72; PoB's 1.73).
    let siege = [("Weapon 1", "Test Siege Axe")];
    let at = |local: i64| {
        let b = with_lines(
            build("Smite", Some("1"), &siege, &OPEN),
            1,
            &[("local_attack_speed_+%", local)],
        );
        value(&run(&b, &siege), Field::Speed)
    };
    let smite = |ms: f64| -> f64 { 1.0 / (1.0 / (1000.0 / ms * (1.0 + -20.0 / 100.0)) / 1.0) };
    assert_eq!(at(15).to_bits(), smite(580.0).to_bits());
    // A reduction, where dividing by the factor would floor wrongly: 667 /
    // (1 - 42 / 100) is 1149.99... in doubles, 66700 / 58 is exactly 1150.
    assert!((667.0_f64 / (1.0 + -42.0 / 100.0)).floor() < 1150.0);
    assert_eq!(at(-42).to_bits(), smite(1150.0).to_bits());
}

#[test]
fn the_weapon_set_in_use_should_hold_the_weapon() {
    let swap = [("Weapon 1 Swap", "Test Axe")];
    let open = run(&build("Smite", Some("1"), &AXE, &OPEN), &AXE);
    let r = run(
        &second_weapon_set(build("Smite", Some("1"), &swap, &OPEN)),
        &swap,
    );
    assert_eq!(
        value(&r, Field::AverageDamage),
        value(&open, Field::AverageDamage)
    );
    // The swap set not in use: no weapon.
    let r = run(&build("Smite", Some("1"), &swap, &OPEN), &swap);
    assert_eq!(
        r.fields[&Field::AverageDamage].state.kind(),
        StateKind::Unsupported
    );
}

#[test]
fn an_off_hand_of_unknown_base_or_a_bad_part_should_not_be_computed() {
    let unknown = [("Weapon 1", "Test Axe"), ("Weapon 2", "Mystery")];
    let r = run(&build("Smite", None, &unknown, &OPEN), &unknown);
    assert_eq!(
        r.fields[&Field::HitChance].state.kind(),
        StateKind::Unsupported
    );
    // Part 0: PoB would index a part that does not exist.
    let r = run(&build("Smite", Some("0"), &AXE, &OPEN), &AXE);
    assert_eq!(
        r.fields[&Field::HitChance].state.kind(),
        StateKind::Unsupported
    );
    // Not a number: part 1, as PoB's `tonumber` gives nil.
    let r = run(&build("Smite", Some("x"), &AXE, &OPEN), &AXE);
    let melee = run(&build("Smite", Some("1"), &AXE, &OPEN), &AXE);
    assert_eq!(
        value(&r, Field::AverageDamage),
        value(&melee, Field::AverageDamage)
    );
}

#[test]
fn conversion_above_100_should_be_scaled_to_100() {
    let r = run(&build("TestOverStrike", None, &AXE, &OPEN), &AXE);
    // `skillConv x 100 / skillTotal`, then `/ 100`; physical keeps 0.
    let share = 150.0 * (100.0 / 150.0) / 100.0;
    let lightning = |m: f64| (m * share + 0.5).floor();
    let (min, max) = (lightning(10.0), lightning(21.0));
    let hit = pass(10.0, 21.0, 0.0) + pass(min, max, 1.0);
    let crit = pass(10.0, 21.0, 0.0) + pass(min, max, 1.5);
    let average = (hit * (1.0 - 5.0 / 100.0) + crit * 5.0 / 100.0) * 100.0 / 100.0;
    assert_eq!(value(&r, Field::AverageDamage).to_bits(), average.to_bits());
}

#[test]
fn another_weapon_class_and_a_base_without_attack_time() {
    let mace = [("Weapon 1", "Test Mace")];
    let r = run(&build("TestMaceStrike", None, &mace, &OPEN), &mace);
    assert_eq!(partial(&r, Field::Speed), ["unmodelled-skill"]);
    assert_eq!(value(&r, Field::Speed), 1.0);
    // Smite cannot use the mace.
    let r = run(&build("Smite", None, &mace, &OPEN), &mace);
    assert_eq!(r.fields[&Field::Speed].state.kind(), StateKind::Unsupported);
    // No attack time: missing data, never an infinite rate taken as known.
    let broken = [("Weapon 1", "Broken Axe")];
    let r = run(&build("Smite", None, &broken, &OPEN), &broken);
    let FieldState::Indeterminate { reasons, .. } = &r.fields[&Field::Speed].state else {
        panic!("not indeterminate");
    };
    assert!(format!("{reasons:?}").contains("attack_time"));
}

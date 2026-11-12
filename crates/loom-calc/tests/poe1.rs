//! The Path of Exile character fields end to end (Phase 3 plan, step 6):
//! attributes, life, mana, energy shield, unreserved life and mana, from
//! the class, per-level growth, passives, mastery effects, items, and
//! jewels; and a reason for each thing not modelled yet, never a silent
//! value.
//!
//! The expected values follow PoB v2.67.2's formulas (module docs of
//! `loom_calc::poe1::phases`); the mechanic fixtures check the same
//! against the oracle (`docs/phase-3/character-fields.md`).

use loom_calc::poe1::Poe1Field;
use loom_calc::{
    ArmourBase, BaseName, CalcData, CalcInput, CalcOptions, CalcResult, CharacterBase, DamageType,
    Field, FieldState, IndeterminateReason, ItemBase, Node, PassiveLinks, Range, Source,
    StatAmount, StateKind, Term, UnsupportedReason, calculate,
};
use loom_domain::poe1::Poe1Choices;
use loom_domain::{
    Attribute, Build, ClassId, ConfigInput, ConfigSet, ConfigSetId, ConfigValue, DataVersion, Game,
    GameSpecific, Gem, Item, ItemBody, ItemId, ItemLine, ItemSet, ItemSetId, ItemsEntry,
    JewelSocket, LineKind, LineResolution, MasteryEffect, ModSection, ModifierLine, NodeId,
    Ordered, PassiveSpec, Resolved, SkillSet, SkillSetId, SlotEntry, SlotId, SocketGroup, Sourced,
    SpecId, StatId, StatLine, StatValue, TreeVersion,
};

// ---------------------------------------------------------------- helpers

fn amount(stat: &str, value: i64) -> StatAmount {
    StatAmount {
        stat: stat.to_owned(),
        value,
    }
}

/// The class start of the small tree: linked to passives 1 to 4; passive
/// 5 is linked to nothing, and 6 is a cluster jewel socket linked to
/// nothing (as one inside a subgraph).
const START: u32 = 100;

/// A small table: the Witch (as the game's characters table), the
/// per-level constants of `Character.ot`, passives 1 to 5 and the start,
/// mastery effect 77.
struct Data {
    passives: Vec<(u32, Vec<StatAmount>)>,
    masteries: Vec<(u32, Vec<StatAmount>)>,
}

impl Data {
    fn new() -> Self {
        Self {
            passives: vec![
                (
                    1,
                    vec![
                        amount("base_maximum_life", 10),
                        amount("maximum_life_+%", 8),
                    ],
                ),
                (2, vec![amount("additional_intelligence", 10)]),
                (3, vec![amount("keystone_unknown_to_the_mapping", 1)]),
                (4, vec![]),
                (5, vec![amount("base_maximum_life", 50)]),
                (6, vec![]),
                (START, vec![]),
            ],
            masteries: vec![(77, vec![amount("maximum_mana_+%", 10)])],
        }
    }
}

fn version() -> DataVersion {
    DataVersion {
        game: Game::Poe1,
        schema_version: 2,
        steam_build: Some(1),
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
            _ => None,
        }
    }
    fn item_base(&self, name: &str) -> Option<ItemBase> {
        let range = |min, max| Range { min, max };
        match name {
            // An armour base with a range, and an energy shield one.
            "Test Plate" => Some(ItemBase::Armour(ArmourBase {
                armour: range(100, 120),
                ..ArmourBase::default()
            })),
            "Test Robe" => Some(ItemBase::Armour(ArmourBase {
                energy_shield: range(50, 50),
                evasion: range(10, 20),
                ..ArmourBase::default()
            })),
            "Test Shield" => Some(ItemBase::Armour(ArmourBase {
                armour: range(10, 10),
                block: Some(24),
                ..ArmourBase::default()
            })),
            "Test Sword" => Some(ItemBase::Weapon),
            "Test Quiver" => Some(ItemBase::Other),
            "Two-Toned Boots" => Some(ItemBase::Ambiguous),
            _ => None,
        }
    }
    fn passive_stats(&self, node: u32) -> Option<&[StatAmount]> {
        self.passives
            .iter()
            .find(|(n, _)| *n == node)
            .map(|(_, s)| s.as_slice())
    }
    fn mastery_effect_stats(&self, effect: u32) -> Option<&[StatAmount]> {
        self.masteries
            .iter()
            .find(|(n, _)| *n == effect)
            .map(|(_, s)| s.as_slice())
    }
    fn passive_links(&self, node: u32) -> Option<PassiveLinks> {
        let links = match node {
            START => vec![1, 2, 3, 4],
            1..=4 => vec![START],
            5 | 6 => vec![],
            _ => return None,
        };
        Some(PassiveLinks {
            root: node == START,
            expansion: node == 6,
            links,
        })
    }
}

fn witch(level: u32) -> Build {
    let mut b = Build::new(Game::Poe1);
    b.data = Some(version());
    b.character.class = Some(ClassId::new("Witch"));
    b.character.level = Some(Sourced::new(level));
    b
}

fn options(tree: Option<&str>) -> CalcOptions {
    CalcOptions {
        tree_version: tree.map(TreeVersion::new),
        ..CalcOptions::default()
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn run_with(b: &Build, options: &CalcOptions) -> CalcResult {
    let data = Data::new();
    let input = CalcInput {
        build: b,
        data: &data,
        options,
    };
    let r = calculate(&input).unwrap();
    r.graph.check().unwrap();
    r
}

fn run(b: &Build) -> CalcResult {
    run_with(b, &options(Some("3_29")))
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure fails the test"
)]
fn value(r: &CalcResult, field: Field) -> f64 {
    r.fields[&field].value.unwrap()
}

fn state(r: &CalcResult, field: Field) -> StateKind {
    r.fields[&field].state.kind()
}

/// The fields P3-6 computes.
const CHARACTER: [Field; 8] = [
    Field::Strength,
    Field::Dexterity,
    Field::Intelligence,
    Field::Life,
    Field::Mana,
    Field::EnergyShield,
    Field::LifeUnreserved,
    Field::ManaUnreserved,
];

/// A spec with the class start and `nodes` allocated.
fn spec(nodes: &[u32]) -> PassiveSpec {
    let mut s = PassiveSpec::new(SpecId(1));
    s.tree_version = Some(TreeVersion::new("3_29"));
    s.nodes = Sourced::new(
        std::iter::once(START)
            .chain(nodes.iter().copied())
            .map(NodeId)
            .collect(),
    );
    s
}

fn with_spec(mut b: Build, s: PassiveSpec) -> Build {
    b.passives.specs.push(s);
    b
}

fn stat(id: &str, v: i64) -> StatLine {
    StatLine {
        stat: StatId::new(id),
        value: StatValue::Exact { min: v, max: v },
    }
}

fn modifier(stats: Vec<StatLine>) -> ItemLine {
    ItemLine {
        source: "a line".to_owned(),
        kind: LineKind::Modifier(ModifierLine {
            text: "a line".to_owned(),
            prefixes: Vec::new(),
            section: ModSection::Explicit,
            selected: true,
            resolution: LineResolution::Resolved {
                result: Resolved::Stats(stats),
                adapted: None,
            },
        }),
    }
}

fn item(id: u32, lines: Vec<ItemLine>) -> Item {
    Item {
        id: ItemId(id),
        body: ItemBody {
            lines,
            ..ItemBody::default()
        },
        layout: Ordered::new(),
        text_spacing: Vec::new(),
        attributes: Vec::new(),
    }
}

/// Adds items and one item set holding `slots` (`(slot, item, active)`).
fn with_items(mut b: Build, items: Vec<Item>, slots: &[(&str, u32, bool)]) -> Build {
    for i in items {
        b.items.entries.push(ItemsEntry::Item(i));
    }
    let mut set = ItemSet {
        id: ItemSetId(1),
        title: None,
        slots: Ordered::new(),
        attributes: Vec::new(),
    };
    for (slot, id, active) in slots {
        set.slots.push(SlotEntry {
            slot: SlotId::new(*slot),
            item: Some(ItemId(*id)),
            attributes: if *active {
                vec![Attribute::new("active", "true")]
            } else {
                Vec::new()
            },
        });
    }
    b.items.entries.push(ItemsEntry::Set(set));
    b
}

fn partial_reasons(r: &CalcResult, field: Field) -> Vec<&'static str> {
    match &r.fields[&field].state {
        FieldState::Partial(p) => p.iter().map(UnsupportedReason::name).collect(),
        other => panic!("{field}: {other:?}"),
    }
}

// ------------------------------------------------------------ base values

#[test]
fn a_bare_character_should_have_its_class_and_level_values() {
    // Level 90 Witch: Str 14, Dex 14, Int 32; Life 38 + 12 * 90 + 14 / 2 =
    // 1125; Mana 34 + 6 * 90 + 32 / 2 = 590; no energy shield.
    let r = run(&witch(90));
    let expect = [
        (Field::Strength, 14.0),
        (Field::Dexterity, 14.0),
        (Field::Intelligence, 32.0),
        (Field::Life, 1125.0),
        (Field::Mana, 590.0),
        (Field::EnergyShield, 0.0),
        (Field::LifeUnreserved, 1125.0),
        (Field::ManaUnreserved, 590.0),
    ];
    for (field, v) in expect {
        assert_eq!(value(&r, field), v, "{field}");
        assert_eq!(state(&r, field), StateKind::Complete, "{field}");
    }
    // The defences (P3-7): evasion 15 x (1 + floor(14 / 5) / 100) = 15.3,
    // 15; no armour; the act 10 penalty on each resistance; no block or
    // suppression.
    let defences = [
        (Field::Evasion, 15.0),
        (Field::Armour, 0.0),
        (Field::Resistance(DamageType::Fire), -60.0),
        (Field::Resistance(DamageType::Cold), -60.0),
        (Field::Resistance(DamageType::Lightning), -60.0),
        (Field::Resistance(DamageType::Chaos), -60.0),
        (Field::BlockChance, 0.0),
        (Field::Poe1(Poe1Field::SpellBlockChance), 0.0),
        (Field::Poe1(Poe1Field::SpellSuppressionChance), 0.0),
    ];
    for (field, v) in defences {
        assert_eq!(value(&r, field), v, "{field}");
        assert_eq!(state(&r, field), StateKind::Complete, "{field}");
    }
    // The other 7 slice fields (the main skill's) are not computed yet.
    let not_computed = r
        .fields
        .values()
        .filter(|f| f.state == FieldState::Unsupported(UnsupportedReason::NotComputed))
        .count();
    assert_eq!(not_computed, 7);
    assert_eq!(r.state, StateKind::Unsupported);
}

#[test]
fn the_level_should_be_clamped_to_1_to_100() {
    assert_eq!(value(&run(&witch(0)), Field::Life), 38.0 + 12.0 + 7.0);
    assert_eq!(value(&run(&witch(200)), Field::Life), 38.0 + 1200.0 + 7.0);
    let mut b = witch(1);
    b.character.level = None;
    assert_eq!(value(&run(&b), Field::Life), 57.0);
}

#[test]
fn an_unknown_class_should_be_indeterminate() {
    let mut b = witch(1);
    b.character.class = Some(ClassId::new("Nobody"));
    let r = run(&b);
    for field in CHARACTER {
        assert_eq!(state(&r, field), StateKind::Indeterminate, "{field}");
    }
    assert!(
        r.reasons
            .indeterminate
            .contains(&IndeterminateReason::MissingData {
                table: "characters",
                key: "Nobody".to_owned()
            })
    );
}

// ---------------------------------------------------------------- sources

#[test]
fn passive_stats_should_reach_life_through_base_and_increased() {
    // (1125 + 10) * 1.08 = 1225.8, rounded half up: 1226.
    let r = run(&with_spec(witch(90), spec(&[1])));
    assert_eq!(value(&r, Field::Life), 1226.0);
    assert_eq!(state(&r, Field::Life), StateKind::Complete);
    assert_eq!(value(&r, Field::LifeUnreserved), 1226.0);
}

#[test]
fn an_attribute_from_a_passive_should_reach_mana_and_energy_shield() {
    // Int 42: mana 34 + 540 + 21 = 595.
    let r = run(&with_spec(witch(90), spec(&[2])));
    assert_eq!(value(&r, Field::Intelligence), 42.0);
    assert_eq!(value(&r, Field::Mana), 595.0);
    // No flat energy shield: no row, 0 whatever the increase.
    assert_eq!(value(&r, Field::EnergyShield), 0.0);
}

#[test]
fn a_mastery_effect_should_count_only_on_an_allocated_mastery() {
    let mut s = spec(&[4]);
    s.mastery_effects = Sourced::new(vec![
        MasteryEffect {
            node: NodeId(4),
            effect: 77,
        },
        MasteryEffect {
            node: NodeId(9),
            effect: 77,
        },
    ]);
    // Mana 590 * 1.1 = 649.
    let r = run(&with_spec(witch(90), s));
    assert_eq!(value(&r, Field::Mana), 649.0);
    assert_eq!(state(&r, Field::Mana), StateKind::Complete);
}

#[test]
fn an_empty_mana_pool_should_have_no_unreserved_mana() {
    // 100% reduced maximum mana: Mana 0; PoB writes no ManaUnreserved.
    let ring = item(1, vec![modifier(vec![stat("maximum_mana_+%", -100)])]);
    let r = run(&with_items(witch(90), vec![ring], &[("Ring 1", 1, false)]));
    assert_eq!(value(&r, Field::Mana), 0.0);
    assert_eq!(state(&r, Field::ManaUnreserved), StateKind::Unsupported);
    assert_eq!(value(&r, Field::LifeUnreserved), 1125.0);
}

#[test]
fn ring_lines_should_reach_attributes_and_global_energy_shield() {
    // Int 32 + 20 = 52; energy shield row 30 * (1 + 5 / 100) = 31.5,
    // rounded half up: 32.
    let ring = item(
        1,
        vec![modifier(vec![
            stat("base_maximum_energy_shield", 30),
            stat("additional_intelligence", 20),
        ])],
    );
    let r = run(&with_items(witch(90), vec![ring], &[("Ring 1", 1, false)]));
    assert_eq!(value(&r, Field::Intelligence), 52.0);
    assert_eq!(value(&r, Field::EnergyShield), 32.0);
    assert_eq!(state(&r, Field::EnergyShield), StateKind::Complete);
    // The row node and its parts, as slice-fields.json names them.
    let es = r.fields[&Field::EnergyShield].node.unwrap();
    let rows: Vec<_> = r
        .graph
        .iter()
        .filter(|(i, n, _)| {
            i.get() < es.get()
                && matches!(
                    n,
                    Node::Product {
                        term: Term::Step("slot.global"),
                        ..
                    }
                )
        })
        .map(|(_, _, v)| v)
        .collect();
    assert_eq!(rows, [31.5]);
}

#[test]
fn a_jewel_should_count_only_in_an_allocated_socket() {
    let jewel = item(5, vec![modifier(vec![stat("additional_strength", 16)])]);
    let socket = |node| JewelSocket {
        node: NodeId(node),
        item: Some(ItemId(5)),
        attributes: Vec::new(),
    };
    let mut s = spec(&[4]);
    s.jewel_sockets.push(socket(4));
    let mut b = with_spec(witch(1), s);
    b.items.entries.push(ItemsEntry::Item(jewel.clone()));
    let r = run(&b);
    assert_eq!(value(&r, Field::Strength), 30.0);
    // Life 50 + floor(30 / 2).
    assert_eq!(value(&r, Field::Life), 65.0);

    let mut s = spec(&[4]);
    s.jewel_sockets.push(socket(8));
    let mut b = with_spec(witch(1), s);
    b.items.entries.push(ItemsEntry::Item(jewel));
    assert_eq!(value(&run(&b), Field::Strength), 14.0);
}

// ------------------------------------------------------------ the gaps

#[test]
fn an_unmapped_stat_should_make_the_fields_partial() {
    let r = run(&with_spec(witch(90), spec(&[3])));
    for field in CHARACTER {
        assert_eq!(partial_reasons(&r, field), ["unmapped-stat"], "{field}");
    }
    // The values are still computed.
    assert_eq!(value(&r, Field::Life), 1125.0);
}

#[test]
fn an_unknown_passive_should_be_indeterminate() {
    let r = run(&with_spec(witch(90), spec(&[999])));
    assert_eq!(state(&r, Field::Life), StateKind::Indeterminate);
    assert!(
        r.reasons
            .indeterminate
            .contains(&IndeterminateReason::MissingData {
                table: "passives",
                key: "999".to_owned()
            })
    );
}

#[test]
fn an_unconnected_node_and_its_mastery_should_not_count() {
    // Node 5 (+50 life) is linked to nothing; PoB drops it on load. Its
    // mastery effect (10% increased mana) does not count either.
    let mut s = spec(&[1, 5]);
    s.mastery_effects = Sourced::new(vec![MasteryEffect {
        node: NodeId(5),
        effect: 77,
    }]);
    let r = run(&with_spec(witch(90), s));
    for field in CHARACTER {
        assert_eq!(
            partial_reasons(&r, field),
            ["unmodelled-passive"],
            "{field}"
        );
    }
    assert!(
        r.reasons
            .partial
            .contains(&UnsupportedReason::UnmodelledPassive {
                source: Source::Passive { node: NodeId(5) },
                what: "unconnected-node",
            })
    );
    // Node 1 counts (1226, as alone); node 5's life and the mastery do not.
    assert_eq!(value(&r, Field::Life), 1226.0);
    assert_eq!(value(&r, Field::Mana), 590.0);
}

#[test]
fn a_cluster_socket_off_the_tree_should_be_a_cluster_reason() {
    let r = run(&with_spec(witch(90), spec(&[6])));
    assert!(
        r.reasons
            .partial
            .contains(&UnsupportedReason::UnmodelledPassive {
                source: Source::Passive { node: NodeId(6) },
                what: "cluster-jewel-node",
            })
    );
    assert_eq!(value(&r, Field::Life), 1125.0);
}

#[test]
fn nodes_without_their_start_should_not_count() {
    // The start is the only root: without it nothing is connected.
    let mut s = spec(&[1]);
    s.nodes = Sourced::new(vec![NodeId(1)]);
    let r = run(&with_spec(witch(90), s));
    assert_eq!(value(&r, Field::Life), 1125.0);
    assert_eq!(partial_reasons(&r, Field::Life), ["unmodelled-passive"]);
}

#[test]
fn a_cluster_jewel_node_should_be_a_reason_not_missing_data() {
    // PoB's subgraph nodes have bit 16 set (65536 + the subgraph's index).
    let r = run(&with_spec(witch(90), spec(&[1, 0x1_0010])));
    for field in CHARACTER {
        assert_eq!(
            partial_reasons(&r, field),
            ["unmodelled-passive"],
            "{field}"
        );
    }
    assert!(r.reasons.indeterminate.is_empty());
    // Node 1 (base life 10, 8% increased) still counts.
    assert_eq!(value(&r, Field::Life), 1226.0);
}

#[test]
fn the_tree_version_should_be_checked_when_passives_are_allocated() {
    let b = with_spec(witch(90), spec(&[1]));
    let r = run_with(&b, &options(None));
    assert_eq!(partial_reasons(&r, Field::Life), ["unchecked-tree-version"]);
    let r = run_with(&b, &options(Some("3_28")));
    assert_eq!(state(&r, Field::Life), StateKind::Indeterminate);
    assert!(
        r.reasons
            .indeterminate
            .contains(&IndeterminateReason::TreeVersion {
                build: "3_29".to_owned(),
                data: "3_28".to_owned()
            })
    );
    // No allocated passive: nothing to check.
    let r = run_with(&witch(90), &options(None));
    assert_eq!(state(&r, Field::Life), StateKind::Complete);
}

#[test]
fn items_beyond_their_lines_should_be_reasons() {
    let plain = || item(2, vec![]);
    for (slot, active, reason) in [
        ("Helmet", false, Some("unmodelled-item")),
        ("Weapon 1", false, Some("unmodelled-item")),
        ("Ring 3", false, Some("unmodelled-item")),
        ("Belt Abyssal Socket 1", false, Some("unmodelled-item")),
        ("Flask 1", true, Some("unmodelled-item")),
        ("Flask 1", false, None),
        ("Weapon 1 Swap", false, None),
        ("Amulet", false, None),
    ] {
        let r = run(&with_items(witch(1), vec![plain()], &[(slot, 2, active)]));
        match reason {
            Some(name) => assert_eq!(partial_reasons(&r, Field::Life), [name], "{slot}"),
            None => assert_eq!(state(&r, Field::Life), StateKind::Complete, "{slot}"),
        }
    }
}

#[test]
fn unread_lines_should_be_reasons() {
    let lines = vec![ItemLine {
        source: "???".to_owned(),
        kind: LineKind::Unrecognized,
    }];
    let r = run(&with_items(
        witch(1),
        vec![item(1, lines)],
        &[("Ring 1", 1, false)],
    ));
    assert_eq!(partial_reasons(&r, Field::Mana), ["unread-line"]);
}

#[test]
fn gems_config_and_choices_should_be_reasons() {
    let mut b = witch(1);
    let mut group = SocketGroup {
        label: None,
        enabled: true,
        slot: None,
        main_active_skill: None,
        gems: Ordered::new(),
        attributes: Vec::new(),
    };
    let gem = |enabled| Gem {
        gem: None,
        skill: None,
        name: None,
        level: Sourced::new(20),
        quality: Sourced::new(0),
        enabled,
        attributes: Vec::new(),
    };
    group.gems.push(gem(true));
    group.gems.push(gem(false));
    let mut set = SkillSet {
        id: SkillSetId(1),
        title: None,
        groups: Ordered::new(),
        attributes: Vec::new(),
    };
    set.groups.push(group);
    b.skills.sets.push(set);
    let r = run(&b);
    let FieldState::Partial(p) = &r.fields[&Field::LifeUnreserved].state else {
        panic!("not partial");
    };
    assert_eq!(
        p.iter().collect::<Vec<_>>(),
        [&UnsupportedReason::UnmodelledGem {
            source: Source::Gem { group: 0, gem: 0 }
        }],
        "the disabled gem gives none"
    );

    let mut b = witch(1);
    let mut config = ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs: Ordered::new(),
        attributes: Vec::new(),
    };
    config.inputs.push(ConfigInput {
        name: "useFrenzyCharges".to_owned(),
        value: ConfigValue::Boolean(false),
        attributes: Vec::new(),
    });
    b.config.sets.push(config);
    assert_eq!(
        partial_reasons(&run(&b), Field::Life),
        ["unmodelled-config"]
    );

    let mut b = witch(1);
    b.set_game_specific(GameSpecific::Poe1(Poe1Choices {
        bandit: Some("Alira".to_owned()),
        pantheon_major: Some("None".to_owned()),
        pantheon_minor: None,
    }))
    .unwrap();
    assert_eq!(
        partial_reasons(&run(&b), Field::Life),
        ["unmodelled-choice"]
    );
    let mut b = witch(1);
    b.set_game_specific(GameSpecific::Poe1(Poe1Choices {
        bandit: Some("None".to_owned()),
        pantheon_major: Some("None".to_owned()),
        pantheon_minor: Some("None".to_owned()),
    }))
    .unwrap();
    assert_eq!(state(&run(&b), Field::Life), StateKind::Complete);
}

// ------------------------------------------------------------- defences

fn spec_line(key: &str, value: &str) -> ItemLine {
    ItemLine {
        source: format!("{key}: {value}"),
        kind: LineKind::Spec {
            key: key.to_owned(),
            value: value.to_owned(),
            known: true,
        },
    }
}

/// Options naming each item's base (`(item, base name)`).
fn with_bases(bases: &[(u32, &str)]) -> CalcOptions {
    let mut o = options(Some("3_29"));
    for (id, name) in bases {
        o.item_bases.insert(
            ItemId(*id),
            BaseName {
                name: (*name).to_owned(),
                fallback: None,
            },
        );
    }
    o
}

const FIRE: Field = Field::Resistance(DamageType::Fire);
const COLD: Field = Field::Resistance(DamageType::Cold);
const SPELL_BLOCK: Field = Field::Poe1(Poe1Field::SpellBlockChance);
const SUPPRESSION: Field = Field::Poe1(Poe1Field::SpellSuppressionChance);

/// The value of the node with step term `name` below `field`'s node (the
/// nearest one, as `calc-parity` finds it).
fn step(r: &CalcResult, field: Field, name: &str) -> Option<f64> {
    let top = r.fields[&field].node?;
    r.graph
        .iter()
        .filter(|(i, _, _)| i.get() <= top.get())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find(|(_, n, _)| match n {
            Node::Sum { term, .. }
            | Node::Product { term, .. }
            | Node::Trunc { term, .. }
            | Node::Min { term, .. }
            | Node::Max { term, .. }
            | Node::Floor { term, .. }
            | Node::RoundHalfUp { term, .. }
            | Node::PercentFactor { term, .. } => {
                matches!(term, Term::Step(s) if *s == name)
            }
            _ => false,
        })
        .map(|(_, _, v)| v)
}

#[test]
fn the_resistance_penalty_should_follow_its_config_input() {
    for (value, expect) in [(0.0, 0.0), (-30.0, -30.0)] {
        let mut b = witch(1);
        let mut config = ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::new(),
            attributes: Vec::new(),
        };
        config.inputs.push(ConfigInput {
            name: "resistancePenalty".to_owned(),
            value: ConfigValue::Number(Sourced::new(loom_domain::Number(value))),
            attributes: Vec::new(),
        });
        b.config.sets.push(config);
        let r = run(&b);
        for t in DamageType::ALL.into_iter().skip(1) {
            let field = Field::Resistance(t);
            assert_eq!(value_of(&r, field), expect, "{field}");
            assert_eq!(state(&r, field), StateKind::Complete, "{field}");
        }
    }
}

fn value_of(r: &CalcResult, field: Field) -> f64 {
    value(r, field)
}

#[test]
fn resistances_should_be_capped_by_their_maximum_and_truncated() {
    // Fire: -60 + 200 + 10 = 150, maximum 75 + 20 + 2 = 97, capped at 90.
    // Cold: -60 + 10 = -50, maximum 77. Chaos: -60 + 3 = -57.
    let ring = item(
        1,
        vec![modifier(vec![
            stat("base_fire_damage_resistance_%", 200),
            stat("base_resist_all_elements_%", 10),
            stat("base_maximum_fire_damage_resistance_%", 20),
            stat("additional_maximum_all_resistances_%", 2),
            stat("base_chaos_damage_resistance_%", 3),
        ])],
    );
    let r = run(&with_items(witch(90), vec![ring], &[("Ring 1", 1, false)]));
    assert_eq!(value(&r, FIRE), 90.0);
    assert_eq!(step(&r, FIRE, "total"), Some(150.0));
    assert_eq!(step(&r, FIRE, "max"), Some(90.0));
    assert_eq!(step(&r, FIRE, "min"), Some(-200.0));
    assert_eq!(value(&r, COLD), -50.0);
    assert_eq!(step(&r, COLD, "max"), Some(77.0));
    assert_eq!(value(&r, Field::Resistance(DamageType::Chaos)), -57.0);
    assert_eq!(state(&r, FIRE), StateKind::Complete);
}

#[test]
fn an_armour_piece_should_add_its_local_value_as_a_row() {
    // Test Plate: armour 100 to 120. With no percentile line the roll is
    // the top: (100 + 20 + 20 x 1) x (1 + 50 / 100) x (1 + 20 / 100) =
    // 252; a global 10% increase: 252 x 1.1 = 277.2, 277.
    let plate = item(
        2,
        vec![
            spec_line("Quality", "20"),
            modifier(vec![
                stat("local_base_physical_damage_reduction_rating", 20),
                stat("local_physical_damage_reduction_rating_+%", 50),
            ]),
        ],
    );
    let ring = item(
        1,
        vec![modifier(vec![stat(
            "physical_damage_reduction_rating_+%",
            10,
        )])],
    );
    let b = with_items(
        witch(90),
        vec![plate, ring],
        &[("Body Armour", 2, false), ("Ring 1", 1, false)],
    );
    let r = run_with(&b, &with_bases(&[(2, "Test Plate")]));
    assert_eq!(value(&r, Field::Armour), 277.0);
    assert_eq!(state(&r, Field::Armour), StateKind::Complete);
    assert_eq!(
        step(&r, Field::Armour, "slot.body-armour"),
        Some(252.0 * 1.1)
    );
    assert_eq!(
        step(&r, Field::Armour, "slot.global"),
        None,
        "no flat armour"
    );
    // The local stats did not reach the global sum.
    assert_eq!(state(&r, Field::Life), StateKind::Complete);

    // A percentile line: (100 + 20 + 20 x 0.5) x 1.5 x 1.2 = 234.
    let plate = item(
        2,
        vec![
            spec_line("Quality", "+20%"),
            spec_line("Armour", "234"),
            spec_line("ArmourBasePercentile", "0.5"),
            modifier(vec![
                stat("local_base_physical_damage_reduction_rating", 20),
                stat("local_physical_damage_reduction_rating_+%", 50),
            ]),
        ],
    );
    let b = with_items(witch(90), vec![plate], &[("Body Armour", 2, false)]);
    let r = run_with(&b, &with_bases(&[(2, "Test Plate")]));
    assert_eq!(value(&r, Field::Armour), 234.0);
    assert_eq!(state(&r, Field::Armour), StateKind::Complete);
}

#[test]
fn attribute_bonuses_should_scale_every_row() {
    // Test Robe in the helmet: energy shield 50, evasion 10 to 20 (20 at
    // the top). Int 32: 3% more... increased: 50 x 1.03 = 51.5, 52.
    // Evasion: helmet 20 x 1.02 = 20.4, global 15 x 1.02 = 15.3; 35.7, 36.
    let robe = item(3, vec![]);
    let b = with_items(witch(90), vec![robe], &[("Helmet", 3, false)]);
    let r = run_with(&b, &with_bases(&[(3, "Test Robe")]));
    assert_eq!(value(&r, Field::EnergyShield), 52.0);
    assert_eq!(value(&r, Field::Evasion), 36.0);
    assert_eq!(step(&r, Field::Evasion, "slot.helmet"), Some(20.0 * 1.02));
    assert_eq!(step(&r, Field::Evasion, "slot.global"), Some(15.0 * 1.02));
    assert_eq!(step(&r, Field::EnergyShield, "slot.helmet"), Some(51.5));
    for field in [Field::EnergyShield, Field::Evasion, Field::Armour] {
        assert_eq!(state(&r, field), StateKind::Complete, "{field}");
    }
}

#[test]
fn a_shield_should_give_block_and_block_increases_should_reach_spell_block() {
    // Shield 24 + 3 = 27, x 1.1 = 29.7, floored: 29. Attack block (29 + 5)
    // x 1.2 = 40.8, 41, below the maximum 77. Spell block 10 x 1.2 = 12.
    // Suppression 120, capped at 100.
    let shield = item(
        4,
        vec![modifier(vec![
            stat("local_additional_block_chance_%", 3),
            stat("local_block_chance_+%", 10),
        ])],
    );
    let ring = item(
        1,
        vec![modifier(vec![
            stat("additional_block_%", 5),
            stat("block_chance_+%", 20),
            stat("maximum_block_%", 2),
            stat("base_spell_block_%", 10),
            stat("base_spell_suppression_chance_%", 120),
        ])],
    );
    let b = with_items(
        witch(90),
        vec![shield, ring],
        &[("Weapon 2", 4, false), ("Ring 1", 1, false)],
    );
    let r = run_with(&b, &with_bases(&[(4, "Test Shield")]));
    assert_eq!(step(&r, Field::BlockChance, "shield-block"), Some(29.0));
    assert_eq!(value(&r, Field::BlockChance), 41.0);
    assert_eq!(step(&r, Field::BlockChance, "max"), Some(77.0));
    assert_eq!(value(&r, SPELL_BLOCK), 12.0);
    assert_eq!(value(&r, SUPPRESSION), 100.0);
    // The shield's armour is a row too: 10.
    assert_eq!(value(&r, Field::Armour), 10.0);
    for field in [Field::BlockChance, SPELL_BLOCK, SUPPRESSION, Field::Armour] {
        assert_eq!(state(&r, field), StateKind::Complete, "{field}");
    }
}

#[test]
fn armour_slots_without_a_known_base_should_be_reasons() {
    let r = |slot: &str, base: Option<&str>, lines: Vec<ItemLine>| {
        let b = with_items(witch(1), vec![item(2, lines)], &[(slot, 2, false)]);
        let o = base.map_or_else(|| options(Some("3_29")), |n| with_bases(&[(2, n)]));
        run_with(&b, &o)
    };
    let reason = |r: &CalcResult| {
        r.reasons
            .partial
            .iter()
            .find_map(|p| match p {
                UnsupportedReason::UnmodelledItem { what, .. } => Some(*what),
                _ => None,
            })
            .unwrap_or("none")
    };
    assert_eq!(reason(&r("Helmet", None, vec![])), "no-base-name");
    assert_eq!(
        reason(&r("Weapon 2", Some("Test Sword"), vec![])),
        "base-properties"
    );
    assert_eq!(
        reason(&r("Boots", Some("Two-Toned Boots"), vec![])),
        "ambiguous-base"
    );
    assert_eq!(
        reason(&r(
            "Body Armour",
            Some("Test Plate"),
            vec![spec_line("Armour", "130")]
        )),
        "base-percentile",
        "a shown value without its percentile"
    );
    // A quiver's lines are global; nothing is missing.
    let quiver = r("Weapon 2", Some("Test Quiver"), vec![]);
    assert_eq!(state(&quiver, Field::Armour), StateKind::Complete);
    // A base the data lacks.
    let unknown = r("Gloves", Some("Nothing"), vec![]);
    assert_eq!(state(&unknown, Field::Armour), StateKind::Indeterminate);
    assert!(
        unknown
            .reasons
            .indeterminate
            .contains(&IndeterminateReason::MissingData {
                table: "base_items",
                key: "Nothing".to_owned()
            })
    );
}

// ------------------------------------------------------------- the graph

#[test]
fn life_should_expose_its_breakdown_terms() {
    let r = run(&with_spec(witch(90), spec(&[1])));
    let life = r.fields[&Field::Life].node.unwrap();
    let all: Vec<_> = r
        .graph
        .iter()
        .filter(|(i, _, _)| i.get() < life.get())
        .collect();
    let below = |term: Term| {
        all.iter()
            .rev()
            .find(|(_, n, _)| match n {
                Node::Sum { term: t, .. }
                | Node::Product { term: t, .. }
                | Node::PercentFactor { term: t, .. } => *t == term,
                _ => false,
            })
            .map(|(_, _, v)| *v)
    };
    assert_eq!(below(Term::Base), Some(1135.0));
    assert_eq!(below(Term::Increased), Some(1.08));
    assert_eq!(below(Term::More), Some(1.0));
    assert!(matches!(
        r.graph.node(life),
        Some(Node::Max {
            term: Term::Result,
            ..
        })
    ));
}

#[test]
fn two_runs_should_give_the_same_result() {
    let b = with_spec(witch(90), spec(&[1, 2, 3]));
    assert_eq!(format!("{:?}", run(&b)), format!("{:?}", run(&b)));
}

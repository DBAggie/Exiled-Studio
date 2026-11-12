//! The Path of Exile character fields end to end (Phase 3 plan, step 6):
//! attributes, life, mana, energy shield, unreserved life and mana, from
//! the class, per-level growth, passives, mastery effects, items, and
//! jewels; and a reason for each thing not modelled yet, never a silent
//! value.
//!
//! The expected values follow PoB v2.67.2's formulas (module docs of
//! `loom_calc::poe1::phases`); the mechanic fixtures check the same
//! against the oracle (`docs/phase-3/character-fields.md`).

use loom_calc::{
    CalcData, CalcInput, CalcOptions, CalcResult, CharacterBase, Field, FieldState,
    IndeterminateReason, Node, Source, StatAmount, StateKind, Term, UnsupportedReason, calculate,
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

/// A small table: the Witch (as the game's characters table), the
/// per-level constants of `Character.ot`, passives 1 to 4, mastery
/// effect 77.
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

fn spec(nodes: &[u32]) -> PassiveSpec {
    let mut s = PassiveSpec::new(SpecId(1));
    s.tree_version = Some(TreeVersion::new("3_29"));
    s.nodes = Sourced::new(nodes.iter().map(|n| NodeId(*n)).collect());
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
    // The other 16 slice fields are not computed yet.
    let not_computed = r
        .fields
        .values()
        .filter(|f| f.state == FieldState::Unsupported(UnsupportedReason::NotComputed))
        .count();
    assert_eq!(not_computed, 16);
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

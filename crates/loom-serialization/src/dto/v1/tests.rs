//! Build document v1: round-trips, determinism, golden bytes, and every
//! error code.

use loom_domain::poe1::Poe1Choices;
use loom_domain::{
    Adapted, AltVariant, AscendancyId, Attribute, Build, BuildPart, ClassId, Config, ConfigInput,
    ConfigSet, ConfigSetId, ConfigValue, DataVersion, Exclusion, Game, GameSpecific, Gem, GemId,
    HeaderLine, InfluenceId, Item, ItemBody, ItemId, ItemLine, ItemPart, ItemSet, ItemSetId,
    ItemsEntry, JewelSocket, LineKind, LineResolution, MasteryEffect, ModSection, ModifierLine,
    NoBase, NodeId, Number, OpaqueNode, Ordered, PassiveSpec, Prefix, PropertyValue, Rarity,
    Resolved, SkillId, SkillSet, SkillSetId, SlotEntry, SlotId, SocketGroup, Sourced, SpecId,
    SpecPart, StatId, StatLine, StatValue, TreeVersion, TypedResolution, UnresolvedReason,
    VariantSelection,
};

use super::{DtoError, from_json, to_json};

fn attr(name: &str, value: &str) -> Attribute {
    Attribute::new(name, value)
}

fn element(name: &str, attributes: Vec<Attribute>, children: Vec<OpaqueNode>) -> OpaqueNode {
    OpaqueNode::Element {
        name: name.to_owned(),
        attributes,
        children,
    }
}

fn modifier(source: &str, section: ModSection, resolution: LineResolution) -> ItemLine {
    ItemLine {
        source: source.to_owned(),
        kind: LineKind::Modifier(ModifierLine {
            text: source.to_owned(),
            prefixes: Vec::new(),
            section,
            selected: true,
            resolution,
        }),
    }
}

fn stat(id: &str, value: StatValue) -> StatLine {
    StatLine {
        stat: StatId::new(id),
        value,
    }
}

fn line(source: &str, kind: LineKind) -> ItemLine {
    ItemLine {
        source: source.to_owned(),
        kind,
    }
}

/// An item with every line kind, every resolution, and every reason.
fn every_line_item(id: u32) -> Item {
    let lines = vec![
        line("Rarity: RARE", LineKind::Header(HeaderLine::Rarity)),
        line("Made-up Name", LineKind::Header(HeaderLine::Name)),
        line("Made-up Base", LineKind::Header(HeaderLine::Base)),
        line("Item Class: Rings", LineKind::Header(HeaderLine::ItemClass)),
        line("--------", LineKind::Header(HeaderLine::Separator)),
        line(
            "Item Level: 84",
            LineKind::Spec {
                key: "Item Level".to_owned(),
                value: "84".to_owned(),
                known: true,
            },
        ),
        line(
            "Odd Key: x",
            LineKind::Spec {
                key: "Odd Key".to_owned(),
                value: "x".to_owned(),
                known: false,
            },
        ),
        line(
            "{variant:2}Other Base",
            LineKind::VariantBase {
                base: "Other Base".to_owned(),
                selected: false,
            },
        ),
        line(
            "Made-up Influence Item",
            LineKind::Influence(InfluenceId::new("Made-up")),
        ),
        line("Corrupted", LineKind::Flag),
        line("??? strange", LineKind::Unrecognized),
        // 11: stats, exact and ranged, case-folded.
        ItemLine {
            source: "{crafted}{range:0.5}+(10-20) to made-up stat".to_owned(),
            kind: LineKind::Modifier(ModifierLine {
                text: "+(10-20) to made-up stat".to_owned(),
                prefixes: vec![
                    Prefix::new("crafted", None),
                    Prefix::new("range", Some("0.5")),
                ],
                section: ModSection::Explicit,
                selected: true,
                resolution: LineResolution::Resolved {
                    result: Resolved::Stats(vec![
                        stat("made_up_a", StatValue::Ranged { min: 10, max: 20 }),
                        stat("made_up_b", StatValue::Exact { min: -3, max: -3 }),
                        stat("made_up_c", StatValue::Hidden { assumed: Some(1) }),
                        stat("made_up_d", StatValue::Hidden { assumed: None }),
                        stat("made_up_e", StatValue::Lookup(vec![2, 7, i64::MAX])),
                    ]),
                    adapted: Some(Adapted::CaseFolded),
                },
            }),
        },
        // 12, 13: a split-section pair.
        modifier(
            "first half",
            ModSection::Implicit,
            LineResolution::Resolved {
                result: Resolved::Stats(vec![stat(
                    "made_up_split",
                    StatValue::Exact { min: 1, max: 1 },
                )]),
                adapted: Some(Adapted::SplitSection),
            },
        ),
        modifier(
            "second half",
            ModSection::Explicit,
            LineResolution::Continues { line: 12 },
        ),
        modifier(
            "Grants Level 20 Made-up Skill",
            ModSection::Other("made-up-section".to_owned()),
            LineResolution::Resolved {
                result: Resolved::Typed(TypedResolution::GrantedSkill {
                    source: "MadeUpGraft".to_owned(),
                    gem: GemId::new("Metadata/Items/Gems/MadeUp"),
                    level: 20,
                }),
                adapted: None,
            },
        ),
        modifier(
            "Made-up property: 1.5",
            ModSection::Enchant,
            LineResolution::Resolved {
                result: Resolved::Typed(TypedResolution::ItemProperty {
                    property: "MadeUpProperty".to_owned(),
                    values: vec![PropertyValue {
                        slot: 1,
                        text: "1.5".to_owned(),
                    }],
                }),
                adapted: None,
            },
        ),
        modifier(
            "Quality (Made-up): +20%",
            ModSection::Explicit,
            LineResolution::Resolved {
                result: Resolved::Typed(TypedResolution::CatalystQuality {
                    quality_type: "MadeUpQuality".to_owned(),
                    stats: vec![stat("made_up_q", StatValue::Exact { min: 20, max: 20 })],
                    quality: "20".to_owned(),
                }),
                adapted: None,
            },
        ),
        modifier("pending", ModSection::Explicit, LineResolution::Pending),
        modifier(
            "not selected",
            ModSection::Explicit,
            LineResolution::Excluded(Exclusion::NotSelected),
        ),
        modifier(
            "custom",
            ModSection::Explicit,
            LineResolution::Excluded(Exclusion::Custom),
        ),
        modifier(
            "section",
            ModSection::Other("class-requirement".to_owned()),
            LineResolution::Excluded(Exclusion::Section),
        ),
        modifier(
            "no template",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::NoTemplate),
        ),
        modifier(
            "no variant fits",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::NoVariantFits),
        ),
        modifier(
            "only shadowed",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::OnlyShadowed),
        ),
        modifier(
            "ambiguous",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::Ambiguous(vec![
                vec![StatId::new("a"), StatId::new("b")],
                vec![StatId::new("c")],
            ])),
        ),
        modifier(
            "unnamed",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::NoBase(NoBase::Unnamed("tie".to_owned()))),
        ),
        modifier(
            "not found",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::NoBase(NoBase::NotFound)),
        ),
        modifier(
            "scope conflict",
            ModSection::Explicit,
            LineResolution::Unresolved(UnresolvedReason::NoBase(NoBase::ScopeConflict)),
        ),
    ];
    let spacing = (0..=lines.len())
        .map(|i| if i % 5 == 0 { "\n\n" } else { "\n" }.to_owned())
        .collect();
    let mut layout = Ordered::new();
    layout.push_opaque(OpaqueNode::Text("an older text".to_owned()));
    layout.push(ItemPart::Body);
    layout.push_opaque(element(
        "ModRange",
        vec![attr("id", "1"), attr("range", "0.5")],
        Vec::new(),
    ));
    Item {
        id: ItemId(id),
        body: ItemBody {
            item_class: Some("Rings".to_owned()),
            rarity: Some(Rarity::Rare),
            name: Some("Made-up Name".to_owned()),
            base: Some("Made-up Base".to_owned()),
            implicits: Some(2),
            variants: VariantSelection {
                count: 2,
                selected: Some(1),
                alternates: vec![
                    AltVariant {
                        index: 0,
                        selected: Some(2),
                    },
                    AltVariant {
                        index: 1,
                        selected: None,
                    },
                ],
            },
            lines,
        },
        layout,
        text_spacing: spacing,
        attributes: vec![attr("variant", "1")],
    }
}

/// A plain item: no optional field set.
fn plain_item(id: u32) -> Item {
    let mut layout = Ordered::new();
    layout.push(ItemPart::Body);
    Item {
        id: ItemId(id),
        body: ItemBody {
            rarity: Some(Rarity::Normal),
            lines: vec![line(
                "Made-up Plain Base",
                LineKind::Header(HeaderLine::Name),
            )],
            ..ItemBody::default()
        },
        layout,
        text_spacing: Vec::new(),
        attributes: Vec::new(),
    }
}

fn number(v: f64, source: Option<&str>) -> ConfigValue {
    ConfigValue::Number(match source {
        Some(s) => Sourced::read(Number(v), s),
        None => Sourced::new(Number(v)),
    })
}

/// A PoE1 build with every DTO type and variant.
pub(crate) fn poe1_build() -> Build {
    let mut b = Build::new(Game::Poe1);
    b.set_game_specific(GameSpecific::Poe1(Poe1Choices {
        bandit: Some("Alira".to_owned()),
        pantheon_major: Some("TheBrineKing".to_owned()),
        pantheon_minor: None,
    }))
    .unwrap();
    b.data = Some(DataVersion {
        game: Game::Poe1,
        schema_version: 1,
        steam_build: Some(26_530_219),
    });
    b.character.class = Some(ClassId::new("Witch"));
    b.character.ascendancy = Some(AscendancyId::new("Occultist"));
    b.character.level = Some(Sourced::read(95, "95"));
    b.character.main_socket_group = Some(Sourced::new(2));
    b.character.attributes = vec![attr("viewMode", "TREE")];
    b.character.children = vec![
        element(
            "PlayerStat",
            vec![attr("stat", "Life"), attr("value", "1.0")],
            Vec::new(),
        ),
        OpaqueNode::Comment(" a comment ".to_owned()),
    ];

    // Passives: two specs, one with every field.
    let mut spec = PassiveSpec::new(SpecId(1));
    spec.title = Some("Leveling".to_owned());
    spec.tree_version = Some(TreeVersion::new("3_26"));
    spec.nodes = Sourced::read(vec![NodeId(1), NodeId(65_535)], "1,65535");
    spec.mastery_effects = Sourced::read(
        vec![MasteryEffect {
            node: NodeId(7),
            effect: 48_385,
        }],
        "{7,48385}",
    );
    spec.jewel_sockets.push(JewelSocket {
        node: NodeId(26_725),
        item: Some(ItemId(3)),
        attributes: Vec::new(),
    });
    spec.jewel_sockets
        .push_opaque(element("Socket", vec![attr("odd", "1")], Vec::new()));
    spec.jewel_sockets.push(JewelSocket {
        node: NodeId(33_631),
        item: None,
        attributes: vec![attr("extra", "y")],
    });
    spec.layout.push_opaque(element(
        "URL",
        Vec::new(),
        vec![OpaqueNode::Text("https://example.invalid/tree".to_owned())],
    ));
    spec.layout.push(SpecPart::JewelSockets);
    spec.attributes = vec![attr("classInternalId", "3")];
    b.passives.active = Some(SpecId(1));
    b.passives.specs.push(spec);
    b.passives
        .specs
        .push_opaque(OpaqueNode::ProcessingInstruction {
            target: "made-up".to_owned(),
            data: None,
        });
    b.passives.specs.push(PassiveSpec::new(SpecId(2)));
    b.passives.attributes = vec![attr("activeSpec", "1")];

    // Skills.
    let mut gems = Ordered::new();
    gems.push(Gem {
        gem: Some(GemId::new("Metadata/Items/Gems/SkillGemMadeUp")),
        skill: Some(SkillId::new("MadeUp")),
        name: Some("Made Up".to_owned()),
        level: Sourced::read(21, "21"),
        quality: Sourced::read(-5, "-5"),
        enabled: true,
        attributes: vec![attr("qualityId", "Default")],
    });
    gems.push_opaque(element("Unknown", Vec::new(), Vec::new()));
    gems.push(Gem {
        gem: None,
        skill: None,
        name: None,
        level: Sourced::new(1),
        quality: Sourced::new(0),
        enabled: false,
        attributes: Vec::new(),
    });
    let mut groups = Ordered::new();
    groups.push(SocketGroup {
        label: Some("Main".to_owned()),
        enabled: true,
        slot: Some(SlotId::new("Body Armour")),
        main_active_skill: Some(Sourced::read(1, "1")),
        gems,
        attributes: Vec::new(),
    });
    groups.push(SocketGroup {
        label: None,
        enabled: false,
        slot: None,
        main_active_skill: None,
        gems: Ordered::new(),
        attributes: vec![attr("source", "x")],
    });
    b.skills.active = Some(SkillSetId(1));
    b.skills.sets.push(SkillSet {
        id: SkillSetId(1),
        title: None,
        groups,
        attributes: Vec::new(),
    });

    // Items: two items and a set, with opaque content between them.
    b.items.active_set = Some(ItemSetId(1));
    b.items.entries.push(ItemsEntry::Item(every_line_item(3)));
    b.items
        .entries
        .push_opaque(OpaqueNode::Comment("between items".to_owned()));
    b.items.entries.push(ItemsEntry::Item(plain_item(4)));
    let mut slots = Ordered::new();
    slots.push(SlotEntry {
        slot: SlotId::new("Ring 1"),
        item: Some(ItemId(3)),
        attributes: Vec::new(),
    });
    slots.push(SlotEntry {
        slot: SlotId::new("Ring 2"),
        item: None,
        attributes: vec![attr("itemPbURL", "")],
    });
    b.items.entries.push(ItemsEntry::Set(ItemSet {
        id: ItemSetId(1),
        title: Some("Default".to_owned()),
        slots,
        attributes: Vec::new(),
    }));
    b.items.attributes = vec![attr("showStatDifferences", "true")];

    // Config: every value kind and special numbers.
    let mut inputs = Ordered::new();
    for (name, value) in [
        ("flag", ConfigValue::Boolean(true)),
        (
            "text",
            ConfigValue::String("quoted \"\u{e9}\" \t text".to_owned()),
        ),
        ("int", number(3.0, Some("3"))),
        ("decimal", number(0.1 + 0.2, None)),
        ("negative-zero", number(-0.0, Some("-0"))),
        ("tiny", number(5e-324, None)),
        ("huge", number(f64::MAX, None)),
        ("infinity", number(f64::INFINITY, Some("inf"))),
        ("negative-infinity", number(f64::NEG_INFINITY, None)),
        ("nan", number(f64::NAN, Some("NaN"))),
        (
            "nan-payload",
            number(f64::from_bits(0xfff0_0000_0000_0001), None),
        ),
    ] {
        inputs.push(ConfigInput {
            name: name.to_owned(),
            value,
            attributes: Vec::new(),
        });
    }
    inputs.push_opaque(element("Placeholder", vec![attr("name", "x")], Vec::new()));
    b.config.active = Some(ConfigSetId(1));
    b.config.sets.push(ConfigSet {
        id: ConfigSetId(1),
        title: Some("Default".to_owned()),
        inputs,
        attributes: Vec::new(),
    });

    // Layout: an untyped section between typed ones; prolog and epilog.
    let mut layout = Ordered::new();
    layout.push(BuildPart::Character);
    layout.push_opaque(element(
        "Notes",
        Vec::new(),
        vec![OpaqueNode::Text("line 1\nline 2".to_owned())],
    ));
    for part in [
        BuildPart::Passives,
        BuildPart::Skills,
        BuildPart::Items,
        BuildPart::Config,
    ] {
        layout.push(part);
    }
    b.layout = layout;
    b.attributes = vec![attr("version", "2")];
    b.prolog = vec![OpaqueNode::ProcessingInstruction {
        target: "pi".to_owned(),
        data: Some("data".to_owned()),
    }];
    b.epilog = vec![OpaqueNode::Comment("end".to_owned())];
    b
}

/// A PoE2 build: no PoE1 choices, a resolved item.
pub(crate) fn poe2_build() -> Build {
    let mut b = Build::new(Game::Poe2);
    b.data = Some(DataVersion {
        game: Game::Poe2,
        schema_version: 1,
        steam_build: None,
    });
    b.character.class = Some(ClassId::new("Made-up Class"));
    b.items.entries.push(ItemsEntry::Item(every_line_item(1)));
    b.config = Config::default();
    b
}

fn round_trip(b: &Build) -> String {
    let json = to_json(b).unwrap();
    assert_eq!(to_json(b).unwrap(), json, "a second run differs");
    let back = from_json(&json).unwrap();
    assert_eq!(&back, b);
    assert_eq!(to_json(&back).unwrap(), json, "the re-written JSON differs");
    json
}

#[test]
fn poe1_build_should_round_trip_through_json() {
    round_trip(&poe1_build());
}

#[test]
fn poe2_build_should_round_trip_through_json() {
    let json = round_trip(&poe2_build());
    assert!(json.contains("\"poe2\": {}"), "{json}");
}

#[test]
fn empty_builds_should_round_trip_through_json() {
    round_trip(&Build::new(Game::Poe1));
    round_trip(&Build::new(Game::Poe2));
}

/// The bytes are fixed: the committed files are what every platform
/// writes. Regenerate with the ignored `write_golden_files` test after a
/// deliberate change.
#[test]
fn synthetic_builds_should_give_the_golden_bytes() {
    assert_eq!(
        to_json(&poe1_build()).unwrap(),
        include_str!("testdata/poe1.json")
    );
    assert_eq!(
        to_json(&poe2_build()).unwrap(),
        include_str!("testdata/poe2.json")
    );
}

#[test]
#[ignore = "writes the golden files"]
#[expect(
    clippy::disallowed_methods,
    reason = "a test-only helper writes the crate's golden files"
)]
fn write_golden_files() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/dto/v1/testdata");
    std::fs::write(format!("{dir}/poe1.json"), to_json(&poe1_build()).unwrap()).unwrap();
    std::fs::write(format!("{dir}/poe2.json"), to_json(&poe2_build()).unwrap()).unwrap();
}

#[test]
fn special_numbers_should_keep_their_bits() {
    let json = to_json(&poe1_build()).unwrap();
    assert!(json.contains("\"value\": 0.30000000000000004"), "{json}");
    assert!(json.contains("\"value\": -0.0"), "{json}");
    assert!(
        json.contains("\"f64_bits\": \"7ff0000000000000\""),
        "{json}"
    );
    assert!(
        json.contains("\"f64_bits\": \"fff0000000000001\""),
        "{json}"
    );
}

fn code(json: &str) -> &'static str {
    from_json(json).err().map_or("ok", |e| e.code())
}

fn poe1_json() -> String {
    to_json(&poe1_build()).unwrap()
}

#[test]
fn a_wrong_format_should_give_dto_format() {
    let json = poe1_json().replacen("\"loom-build\"", "\"other\"", 1);
    assert_eq!(code(&json), "DTO_FORMAT");
}

#[test]
fn another_version_should_give_dto_version() {
    let json = poe1_json().replacen("\"version\": 1,", "\"version\": 2,", 1);
    assert_eq!(code(&json), "DTO_VERSION");
}

#[test]
fn an_unknown_field_should_give_dto_json() {
    let json = poe1_json().replacen("\"game\": \"poe1\",", "\"game\": \"poe1\", \"x\": 1,", 1);
    assert_eq!(code(&json), "DTO_JSON");
    assert_eq!(code("not json"), "DTO_JSON");
    assert_eq!(code("{}"), "DTO_JSON");
}

#[test]
fn another_games_choices_should_give_dto_domain() {
    let json =
        to_json(&Build::new(Game::Poe2))
            .unwrap()
            .replacen("\"poe2\": {}", "\"poe1\": {}", 1);
    assert_eq!(code(&json), "DTO_DOMAIN");
}

#[test]
fn a_broken_link_should_give_dto_item_links_both_ways() {
    let json = poe1_json().replacen("\"line\": 12", "\"line\": 11", 1);
    assert_eq!(code(&json), "DTO_ITEM_LINKS");
    let mut b = poe1_build();
    let item = b.items.entries.typed_mut().find_map(|e| match e {
        ItemsEntry::Item(i) => Some(i),
        ItemsEntry::Set(_) => None,
    });
    if let Some(LineKind::Modifier(m)) = item.map(|i| &mut i.body.lines[13].kind) {
        m.resolution = LineResolution::Pending;
    }
    assert!(matches!(to_json(&b), Err(DtoError::Item { item: 3, .. })));
}

#[test]
fn bad_number_bits_should_give_dto_number() {
    let json = poe1_json();
    for bad in [
        "3ff0000000000000",
        "7FF0000000000000",
        "7ff00000",
        "zzzzzzzzzzzzzzzz",
    ] {
        let changed = json.replacen("7ff0000000000000", bad, 1);
        assert_eq!(code(&changed), "DTO_NUMBER", "{bad}");
    }
}

#[cfg(target_pointer_width = "32")]
#[test]
fn an_index_past_usize_should_give_dto_out_of_range() {
    let json = poe1_json().replacen("\"line\": 12", "\"line\": 4294967296", 1);
    assert_eq!(code(&json), "DTO_OUT_OF_RANGE");
}

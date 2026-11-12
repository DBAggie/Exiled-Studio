use crate::poe1::Poe1Choices;
use crate::*;

fn line(source: &str, kind: LineKind) -> ItemLine {
    ItemLine {
        source: source.to_owned(),
        kind,
    }
}

fn modifier(text: &str, section: ModSection, resolution: LineResolution) -> LineKind {
    LineKind::Modifier(ModifierLine {
        text: text.to_owned(),
        prefixes: Vec::new(),
        section,
        selected: true,
        resolution,
    })
}

fn stats(stat: &str, value: i64) -> Resolved {
    Resolved::Stats(vec![StatLine {
        stat: StatId::new(stat),
        value: StatValue::Exact {
            min: value,
            max: value,
        },
    }])
}

fn resolved(result: Resolved, adapted: Option<Adapted>) -> LineResolution {
    LineResolution::Resolved { result, adapted }
}

/// An item with every line kind and every resolution state.
fn every_line_item(id: u32) -> Item {
    let lines = vec![
        line("Item Class: Rings", LineKind::Header(HeaderLine::ItemClass)),
        line("Rarity: RARE", LineKind::Header(HeaderLine::Rarity)),
        line("Doom Loop", LineKind::Header(HeaderLine::Name)),
        line("Two-Stone Ring", LineKind::Header(HeaderLine::Base)),
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
            "Odd Key: 1",
            LineKind::Spec {
                key: "Odd Key".to_owned(),
                value: "1".to_owned(),
                known: false,
            },
        ),
        line(
            "{variant:1}Two-Stone Ring",
            LineKind::VariantBase {
                base: "Two-Stone Ring".to_owned(),
                selected: true,
            },
        ),
        line(
            "Shaper Item",
            LineKind::Influence(InfluenceId::new("Shaper")),
        ),
        line("Corrupted", LineKind::Flag),
        line("some stray text", LineKind::Unrecognized),
        // 11: exact stat resolution.
        line(
            "+20 to maximum Life",
            modifier(
                "+20 to maximum Life",
                ModSection::Implicit,
                resolved(stats("base_maximum_life", 20), None),
            ),
        ),
        // 12: case-folded.
        line(
            "+30 to Maximum Life",
            modifier(
                "+30 to Maximum Life",
                ModSection::Explicit,
                resolved(stats("base_maximum_life", 30), Some(Adapted::CaseFolded)),
            ),
        ),
        // 13 and 14: a split-section pair.
        line(
            "{enchant}first half",
            modifier(
                "first half",
                ModSection::Enchant,
                resolved(stats("some_stat", 1), Some(Adapted::SplitSection)),
            ),
        ),
        line(
            "second half",
            modifier(
                "second half",
                ModSection::Explicit,
                LineResolution::Continues { line: 13 },
            ),
        ),
        // 15 to 17: typed resolutions.
        line(
            "Uses level 20 Some Skill",
            modifier(
                "Uses level 20 Some Skill",
                ModSection::Implicit,
                resolved(
                    Resolved::Typed(TypedResolution::GrantedSkill {
                        source: "GraftA".to_owned(),
                        gem: GemId::new("Metadata/Items/Gems/SomeSkill"),
                        level: 20,
                    }),
                    None,
                ),
            ),
        ),
        line(
            "Lasts 4 Seconds",
            modifier(
                "Lasts 4 Seconds",
                ModSection::Explicit,
                resolved(
                    Resolved::Typed(TypedResolution::ItemProperty {
                        property: "ItemDisplayFlaskBuffDurationNSeconds".to_owned(),
                        values: vec![PropertyValue {
                            slot: 0,
                            text: "4".to_owned(),
                        }],
                    }),
                    None,
                ),
            ),
        ),
        line(
            "Quality (Attribute Modifiers): +20%",
            modifier(
                "Quality (Attribute Modifiers): +20%",
                ModSection::Explicit,
                resolved(
                    Resolved::Typed(TypedResolution::CatalystQuality {
                        quality_type: "Attribute".to_owned(),
                        stats: vec![StatLine {
                            stat: StatId::new("jewellery_attribute_quality"),
                            value: StatValue::Exact { min: 1, max: 1 },
                        }],
                        quality: "20".to_owned(),
                    }),
                    None,
                ),
            ),
        ),
        // 18 to 20: excluded.
        line(
            "{variant:2}+5 to Strength",
            LineKind::Modifier(ModifierLine {
                text: "+5 to Strength".to_owned(),
                prefixes: vec![Prefix::new("variant", Some("2"))],
                section: ModSection::Explicit,
                selected: false,
                resolution: LineResolution::Excluded(Exclusion::NotSelected),
            }),
        ),
        line(
            "{custom}My note",
            LineKind::Modifier(ModifierLine {
                text: "My note".to_owned(),
                prefixes: vec![Prefix::new("custom", None)],
                section: ModSection::Explicit,
                selected: true,
                resolution: LineResolution::Excluded(Exclusion::Custom),
            }),
        ),
        line(
            "Requires Class Witch",
            modifier(
                "Requires Class Witch",
                ModSection::Other("class-requirement".to_owned()),
                LineResolution::Excluded(Exclusion::Section),
            ),
        ),
        // 21 to 25: quarantined.
        line(
            "Nothing like this",
            modifier(
                "Nothing like this",
                ModSection::Explicit,
                LineResolution::Unresolved(UnresolvedReason::NoTemplate),
            ),
        ),
        line(
            "+9999 to maximum Life",
            modifier(
                "+9999 to maximum Life",
                ModSection::Explicit,
                LineResolution::Unresolved(UnresolvedReason::NoVariantFits),
            ),
        ),
        line(
            "Shadowed text",
            modifier(
                "Shadowed text",
                ModSection::Explicit,
                LineResolution::Unresolved(UnresolvedReason::OnlyShadowed),
            ),
        ),
        line(
            "Two readings",
            modifier(
                "Two readings",
                ModSection::Explicit,
                LineResolution::Unresolved(UnresolvedReason::Ambiguous(vec![
                    vec![StatId::new("a")],
                    vec![StatId::new("b")],
                ])),
            ),
        ),
        line(
            "Not yet resolved",
            modifier(
                "Not yet resolved",
                ModSection::Explicit,
                LineResolution::Pending,
            ),
        ),
    ];
    let mut layout = Ordered::new();
    layout.push_opaque(OpaqueNode::Text("an older text of the item".to_owned()));
    layout.push(ItemPart::Body);
    layout.push_opaque(OpaqueNode::Element {
        name: "ModRange".to_owned(),
        attributes: vec![Attribute::new("id", "1"), Attribute::new("range", "0.5")],
        children: Vec::new(),
    });
    Item {
        id: ItemId(id),
        body: ItemBody {
            item_class: Some("Rings".to_owned()),
            rarity: Some(Rarity::Rare),
            name: Some("Doom Loop".to_owned()),
            base: Some("Two-Stone Ring".to_owned()),
            implicits: Some(1),
            variants: VariantSelection {
                count: 2,
                selected: Some(1),
                alternates: vec![AltVariant {
                    index: 0,
                    selected: None,
                }],
            },
            lines,
        },
        layout,
        text_spacing: Vec::new(),
        attributes: Attributes::new(),
    }
}

fn gem(name: &str) -> Gem {
    Gem {
        gem: Some(GemId::new(format!("Metadata/Items/Gems/{name}"))),
        skill: Some(SkillId::new(name)),
        name: Some(name.to_owned()),
        level: Sourced::read(20, "20"),
        quality: Sourced::read(0, "0"),
        enabled: true,
        attributes: Attributes::new(),
    }
}

/// A build of `game` with one of each typed section, opaque content
/// between typed siblings, and `item`.
fn sample_build(game: Game, item: Item) -> Build {
    let mut build = Build::new(game);
    build.data = Some(DataVersion {
        game,
        schema_version: 1,
        steam_build: Some(1),
    });
    build.character.class = Some(ClassId::new("Class1"));
    build.character.ascendancy = Some(AscendancyId::new("Ascendancy1"));
    build.character.level = Some(Sourced::read(90, "90"));
    build.character.main_socket_group = Some(Sourced::read(1, "1"));

    let mut spec = PassiveSpec::new(SpecId(1));
    spec.tree_version = Some(TreeVersion::new("1_0"));
    spec.nodes = Sourced::read(vec![NodeId(10), NodeId(20)], "10,20");
    spec.mastery_effects = Sourced::read(
        vec![MasteryEffect {
            node: NodeId(20),
            effect: 7,
        }],
        "{20,7}",
    );
    spec.jewel_sockets.push(JewelSocket {
        node: NodeId(30),
        item: Some(item.id),
        attributes: Attributes::new(),
    });
    spec.layout.push_opaque(OpaqueNode::element("URL"));
    spec.layout.push(SpecPart::JewelSockets);
    build.passives.active = Some(SpecId(1));
    build.passives.specs.push(spec);

    let mut group = SocketGroup {
        label: None,
        enabled: true,
        slot: Some(SlotId::new("Weapon 1")),
        main_active_skill: Some(Sourced::read(1, "1")),
        gems: Ordered::new(),
        attributes: Attributes::new(),
    };
    group.gems.push(gem("SkillA"));
    group
        .gems
        .push_opaque(OpaqueNode::Comment("between gems".to_owned()));
    group.gems.push(gem("SupportB"));
    let mut set = SkillSet {
        id: SkillSetId(1),
        title: None,
        groups: Ordered::new(),
        attributes: Attributes::new(),
    };
    set.groups.push(group);
    build.skills.active = Some(SkillSetId(1));
    build.skills.sets.push(set);

    let mut slots = Ordered::new();
    slots.push(SlotEntry {
        slot: SlotId::new("Ring 1"),
        item: Some(item.id),
        attributes: Attributes::new(),
    });
    build.items.active_set = Some(ItemSetId(1));
    build.items.entries.push(ItemsEntry::Item(item));
    build.items.entries.push(ItemsEntry::Set(ItemSet {
        id: ItemSetId(1),
        title: None,
        slots,
        attributes: Attributes::new(),
    }));

    let mut inputs = Ordered::new();
    inputs.push(ConfigInput {
        name: "flag".to_owned(),
        value: ConfigValue::Boolean(true),
        kind: crate::ConfigInputKind::Input,
        attributes: Attributes::new(),
    });
    inputs.push(ConfigInput {
        name: "count".to_owned(),
        value: ConfigValue::Number(Sourced::read(Number(10.0), "10")),
        kind: crate::ConfigInputKind::Input,
        attributes: Attributes::new(),
    });
    inputs.push(ConfigInput {
        name: "choice".to_owned(),
        value: ConfigValue::String("Fire".to_owned()),
        kind: crate::ConfigInputKind::Input,
        attributes: Attributes::new(),
    });
    build.config.active = Some(ConfigSetId(1));
    build.config.sets.push(ConfigSet {
        id: ConfigSetId(1),
        title: Some("Default".to_owned()),
        inputs,
        custom_modifiers: Vec::new(),
        attributes: Attributes::new(),
    });

    build.layout = [
        Entry::Typed(BuildPart::Character),
        Entry::Opaque(OpaqueNode::element("Import")),
        Entry::Typed(BuildPart::Passives),
        Entry::Opaque(OpaqueNode::element("Notes")),
        Entry::Typed(BuildPart::Skills),
        Entry::Typed(BuildPart::Items),
        Entry::Typed(BuildPart::Config),
    ]
    .into_iter()
    .collect();
    build
}

fn clean_item(id: u32) -> Item {
    Item {
        id: ItemId(id),
        body: ItemBody {
            rarity: Some(Rarity::Magic),
            lines: vec![line(
                "+20 to maximum Life",
                modifier(
                    "+20 to maximum Life",
                    ModSection::Explicit,
                    resolved(stats("base_maximum_life", 20), None),
                ),
            )],
            ..ItemBody::default()
        },
        layout: [Entry::Typed(ItemPart::Body)].into_iter().collect(),
        text_spacing: Vec::new(),
        attributes: Attributes::new(),
    }
}

#[test]
fn new_build_should_hold_choices_of_its_own_game() {
    assert_eq!(
        [Game::Poe1, Game::Poe2].map(|g| Build::new(g).game_specific().game()),
        [Game::Poe1, Game::Poe2]
    );
}

#[test]
fn set_game_specific_should_refuse_other_game_choices() {
    let mut build = Build::new(Game::Poe2);
    let err = build.set_game_specific(GameSpecific::Poe1(Poe1Choices::default()));
    assert_eq!(err.map_err(|e| e.code()), Err("DOMAIN_GAME_MISMATCH"));
}

#[test]
fn poe1_build_should_keep_its_game_specific_choices() {
    let mut build = sample_build(Game::Poe1, clean_item(1));
    let choices = GameSpecific::Poe1(Poe1Choices {
        bandit: Some("None".to_owned()),
        pantheon_major: Some("Lunaris".to_owned()),
        pantheon_minor: Some("Abberath".to_owned()),
    });
    build.set_game_specific(choices.clone()).unwrap();
    assert_eq!(build.game_specific(), &choices);
}

#[test]
fn poe2_build_should_hold_every_typed_section() {
    let build = sample_build(Game::Poe2, clean_item(1));
    let counts = (
        build.passives.specs.typed().count(),
        build.skills.sets.typed().count(),
        build.items.items().count(),
        build.items.sets().count(),
        build.config.sets.typed().count(),
    );
    assert_eq!((build.game(), counts), (Game::Poe2, (1, 1, 1, 1, 1)));
}

#[test]
fn clean_build_should_be_complete() {
    let build = sample_build(Game::Poe2, clean_item(1));
    assert_eq!(build.completeness(), Completeness::Complete);
}

#[test]
fn item_should_hold_every_line_kind() {
    let item = every_line_item(1);
    let mut kinds: Vec<&str> = item.body.lines.iter().map(|l| l.kind.name()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(
        kinds,
        [
            "base",
            "flag",
            "influence",
            "item-class",
            "modifier",
            "name",
            "rarity",
            "separator",
            "spec",
            "spec-unknown",
            "unrecognized",
            "variant-base",
        ]
    );
}

#[test]
fn item_should_hold_every_typed_resolution_kind() {
    let item = every_line_item(1);
    let kinds: Vec<&str> = item
        .body
        .modifiers()
        .filter_map(|(_, m)| match &m.resolution {
            LineResolution::Resolved {
                result: Resolved::Typed(t),
                ..
            } => Some(t.kind()),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        ["granted-skill", "item-property", "catalyst-quality"]
    );
}

#[test]
fn quarantine_should_list_each_uninterpreted_line_with_its_code() {
    let build = sample_build(Game::Poe1, every_line_item(4));
    let found: Vec<(u32, usize, &str)> = build
        .quarantine()
        .iter()
        .map(|q| (q.item.0, q.line, q.reason.code()))
        .collect();
    assert_eq!(
        found,
        [
            (4, 6, "LINE_UNKNOWN_SPEC"),
            (4, 10, "LINE_UNRECOGNIZED"),
            (4, 21, "LINE_NO_TEMPLATE"),
            (4, 22, "LINE_NO_VARIANT_FITS"),
            (4, 23, "LINE_ONLY_SHADOWED"),
            (4, 24, "LINE_AMBIGUOUS"),
            (4, 25, "LINE_PENDING"),
        ]
    );
}

#[test]
fn build_with_quarantined_lines_should_be_partial() {
    let build = sample_build(Game::Poe1, every_line_item(1));
    assert_eq!(build.completeness(), Completeness::Partial);
}

#[test]
fn item_accessors_should_read_influences_and_flags_from_lines() {
    let item = every_line_item(1);
    let influences: Vec<&str> = item.body.influences().map(InfluenceId::as_str).collect();
    let flags: Vec<&str> = item.body.flags().collect();
    assert_eq!((influences, flags), (vec!["Shaper"], vec!["Corrupted"]));
}

#[test]
fn split_section_pair_should_pass_the_link_check() {
    assert_eq!(every_line_item(1).body.check_links(), Ok(()));
}

#[test]
fn continuation_of_a_non_adjacent_line_should_fail_the_link_check() {
    let mut item = every_line_item(1);
    if let LineKind::Modifier(m) = &mut item.body.lines[14].kind {
        m.resolution = LineResolution::Continues { line: 11 };
    }
    let err = item.body.check_links().map_err(|e| e.code());
    assert_eq!(err, Err("DOMAIN_BROKEN_CONTINUATION"));
}

#[test]
fn split_line_without_continuation_should_fail_the_link_check() {
    let mut item = every_line_item(1);
    if let LineKind::Modifier(m) = &mut item.body.lines[14].kind {
        m.resolution = LineResolution::Unresolved(UnresolvedReason::NoTemplate);
    }
    assert_eq!(
        item.body.check_links(),
        Err(DomainError::BrokenContinuation { line: 13 })
    );
}

#[test]
fn forward_continuation_should_fail_the_link_check() {
    let mut item = every_line_item(1);
    if let LineKind::Modifier(m) = &mut item.body.lines[11].kind {
        m.resolution = LineResolution::Continues { line: 13 };
    }
    assert_eq!(
        item.body.check_links(),
        Err(DomainError::BrokenContinuation { line: 11 })
    );
}

#[test]
fn ordered_should_keep_opaque_content_between_typed_siblings() {
    let build = sample_build(Game::Poe1, clean_item(1));
    let set = build.skills.sets.typed().next().unwrap();
    let group = set.groups.typed().next().unwrap();
    let shape: Vec<&str> = group
        .gems
        .entries()
        .iter()
        .map(|e| match e {
            Entry::Typed(g) => g.name.as_deref().unwrap_or(""),
            Entry::Opaque(_) => "opaque",
        })
        .collect();
    assert_eq!(shape, ["SkillA", "opaque", "SupportB"]);
}

#[test]
fn build_layout_should_keep_untyped_sections_in_place() {
    let build = sample_build(Game::Poe1, clean_item(1));
    let shape: Vec<String> = build
        .layout
        .entries()
        .iter()
        .map(|e| match e {
            Entry::Typed(p) => format!("{p:?}"),
            Entry::Opaque(OpaqueNode::Element { name, .. }) => name.clone(),
            Entry::Opaque(other) => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        shape,
        [
            "Character",
            "Import",
            "Passives",
            "Notes",
            "Skills",
            "Items",
            "Config"
        ]
    );
}

#[test]
fn item_layout_should_keep_other_texts_and_children_around_the_body() {
    let item = every_line_item(1);
    let shape: Vec<bool> = item
        .layout
        .entries()
        .iter()
        .map(|e| matches!(e, Entry::Typed(ItemPart::Body)))
        .collect();
    assert_eq!(shape, [false, true, false]);
}

#[test]
fn sourced_set_should_drop_the_source_text() {
    let mut level = Sourced::read(90, "90");
    level.set(91);
    assert_eq!((level.value(), level.source()), (&91, None));
}

#[test]
fn sourced_should_keep_the_source_text_while_unchanged() {
    let nodes = Sourced::read(vec![NodeId(1)], "1");
    assert_eq!(nodes.source(), Some("1"));
}

#[test]
fn number_should_equal_itself_when_nan() {
    assert_eq!(Number(f64::NAN), Number(f64::NAN));
}

#[test]
fn sourced_numbers_should_differ_by_source_text() {
    let a = Sourced::read(Number(1.0), "1");
    let b = Sourced::read(Number(1.0), "1.0");
    assert_ne!(a, b);
}

#[test]
fn stable_names_should_match_the_spike_b_report_names() {
    assert_eq!(
        [
            Adapted::CaseFolded.name(),
            Adapted::SplitSection.name(),
            Game::Poe1.name(),
            Game::Poe2.name(),
            Completeness::Partial.name(),
        ],
        ["case-folded", "split-section", "poe1", "poe2", "partial"]
    );
}

/// ADR-001: PoE1-only choices appear only in the `poe1` module. The
/// Phase 2 gate scan (3c) covers every shared crate; this one covers
/// `loom-domain`.
#[test]
fn shared_modules_should_not_name_poe1_only_choices() {
    let sources = [
        ("build.rs", include_str!("build.rs")),
        ("config.rs", include_str!("config.rs")),
        ("error.rs", include_str!("error.rs")),
        ("game.rs", include_str!("game.rs")),
        ("id.rs", include_str!("id.rs")),
        ("item/mod.rs", include_str!("item/mod.rs")),
        ("item/resolution.rs", include_str!("item/resolution.rs")),
        ("lib.rs", include_str!("lib.rs")),
        ("opaque.rs", include_str!("opaque.rs")),
        ("passive.rs", include_str!("passive.rs")),
        ("quarantine.rs", include_str!("quarantine.rs")),
        ("skill.rs", include_str!("skill.rs")),
        ("value.rs", include_str!("value.rs")),
    ];
    let found: Vec<(&str, &str)> = sources
        .iter()
        .flat_map(|(file, text)| {
            let lower = text.to_ascii_lowercase();
            ["bandit", "pantheon"]
                .into_iter()
                .filter(move |word| lower.contains(word))
                .map(move |word| (*file, word))
        })
        .collect();
    assert_eq!(found, []);
}

fn multi_line_item(resolutions: [LineResolution; 3]) -> ItemBody {
    let lines = resolutions
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let text = format!("line {i}");
            line(&text, modifier(&text, ModSection::Explicit, r))
        })
        .collect();
    ItemBody {
        lines,
        ..ItemBody::default()
    }
}

#[test]
fn same_section_multi_line_description_should_pass_the_link_check() {
    let body = multi_line_item([
        resolved(stats("some_stat", 1), None),
        LineResolution::Continues { line: 0 },
        LineResolution::Continues { line: 0 },
    ]);
    assert_eq!(body.check_links(), Ok(()));
}

#[test]
fn continuation_of_an_unresolved_line_should_fail_the_link_check() {
    let body = multi_line_item([
        LineResolution::Unresolved(UnresolvedReason::NoTemplate),
        LineResolution::Continues { line: 0 },
        LineResolution::Pending,
    ]);
    assert_eq!(
        body.check_links(),
        Err(DomainError::BrokenContinuation { line: 1 })
    );
}

#[test]
fn continuation_after_a_gap_should_fail_the_link_check() {
    let body = multi_line_item([
        resolved(stats("some_stat", 1), None),
        LineResolution::Pending,
        LineResolution::Continues { line: 0 },
    ]);
    assert_eq!(
        body.check_links(),
        Err(DomainError::BrokenContinuation { line: 2 })
    );
}

#[test]
fn excluded_lines_should_not_be_quarantined() {
    let build = sample_build(Game::Poe1, every_line_item(1));
    let lines: Vec<usize> = build.quarantine().iter().map(|q| q.line).collect();
    assert!(!lines.iter().any(|l| (18..=20).contains(l)), "{lines:?}");
}

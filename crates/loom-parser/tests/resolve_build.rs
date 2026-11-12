//! `resolve_build` (Phase 2 plan, step 6; P2-6b) over the synthetic PoE1
//! and PoE2 bundles (made-up rows; gate 3b): each outcome and each `kind`,
//! and source lines in equal source lines out.
//!
//! The default scope of both fixtures has one description per outcome
//! (`crates/loom-data/tests/fixtures/make_bundles.py`).

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the fixture bundles into a memory source; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a test over fixed fixtures; a failure should panic"
)]

mod common;

use common::bundle;
use loom_data::bundle::Bundle;
use loom_domain::{
    Adapted, Build, Completeness, Exclusion, Game, GemId, HeaderLine, Item, ItemBody, ItemId,
    ItemLine, ItemPart, ItemsEntry, LineKind, LineResolution, ModSection, ModifierLine, NoBase,
    Ordered, PropertyValue, Rarity, Resolved, StatId, StatLine, StatValue, TypedResolution,
    UnresolvedReason,
};
use loom_parser::import::{BundleData, ItemBaseName, Resolver, data_version, resolve_build};

const RING: &str = "Plain Test Ring";
const CHARM: &str = "Plain Test Charm";

fn modifier(text: &str, section: ModSection) -> ItemLine {
    ItemLine {
        source: text.to_owned(),
        kind: LineKind::Modifier(ModifierLine {
            text: text.to_owned(),
            prefixes: Vec::new(),
            section,
            selected: true,
            resolution: LineResolution::Pending,
        }),
    }
}

fn explicit(text: &str) -> ItemLine {
    modifier(text, ModSection::Explicit)
}

fn excluded(text: &str, why: Exclusion) -> ItemLine {
    let mut line = explicit(text);
    if let LineKind::Modifier(m) = &mut line.kind {
        m.resolution = LineResolution::Excluded(why);
    }
    line
}

fn other(source: &str, kind: LineKind) -> ItemLine {
    ItemLine {
        source: source.to_owned(),
        kind,
    }
}

fn item(id: u32, base: Option<&str>, lines: Vec<ItemLine>) -> Item {
    let mut layout = Ordered::new();
    layout.push(ItemPart::Body);
    Item {
        id: ItemId(id),
        body: ItemBody {
            rarity: Some(Rarity::Rare),
            base: base.map(str::to_owned),
            lines,
            ..ItemBody::default()
        },
        layout,
        text_spacing: Vec::new(),
        attributes: Vec::new(),
    }
}

fn build(game: Game, items: Vec<Item>) -> Build {
    let mut b = Build::new(game);
    for i in items {
        b.items.entries.push(ItemsEntry::Item(i));
    }
    b
}

/// The test adapter's base name: the item's base line, as written.
fn base_line(body: &ItemBody) -> Result<ItemBaseName, String> {
    body.base
        .as_ref()
        .map(|name| ItemBaseName {
            name: name.clone(),
            fallback: None,
        })
        .ok_or_else(|| "no-base-line".to_owned())
}

/// Every line's resolution of item `id` (`None` for a line that is not a
/// modifier).
fn resolutions(b: &Build, id: u32) -> Vec<Option<LineResolution>> {
    b.items
        .item(ItemId(id))
        .unwrap()
        .body
        .lines
        .iter()
        .map(|l| match &l.kind {
            LineKind::Modifier(m) => Some(m.resolution.clone()),
            _ => None,
        })
        .collect()
}

/// Every item's source lines, in order.
fn sources(b: &Build) -> Vec<Vec<String>> {
    b.items
        .items()
        .map(|i| i.body.lines.iter().map(|l| l.source.clone()).collect())
        .collect()
}

fn exact(stat: &str, v: i64) -> StatLine {
    StatLine {
        stat: StatId::new(stat),
        value: StatValue::Exact { min: v, max: v },
    }
}

fn stats(lines: Vec<StatLine>, adapted: Option<Adapted>) -> Option<LineResolution> {
    Some(LineResolution::Resolved {
        result: Resolved::Stats(lines),
        adapted,
    })
}

fn typed(t: TypedResolution) -> Option<LineResolution> {
    Some(LineResolution::Resolved {
        result: Resolved::Typed(t),
        adapted: None,
    })
}

fn unresolved(reason: UnresolvedReason) -> Option<LineResolution> {
    Some(LineResolution::Unresolved(reason))
}

fn property(id: &str, value: &str) -> TypedResolution {
    TypedResolution::ItemProperty {
        property: id.to_owned(),
        values: vec![PropertyValue {
            slot: 0,
            text: value.to_owned(),
        }],
    }
}

/// An item on the default scope's base with a line for each outcome the
/// default scope gives.
fn ring() -> Item {
    item(
        1,
        Some(RING),
        vec![
            other("Rarity: RARE", LineKind::Header(HeaderLine::Rarity)),
            explicit("+15 to maximum Life"),
            explicit("+15 TO MAXIMUM LIFE"),
            explicit("+12 to Accuracy"),
            explicit("+12 to Evasion"),
            explicit("+50 to Strength"),
            explicit("+5 to Shadowed Text"),
            explicit("Nothing like this"),
            explicit("+1 first half"),
            explicit("+2 second half"),
            explicit("While a Test Enemy is near, +15 to maximum Life"),
            other("Corrupted", LineKind::Flag),
            modifier("+3 first half", ModSection::Implicit),
            explicit("+4 second half"),
            excluded("+15 to maximum Life", Exclusion::Custom),
            explicit("Lasts 4 Seconds"),
            explicit("Test Strands: 5"),
            explicit("Quality (Test Modifiers): +20%"),
        ],
    )
}

/// The ring's resolutions on both games' fixtures; the last line's
/// catalyst quality needs the PoE1 section.
fn ring_expected(quality: Option<LineResolution>) -> Vec<Option<LineResolution>> {
    vec![
        None,
        stats(vec![exact("test_maximum_life", 15)], None),
        stats(
            vec![exact("test_maximum_life", 15)],
            Some(Adapted::CaseFolded),
        ),
        // The mod row backs the first reading; the filter keeps it.
        stats(vec![exact("test_accuracy_a", 12)], None),
        unresolved(UnresolvedReason::Ambiguous(vec![
            vec![StatId::new("test_evasion_a")],
            vec![StatId::new("test_evasion_b")],
        ])),
        unresolved(UnresolvedReason::NoVariantFits),
        unresolved(UnresolvedReason::OnlyShadowed),
        unresolved(UnresolvedReason::NoTemplate),
        stats(
            vec![exact("test_first_half", 1), exact("test_second_half", 2)],
            None,
        ),
        Some(LineResolution::Continues { line: 8 }),
        stats(
            vec![exact("test_maximum_life", 15), exact("test_presence", 1)],
            None,
        ),
        None,
        stats(
            vec![exact("test_first_half", 3), exact("test_second_half", 4)],
            Some(Adapted::SplitSection),
        ),
        Some(LineResolution::Continues { line: 12 }),
        Some(LineResolution::Excluded(Exclusion::Custom)),
        typed(TypedResolution::ItemProperty {
            property: "DisplayLastsSeconds".to_owned(),
            values: vec![PropertyValue {
                slot: 0,
                text: "4".to_owned(),
            }],
        }),
        typed(property("TestStrands", "5")),
        quality,
    ]
}

/// Items with no base: one not in the data, one the adapter names none
/// for.
fn baseless() -> [Item; 2] {
    [
        item(
            3,
            Some("Not A Test Base"),
            vec![
                explicit("+15 to maximum Life"),
                excluded("+15 to maximum Life", Exclusion::NotSelected),
            ],
        ),
        item(4, None, vec![explicit("+15 to maximum Life")]),
    ]
}

/// The charm, on the special scope, with a granted-skill line.
fn charm() -> Item {
    item(
        2,
        Some(CHARM),
        vec![
            explicit("Grants Level 5 Test Bolt Skill"),
            explicit("+15 to maximum Life"),
        ],
    )
}

fn resolved(game: Game, b: &Bundle) -> Build {
    let [a, c] = baseless();
    let mut build = build(game, vec![ring(), charm(), a, c]);
    let before = sources(&build);
    let data = BundleData::new(b);
    let completeness = resolve_build(&mut build, &data, base_line).unwrap();
    // Source lines in equal source lines out.
    assert_eq!(sources(&build), before);
    assert_eq!(completeness, Completeness::Partial);
    assert_eq!(build.data, Some(data_version(&data)));
    for i in build.items.items() {
        assert_eq!(i.body.check_links(), Ok(()), "item {}", i.id.0);
    }
    build
}

#[test]
fn poe1_should_give_each_outcome_and_kind() {
    let b = bundle("bundle-poe1");
    let build = resolved(Game::Poe1, &b);
    assert_eq!(
        resolutions(&build, 1),
        ring_expected(typed(TypedResolution::CatalystQuality {
            quality_type: "TestQuality".to_owned(),
            stats: vec![exact("test_maximum_life", 1)],
            quality: "20".to_owned(),
        }))
    );
    assert_eq!(
        resolutions(&build, 2),
        vec![
            typed(TypedResolution::GrantedSkill {
                source: "TestGraft1".to_owned(),
                gem: GemId::new("Test/Gems/Bolt"),
                level: 5,
            }),
            stats(vec![exact("test_maximum_life", 15)], None),
        ]
    );
    assert_eq!(
        resolutions(&build, 3),
        vec![
            unresolved(UnresolvedReason::NoBase(NoBase::NotFound)),
            Some(LineResolution::Excluded(Exclusion::NotSelected)),
        ]
    );
    assert_eq!(
        resolutions(&build, 4),
        vec![unresolved(UnresolvedReason::NoBase(NoBase::Unnamed(
            "no-base-line".to_owned()
        )))]
    );
}

#[test]
fn poe2_should_give_each_outcome_its_data_has() {
    let b = bundle("bundle-poe2");
    let build = resolved(Game::Poe2, &b);
    // No quality types and no granted skills in the PoE2 data: the lines
    // stay without a template.
    assert_eq!(
        resolutions(&build, 1),
        ring_expected(unresolved(UnresolvedReason::NoTemplate))
    );
    assert_eq!(
        resolutions(&build, 2),
        vec![
            unresolved(UnresolvedReason::NoTemplate),
            stats(vec![exact("test_maximum_life", 15)], None),
        ]
    );
}

#[test]
fn quarantine_should_name_every_unresolved_line() {
    let b = bundle("bundle-poe1");
    let build = resolved(Game::Poe1, &b);
    let codes: Vec<(u32, usize, &str)> = build
        .quarantine()
        .iter()
        .map(|q| (q.item.0, q.line, q.reason.code()))
        .collect();
    assert_eq!(
        codes,
        [
            (1, 4, "LINE_AMBIGUOUS"),
            (1, 5, "LINE_NO_VARIANT_FITS"),
            (1, 6, "LINE_ONLY_SHADOWED"),
            (1, 7, "LINE_NO_TEMPLATE"),
            (3, 0, "LINE_NO_BASE"),
            (4, 0, "LINE_NO_BASE"),
        ]
    );
}

#[test]
fn resolve_build_ranged_should_give_the_exact_ends_of_each_range() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let mut build = build(
        Game::Poe1,
        vec![item(
            1,
            Some(RING),
            vec![
                explicit("+15 to maximum Life"),
                explicit("+(10-20) to maximum Life"),
            ],
        )],
    );
    let resolver = Resolver::new(&data).unwrap();
    let (_, ends) = resolver
        .resolve_build_ranged(&mut build, base_line)
        .unwrap();
    // The ranged line only, its one stat with both ends in shown order.
    let line = ends.get(&ItemId(1)).unwrap();
    assert_eq!(line.keys().copied().collect::<Vec<_>>(), [1]);
    assert_eq!(
        line[&1],
        [Some(loom_parser::resolve::RangeEnds { from: 10, to: 20 })]
    );
    assert_eq!(
        resolutions(&build, 1)[1],
        stats(
            vec![StatLine {
                stat: StatId::new("test_maximum_life"),
                value: StatValue::Ranged { min: 10, max: 20 },
            }],
            None
        )
    );
    // Resolved lines are not read again: a second run gives no ends.
    let (_, again) = resolver
        .resolve_build_ranged(&mut build, base_line)
        .unwrap();
    assert!(again.is_empty());
}

#[test]
fn a_second_run_should_change_nothing() {
    let b = bundle("bundle-poe1");
    let mut build = resolved(Game::Poe1, &b);
    let once = build.clone();
    let data = BundleData::new(&b);
    resolve_build(&mut build, &data, base_line).unwrap();
    assert_eq!(build, once);
}

#[test]
fn a_base_whose_rows_disagree_on_the_scope_should_be_no_base() {
    let mut b = bundle("bundle-poe1");
    // A second row of the ring's name on the special scope's domain.
    let mut row = b
        .base_items
        .iter()
        .find(|r| r.name == CHARM)
        .unwrap()
        .clone();
    row.id = "Test/Items/Ring2".to_owned();
    row.name = RING.to_owned();
    b.base_items.push(row);
    let data = BundleData::new(&b);
    let mut build = build(
        Game::Poe1,
        vec![item(1, Some(RING), vec![explicit("+15 to maximum Life")])],
    );
    resolve_build(&mut build, &data, base_line).unwrap();
    assert_eq!(
        resolutions(&build, 1),
        vec![unresolved(UnresolvedReason::NoBase(NoBase::ScopeConflict))]
    );
}

#[test]
fn the_adapters_fallback_name_should_be_looked_up() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let mut build = build(
        Game::Poe1,
        vec![item(
            1,
            Some("Plain Test Ring (Old)"),
            vec![explicit("+15 to maximum Life")],
        )],
    );
    resolve_build(&mut build, &data, |body| {
        let name = body.base.clone().ok_or_else(String::new)?;
        let fallback = name.strip_suffix(" (Old)").map(str::to_owned);
        Ok(ItemBaseName { name, fallback })
    })
    .unwrap();
    assert_eq!(
        resolutions(&build, 1),
        vec![stats(vec![exact("test_maximum_life", 15)], None)]
    );
}

#[test]
fn another_games_bundle_should_be_refused_and_change_nothing() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let mut build = build(Game::Poe2, vec![ring()]);
    let before = build.clone();
    let err = resolve_build(&mut build, &data, base_line).unwrap_err();
    assert_eq!((err.code(), build), ("IMPORT_GAME_MISMATCH", before));
}

#[test]
fn a_build_recording_other_data_should_be_refused() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let mut build = build(Game::Poe1, vec![ring()]);
    let mut other = data_version(&data);
    other.steam_build = Some(1);
    build.data = Some(other);
    let before = build.clone();
    let err = resolve_build(&mut build, &data, base_line).unwrap_err();
    assert_eq!(
        (err.code(), build),
        ("IMPORT_DATA_VERSION_MISMATCH", before)
    );
}

#[test]
fn a_bundle_with_skipped_references_should_be_refused() {
    let mut b = bundle("bundle-poe1");
    b.base_items[0].implicits.push("AbsentMod".to_owned());
    let data = BundleData::new(&b);
    let err = Resolver::new(&data).unwrap_err();
    assert_eq!(err.code(), "IMPORT_DATA_UNVALIDATED");
}

/// Custom modifier lines (P3-12c): each read alone in the default scope,
/// with no base, in every block of every set, enabled or not, and in a
/// config input (PoB's legacy `customMods`, P3-12d).
#[test]
fn custom_modifier_lines_should_be_resolved_alone_without_a_base() {
    use loom_domain::{
        ConfigInput, ConfigInputKind, ConfigSet, ConfigSetId, ConfigValue, CustomModifierBlock,
        CustomModifierLine,
    };

    let texts = [
        "+15 to maximum Life",
        "+15 TO MAXIMUM LIFE",
        "+(10-20) to maximum Life",
        "+12 to Accuracy",
        "+12 to Evasion",
        "+5 to Shadowed Text",
        "Nothing like this",
        "+1 first half",
    ];
    let pending = || -> Vec<_> {
        texts
            .iter()
            .map(|t| CustomModifierLine {
                text: (*t).to_owned(),
                resolution: LineResolution::Pending,
            })
            .collect()
    };
    let block = |enabled: bool| CustomModifierBlock {
        title: None,
        enabled,
        text: String::new(),
        lines: pending(),
        attributes: Vec::new(),
    };
    let mut inputs = Ordered::new();
    inputs.push(ConfigInput {
        name: "customMods".to_owned(),
        value: ConfigValue::String(texts.join(
            "
",
        )),
        kind: ConfigInputKind::Input,
        lines: pending(),
        attributes: Vec::new(),
    });
    let mut b = build(Game::Poe1, Vec::new());
    b.config.sets.push(ConfigSet {
        id: ConfigSetId(1),
        title: None,
        inputs,
        custom_modifiers: vec![block(true), block(false)],
        attributes: Vec::new(),
    });
    let bundle = bundle("bundle-poe1");
    let data = BundleData::new(&bundle);
    // No item, but unresolved custom lines: the build is not complete.
    assert_eq!(
        resolve_build(&mut b, &data, base_line).unwrap(),
        Completeness::Partial
    );
    let expected = vec![
        stats(vec![exact("test_maximum_life", 15)], None),
        stats(
            vec![exact("test_maximum_life", 15)],
            Some(Adapted::CaseFolded),
        ),
        stats(
            vec![StatLine {
                stat: StatId::new("test_maximum_life"),
                value: StatValue::Ranged { min: 10, max: 20 },
            }],
            None,
        ),
        // No base, so no mod row settles it (an item line's does).
        unresolved(UnresolvedReason::Ambiguous(vec![
            vec![StatId::new("test_accuracy_a")],
            vec![StatId::new("test_accuracy_b")],
        ])),
        unresolved(UnresolvedReason::Ambiguous(vec![
            vec![StatId::new("test_evasion_a")],
            vec![StatId::new("test_evasion_b")],
        ])),
        unresolved(UnresolvedReason::OnlyShadowed),
        unresolved(UnresolvedReason::NoTemplate),
        // Half of a two-line description, read alone.
        unresolved(UnresolvedReason::NoTemplate),
    ];
    let set = b.config.sets.typed().next().unwrap();
    for blk in &set.custom_modifiers {
        let got: Vec<_> = blk
            .lines
            .iter()
            .map(|l| Some(l.resolution.clone()))
            .collect();
        assert_eq!(got, expected);
    }
    let input = set.inputs.typed().next().unwrap();
    let got: Vec<_> = input
        .lines
        .iter()
        .map(|l| Some(l.resolution.clone()))
        .collect();
    assert_eq!(got, expected);
    // Resolved lines are kept: a second run changes nothing.
    let before = b.clone();
    resolve_build(&mut b, &data, base_line).unwrap();
    assert_eq!(b, before);
    // Only resolved lines left: complete.
    let set = b.config.sets.typed_mut().next().unwrap();
    for blk in &mut set.custom_modifiers {
        blk.lines.truncate(3);
    }
    assert_eq!(b.completeness(), Completeness::Partial, "the input's lines");
    let set = b.config.sets.typed_mut().next().unwrap();
    for input in set.inputs.typed_mut() {
        input.lines.truncate(3);
    }
    assert_eq!(b.completeness(), Completeness::Complete);
}

//! The tooltip's inferred mods and tiers (B-2, PR 2,
//! `loom_parser::import::affixes`), over the synthetic fixture bundle with
//! made-up rows added per test.

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the fixture bundles into a memory source; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a test over fixed fixtures; a failure should panic"
)]

mod common;

use loom_data::bundle::{Bundle, ModEntry, ModStat, SpawnWeight};
use loom_data::rules::AffixGenerations;
use loom_domain::{
    ItemBody, ItemLine, LineKind, LineResolution, ModSection, ModifierLine, Prefix, Rarity,
    Resolved, StatId, StatLine, StatValue,
};
use loom_parser::import::affixes::{Affix, AffixKind, item_affixes};
use loom_parser::import::{BaseQuery, BundleData, find_base};

const LIFE: &str = "test_maximum_life";
const ARMOUR: &str = "test_local_armour";
const ACCURACY: &str = "test_accuracy_a";

/// A made-up row: `(stat, min, max)`, spawn `(tag, weight)`.
fn entry(
    id: &str,
    generation: i64,
    mod_type: &str,
    level: i64,
    stats: &[(&str, i64, i64)],
    spawn: &[(&str, i64)],
) -> ModEntry {
    ModEntry {
        id: id.to_owned(),
        domain: 1,
        generation,
        mod_type: Some(mod_type.to_owned()),
        stats: stats
            .iter()
            .map(|(s, min, max)| ModStat {
                stat: (*s).to_owned(),
                min: *min,
                max: *max,
            })
            .collect(),
        spawn: spawn
            .iter()
            .map(|(t, w)| SpawnWeight {
                tag: (*t).to_owned(),
                weight: *w,
            })
            .collect(),
        influence: None,
        craft_classes: Vec::new(),
        essence_only: false,
        essence_classes: None,
        implicit_tags: Vec::new(),
        name: id.to_owned(),
        level: Some(level),
    }
}

/// The fixture bundle with the game's affix generations (prefix 1, suffix
/// 2) and three tiers of one life mod type: `TestLife1` (the fixture's,
/// level 1, 10 to 19), `TestLife2` (level 20, 20 to 29), and `TestLife3`
/// (level 40, 30 to 39, spawning only on wands).
fn bundle() -> Bundle {
    let mut b = common::bundle("bundle-poe1");
    b.item_rules.mods.affix_generations = AffixGenerations {
        prefix: vec![1],
        suffix: vec![2],
    };
    for m in b.mods.iter_mut().filter(|m| m.id == "TestLife1") {
        m.mod_type = Some("TestLife".to_owned());
        m.level = Some(1);
    }
    b.mods.push(entry(
        "TestLife2",
        1,
        "TestLife",
        20,
        &[(LIFE, 20, 29)],
        &[("default", 1000)],
    ));
    b.mods.push(entry(
        "TestLife3",
        1,
        "TestLife",
        40,
        &[(LIFE, 30, 39)],
        &[("wand", 1000), ("default", 0)],
    ));
    b
}

fn line(text: &str, stats: &[(&str, i64)], section: ModSection, flag: Option<&str>) -> ItemLine {
    ItemLine {
        source: text.to_owned(),
        kind: LineKind::Modifier(ModifierLine {
            text: text.to_owned(),
            prefixes: flag
                .map(|f| Prefix {
                    key: f.to_owned(),
                    value: None,
                })
                .into_iter()
                .collect(),
            section,
            selected: true,
            resolution: LineResolution::Resolved {
                result: Resolved::Stats(
                    stats
                        .iter()
                        .map(|(s, v)| StatLine {
                            stat: StatId::new(*s),
                            value: StatValue::Exact { min: *v, max: *v },
                        })
                        .collect(),
                ),
                adapted: None,
            },
        }),
    }
}

fn explicit(text: &str, stats: &[(&str, i64)]) -> ItemLine {
    line(text, stats, ModSection::Explicit, None)
}

fn ring(lines: Vec<ItemLine>) -> ItemBody {
    ItemBody {
        rarity: Some(Rarity::Rare),
        lines,
        ..ItemBody::default()
    }
}

/// The item's affixes on a Plain Test Ring.
fn on_ring(bundle: &Bundle, body: &ItemBody) -> Vec<Affix> {
    let data = BundleData::new(bundle);
    let base = find_base(
        &data,
        BaseQuery {
            name: "Plain Test Ring",
            fallback: None,
        },
    )
    .unwrap();
    item_affixes(body, &data, Some(base.rows))
}

/// `(kind, mod ID, tier, lines)` of each affix.
fn summary(affixes: &[Affix]) -> Vec<(AffixKind, &str, Option<u32>, Vec<usize>)> {
    affixes
        .iter()
        .map(|a| (a.kind, a.mod_id.as_str(), a.tier, a.lines.clone()))
        .collect()
}

#[test]
fn a_line_with_one_reading_should_get_its_mod_and_tier() {
    let b = bundle();
    let body = ring(vec![
        explicit("+25 to maximum Life", &[(LIFE, 25)]),
        explicit("+15 to Accuracy", &[(ACCURACY, 15)]),
    ]);
    let a = on_ring(&b, &body);
    // TestLife3 does not spawn on rings: TestLife2 is tier 1. Accuracy has
    // no mod type: no tier.
    assert_eq!(
        summary(&a),
        [
            (AffixKind::Prefix, "TestLife2", Some(1), vec![0]),
            (AffixKind::Prefix, "TestAccuracy1", None, vec![1]),
        ]
    );
    assert!(a.iter().all(|x| x.inferred && !x.crafted));
    // The lower tier is tier 2.
    let body = ring(vec![explicit("+15 to maximum Life", &[(LIFE, 15)])]);
    assert_eq!(
        summary(&on_ring(&b, &body)),
        [(AffixKind::Prefix, "TestLife1", Some(2), vec![0])]
    );
}

#[test]
fn a_line_with_two_readings_should_get_no_header() {
    let mut b = bundle();
    // A suffix that makes the same line.
    b.mods.push(entry(
        "TestLifeSuffix",
        2,
        "TestLifeSuffix",
        30,
        &[(LIFE, 22, 26)],
        &[("default", 1000)],
    ));
    let body = ring(vec![
        explicit("+25 to maximum Life", &[(LIFE, 25)]),
        explicit("+15 to Accuracy", &[(ACCURACY, 15)]),
    ]);
    // The accuracy line is a group of its own: it keeps its header.
    assert_eq!(
        summary(&on_ring(&b, &body)),
        [(AffixKind::Prefix, "TestAccuracy1", None, vec![1])]
    );
}

#[test]
fn a_summed_line_should_go_under_the_hybrid_and_the_pure_mod() {
    let mut b = bundle();
    b.mods.push(entry(
        "TestHybrid1",
        1,
        "TestHybrid",
        10,
        &[(ARMOUR, 3, 4), (LIFE, 5, 6)],
        &[("default", 1000)],
    ));
    // 4 armour: only the hybrid (TestArmour1 is 5 to 9); 30 life: the
    // hybrid's 5 to 6 and TestLife2's 20 to 29, apart from the armour line.
    let body = ring(vec![
        explicit("+30 to maximum Life", &[(LIFE, 30)]),
        explicit("+4 to Armour", &[(ARMOUR, 4)]),
    ]);
    assert_eq!(
        summary(&on_ring(&b, &body)),
        [
            (AffixKind::Prefix, "TestLife2", Some(1), vec![0]),
            (AffixKind::Prefix, "TestHybrid1", Some(1), vec![0, 1]),
        ]
    );
}

#[test]
fn no_mod_should_be_inferred_without_the_rules_affix_generations_or_on_a_unique() {
    let mut b = bundle();
    let body = ring(vec![explicit("+25 to maximum Life", &[(LIFE, 25)])]);
    let mut unique = body.clone();
    unique.rarity = Some(Rarity::Unique);
    assert_eq!(on_ring(&b, &unique), []);
    b.item_rules.mods.affix_generations = AffixGenerations::default();
    assert_eq!(on_ring(&b, &body), []);
}

#[test]
fn a_named_mod_should_get_its_tier_and_keep_its_lines() {
    let b = bundle();
    let mut body = ring(vec![explicit("+25 to maximum Life", &[(LIFE, 25)])]);
    body.lines.insert(
        0,
        ItemLine {
            source: "Prefix: {range:0.5}TestLife2".to_owned(),
            kind: LineKind::Spec {
                key: "Prefix".to_owned(),
                value: "{range:0.5}TestLife2".to_owned(),
                known: true,
            },
        },
    );
    let a = on_ring(&b, &body);
    assert_eq!(
        summary(&a),
        [(AffixKind::Prefix, "TestLife2", Some(1), vec![1])]
    );
    assert!(!a[0].inferred);
}

#[test]
fn crafted_and_implicit_lines_should_be_headed_apart() {
    let mut b = bundle();
    // A bench row (no spawn weight) and the ring's own implicit row.
    b.mods.push(entry(
        "TestCraftedLife",
        1,
        "TestCraftedLife",
        1,
        &[(LIFE, 40, 45)],
        &[],
    ));
    b.mods.push(entry(
        "TestRingImplicit",
        3,
        "TestRingImplicit",
        1,
        &[(ACCURACY, 5, 5)],
        &[],
    ));
    for base in b
        .base_items
        .iter_mut()
        .filter(|x| x.name == "Plain Test Ring")
    {
        base.implicits = vec!["TestRingImplicit".to_owned()];
    }
    let body = ring(vec![
        line(
            "+5 to Accuracy",
            &[(ACCURACY, 5)],
            ModSection::Implicit,
            None,
        ),
        line(
            "+42 to maximum Life",
            &[(LIFE, 42)],
            ModSection::Explicit,
            Some("crafted"),
        ),
    ]);
    let a = on_ring(&b, &body);
    assert_eq!(
        summary(&a),
        [
            (AffixKind::Implicit, "TestRingImplicit", None, vec![0]),
            (AffixKind::Prefix, "TestCraftedLife", None, vec![1]),
        ]
    );
    assert_eq!(
        a.iter().map(|x| x.crafted).collect::<Vec<_>>(),
        [false, true]
    );
}

#[test]
fn a_row_with_a_stat_no_line_shows_should_count_as_a_reading() {
    let mut b = bundle();
    // A hybrid whose second stat the line carries with no shown value.
    b.mods.push(entry(
        "TestLifeFlag",
        1,
        "TestLifeFlag",
        10,
        &[(LIFE, 22, 27), ("test_flag", 1, 1)],
        &[("default", 1000)],
    ));
    let mut life = explicit("+25 to maximum Life", &[(LIFE, 25)]);
    if let LineKind::Modifier(m) = &mut life.kind
        && let LineResolution::Resolved {
            result: Resolved::Stats(stats),
            ..
        } = &mut m.resolution
    {
        stats.push(StatLine {
            stat: StatId::new("test_flag"),
            value: StatValue::Hidden { assumed: Some(1) },
        });
    }
    // TestLife2 or the hybrid: two readings, no header.
    assert_eq!(on_ring(&b, &ring(vec![life])), []);
}

#[test]
fn a_mod_type_a_named_mod_holds_should_not_be_inferred_again() {
    let mut b = bundle();
    // A row of the named mod's type that also makes the accuracy line.
    b.mods.push(entry(
        "TestLifeAccuracy",
        1,
        "TestLife",
        10,
        &[(ACCURACY, 14, 16)],
        &[("default", 1000)],
    ));
    let accuracy = || explicit("+15 to Accuracy", &[(ACCURACY, 15)]);
    // Unnamed: TestAccuracy1 or TestLifeAccuracy, no header.
    assert_eq!(on_ring(&b, &ring(vec![accuracy()])), []);
    // TestLife2 named: its type is taken, TestAccuracy1 is the one reading.
    let mut body = ring(vec![
        explicit("+25 to maximum Life", &[(LIFE, 25)]),
        accuracy(),
    ]);
    body.lines.insert(
        0,
        ItemLine {
            source: "Prefix: TestLife2".to_owned(),
            kind: LineKind::Spec {
                key: "Prefix".to_owned(),
                value: "TestLife2".to_owned(),
                known: true,
            },
        },
    );
    assert_eq!(
        summary(&on_ring(&b, &body)),
        [
            (AffixKind::Prefix, "TestLife2", Some(1), vec![1]),
            (AffixKind::Prefix, "TestAccuracy1", None, vec![2]),
        ]
    );
}

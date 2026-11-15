//! The tooltip's affix headers from PoB's `Prefix:`/`Suffix:` lines (B-2,
//! `loom_parser::import::affixes`), over the synthetic fixture bundle.

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the fixture bundles into a memory source; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test over fixed fixtures; a failure should panic"
)]

mod common;

use loom_data::bundle::{ModEntry, ModStat};
use loom_domain::{
    ItemBody, ItemLine, LineKind, LineResolution, ModSection, ModifierLine, Prefix, Resolved,
    StatId, StatLine, StatValue,
};
use loom_parser::import::affixes::{AffixKind, named_affixes};

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

/// An explicit line resolved to `stats` (exact values).
fn line(text: &str, stats: &[(&str, i64)], crafted: bool) -> ItemLine {
    ItemLine {
        source: text.to_owned(),
        kind: LineKind::Modifier(ModifierLine {
            text: text.to_owned(),
            prefixes: if crafted {
                vec![Prefix {
                    key: "crafted".to_owned(),
                    value: None,
                }]
            } else {
                Vec::new()
            },
            section: ModSection::Explicit,
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

fn item(lines: Vec<ItemLine>) -> ItemBody {
    ItemBody {
        lines,
        ..ItemBody::default()
    }
}

#[test]
fn named_affixes_should_group_lines_under_their_mod_with_its_name_and_tags() {
    let bundle = common::bundle("bundle-poe1");
    let body = item(vec![
        spec("Prefix", "{range:0.5}TestAccuracy1"),
        spec("Prefix", "None"),
        spec("Suffix", "{range:1}TestArmour1"),
        // Lines in stat-description order, not mod order.
        line("+9 to Armour", &[("test_local_armour", 9)], false),
        line("+15 to Accuracy", &[("test_accuracy_a", 15)], false),
        line("+12 to Accuracy", &[("test_accuracy_a", 12)], true),
    ]);
    let affixes = named_affixes(&body, &bundle);
    let got: Vec<_> = affixes
        .iter()
        .map(|a| {
            (
                a.kind,
                a.name.as_deref(),
                a.level,
                a.tags.clone(),
                a.lines.clone(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            // A crafted line is not the named mod's.
            (
                AffixKind::Prefix,
                Some("Steady"),
                Some(12),
                vec!["Attack".to_owned()],
                vec![4]
            ),
            (
                AffixKind::Suffix,
                Some("of Test Plating"),
                Some(5),
                vec!["Defences".to_owned()],
                vec![3]
            ),
        ]
    );
}

#[test]
fn a_spec_should_be_dropped_when_unknown_or_no_line_fits_it() {
    let bundle = common::bundle("bundle-poe1");
    let body = item(vec![
        spec("Prefix", "{range:0.5}NotInTheBundle"),
        spec("Suffix", "TestArmour1"),
        // Edited by hand: 40 is outside 5 to 9.
        line("+40 to Armour", &[("test_local_armour", 40)], false),
    ]);
    assert_eq!(named_affixes(&body, &bundle), []);
}

#[test]
fn a_summed_line_should_go_under_both_mods_only_when_the_sum_fits() {
    let mut bundle = common::bundle("bundle-poe1");
    // A hybrid with armour and life, next to the pure armour mod.
    bundle.mods.push(ModEntry {
        id: "TestHybrid1".to_owned(),
        domain: 1,
        generation: 1,
        mod_type: None,
        stats: vec![
            ModStat {
                stat: "test_local_armour".to_owned(),
                min: 3,
                max: 4,
            },
            ModStat {
                stat: "test_maximum_life".to_owned(),
                min: 5,
                max: 6,
            },
        ],
        spawn: Vec::new(),
        influence: None,
        craft_classes: Vec::new(),
        essence_only: false,
        essence_classes: None,
        implicit_tags: Vec::new(),
        name: "Test Hybrid".to_owned(),
        level: Some(1),
    });
    let with_armour = |armour: i64| {
        item(vec![
            spec("Prefix", "TestHybrid1"),
            spec("Suffix", "TestArmour1"),
            line("+armour", &[("test_local_armour", armour)], false),
            line("+5 to Life", &[("test_maximum_life", 5)], false),
        ])
    };
    let lines = |body: &ItemBody| {
        named_affixes(body, &bundle)
            .into_iter()
            .map(|a| (a.mod_id, a.lines))
            .collect::<Vec<_>>()
    };
    // 12 is in 8 to 13: the armour line is both mods'.
    assert_eq!(
        lines(&with_armour(12)),
        [
            ("TestHybrid1".to_owned(), vec![2, 3]),
            ("TestArmour1".to_owned(), vec![2])
        ]
    );
    // 20 fits neither sum: the line goes under no header.
    assert_eq!(
        lines(&with_armour(20)),
        [("TestHybrid1".to_owned(), vec![3])]
    );
}

#[test]
fn an_older_bundle_should_give_headers_without_names_or_tags() {
    let mut bundle = common::bundle("bundle-poe1");
    bundle.manifest.data_schema = 3;
    let body = item(vec![
        spec("Suffix", "TestArmour1"),
        line("+9 to Armour", &[("test_local_armour", 9)], false),
    ]);
    let a = named_affixes(&body, &bundle);
    assert_eq!(a.len(), 1);
    assert_eq!(
        (a[0].name.clone(), a[0].level, a[0].tags.len()),
        (None, None, 0)
    );
}

#[test]
fn the_rules_tag_names_should_name_and_hide_tags() {
    let mut bundle = common::bundle("bundle-poe1");
    // The rules name `attack` only: `defences` is not shown.
    bundle.item_rules.text.mod_tag_names = vec![("attack".to_owned(), "Attacks".to_owned())];
    let body = item(vec![
        spec("Prefix", "TestAccuracy1"),
        spec("Suffix", "TestArmour1"),
        line("+9 to Armour", &[("test_local_armour", 9)], false),
        line("+15 to Accuracy", &[("test_accuracy_a", 15)], false),
    ]);
    let tags: Vec<_> = named_affixes(&body, &bundle)
        .into_iter()
        .map(|a| (a.mod_id, a.tags))
        .collect();
    assert_eq!(
        tags,
        [
            ("TestAccuracy1".to_owned(), vec!["Attacks".to_owned()]),
            ("TestArmour1".to_owned(), Vec::new()),
        ]
    );
}

//! `BundleData` (Phase 2 plan, step 6; P2-6a) over the synthetic PoE1 and
//! PoE2 bundles of `loom-data` (`crates/loom-data/tests/fixtures/`, made-up
//! rows; plan decision 10). Each bundle is validated before use, as the app
//! does.

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the fixture bundles into a memory source; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test over fixed fixtures; a failure should panic"
)]

mod common;

use std::path::PathBuf;

use common::{bundle, source};
use loom_data::bundle::poe1::{GraftBase, Grafts as BundleGrafts};
use loom_data::bundle::{Bundle, ClientText, ModEntry, ModStat, validate};
use loom_parser::import::mods::Locality;
use loom_parser::import::{BaseQuery, BundleData, ItemData, find_base};
use loom_parser::resolve::Outcome;

fn mod_entry(id: &str, mod_type: Option<&str>, stats: &[(&str, i64, i64)]) -> ModEntry {
    ModEntry {
        id: id.to_owned(),
        domain: 1,
        generation: 1,
        mod_type: mod_type.map(str::to_owned),
        stats: stats
            .iter()
            .map(|(s, min, max)| ModStat {
                stat: (*s).to_owned(),
                min: *min,
                max: *max,
            })
            .collect(),
        spawn: Vec::new(),
        influence: None,
        craft_classes: vec!["TestRing".to_owned(), "TestCharm".to_owned()],
        essence_only: false,
        essence_classes: Some(vec!["TestRing".to_owned()]),
        implicit_tags: Vec::new(),
    }
}

#[test]
fn bases_should_map_classes_and_implicits_to_row_numbers() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let (name, rows) = data.base("Plain Test Ring").unwrap();
    let row = rows.first().unwrap();
    assert_eq!(
        (
            name,
            rows.len(),
            row.domain,
            row.class,
            row.implicits.clone(),
            row.tags.clone(),
            data.tags_unknown(),
        ),
        (
            "Plain Test Ring",
            1,
            1,
            Some(1),
            vec![0],
            Some(vec!["ring".to_owned(), "default".to_owned()]),
            0,
        )
    );
    let names: Vec<&str> = data.base_names().collect();
    assert_eq!(
        names,
        [
            "Plain Test Charm",
            "Plain Test Ring",
            "Plain Test Shield",
            "Plain Test Wand",
            "Test Bolt",
            "Test Damage Support"
        ]
    );
}

#[test]
fn base_lookup_should_give_each_domain_its_scope() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let scope = |name: &str| {
        find_base(
            &data,
            BaseQuery {
                name,
                fallback: None,
            },
        )
        .unwrap()
        .scope
    };
    assert_eq!(
        (scope("Plain Test Ring"), scope("Plain Test Charm")),
        ("stat_descriptions", "charm_stat_descriptions")
    );
}

#[test]
fn mods_should_split_unset_stats_and_number_types_and_classes() {
    let mut b = bundle("bundle-poe1");
    b.mods.push(mod_entry(
        "A",
        Some("TypeX"),
        &[("test_maximum_life", 5, 1), ("test_local_armour", 0, 0)],
    ));
    b.mods.push(mod_entry("B", Some("TypeY"), &[]));
    b.mods.push(mod_entry("C", Some("TypeX"), &[]));
    let data = BundleData::new(&b);
    let mods = data.mods();
    let row = |n: usize| mods.row(n).unwrap();
    // The fixture has 3 rows; the pushed ones are rows 3 to 5.
    let a = row(3);
    assert_eq!(
        (
            mods.row_count(),
            a.stats.clone(),
            a.unset.clone(),
            a.craft_classes.clone(),
            a.essence_classes.clone(),
        ),
        (
            6,
            vec![("test_maximum_life".to_owned(), 1, 5)],
            vec!["test_local_armour".to_owned()],
            vec![0, 1],
            Some(vec![1]),
        )
    );
    assert_eq!(a.mod_type, row(5).mod_type);
    assert_ne!(a.mod_type, row(4).mod_type);
    assert_eq!(row(0).mod_type, None);
    assert_eq!(
        mods.locality.get("test_local_armour"),
        Some(&Locality::Item)
    );
    assert_eq!(mods.locality.get("test_maximum_life"), None);
}

#[test]
fn poe1_section_should_give_grafts_gems_and_quality_types() {
    let mut b = bundle("bundle-poe1");
    b.client_texts.push(ClientText {
        id: "TestGraftSkill".to_owned(),
        text: "Uses level {1} {0}".to_owned(),
    });
    b.item_rules.text.granted_skill_wrapper = Some("TestGraftSkill".to_owned());
    let data = BundleData::new(&b);
    let g = data
        .grafts()
        .resolve("Plain Test Charm", "Uses level 5 Test Bolt")
        .unwrap();
    assert_eq!(
        (
            g.graft.id.as_str(),
            g.level,
            data.granted_gem("TestGraft1"),
            data.properties().inputs(),
        ),
        ("TestGraft1", 5, Some("Test/Gems/Bolt"), (1, 1, 1))
    );
}

#[test]
fn graft_rows_naming_two_grafts_should_be_dropped() {
    let mut b = bundle("bundle-poe1");
    let grafts: &mut BundleGrafts = b.poe1.as_mut().unwrap().grafts.as_mut().unwrap();
    grafts.bases.push(GraftBase {
        base: "Plain Test Charm".to_owned(),
        graft: "TestGraft2".to_owned(),
        gem: "Test/Gems/Bolt".to_owned(),
        skill: "Test Bolt".to_owned(),
    });
    let data = BundleData::new(&b);
    assert_eq!(
        (data.grafts().len(), data.grafts().conflicts()),
        (0, ["Plain Test Charm".to_owned()].as_slice())
    );
}

#[test]
fn compositions_should_be_read_in_rule_order_and_missing_ones_listed() {
    let mut b = bundle("bundle-poe1");
    b.client_texts.push(ClientText {
        id: "TestWrap".to_owned(),
        text: "Wrapped: {0}".to_owned(),
    });
    b.client_texts.push(ClientText {
        id: "TestBadWrap".to_owned(),
        text: "{0} and {1}".to_owned(),
    });
    b.item_rules.text.compositions = vec![
        ("TestBadWrap".to_owned(), "test_maximum_life".to_owned()),
        ("TestWrap".to_owned(), "test_maximum_life".to_owned()),
        ("TestAbsent".to_owned(), "test_maximum_life".to_owned()),
    ];
    let data = BundleData::new(&b);
    let read: Vec<&str> = data.compositions().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        (read, data.compositions_missing()),
        (vec!["TestWrap"], vec!["TestBadWrap", "TestAbsent"])
    );
}

#[test]
fn poe2_bundle_should_build_without_a_poe1_section() {
    let b = bundle("bundle-poe2");
    let data = BundleData::new(&b);
    let indexes = data.indexes().unwrap();
    assert_eq!(
        (
            b.poe1.is_none(),
            data.grafts().is_empty(),
            data.properties().inputs().2,
            indexes.keys().copied().collect::<Vec<_>>(),
        ),
        (
            true,
            true,
            0,
            vec!["charm_stat_descriptions", "stat_descriptions"]
        )
    );
}

#[test]
fn indexes_should_cover_every_scope_and_resolve_a_line() {
    let b = bundle("bundle-poe1");
    let data = BundleData::new(&b);
    let indexes = data.indexes().unwrap();
    let Outcome::Resolved(c) = indexes
        .get("stat_descriptions")
        .unwrap()
        .resolve("+12 to maximum Life")
        .outcome
    else {
        panic!("the fixture line did not resolve");
    };
    let stats: Vec<&str> = c.stats.iter().map(|s| s.stat.as_str()).collect();
    assert_eq!(
        (indexes.len(), stats),
        (b.item_rules.scopes().len(), vec!["test_maximum_life"])
    );
}

#[test]
fn a_scope_without_a_file_should_give_bundle_scope() {
    let mut b = bundle("bundle-poe1");
    b.item_rules.default_scope = "absent_stat_descriptions".to_owned();
    let data = BundleData::new(&b);
    let Err(e) = data.indexes() else {
        panic!("a missing scope file built an index");
    };
    assert_eq!(
        (e.code(), e.to_string()),
        (
            "BUNDLE_SCOPE",
            "absent_stat_descriptions: no stat-description file".to_owned()
        )
    );
}

/// Maintainer-run: a real bundle in the folder `LOOM_BUNDLE_DIR` (made by
/// `loom-parity-cli bundle`; local, ADR-007) validates, builds, and indexes
/// every scope, with one mod row per bundle mod and one base row per base
/// item. Only counts are printed. `LOOM_BUNDLE_DIR=<dir> cargo test -p
/// loom-parser --test bundle_data -- --ignored --nocapture`.
#[test]
#[ignore = "needs LOOM_BUNDLE_DIR (a local bundle)"]
#[expect(
    clippy::print_stdout,
    reason = "a maintainer-run test reports its counts"
)]
fn real_bundle_should_build() {
    let src = source(&PathBuf::from(std::env::var("LOOM_BUNDLE_DIR").unwrap()));
    let report = validate::validate(&src);
    assert!(report.is_valid(), "{:?}", report.codes());
    let b = Bundle::load(&src).unwrap();
    let data = BundleData::new(&b);
    let indexes = data.indexes().unwrap();
    let base_rows: usize = data
        .base_names()
        .filter_map(|n| data.base(n))
        .map(|(_, rows)| rows.len())
        .sum();
    assert_eq!(
        (data.mods().row_count(), base_rows, indexes.len()),
        (
            b.mods.len(),
            b.base_items.len(),
            b.item_rules.scopes().len()
        )
    );
    let (templates, labels, qualities) = data.properties().inputs();
    println!(
        "{}: {} mod rows, {} base rows ({} names, {} tags unknown), {} scopes indexed, \
         {} granted-skill bases, properties {templates}/{labels}/{qualities}, \
         {} compositions ({} missing)",
        b.manifest.game,
        data.mods().row_count(),
        base_rows,
        data.base_names().count(),
        data.tags_unknown(),
        indexes.len(),
        data.grafts().len(),
        data.compositions().len(),
        data.compositions_missing().len(),
    );
}

#[test]
fn dangling_references_should_be_skipped_and_counted() {
    let mut b = bundle("bundle-poe1");
    b.base_items[0].implicits.push("AbsentMod".to_owned());
    b.base_items[0].item_class = Some("AbsentClass".to_owned());
    b.mods[0].craft_classes = vec!["AbsentClass".to_owned(), "TestRing".to_owned()];
    let data = BundleData::new(&b);
    let row = data
        .base("Plain Test Ring")
        .unwrap()
        .1
        .first()
        .unwrap()
        .clone();
    assert_eq!(
        (
            data.skipped_references(),
            row.implicits,
            row.class,
            data.mods().row(0).unwrap().craft_classes.clone(),
        ),
        (3, vec![0], None, vec![1])
    );
}

//! `CalcData::item_base` on a real bundle: the synthetic PoE1 fixture
//! bundle of `loom-data` (made-up rows), read from its folder as the app
//! reads a bundle. The calculation tests use a stub; this checks the join
//! of base items, armour types, shield types, and weapon types.

use std::path::{Path, PathBuf};

use loom_calc::{ArmourBase, CalcData, ItemBase, Range};
use loom_data::bundle::properties::{ArmourType, ShieldType, ValueRange};
use loom_data::bundle::{BaseItem, Bundle, MemorySource};

#[expect(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "a test helper reads the fixture bundle (the crate itself does no I/O); a failure fails the test"
)]
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[expect(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "a test helper reads the fixture bundle (the crate itself does no I/O); a failure fails the test"
)]
fn bundle() -> Bundle {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../loom-data/tests/fixtures/bundle-poe1");
    let mut paths = Vec::new();
    files(&root, &mut paths);
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        src.insert(rel, std::fs::read(&p).unwrap());
    }
    Bundle::load(&src).unwrap()
}

#[test]
fn bases_should_join_their_property_tables() {
    let b = bundle();
    assert_eq!(
        b.item_base("Plain Test Shield"),
        Some(ItemBase::Armour(ArmourBase {
            armour: Range { min: 9, max: 12 },
            block: Some(24),
            ..ArmourBase::default()
        }))
    );
    assert_eq!(b.item_base("Plain Test Wand"), Some(ItemBase::Weapon));
    assert_eq!(b.item_base("Plain Test Ring"), Some(ItemBase::Other));
    assert_eq!(b.item_base("No Such Base"), None);
}

#[test]
fn rows_of_one_name_should_agree_or_be_ambiguous() {
    let mut b = bundle();
    let row = |id: &str| BaseItem {
        id: id.to_owned(),
        name: "Plain Test Shield".to_owned(),
        domain: 1,
        item_class: None,
        tags: None,
        implicits: Vec::new(),
    };
    let props = b.properties.as_mut().unwrap();
    let mut same: ArmourType = props.armour_types[0].clone();
    same.base_item = "Test/Items/Shield2".to_owned();
    props.armour_types.push(same);
    props.shield_types.push(ShieldType {
        base_item: "Test/Items/Shield2".to_owned(),
        block: 24,
    });
    b.base_items.push(row("Test/Items/Shield2"));
    assert!(
        matches!(b.item_base("Plain Test Shield"), Some(ItemBase::Armour(_))),
        "equal properties under one name are one base"
    );

    let props = b.properties.as_mut().unwrap();
    let mut other: ArmourType = props.armour_types[0].clone();
    other.base_item = "Test/Items/Shield3".to_owned();
    other.armour = ValueRange { min: 1, max: 2 };
    props.armour_types.push(other);
    b.base_items.push(row("Test/Items/Shield3"));
    assert_eq!(b.item_base("Plain Test Shield"), Some(ItemBase::Ambiguous));
}

#[test]
fn a_bundle_without_property_tables_should_know_no_base() {
    let mut b = bundle();
    b.properties = None;
    assert_eq!(b.item_base("Plain Test Shield"), None);
    assert_eq!(b.item_base("Plain Test Ring"), None);
}

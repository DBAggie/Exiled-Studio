use std::path::Path;

use loom_data::bundle::Bundle;
use loom_parser::import::mods::{ModRow, Mods};
use loom_parser::import::{BaseRow, BundleData};
use loom_parser::resolve::compose::Composition;

use super::compare;
use crate::resolve_corpus::Bases;

/// The synthetic PoE1 bundle of `loom-data` (made-up rows).
fn fixture() -> Bundle {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/loom-data/tests/fixtures/bundle-poe1");
    crate::load_bundle(&dir).unwrap()
}

/// The bundle's mod rows, each passed through `edit`, as an extract
/// reader would give them.
fn mods_of(data: &BundleData<'_>, edit: impl Fn(usize, &mut ModRow)) -> Mods {
    let rows = (0..data.mods().row_count())
        .filter_map(|n| {
            let mut r = data.mods().row(n)?.clone();
            r.type_spawn = None;
            edit(n, &mut r);
            Some(r)
        })
        .collect();
    let mut mods = Mods::from_rows(rows);
    mods.locality = data.mods().locality.clone();
    mods
}

#[test]
fn bundle_data_should_equal_itself() {
    let b = fixture();
    let data = BundleData::new(&b);
    let report = compare(
        &Bases::from_bundle(&data),
        &mods_of(&data, |_, _| {}),
        data.compositions(),
        &data,
    );
    assert_eq!(
        (report.equal(), report.mod_rows, report.base_names),
        (true, 3, 3)
    );
}

#[test]
fn craft_class_order_and_repeats_and_type_numbers_should_not_count() {
    let b = fixture();
    let data = BundleData::new(&b);
    let mods = mods_of(&data, |n, r| {
        r.craft_classes = vec![1, 0, 1];
        r.mod_type = Some(100 + i64::try_from(n).unwrap());
    });
    let mut bundle_side = b.clone();
    for (n, m) in bundle_side.mods.iter_mut().enumerate() {
        m.craft_classes = vec!["TestCharm".to_owned(), "TestRing".to_owned()];
        m.mod_type = Some(format!("T{n}"));
    }
    let data = BundleData::new(&bundle_side);
    let report = compare(
        &Bases::from_bundle(&data),
        &mods,
        data.compositions(),
        &data,
    );
    assert!(report.equal(), "{:?}", report.listed);
}

#[test]
fn a_merged_mod_type_should_be_a_mismatch() {
    let b = fixture();
    let mut split = b.clone();
    for (n, m) in split.mods.iter_mut().enumerate() {
        m.mod_type = Some(format!("T{n}"));
    }
    let data = BundleData::new(&split);
    // The extract side puts every row in one type.
    let mods = mods_of(&data, |_, r| r.mod_type = Some(7));
    let report = compare(
        &Bases::from_bundle(&data),
        &mods,
        data.compositions(),
        &data,
    );
    assert_eq!(
        (report.mismatches, report.listed),
        (
            2,
            vec![
                "mods[1] TestArmour1: mod type grouping differs".to_owned(),
                "mods[2] TestAccuracy1: mod type grouping differs".to_owned(),
            ]
        )
    );
}

#[test]
fn a_changed_row_or_base_should_be_a_mismatch() {
    let b = fixture();
    let data = BundleData::new(&b);
    let mods = mods_of(&data, |n, r| {
        if n == 0 {
            r.domain = 9;
        }
    });
    let bases = Bases::from_rows(vec![("Plain Test Ring", BaseRow::default())]);
    let report = compare(&bases, &mods, data.compositions(), &data);
    assert_eq!(
        report.listed,
        [
            "mods[0] TestLife1: fields differ",
            "bases: 1 names in the extract, 3 in the bundle, or another order",
            "bases[Plain Test Ring]: rows differ",
            "granted skills differ",
            "properties differ",
        ]
    );
}

#[test]
fn a_skipped_reference_or_other_compositions_should_be_a_mismatch() {
    let mut b = fixture();
    b.base_items[0].implicits.push("Absent".to_owned());
    let data = BundleData::new(&b);
    let wrap = Composition::new("W", "Wrapped: {0}", "test_maximum_life", 1).unwrap();
    let report = compare(
        &Bases::from_bundle(&data),
        &mods_of(&data, |_, _| {}),
        &[wrap],
        &data,
    );
    assert_eq!(
        report.listed,
        ["bundle: 1 references skipped", "compositions differ"]
    );
}

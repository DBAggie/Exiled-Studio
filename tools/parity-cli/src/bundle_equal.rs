//! `bundle-equal` (Phase 2 plan, step 6; P2-6a): checks that the item
//! pipeline's data built from a data bundle (`loom_parser::import::
//! BundleData`) equal the data the corpus harness reads from the extract
//! the bundle was made from (`resolve_corpus::Bases::load`,
//! `mod_filter::load`).
//!
//! Compared:
//! - every mod row, field by field, in row order. Two fields are compared
//!   by meaning, not by number: crafting classes as a set (the bundle keeps
//!   each class once, sorted), and mod types as a partition (the extract
//!   numbers a type by its `ModType` row, the bundle names it, and only
//!   equality of types is read);
//! - the local stats;
//! - every base name and its rows (domain, class, tags, implicits);
//! - the count of rows with unknown tags, the essence rows (with known and
//!   unknown classes), the granted skills, the property texts and quality
//!   types, and the composition wrappers (the extract side from
//!   `categorize::GameText`);
//! - that no bundle reference was skipped
//!   (`BundleData::skipped_references`).
//!
//! The report names mismatches by row ID or base name only (game data, no
//! build content).

use std::collections::BTreeMap;

use loom_parser::import::mods::{ModRow, Mods};
use loom_parser::import::{BundleData, ItemData};
use loom_parser::resolve::compose::Composition;
use serde::Serialize;

use crate::resolve_corpus::Bases;

/// Mismatches listed in full; the rest are counted.
const LISTED: usize = 50;

/// What `bundle-equal` found.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// Mod rows compared (the larger count of the two sides).
    pub mod_rows: usize,
    /// Base names compared (the larger count of the two sides).
    pub base_names: usize,
    /// Every mismatch.
    pub mismatches: usize,
    /// The first [`LISTED`] mismatches.
    pub listed: Vec<String>,
}

impl Report {
    /// Whether the two sides are equal.
    #[must_use]
    pub fn equal(&self) -> bool {
        self.mismatches == 0
    }

    fn add(&mut self, what: String) {
        self.mismatches += 1;
        if self.listed.len() < LISTED {
            self.listed.push(what);
        }
    }
}

/// The row with its crafting classes sorted and deduplicated, and its mod
/// type cleared (compared as a partition).
fn comparable(row: &ModRow) -> ModRow {
    let mut r = row.clone();
    r.craft_classes.sort_unstable();
    r.craft_classes.dedup();
    r.mod_type = None;
    r
}

/// Compares the bundle's data with the extract's (module docs).
#[must_use]
pub fn compare(
    bases: &Bases,
    mods: &Mods,
    compositions: &[Composition],
    data: &BundleData<'_>,
) -> Report {
    let mut report = Report::default();
    if data.skipped_references() > 0 {
        report.add(format!(
            "bundle: {} references skipped",
            data.skipped_references()
        ));
    }
    let bundle_mods = data.mods();
    report.mod_rows = mods.row_count().max(bundle_mods.row_count());
    if mods.row_count() != bundle_mods.row_count() {
        report.add(format!(
            "mods: {} rows in the extract, {} in the bundle",
            mods.row_count(),
            bundle_mods.row_count()
        ));
    }
    // Mod types: extract type to bundle type must be one to one.
    let mut forward: BTreeMap<Option<i64>, Option<i64>> = BTreeMap::new();
    let mut backward: BTreeMap<Option<i64>, Option<i64>> = BTreeMap::new();
    for n in 0..report.mod_rows {
        let (Some(a), Some(b)) = (mods.row(n), bundle_mods.row(n)) else {
            continue;
        };
        let (ca, cb) = (comparable(a), comparable(b));
        if ca != cb {
            report.add(format!("mods[{n}] {}: fields differ", a.id));
        }
        let f = *forward.entry(a.mod_type).or_insert(b.mod_type);
        let r = *backward.entry(b.mod_type).or_insert(a.mod_type);
        if f != b.mod_type || r != a.mod_type {
            report.add(format!("mods[{n}] {}: mod type grouping differs", a.id));
        }
    }
    if mods.essence_rows() != bundle_mods.essence_rows() {
        report.add("mods: essence rows differ".to_owned());
    }
    if compositions != data.compositions() {
        report.add("compositions differ".to_owned());
    }
    if mods.locality != bundle_mods.locality {
        report.add("stats: local stats differ".to_owned());
    }
    let names_a: Vec<&str> = bases.base_names().collect();
    let names_b: Vec<&str> = data.base_names().collect();
    report.base_names = names_a.len().max(names_b.len());
    if names_a != names_b {
        report.add(format!(
            "bases: {} names in the extract, {} in the bundle, or another order",
            names_a.len(),
            names_b.len()
        ));
    }
    for name in &names_a {
        if bases.base(name).map(|(_, r)| r) != data.base(name).map(|(_, r)| r) {
            report.add(format!("bases[{name}]: rows differ"));
        }
    }
    if bases.tags_unknown != data.tags_unknown() {
        report.add(format!(
            "bases: {} rows with unknown tags in the extract, {} in the bundle",
            bases.tags_unknown,
            data.tags_unknown()
        ));
    }
    if bases.grafts != *data.grafts() {
        report.add("granted skills differ".to_owned());
    }
    if bases.properties != *data.properties() {
        report.add("properties differ".to_owned());
    }
    report
}

#[cfg(test)]
mod tests;

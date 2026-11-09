//! A [`Lookup`] over the extract's tables (Spike B, PR B2b).
//!
//! Key forms, with their evidence (Spike B context file, "PR B2b"):
//! - `display_indexable_support`, `display_indexable_non_active_support`,
//!   `display_indexable_skill`: the table's `Index` column. Mods rows give
//!   these stats the ranges `1-172`, `1-164`, and `1-287`, which are the
//!   tables' `Index` spans and row counts (PoE1 26530219), so a 0-based
//!   row index is refuted. `Index` equals row + 1 in every row of the
//!   three tables, so the data cannot tell `Index` from row + 1.
//! - `display_indexable_skill` writes `Name1` (hypothesis: the table also
//!   has `Name2`, which differs in some rows; no evidence picks one).
//! - `passive_hash`: `PassiveSkills.PassiveSkillGraphId` (the passive
//!   tree's node hash, Spike A section 16) to `Name`. Hypothesis from the
//!   handler name; PoB's export prints the raw value as an unsigned 16-bit
//!   number (`v2.67.2:src/Export/statdesc.lua` `describeStats` adds 65536
//!   to a negative value), which fits a 16-bit hash. Negative stat values
//!   are not wrapped here; no Mods row has a fixed value to test it.
//!
//! Not provided, so their variants are counted as `lookup_unavailable`:
//! `tree_expansion_jewel_passive` (no Mods row has a non-zero value and no
//! table names its key; the corpus's cluster-jewel lines are ordinary
//! descriptions), `mod_value_to_item_class` (Mods values do not follow
//! `ItemClasses` row order: amulets 0 is row 4, belts 15 is row 21), and
//! every handler whose table is not extracted (Spike B plan, lookup table).

use std::collections::BTreeMap;

use loom_data::stat_desc::{Lookup, LookupHandler};
use serde::Serialize;

use crate::extract::{Read, Table};

/// Names by key, per handler.
#[derive(Debug, Default)]
pub struct TableLookup {
    names: BTreeMap<LookupHandler, BTreeMap<i64, String>>,
}

/// What each handler's table gave, for reports.
#[derive(Debug, Default, Serialize)]
pub struct LookupReport {
    /// Handler name to the number of keys with a name (0 for an extracted
    /// table without rows).
    pub keys: BTreeMap<&'static str, usize>,
    /// Handler name to rows whose key an earlier row already has (the
    /// first row's name is kept), when non-zero.
    pub duplicate_keys: BTreeMap<&'static str, usize>,
    /// Handler name to rows without an integer key or a name, when
    /// non-zero.
    pub skipped_rows: BTreeMap<&'static str, usize>,
    /// Handler name to why it has no names (table not in the extract, or
    /// not supported, with the reason).
    pub unavailable: BTreeMap<&'static str, String>,
}

/// (handler, table, key column, name column).
const SOURCES: [(LookupHandler, &str, &str, &str); 4] = [
    (
        LookupHandler::DisplayIndexableSupport,
        "IndexableSupportGems",
        "Index",
        "Name",
    ),
    (
        LookupHandler::DisplayIndexableNonActiveSupport,
        "IndexableNonActiveSupportGems",
        "Index",
        "Name",
    ),
    (
        LookupHandler::DisplayIndexableSkill,
        "IndexableSkillGems",
        "Index",
        "Name1",
    ),
    (
        LookupHandler::PassiveHash,
        "PassiveSkills",
        "PassiveSkillGraphId",
        "Name",
    ),
];

impl TableLookup {
    /// Builds the lookup from the extract's tables.
    ///
    /// # Errors
    /// A message when a table exists but lacks a column or does not parse.
    pub fn load(read: Read<'_>) -> Result<(Self, LookupReport), String> {
        let mut lookup = Self::default();
        let mut report = LookupReport::default();
        for (handler, table, key, name) in SOURCES {
            match Table::read(read, table)? {
                Some(t) => {
                    let mapped = t.map(key, name)?;
                    report.keys.insert(handler.name(), mapped.texts.len());
                    if mapped.duplicate_keys > 0 {
                        report
                            .duplicate_keys
                            .insert(handler.name(), mapped.duplicate_keys);
                    }
                    if mapped.skipped_rows > 0 {
                        report
                            .skipped_rows
                            .insert(handler.name(), mapped.skipped_rows);
                    }
                    lookup.names.insert(handler, mapped.texts);
                }
                None => {
                    report
                        .unavailable
                        .insert(handler.name(), format!("{table} is not in the extract"));
                }
            }
        }
        for handler in LookupHandler::ALL {
            if !SOURCES.iter().any(|(h, ..)| *h == handler) {
                report.unavailable.insert(
                    handler.name(),
                    "no supported key form (see parity-cli lookup.rs)".to_owned(),
                );
            }
        }
        Ok((lookup, report))
    }
}

impl Lookup for TableLookup {
    fn name(&self, handler: LookupHandler, value: i64) -> Option<String> {
        self.names.get(&handler)?.get(&value).cloned()
    }

    fn entries(&self, handler: LookupHandler) -> Vec<(i64, String)> {
        self.names
            .get(&handler)
            .map(|m| m.iter().map(|(k, n)| (*k, n.clone())).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::tests::table_json;

    #[test]
    fn load_should_read_present_tables_and_report_missing_ones() {
        let support = table_json("IndexableSupportGems", &[(1, "Fork"), (2, "Pierce")]);
        let read = |p: &str| {
            (p == "tables/IndexableSupportGems.json")
                .then(|| support.clone().into_bytes())
                .ok_or_else(|| "missing".to_owned())
        };
        let (lookup, report) = TableLookup::load(&read).unwrap();
        let h = LookupHandler::DisplayIndexableSupport;
        assert_eq!(lookup.name(h, 2).as_deref(), Some("Pierce"));
        assert_eq!(lookup.name(h, 3), None);
        assert_eq!(lookup.entries(h).len(), 2);
        assert_eq!(report.keys.get("display_indexable_support"), Some(&2));
        assert!(report.unavailable.contains_key("passive_hash"));
        assert!(report.unavailable.contains_key("mod_value_to_item_class"));
        assert_eq!(
            report.keys.len() + report.unavailable.len(),
            LookupHandler::ALL.len()
        );
    }
}

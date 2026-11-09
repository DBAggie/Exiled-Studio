//! Readers for the extractor's own output (`extract`): `manifest.json`,
//! `tables/<Name>.json`, and `stat_descriptions/**/*.json`. Table and
//! manifest fields are read strictly, so a change to the output format fails
//! here instead of being misread.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

/// One column of an extracted table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    /// An array of values per row.
    pub array: bool,
    /// A `[min, max]` pair per row.
    pub interval: bool,
    /// Name, if the schema knows it.
    pub name: Option<String>,
    /// Target table or enumeration of a key or enum column.
    pub references: Option<String>,
    /// Schema type name.
    #[serde(rename = "type")]
    pub kind: String,
}

/// An extracted table (`tables/<Name>.json`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    /// Columns in row order.
    pub columns: Vec<Column>,
    /// Rows, each one value per column.
    pub rows: Vec<Vec<Value>>,
    /// Table name.
    pub table: String,
}

impl Table {
    /// Parses a table file.
    ///
    /// # Errors
    /// If the bytes are not a table file.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Position of the column named `name`.
    #[must_use]
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name.as_deref() == Some(name))
    }
}

/// A row's value as a string, if it is one.
#[must_use]
pub fn text(row: &[Value], col: usize) -> Option<&str> {
    row.get(col).and_then(Value::as_str)
}

/// A row's value as an unsigned integer, if it is one.
#[must_use]
pub fn uint(row: &[Value], col: usize) -> Option<u64> {
    row.get(col).and_then(Value::as_u64)
}

/// A variant condition on one stat's value (see `statdesc::Condition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Condition {
    /// `min <= value <= max`; `None` is open.
    Range {
        /// Upper bound.
        max: Option<i64>,
        /// Lower bound.
        min: Option<i64>,
    },
    /// The value is not `value`.
    Not {
        /// The excluded value.
        value: i64,
    },
}

impl Condition {
    /// Whether `value` meets the condition.
    #[must_use]
    pub fn holds(self, value: i64) -> bool {
        match self {
            Self::Range { min, max } => {
                min.is_none_or(|min| value >= min) && max.is_none_or(|max| value <= max)
            }
            Self::Not { value: not } => value != not,
        }
    }
}

/// One English variant (the fields the cross-check needs).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Variant {
    /// One condition per stat, in stat order.
    pub conditions: Vec<Condition>,
    /// Variant flags (`table_only`, `gem_quality`).
    pub flags: Vec<String>,
    /// Format string as written.
    pub text: String,
}

/// One description in a stat-description output file (the fields the
/// cross-check needs).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Description {
    /// Main-hand and off-hand stats are listed separately.
    pub handed: bool,
    /// Off-hand stat IDs of a handed description.
    pub off_hand_stats: Option<Vec<String>>,
    /// Stat IDs (the main-hand ones for a handed description).
    pub stats: Vec<String>,
    /// English variants.
    pub variants: Vec<Variant>,
}

impl Description {
    /// Every stat ID the description names (main hand and off hand).
    pub fn all_stats(&self) -> impl Iterator<Item = &str> {
        self.stats
            .iter()
            .chain(self.off_hand_stats.iter().flatten())
            .map(String::as_str)
    }
}

/// A stat-description output file (`stat_descriptions/<path>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StatDescFile {
    /// Descriptions in file order.
    pub descriptions: Vec<Description>,
    /// Files included, as written (`Metadata/StatDescriptions/x.txt`).
    pub includes: Vec<String>,
    /// Stats listed as deliberately not shown.
    pub no_description: Vec<String>,
}

impl StatDescFile {
    /// Parses a stat-description output file.
    ///
    /// # Errors
    /// If the bytes are not such a file.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// The extractor build recorded in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ManifestExtractor {
    /// Git commit, if known.
    pub commit: Option<String>,
    /// Whether the tree had changes, if known.
    pub dirty: Option<bool>,
}

/// One table entry in the manifest (the field the cross-check needs).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ManifestTable {
    /// `ok`, `row-size-mismatch`, `failed`, `no-file`, or `no-schema`.
    pub status: String,
}

/// The manifest fields the cross-check reads.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    /// The extractor build.
    pub extractor: ManifestExtractor,
    /// Manifest format.
    pub format: u32,
    /// `poe1` or `poe2`.
    pub game: String,
    /// Steam build ID, if found.
    pub steam_build_id: Option<String>,
    /// Every planned or referenced table.
    pub tables: BTreeMap<String, ManifestTable>,
}

impl Manifest {
    /// Parses `manifest.json`.
    ///
    /// # Errors
    /// If the bytes are not a manifest.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_should_parse_and_find_columns() {
        let table = Table::parse(
            br#"{"columns": [{"array":false,"interval":false,"name":"Id","references":null,"type":"string"},{"array":false,"interval":false,"name":null,"references":null,"type":"i32"}],
"rows": [["A", 1], ["B", null]],
"table": "T"}"#,
        )
        .unwrap();
        assert_eq!(table.column("Id"), Some(0));
        assert_eq!(table.column("Missing"), None);
        let row = table.rows.first().unwrap();
        assert_eq!(text(row, 0), Some("A"));
        assert_eq!(uint(row, 1), Some(1));
        assert_eq!(uint(row, 0), None);
    }

    #[test]
    fn table_should_reject_unknown_fields() {
        assert!(Table::parse(br#"{"columns": [], "rows": [], "table": "T", "extra": 1}"#).is_err());
    }

    #[test]
    fn stat_desc_file_should_list_main_and_off_hand_stats() {
        let file = StatDescFile::parse(
            br#"{"descriptions": [{"handed":true,"line":1,"name":null,"off_hand_stats":["b"],"stats":["a"],"variants":[{"conditions":[{"kind":"range","max":null,"min":1},{"kind":"not","value":0}],"flags":[],"handlers":[],"line":2,"placeholders":[],"text":"x"}]}],
"file_flags": [], "includes": ["Metadata/StatDescriptions/i.txt"], "languages": {}, "no_description": ["c"], "source": "s"}"#,
        )
        .unwrap();
        let desc = file.descriptions.first().unwrap();
        assert_eq!(desc.all_stats().collect::<Vec<_>>(), ["a", "b"]);
        let variant = desc.variants.first().unwrap();
        assert_eq!(variant.text, "x");
        assert_eq!(file.includes, ["Metadata/StatDescriptions/i.txt"]);
        assert_eq!(file.no_description, ["c"]);
        let [range, not] = variant.conditions.as_slice() else {
            panic!("two conditions")
        };
        assert!(range.holds(1) && range.holds(99) && !range.holds(0));
        assert!(not.holds(1) && !not.holds(0));
    }
}

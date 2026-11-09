//! Reading a `loom-extractor extract` output: the stat-description files
//! as one [`StatDescriptions`] set, and tables as JSON rows. Disk access
//! stays in `main.rs`; this module takes a `read` function.

use std::collections::BTreeMap;

use loom_data::stat_desc::{StatDescFile, StatDescManifest, StatDescriptions};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Reads the bytes of a path relative to the extract directory.
pub type Read<'r> = &'r dyn Fn(&str) -> Result<Vec<u8>, String>;

/// The extract's recorded inputs, for reports.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Inputs {
    /// `poe1` or `poe2`.
    pub game: String,
    /// Steam build of the extract.
    pub steam_build_id: Option<String>,
    /// SHA-256 of `manifest.json`, hex.
    pub manifest_sha256: String,
}

/// A loaded extract: inputs and every `ok` stat-description file.
pub struct Extract {
    /// Recorded inputs.
    pub inputs: Inputs,
    /// Manifest path of every loaded file (`ok`, kind `descriptions`), in
    /// manifest order.
    pub files: Vec<String>,
    /// The files.
    pub set: StatDescriptions,
}

/// Loads `manifest.json` and every `ok` stat-description file. Any file
/// that fails to load fails the run: the checks need the whole set.
///
/// # Errors
/// A message naming the file.
pub fn load(read: Read<'_>) -> Result<Extract, String> {
    let bytes = read("manifest.json")?;
    let manifest = StatDescManifest::from_json(&bytes)
        .map_err(|e| format!("manifest.json: {} ({e})", e.code()))?;
    let inputs = Inputs {
        game: manifest.game.clone(),
        steam_build_id: manifest.steam_build_id.clone(),
        manifest_sha256: hex(&Sha256::digest(&bytes)),
    };
    let mut set = StatDescriptions::new();
    let mut files = Vec::new();
    for (path, entry) in &manifest.files {
        if entry.status != "ok" || entry.kind != "descriptions" {
            continue;
        }
        let output = entry
            .output
            .as_deref()
            .ok_or_else(|| format!("{path}: ok but no output path"))?;
        let file = StatDescFile::from_json(&read(output)?)
            .map_err(|e| format!("{output}: {} ({e})", e.code()))?;
        if set.insert(file).is_some() {
            return Err(format!("{path}: loaded twice"));
        }
        files.push(path.clone());
    }
    Ok(Extract { inputs, files, set })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One extracted table (`tables/<Name>.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct Table {
    /// Table name.
    pub table: String,
    /// Columns in row order.
    pub columns: Vec<Column>,
    /// Rows, one value per column.
    pub rows: Vec<Vec<Value>>,
}

/// A column of an extracted table.
#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    /// Name; `None` for columns the schema does not name.
    pub name: Option<String>,
    /// Referenced table, for key columns.
    pub references: Option<String>,
}

impl Table {
    /// Reads `tables/<name>.json`; `Ok(None)` when the extract has no such
    /// file (a table the game or the extract does not have).
    ///
    /// # Errors
    /// A message when the file exists but does not parse.
    pub fn read(read: Read<'_>, name: &str) -> Result<Option<Self>, String> {
        let path = format!("tables/{name}.json");
        let Ok(bytes) = read(&path) else {
            return Ok(None);
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("{path}: {e}"))
    }

    /// Reads a table the check cannot run without.
    ///
    /// # Errors
    /// A message when the table is missing or does not parse.
    pub fn require(read: Read<'_>, name: &str) -> Result<Self, String> {
        Self::read(read, name)?.ok_or_else(|| format!("tables/{name}.json: not in the extract"))
    }

    /// Index of the column named `name`.
    ///
    /// # Errors
    /// A message naming the table and column.
    pub fn column(&self, name: &str) -> Result<usize, String> {
        self.columns
            .iter()
            .position(|c| c.name.as_deref() == Some(name))
            .ok_or_else(|| format!("{}: no column {name}", self.table))
    }

    /// Integer cell.
    #[must_use]
    pub fn int(row: &[Value], col: usize) -> Option<i64> {
        row.get(col)?.as_i64()
    }

    /// String cell.
    #[must_use]
    pub fn text(row: &[Value], col: usize) -> Option<&str> {
        row.get(col)?.as_str()
    }

    /// `key column -> text column`. The first row with a key wins; later
    /// rows with the same key and rows without an integer key or a text
    /// are counted, not dropped silently.
    ///
    /// # Errors
    /// A message when a column is missing.
    pub fn map(&self, key: &str, text: &str) -> Result<KeyedTexts, String> {
        let (k, t) = (self.column(key)?, self.column(text)?);
        let mut out = KeyedTexts::default();
        for row in &self.rows {
            let (Some(key), Some(text)) = (Self::int(row, k), Self::text(row, t)) else {
                out.skipped_rows += 1;
                continue;
            };
            match out.texts.entry(key) {
                std::collections::btree_map::Entry::Occupied(_) => out.duplicate_keys += 1,
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(text.to_owned());
                }
            }
        }
        Ok(out)
    }
}

/// Output of [`Table::map`].
#[derive(Debug, Default)]
pub struct KeyedTexts {
    /// Key to text.
    pub texts: BTreeMap<i64, String>,
    /// Rows whose key was already taken (the first row's text is kept).
    pub duplicate_keys: usize,
    /// Rows without an integer key or a text.
    pub skipped_rows: usize,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A table JSON with integer key column `Index` and text column `Name`.
    pub fn table_json(name: &str, rows: &[(i64, &str)]) -> String {
        let rows: Vec<String> = rows
            .iter()
            .map(|(k, n)| format!("[{k}, {}]", serde_json::to_string(n).unwrap()))
            .collect();
        format!(
            r#"{{"table": "{name}", "columns": [{{"name": "Index", "references": null, "type": "i32", "array": false, "interval": false}}, {{"name": "Name", "references": null}}], "rows": [{}]}}"#,
            rows.join(", ")
        )
    }

    #[test]
    fn table_map_should_count_duplicate_keys_and_rows_it_cannot_read() {
        let json = r#"{"table": "T", "columns": [{"name": "Index", "references": null},
            {"name": "Name", "references": null}],
            "rows": [[1, "A"], [1, "B"], [null, "C"], [2, null], [3, "D"]]}"#;
        let t: Table = serde_json::from_str(json).unwrap();
        let m = t.map("Index", "Name").unwrap();
        assert_eq!(
            m.texts,
            BTreeMap::from([(1, "A".to_owned()), (3, "D".to_owned())])
        );
        assert_eq!((m.duplicate_keys, m.skipped_rows), (1, 2));
    }

    #[test]
    fn table_should_map_a_key_column_to_a_text_column() {
        let json = table_json("T", &[(1, "A"), (2, "B")]);
        let read = |p: &str| {
            (p == "tables/T.json")
                .then(|| json.clone().into_bytes())
                .ok_or_else(|| "missing".to_owned())
        };
        let t = Table::require(&read, "T").unwrap();
        assert_eq!(
            t.map("Index", "Name").unwrap().texts,
            BTreeMap::from([(1, "A".to_owned()), (2, "B".to_owned())])
        );
        assert!(t.column("Nope").is_err());
        assert!(Table::read(&read, "U").unwrap().is_none());
    }
}

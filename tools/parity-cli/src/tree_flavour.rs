//! The passive tree's flavour texts for the PoE1 bundle (B-2 item
//! evidence, E10): `PassiveSkills.FlavourText` and `Ascendancy.FlavourText`
//! (with `FlavourTextColour`) to `poe1/tree_flavour.json`
//! (`loom_data::bundle::poe1::TreeFlavour`).
//!
//! The game files are the source (ADR-005's main rule), not GGG's tree
//! export. Rows with no text are left out; the table's `\r\n` line breaks
//! become `\n`. An extract whose tables have no flavour column gives no
//! file (`None`), as an extract without the tables.

use loom_data::bundle::poe1::{AscendancyFlavour, PassiveFlavour, TreeFlavour};

use crate::extract::{Read, Table};

/// A text cell, trimmed, with `\n` line breaks; `None` when empty.
fn text(row: &[serde_json::Value], column: usize) -> Option<String> {
    let t = Table::text(row, column)?.replace("\r\n", "\n");
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// The texts; `None` when neither table has a flavour column, or no row
/// has a text.
///
/// # Errors
/// A table that does not read.
pub fn rows(read: Read<'_>) -> Result<Option<TreeFlavour>, String> {
    let mut out = TreeFlavour::default();
    if let Some(t) = Table::read(read, "PassiveSkills")?
        && let (Ok(node), Ok(flavour)) = (t.column("PassiveSkillGraphId"), t.column("FlavourText"))
    {
        for r in &t.rows {
            if let (Some(node), Some(text)) = (Table::int(r, node), text(r, flavour)) {
                out.passives.push(PassiveFlavour { node, text });
            }
        }
    }
    if let Some(t) = Table::read(read, "Ascendancy")?
        && let (Ok(id), Ok(flavour)) = (t.column("Id"), t.column("FlavourText"))
    {
        let colour = t.column("FlavourTextColour").ok();
        for r in &t.rows {
            if let (Some(id), Some(text)) = (Table::text(r, id), text(r, flavour)) {
                out.ascendancies.push(AscendancyFlavour {
                    id: id.to_owned(),
                    text,
                    colour: colour.and_then(|c| Table::text(r, c)).map(str::to_owned),
                });
            }
        }
    }
    Ok((!out.passives.is_empty() || !out.ascendancies.is_empty()).then_some(out))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::*;

    fn table(name: &str, columns: &[&str], rows: &Value) -> (String, Vec<u8>) {
        let columns: Vec<Value> = columns
            .iter()
            .map(|c| json!({"name": c, "references": null}))
            .collect();
        let body = json!({"table": name, "columns": columns, "rows": rows});
        (format!("tables/{name}.json"), body.to_string().into_bytes())
    }

    fn run(files: &BTreeMap<String, Vec<u8>>) -> Result<Option<TreeFlavour>, String> {
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        rows(&read)
    }

    #[test]
    fn rows_should_keep_each_text_with_its_node_or_ascendancy() {
        let files: BTreeMap<String, Vec<u8>> = [
            table(
                "PassiveSkills",
                &["Id", "PassiveSkillGraphId", "FlavourText"],
                &json!([
                    ["a", 1, "A line.\r\nAnother. "],
                    ["b", 2, ""],
                    ["c", 3, null]
                ]),
            ),
            table(
                "Ascendancy",
                &["Id", "FlavourText", "FlavourTextColour"],
                &json!([["Asc1", "A quote.", "255,200,100"], ["Asc2", "", null]]),
            ),
        ]
        .into_iter()
        .collect();
        let f = run(&files).unwrap().unwrap();
        assert_eq!(
            f.passives,
            [PassiveFlavour {
                node: 1,
                text: "A line.\nAnother.".to_owned()
            }]
        );
        assert_eq!(
            f.ascendancies,
            [AscendancyFlavour {
                id: "Asc1".to_owned(),
                text: "A quote.".to_owned(),
                colour: Some("255,200,100".to_owned())
            }]
        );
    }

    #[test]
    fn rows_should_be_none_without_the_tables_or_their_columns() {
        assert_eq!(run(&BTreeMap::new()).unwrap(), None);
        let files: BTreeMap<String, Vec<u8>> = [table(
            "PassiveSkills",
            &["Id", "PassiveSkillGraphId"],
            &json!([["a", 1]]),
        )]
        .into_iter()
        .collect();
        assert_eq!(run(&files).unwrap(), None);
    }
}

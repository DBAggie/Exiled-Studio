//! The jewel radii for the PoE1 bundle (passive tree plan, T-f):
//! `PassiveJewelRadii` to `poe1/jewel_radii.json`
//! (`loom_data::bundle::poe1::JewelRadius`).
//!
//! Each row in the table's order: its ID, its name (the `ClientStrings`
//! row's text, what an item's `Radius:` line shows), its radius, and its
//! ring (the inner and outer radius a ring jewel such as Thread of Hope
//! uses). A row with a name the extract does not have keeps an empty name;
//! a radius or ring that is not a number fails the conversion: the table
//! would be misread.

use loom_data::bundle::poe1::JewelRadius;

use crate::extract::{Read, Table};

/// The first of `names` the table has.
fn column(t: &Table, names: &[&str]) -> Result<usize, String> {
    names
        .iter()
        .find_map(|n| t.column(n).ok())
        .ok_or_else(|| format!("{}: no column {}", t.table, names.join(" or ")))
}

/// The radii, in the table's order; `None` when the extract has no
/// `PassiveJewelRadii`.
///
/// # Errors
/// A column missing, or a radius or ring that is not an integer.
pub fn rows(read: Read<'_>) -> Result<Option<Vec<JewelRadius>>, String> {
    let Some(t) = Table::read(read, "PassiveJewelRadii")? else {
        return Ok(None);
    };
    let id = column(&t, &["ID", "Id"])?;
    let outer = column(&t, &["RingOuterRadius", "RingOuter"])?;
    let inner = column(&t, &["RingInnerRadius", "RingInner"])?;
    let radius = column(&t, &["Radius"])?;
    let name = column(&t, &["ClientString", "Name"]).ok();
    let texts: Vec<Option<String>> = match Table::read(read, "ClientStrings")? {
        Some(cs) => {
            let text = cs.column("Text")?;
            cs.rows
                .iter()
                .map(|r| Table::text(r, text).map(str::to_owned))
                .collect()
        }
        None => Vec::new(),
    };
    let mut out = Vec::new();
    for r in &t.rows {
        let id = Table::text(r, id)
            .filter(|s| !s.is_empty())
            .ok_or("PassiveJewelRadii: a row has no ID")?
            .to_owned();
        let int = |col: usize, what: &str| {
            Table::int(r, col)
                .ok_or_else(|| format!("PassiveJewelRadii {id}: {what} is not a number"))
        };
        out.push(JewelRadius {
            name: name
                .and_then(|c| Table::int(r, c))
                .and_then(|n| usize::try_from(n).ok())
                .and_then(|n| texts.get(n).cloned().flatten())
                .unwrap_or_default(),
            radius: int(radius, "Radius")?,
            ring_inner: int(inner, "RingInnerRadius")?,
            ring_outer: int(outer, "RingOuterRadius")?,
            id,
        });
    }
    Ok(Some(out))
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

    fn run(files: &BTreeMap<String, Vec<u8>>) -> Result<Option<Vec<JewelRadius>>, String> {
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        rows(&read)
    }

    const COLUMNS: [&str; 5] = [
        "ID",
        "RingOuterRadius",
        "RingInnerRadius",
        "Radius",
        "ClientString",
    ];

    #[test]
    fn rows_should_read_each_radius_with_its_name_and_ring() {
        let files: BTreeMap<String, Vec<u8>> = [
            table(
                "ClientStrings",
                &["Id", "Text"],
                &json!([["A", "Small"], ["B", "Medium"]]),
            ),
            table(
                "PassiveJewelRadii",
                &COLUMNS,
                &json!([
                    ["Small", 1100, 800, 800, 0],
                    ["Medium", 1400, 1100, 1200, 1],
                    ["X", 0, 0, 5, null]
                ]),
            ),
        ]
        .into_iter()
        .collect();
        let r = run(&files).unwrap().unwrap();
        assert_eq!(
            r[0],
            JewelRadius {
                id: "Small".to_owned(),
                name: "Small".to_owned(),
                radius: 800,
                ring_inner: 800,
                ring_outer: 1100,
            }
        );
        assert_eq!(r[1].name, "Medium");
        assert_eq!(r[2].name, "");
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn rows_should_be_none_without_the_table_and_fail_on_a_bad_radius() {
        assert_eq!(run(&BTreeMap::new()).unwrap(), None);
        let files: BTreeMap<String, Vec<u8>> = [table(
            "PassiveJewelRadii",
            &COLUMNS,
            &json!([["Small", 1, 0, "x", null]]),
        )]
        .into_iter()
        .collect();
        assert!(run(&files).unwrap_err().contains("Radius"));
    }
}

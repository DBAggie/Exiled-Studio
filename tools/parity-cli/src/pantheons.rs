//! The pantheon's gods for the PoE1 bundle (P3-12g): `PantheonPanelLayout`
//! to `poe1/pantheons.json` (`loom_data::bundle::poe1::Pantheon`).
//!
//! PoB v2.67.2 exports its `Data/Pantheons.lua` from the same table
//! (`src/Export/Scripts/pantheons.lua`), and the conversion reads it the
//! same way:
//! - rows with `IsDisabled` set are left out (`GetRowList("IsDisabled",
//!   false)`), and counted;
//! - each god has up to four souls, `GodName<n>` with `Effect<n>_StatsKeys`
//!   and `Effect<n>_Values` paired in order (`zip`); a soul without stats
//!   is left out (`if next(god.statKeys)`);
//! - a stat key is a `Stats` row, read as its `Id`.
//!
//! A soul whose key and value lists differ in length, or whose key is no
//! `Stats` row, fails the conversion: the table would be misread.

use loom_data::bundle::poe1::{Pantheon, PantheonSoul};
use serde::Serialize;
use serde_json::Value;

use crate::extract::{Read, Table};

/// The conversion's counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PantheonSummary {
    /// Rows left out as disabled.
    pub disabled: usize,
    /// Souls left out for having no stats.
    pub souls_without_stats: usize,
}

/// The gods of `PantheonPanelLayout`, in table order; `None` without the
/// table (a PoE2 extract, or one from before P3-12g).
///
/// # Errors
/// A message when a column is missing, a row has no ID, or a soul cannot
/// be read (module docs).
pub fn rows(read: Read<'_>) -> Result<Option<(Vec<Pantheon>, PantheonSummary)>, String> {
    let Some(t) = Table::read(read, "PantheonPanelLayout")? else {
        return Ok(None);
    };
    let stats = Table::require(read, "Stats")?;
    let stat_id = stats.column("Id")?;
    let (id, major, disabled) = (
        t.column("Id")?,
        t.column("IsMajorGod")?,
        t.column("IsDisabled")?,
    );
    let mut souls = Vec::new();
    for n in 1..=4 {
        souls.push((
            t.column(&format!("GodName{n}"))?,
            t.column(&format!("Effect{n}_StatsKeys"))?,
            t.column(&format!("Effect{n}_Values"))?,
        ));
    }
    let mut summary = PantheonSummary::default();
    let mut gods = Vec::new();
    for r in &t.rows {
        if r.get(disabled).and_then(Value::as_bool) == Some(true) {
            summary.disabled += 1;
            continue;
        }
        let god = Table::text(r, id)
            .filter(|s| !s.is_empty())
            .ok_or("PantheonPanelLayout: a row has no ID")?;
        let mut list = Vec::new();
        for &(name, keys, values) in &souls {
            let keys = ints(r, keys).ok_or_else(|| format!("{god}: stat keys are not a list"))?;
            let values = ints(r, values).ok_or_else(|| format!("{god}: values are not a list"))?;
            if keys.is_empty() {
                summary.souls_without_stats += 1;
                continue;
            }
            if keys.len() != values.len() {
                return Err(format!(
                    "{god}: {} stat keys for {} values",
                    keys.len(),
                    values.len()
                ));
            }
            let mut pairs = Vec::new();
            for (k, v) in keys.into_iter().zip(values) {
                let s = usize::try_from(k)
                    .ok()
                    .and_then(|k| stats.rows.get(k))
                    .and_then(|row| Table::text(row, stat_id))
                    .ok_or_else(|| format!("{god}: stat key {k} is no stat"))?;
                pairs.push((s.to_owned(), v));
            }
            list.push(PantheonSoul {
                name: Table::text(r, name).unwrap_or_default().to_owned(),
                stats: pairs,
            });
        }
        gods.push(Pantheon {
            id: god.to_owned(),
            major: r.get(major).and_then(Value::as_bool).unwrap_or(false),
            souls: list,
        });
    }
    Ok(Some((gods, summary)))
}

/// A list of integers (a null cell is an empty list).
fn ints(row: &[Value], col: usize) -> Option<Vec<i64>> {
    match row.get(col)? {
        Value::Null => Some(Vec::new()),
        Value::Array(a) => a.iter().map(Value::as_i64).collect(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;

    fn table(name: &str, columns: &[&str], rows: &Value) -> (String, Vec<u8>) {
        let columns: Vec<Value> = columns
            .iter()
            .map(|c| json!({"name": c, "references": null}))
            .collect();
        let body = json!({"table": name, "columns": columns, "rows": rows});
        (format!("tables/{name}.json"), body.to_string().into_bytes())
    }

    const COLUMNS: [&str; 15] = [
        "Id",
        "IsMajorGod",
        "IsDisabled",
        "GodName1",
        "Effect1_StatsKeys",
        "Effect1_Values",
        "GodName2",
        "Effect2_StatsKeys",
        "Effect2_Values",
        "GodName3",
        "Effect3_StatsKeys",
        "Effect3_Values",
        "GodName4",
        "Effect4_StatsKeys",
        "Effect4_Values",
    ];

    fn extract(rows: &Value) -> BTreeMap<String, Vec<u8>> {
        [
            table(
                "Stats",
                &["Id"],
                &json!([["stat_a"], ["stat_b"], ["stat_c"]]),
            ),
            table("PantheonPanelLayout", &COLUMNS, rows),
        ]
        .into_iter()
        .collect()
    }

    fn run(
        files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Option<(Vec<Pantheon>, PantheonSummary)>, String> {
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        rows(&read)
    }

    #[test]
    fn rows_should_pair_each_souls_stats_and_drop_disabled_gods_and_empty_souls() {
        let files = extract(&json!([
            [
                "GodA",
                true,
                false,
                "Soul of A",
                [0, 1],
                [6, 20],
                "Upgrade",
                [2],
                [-8],
                "",
                [],
                [],
                "Last",
                [0],
                [1]
            ],
            [
                "GodB",
                false,
                true,
                "Soul of B",
                [0],
                [1],
                "",
                [],
                [],
                "",
                [],
                [],
                "",
                [],
                []
            ],
            [
                "GodC",
                false,
                false,
                "Soul of C",
                [1],
                [10],
                "",
                null,
                null,
                "",
                [],
                [],
                "",
                [],
                []
            ],
        ]));
        let (gods, summary) = run(&files).unwrap().unwrap();
        assert_eq!(
            summary,
            PantheonSummary {
                disabled: 1,
                souls_without_stats: 4
            }
        );
        assert_eq!(
            gods,
            [
                Pantheon {
                    id: "GodA".to_owned(),
                    major: true,
                    souls: vec![
                        PantheonSoul {
                            name: "Soul of A".to_owned(),
                            stats: vec![("stat_a".to_owned(), 6), ("stat_b".to_owned(), 20)],
                        },
                        PantheonSoul {
                            name: "Upgrade".to_owned(),
                            stats: vec![("stat_c".to_owned(), -8)],
                        },
                        PantheonSoul {
                            name: "Last".to_owned(),
                            stats: vec![("stat_a".to_owned(), 1)],
                        },
                    ],
                },
                Pantheon {
                    id: "GodC".to_owned(),
                    major: false,
                    souls: vec![PantheonSoul {
                        name: "Soul of C".to_owned(),
                        stats: vec![("stat_b".to_owned(), 10)],
                    }],
                },
            ]
        );
    }

    #[test]
    fn rows_should_give_none_without_the_table() {
        let files: BTreeMap<String, Vec<u8>> = [table("Stats", &["Id"], &json!([["stat_a"]]))]
            .into_iter()
            .collect();
        assert_eq!(run(&files).unwrap(), None);
    }

    #[test]
    fn rows_should_fail_on_a_table_it_would_misread() {
        let row = |keys: Value, values: Value| {
            json!([[
                "GodA",
                true,
                false,
                "Soul",
                keys,
                values,
                "",
                [],
                [],
                "",
                [],
                [],
                "",
                [],
                []
            ]])
        };
        for (what, rows) in [
            ("lengths differ", row(json!([0, 1]), json!([5]))),
            ("key past the table", row(json!([9]), json!([5]))),
            ("keys not a list", row(json!(3), json!([5]))),
            (
                "no ID",
                json!([[
                    "",
                    true,
                    false,
                    "Soul",
                    [0],
                    [1],
                    "",
                    [],
                    [],
                    "",
                    [],
                    [],
                    "",
                    [],
                    []
                ]]),
            ),
        ] {
            assert!(run(&extract(&rows)).is_err(), "{what}");
        }
        // A missing column.
        let mut files = extract(&row(json!([0]), json!([1])));
        files.insert(
            "tables/PantheonPanelLayout.json".to_owned(),
            table("PantheonPanelLayout", &COLUMNS[..14], &json!([])).1,
        );
        assert!(run(&files).is_err(), "missing column");
    }
}

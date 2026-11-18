//! The flask bases for the PoE1 bundle (Phase 6, P6-4b): `Flasks` to
//! `poe1/flasks.json` (`loom_data::bundle::poe1::Flask`).
//!
//! PoB v2.67.2 exports each flask base's `flask` table from the same row
//! (`src/Export/Scripts/bases.lua` 247 to 270), and the conversion reads it
//! the same way:
//! - `LifePerUse`, `ManaPerUse`, and `RecoveryTime` (tenths of a second;
//!   PoB divides by 10);
//! - the buff (`BuffDefinitionsKey`): its `StatsKeys` paired in order with
//!   the row's first value list (`BuffStatValues`, PoB's `BuffMagnitudes`),
//!   then its `GrantedFlags` at 1;
//! - a stat key is a `Stats` row, read as its `Id`; the base is a
//!   `BaseItemTypes` row, read as its `Id` and `Name`.
//!
//! A buff whose key and value lists differ in length, or a key that is no
//! row, fails the conversion: the table would be misread.

use loom_data::bundle::poe1::Flask;
use serde_json::Value;

use crate::extract::{Read, Table};

/// The flask bases of `Flasks`, in table order; `None` without the table
/// (a PoE2 extract, or one from before P6-4b).
///
/// # Errors
/// A message when a column is missing or a row cannot be read (module
/// docs).
pub fn rows(read: Read<'_>) -> Result<Option<Vec<Flask>>, String> {
    let Some(t) = Table::read(read, "Flasks")? else {
        return Ok(None);
    };
    let stats = Table::require(read, "Stats")?;
    let stat_id = stats.column("Id")?;
    let buffs = Table::require(read, "BuffDefinitions")?;
    let (buff_id, buff_stats, buff_flags) = (
        buffs.column("Id")?,
        buffs.column("StatsKeys")?,
        buffs.column("GrantedFlags")?,
    );
    let bases = Table::require(read, "BaseItemTypes")?;
    let (base_id, base_name) = (bases.column("Id")?, bases.column("Name")?);
    let (base, life, mana, time, buff, values) = (
        t.column("BaseItemTypesKey")?,
        t.column("LifePerUse")?,
        t.column("ManaPerUse")?,
        t.column("RecoveryTime")?,
        t.column("BuffDefinitionsKey")?,
        t.column("BuffStatValues")?,
    );
    let stat = |k: i64, at: &str| {
        usize::try_from(k)
            .ok()
            .and_then(|k| stats.rows.get(k))
            .and_then(|row| Table::text(row, stat_id))
            .map(str::to_owned)
            .ok_or_else(|| format!("{at}: stat key {k} is no stat"))
    };
    let mut out = Vec::new();
    for (n, r) in t.rows.iter().enumerate() {
        let row = key(r, base)
            .and_then(|k| bases.rows.get(k))
            .ok_or_else(|| format!("Flasks row {n}: no base item"))?;
        let id = Table::text(row, base_id)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("Flasks row {n}: the base item has no ID"))?;
        let int = |c: usize| r.get(c).and_then(Value::as_i64).unwrap_or(0);
        let mut flask = Flask {
            base: id.to_owned(),
            name: Table::text(row, base_name).unwrap_or_default().to_owned(),
            life: int(life),
            mana: int(mana),
            duration: int(time),
            buff: None,
            stats: Vec::new(),
        };
        if let Some(b) = key(r, buff) {
            let brow = buffs
                .rows
                .get(b)
                .ok_or_else(|| format!("{id}: buff key {b} is no buff"))?;
            flask.buff = Table::text(brow, buff_id).map(str::to_owned);
            let keys = ints(brow, buff_stats).ok_or_else(|| format!("{id}: buff stats"))?;
            let vals = ints(r, values).ok_or_else(|| format!("{id}: buff values"))?;
            if keys.len() != vals.len() {
                return Err(format!(
                    "{id}: {} buff stats for {} values",
                    keys.len(),
                    vals.len()
                ));
            }
            for (k, v) in keys.into_iter().zip(vals) {
                flask.stats.push((stat(k, id)?, v));
            }
            for k in ints(brow, buff_flags).ok_or_else(|| format!("{id}: buff flags"))? {
                flask.stats.push((stat(k, id)?, 1));
            }
        }
        out.push(flask);
    }
    Ok(Some(out))
}

/// A row key (a null cell is none).
fn key(row: &[Value], col: usize) -> Option<usize> {
    row.get(col)?.as_u64().and_then(|k| usize::try_from(k).ok())
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

    fn files() -> BTreeMap<String, Vec<u8>> {
        [
            table(
                "Flasks",
                &[
                    "BaseItemTypesKey",
                    "LifePerUse",
                    "ManaPerUse",
                    "RecoveryTime",
                    "BuffDefinitionsKey",
                    "BuffStatValues",
                ],
                &json!([
                    [0, 70, 0, 30, null, null],
                    [1, 0, 0, 80, 0, [40, 5]],
                    [2, 0, 0, 60, 1, []]
                ]),
            ),
            table(
                "BuffDefinitions",
                &["Id", "StatsKeys", "GrantedFlags"],
                &json!([
                    ["flask_utility_resist_fire", [0, 1], []],
                    ["flask_utility_haste", [], [2]]
                ]),
            ),
            table(
                "Stats",
                &["Id"],
                &json!([
                    ["base_fire_damage_resistance_%"],
                    ["base_maximum_fire_damage_resistance_%"],
                    ["silver_flask_display_onslaught"]
                ]),
            ),
            table(
                "BaseItemTypes",
                &["Id", "Name"],
                &json!([
                    ["Metadata/Items/Flasks/FlaskLife1", "Small Life Flask"],
                    ["Metadata/Items/Flasks/FlaskUtility2", "Ruby Flask"],
                    ["Metadata/Items/Flasks/FlaskUtility5", "Silver Flask"]
                ]),
            ),
        ]
        .into_iter()
        .collect()
    }

    /// Recovery, duration, the buff's stats with values in order, and its
    /// granted flags at 1 (PoB's exporter).
    #[test]
    fn flasks_should_read_recovery_and_buff_stats() {
        let files = files();
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        let rows = rows(&read).unwrap().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "Small Life Flask");
        assert_eq!((rows[0].life, rows[0].duration), (70, 30));
        assert_eq!(rows[0].buff, None);
        assert_eq!(rows[1].buff.as_deref(), Some("flask_utility_resist_fire"));
        assert_eq!(
            rows[1].stats,
            [
                ("base_fire_damage_resistance_%".to_owned(), 40),
                ("base_maximum_fire_damage_resistance_%".to_owned(), 5)
            ]
        );
        assert_eq!(
            rows[2].stats,
            [("silver_flask_display_onslaught".to_owned(), 1)]
        );
    }

    #[test]
    fn a_buff_with_unpaired_values_should_fail() {
        let mut files = files();
        let (k, v) = table(
            "Flasks",
            &[
                "BaseItemTypesKey",
                "LifePerUse",
                "ManaPerUse",
                "RecoveryTime",
                "BuffDefinitionsKey",
                "BuffStatValues",
            ],
            &json!([[1, 0, 0, 80, 0, [40]]]),
        );
        files.insert(k, v);
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        assert!(
            rows(&read)
                .unwrap_err()
                .contains("2 buff stats for 1 values")
        );
    }
}

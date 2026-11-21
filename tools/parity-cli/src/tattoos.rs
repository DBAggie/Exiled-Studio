//! Tattoos for the PoE1 bundle (Phase 6 P6-11av): `PassiveSkillOverrides`
//! to `poe1/tattoos.json` (`loom_data::bundle::tattoos::Tattoos`), as PoB's
//! `Export/Scripts/tattooPassives.lua` reads it: each override's name, type,
//! and stats (`Stats` paired with `StatValues` by position), a keystone
//! tattoo's passive skill (`AllocatedPassiveSkill`), and whether it grants
//! a skill (`Effect`). A keystone tattoo's name and stats are its
//! passive's (PoB's exporter reads the passive's row for it, so a spec
//! names it so; its own `Name` and `Stats` are empty): its `Stats`
//! paired with `Stat<n>Value` by position. `None` when the extract lacks
//! the table.

use loom_data::bundle::passives::StatAmount;
use loom_data::bundle::tattoos::{Override, Tattoos};
use serde_json::Value;

use crate::extract::{Read, Table};

/// The override type of a keystone tattoo.
const KEYSTONE_TATTOO: &str = "KeystoneTattoo";

/// The overrides (module docs).
///
/// # Errors
/// A message when a column is missing, a key points nowhere, or the
/// stats and their values differ in count.
pub fn rows(read: Read<'_>) -> Result<Option<Tattoos>, String> {
    let (Some(t), Some(stats), Some(types), Some(passives)) = (
        Table::read(read, "PassiveSkillOverrides")?,
        Table::read(read, "Stats")?,
        Table::read(read, "PassiveSkillOverrideTypes")?,
        Table::read(read, "PassiveSkills")?,
    ) else {
        return Ok(None);
    };
    let c = |n: &str| t.column(n);
    let (id, name, keys, values, kind, effect, passive) = (
        c("Id")?,
        c("Name")?,
        c("Stats")?,
        c("StatValues")?,
        c("Type")?,
        c("Effect")?,
        c("AllocatedPassiveSkill")?,
    );
    let text_of = |table: &Table, col: usize, row: i64| -> Result<String, String> {
        usize::try_from(row)
            .ok()
            .and_then(|r| table.rows.get(r))
            .and_then(|r| Table::text(r, col))
            .map(str::to_owned)
            .ok_or_else(|| format!("{}: no row {row}", table.table))
    };
    let (stat_id, type_id, passive_id, passive_name, passive_stats) = (
        stats.column("Id")?,
        types.column("Id")?,
        passives.column("Id")?,
        passives.column("Name")?,
        passives.column("Stats")?,
    );
    let passive_values: Vec<usize> = (1..)
        .map_while(|n| passives.column(&format!("Stat{n}Value")).ok())
        .collect();
    let ints = |r: &[Value], col: usize| -> Result<Vec<i64>, String> {
        match r.get(col) {
            Some(Value::Array(items)) => items
                .iter()
                .map(|v| {
                    v.as_i64()
                        .ok_or_else(|| format!("PassiveSkillOverrides: column {col}"))
                })
                .collect(),
            _ => Ok(Vec::new()),
        }
    };
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        let row_id = Table::text(r, id).unwrap_or_default().to_owned();
        let (k, v) = (ints(r, keys)?, ints(r, values)?);
        if k.len() != v.len() {
            return Err(format!(
                "PassiveSkillOverrides {row_id}: {} stats, {} values",
                k.len(),
                v.len()
            ));
        }
        let mut amounts = k
            .into_iter()
            .zip(v)
            .map(|(key, value)| {
                Ok(StatAmount {
                    stat: text_of(&stats, stat_id, key)?,
                    value,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let kind = match Table::int(r, kind) {
            Some(k) => text_of(&types, type_id, k)?,
            None => String::new(),
        };
        let keystone = Table::int(r, passive).filter(|_| kind == KEYSTONE_TATTOO);
        let name = match keystone {
            Some(p) => {
                let row = usize::try_from(p)
                    .ok()
                    .and_then(|i| passives.rows.get(i))
                    .ok_or_else(|| format!("PassiveSkills: no row {p}"))?;
                let keys = ints(row, passive_stats)?;
                amounts = keys
                    .into_iter()
                    .enumerate()
                    .map(|(i, key)| {
                        let value = passive_values
                            .get(i)
                            .and_then(|c| Table::int(row, *c))
                            .ok_or_else(|| format!("PassiveSkills {p}: no value {i}"))?;
                        Ok(StatAmount {
                            stat: text_of(&stats, stat_id, key)?,
                            value,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                text_of(&passives, passive_name, p)?
            }
            None => Table::text(r, name).unwrap_or_default().to_owned(),
        };
        out.push(Override {
            name,
            kind,
            stats: amounts,
            passive: Table::int(r, passive)
                .map(|p| text_of(&passives, passive_id, p))
                .transpose()?,
            grants_skill: Table::int(r, effect).is_some(),
            id: row_id,
        });
    }
    Ok(Some(Tattoos { overrides: out }))
}

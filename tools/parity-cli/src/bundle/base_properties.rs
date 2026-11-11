//! The item property and monster tables of bundle format 2 (Phase 3 plan,
//! step 1c), from the extract (`loom_data::bundle::properties` has the
//! target types):
//! - `WeaponTypes`, `ArmourTypes`, `ShieldTypes` by base item ID (PoE1
//!   defences as `*Min`/`*Max` columns, PoE2 one column each; one of the two
//!   sets must be there in full);
//! - `DefaultMonsterStats` (`DisplayLevel` read as an integer);
//! - `GameConstants`.
//!
//! Columns both games have are required; a row whose base item is past the
//! table's end is dropped and counted.

use loom_data::bundle::properties::{
    ArmourType, GameConstant, MonsterLevel, PropertyTables, ShieldType, ValueRange, WeaponType,
};
use serde::Serialize;
use serde_json::Value;

use super::passives::required;
use super::skills::{float, one_of};
use super::{Counts, at, column, text_column};
use crate::extract::{Read, Table};

/// What the conversion did not copy one to one.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct PropertySummary {
    /// Weapon, armour, or shield rows whose base item is past the end of
    /// `BaseItemTypes` (dropped).
    pub dropped_rows: usize,
}

fn text(row: &[Value], col: usize) -> String {
    Table::text(row, col).unwrap_or("").to_owned()
}

/// Converts the item property and monster tables (module docs).
///
/// # Errors
/// A message when a table or column is missing, a required value is
/// missing, a float is not finite, or a level is not an integer.
pub fn convert(
    read: Read<'_>,
    counts: &Counts,
) -> Result<(PropertyTables, PropertySummary), String> {
    let bases = text_column(read, "BaseItemTypes", "Id")?;
    let mut summary = PropertySummary::default();
    let tables = PropertyTables {
        weapon_types: weapons(read, &bases, counts, &mut summary)?,
        armour_types: armours(read, &bases, counts, &mut summary)?,
        shield_types: shields(read, &bases, counts, &mut summary)?,
        monster_levels: monster_levels(read)?,
        game_constants: game_constants(read)?,
    };
    Ok((tables, summary))
}

/// The base item of each row, `None` (counted) when past the table's end.
fn base(
    bases: &[String],
    row: &[Value],
    col: usize,
    counts: &Counts,
    summary: &mut PropertySummary,
) -> Option<String> {
    let found = at(bases, Table::int(row, col), counts);
    if found.is_none() {
        summary.dropped_rows += 1;
    }
    found
}

fn weapons(
    read: Read<'_>,
    bases: &[String],
    counts: &Counts,
    summary: &mut PropertySummary,
) -> Result<Vec<WeaponType>, String> {
    let t = Table::require(read, "WeaponTypes")?;
    let key = column(&t, &["BaseItemTypesKey", "BaseItemType"])?;
    let crit = column(&t, &["Critical", "CritChance"])?;
    let c = |n: &str| t.column(n);
    let (speed, lo, hi, range) = (
        c("Speed")?,
        c("DamageMin")?,
        c("DamageMax")?,
        c("RangeMax")?,
    );
    // PoE2 only.
    let reload = t.column("ReloadTime").ok();
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        let Some(base_item) = base(bases, r, key, counts, summary) else {
            continue;
        };
        out.push(WeaponType {
            crit_chance: required(&t, r, crit, "critical chance")?,
            attack_time: required(&t, r, speed, "attack time")?,
            damage_min: required(&t, r, lo, "minimum damage")?,
            damage_max: required(&t, r, hi, "maximum damage")?,
            range: required(&t, r, range, "range")?,
            reload_time: reload
                .map(|col| required(&t, r, col, "reload time"))
                .transpose()?,
            base_item,
        });
    }
    Ok(out)
}

fn armours(
    read: Read<'_>,
    bases: &[String],
    counts: &Counts,
    summary: &mut PropertySummary,
) -> Result<Vec<ArmourType>, String> {
    let t = Table::require(read, "ArmourTypes")?;
    let key = column(&t, &["BaseItemTypesKey", "BaseItemType"])?;
    let speed = t.column("IncreasedMovementSpeed")?;
    // PoE1: a range per defence; PoE2: one value (min = max).
    let cols: [(usize, usize); 4] = match one_of(
        &t,
        &[[
            "ArmourMin",
            "ArmourMax",
            "EvasionMin",
            "EvasionMax",
            "EnergyShieldMin",
            "EnergyShieldMax",
            "WardMin",
            "WardMax",
        ]],
    ) {
        Ok([a0, a1, e0, e1, s0, s1, w0, w1]) => [(a0, a1), (e0, e1), (s0, s1), (w0, w1)],
        Err(_) => {
            let [a, e, s, w] = one_of(&t, &[["Armour", "Evasion", "EnergyShield", "Ward"]])?;
            [(a, a), (e, e), (s, s), (w, w)]
        }
    };
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        let Some(base_item) = base(bases, r, key, counts, summary) else {
            continue;
        };
        let mut ranges = [ValueRange { min: 0, max: 0 }; 4];
        for (slot, (lo, hi)) in ranges.iter_mut().zip(cols) {
            *slot = ValueRange {
                min: required(&t, r, lo, "defence")?,
                max: required(&t, r, hi, "defence")?,
            };
        }
        let [armour, evasion, energy_shield, ward] = ranges;
        out.push(ArmourType {
            base_item,
            armour,
            evasion,
            energy_shield,
            ward,
            movement_speed: required(&t, r, speed, "movement speed")?,
        });
    }
    Ok(out)
}

fn shields(
    read: Read<'_>,
    bases: &[String],
    counts: &Counts,
    summary: &mut PropertySummary,
) -> Result<Vec<ShieldType>, String> {
    let t = Table::require(read, "ShieldTypes")?;
    let key = column(&t, &["BaseItemTypesKey", "BaseItemType"])?;
    let block = t.column("Block")?;
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        let Some(base_item) = base(bases, r, key, counts, summary) else {
            continue;
        };
        out.push(ShieldType {
            base_item,
            block: required(&t, r, block, "block")?,
        });
    }
    Ok(out)
}

fn monster_levels(read: Read<'_>) -> Result<Vec<MonsterLevel>, String> {
    let t = Table::require(read, "DefaultMonsterStats")?;
    let c = |n: &str| t.column(n);
    let level = c("DisplayLevel")?;
    let (damage, evasion, accuracy, life) = (
        c("Damage")?,
        c("Evasion")?,
        c("Accuracy")?,
        c("MonsterLife")?,
    );
    let (armour, ailment, evasive) = (c("Armour")?, c("AilmentThreshold")?, c("EvasiveEvasion")?);
    let ally_life = column(&t, &["MinionLife", "AllyLife"])?;
    let (ally_damage, alt_life_2) = (c("MinionDamage")?, c("AltLife2")?);
    // PoE1 only.
    let (alt_life_1, conversion) = (
        t.column("AltLife1").ok(),
        t.column("MonsterPhysConversionMulti").ok(),
    );
    let opt = |r: &[Value], col: Option<usize>, what: &str| {
        col.map(|c| required(&t, r, c, what)).transpose()
    };
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        let shown = text(r, level);
        let level = shown
            .parse()
            .map_err(|_| format!("DefaultMonsterStats: level {shown:?} is not an integer"))?;
        out.push(MonsterLevel {
            level,
            damage: float(&t, r, damage)?,
            evasion: required(&t, r, evasion, "evasion")?,
            accuracy: required(&t, r, accuracy, "accuracy")?,
            life: required(&t, r, life, "life")?,
            armour: required(&t, r, armour, "armour")?,
            ailment_threshold: required(&t, r, ailment, "ailment threshold")?,
            evasive_evasion: required(&t, r, evasive, "evasive evasion")?,
            ally_life: required(&t, r, ally_life, "ally life")?,
            ally_damage: float(&t, r, ally_damage)?,
            alt_life_2: required(&t, r, alt_life_2, "second alternative life")?,
            alt_life_1: opt(r, alt_life_1, "first alternative life")?,
            phys_conversion_multi: opt(r, conversion, "conversion multiplier")?,
        });
    }
    Ok(out)
}

fn game_constants(read: Read<'_>) -> Result<Vec<GameConstant>, String> {
    let t = Table::require(read, "GameConstants")?;
    let (id, value, divisor) = (t.column("Id")?, t.column("Value")?, t.column("Divisor")?);
    t.rows
        .iter()
        .map(|r| {
            Ok(GameConstant {
                id: text(r, id),
                value: required(&t, r, value, "value")?,
                divisor: required(&t, r, divisor, "divisor")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;

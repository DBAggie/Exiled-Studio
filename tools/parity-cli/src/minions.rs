//! Monster varieties for the PoE1 bundle (Phase 6, P6-7a): `MonsterVarieties`
//! and the `MonsterTypes` they use to `poe1/minions.json`
//! (`loom_data::bundle::poe1::Minions`): every variety, since a spectre can
//! be any monster.
//!
//! PoB v2.67.2 exports its minions and spectres from the same rows
//! (`src/Export/Scripts/minions.lua`, `#emit`), and the conversion keeps
//! what it reads:
//! - a variety's name, tags, `LifeMultiplier`, `DamageMultiplier`, attack
//!   duration (the schema's `AttackSpeed`, milliseconds: PoB's
//!   `AttackDuration`), `MaximumAttackDistance`, main- and off-hand item
//!   classes, granted effects, mods, special mods, and its object type
//!   (`BaseMonsterTypeIndex`);
//! - its type's accuracy, armour, evasion, energy shield, damage spread,
//!   the merciless resistances (`MonsterResistances`), the alternate life
//!   scalings, and whether its base damage ignores attack speed.
//!
//! A key that is no row fails the conversion: the table would be misread.

use loom_data::bundle::poe1::{MinionType, MinionVariety, Minions};
use serde_json::Value;

use crate::extract::{Read, Table};

/// The monster types and varieties; `None` without `MonsterVarieties`.
///
/// # Errors
/// A message when a column is missing or a key is no row.
pub fn rows(read: Read<'_>) -> Result<Option<Minions>, String> {
    let Some(v) = Table::read(read, "MonsterVarieties")? else {
        return Ok(None);
    };
    let types = Table::require(read, "MonsterTypes")?;
    let resists = Table::require(read, "MonsterResistances")?;
    let tags = Table::require(read, "Tags")?;
    let classes = Table::require(read, "ItemClasses")?;
    let effects = Table::require(read, "GrantedEffects")?;
    let mods = Table::require(read, "Mods")?;
    let id_of = |t: &Table, k: usize, what: &str| -> Result<String, String> {
        let col = t.column("Id")?;
        t.rows
            .get(k)
            .and_then(|r| Table::text(r, col))
            .map(str::to_owned)
            .ok_or_else(|| format!("{what} key {k} is no row"))
    };
    let ids = |t: &Table, row: &[Value], col: usize, what: &str| -> Result<Vec<String>, String> {
        keys(row, col)
            .into_iter()
            .map(|k| id_of(t, k, what))
            .collect()
    };
    let int = |row: &[Value], col: usize| row.get(col).and_then(Value::as_i64).unwrap_or(0);
    let flag = |row: &[Value], col: usize| row.get(col).and_then(Value::as_bool).unwrap_or(false);

    let (t_id, t_acc, t_arm, t_eva, t_es, t_spread, t_res, t_alt1, t_alt2, t_ignore, t_player) = (
        types.column("Id")?,
        types.column("Accuracy")?,
        types.column("Armour")?,
        types.column("Evasion")?,
        types.column("EnergyShield")?,
        types.column("DamageSpread")?,
        types.column("Resistances")?,
        types.column("AltLife1")?,
        types.column("AltLife2")?,
        types.column("BaseDamageIgnoresAttackSpeed")?,
        types.column("IsPlayerMinion")?,
    );
    let res_cols = [
        resists.column("FireMerciless")?,
        resists.column("ColdMerciless")?,
        resists.column("LightningMerciless")?,
        resists.column("ChaosMerciless")?,
    ];
    let mut out_types = Vec::new();
    for r in &types.rows {
        let id = Table::text(r, t_id).unwrap_or_default().to_owned();
        let resistances = match key(r, t_res) {
            Some(k) => {
                let row = resists
                    .rows
                    .get(k)
                    .ok_or_else(|| format!("{id}: resistances key {k} is no row"))?;
                res_cols.map(|c| int(row, c))
            }
            None => [0; 4],
        };
        out_types.push(MinionType {
            id,
            accuracy: int(r, t_acc),
            armour: int(r, t_arm),
            evasion: int(r, t_eva),
            energy_shield: int(r, t_es),
            damage_spread: int(r, t_spread),
            resistances,
            alt_life1: flag(r, t_alt1),
            alt_life2: flag(r, t_alt2),
            damage_ignores_attack_speed: flag(r, t_ignore),
            player_minion: flag(r, t_player),
        });
    }

    let c = |n: &str| v.column(n);
    let (v_id, v_type, v_name, v_tags, v_dmg, v_life, v_time, v_range, v_main, v_off) = (
        c("Id")?,
        c("MonsterTypesKey")?,
        c("Name")?,
        c("TagsKeys")?,
        c("DamageMultiplier")?,
        c("LifeMultiplier")?,
        c("AttackSpeed")?,
        c("MaximumAttackDistance")?,
        c("MainHand_ItemClassesKey")?,
        c("OffHand_ItemClassesKey")?,
    );
    let (v_skills, v_mods, v_special, v_object) = (
        c("GrantedEffectsKeys")?,
        c("ModsKeys")?,
        c("Special_ModsKeys")?,
        c("BaseMonsterTypeIndex")?,
    );
    let mut out = Vec::new();
    for r in &v.rows {
        let id = Table::text(r, v_id)
            .filter(|s| !s.is_empty())
            .ok_or("MonsterVarieties: a row has no ID")?
            .to_owned();
        let at = |what: &str| format!("{id}: {what}");
        // Every variety has a type (P6-7a review: a missing one fails).
        let monster_type = key(r, v_type)
            .ok_or_else(|| at("no type"))
            .and_then(|k| id_of(&types, k, &at("type")))?;
        let class = |col: usize| -> Result<Option<String>, String> {
            key(r, col)
                .map(|k| id_of(&classes, k, &at("item class")))
                .transpose()
        };
        out.push(MinionVariety {
            name: Table::text(r, v_name).unwrap_or_default().to_owned(),
            monster_type,
            tags: ids(&tags, r, v_tags, &at("tag"))?,
            damage: int(r, v_dmg),
            life: int(r, v_life),
            attack_time: int(r, v_time),
            attack_range: int(r, v_range),
            main_hand: class(v_main)?,
            off_hand: class(v_off)?,
            skills: ids(&effects, r, v_skills, &at("granted effect"))?,
            mods: ids(&mods, r, v_mods, &at("mod"))?,
            special_mods: ids(&mods, r, v_special, &at("special mod"))?,
            object_type: Table::text(r, v_object).unwrap_or_default().to_owned(),
            id,
        });
    }
    Ok(Some(Minions {
        types: out_types,
        varieties: out,
    }))
}

/// A row key (a null cell is none).
fn key(row: &[Value], col: usize) -> Option<usize> {
    row.get(col)?.as_u64().and_then(|k| usize::try_from(k).ok())
}

/// A list of row keys (a null cell is an empty list).
fn keys(row: &[Value], col: usize) -> Vec<usize> {
    match row.get(col) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(Value::as_u64)
            .filter_map(|k| usize::try_from(k).ok())
            .collect(),
        _ => Vec::new(),
    }
}

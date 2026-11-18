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
//!
//! The `.ot` stats (P6-7b1, [`object_stats`]): for each object type a
//! variety names, the stats PoB's `getOTStats` (`minions.lua` 41 to 73)
//! reads from the extract's monster templates (manifest format 6,
//! `monster_templates`, each file's lines as PoB's reader keeps them).

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::poe1::{MinionType, MinionVariety, Minions, ObjectStats};
use serde_json::Value;

use crate::extract::{Read, Table};

/// The object type PoB's exporter does not read (`minions.lua` 220), and
/// the `extends` targets its reader stops at (55).
const MONSTER: &str = "Metadata/Monsters/Monster";
const NOTHING: &str = "nothing";

/// The monster types and varieties; `None` without `MonsterVarieties`.
/// `manifest` is the extract's: its `monster_templates` section gives the
/// `.ot` stats ([`object_stats`]).
///
/// # Errors
/// A message when a column is missing, a key is no row, or a monster
/// template is malformed or on a cycle.
pub fn rows(read: Read<'_>, manifest: &Value) -> Result<Option<Minions>, String> {
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
    let object_stats = object_stats(read, manifest, out.iter().map(|v| v.object_type.as_str()))?;
    Ok(Some(Minions {
        types: out_types,
        varieties: out,
        object_stats,
    }))
}

/// One kept line of a monster template (the extractor's
/// `monstertemplate::Line`).
#[derive(Clone)]
enum Line {
    Extends(String),
    Stat(String, String),
}

/// The `.ot` stats of `object_types` (module docs), sorted by object type,
/// each type once; types with nothing to carry are left out. `None` when
/// the manifest has no `monster_templates` section (an extract before
/// format 6).
///
/// # Errors
/// A message when a template's file is malformed or a chain loops (PoB's
/// reader would recurse without end).
pub fn object_stats<'a>(
    read: Read<'_>,
    manifest: &Value,
    object_types: impl Iterator<Item = &'a str>,
) -> Result<Option<Vec<ObjectStats>>, String> {
    let Some(files) = manifest["monster_templates"]["files"].as_object() else {
        return Ok(None);
    };
    let mut cache: BTreeMap<String, Option<Vec<Line>>> = BTreeMap::new();
    let mut load = |target: &str| -> Result<Option<Vec<Line>>, String> {
        let key = format!("{}.ot", target.to_ascii_lowercase());
        if let Some(hit) = cache.get(&key) {
            return Ok(hit.clone());
        }
        let lines = match files.get(&key) {
            None => None,
            Some(entry) => {
                let output = entry["output"]
                    .as_str()
                    .filter(|_| entry["status"] == "ok")
                    .ok_or_else(|| format!("{key}: monster template was not read"))?;
                let parsed: Value =
                    serde_json::from_slice(&read(output)?).map_err(|e| format!("{output}: {e}"))?;
                Some(template_lines(&parsed).map_err(|e| format!("{output}: {e}"))?)
            }
        };
        cache.insert(key, lines.clone());
        Ok(lines)
    };
    let wanted: BTreeSet<&str> = object_types
        .filter(|t| !t.is_empty() && *t != MONSTER)
        .collect();
    let mut out = Vec::new();
    for object_type in wanted {
        let mut o = ObjectStats {
            object_type: object_type.to_owned(),
            stats: Vec::new(),
            values: Vec::new(),
            unread: None,
        };
        resolve(object_type, &mut load, &mut Vec::new(), &mut o)?;
        if !(o.stats.is_empty() && o.values.is_empty() && o.unread.is_none()) {
            out.push(o);
        }
    }
    Ok(Some(out))
}

/// Reads one template's lines (`{"lines": [{"extends": ...} | {"stat":
/// {"key", "value"}}]}`).
fn template_lines(parsed: &Value) -> Result<Vec<Line>, String> {
    parsed["lines"]
        .as_array()
        .ok_or("no lines")?
        .iter()
        .map(|l| {
            if let Some(t) = l["extends"].as_str() {
                return Ok(Line::Extends(t.to_owned()));
            }
            match (l["stat"]["key"].as_str(), l["stat"]["value"].as_str()) {
                (Some(k), Some(v)) => Ok(Line::Stat(k.to_owned(), v.to_owned())),
                _ => Err(format!("a malformed line: {l}")),
            }
        })
        .collect()
}

/// PoB's `getOTStats` for `target` (module docs): its lines in order, each
/// superclass's stats where its `extends` line stands. `stack` holds the
/// files being read, to find a loop.
fn resolve(
    target: &str,
    load: &mut impl FnMut(&str) -> Result<Option<Vec<Line>>, String>,
    stack: &mut Vec<String>,
    out: &mut ObjectStats,
) -> Result<(), String> {
    let key = target.to_ascii_lowercase();
    if stack.contains(&key) {
        return Err(format!(
            "{}: its .ot chain loops at {target}",
            out.object_type
        ));
    }
    let Some(lines) = load(target)? else {
        out.unread.get_or_insert_with(|| target.to_owned());
        return Ok(());
    };
    stack.push(key);
    for line in lines {
        match line {
            Line::Extends(s) if s != MONSTER && s != NOTHING => resolve(&s, load, stack, out)?,
            Line::Extends(_) => {}
            Line::Stat(k, v) => match v.parse::<i64>() {
                Ok(n) => out.stats.push((k, n)),
                Err(_) => out.values.push((k, v)),
            },
        }
    }
    stack.pop();
    Ok(())
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;

    /// [`object_stats`] on monster templates given as `(index path,
    /// lines)`, for `types`.
    fn run(files: &[(&str, Value)], types: &[&str]) -> Result<Option<Vec<ObjectStats>>, String> {
        let mut entries = serde_json::Map::new();
        let mut store = BTreeMap::new();
        for (path, lines) in files {
            let out = format!("monster_templates/{path}.json");
            entries.insert((*path).to_owned(), json!({"status": "ok", "output": out}));
            store.insert(out, serde_json::to_vec(&json!({ "lines": lines })).unwrap());
        }
        let manifest = json!({"monster_templates": {"files": entries}});
        let read = |p: &str| store.get(p).cloned().ok_or_else(|| p.to_owned());
        object_stats(&read, &manifest, types.iter().copied())
    }

    fn ext(target: &str) -> Value {
        json!({ "extends": target })
    }

    fn stat(key: &str, value: &str) -> Value {
        json!({"stat": {"key": key, "value": value}})
    }

    fn pairs(list: &[(&str, i64)]) -> Vec<(String, i64)> {
        list.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    /// PoB's `getOTStats` (`minions.lua` 41 to 73): a superclass's stats
    /// where its `extends` line stands, repeats kept, `Monster` and
    /// `nothing` not read; each object type once, sorted; a type with
    /// nothing to carry left out.
    #[test]
    fn object_stats_should_follow_pobs_reader() {
        let files = [
            (
                "metadata/monsters/a/child.ot",
                json!([
                    ext("Metadata/Monsters/Monster"),
                    ext("Metadata/Monsters/A/Parent"),
                    stat("own", "2"),
                    ext("Metadata/Monsters/A/Second"),
                    stat("own", "3")
                ]),
            ),
            (
                "metadata/monsters/a/parent.ot",
                json!([ext("nothing"), stat("from_parent", "1")]),
            ),
            (
                "metadata/monsters/a/second.ot",
                json!([stat("from_second", "-4"), stat("script", "{x}")]),
            ),
            ("metadata/monsters/monster.ot", json!([stat("never", "9")])),
            (
                "metadata/monsters/b/out.ot",
                json!([ext("Metadata/Effects/Effect"), stat("b", "1")]),
            ),
            (
                "metadata/monsters/c/empty.ot",
                json!([ext("Metadata/Monsters/Monster")]),
            ),
        ];
        let types = [
            "Metadata/Monsters/B/Out",
            "Metadata/Monsters/A/Child",
            "Metadata/Monsters/C/Empty",
            "Metadata/Monsters/Monster",
            "",
            "Metadata/Monsters/A/Child",
        ];
        let got = run(&files, &types).unwrap().unwrap();
        let ids: Vec<&str> = got.iter().map(|o| o.object_type.as_str()).collect();
        assert_eq!(
            ids,
            ["Metadata/Monsters/A/Child", "Metadata/Monsters/B/Out"]
        );
        let child = &got[0];
        assert_eq!(
            child.stats,
            pairs(&[
                ("from_parent", 1),
                ("own", 2),
                ("from_second", -4),
                ("own", 3)
            ])
        );
        assert_eq!(child.values, [("script".to_owned(), "{x}".to_owned())]);
        assert_eq!(child.unread, None);
        // A chain that leaves the extract keeps what it read and names the
        // first file it could not read.
        let out = &got[1];
        assert_eq!(out.stats, pairs(&[("b", 1)]));
        assert_eq!(out.unread.as_deref(), Some("Metadata/Effects/Effect"));
    }

    /// A chain that loops fails the conversion (PoB would not end), and a
    /// diamond is no loop: its shared file is read twice, as PoB does.
    #[test]
    fn object_stats_should_refuse_a_loop_and_keep_a_diamond() {
        let looping = [
            (
                "metadata/monsters/x.ot",
                json!([ext("Metadata/Monsters/Y")]),
            ),
            (
                "metadata/monsters/y.ot",
                json!([ext("Metadata/Monsters/X")]),
            ),
        ];
        let err = run(&looping, &["Metadata/Monsters/X"]).unwrap_err();
        assert!(err.contains("loops"), "{err}");
        let diamond = [
            (
                "metadata/monsters/d.ot",
                json!([ext("Metadata/Monsters/L"), ext("Metadata/Monsters/R")]),
            ),
            (
                "metadata/monsters/l.ot",
                json!([ext("Metadata/Monsters/S")]),
            ),
            (
                "metadata/monsters/r.ot",
                json!([ext("Metadata/Monsters/S")]),
            ),
            ("metadata/monsters/s.ot", json!([stat("shared", "1")])),
        ];
        let got = run(&diamond, &["Metadata/Monsters/D"]).unwrap().unwrap();
        assert_eq!(got[0].stats, pairs(&[("shared", 1), ("shared", 1)]));
    }

    /// An extract before manifest format 6 has no monster templates: the
    /// `.ot` stats are unknown, not empty.
    #[test]
    fn object_stats_should_be_unknown_without_monster_templates() {
        let read = |p: &str| Err(p.to_owned());
        let got = object_stats(
            &read,
            &json!({"format": 5}),
            ["Metadata/Monsters/A"].into_iter(),
        );
        assert_eq!(got, Ok(None));
    }
}

//! The character and passive tables of bundle format 2 (Phase 3 plan, step
//! 1b), from the extract (`loom_data::bundle::passives` has the target
//! types):
//! - `Characters` to characters (`IntegerId` as the class number); a class
//!   names its object template when the extract has `<Id>.ot` (PoE1);
//! - `Ascendancy` to ascendancies (`ClassNo`);
//! - the extract's object templates (manifest format 5, `object_templates`)
//!   to their `Stats` blocks: integer entries whose key is a stat as stats,
//!   others kept as written (`values`);
//! - `PassiveSkills` to passives (stats paired with `Stat<n>Value` by
//!   position), `PassiveSkillMasteryGroups`, `PassiveSkillMasteryEffects`;
//! - the `PassiveSkillTrees` row `Default` and its graph file to the tree.
//!
//! Columns are required; the summary counts what was not copied one to one.

use std::collections::BTreeSet;

use loom_data::bundle::passives::{
    Ascendancy, Character, MasteryEffect, MasteryGroup, ObjectTemplate, Passive, PassiveTables,
    PassiveTree, StatAmount, TemplateValue, TreeConnection, TreeGroup, TreeNode,
};
use serde::Serialize;
use serde_json::Value;

use super::skills::to_f32;
use super::{Counts, at, column, text_column};
use crate::extract::{Read, Table};
use crate::mod_filter::ints;

/// The `PassiveSkillTrees` row of the main tree.
pub const MAIN_TREE: &str = "Default";

/// What the conversion did not copy one to one.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct PassiveSummary {
    /// Object templates read.
    pub templates: usize,
    /// Template `Stats` entries kept as written (no stat, or no integer).
    pub template_values: usize,
    /// Stats dropped: no value column for their position, or the stat
    /// reference past the table's end.
    pub dropped_stats: usize,
}

fn bump(cell: &mut usize) {
    *cell += 1;
}

fn flag(row: &[Value], col: usize) -> bool {
    row.get(col).and_then(Value::as_bool).unwrap_or(false)
}

fn int_or_0(row: &[Value], col: usize) -> i64 {
    Table::int(row, col).unwrap_or(0)
}

/// An integer that identifies a row or drives the calc: missing is an
/// error, never 0.
pub(super) fn required(t: &Table, row: &[Value], col: usize, what: &str) -> Result<i64, String> {
    Table::int(row, col).ok_or_else(|| {
        let id = row.first().and_then(Value::as_str).unwrap_or("?");
        format!("{}: row {id:?} has no {what}", t.table)
    })
}

fn text(row: &[Value], col: usize) -> String {
    Table::text(row, col).unwrap_or("").to_owned()
}

/// IDs of the rows a list cell names.
fn ids(list: &[String], row: &[Value], col: usize, counts: &Counts) -> Vec<String> {
    ints(row, col)
        .into_iter()
        .filter_map(|n| at(list, Some(n), counts))
        .collect()
}

/// The `Stat<n>Value` columns, in order.
fn value_columns(t: &Table) -> Vec<usize> {
    (1..)
        .map_while(|n| t.column(&format!("Stat{n}Value")).ok())
        .collect()
}

/// `Stats` paired with the value columns by position.
fn stat_values(
    stats: &[String],
    row: &[Value],
    keys: usize,
    values: &[usize],
    counts: &Counts,
    summary: &mut PassiveSummary,
) -> Vec<StatAmount> {
    let mut out = Vec::new();
    for (i, k) in ints(row, keys).into_iter().enumerate() {
        let stat = at(stats, Some(k), counts);
        let value = values.get(i).and_then(|c| Table::int(row, *c));
        match (stat, value) {
            (Some(stat), Some(value)) => out.push(StatAmount { stat, value }),
            _ => bump(&mut summary.dropped_stats),
        }
    }
    out
}

/// Converts the character and passive tables (module docs).
///
/// # Errors
/// A message when a table, column, graph, or the manifest's
/// `object_templates` section is missing or malformed.
pub fn convert(
    read: Read<'_>,
    manifest: &Value,
    counts: &Counts,
) -> Result<(PassiveTables, PassiveSummary), String> {
    let mut summary = PassiveSummary::default();
    let stats = text_column(read, "Stats", "Id")?;
    let object_templates = templates(read, manifest, &stats, &mut summary)?;
    let template_ids: BTreeSet<&str> = object_templates.iter().map(|t| t.id.as_str()).collect();
    let characters = characters(read, &template_ids)?;
    let character_ids = text_column(read, "Characters", "Id")?;
    let tables = PassiveTables {
        ascendancies: ascendancies(read, &character_ids, counts)?,
        passives: passives(read, &stats, &character_ids, counts, &mut summary)?,
        mastery_groups: mastery_groups(read, counts)?,
        mastery_effects: mastery_effects(read, &stats, counts, &mut summary)?,
        tree: tree(read)?,
        characters,
        object_templates,
    };
    Ok((tables, summary))
}

fn characters(read: Read<'_>, templates: &BTreeSet<&str>) -> Result<Vec<Character>, String> {
    let t = Table::require(read, "Characters")?;
    let c = |n: &str| t.column(n);
    let (id, name, number) = (c("Id")?, c("Name")?, c("IntegerId")?);
    let (life, mana) = (c("BaseMaxLife")?, c("BaseMaxMana")?);
    let (s, d, i) = (
        c("BaseStrength")?,
        c("BaseDexterity")?,
        c("BaseIntelligence")?,
    );
    t.rows
        .iter()
        .map(|r| {
            let id = text(r, id);
            let own = id.to_ascii_lowercase();
            Ok(Character {
                template: templates.contains(own.as_str()).then_some(own),
                id,
                name: text(r, name),
                class_number: required(&t, r, number, "class number")?,
                base_life: required(&t, r, life, "base life")?,
                base_mana: required(&t, r, mana, "base mana")?,
                strength: required(&t, r, s, "strength")?,
                dexterity: required(&t, r, d, "dexterity")?,
                intelligence: required(&t, r, i, "intelligence")?,
            })
        })
        .collect()
}

fn ascendancies(
    read: Read<'_>,
    characters: &[String],
    counts: &Counts,
) -> Result<Vec<Ascendancy>, String> {
    let t = Table::require(read, "Ascendancy")?;
    let (id, name, character, number) = (
        t.column("Id")?,
        t.column("Name")?,
        t.column("Character")?,
        t.column("ClassNo")?,
    );
    t.rows
        .iter()
        .map(|r| {
            Ok(Ascendancy {
                id: text(r, id),
                name: text(r, name),
                character: at(characters, Table::int(r, character), counts),
                class_number: required(&t, r, number, "class number")?,
            })
        })
        .collect()
}

/// An object template's ID: its lowercase path without `.ot`.
fn template_id(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    lower.strip_suffix(".ot").unwrap_or(&lower).to_owned()
}

fn templates(
    read: Read<'_>,
    manifest: &Value,
    stats: &[String],
    summary: &mut PassiveSummary,
) -> Result<Vec<ObjectTemplate>, String> {
    let files = manifest["object_templates"]["files"]
        .as_object()
        .ok_or("manifest.json: no object_templates section (extract format 5 needed)")?;
    let known: BTreeSet<&str> = stats.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for (path, entry) in files {
        if entry["status"] != "ok" {
            return Err(format!("{path}: object template did not parse"));
        }
        let output = entry["output"]
            .as_str()
            .ok_or_else(|| format!("{path}: no output"))?;
        let parsed: Value =
            serde_json::from_slice(&read(output)?).map_err(|e| format!("{output}: {e}"))?;
        let extends = parsed["extends"]
            .as_str()
            .ok_or_else(|| format!("{output}: no extends"))?;
        let (mut tstats, mut values) = (Vec::new(), Vec::new());
        let blocks = parsed["blocks"].as_array().map_or(&[][..], Vec::as_slice);
        for e in blocks
            .iter()
            .filter(|b| b["name"] == "Stats")
            .flat_map(|b| b["entries"].as_array().map_or(&[][..], Vec::as_slice))
        {
            let key = e["key"].as_str().unwrap_or("");
            let value = e["value"].as_str().unwrap_or("");
            match (
                known.contains(key),
                e["quoted"] == false,
                value.parse::<i64>(),
            ) {
                (true, true, Ok(v)) => tstats.push(StatAmount {
                    stat: key.to_owned(),
                    value: v,
                }),
                _ => {
                    bump(&mut summary.template_values);
                    values.push(TemplateValue {
                        key: key.to_owned(),
                        value: value.to_owned(),
                    });
                }
            }
        }
        bump(&mut summary.templates);
        out.push(ObjectTemplate {
            id: template_id(path),
            extends: (extends != "nothing").then(|| extends.to_ascii_lowercase()),
            stats: tstats,
            values,
        });
    }
    Ok(out)
}

fn passives(
    read: Read<'_>,
    stats: &[String],
    characters: &[String],
    counts: &Counts,
    summary: &mut PassiveSummary,
) -> Result<Vec<Passive>, String> {
    let groups = text_column(read, "PassiveSkillMasteryGroups", "Id")?;
    let ascendancies = text_column(read, "Ascendancy", "Id")?;
    let t = Table::require(read, "PassiveSkills")?;
    let c = |n: &str| t.column(n);
    let (id, node, name, keys) = (c("Id")?, c("PassiveSkillGraphId")?, c("Name")?, c("Stats")?);
    let values = value_columns(&t);
    let (keystone, notable, socket, expansion, anoint) = (
        c("IsKeystone")?,
        c("IsNotable")?,
        c("IsJewelSocket")?,
        c("IsExpansion")?,
        c("IsAnointmentOnly")?,
    );
    let (choice, option, points, mastery) = (
        c("IsMultipleChoice")?,
        c("IsMultipleChoiceOption")?,
        c("SkillPointsGranted")?,
        c("MasteryGroup")?,
    );
    let ascendancy = column(&t, &["AscendancyKey", "Ascendancy"])?;
    let (start, classes) = (c("IsAscendancyStartingNode")?, c("Characters")?);
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        out.push(Passive {
            id: text(r, id),
            node: required(&t, r, node, "node number")?,
            name: text(r, name),
            stats: stat_values(stats, r, keys, &values, counts, summary),
            keystone: flag(r, keystone),
            notable: flag(r, notable),
            jewel_socket: flag(r, socket),
            expansion: flag(r, expansion),
            anoint_only: flag(r, anoint),
            multiple_choice: flag(r, choice),
            multiple_choice_option: flag(r, option),
            skill_points: int_or_0(r, points),
            mastery_group: at(&groups, Table::int(r, mastery), counts),
            ascendancy: at(&ascendancies, Table::int(r, ascendancy), counts),
            ascendancy_start: flag(r, start),
            characters: ids(characters, r, classes, counts),
        });
    }
    Ok(out)
}

fn mastery_groups(read: Read<'_>, counts: &Counts) -> Result<Vec<MasteryGroup>, String> {
    let effects = text_column(read, "PassiveSkillMasteryEffects", "Id")?;
    let t = Table::require(read, "PassiveSkillMasteryGroups")?;
    let (id, list) = (t.column("Id")?, t.column("MasteryEffects")?);
    Ok(t.rows
        .iter()
        .map(|r| MasteryGroup {
            id: text(r, id),
            effects: ids(&effects, r, list, counts),
        })
        .collect())
}

fn mastery_effects(
    read: Read<'_>,
    stats: &[String],
    counts: &Counts,
    summary: &mut PassiveSummary,
) -> Result<Vec<MasteryEffect>, String> {
    let t = Table::require(read, "PassiveSkillMasteryEffects")?;
    let (id, hash, keys) = (t.column("Id")?, t.column("HASH16")?, t.column("Stats")?);
    let values = value_columns(&t);
    let mut out = Vec::with_capacity(t.rows.len());
    for r in &t.rows {
        out.push(MasteryEffect {
            id: text(r, id),
            hash: required(&t, r, hash, "hash")?,
            stats: stat_values(stats, r, keys, &values, counts, summary),
        });
    }
    Ok(out)
}

fn num(v: &Value, what: &str) -> Result<i64, String> {
    v.as_i64()
        .ok_or_else(|| format!("passive graph: {what} is not an integer"))
}

fn tree(read: Read<'_>) -> Result<PassiveTree, String> {
    let t = Table::require(read, "PassiveSkillTrees")?;
    let (id, graph) = (t.column("Id")?, t.column("PassiveSkillGraph")?);
    let graph = t
        .rows
        .iter()
        .find(|r| Table::text(r, id) == Some(MAIN_TREE))
        .and_then(|r| Table::text(r, graph))
        .ok_or_else(|| format!("PassiveSkillTrees: no {MAIN_TREE} row with a graph"))?
        .to_owned();
    let path = format!(
        "passive_skill_graphs/{}.psg.json",
        graph.to_ascii_lowercase()
    );
    let g: Value = serde_json::from_slice(&read(&path)?).map_err(|e| format!("{path}: {e}"))?;
    // Every list the tree has must be there: a missing one is an error,
    // never an empty tree.
    let list = |v: &Value, what: &str| -> Result<Vec<Value>, String> {
        v.as_array()
            .cloned()
            .ok_or_else(|| format!("{path}: no {what} list"))
    };
    let ints_of = |v: &Value, what: &str| -> Result<Vec<i64>, String> {
        list(v, what)?.iter().map(|x| num(x, what)).collect()
    };
    let mut groups = Vec::new();
    for gr in list(&g["groups"], "groups")? {
        let mut nodes = Vec::new();
        for n in list(&gr["nodes"], "nodes")? {
            let mut connections = Vec::new();
            for c in list(&n["connections"], "connections")? {
                connections.push(TreeConnection {
                    node: num(&c["id"], "a connection")?,
                    orbit: if c["orbit"].is_null() {
                        None
                    } else {
                        Some(num(&c["orbit"], "a connection orbit")?)
                    },
                });
            }
            nodes.push(TreeNode {
                node: num(&n["id"], "a node")?,
                orbit: num(&n["orbit"], "an orbit")?,
                orbit_index: num(&n["orbit_index"], "an orbit index")?,
                connections,
            });
        }
        let coord = |k: &str| {
            gr[k]
                .as_f64()
                .map(to_f32)
                .ok_or_else(|| format!("passive graph: group {k} is not a number"))
        };
        groups.push(TreeGroup {
            x: coord("x")?,
            y: coord("y")?,
            flags: num(&gr["flags"], "group flags")?,
            nodes,
        });
    }
    Ok(PassiveTree {
        tree: MAIN_TREE.to_owned(),
        graph,
        orbit_slots: ints_of(&g["orbit_slots"], "an orbit slot")?,
        roots: ints_of(&g["roots"], "a root")?,
        groups,
    })
}

#[cfg(test)]
mod tests;

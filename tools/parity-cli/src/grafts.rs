//! Graft skill lines (Spike B, PR B5g; area 3 of the B4b verdict,
//! `docs/spikes/spike-b-resolution.md` section 7a).
//!
//! A graft's implicit shows the skill it grants as `Uses level {1} {0}`
//! (`ClientStrings` `BrequelGraftSkillString`; `{1}` the skill level, `{0}`
//! the skill's name). No stat description holds the line and no stat
//! stands for it: the game reads the skill from `BrequelGrafts` (its `Id`
//! is the last segment of the graft base's metadata id; `SkillGem` the
//! granted skill gem, whose base item names the skill) and the level from
//! the area level (`BrequelGraftGrantedSkillLevels`). PoE1 26530219: 16
//! grafts, levels 1 to 30.
//!
//! A line resolves as a **granted skill** (kind `granted-skill`, not a
//! stat ID) when the item's base is a graft, the line matches the wrapper
//! with a whole-number level that `BrequelGraftGrantedSkillLevels` allows,
//! and the name equals the graft's skill. A range level (`(1-30)`, PoB's
//! placeholder text) or another name does not resolve and keeps its
//! category.

//!
//! Since P2-2d the reading is `loom_parser::import::grafts`; the wrapper id
//! is PoE1 data in `crate::poe1`. This module joins the tables.

use std::collections::BTreeMap;

use loom_parser::import::grafts::Graft;
pub use loom_parser::import::grafts::Grafts;

use crate::extract::{Read, Table};
pub use crate::poe1::GRAFT_WRAPPER_ID as WRAPPER_ID;

/// What [`load`] read, for the summary: a missing wrapper or level table
/// would turn every graft line back to `parser-gap`, so it is shown.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Inputs {
    /// `BrequelGraftSkillString` found with `{1}` then `{0}`.
    pub wrapper: bool,
    /// The `SkillLevel` range.
    pub levels: Option<(u32, u32)>,
    /// Base names whose rows name different skills (dropped).
    pub conflicts: Vec<String>,
}

/// What was read, for the summary.
#[must_use]
pub fn inputs(grafts: &Grafts) -> Inputs {
    Inputs {
        wrapper: grafts.has_wrapper(),
        levels: grafts.levels(),
        conflicts: grafts.conflicts().to_vec(),
    }
}

/// Reads `BrequelGrafts`, `SkillGems`, `BaseItemTypes`,
/// `BrequelGraftGrantedSkillLevels`, and the wrapper from `ClientStrings`.
/// The graft tables are optional on purpose: an extract before PR B5x, or
/// a test fixture, gives no grafts; what was read is in [`inputs`].
///
/// # Errors
/// A message when a table or column the others need is missing.
pub fn load(read: Read<'_>) -> Result<Grafts, String> {
    let Some(raw) = rows(read)? else {
        return Ok(Grafts::default());
    };
    let bases = raw.bases.into_iter().map(|b| {
        (
            b.base,
            Graft {
                id: b.graft,
                skill: b.skill,
            },
        )
    });
    Ok(Grafts::new(bases, raw.wrapper.as_deref(), &raw.levels))
}

/// The graft rows as read, for the bundle converter and [`load`]: each
/// graft base with its graft, the granted gem (its base item's `Id`) and
/// skill name; the allowed levels; the wrapper text.
pub struct GraftRows {
    /// Graft bases, in `BaseItemTypes` order.
    pub bases: Vec<loom_data::bundle::poe1::GraftBase>,
    /// `BrequelGraftGrantedSkillLevels.SkillLevel`.
    pub levels: Vec<u32>,
    /// The `ClientStrings` wrapper text.
    pub wrapper: Option<String>,
}

/// Reads the graft tables; `None` without `BrequelGrafts`.
///
/// # Errors
/// A message when a table or column the others need is missing.
pub fn rows(read: Read<'_>) -> Result<Option<GraftRows>, String> {
    let Some(grafts) = Table::read(read, "BrequelGrafts")? else {
        return Ok(None);
    };
    let bases = Table::require(read, "BaseItemTypes")?;
    let gems = Table::require(read, "SkillGems")?;
    let (base_id, base_name) = (bases.column("Id")?, bases.column("Name")?);
    let gem_base = gems.column("BaseItemTypesKey")?;
    let (graft_id, graft_gem) = (grafts.column("Id")?, grafts.column("SkillGem")?);
    let row_at = |rows: &[Vec<serde_json::Value>], n: Option<i64>| {
        n.and_then(|n| usize::try_from(n).ok())
            .and_then(|n| rows.get(n).cloned())
    };
    let mut skill_of: BTreeMap<String, (String, String)> = BTreeMap::new();
    for g in &grafts.rows {
        let Some(id) = Table::text(g, graft_id) else {
            continue;
        };
        let skill = row_at(&gems.rows, Table::int(g, graft_gem))
            .and_then(|gem| row_at(&bases.rows, Table::int(&gem, gem_base)))
            .and_then(|b| {
                Some((
                    Table::text(&b, base_name)?.to_owned(),
                    Table::text(&b, base_id)?.to_owned(),
                ))
            });
        if let Some(skill) = skill {
            skill_of.insert(id.to_owned(), skill);
        }
    }
    let mut graft_bases = Vec::new();
    for b in &bases.rows {
        let (Some(id), Some(name)) = (Table::text(b, base_id), Table::text(b, base_name)) else {
            continue;
        };
        let segment = id.rsplit('/').next().unwrap_or(id);
        if let Some((skill, gem)) = skill_of.get(segment) {
            graft_bases.push(loom_data::bundle::poe1::GraftBase {
                base: name.to_owned(),
                graft: segment.to_owned(),
                gem: gem.clone(),
                skill: skill.clone(),
            });
        }
    }
    let mut levels = Vec::new();
    if let Some(t) = Table::read(read, "BrequelGraftGrantedSkillLevels")? {
        let c = t.column("SkillLevel")?;
        levels = t
            .rows
            .iter()
            .filter_map(|r| Table::int(r, c).and_then(|v| u32::try_from(v).ok()))
            .collect();
    }
    let mut wrapper = None;
    if let Some(cs) = Table::read(read, "ClientStrings")? {
        let (id, text) = (cs.column("Id")?, cs.column("Text")?);
        wrapper = cs
            .rows
            .iter()
            .find(|r| Table::text(r, id) == Some(WRAPPER_ID))
            .and_then(|r| Table::text(r, text))
            .map(str::to_owned);
    }
    Ok(Some(GraftRows {
        bases: graft_bases,
        levels,
        wrapper,
    }))
}

/// One graft base with the standard wrapper and levels 1..30 (tests of
/// other modules).
#[cfg(test)]
#[must_use]
pub fn with_one(base: &str, id: &str, skill: &str) -> Grafts {
    Grafts::new(
        [(
            base.to_owned(),
            Graft {
                id: id.to_owned(),
                skill: skill.to_owned(),
            },
        )],
        Some("Uses level {1} {0}"),
        &[1, 30],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(name: &str, columns: &[&str], rows: &str) -> String {
        let cols: Vec<String> = columns
            .iter()
            .map(|c| format!(r#"{{"name": "{c}", "references": null}}"#))
            .collect();
        format!(
            r#"{{"table": "{name}", "columns": [{}], "rows": {rows}}}"#,
            cols.join(", ")
        )
    }

    /// Two graft bases (`TulGraft4`; `XophGraft1` twice under one name with
    /// different skills), their gems, levels 1..30, and the wrapper.
    fn read(path: &str) -> Result<Vec<u8>, String> {
        let text = match path {
            "tables/BrequelGrafts.json" => table(
                "BrequelGrafts",
                &["Id", "SkillGem"],
                r#"[["TulGraft4", 0], ["XophGraft1", 1], ["XophGraft2", 0]]"#,
            ),
            "tables/SkillGems.json" => table("SkillGems", &["BaseItemTypesKey"], "[[0], [1]]"),
            "tables/BaseItemTypes.json" => table(
                "BaseItemTypes",
                &["Id", "Name"],
                r#"[["Gems/Stillness", "Preserving Stillness"], ["Gems/Flame", "Heart of Flame"],
                    ["Metadata/Items/Chayula/TulGraft4", "Aegis Tulgraft"],
                    ["Metadata/Items/Chayula/XophGraft1", "Twin Xophgraft"],
                    ["Metadata/Items/Chayula/XophGraft2", "Twin Xophgraft"]]"#,
            ),
            "tables/BrequelGraftGrantedSkillLevels.json" => table(
                "BrequelGraftGrantedSkillLevels",
                &["SkillLevel"],
                "[[1], [30]]",
            ),
            "tables/ClientStrings.json" => table(
                "ClientStrings",
                &["Id", "Text"],
                r#"[["BrequelGraftSkillString", "Uses level {1} {0}"]]"#,
            ),
            _ => return Err(format!("{path}: not found")),
        };
        Ok(text.into_bytes())
    }

    #[test]
    fn load_should_join_bases_grafts_and_gems_and_drop_conflicts() {
        let g = load(&read).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(
            inputs(&g),
            Inputs {
                wrapper: true,
                levels: Some((1, 30)),
                conflicts: vec!["Twin Xophgraft".to_owned()],
            }
        );
        let r = g
            .resolve("Aegis Tulgraft", "Uses level 12 Preserving Stillness")
            .unwrap();
        assert_eq!(
            (r.graft.skill.as_str(), r.level),
            ("Preserving Stillness", 12)
        );
        assert_eq!(
            g.resolve("Twin Xophgraft", "Uses level 1 Heart of Flame"),
            None
        );
    }

    #[test]
    fn load_should_give_no_grafts_without_the_table() {
        let none = |_: &str| -> Result<Vec<u8>, String> { Err("absent".to_owned()) };
        let g = load(&none).unwrap();
        assert_eq!((g.len(), inputs(&g)), (0, Inputs::default()));
    }

    #[test]
    fn resolve_should_refuse_ranges_other_names_bases_and_levels() {
        let g = with_one("Aegis Tulgraft", "TulGraft4", "Preserving Stillness");
        assert_eq!(
            g.resolve("Aegis Tulgraft", "Uses level (1-30) Aegis Tulgraft"),
            None
        );
        assert_eq!(
            g.resolve("Aegis Tulgraft", "Uses level 30 Call the Pyre"),
            None
        );
        assert_eq!(
            g.resolve("Coral Ring", "Uses level 30 Preserving Stillness"),
            None
        );
        assert_eq!(
            g.resolve("Aegis Tulgraft", "Uses level 31 Preserving Stillness"),
            None
        );
    }
}

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

use std::collections::BTreeMap;

use crate::extract::{Read, Table};

/// The `ClientStrings` id of the wrapper.
pub const WRAPPER_ID: &str = "BrequelGraftSkillString";

/// The granted skill of one graft base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graft {
    /// `BrequelGrafts.Id`.
    pub id: String,
    /// The granted skill gem's base item name.
    pub skill: String,
}

/// The wrapper split around `{1}` (level) and `{0}` (name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Wrapper {
    prefix: String,
    middle: String,
    suffix: String,
}

impl Wrapper {
    /// `<prefix>{1}<middle>{0}<suffix>`; `None` for any other shape.
    fn new(text: &str) -> Option<Self> {
        let (prefix, rest) = text.split_once("{1}")?;
        let (middle, suffix) = rest.split_once("{0}")?;
        if [prefix, middle, suffix].iter().any(|p| p.contains('{')) {
            return None;
        }
        Some(Self {
            prefix: prefix.to_owned(),
            middle: middle.to_owned(),
            suffix: suffix.to_owned(),
        })
    }

    /// `(level, name)` of a line, when the level is a whole number.
    fn read<'l>(&self, line: &'l str) -> Option<(i64, &'l str)> {
        let rest = line.trim().strip_prefix(self.prefix.as_str())?;
        let (level, name) = rest.split_once(self.middle.as_str())?;
        let name = name.strip_suffix(self.suffix.as_str())?;
        if level.is_empty() || !level.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let level = level.parse().ok()?;
        (!name.is_empty()).then_some((level, name))
    }
}

/// Graft bases by name, the wrapper, and the allowed skill levels.
#[derive(Debug, Clone, Default)]
pub struct Grafts {
    by_base: BTreeMap<String, Graft>,
    wrapper: Option<Wrapper>,
    /// Lowest and highest `SkillLevel`; a level in between is allowed even
    /// if no row names it (the table has every level 1..30 in PoE1
    /// 26530219).
    levels: Option<(i64, i64)>,
    conflicts: Vec<String>,
}

/// What [`Grafts::load`] read, for the summary: a missing wrapper or level
/// table would turn every graft line back to `parser-gap`, so it is shown.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Inputs {
    /// `BrequelGraftSkillString` found with `{1}` then `{0}`.
    pub wrapper: bool,
    /// The `SkillLevel` range.
    pub levels: Option<(i64, i64)>,
    /// Base names whose rows name different skills (dropped).
    pub conflicts: Vec<String>,
}

/// A resolved graft line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantedSkill {
    /// The graft.
    pub graft: Graft,
    /// The skill level shown.
    pub level: i64,
}

impl Grafts {
    /// Reads `BrequelGrafts`, `SkillGems`, `BaseItemTypes`,
    /// `BrequelGraftGrantedSkillLevels`, and the wrapper from
    /// `ClientStrings`. The graft tables are optional on purpose: an
    /// extract before PR B5x, or a test fixture, gives no grafts; what was
    /// read is in [`Grafts::inputs`].
    ///
    /// # Errors
    /// A message when a table or column the others need is missing.
    pub fn load(read: Read<'_>) -> Result<Self, String> {
        let Some(grafts) = Table::read(read, "BrequelGrafts")? else {
            return Ok(Self::default());
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
        let mut skill_of: BTreeMap<String, String> = BTreeMap::new();
        for g in &grafts.rows {
            let Some(id) = Table::text(g, graft_id) else {
                continue;
            };
            let skill = row_at(&gems.rows, Table::int(g, graft_gem))
                .and_then(|gem| row_at(&bases.rows, Table::int(&gem, gem_base)))
                .and_then(|b| Table::text(&b, base_name).map(str::to_owned));
            if let Some(skill) = skill {
                skill_of.insert(id.to_owned(), skill);
            }
        }
        let mut out = Self::default();
        for b in &bases.rows {
            let (Some(id), Some(name)) = (Table::text(b, base_id), Table::text(b, base_name))
            else {
                continue;
            };
            let segment = id.rsplit('/').next().unwrap_or(id);
            if let Some(skill) = skill_of.get(segment) {
                let graft = Graft {
                    id: segment.to_owned(),
                    skill: skill.clone(),
                };
                match out.by_base.get(name) {
                    Some(old) if *old != graft => {
                        out.conflicts.push(name.to_owned());
                    }
                    _ => {
                        out.by_base.insert(name.to_owned(), graft);
                    }
                }
            }
        }
        out.conflicts.sort();
        out.conflicts.dedup();
        for name in &out.conflicts {
            out.by_base.remove(name);
        }
        if let Some(levels) = Table::read(read, "BrequelGraftGrantedSkillLevels")? {
            let c = levels.column("SkillLevel")?;
            let all: Vec<i64> = levels
                .rows
                .iter()
                .filter_map(|r| Table::int(r, c))
                .collect();
            out.levels = all.iter().min().copied().zip(all.iter().max().copied());
        }
        if let Some(cs) = Table::read(read, "ClientStrings")? {
            let (id, text) = (cs.column("Id")?, cs.column("Text")?);
            out.wrapper = cs
                .rows
                .iter()
                .find(|r| Table::text(r, id) == Some(WRAPPER_ID))
                .and_then(|r| Table::text(r, text))
                .and_then(Wrapper::new);
        }
        Ok(out)
    }

    /// What was read, for the summary.
    #[must_use]
    pub fn inputs(&self) -> Inputs {
        Inputs {
            wrapper: self.wrapper.is_some(),
            levels: self.levels,
            conflicts: self.conflicts.clone(),
        }
    }

    /// One graft base with the standard wrapper and levels 1..30 (tests of
    /// other modules).
    #[cfg(test)]
    #[must_use]
    pub fn with_one(base: &str, id: &str, skill: &str) -> Self {
        Self {
            by_base: [(
                base.to_owned(),
                Graft {
                    id: id.to_owned(),
                    skill: skill.to_owned(),
                },
            )]
            .into(),
            wrapper: Wrapper::new("Uses level {1} {0}"),
            levels: Some((1, 30)),
            conflicts: Vec::new(),
        }
    }

    /// Number of graft bases read.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_base.len()
    }

    /// The granted skill a line of an item with base `base` shows (module
    /// docs).
    #[must_use]
    pub fn resolve(&self, base: &str, line: &str) -> Option<GrantedSkill> {
        let graft = self.by_base.get(base)?;
        let (level, name) = self.wrapper.as_ref()?.read(line)?;
        let (lo, hi) = self.levels?;
        (name == graft.skill && (lo..=hi).contains(&level)).then(|| GrantedSkill {
            graft: graft.clone(),
            level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grafts() -> Grafts {
        Grafts {
            by_base: [(
                "Aegis Tulgraft".to_owned(),
                Graft {
                    id: "TulGraft4".to_owned(),
                    skill: "Preserving Stillness".to_owned(),
                },
            )]
            .into(),
            wrapper: Wrapper::new("Uses level {1} {0}"),
            levels: Some((1, 30)),
            conflicts: Vec::new(),
        }
    }

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
        let g = Grafts::load(&read).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(
            g.inputs(),
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
        let g = Grafts::load(&none).unwrap();
        assert_eq!((g.len(), g.inputs()), (0, Inputs::default()));
    }

    #[test]
    fn resolve_should_read_the_graft_skill_and_level() {
        let g = grafts();
        let r = g
            .resolve("Aegis Tulgraft", "Uses level 30 Preserving Stillness")
            .unwrap();
        assert_eq!((r.graft.id.as_str(), r.level), ("TulGraft4", 30));
    }

    #[test]
    fn resolve_should_refuse_ranges_other_names_bases_and_levels() {
        let g = grafts();
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

    #[test]
    fn wrapper_should_need_both_placeholders() {
        assert!(Wrapper::new("Uses level {1} {0}").is_some());
        assert!(Wrapper::new("Uses {0}").is_none());
    }
}

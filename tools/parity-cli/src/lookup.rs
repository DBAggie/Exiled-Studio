//! A [`Lookup`] over the extract's tables (Spike B, PR B2b).
//!
//! Key forms, with their evidence (Spike B context file, "PR B2b"):
//! - `display_indexable_support`, `display_indexable_non_active_support`,
//!   `display_indexable_skill`: the table's `Index` column. Mods rows give
//!   these stats the ranges `1-172`, `1-164`, and `1-287`, which are the
//!   tables' `Index` spans and row counts (PoE1 26530219), so a 0-based
//!   row index is refuted. `Index` equals row + 1 in every row of the
//!   three tables, so the data cannot tell `Index` from row + 1.
//! - `display_indexable_skill` writes `Name1` (hypothesis: the table also
//!   has `Name2`, which differs in some rows; no evidence picks one).
//! - `passive_hash`: `PassiveSkills.PassiveSkillGraphId` (the passive
//!   tree's node hash, Spike A section 16) to `Name`. Hypothesis from the
//!   handler name; PoB's export prints the raw value as an unsigned 16-bit
//!   number (`v2.67.2:src/Export/statdesc.lua` `describeStats` adds 65536
//!   to a negative value), which fits a 16-bit hash. Negative stat values
//!   are not wrapped here; no Mods row has a fixed value to test it.
//!
//! - `tree_expansion_jewel_passive` (PR B4c): a cluster jewel's small
//!   passive. The name is the game's own text: the passive's stats
//!   (`PassiveTreeExpansionSkills.PassiveSkillsKey` to `PassiveSkills`
//!   `Stats` and `Stat1Value`..`Stat5Value`) rendered through
//!   `passive_skill_stat_descriptions.txt`, each line put into `ClientStrings`
//!   `StatDescripotionTreeExpansionJewelGrantedSmallStat` (`Added Small
//!   Passive Skills grant: {0}`), lines joined by a line break. PoB's export
//!   writes the same lines (`v2.67.2:src/Export/Scripts/cluster.lua`,
//!   `enchant`). The key is the `PassiveTreeExpansionSkills` row index
//!   (hypothesis: every Mods row gives the stat 0, no table names the key,
//!   and PoB's export numbers the rows from 1 only in commented-out code; no
//!   evidence picks 0- or 1-based). The stat ID a line resolves to does not
//!   depend on the key form.
//!
//! Not provided, so their variants are counted as `lookup_unavailable`:
//! `mod_value_to_item_class` (Mods values do not follow `ItemClasses` row
//! order: amulets 0 is row 4, belts 15 is row 21), and every handler whose
//! table is not extracted (Spike B plan, lookup table).

use std::collections::BTreeMap;

use loom_data::stat_desc::{Lookup, LookupHandler, NoLookup, StatValue, VariantFilter};
use serde::Serialize;

use crate::extract::{Extract, Read, Table};

/// The stat-description file the game describes passive stats with.
const PASSIVE_DESCRIPTIONS: &str = "passive_skill_stat_descriptions.txt";

/// `ClientStrings.Id` of the small-passive line template (the game's
/// spelling).
const SMALL_PASSIVE_STRING: &str = "StatDescripotionTreeExpansionJewelGrantedSmallStat";

/// `PassiveSkills` stat value columns, in `Stats` order.
const PASSIVE_STAT_VALUES: [&str; 5] = [
    "Stat1Value",
    "Stat2Value",
    "Stat3Value",
    "Stat4Value",
    "Stat5Value",
];

/// Names by key, per handler.
#[derive(Debug, Default)]
pub struct TableLookup {
    names: BTreeMap<LookupHandler, BTreeMap<i64, String>>,
}

/// What each handler's table gave, for reports.
#[derive(Debug, Default, Serialize)]
pub struct LookupReport {
    /// Handler name to the number of keys with a name (0 for an extracted
    /// table without rows).
    pub keys: BTreeMap<&'static str, usize>,
    /// Handler name to rows whose key an earlier row already has (the
    /// first row's name is kept), when non-zero.
    pub duplicate_keys: BTreeMap<&'static str, usize>,
    /// Handler name to rows without an integer key or a name, when
    /// non-zero.
    pub skipped_rows: BTreeMap<&'static str, usize>,
    /// Handler name to why it has no names (table not in the extract, or
    /// not supported, with the reason).
    pub unavailable: BTreeMap<&'static str, String>,
}

/// (handler, table, key column, name column).
const SOURCES: [(LookupHandler, &str, &str, &str); 4] = [
    (
        LookupHandler::DisplayIndexableSupport,
        "IndexableSupportGems",
        "Index",
        "Name",
    ),
    (
        LookupHandler::DisplayIndexableNonActiveSupport,
        "IndexableNonActiveSupportGems",
        "Index",
        "Name",
    ),
    (
        LookupHandler::DisplayIndexableSkill,
        "IndexableSkillGems",
        "Index",
        "Name1",
    ),
    (
        LookupHandler::PassiveHash,
        "PassiveSkills",
        "PassiveSkillGraphId",
        "Name",
    ),
];

/// `tree_expansion_jewel_passive` names: `Ok((names, skipped rows))`, or
/// `Err(reason)` inside when a table, the template, or the description file
/// is missing. A row whose passive renders no line, or leaves a stat
/// unrendered, is counted in `skipped rows`, not named.
type TreeNames = Result<(BTreeMap<i64, String>, usize), String>;

/// Builds the `tree_expansion_jewel_passive` names (module docs).
///
/// # Errors
/// A message when a table exists but lacks a column, or the description
/// file does not build a scope.
fn tree_expansion_names(read: Read<'_>, extract: &Extract) -> Result<TreeNames, String> {
    let mut tables = Vec::new();
    for name in [
        "PassiveTreeExpansionSkills",
        "PassiveSkills",
        "Stats",
        "ClientStrings",
    ] {
        match Table::read(read, name)? {
            Some(t) => tables.push(t),
            None => return Ok(Err(format!("{name} is not in the extract"))),
        }
    }
    let [skills, passives, stats, strings] = tables.as_slice() else {
        return Ok(Err("tables not read".to_owned()));
    };
    let (id, text) = (strings.column("Id")?, strings.column("Text")?);
    let Some(template) = strings
        .rows
        .iter()
        .find(|r| Table::text(r, id) == Some(SMALL_PASSIVE_STRING))
        .and_then(|r| Table::text(r, text))
        .filter(|t| t.contains("{0}"))
    else {
        return Ok(Err(format!(
            "ClientStrings has no {SMALL_PASSIVE_STRING} with a {{0}}"
        )));
    };
    let Some(path) = extract.files.iter().find(|p| {
        p.rsplit('/')
            .next()
            .is_some_and(|f| f.eq_ignore_ascii_case(PASSIVE_DESCRIPTIONS))
    }) else {
        return Ok(Err(format!("{PASSIVE_DESCRIPTIONS} is not in the extract")));
    };
    let scope = extract
        .set
        .scope(path)
        .map_err(|e| format!("{path}: {} ({e})", e.code()))?;
    let stat_ids = stats.column("Id")?;
    let (node, node_stats) = (
        skills.column("PassiveSkillsKey")?,
        passives.column("Stats")?,
    );
    let values = PASSIVE_STAT_VALUES
        .iter()
        .map(|c| passives.column(c))
        .collect::<Result<Vec<_>, _>>()?;
    let (mut names, mut skipped) = (BTreeMap::new(), 0);
    for (key, row) in skills.rows.iter().enumerate() {
        let passive = Table::int(row, node)
            .and_then(|n| usize::try_from(n).ok())
            .and_then(|n| passives.rows.get(n));
        let given: Option<BTreeMap<String, StatValue>> = passive.and_then(|p| {
            p.get(node_stats)?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let stat = usize::try_from(k.as_i64()?).ok()?;
                    let id = Table::text(stats.rows.get(stat)?, stat_ids)?;
                    let value = Table::int(p, *values.get(i)?)?;
                    Some((id.to_owned(), StatValue::exact(value)))
                })
                .collect()
        });
        let rendered = given.map(|g| scope.render(&g, &NoLookup, VariantFilter::default()));
        match (rendered, i64::try_from(key)) {
            (Some(r), Ok(key)) if r.not_rendered.is_empty() && !r.lines.is_empty() => {
                let lines: Vec<String> = r
                    .lines
                    .iter()
                    .map(|l| template.replace("{0}", &l.text))
                    .collect();
                names.insert(key, lines.join("\n"));
            }
            _ => skipped += 1,
        }
    }
    Ok(Ok((names, skipped)))
}

impl TableLookup {
    /// Builds the lookup from the extract's tables (and, for
    /// `tree_expansion_jewel_passive`, its stat descriptions).
    ///
    /// # Errors
    /// A message when a table exists but lacks a column or does not parse,
    /// or the passive description file does not build a scope.
    pub fn load(read: Read<'_>, extract: &Extract) -> Result<(Self, LookupReport), String> {
        let mut lookup = Self::default();
        let mut report = LookupReport::default();
        for (handler, table, key, name) in SOURCES {
            match Table::read(read, table)? {
                Some(t) => {
                    let mapped = t.map(key, name)?;
                    report.keys.insert(handler.name(), mapped.texts.len());
                    if mapped.duplicate_keys > 0 {
                        report
                            .duplicate_keys
                            .insert(handler.name(), mapped.duplicate_keys);
                    }
                    if mapped.skipped_rows > 0 {
                        report
                            .skipped_rows
                            .insert(handler.name(), mapped.skipped_rows);
                    }
                    lookup.names.insert(handler, mapped.texts);
                }
                None => {
                    report
                        .unavailable
                        .insert(handler.name(), format!("{table} is not in the extract"));
                }
            }
        }
        let tree = LookupHandler::TreeExpansionJewelPassive;
        match tree_expansion_names(read, extract)? {
            Ok((names, skipped)) => {
                report.keys.insert(tree.name(), names.len());
                if skipped > 0 {
                    report.skipped_rows.insert(tree.name(), skipped);
                }
                lookup.names.insert(tree, names);
            }
            Err(reason) => {
                report.unavailable.insert(tree.name(), reason);
            }
        }
        for handler in LookupHandler::ALL {
            if handler != tree && !SOURCES.iter().any(|(h, ..)| *h == handler) {
                report.unavailable.insert(
                    handler.name(),
                    "no supported key form (see parity-cli lookup.rs)".to_owned(),
                );
            }
        }
        Ok((lookup, report))
    }
}

impl Lookup for TableLookup {
    fn name(&self, handler: LookupHandler, value: i64) -> Option<String> {
        self.names.get(&handler)?.get(&value).cloned()
    }

    fn entries(&self, handler: LookupHandler) -> Vec<(i64, String)> {
        self.names
            .get(&handler)
            .map(|m| m.iter().map(|(k, n)| (*k, n.clone())).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::tests::table_json;

    fn empty_extract() -> Extract {
        Extract {
            inputs: crate::extract::Inputs::default(),
            files: Vec::new(),
            set: loom_data::stat_desc::StatDescriptions::new(),
        }
    }

    const MANIFEST: &str = r#"{"format": 3, "game": "poe1", "steam_build_id": "1",
        "stat_descriptions": {"files": {
          "metadata/statdescriptions/stat_descriptions.txt": {"kind": "descriptions",
            "status": "ok", "output": "item.json", "descriptions": 1, "english_variants": 1},
          "metadata/statdescriptions/passive_skill_stat_descriptions.txt": {"kind": "descriptions",
            "status": "ok", "output": "passive.json", "descriptions": 2, "english_variants": 2}},
        "handlers": {}}}"#;

    fn desc(line: u32, stat: &str, text: &str, handlers: &str) -> String {
        format!(
            r#"{{"handed":false,"line":{line},"name":null,"off_hand_stats":null,"stats":["{stat}"],"variants":[
              {{"conditions":[{{"kind":"range","max":null,"min":null}}],"flags":[],"handlers":[{handlers}],"line":{line},
               "placeholders":[{{"format":"","index":0}}],"text":"{text}"}}]}}"#
        )
    }

    fn file(source: &str, descriptions: &[String]) -> String {
        format!(
            r#"{{"descriptions": [{}], "file_flags": [], "includes": [], "languages": {{}},
            "no_description": [], "source": "metadata/statdescriptions/{source}"}}"#,
            descriptions.join(",")
        )
    }

    /// `rows` of a table whose columns are `columns` (JSON cell text).
    fn table(name: &str, columns: &[&str], rows: &[&[&str]]) -> String {
        let columns: Vec<String> = columns
            .iter()
            .map(|c| format!(r#"{{"name": "{c}", "references": null}}"#))
            .collect();
        let rows: Vec<String> = rows.iter().map(|r| format!("[{}]", r.join(", "))).collect();
        format!(
            r#"{{"table": "{name}", "columns": [{}], "rows": [{}]}}"#,
            columns.join(", "),
            rows.join(", ")
        )
    }

    /// Small passives: row 0 one stat, row 1 two stats, row 2 a stat with
    /// no description (skipped).
    fn cluster_read(path: &str, template: bool) -> Result<Vec<u8>, String> {
        let text = match path {
            "manifest.json" => MANIFEST.to_owned(),
            "item.json" => file(
                "stat_descriptions.txt",
                &[desc(
                    1,
                    "local_jewel_expansion_passive_node_index",
                    "{0}",
                    r#"{"name": "tree_expansion_jewel_passive", "stat": 1}"#,
                )],
            ),
            "passive.json" => file(
                "passive_skill_stat_descriptions.txt",
                &[
                    desc(1, "fire", "{0}% increased Fire Damage", ""),
                    desc(2, "cold", "{0}% increased Cold Damage", ""),
                ],
            ),
            "tables/PassiveTreeExpansionSkills.json" => table(
                "PassiveTreeExpansionSkills",
                &["PassiveSkillsKey"],
                &[&["0"], &["1"], &["2"]],
            ),
            "tables/PassiveSkills.json" => table(
                "PassiveSkills",
                &[
                    "Stats",
                    "Stat1Value",
                    "Stat2Value",
                    "Stat3Value",
                    "Stat4Value",
                    "Stat5Value",
                    "PassiveSkillGraphId",
                    "Name",
                ],
                &[
                    &["[0]", "12", "0", "0", "0", "0", "100", r#""A""#],
                    &["[0, 1]", "10", "8", "0", "0", "0", "101", r#""B""#],
                    &["[2]", "5", "0", "0", "0", "0", "102", r#""C""#],
                ],
            ),
            "tables/Stats.json" => table(
                "Stats",
                &["Id"],
                &[&[r#""fire""#], &[r#""cold""#], &[r#""none""#]],
            ),
            "tables/ClientStrings.json" if template => table(
                "ClientStrings",
                &["Id", "Text"],
                &[&[
                    r#""StatDescripotionTreeExpansionJewelGrantedSmallStat""#,
                    r#""Added Small Passive Skills grant: {0}""#,
                ]],
            ),
            _ => return Err(format!("{path}: not found")),
        };
        Ok(text.into_bytes())
    }

    #[test]
    fn tree_expansion_names_should_render_the_passive_stats_into_the_game_template() {
        let read = |p: &str| cluster_read(p, true);
        let extract = crate::extract::load(&read).unwrap();
        let (lookup, report) = TableLookup::load(&read, &extract).unwrap();
        let h = LookupHandler::TreeExpansionJewelPassive;
        assert_eq!(
            lookup.name(h, 0).as_deref(),
            Some("Added Small Passive Skills grant: 12% increased Fire Damage")
        );
        assert_eq!(
            lookup.name(h, 1).as_deref(),
            Some(
                "Added Small Passive Skills grant: 10% increased Fire Damage\n\
                 Added Small Passive Skills grant: 8% increased Cold Damage"
            )
        );
        assert_eq!(lookup.name(h, 2), None);
        assert_eq!(
            report.skipped_rows.get("tree_expansion_jewel_passive"),
            Some(&1)
        );
    }

    #[test]
    fn tree_expansion_names_should_be_unavailable_without_the_template() {
        let read = |p: &str| cluster_read(p, false);
        let extract = crate::extract::load(&read).unwrap();
        let (_, report) = TableLookup::load(&read, &extract).unwrap();
        assert!(
            report
                .unavailable
                .contains_key("tree_expansion_jewel_passive")
        );
    }

    #[test]
    fn a_small_passive_line_should_resolve_to_the_node_index_stat() {
        let read = |p: &str| cluster_read(p, true);
        let extract = crate::extract::load(&read).unwrap();
        let (lookup, _) = TableLookup::load(&read, &extract).unwrap();
        let path = extract
            .files
            .iter()
            .find(|f| f.ends_with("/stat_descriptions.txt"))
            .unwrap();
        let scope = extract.set.scope(path).unwrap();
        let index = loom_parser::resolve::Index::build_with(&scope, &lookup);
        let lines = [
            "Added Small Passive Skills grant: 10% increased Fire Damage",
            "Added Small Passive Skills grant: 8% increased Cold Damage",
        ];
        let resolved = index.resolve_lines(&lines);
        let [one] = resolved.as_slice() else {
            panic!("{resolved:?}");
        };
        let loom_parser::resolve::Outcome::Resolved(c) = &one.outcome else {
            panic!("{:?}", one.outcome);
        };
        assert_eq!(
            (one.lines, c.stats[0].stat.as_str()),
            (2, "local_jewel_expansion_passive_node_index")
        );
        assert_eq!(
            c.stats[0].value,
            loom_parser::resolve::RawValue::Lookup(vec![1])
        );
    }

    #[test]
    fn load_should_read_present_tables_and_report_missing_ones() {
        let support = table_json("IndexableSupportGems", &[(1, "Fork"), (2, "Pierce")]);
        let read = |p: &str| {
            (p == "tables/IndexableSupportGems.json")
                .then(|| support.clone().into_bytes())
                .ok_or_else(|| "missing".to_owned())
        };
        let (lookup, report) = TableLookup::load(&read, &empty_extract()).unwrap();
        let h = LookupHandler::DisplayIndexableSupport;
        assert_eq!(lookup.name(h, 2).as_deref(), Some("Pierce"));
        assert_eq!(lookup.name(h, 3), None);
        assert_eq!(lookup.entries(h).len(), 2);
        assert_eq!(report.keys.get("display_indexable_support"), Some(&2));
        assert!(report.unavailable.contains_key("passive_hash"));
        assert!(report.unavailable.contains_key("mod_value_to_item_class"));
        assert_eq!(
            report.keys.len() + report.unavailable.len(),
            LookupHandler::ALL.len()
        );
    }
}

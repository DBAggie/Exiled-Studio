//! The cluster jewels for the PoE1 bundle (passive tree plan, T-i):
//! `PassiveTreeExpansionJewelSizes`, `...Jewels`, `...Skills`,
//! `...SpecialSkills`, and `PassiveJewelSlots` to
//! `poe1/cluster_jewels.json` (`loom_data::bundle::poe1::ClusterJewels`),
//! as PoB's `Export/Scripts/cluster.lua` reads them (v2.67.2).
//!
//! - **Passives** are their `PassiveSkillGraphId`s, as the passives table's;
//!   bases their `BaseItemTypes.Id`; tags their `Tags.Id`; a special's stat
//!   its `Stats.Id`, and its order the stat's row (PoB's
//!   `notableSortOrder`); a special is a keystone when its passive is.
//! - **Slots** are the jewel slots with a cluster size; a slot's parent is
//!   the socket of the slot it replaces (`ReplacesSlot`).
//! - **Grants** pair each "Added Small Passive Skills also grant: ..."
//!   stat (`local_affliction_jewel_small_nodes_grant_*`) with the global
//!   stat whose description reads as the rest of its line ([`grants`]).
//!
//! `None` when the extract lacks one of the tables (an extract made before
//! T3 has no `PassiveTreeExpansionSpecialSkills`): the bundle then has no
//! cluster jewels and their nodes stay a reason. A key past its table, or
//! a value of the wrong type, fails the conversion: the table would be
//! misread.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::poe1::{
    ClusterGrant, ClusterJewelBase, ClusterJewels, ClusterSkill, ClusterSlot, ClusterSpecial,
};
use loom_data::stat_desc::{Handler, StatDescFile};
use serde_json::Value;

use crate::extract::{Read, Table};

/// The prefix of every "also grant" stat.
const GRANT_PREFIX: &str = "local_affliction_jewel_small_nodes_grant_";
/// The text an "also grant" line starts with.
const GRANT_TEXT: &str = "Added Small Passive Skills also grant: ";

/// "Also grant" stats whose line the global stat's description words
/// apart, with the stat each grants: PoB reads the rest of the line as
/// the same modifier (`ModParser.lua`, "Added Small Passive Skills also
/// grant"), checked by `data-parity`'s `cluster_grants`.
const GRANT_PAIRS: &[(&str, &str)] = &[
    // "+# to Critical Strike Multiplier"; the global line says "Global".
    (
        "local_affliction_jewel_small_nodes_grant_base_critical_strike_multiplier_+",
        "base_critical_strike_multiplier_+",
    ),
    // "+# to Evasion"; the global line says "Evasion Rating".
    (
        "local_affliction_jewel_small_nodes_grant_evasion",
        "base_evasion_rating",
    ),
    // "Minions Regenerate #% of Life per Second": the same per-minute
    // value, shown with two decimals where the global line shows one.
    (
        "local_affliction_jewel_small_nodes_grant_minion_life_regeneration_rate_per_minute_%",
        "minion_life_regeneration_rate_per_minute_%",
    ),
];

fn ints(row: &[Value], col: usize) -> Option<Vec<i64>> {
    match row.get(col)? {
        Value::Null => Some(Vec::new()),
        Value::Array(a) => a.iter().map(Value::as_i64).collect(),
        _ => None,
    }
}

fn indices(row: &[Value], col: usize, what: &str) -> Result<Vec<u32>, String> {
    ints(row, col)
        .and_then(|v| v.into_iter().map(|x| u32::try_from(x).ok()).collect())
        .ok_or_else(|| format!("{what}: not a list of indices"))
}

/// A key column's row, if the cell holds one (`null` is none).
fn key(row: &[Value], col: usize) -> Option<usize> {
    Table::int(row, col).and_then(|k| usize::try_from(k).ok())
}

/// `rows[k]`'s text in `col`, else an error naming `what`.
fn text_at(t: &Table, col: usize, k: usize, what: &str) -> Result<String, String> {
    t.rows
        .get(k)
        .and_then(|r| Table::text(r, col))
        .map(str::to_owned)
        .ok_or_else(|| format!("{what}: key {k} is not a {} row", t.table))
}

/// The passives table's graph IDs and flags, by row.
struct Passives {
    nodes: Vec<Option<i64>>,
    keystone: Vec<bool>,
}

impl Passives {
    fn read(read: Read<'_>) -> Result<Self, String> {
        let t = Table::require(read, "PassiveSkills")?;
        let (node, keystone) = (t.column("PassiveSkillGraphId")?, t.column("IsKeystone")?);
        Ok(Self {
            nodes: t.rows.iter().map(|r| Table::int(r, node)).collect(),
            keystone: t
                .rows
                .iter()
                .map(|r| r.get(keystone).and_then(Value::as_bool).unwrap_or(false))
                .collect(),
        })
    }

    fn node(&self, k: Option<usize>, what: &str) -> Result<i64, String> {
        k.and_then(|k| self.nodes.get(k).copied().flatten())
            .ok_or_else(|| format!("{what}: no passive"))
    }
}

/// The cluster jewels (module docs), with the "also grant" pairs from
/// `descriptions`.
///
/// # Errors
/// A column missing, a key past its table, or a value of the wrong type.
pub fn rows<'f>(
    read: Read<'_>,
    descriptions: impl Iterator<Item = &'f StatDescFile>,
) -> Result<Option<ClusterJewels>, String> {
    let tables = [
        "PassiveTreeExpansionJewelSizes",
        "PassiveTreeExpansionJewels",
        "PassiveTreeExpansionSkills",
        "PassiveTreeExpansionSpecialSkills",
        "PassiveJewelSlots",
    ];
    let mut t = Vec::new();
    for name in tables {
        match Table::read(read, name)? {
            Some(x) => t.push(x),
            None => return Ok(None),
        }
    }
    let [sizes_t, jewels_t, skills_t, specials_t, slots_t] =
        <[Table; 5]>::try_from(t).map_err(|_| "five tables".to_owned())?;
    let passives = Passives::read(read)?;
    let stats_t = Table::require(read, "Stats")?;
    let stat_id = stats_t.column("Id")?;
    let bases_t = Table::require(read, "BaseItemTypes")?;
    let base_id = bases_t.column("Id")?;
    let tags_t = Table::require(read, "Tags")?;
    let tag_id = tags_t.column("Id")?;

    let name = sizes_t.column("Name")?;
    let sizes: Vec<String> = sizes_t
        .rows
        .iter()
        .map(|r| Table::text(r, name).unwrap_or_default().to_owned())
        .collect();
    let size_of = |r: &[Value], col: usize, what: &str| {
        key(r, col)
            .filter(|k| *k < sizes.len())
            .and_then(|k| u32::try_from(k).ok())
            .ok_or_else(|| format!("{what}: no jewel size"))
    };

    let c = |t: &Table, n: &str| t.column(n);
    let mut jewels = Vec::new();
    for (i, r) in jewels_t.rows.iter().enumerate() {
        let what = format!("PassiveTreeExpansionJewels {i}");
        let base =
            key(r, c(&jewels_t, "BaseItemTypesKey")?).ok_or_else(|| format!("{what}: no base"))?;
        let int = |n: &str| {
            c(&jewels_t, n).and_then(|col| {
                Table::int(r, col)
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or_else(|| format!("{what}: {n} is not a count"))
            })
        };
        jewels.push(ClusterJewelBase {
            base: text_at(&bases_t, base_id, base, &what)?,
            size: size_of(r, c(&jewels_t, "PassiveTreeExpansionJewelSizesKey")?, &what)?,
            min_nodes: int("MinNodes")?,
            max_nodes: int("MaxNodes")?,
            small_indices: indices(r, c(&jewels_t, "SmallIndices")?, &what)?,
            notable_indices: indices(r, c(&jewels_t, "NotableIndices")?, &what)?,
            socket_indices: indices(r, c(&jewels_t, "SocketIndices")?, &what)?,
            total_indices: int("TotalIndices")?,
        });
    }

    let mut skills = Vec::new();
    for (i, r) in skills_t.rows.iter().enumerate() {
        let what = format!("PassiveTreeExpansionSkills {i}");
        let tag = key(r, c(&skills_t, "TagsKey")?).ok_or_else(|| format!("{what}: no tag"))?;
        skills.push(ClusterSkill {
            node: passives.node(key(r, c(&skills_t, "PassiveSkillsKey")?), &what)?,
            mastery: match key(r, c(&skills_t, "Mastery_PassiveSkillsKey")?) {
                Some(k) => Some(passives.node(Some(k), &what)?),
                None => None,
            },
            tag: text_at(&tags_t, tag_id, tag, &what)?,
            size: size_of(r, c(&skills_t, "PassiveTreeExpansionJewelSizesKey")?, &what)?,
        });
    }

    let mut specials = Vec::new();
    for (i, r) in specials_t.rows.iter().enumerate() {
        let what = format!("PassiveTreeExpansionSpecialSkills {i}");
        let passive = key(r, c(&specials_t, "PassiveSkillsKey")?);
        let stat = key(r, c(&specials_t, "StatsKey")?).ok_or_else(|| format!("{what}: no stat"))?;
        specials.push(ClusterSpecial {
            node: passives.node(passive, &what)?,
            stat: text_at(&stats_t, stat_id, stat, &what)?,
            order: i64::try_from(stat).map_err(|e| e.to_string())?,
            keystone: passive.is_some_and(|k| passives.keystone.get(k) == Some(&true)),
        });
    }

    let slot_node = c(&slots_t, "Slot")?;
    let mut slots = Vec::new();
    for (i, r) in slots_t.rows.iter().enumerate() {
        let what = format!("PassiveJewelSlots {i}");
        let Some(size) = key(r, c(&slots_t, "ClusterJewelSize")?) else {
            continue;
        };
        let size = u32::try_from(size)
            .ok()
            .filter(|s| (*s as usize) < sizes.len())
            .ok_or_else(|| format!("{what}: no jewel size"))?;
        let parent = match key(r, c(&slots_t, "ReplacesSlot")?) {
            Some(k) => {
                Some(passives.node(slots_t.rows.get(k).and_then(|p| key(p, slot_node)), &what)?)
            }
            None => None,
        };
        slots.push(ClusterSlot {
            socket: passives.node(key(r, slot_node), &what)?,
            size,
            index: Table::int(r, c(&slots_t, "ClusterIndex")?)
                .ok_or_else(|| format!("{what}: no cluster index"))?,
            parent,
            proxy: passives.node(key(r, c(&slots_t, "ProxySlot")?), &what)?,
            start_indices: indices(r, c(&slots_t, "StartIndices")?, &what)?,
        });
    }

    let known: BTreeSet<&str> = stats_t
        .rows
        .iter()
        .filter_map(|r| Table::text(r, stat_id))
        .collect();
    Ok(Some(ClusterJewels {
        sizes,
        jewels,
        skills,
        specials,
        slots,
        grants: grants(descriptions, &known),
    }))
}

/// A variant's handlers that change its value (reminders only add text).
fn value_handlers(handlers: &[Handler]) -> Vec<&Handler> {
    handlers
        .iter()
        .filter(|h| !matches!(h, Handler::Reminder(_)))
        .collect()
}

/// Each "also grant" stat with the stat it grants (module docs): the one
/// non-local single-stat description, in the same file, with a variant
/// whose text is the rest of the "also grant" variant's line (any case)
/// and whose handlers, reminders aside, are the same, for every variant;
/// else [`GRANT_PAIRS`]' pair, when both stats are in `known`. A stat
/// with neither is left out (a jewel with it stays unbuilt).
pub fn grants<'f>(
    files: impl Iterator<Item = &'f StatDescFile>,
    known: &BTreeSet<&str>,
) -> Vec<ClusterGrant> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for file in files {
        let globals: Vec<_> = file
            .descriptions
            .iter()
            .filter(|d| d.stats.len() == 1 && !d.stats[0].starts_with("local_"))
            .collect();
        for d in &file.descriptions {
            let [stat] = d.stats.as_slice() else { continue };
            if !stat.starts_with(GRANT_PREFIX) || out.contains_key(stat) {
                continue;
            }
            let mut found: Option<BTreeSet<&str>> = None;
            for v in &d.variants {
                let Some(rest) = v.text.strip_prefix(GRANT_TEXT) else {
                    found = Some(BTreeSet::new());
                    break;
                };
                let these: BTreeSet<&str> = globals
                    .iter()
                    .filter(|g| {
                        g.variants.iter().any(|gv| {
                            gv.text.eq_ignore_ascii_case(rest)
                                && value_handlers(&gv.handlers) == value_handlers(&v.handlers)
                        })
                    })
                    .map(|g| g.stats[0].as_str())
                    .collect();
                found = Some(match found {
                    None => these,
                    Some(f) => f.intersection(&these).copied().collect(),
                });
            }
            if let Some(f) = found
                && let [granted] = f.into_iter().collect::<Vec<_>>().as_slice()
            {
                out.insert(stat.clone(), (*granted).to_owned());
            }
        }
    }
    for (stat, granted) in GRANT_PAIRS {
        if known.contains(stat) && known.contains(granted) {
            out.entry((*stat).to_owned())
                .or_insert_with(|| (*granted).to_owned());
        }
    }
    out.into_iter()
        .map(|(stat, granted)| ClusterGrant { stat, granted })
        .collect()
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
                "PassiveSkills",
                &["Id", "PassiveSkillGraphId", "IsKeystone"],
                &json!([
                    ["small", 100, false],
                    ["mastery", 101, false],
                    ["notable", 102, false],
                    ["keystone", 103, true],
                    ["socket", 200, false],
                    ["proxy", 201, false],
                    ["nested", 202, false],
                    ["nested_proxy", 203, false]
                ]),
            ),
            table(
                "Stats",
                &["Id"],
                &json!([
                    ["x"],
                    ["local_affliction_notable_n"],
                    ["local_jewel_expansion_keystone_k"]
                ]),
            ),
            table(
                "BaseItemTypes",
                &["Id"],
                &json!([["Jewel/Small"], ["Jewel/Large"]]),
            ),
            table("Tags", &["Id"], &json!([["fire"]])),
            table(
                "PassiveTreeExpansionJewelSizes",
                &["Name"],
                &json!([["Small"], ["Medium"], ["Large"]]),
            ),
            table(
                "PassiveTreeExpansionJewels",
                &[
                    "BaseItemTypesKey",
                    "PassiveTreeExpansionJewelSizesKey",
                    "MinNodes",
                    "MaxNodes",
                    "SmallIndices",
                    "NotableIndices",
                    "SocketIndices",
                    "TotalIndices",
                ],
                &json!([[1, 2, 8, 12, [0, 2], [6, 10], [4, 8], 12]]),
            ),
            table(
                "PassiveTreeExpansionSkills",
                &[
                    "PassiveSkillsKey",
                    "Mastery_PassiveSkillsKey",
                    "TagsKey",
                    "PassiveTreeExpansionJewelSizesKey",
                ],
                &json!([[0, 1, 0, 2], [0, null, 0, 0]]),
            ),
            table(
                "PassiveTreeExpansionSpecialSkills",
                &["PassiveSkillsKey", "StatsKey"],
                &json!([[2, 1], [3, 2]]),
            ),
            table(
                "PassiveJewelSlots",
                &[
                    "Slot",
                    "ClusterJewelSize",
                    "ClusterIndex",
                    "ReplacesSlot",
                    "ProxySlot",
                    "StartIndices",
                ],
                &json!([
                    [4, 2, 0, null, 5, [3, 5, 5]],
                    [6, 1, 1, 0, 7, [1, 1]],
                    [5, null, 0, null, null, []]
                ]),
            ),
        ]
        .into_iter()
        .collect()
    }

    fn run(files: &BTreeMap<String, Vec<u8>>) -> Result<Option<ClusterJewels>, String> {
        let read = |p: &str| files.get(p).cloned().ok_or_else(|| format!("{p}: missing"));
        rows(&read, std::iter::empty())
    }

    #[test]
    fn rows_should_read_the_tables_as_pob_does() {
        let c = run(&files()).unwrap().unwrap();
        assert_eq!(c.sizes, ["Small", "Medium", "Large"]);
        assert_eq!(
            c.jewels,
            [ClusterJewelBase {
                base: "Jewel/Large".to_owned(),
                size: 2,
                min_nodes: 8,
                max_nodes: 12,
                small_indices: vec![0, 2],
                notable_indices: vec![6, 10],
                socket_indices: vec![4, 8],
                total_indices: 12,
            }]
        );
        assert_eq!(c.skills[0].mastery, Some(101));
        assert_eq!(c.skills[1].mastery, None);
        assert_eq!((c.skills[0].node, c.skills[0].tag.as_str()), (100, "fire"));
        assert_eq!(
            c.specials,
            [
                ClusterSpecial {
                    node: 102,
                    stat: "local_affliction_notable_n".to_owned(),
                    order: 1,
                    keystone: false,
                },
                ClusterSpecial {
                    node: 103,
                    stat: "local_jewel_expansion_keystone_k".to_owned(),
                    order: 2,
                    keystone: true,
                },
            ]
        );
        // The slot without a cluster size is left out; the nested one's
        // parent is the slot it replaces.
        assert_eq!(c.slots.len(), 2);
        assert_eq!(
            c.slots[1],
            ClusterSlot {
                socket: 202,
                size: 1,
                index: 1,
                parent: Some(200),
                proxy: 203,
                start_indices: vec![1, 1],
            }
        );
    }

    #[test]
    fn rows_should_be_none_without_a_table_and_fail_on_a_bad_key() {
        let mut f = files();
        f.remove("tables/PassiveTreeExpansionSpecialSkills.json");
        assert_eq!(run(&f).unwrap(), None);
        let mut f = files();
        let (k, v) = table(
            "PassiveTreeExpansionSpecialSkills",
            &["PassiveSkillsKey", "StatsKey"],
            &json!([[2, 9]]),
        );
        f.insert(k, v);
        assert!(run(&f).unwrap_err().contains("not a Stats row"));
    }

    fn descriptions(entries: &Value) -> StatDescFile {
        let body = json!({
            "descriptions": entries,
            "file_flags": [],
            "includes": [],
            "languages": {},
            "no_description": [],
            "source": "Metadata/StatDescriptions/stat_descriptions.txt",
        });
        StatDescFile::from_json(body.to_string().as_bytes()).unwrap()
    }

    fn description(line: usize, stat: &str, text: &str, handlers: &Value) -> Value {
        json!({
            "handed": false,
            "line": line,
            "name": null,
            "off_hand_stats": null,
            "stats": [stat],
            "variants": [{
                "conditions": [{"kind": "range", "max": null, "min": null}],
                "flags": [],
                "handlers": handlers,
                "line": line + 1,
                "placeholders": [{"format": "+d", "index": 0}],
                "text": text,
            }],
        })
    }

    #[test]
    fn grants_should_pair_by_the_rest_of_the_line_and_the_listed_pairs() {
        let reminder = json!([{"name": "reminderstring", "reminder": "R"}]);
        let file = descriptions(&json!([
            description(1, "base_maximum_life", "{0:+d} to maximum Life", &json!([])),
            description(
                3,
                "local_affliction_jewel_small_nodes_grant_maximum_life",
                "Added Small Passive Skills also grant: {0:+d} to Maximum Life",
                &reminder
            ),
            // Two globals read the same: no pair.
            description(5, "a_one", "{0:+d} to Twice", &json!([])),
            description(7, "a_two", "{0:+d} to Twice", &json!([])),
            description(
                9,
                "local_affliction_jewel_small_nodes_grant_twice",
                "Added Small Passive Skills also grant: {0:+d} to Twice",
                &json!([])
            ),
            description(
                11,
                "local_affliction_jewel_small_nodes_grant_evasion",
                "Added Small Passive Skills also grant: {0:+d} to Evasion",
                &json!([])
            ),
        ]));
        let known: BTreeSet<&str> = [
            "local_affliction_jewel_small_nodes_grant_evasion",
            "base_evasion_rating",
        ]
        .into_iter()
        .collect();
        let g = grants(std::iter::once(&file), &known);
        assert_eq!(
            g,
            [
                ClusterGrant {
                    stat: "local_affliction_jewel_small_nodes_grant_evasion".to_owned(),
                    granted: "base_evasion_rating".to_owned(),
                },
                ClusterGrant {
                    stat: "local_affliction_jewel_small_nodes_grant_maximum_life".to_owned(),
                    granted: "base_maximum_life".to_owned(),
                },
            ]
        );
    }
}

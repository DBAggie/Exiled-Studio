//! `bundle` (Phase 2 plan, step 3b; Phase 3 plan, step 1): converts a
//! `loom-extractor extract` output into a data bundle of format 2
//! (`loom_data::bundle`): format 1's files plus the calc tables.
//!
//! The extract holds the game's tables in the community dat schema's shape;
//! the bundle holds our own canonical types (plan decision 8). The mapping
//! lives here, with the readers the corpus harness already uses, so the two
//! read the tables one way:
//! - `Stats` to stats (`IsLocal`, `IsWeaponLocal`);
//! - `Mods` to mod rows by ID (stats with their ranges, `(0, 0)` kept; spawn
//!   weights by tag; influence, `NONE` dropped; crafting classes and
//!   essence classes by `ItemClasses.Id`, the essence-column map of
//!   `mod_filter`; mod type by `ModType.Name`);
//! - `BaseItemTypes` (named rows) to base items, with the tags `item_tags`
//!   derives and the implicits by mod ID;
//! - `ItemClasses` to class IDs; the `ClientStrings` rows the game's item
//!   rules name; the lookup tables `lookup` builds; the stat-description
//!   files, copied as the extractor wrote them;
//! - PoE1: `poe1/quality_types.json` (`properties::quality_types`) and
//!   `poe1/grafts.json` (`grafts::rows`);
//! - the game's item rules: `poe1::item_rules` or `poe2::item_rules`;
//! - format 2, both games: the skill tables ([`skills`]).
//!
//! JSON is written with sorted keys and two-space indentation, rows in
//! table order, so two runs on one extract give the same bytes.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::poe1::{self as bundle_poe1, Grafts as BundleGrafts};
use loom_data::bundle::skills as bundle_skills;
use loom_data::bundle::{
    self, BaseItem, ClientText, FileEntry, LookupTable, Manifest, ModEntry, ModStat, Provenance,
    SpawnWeight, Stat,
};
use loom_data::stat_desc::{Lookup, LookupHandler, StatDescManifest};
use loom_parser::import::ItemRules;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::extract::{Extract, Read, Table};
use crate::item_tags::ItemTags;
use crate::lookup::{LookupReport, TableLookup};
use crate::{grafts, mod_filter, poe1, poe2, properties};

mod skills;

/// The bundle's files by path (the manifest included).
pub type Files = BTreeMap<String, Vec<u8>>;

/// One file of the summary.
#[derive(Debug, Serialize)]
pub struct FileSummary {
    /// Rows (top-level array length), when the file is an array.
    pub rows: Option<usize>,
    /// SHA-256.
    pub sha256: String,
    /// Bytes.
    pub size: u64,
}

/// What `bundle` wrote (no game text; scrubbed by construction).
#[derive(Debug, Serialize)]
pub struct Summary {
    /// `poe1` or `poe2`.
    pub game: String,
    /// The extract's Steam build.
    pub steam_build: Option<u64>,
    /// SHA-256 of `bundle.json`.
    pub manifest_sha256: String,
    /// Every other file.
    pub files: BTreeMap<String, FileSummary>,
    /// `Mods` rows read without spawn weights (tag and weight lists of
    /// different lengths).
    pub mods_bad_spawn_rows: usize,
    /// Named base rows whose tags are unknown.
    pub bases_tags_unknown: usize,
    /// Row references (an index into another table) that point past the
    /// table's end, dropped: craft and essence classes, implicits, spawn
    /// and base tags, item classes, mod types, stats.
    pub dangling_references: usize,
    /// `BaseItemTypes` rows without a name (not base items; skipped).
    pub nameless_bases_skipped: usize,
    /// Stat slots with a stat and no value range (dropped, as the corpus
    /// reader drops them).
    pub stat_slots_without_range: usize,
    /// Client text IDs the rules name that `ClientStrings` lacks.
    pub client_texts_missing: Vec<String>,
    /// `ModType.Name` values shared by more than one row (their mods'
    /// types merge in the bundle).
    pub mod_type_duplicate_names: usize,
    /// The lookup tables as `lookup` built them.
    pub lookup: LookupReport,
    /// The skill tables (format 2).
    pub skills: skills::SkillSummary,
}

/// Counters the readers fill (see [`Summary`]).
#[derive(Default)]
struct Counts {
    dangling: Cell<usize>,
    stat_slots_without_range: Cell<usize>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn to_json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let v = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let mut out = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    out.push(b'\n');
    Ok(out)
}

fn text_column(read: Read<'_>, table: &str, column: &str) -> Result<Vec<String>, String> {
    let t = Table::require(read, table)?;
    let c = t.column(column)?;
    Ok(t.rows
        .iter()
        .map(|r| Table::text(r, c).unwrap_or("").to_owned())
        .collect())
}

/// The first of `names` the table has (the PoE1 and PoE2 schemas name some
/// columns differently: `ModTypeKey` / `ModType`, `ItemClassesKey` /
/// `ItemClass`, ...).
fn column(t: &Table, names: &[&str]) -> Result<usize, String> {
    names
        .iter()
        .find_map(|n| t.column(n).ok())
        .ok_or_else(|| format!("{}: none of the columns {names:?}", t.table))
}

/// Where a mod row keeps one stat: its key and either two range columns
/// (PoE1 `StatNMin`, `StatNMax`) or one `[min, max]` column (PoE2
/// `StatNValue`).
enum StatSlot {
    Split(usize, usize, usize),
    Pair(usize, usize),
}

impl StatSlot {
    fn find(t: &Table, n: usize) -> Result<Self, String> {
        let key = column(t, &[&format!("StatsKey{n}"), &format!("Stat{n}")])?;
        if let (Ok(lo), Ok(hi)) = (
            t.column(&format!("Stat{n}Min")),
            t.column(&format!("Stat{n}Max")),
        ) {
            return Ok(Self::Split(key, lo, hi));
        }
        Ok(Self::Pair(key, t.column(&format!("Stat{n}Value"))?))
    }

    fn read(&self, row: &[Value]) -> (Option<i64>, Option<(i64, i64)>) {
        match *self {
            Self::Split(k, lo, hi) => (
                Table::int(row, k),
                Table::int(row, lo).zip(Table::int(row, hi)),
            ),
            Self::Pair(k, v) => {
                let pair = row
                    .get(v)
                    .and_then(Value::as_array)
                    .and_then(|a| Some((a.first()?.as_i64()?, a.get(1)?.as_i64()?)));
                (Table::int(row, k), pair)
            }
        }
    }
}

/// Row `n` of `list`; a reference past the end is counted as dangling.
fn at(list: &[String], n: Option<i64>, counts: &Counts) -> Option<String> {
    let n = n?;
    let found = usize::try_from(n).ok().and_then(|n| list.get(n)).cloned();
    if found.is_none() {
        counts.dangling.set(counts.dangling.get() + 1);
    }
    found
}

/// Fails on an ID that two rows share.
fn unique_ids<'a>(table: &str, ids: impl Iterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("{table}: the ID {id:?} is on two rows"));
        }
    }
    Ok(())
}

fn stats(read: Read<'_>) -> Result<Vec<Stat>, String> {
    let t = Table::require(read, "Stats")?;
    let (id, local, weapon) = (
        t.column("Id")?,
        t.column("IsLocal")?,
        t.column("IsWeaponLocal")?,
    );
    let flag = |r: &[Value], c: usize| r.get(c).and_then(Value::as_bool).unwrap_or(false);
    Ok(t.rows
        .iter()
        .filter_map(|r| {
            Some(Stat {
                id: Table::text(r, id)?.to_owned(),
                local: flag(r, local),
                weapon_local: flag(r, weapon),
            })
        })
        .collect())
}

/// Mod rows by ID, and the count of rows read without spawn weights.
fn mods(
    read: Read<'_>,
    classes: &[String],
    counts: &Counts,
) -> Result<(Vec<ModEntry>, usize, usize), String> {
    let stat_ids = text_column(read, "Stats", "Id")?;
    let tags = mod_filter::tag_ids(read)?;
    let mod_types = text_column(read, "ModType", "Name")?;
    let t = Table::require(read, "Mods")?;
    let col = |n: &str| t.column(n);
    let (id, dom, gen_) = (col("Id")?, col("Domain")?, col("GenerationType")?);
    let mod_type = column(&t, &["ModTypeKey", "ModType"])?;
    let spawn_tags = column(&t, &["SpawnWeight_TagsKeys", "SpawnWeight_Tags"])?;
    let spawn_values = col("SpawnWeight_Values")?;
    let (influence, craft, essence) = (
        column(&t, &["InfluenceTypes", "InfluenceType"])?,
        col("CraftingItemClassRestrictions")?,
        col("IsEssenceOnlyModifier")?,
    );
    let mut slots = Vec::new();
    for n in 1..=8 {
        slots.push(StatSlot::find(&t, n)?);
    }
    let essence_of = mod_filter::essence_classes(read)?.unwrap_or_default();
    let class_ids = |rows: &BTreeSet<i64>| -> Vec<String> {
        rows.iter()
            .filter_map(|n| at(classes, Some(*n), counts))
            .collect()
    };
    let mut out = Vec::with_capacity(t.rows.len());
    let mut bad_spawn = 0;
    for (n, r) in t.rows.iter().enumerate() {
        let mut row_stats = Vec::new();
        for slot in &slots {
            let (key, range) = slot.read(r);
            let Some(stat) = at(&stat_ids, key, counts) else {
                continue;
            };
            if let Some((a, b)) = range {
                row_stats.push(ModStat {
                    stat,
                    min: a.min(b),
                    max: a.max(b),
                });
            } else {
                counts
                    .stat_slots_without_range
                    .set(counts.stat_slots_without_range.get() + 1);
            }
        }
        let spawn = mod_filter::spawn_pairs(r, spawn_tags, spawn_values, &tags)
            .unwrap_or_else(|| {
                bad_spawn += 1;
                Vec::new()
            })
            .into_iter()
            .map(|(tag, weight)| SpawnWeight { tag, weight })
            .collect();
        let craft_rows: BTreeSet<i64> = mod_filter::ints(r, craft).into_iter().collect();
        out.push(ModEntry {
            id: Table::text(r, id).unwrap_or("").to_owned(),
            domain: Table::int(r, dom).unwrap_or(0),
            generation: Table::int(r, gen_).unwrap_or(0),
            mod_type: at(&mod_types, Table::int(r, mod_type), counts),
            stats: row_stats,
            spawn,
            influence: Table::int(r, influence).filter(|v| *v != poe1::NO_INFLUENCE),
            craft_classes: class_ids(&craft_rows),
            essence_only: r.get(essence).and_then(Value::as_bool).unwrap_or(false),
            essence_classes: essence_of.get(&n).map(class_ids),
        });
    }
    let names: BTreeSet<&str> = mod_types.iter().map(String::as_str).collect();
    let duplicate_types = mod_types.len() - names.len();
    Ok((out, bad_spawn, duplicate_types))
}

/// Named base items, and the count whose tags are unknown.
fn base_items(
    read: Read<'_>,
    classes: &[String],
    mods: &[ModEntry],
    counts: &Counts,
) -> Result<(Vec<BaseItem>, usize, usize), String> {
    let tag_ids = mod_filter::tag_ids(read)?;
    let t = Table::require(read, "BaseItemTypes")?;
    let col = |n: &str| t.column(n);
    let (id, name, domain, class) = (
        col("Id")?,
        col("Name")?,
        col("ModDomain")?,
        column(&t, &["ItemClassesKey", "ItemClass"])?,
    );
    let (inherits, tags, implicits) = (
        col("InheritsFrom")?,
        column(&t, &["TagsKeys", "Tags"])?,
        column(&t, &["Implicit_ModsKeys", "Implicit_Mods"])?,
    );
    let mut item_tags = ItemTags::default();
    let (mut unknown, mut nameless) = (0, 0);
    let mut out = Vec::new();
    for r in &t.rows {
        let (Some(i), Some(n)) = (Table::text(r, id), Table::text(r, name)) else {
            continue;
        };
        if n.is_empty() {
            nameless += 1;
            continue;
        }
        let row_tags = Table::text(r, inherits)
            .and_then(|path| item_tags.tags(read, path))
            .map(|mut t| {
                t.extend(
                    mod_filter::ints(r, tags)
                        .into_iter()
                        .filter_map(|k| at(&tag_ids, Some(k), counts)),
                );
                t
            });
        if row_tags.is_none() {
            unknown += 1;
        }
        out.push(BaseItem {
            id: i.to_owned(),
            name: n.to_owned(),
            domain: Table::int(r, domain).unwrap_or(0),
            item_class: at(classes, Table::int(r, class), counts),
            tags: row_tags,
            implicits: mod_filter::ints(r, implicits)
                .into_iter()
                .filter_map(|k| {
                    let found = usize::try_from(k)
                        .ok()
                        .and_then(|n| mods.get(n))
                        .map(|m| m.id.clone());
                    if found.is_none() {
                        counts.dangling.set(counts.dangling.get() + 1);
                    }
                    found
                })
                .collect(),
        });
    }
    Ok((out, unknown, nameless))
}

/// The `ClientStrings` rows the rules name, sorted by ID.
/// Also the named IDs `ClientStrings` lacks.
fn client_texts(
    read: Read<'_>,
    rules: &ItemRules,
) -> Result<(Vec<ClientText>, Vec<String>), String> {
    let t = &rules.text;
    let named: BTreeSet<&str> = t
        .compositions
        .iter()
        .map(|(id, _)| id.as_str())
        .chain(t.property_ids.iter().map(String::as_str))
        .chain(t.label_ids.iter().map(String::as_str))
        .chain(t.granted_skill_wrapper.as_deref())
        .collect();
    let mut out = Vec::new();
    if let Some(cs) = Table::read(read, "ClientStrings")? {
        let (id, text) = (cs.column("Id")?, cs.column("Text")?);
        for r in &cs.rows {
            if let (Some(i), Some(x)) = (Table::text(r, id), Table::text(r, text))
                && named.contains(i)
            {
                out.push(ClientText {
                    id: i.to_owned(),
                    text: x.to_owned(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    let missing = named
        .iter()
        .filter(|id| out.iter().all(|c| c.id != **id))
        .map(|id| (*id).to_owned())
        .collect();
    Ok((out, missing))
}

fn lookups(read: Read<'_>, extract: &Extract) -> Result<(Vec<LookupTable>, LookupReport), String> {
    let (lookup, report) = TableLookup::load(read, extract)?;
    let tables = LookupHandler::ALL
        .iter()
        .filter_map(|h| {
            let entries = lookup.entries(*h);
            (!entries.is_empty()).then(|| LookupTable {
                handler: h.name().to_owned(),
                entries,
            })
        })
        .collect();
    Ok((tables, report))
}

/// `bundle.json`'s provenance from the extract's `manifest.json`.
fn provenance(manifest: &Value) -> Provenance {
    let text = |path: &[&str]| -> String {
        let mut v = manifest;
        for p in path {
            v = &v[*p];
        }
        v.as_str().unwrap_or("").to_owned()
    };
    Provenance {
        steam_build: text(&["steam_build_id"]).parse().ok(),
        index_sha256: text(&["index", "sha256"]),
        extractor_commit: text(&["extractor", "commit"]),
        decoder: format!(
            "{} {}",
            text(&["decoder", "program"]),
            text(&["decoder", "version"])
        )
        .trim()
        .to_owned(),
        dat_schema: format!(
            "{}@{}",
            text(&["schema", "source"]),
            text(&["schema", "commit"])
        ),
    }
}

fn put(
    files: &mut Files,
    rows: &mut BTreeMap<String, usize>,
    path: &str,
    bytes: Vec<u8>,
    n: Option<usize>,
) {
    if let Some(n) = n {
        rows.insert(path.to_owned(), n);
    }
    files.insert(path.to_owned(), bytes);
}

/// Converts an extract (module docs); returns the files and the summary.
///
/// # Errors
/// A message when a table, column, or file is missing or malformed, or the
/// game is not `poe1` or `poe2`.
pub fn convert(read: Read<'_>, extract: &Extract) -> Result<(Files, Summary), String> {
    let manifest_bytes = read("manifest.json")?;
    let raw: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    let game = extract.inputs.game.clone();
    let rules = match game.as_str() {
        "poe1" => poe1::item_rules(),
        "poe2" => poe2::item_rules(),
        other => return Err(format!("unknown game {other:?}")),
    };
    rules
        .mods
        .validate()
        .map_err(|e| format!("item rules: {}", e.code()))?;
    let counts = Counts::default();
    let classes = text_column(read, "ItemClasses", "Id")?;
    unique_ids("ItemClasses", classes.iter().map(String::as_str))?;
    let (mod_rows, bad_spawn, duplicate_types) = mods(read, &classes, &counts)?;
    unique_ids("Mods", mod_rows.iter().map(|m| m.id.as_str()))?;
    let (bases, tags_unknown, nameless) = base_items(read, &classes, &mod_rows, &counts)?;
    let (skill_tables, skill_summary) = skills::convert(read, &classes, &counts)?;

    let mut files: Files = BTreeMap::new();
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    let st = stats(read)?;
    unique_ids("Stats", st.iter().map(|x| x.id.as_str()))?;
    put(
        &mut files,
        &mut rows,
        bundle::STATS,
        to_json(&st)?,
        Some(st.len()),
    );
    put(
        &mut files,
        &mut rows,
        bundle::MODS,
        to_json(&mod_rows)?,
        Some(mod_rows.len()),
    );
    put(
        &mut files,
        &mut rows,
        bundle::BASE_ITEMS,
        to_json(&bases)?,
        Some(bases.len()),
    );
    put(
        &mut files,
        &mut rows,
        bundle::ITEM_CLASSES,
        to_json(&classes)?,
        Some(classes.len()),
    );
    let (ct, client_missing) = client_texts(read, &rules)?;
    put(
        &mut files,
        &mut rows,
        bundle::CLIENT_TEXTS,
        to_json(&ct)?,
        Some(ct.len()),
    );
    let (lk, lookup_report) = lookups(read, extract)?;
    put(
        &mut files,
        &mut rows,
        bundle::LOOKUPS,
        to_json(&lk)?,
        Some(lk.len()),
    );
    put(
        &mut files,
        &mut rows,
        bundle::ITEM_RULES,
        to_json(&rules)?,
        None,
    );

    let sk = &skill_tables;
    let skill_files: [(&str, Vec<u8>, usize); 7] = [
        (
            bundle_skills::SKILL_TYPES,
            to_json(&sk.skill_types)?,
            sk.skill_types.len(),
        ),
        (
            bundle_skills::ACTIVE_SKILLS,
            to_json(&sk.active_skills)?,
            sk.active_skills.len(),
        ),
        (
            bundle_skills::GRANTED_EFFECTS,
            to_json(&sk.granted_effects)?,
            sk.granted_effects.len(),
        ),
        (
            bundle_skills::STAT_SETS,
            to_json(&sk.stat_sets)?,
            sk.stat_sets.len(),
        ),
        (bundle_skills::GEMS, to_json(&sk.gems)?, sk.gems.len()),
        (
            bundle_skills::GEM_EFFECTS,
            to_json(&sk.gem_effects)?,
            sk.gem_effects.len(),
        ),
        (
            bundle_skills::GEM_TAGS,
            to_json(&sk.gem_tags)?,
            sk.gem_tags.len(),
        ),
    ];
    for (path, bytes, n) in skill_files {
        put(&mut files, &mut rows, path, bytes, Some(n));
    }

    let statdesc = StatDescManifest::from_json(&manifest_bytes).map_err(|e| e.to_string())?;
    for path in &extract.files {
        let output = statdesc
            .files
            .get(path)
            .and_then(|f| f.output.clone())
            .ok_or_else(|| format!("{path}: no output in the manifest"))?;
        // The extract's own path below its stat-description folder: PoE2
        // has files of one name in several folders.
        let name = output
            .strip_prefix(bundle::STAT_DESCRIPTIONS_DIR)
            .unwrap_or(&output);
        let target = format!("{}{name}", bundle::STAT_DESCRIPTIONS_DIR);
        if files.contains_key(&target) {
            return Err(format!(
                "{target}: two stat-description files share the name"
            ));
        }
        put(&mut files, &mut rows, &target, read(&output)?, None);
    }

    if game == "poe1" {
        if let Some(q) = properties::quality_types(read)? {
            let q: Vec<bundle_poe1::QualityType> = q
                .into_iter()
                .map(|q| bundle_poe1::QualityType {
                    id: q.id,
                    description: q.description,
                    stats: q.stats,
                })
                .collect();
            put(
                &mut files,
                &mut rows,
                bundle_poe1::QUALITY_TYPES,
                to_json(&q)?,
                Some(q.len()),
            );
        }
        if let Some(g) = grafts::rows(read)? {
            let n = g.bases.len();
            let g = BundleGrafts {
                bases: g.bases,
                levels: g.levels,
            };
            put(
                &mut files,
                &mut rows,
                bundle_poe1::GRAFTS,
                to_json(&g)?,
                Some(n),
            );
        }
    }

    let entries: Vec<FileEntry> = files
        .iter()
        .map(|(path, bytes)| FileEntry {
            path: path.clone(),
            sha256: hex(&Sha256::digest(bytes)),
            size: bytes.len() as u64,
        })
        .collect();
    let manifest = Manifest {
        format: bundle::CALC_FORMAT,
        data_schema: 2,
        game: game.clone(),
        source: provenance(&raw),
        files: entries,
        signatures: Vec::new(),
    };
    let manifest_json = to_json(&manifest)?;
    let summary = Summary {
        game,
        steam_build: manifest.source.steam_build,
        manifest_sha256: hex(&Sha256::digest(&manifest_json)),
        files: manifest
            .files
            .iter()
            .map(|f| {
                (
                    f.path.clone(),
                    FileSummary {
                        rows: rows.get(&f.path).copied(),
                        sha256: f.sha256.clone(),
                        size: f.size,
                    },
                )
            })
            .collect(),
        mods_bad_spawn_rows: bad_spawn,
        bases_tags_unknown: tags_unknown,
        dangling_references: counts.dangling.get(),
        nameless_bases_skipped: nameless,
        stat_slots_without_range: counts.stat_slots_without_range.get(),
        client_texts_missing: client_missing,
        mod_type_duplicate_names: duplicate_types,
        lookup: lookup_report,
        skills: skill_summary,
    };
    files.insert(bundle::MANIFEST.to_owned(), manifest_json);
    Ok((files, summary))
}

/// The tables `convert` reads for PoE1 only (the `poe1/` section and the
/// essence classes), for gate 3d.
#[cfg(test)]
const POE1_ONLY_TABLES: [&str; 4] = [
    "AlternateQualityTypes",
    "BrequelGrafts",
    "BrequelGraftGrantedSkillLevels",
    "Essences",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 2 gate 3d: the game-specific tables the bundle reads are the
    /// ones the extractor extracts for PoE1 only (`tools/extractor/src/
    /// extract.rs`, the `*_POE1` lists; the tools cannot depend on each
    /// other, so the list is read as text).
    #[test]
    fn poe1_only_tables_should_be_in_the_extractor_poe1_lists() {
        let source = include_str!("../../extractor/src/extract.rs");
        // Each `pub const NAME: [&str; n] = [...];`, cut at its `];`.
        let lists: Vec<(&str, &str)> = source
            .split("pub const ")
            .skip(1)
            .filter_map(|c| {
                let (name, rest) = c.split_once(':')?;
                Some((name.trim(), rest.split_once("];")?.0))
            })
            .collect();
        let poe1_lists: Vec<&str> = lists
            .iter()
            .filter(|(n, _)| n.ends_with("_POE1"))
            .map(|(_, body)| *body)
            .collect();
        // No PoE1-only table is in a list extracted for both games.
        for (name, body) in lists
            .iter()
            .filter(|(n, _)| ["SPIKE_TABLES", "LOOKUP_TABLES", "SKILL_TABLES"].contains(n))
        {
            for t in POE1_ONLY_TABLES {
                assert!(
                    !body.contains(&format!("\"{t}\"")),
                    "{t} is in the shared {name}"
                );
            }
        }
        let missing: Vec<&str> = POE1_ONLY_TABLES
            .into_iter()
            .filter(|t| !poe1_lists.iter().any(|l| l.contains(&format!("\"{t}\""))))
            .collect();
        assert_eq!(missing, [] as [&str; 0]);
    }

    fn table(columns: &[&str]) -> Table {
        Table {
            table: "Mods".to_owned(),
            columns: columns
                .iter()
                .map(|c| crate::extract::Column {
                    name: Some((*c).to_owned()),
                    references: None,
                })
                .collect(),
            rows: Vec::new(),
        }
    }

    #[test]
    fn stat_slot_should_read_poe1_split_ranges() {
        let t = table(&["StatsKey1", "Stat1Min", "Stat1Max"]);
        let row = serde_json::json!([4, 9, 3]);
        let slot = StatSlot::find(&t, 1).unwrap();
        assert_eq!(slot.read(row.as_array().unwrap()), (Some(4), Some((9, 3))));
    }

    #[test]
    fn stat_slot_should_read_poe2_value_pairs() {
        let t = table(&["Stat1", "Stat1Value"]);
        let row = serde_json::json!([4, [5, 8]]);
        let slot = StatSlot::find(&t, 1).unwrap();
        assert_eq!(slot.read(row.as_array().unwrap()), (Some(4), Some((5, 8))));
    }

    #[test]
    fn column_should_take_the_first_name_the_table_has() {
        let t = table(&["ModType"]);
        assert_eq!(
            (
                column(&t, &["ModTypeKey", "ModType"]),
                column(&t, &["Nope"]).is_err()
            ),
            (Ok(0), true)
        );
    }
}

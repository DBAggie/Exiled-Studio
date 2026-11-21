//! Timeless jewels for the PoE1 bundle (Phase 6 P6-10a; plan
//! `dev/active/phase-6-mechanics/p6-10a-timeless-plan.md`):
//!
//! - [`rows`]: `AlternatePassiveSkills`, `AlternatePassiveAdditions`, and
//!   `AlternateTreeVersions` to `poe1/timeless.json`
//!   (`loom_data::bundle::timeless::Timeless`), as PoB's
//!   `Export/Scripts/legionPassives.lua` reads them. The schema leaves
//!   some columns unnamed; PoB's `Export/spec.lua` names them, so they are
//!   read by position, after the named ones are checked at theirs:
//!   `AlternatePassiveSkills` `Stat3` to `Stat6` (columns 9 to 16),
//!   `ConquerorIndex` (18), and `ConquerorVersion` (24);
//!   `AlternatePassiveAdditions` `Stat2` and `Stat3` (6 to 9);
//!   `AlternateTreeVersions` `SmallAttributeReplaced` (1) and
//!   `SmallNormalPassiveReplaced` (2). `None` when the extract lacks one
//!   of the tables.
//! - [`pack`] (`loom-parity-cli timeless-pack`): a copy of a bundle with
//!   PoB's results added (`poe1/timeless_luts.json` and
//!   `poe1/timeless/<Type>.bin`, PoB's MIT data, plan decision T1): its
//!   `NodeIndexMapping.lua`, its `Modules/Data.lua` jewel types and seed
//!   ranges, and each `Data/TimelessJewelData/<Type>.zip` (or its parts)
//!   inflated, byte for byte.

use std::collections::BTreeMap;
use std::path::Path;

use loom_data::bundle::Bundle;
use loom_data::bundle::timeless::{
    AltAddition, AltPassive, AltStat, AltTreeVersion, LutId, LutJewel, LutNode, LutSource,
    TIMELESS, TIMELESS_DIR, TIMELESS_LUTS, Timeless, TimelessLuts,
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::extract::{Read, Table};
use crate::tree_layout::{hex, read, write_copy};

/// The extracted tables' rows as PoB's spec names their columns.
///
/// # Errors
/// A message when a table's named columns are not where the positions
/// assume, or a cell has the wrong type.
pub fn rows(read: Read<'_>) -> Result<Option<Timeless>, String> {
    let (Some(skills), Some(additions), Some(versions), Some(stats)) = (
        Table::read(read, "AlternatePassiveSkills")?,
        Table::read(read, "AlternatePassiveAdditions")?,
        Table::read(read, "AlternateTreeVersions")?,
        Table::read(read, "Stats")?,
    ) else {
        return Ok(None);
    };
    // A key is the stat's row.
    let stat_col = stats.column("Id")?;
    let stat_of = |row: i64| -> Result<String, String> {
        usize::try_from(row)
            .ok()
            .and_then(|r| stats.rows.get(r))
            .and_then(|r| Table::text(r, stat_col))
            .map(str::to_owned)
            .ok_or_else(|| format!("Stats: no row {row}"))
    };
    let expect = |t: &Table, at: &[(usize, &str)]| -> Result<(), String> {
        for (col, name) in at {
            if t.columns.get(*col).and_then(|c| c.name.as_deref()) != Some(*name) {
                return Err(format!("{}: column {col} is not {name}", t.table));
            }
        }
        Ok(())
    };
    expect(
        &skills,
        &[
            (0, "Id"),
            (1, "AlternateTreeVersionsKey"),
            (2, "Name"),
            (3, "PassiveType"),
            (4, "StatsKeys"),
            (5, "Stat1Min"),
            (6, "Stat1Max"),
            (7, "Stat2Min"),
            (8, "Stat2Max"),
            (17, "SpawnWeight"),
            (19, "RandomMin"),
            (21, "FlavourText"),
        ],
    )?;
    expect(
        &additions,
        &[
            (0, "Id"),
            (1, "AlternateTreeVersionsKey"),
            (3, "StatsKeys"),
            (4, "Stat1Min"),
            (5, "Stat1Max"),
            (10, "PassiveType"),
        ],
    )?;
    expect(&versions, &[(0, "Id")])?;
    let int = |t: &Table, r: &[Value], c: usize| -> Result<i64, String> {
        Table::int(r, c).ok_or_else(|| format!("{}: column {c} is not a number", t.table))
    };
    let u = |t: &Table, r: &[Value], c: usize| -> Result<u32, String> {
        u32::try_from(int(t, r, c)?).map_err(|e| format!("{}: column {c}: {e}", t.table))
    };
    let list = |t: &Table, r: &[Value], c: usize| -> Result<Vec<i64>, String> {
        match r.get(c) {
            Some(Value::Array(items)) => items
                .iter()
                .map(|v| v.as_i64().ok_or_else(|| format!("{}: column {c}", t.table)))
                .collect(),
            Some(v) => v
                .as_i64()
                .map(|n| vec![n])
                .ok_or_else(|| format!("{}: column {c}", t.table)),
            None => Ok(Vec::new()),
        }
    };
    let stats_of =
        |t: &Table, r: &[Value], keys: usize, ranges: &[usize]| -> Result<Vec<AltStat>, String> {
            list(t, r, keys)?
                .into_iter()
                .zip(ranges)
                .map(|(k, &c)| {
                    Ok(AltStat {
                        stat: stat_of(k)?,
                        min: int(t, r, c)?,
                        max: int(t, r, c + 1)?,
                    })
                })
                .collect()
        };
    let types = |t: &Table, r: &[Value], c: usize| -> Result<Vec<u32>, String> {
        list(t, r, c)?
            .into_iter()
            .map(|n| u32::try_from(n).map_err(|e| e.to_string()))
            .collect()
    };
    let mut out = Timeless {
        tree_versions: Vec::new(),
        passives: Vec::new(),
        additions: Vec::new(),
    };
    for (i, r) in versions.rows.iter().enumerate() {
        let flag = |c: usize| r.get(c).and_then(Value::as_bool).unwrap_or(false);
        out.tree_versions.push(AltTreeVersion {
            row: u32::try_from(i).map_err(|e| e.to_string())?,
            id: Table::text(r, 0).unwrap_or_default().to_owned(),
            small_attribute_replaced: flag(1),
            small_normal_replaced: flag(2),
        });
    }
    for (i, r) in skills.rows.iter().enumerate() {
        out.passives.push(AltPassive {
            row: u32::try_from(i).map_err(|e| e.to_string())?,
            id: Table::text(r, 0).unwrap_or_default().to_owned(),
            name: Table::text(r, 2).unwrap_or_default().to_owned(),
            tree_version: u(&skills, r, 1)?,
            passive_types: types(&skills, r, 3)?,
            stats: stats_of(&skills, r, 4, &[5, 7, 9, 11, 13, 15])?,
            conqueror_index: u(&skills, r, 18)?,
            conqueror_version: u(&skills, r, 24)?,
        });
    }
    for (i, r) in additions.rows.iter().enumerate() {
        out.additions.push(AltAddition {
            row: u32::try_from(i).map_err(|e| e.to_string())?,
            id: Table::text(r, 0).unwrap_or_default().to_owned(),
            tree_version: u(&additions, r, 1)?,
            passive_types: types(&additions, r, 10)?,
            stats: stats_of(&additions, r, 3, &[4, 6, 8])?,
        });
    }
    Ok(Some(out))
}

/// What [`pack`] wrote.
#[derive(Debug, Serialize)]
pub struct PackSummary {
    /// PoB's version.
    pub pob_version: String,
    /// Each results file and its bytes.
    pub files: BTreeMap<String, u64>,
    /// The nodes and notables of the mapping.
    pub node_count: u32,
    /// Notables.
    pub notable_count: u32,
    /// The written manifest's SHA-256.
    pub manifest_sha256: String,
}

/// A Lua table of `[n] = <value>` entries after `name = {` in `text`.
fn lua_numbers(text: &str, name: &str) -> Result<BTreeMap<u32, String>, String> {
    let start = text
        .find(&format!("{name} = {{"))
        .ok_or_else(|| format!("Data.lua: no {name}"))?;
    let body = &text[start..];
    let end = body
        .find('}')
        .ok_or_else(|| format!("Data.lua: {name} not closed"))?;
    let mut out = BTreeMap::new();
    for line in body[..end].lines() {
        let line = line.trim().trim_end_matches(',');
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((key, value)) = rest.split_once("] =") else {
            continue;
        };
        let key: u32 = key.trim().parse().map_err(|e| format!("{name}: {e}"))?;
        out.insert(key, value.trim().trim_matches('"').to_owned());
    }
    Ok(out)
}

/// A Lua number expression of PoB's seed tables (`2000 / 20`).
fn lua_value(text: &str) -> Result<u32, String> {
    let parts: Vec<&str> = text.split('/').map(str::trim).collect();
    let n = |s: &str| s.parse::<u32>().map_err(|e| format!("{text}: {e}"));
    match parts.as_slice() {
        [a] => n(a),
        [a, b] => Ok(n(a)? / n(b)?),
        _ => Err(format!("{text}: not a number")),
    }
}

/// PoB's `NodeIndexMapping.lua` read as its `nodeIDList`.
fn mapping(text: &str) -> Result<(u32, u32, Vec<LutNode>, Vec<LutId>), String> {
    let mut size = None;
    let mut notable = None;
    let mut nodes = Vec::new();
    let mut ids = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("nodeIDList[\"size\"] = ") {
            size = Some(v.parse::<u32>().map_err(|e| e.to_string())?);
        } else if let Some(v) = line.strip_prefix("nodeIDList[\"sizeNotable\"] = ") {
            notable = Some(v.parse::<u32>().map_err(|e| e.to_string())?);
        } else if let Some(rest) = line.strip_prefix("nodeIDList[\"localIdToGlobalId\"][") {
            // `<jewel>][<local>] = <global>`; the `["size"]` and table lines
            // carry no pair.
            let Some((jewel, rest)) = rest.split_once("][") else {
                continue;
            };
            let Some((local, global)) = rest.split_once("] = ") else {
                continue;
            };
            let (Ok(jewel), Ok(local), Ok(global)) =
                (jewel.parse(), local.parse(), global.trim().parse())
            else {
                continue;
            };
            ids.push(LutId {
                jewel,
                local,
                global,
            });
        } else if let Some(rest) = line.strip_prefix("nodeIDList[") {
            let Some((node, rest)) = rest.split_once("] = { index = ") else {
                continue;
            };
            let Some((index, rest)) = rest.split_once(", size = ") else {
                continue;
            };
            let size = rest.trim_end_matches(" }").trim();
            nodes.push(LutNode {
                node: node.parse().map_err(|e| format!("node {node}: {e}"))?,
                index: index.parse().map_err(|e| format!("index {index}: {e}"))?,
                size: size.parse().map_err(|e| format!("size {size}: {e}"))?,
            });
        }
    }
    nodes.sort_by_key(|n| n.index);
    ids.sort_by_key(|i| (i.jewel, i.local));
    Ok((
        size.ok_or("NodeIndexMapping.lua: no size")?,
        notable.ok_or("NodeIndexMapping.lua: no sizeNotable")?,
        nodes,
        ids,
    ))
}

/// A jewel type's zip (or its numbered parts) inflated.
fn inflate(dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    let whole = dir.join(format!("{name}.zip"));
    let data = if whole.exists() {
        read(&whole)?
    } else {
        let mut data = Vec::new();
        for part in 0.. {
            let p = dir.join(format!("{name}.zip.part{part}"));
            if !p.exists() {
                break;
            }
            data.extend(read(&p)?);
        }
        data
    };
    if data.is_empty() {
        return Err(format!("{name}: no zip"));
    }
    miniz_oxide::inflate::decompress_to_vec_zlib(&data).map_err(|e| format!("{name}: {e:?}"))
}

/// Copies the bundle `bundle` into `out` (which must not exist) with PoB's
/// results added (module docs), read from the PoB checkout's `src` folder
/// `pob`. Refuses a bundle without `poe1/timeless.json`.
///
/// # Errors
/// A message naming the first problem.
pub fn pack(
    bundle: &Path,
    pob: &Path,
    pob_version: &str,
    out: &Path,
) -> Result<PackSummary, String> {
    let source = crate::DirSource(bundle);
    let report = loom_data::bundle::validate::validate(&source);
    if !report.is_valid() {
        return Err(format!(
            "{}: the bundle does not validate",
            bundle.display()
        ));
    }
    let loaded = Bundle::load(&source).map_err(|e| format!("{}: {e}", bundle.display()))?;
    let timeless = loaded
        .poe1
        .as_ref()
        .and_then(|p| p.timeless.as_ref())
        .ok_or(format!("the bundle has no {TIMELESS}"))?;
    let data_dir = pob.join("Data").join("TimelessJewelData");
    let data_lua =
        String::from_utf8_lossy(&read(&pob.join("Modules").join("Data.lua"))?).into_owned();
    let names = lua_numbers(&data_lua, "data.timelessJewelTypes")?;
    let mins = lua_numbers(&data_lua, "data.timelessJewelSeedMin")?;
    let maxs = lua_numbers(&data_lua, "data.timelessJewelSeedMax")?;
    let additions_line = data_lua
        .lines()
        .find_map(|l| l.trim().strip_prefix("data.timelessJewelAdditions = "))
        .ok_or("Data.lua: no timelessJewelAdditions")?;
    let additions: u32 = additions_line
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .parse()
        .map_err(|e| format!("timelessJewelAdditions: {e}"))?;
    let count = u32::try_from(timeless.additions.len()).map_err(|e| e.to_string())?;
    if additions != count {
        return Err(format!(
            "PoB counts {additions} additions, the extract {count}"
        ));
    }
    let (node_count, notable_count, nodes, local_to_global) = mapping(&String::from_utf8_lossy(
        &read(&data_dir.join("NodeIndexMapping.lua"))?,
    ))?;
    let mut added: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut jewels = Vec::new();
    let mut files = BTreeMap::new();
    for (jewel, name) in &names {
        let file_name: String = name.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = inflate(&data_dir, &file_name)?;
        let path = format!("{TIMELESS_DIR}{file_name}.bin");
        let raw_min = lua_value(mins.get(jewel).ok_or(format!("no seed min for {name}"))?)?;
        let raw_max = lua_value(maxs.get(jewel).ok_or(format!("no seed max for {name}"))?)?;
        let divisor = if *jewel == loom_data::bundle::timeless::ELEGANT_HUBRIS {
            20
        } else {
            1
        };
        files.insert(path.clone(), bytes.len() as u64);
        jewels.push(LutJewel {
            jewel: *jewel,
            name: name.clone(),
            file: path.clone(),
            seed_min: raw_min,
            seed_max: raw_max,
            seed_divisor: divisor,
            sha256: hex(&Sha256::digest(&bytes)),
        });
        added.insert(path, bytes);
    }
    let luts = TimelessLuts {
        source: LutSource {
            pob_version: pob_version.to_owned(),
            licence: "MIT".to_owned(),
        },
        additions,
        node_count,
        notable_count,
        nodes,
        local_to_global,
        jewels,
    };
    for j in &luts.jewels {
        if let Some(len) = luts.expected_len(j) {
            let have = files.get(&j.file).copied().unwrap_or(0);
            if have != len {
                return Err(format!("{}: {have} bytes, its layout {len}", j.file));
            }
        }
    }
    let mut json = serde_json::to_vec_pretty(&luts).map_err(|e| e.to_string())?;
    json.push(b'\n');
    added.insert(TIMELESS_LUTS.to_owned(), json);
    let manifest_json = write_copy(
        bundle,
        loaded.manifest.clone(),
        &added,
        |path| path.starts_with(TIMELESS_DIR),
        out,
    )?;
    Ok(PackSummary {
        pob_version: pob_version.to_owned(),
        files,
        node_count,
        notable_count,
        manifest_sha256: hex(&Sha256::digest(&manifest_json)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pobs_tables_should_parse() {
        let data = "data.timelessJewelTypes = {\n\t[1] = \"Glorious Vanity\",\n\t[5] = \"Elegant Hubris\",\n}\ndata.timelessJewelSeedMin = {\n\t[1] = 100,\n\t[5] = 2000 / 20,\n}\n";
        let names = lua_numbers(data, "data.timelessJewelTypes").unwrap();
        assert_eq!(names.get(&5).map(String::as_str), Some("Elegant Hubris"));
        let mins = lua_numbers(data, "data.timelessJewelSeedMin").unwrap();
        assert_eq!(lua_value(&mins[&5]).unwrap(), 100);
        let map = "nodeIDList = { }\nnodeIDList[\"size\"] = 2\nnodeIDList[\"sizeNotable\"] = 1\nnodeIDList[6] = { index = 0, size = 3 }\nnodeIDList[9] = { index = 1, size = 4 }\nnodeIDList[\"localIdToGlobalId\"] = { }\nnodeIDList[\"localIdToGlobalId\"][1] = { }\nnodeIDList[\"localIdToGlobalId\"][1][\"size\"] = 1\nnodeIDList[\"localIdToGlobalId\"][1][0] = 7\nreturn nodeIDList\n";
        let (size, notable, nodes, ids) = mapping(map).unwrap();
        assert_eq!((size, notable), (2, 1));
        assert_eq!(
            nodes[1],
            LutNode {
                node: 9,
                index: 1,
                size: 4
            }
        );
        assert_eq!(
            ids,
            vec![LutId {
                jewel: 1,
                local: 0,
                global: 7
            }]
        );
    }
}

//! `timeless` (Phase 6 P6-10a): the bundle's timeless jewel data against
//! PoB's (export format 15):
//! - `node|<row>` and `addition|<row>`: the extracted tables' rows
//!   (`poe1/timeless.json`) against PoB's `LegionPassives.lua` (its own
//!   export of the same game tables): the ID and each stat's range;
//! - `mapping`: the additions' count, the node and notable counts;
//!   `index|<node>`: each node's index and size; `id|<jewel>|<local>`: each
//!   local-to-global ID (`poe1/timeless_luts.json` against PoB's
//!   `NodeIndexMapping.lua` and `Modules/Data.lua`);
//! - `jewel|<type>`: the name, the seed range, and the results file's
//!   Adler-32 (computed from the shipped `.bin`) against the Adler-32
//!   trailer of PoB's zlib stream (its inflated bytes' checksum): equal
//!   bytes, checked without PoB's side inflating.
//!
//! A row on PoB's side only is a difference; on the bundle's only, a
//! bundle-only row.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::Bundle;
use loom_data::bundle::timeless::AltStat;
use serde_json::{Value, json};

use super::compare::Comparison;
use super::pob::Export;

/// The table's name.
const T: &str = "timeless";

/// Adler-32 of `bytes` (RFC 1950).
#[must_use]
pub fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let (mut a, mut b) = (1_u32, 0_u32);
    for chunk in bytes.chunks(5_552) {
        for &x in chunk {
            a += u32::from(x);
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

fn ranges(stats: &[AltStat]) -> BTreeMap<&str, (i64, i64)> {
    stats
        .iter()
        .map(|s| (s.stat.as_str(), (s.min, s.max)))
        .collect()
}

/// Compares the table (module docs); `adlers` holds each results file's
/// Adler-32 by path.
pub fn compare_timeless(
    out: &mut Comparison,
    bundle: &Bundle,
    export: &Export,
    adlers: &BTreeMap<String, u32>,
) {
    let Some(pob) = &export.timeless else {
        return;
    };
    let section = bundle.poe1.as_ref();
    let rows = section.and_then(|p| p.timeless.as_ref());
    let luts = section.and_then(|p| p.timeless_luts.as_ref());

    // The extracted tables against PoB's legion tables, by row.
    let mut check_rows =
        |kind: &str, pob_rows: &[super::pob::LegionRow], ours: Vec<(u32, &str, &[AltStat])>| {
            let by_row: BTreeMap<u32, (&str, &[AltStat])> =
                ours.into_iter().map(|(r, id, s)| (r, (id, s))).collect();
            for (i, p) in pob_rows.iter().enumerate() {
                let row = u32::try_from(i).unwrap_or(u32::MAX);
                let key = format!("{kind}|{row}");
                let Some((id, stats)) = by_row.get(&row) else {
                    out.pob_only(T, &key);
                    continue;
                };
                out.check(T, &key, "id", json!(id), json!(p.id));
                let ours = ranges(stats);
                let names: BTreeSet<&str> = ours
                    .keys()
                    .copied()
                    .chain(p.stats.keys().map(String::as_str))
                    .collect();
                for name in names {
                    let a = ours.get(name);
                    let b = p.stats.get(name);
                    out.check(
                        T,
                        &key,
                        format!("stat.{name}.min"),
                        a.map_or(Value::Null, |r| json!(r.0 as f64)),
                        b.map_or(Value::Null, |r| json!(r.min)),
                    );
                    out.check(
                        T,
                        &key,
                        format!("stat.{name}.max"),
                        a.map_or(Value::Null, |r| json!(r.1 as f64)),
                        b.map_or(Value::Null, |r| json!(r.max)),
                    );
                }
            }
            let extra: Vec<String> = by_row
                .keys()
                .filter(|r| **r as usize >= pob_rows.len())
                .map(|r| format!("{kind}|{r}"))
                .collect();
            out.bundle_only(T, extra);
        };
    let nodes = rows.map_or_else(Vec::new, |t| {
        t.passives
            .iter()
            .map(|p| (p.row, p.id.as_str(), p.stats.as_slice()))
            .collect()
    });
    check_rows("node", &pob.nodes, nodes);
    let additions = rows.map_or_else(Vec::new, |t| {
        t.additions
            .iter()
            .map(|p| (p.row, p.id.as_str(), p.stats.as_slice()))
            .collect()
    });
    check_rows("addition", &pob.additions, additions);

    // The mapping.
    let Some(l) = luts else {
        out.pob_only(T, "mapping");
        return;
    };
    out.check(
        T,
        "mapping",
        "additions",
        json!(l.additions),
        json!(pob.additions_count),
    );
    out.check(
        T,
        "mapping",
        "node_count",
        json!(l.node_count),
        json!(pob.node_count),
    );
    out.check(
        T,
        "mapping",
        "notable_count",
        json!(l.notable_count),
        json!(pob.notable_count),
    );
    let ours: BTreeMap<String, (u32, u32)> = l
        .nodes
        .iter()
        .map(|n| (n.node.to_string(), (n.index, n.size)))
        .collect();
    for (node, p) in &pob.node_index {
        let key = format!("index|{node}");
        match ours.get(node) {
            Some((index, size)) => {
                out.check(T, &key, "index", json!(index), json!(p.index));
                out.check(T, &key, "size", json!(size), json!(p.size));
            }
            None => out.pob_only(T, &key),
        }
    }
    out.bundle_only(
        T,
        ours.keys()
            .filter(|n| !pob.node_index.contains_key(*n))
            .map(|n| format!("index|{n}"))
            .collect::<Vec<_>>(),
    );
    let ids: BTreeMap<(String, String), u32> = l
        .local_to_global
        .iter()
        .map(|i| ((i.jewel.to_string(), i.local.to_string()), i.global))
        .collect();
    let mut seen = BTreeSet::new();
    for (jewel, pairs) in &pob.local_to_global {
        for (local, global) in pairs {
            let k = (jewel.clone(), local.clone());
            let key = format!("id|{jewel}|{local}");
            match ids.get(&k) {
                Some(g) => out.check(T, &key, "global", json!(g), json!(global)),
                None => out.pob_only(T, &key),
            }
            seen.insert(k);
        }
    }
    out.bundle_only(
        T,
        ids.keys()
            .filter(|k| !seen.contains(*k))
            .map(|(j, l)| format!("id|{j}|{l}"))
            .collect::<Vec<_>>(),
    );

    // The jewel types and their results files.
    for (jewel, p) in &pob.jewels {
        let key = format!("jewel|{jewel}");
        let Some(j) = l.jewels.iter().find(|j| j.jewel.to_string() == *jewel) else {
            out.pob_only(T, &key);
            continue;
        };
        out.check(T, &key, "name", json!(j.name), json!(p.name));
        out.check(T, &key, "seed_min", json!(j.seed_min), json!(p.seed_min));
        out.check(T, &key, "seed_max", json!(j.seed_max), json!(p.seed_max));
        out.check(
            T,
            &key,
            "adler32",
            adlers.get(&j.file).map_or(Value::Null, |a| json!(a)),
            json!(p.adler32),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::adler32;

    /// RFC 1950's example and the empty input.
    #[test]
    fn adler32_should_match_the_rfc() {
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }
}

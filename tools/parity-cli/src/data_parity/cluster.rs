//! `cluster_jewels` (passive tree plan, T-i): the bundle's cluster jewels
//! (`poe1/cluster_jewels.json`) against PoB's (`data.clusterJewels`,
//! export format 9), both read from the same game tables (PoB's by
//! `v2.67.2:src/Export/Scripts/cluster.lua`).
//!
//! Rows, each keyed as written:
//! - `jewel|<base name>`: size index, node counts, the three index lists,
//!   the ring's size;
//! - `skill|<base name>|<passive ID>`: each small passive of the jewel's
//!   size, its tag and name;
//! - `notable|<name>`: a notable's place in the sort order (PoB's 1-based
//!   stat row);
//! - `keystone|<name>`: on both sides or one;
//! - `proxy|<node>`: a slot's start indices.
//!
//! A row on PoB's side only is a difference; on the bundle's only, a
//! bundle-only row. A bundle without the file has PoB's rows only.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::Bundle;
use serde_json::json;

use super::compare::Comparison;
use super::pob::Export;

/// The table's name.
const T: &str = "cluster_jewels";

/// Compares the table (module docs).
pub fn compare_cluster_jewels(out: &mut Comparison, bundle: &Bundle, export: &Export) {
    let pob = &export.cluster_jewels;
    let data = bundle.poe1.as_ref().and_then(|p| p.cluster_jewels.as_ref());
    let empty = loom_data::bundle::poe1::ClusterJewels::default();
    let data = data.unwrap_or(&empty);
    let passives: BTreeMap<i64, (&str, &str)> = bundle
        .passives
        .iter()
        .flat_map(|p| &p.passives)
        .map(|x| (x.node, (x.id.as_str(), x.name.as_str())))
        .collect();
    let base_names: BTreeMap<&str, &str> = bundle
        .base_items
        .iter()
        .map(|b| (b.id.as_str(), b.name.as_str()))
        .collect();

    // Jewels and their small passives.
    let mut seen = BTreeSet::new();
    for j in &data.jewels {
        let name = base_names.get(j.base.as_str()).copied().unwrap_or(&j.base);
        let key = format!("jewel|{name}");
        let Some(p) = pob.jewels.get(name) else {
            out.bundle_only(T, [key]);
            continue;
        };
        seen.insert(name.to_owned());
        out.check(T, &key, "size_index", json!(j.size), json!(p.size_index));
        out.check(
            T,
            &key,
            "nodes",
            json!([j.min_nodes, j.max_nodes]),
            json!([p.min_nodes, p.max_nodes]),
        );
        out.check(T, &key, "small", json!(j.small_indices), json!(p.small));
        out.check(
            T,
            &key,
            "notable",
            json!(j.notable_indices),
            json!(p.notable),
        );
        out.check(T, &key, "socket", json!(j.socket_indices), json!(p.socket));
        out.check(T, &key, "total", json!(j.total_indices), json!(p.total));
        let mut ours = BTreeSet::new();
        for s in data.skills.iter().filter(|s| s.size == j.size) {
            let (id, passive) = passives.get(&s.node).copied().unwrap_or_default();
            let key = format!("skill|{name}|{id}");
            ours.insert(id.to_owned());
            let Some(ps) = p.skills.get(id) else {
                out.bundle_only(T, [key]);
                continue;
            };
            out.check(T, &key, "tag", json!(s.tag), json!(ps.tag));
            // PoB marks a skill whose tag is retired.
            let shown = if s.tag.contains("old_do_not_use") {
                format!("{passive} (Legacy)")
            } else {
                passive.to_owned()
            };
            out.check(T, &key, "name", json!(shown), json!(ps.name));
        }
        for id in p.skills.keys().filter(|id| !ours.contains(*id)) {
            out.pob_only(T, &format!("skill|{name}|{id}"));
        }
    }
    for name in pob.jewels.keys().filter(|n| !seen.contains(*n)) {
        out.pob_only(T, &format!("jewel|{name}"));
    }

    // Notables' order and keystones, by name.
    let name_of = |node: i64| passives.get(&node).map_or("", |(_, n)| *n);
    let mut notables = BTreeSet::new();
    let mut keystones = BTreeSet::new();
    for s in &data.specials {
        let name = name_of(s.node);
        if s.keystone {
            keystones.insert(name);
            continue;
        }
        notables.insert(name);
        let key = format!("notable|{name}");
        match pob.notable_sort_order.get(name) {
            Some(order) => out.check(T, &key, "order", json!(s.order + 1), json!(order)),
            None => out.bundle_only(T, [key]),
        }
    }
    for name in pob
        .notable_sort_order
        .keys()
        .filter(|n| !notables.contains(n.as_str()))
    {
        out.pob_only(T, &format!("notable|{name}"));
    }
    let pob_keystones: BTreeSet<&str> = pob.keystones.iter().map(String::as_str).collect();
    for k in &keystones {
        let key = format!("keystone|{k}");
        if pob_keystones.contains(k) {
            out.check(T, &key, "keystone", json!(true), json!(true));
        } else {
            out.bundle_only(T, [key]);
        }
    }
    for k in pob_keystones.difference(&keystones) {
        out.pob_only(T, &format!("keystone|{k}"));
    }

    // Each proxy's start indices.
    let mut proxies = BTreeSet::new();
    for s in &data.slots {
        let proxy = s.proxy.to_string();
        let key = format!("proxy|{proxy}");
        match pob.orbit_offsets.get(&proxy) {
            Some(p) => out.check(T, &key, "start", json!(s.start_indices), json!(p)),
            None => out.bundle_only(T, [key]),
        }
        proxies.insert(proxy);
    }
    for proxy in pob.orbit_offsets.keys().filter(|p| !proxies.contains(*p)) {
        out.pob_only(T, &format!("proxy|{proxy}"));
    }
}

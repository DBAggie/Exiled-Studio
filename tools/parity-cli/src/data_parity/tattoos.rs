//! `tattoos` (Phase 6 P6-11av): the bundle's passive skill overrides
//! (`poe1/tattoos.json`) against PoB's `Data/TattooPassives.lua` (its own
//! export of the same game table; export format 16), by name (what a
//! spec's `<Override>` names, PoB's key):
//! - `id`, `override_type`, and whether it is a keystone tattoo;
//! - each stat's value against PoB's range (a keystone tattoo's stats are
//!   its passive's, put on the override by `bundle`, as PoB's exporter
//!   reads the passive's row).
//!
//! A name on PoB's side only is a difference. The bundle's overrides PoB's
//! exporter skips (an empty name, `[DNT ...]`, "of the Test",
//! `DisplayRandomKeystone`) are bundle-only rows.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::Bundle;
use serde_json::{Value, json};

use super::compare::Comparison;
use super::pob::Export;

/// The table's name.
const T: &str = "tattoos";

/// Compares the table (module docs).
pub fn compare_tattoos(out: &mut Comparison, bundle: &Bundle, export: &Export) {
    let Some(pob) = &export.tattoos else {
        return;
    };
    let ours = bundle.poe1.as_ref().and_then(|p| p.tattoos.as_ref());
    let mut seen = BTreeSet::new();
    for (name, p) in pob {
        let key = format!("override|{name}");
        let Some(o) = ours.and_then(|t| t.named(name)) else {
            out.pob_only(T, &key);
            continue;
        };
        seen.insert(o.id.as_str());
        out.check(T, &key, "id", json!(o.id), json!(p.id));
        out.check(
            T,
            &key,
            "override_type",
            json!(o.kind),
            json!(p.override_type),
        );
        out.check(
            T,
            &key,
            "keystone",
            json!(o.kind == "KeystoneTattoo"),
            json!(p.keystone),
        );
        let values: BTreeMap<&str, i64> =
            o.stats.iter().map(|s| (s.stat.as_str(), s.value)).collect();
        let names: BTreeSet<&str> = values
            .keys()
            .copied()
            .chain(p.stats.keys().map(String::as_str))
            .collect();
        for stat in names {
            let a = values.get(stat).map_or(Value::Null, |v| json!(*v as f64));
            let b = p.stats.get(stat);
            out.check(
                T,
                &key,
                format!("stat.{stat}.min"),
                a.clone(),
                b.map_or(Value::Null, |r| json!(r.min)),
            );
            out.check(
                T,
                &key,
                format!("stat.{stat}.max"),
                a,
                b.map_or(Value::Null, |r| json!(r.max)),
            );
        }
    }
    let extra: Vec<String> = ours
        .map(|t| {
            t.overrides
                .iter()
                .filter(|o| !seen.contains(o.id.as_str()))
                .map(|o| format!("override-id|{}", o.id))
                .collect()
        })
        .unwrap_or_default();
    out.bundle_only(T, extra);
}

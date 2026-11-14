//! `mod_affixes` and `base_requirements` (UX sprint B-2, data schema 4;
//! `dev/active/ux-improvements/b2-item-data-plan.md`): the affix names and
//! levels the item tooltip's Alt view shows, and the requirement line, in
//! the bundle against PoB v2.67.2's data.
//!
//! `mod_affixes`: each mod of the keyed `data.itemMods` files, by mod ID
//! (the key `Prefix:`/`Suffix:` spec lines name), its `affix` and `level`.
//! Bundle mods PoB does not export are not listed (monster, area, and
//! other domains; `data_parity::NOT_COMPARED`).
//!
//! `base_requirements`: each `data.itemBases` entry, matched by name as
//! `bases` is, its `req`: the level PoB's exporter derived from the drop
//! level and the implicits' levels, and the attributes. Several game bases
//! under one name (an old and a new quiver) compare as one when their
//! values agree, else the row differs. Bundle bases PoB does not have are
//! not listed (currency, gems, and the like).

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::properties::PropertyTables;
use loom_data::bundle::{AFFIX_SCHEMA, BaseItem, Bundle, ModEntry};
use serde_json::{Value, json};

use super::compare::{Comparison, num, pob_name};
use super::pob::Export;

/// The bundle's required level in PoB's form
/// (`v2.67.2:src/Export/Scripts/bases.lua:277-293`): the drop level when
/// above 4 for a weapon or armour and above 2 for a flask, else 1; raised
/// to `floor(level * 0.8)` of each implicit; written only when above 1.
fn required_level(
    drop_level: i64,
    weapon_or_armour: bool,
    flask: bool,
    implicit_levels: impl IntoIterator<Item = i64>,
) -> Option<i64> {
    let mut level = 1;
    if weapon_or_armour && drop_level > 4 {
        level = drop_level;
    }
    if flask && drop_level > 2 {
        level = drop_level;
    }
    for implicit in implicit_levels {
        // Lua's math.floor(mod.Level * 0.8), in the same f64 arithmetic.
        #[expect(clippy::cast_precision_loss, reason = "game levels")]
        #[expect(clippy::cast_possible_truncation, reason = "floored game levels")]
        let scaled = (implicit as f64 * 0.8).floor() as i64;
        level = level.max(scaled);
    }
    (level > 1).then_some(level)
}

/// An integer field PoB writes only when not 0.
fn written(x: i64) -> Value {
    #[expect(clippy::cast_precision_loss, reason = "game integers")]
    if x == 0 { Value::Null } else { num(x as f64) }
}

/// A PoB number field, absent as `null`.
fn pob(x: Option<f64>) -> Value {
    x.map_or(Value::Null, num)
}

/// Whether the bundle carries affix names, levels, and requirements.
///
/// # Errors
/// A message for a bundle before data schema 4.
pub fn check_schema(bundle: &Bundle) -> Result<(), String> {
    if bundle.manifest.data_schema < AFFIX_SCHEMA {
        return Err(format!(
            "the bundle is data schema {}; mod affixes and base requirements need {AFFIX_SCHEMA}",
            bundle.manifest.data_schema
        ));
    }
    Ok(())
}

/// Compares `mod_affixes`.
pub fn compare_mod_affixes(out: &mut Comparison, bundle: &Bundle, export: &Export) {
    const T: &str = "mod_affixes";
    let mods: BTreeMap<&str, &ModEntry> = bundle.mods.iter().map(|m| (m.id.as_str(), m)).collect();
    for (id, pob_mod) in &export.mods {
        if pob_mod.conflict {
            // Two of PoB's files disagree: no single PoB value to compare.
            out.check(T, id, "pob_tables", Value::Null, json!(pob_mod.tables));
        }
        let Some(ours) = mods.get(id.as_str()) else {
            out.pob_only(T, id);
            continue;
        };
        out.check(T, id, "affix", json!(ours.name), json!(pob_mod.affix));
        #[expect(clippy::cast_precision_loss, reason = "game levels")]
        let level = ours.level.map_or(Value::Null, |l| num(l as f64));
        out.check(T, id, "level", level, pob(pob_mod.level));
    }
}

/// Compares `base_requirements`.
pub fn compare_base_requirements(
    out: &mut Comparison,
    bundle: &Bundle,
    properties: &PropertyTables,
    export: &Export,
) {
    const T: &str = "base_requirements";
    let weapon_or_armour: BTreeSet<&str> = properties
        .weapon_types
        .iter()
        .map(|w| w.base_item.as_str())
        .chain(properties.armour_types.iter().map(|a| a.base_item.as_str()))
        .collect();
    let mod_levels: BTreeMap<&str, i64> = bundle
        .mods
        .iter()
        .filter_map(|m| Some((m.id.as_str(), m.level?)))
        .collect();
    // Candidates by PoB's name; PoB skips Royale bases (bases.lua:342).
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, base) in bundle.base_items.iter().enumerate() {
        if !base.id.contains("Royale") {
            by_name.entry(pob_name(&base.name)).or_default().push(i);
        }
    }
    // A base's values in PoB's form: flask, level, str, dex, int.
    let pob_form = |base: &BaseItem| -> [Value; 5] {
        // PoB's exporter tests the game's Flasks table (bases.lua:283); the
        // bundle's flask classes all end in "Flask".
        let flask = base
            .item_class
            .as_deref()
            .is_some_and(|c| c.ends_with("Flask"));
        let level = required_level(
            base.drop_level.unwrap_or(0),
            weapon_or_armour.contains(base.id.as_str()),
            flask,
            base.implicits
                .iter()
                .filter_map(|m| mod_levels.get(m.as_str()).copied()),
        );
        let attributes = base.requirements.unwrap_or_default();
        #[expect(clippy::cast_precision_loss, reason = "game levels")]
        let level = level.map_or(Value::Null, |l| num(l as f64));
        [
            json!(flask),
            level,
            written(attributes.strength),
            written(attributes.dexterity),
            written(attributes.intelligence),
        ]
    };
    for (name, req) in &export.base_requirements {
        let candidates = by_name.get(name).map(Vec::as_slice).unwrap_or_default();
        let Some((&first, rest)) = candidates.split_first() else {
            out.pob_only(T, name);
            continue;
        };
        let ours = pob_form(&bundle.base_items[first]);
        // Several game bases under one name (an old and a new quiver):
        // PoB holds one, so they compare as one when their values agree.
        if rest
            .iter()
            .any(|&i| pob_form(&bundle.base_items[i]) != ours)
        {
            let ids = candidates
                .iter()
                .map(|&i| json!(bundle.base_items[i].id))
                .collect();
            out.check(T, name, "row", Value::Array(ids), json!("one base"));
            continue;
        }
        let theirs = [
            json!(req.flask),
            pob(req.level),
            pob(req.str),
            pob(req.dex),
            pob(req.int),
        ];
        for ((field, ours), theirs) in ["flask", "level", "str", "dex", "int"]
            .into_iter()
            .zip(ours)
            .zip(theirs)
        {
            out.check(T, name, field, ours, theirs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::required_level;

    #[test]
    fn required_level_should_follow_pobs_exporter() {
        // Weapons and armour: the drop level above 4.
        assert_eq!(required_level(4, true, false, []), None);
        assert_eq!(required_level(5, true, false, []), Some(5));
        // Flasks: above 2.
        assert_eq!(required_level(3, false, true, []), Some(3));
        // Other bases: 1 unless an implicit raises it.
        assert_eq!(required_level(60, false, false, []), None);
        // floor(level * 0.8) of each implicit: 68 * 0.8 = 54.4.
        assert_eq!(required_level(5, false, false, [68, 10]), Some(54));
        assert_eq!(required_level(70, true, false, [68]), Some(70));
    }
}

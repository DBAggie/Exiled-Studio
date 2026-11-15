//! An item card's values (B-2 item evidence, E5): the item's defences,
//! block, weapon damage, rate, and critical strike chance, as the
//! calculation computes them ([`loom_calc::item_values`]), so the card
//! shows what the game's tooltip shows and what the calculation uses.
//!
//! Only the item's own lines act on these values, so it is calculated
//! alone: a new level 1 build holding the item in a slot that fits its
//! class (a weapon in the main hand, a shield in the off hand, an armour
//! piece in its own slot), with the item's range ends from the build.

use loom_domain::{Build, Item, ItemsEntry};
use loom_pipeline::RangeEnds;

use crate::data::GameData;
use crate::items::{ONE_HANDED, TWO_HANDED};
use crate::{api, items};

/// The slot an item of `class` is calculated in, if it has values.
fn slot_for(class: &str) -> Option<&'static str> {
    match class {
        "Helmet" => Some("Helmet"),
        "Gloves" => Some("Gloves"),
        "Boots" => Some("Boots"),
        "Body Armour" => Some("Body Armour"),
        "Shield" => Some("Weapon 2"),
        c if ONE_HANDED.contains(&c) || TWO_HANDED.contains(&c) => Some("Weapon 1"),
        _ => None,
    }
}

/// The values of `item`, whose range ends are in `ends`; none for an item
/// without such values (a ring, a flask, a jewel), an item whose base the
/// data does not know, or when the calculation fails.
#[must_use]
pub fn values(item: &Item, ends: &RangeEnds, data: &GameData) -> Option<api::CardValues> {
    let class = crate::view::item_class(data.bundle, item)?;
    let slot = slot_for(&class)?;
    let build = alone(item, slot, data)?;
    let ends: RangeEnds = ends
        .get(&item.id)
        .map(|e| [(item.id, e.clone())].into_iter().collect())
        .unwrap_or_default();
    let options = loom_pipeline::options(&build, data.data, &ends, None, None);
    let v = loom_pipeline::item_values(&build, data.bundle, &options).ok()?;
    let weapon = v.weapon.as_ref();
    Some(api::CardValues {
        defences: v
            .defences
            .iter()
            .map(|(_, d, value)| api::CardDefence {
                defence: d.name().to_owned(),
                value: *value,
            })
            .collect(),
        block: v.block,
        damage: weapon
            .map(|w| {
                w.damage
                    .iter()
                    .map(|(t, min, max)| api::CardDamage {
                        damage_type: t.name().to_owned(),
                        min: *min,
                        max: *max,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        attack_rate: weapon.map(|w| w.attack_rate),
        crit_chance: weapon.map(|w| w.crit_chance),
        partial: !v.reasons.is_empty(),
    })
}

/// A level 1 build with `item` alone, in `slot`.
fn alone(item: &Item, slot: &str, data: &GameData) -> Option<Build> {
    let mut build =
        loom_pipeline::read_pob(&crate::session::NEW_BUILD.replace("{tree}", "")).ok()?;
    loom_pipeline::resolve(data.resolver, data.data, &mut build).ok()?;
    build.items.entries.push(ItemsEntry::Item(item.clone()));
    for entry in build.items.entries.typed_mut() {
        if let ItemsEntry::Set(set) = entry {
            items::put(set, slot, Some(item.id));
        }
    }
    Some(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_with_values_should_be_calculated_in_a_slot_that_fits() {
        assert_eq!(slot_for("Two Hand Mace"), Some("Weapon 1"));
        assert_eq!(slot_for("Rune Dagger"), Some("Weapon 1"));
        assert_eq!(slot_for("Shield"), Some("Weapon 2"));
        assert_eq!(slot_for("Body Armour"), Some("Body Armour"));
        assert_eq!(slot_for("Quiver"), None);
        assert_eq!(slot_for("Ring"), None);
        assert_eq!(slot_for("LifeFlask"), None);
    }
}

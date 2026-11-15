//! Adding an item (B-2 item evidence, E3): the game's item copy or PoB's
//! item text becomes a new item of the build, read and resolved as an
//! imported one, and is put in a slot of the active item set if asked.
//!
//! A game copy is turned into PoB's item text first
//! ([`loom_serialization::legacy::game_copy::poe1`]), as PoB's paste does.
//! The item must have a base the data knows; a slot must fit the base's item
//! class. A two-handed weapon put in `Weapon 1` empties `Weapon 2` (a bow
//! keeps a quiver), as the game does; an off-hand item is refused while the
//! main hand holds a two-handed weapon.

use loom_domain::{
    Attributes, Build, Item, ItemId, ItemPart, ItemSet, ItemsEntry, Ordered, SlotEntry, SlotId,
};
use loom_pipeline::RangeEnds;
use loom_serialization::legacy::game_copy::poe1 as game_copy;
use loom_serialization::legacy::pob_item;

use crate::data::GameData;
use crate::error::{AppError, Code};

/// The longest text an add takes (64 KiB): a game copy is under 4 KiB.
pub const MAX_TEXT: usize = 64 * 1024;

/// A build and its range ends after an add, and the new item's ID.
pub struct Added {
    /// The build.
    pub build: Build,
    /// Its range ends.
    pub ends: RangeEnds,
    /// The new item.
    pub item: ItemId,
}

/// One-handed weapon classes (the bundle's `item_class`).
pub(crate) const ONE_HANDED: [&str; 9] = [
    "One Hand Axe",
    "One Hand Mace",
    "One Hand Sword",
    "Thrusting One Hand Sword",
    "Claw",
    "Dagger",
    "Rune Dagger",
    "Wand",
    "Sceptre",
];

/// Two-handed weapon classes.
pub(crate) const TWO_HANDED: [&str; 7] = [
    "Two Hand Axe",
    "Two Hand Mace",
    "Two Hand Sword",
    "Staff",
    "Warstaff",
    "Bow",
    "FishingRod",
];

const FLASKS: [&str; 4] = ["LifeFlask", "ManaFlask", "HybridFlask", "UtilityFlask"];

/// Whether an item of `class` goes in `slot` (a swap slot as its main
/// slot).
#[must_use]
pub fn fits(class: &str, slot: &str) -> bool {
    let slot = slot.strip_suffix(" Swap").unwrap_or(slot);
    match slot {
        "Weapon 1" => ONE_HANDED.contains(&class) || TWO_HANDED.contains(&class),
        "Weapon 2" => ONE_HANDED.contains(&class) || matches!(class, "Shield" | "Quiver"),
        "Ring 1" | "Ring 2" => class == "Ring",
        "Flask 1" | "Flask 2" | "Flask 3" | "Flask 4" | "Flask 5" => FLASKS.contains(&class),
        "Helmet" | "Body Armour" | "Gloves" | "Boots" | "Amulet" | "Belt" => class == slot,
        _ => false,
    }
}

/// Adds the item `text` describes to `build`; with `slot`, puts it there in
/// the active item set.
///
/// # Errors
/// `APP_ADD_ITEM`: the text is too long, not an item's, of a base the data
/// does not know, or the slot does not fit; `APP_ITEM_SET` when a slot is
/// named and the build has no item set.
pub fn add(
    build: &Build,
    ends: &RangeEnds,
    text: &str,
    slot: Option<&str>,
    data: &GameData,
) -> Result<Added, AppError> {
    if text.len() > MAX_TEXT {
        return Err(AppError::new(
            Code::AddItem,
            format!("an item's text is at most {MAX_TEXT} bytes"),
        ));
    }
    let pob_text = if game_copy::is_game_copy(text) {
        game_copy::convert(text)
            .map_err(|e| AppError::new(Code::AddItem, e.to_string()))?
            .text
    } else {
        text.to_owned()
    };
    let read = pob_item::read(&pob_text);
    if read.rarity.is_none() {
        return Err(AppError::new(Code::AddItem, "the text is not an item"));
    }
    let mut body = pob_item::to_domain(&read);
    let id = ItemId(build.items.items().map(|i| i.id.0).max().unwrap_or(0) + 1);
    let mut item = Item {
        id,
        body: body.clone(),
        layout: Ordered::new(),
        text_spacing: Vec::new(),
        attributes: Attributes::new(),
    };
    item.layout.push(ItemPart::Body);
    let class = crate::view::item_class(data.bundle, &item)
        .ok_or_else(|| AppError::new(Code::AddItem, "the data does not know the item's base"))?;
    let item_ends = loom_pipeline::resolve_item(data.resolver, data.data, &mut body);
    item.body = body;

    let mut out = build.clone();
    let last_item = out
        .items
        .entries
        .typed()
        .enumerate()
        .filter(|(_, e)| matches!(e, ItemsEntry::Item(_)))
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0);
    out.items
        .entries
        .insert_typed(last_item, ItemsEntry::Item(item));
    if let Some(slot) = slot {
        equip(&mut out, data, id, &class, slot)?;
    }
    let ends = loom_pipeline::with_item_ends(ends, id, item_ends);
    Ok(Added {
        build: out,
        ends,
        item: id,
    })
}

/// Puts item `id` of `class` in `slot` of the active item set.
fn equip(
    build: &mut Build,
    data: &GameData,
    id: ItemId,
    class: &str,
    slot: &str,
) -> Result<(), AppError> {
    if !fits(class, slot) {
        return Err(AppError::new(
            Code::AddItem,
            format!("a {class} does not go in {slot}"),
        ));
    }
    let set_id = loom_calc::poe1::collect::active_item_set(build)
        .map(|s| s.id)
        .ok_or_else(|| AppError::new(Code::ItemSet, "the build has no item set"))?;
    let swap = if slot.ends_with(" Swap") { " Swap" } else { "" };
    let (main_slot, off_slot) = (format!("Weapon 1{swap}"), format!("Weapon 2{swap}"));
    let class_in = |build: &Build, set: &ItemSet, s: &str| {
        set.slots
            .typed()
            .find(|e| e.slot.as_str() == s)
            .and_then(|e| e.item)
            .and_then(|i| build.items.item(i))
            .and_then(|i| crate::view::item_class(data.bundle, i))
    };
    let set = build
        .items
        .sets()
        .find(|s| s.id == set_id)
        .ok_or_else(|| AppError::new(Code::ItemSet, "the build has no item set"))?;
    let main = class_in(build, set, &main_slot);
    let off = class_in(build, set, &off_slot);
    let mut empty_off = false;
    if slot == off_slot
        && let Some(main) = main.as_deref()
        && TWO_HANDED.contains(&main)
        && !(main == "Bow" && class == "Quiver")
    {
        return Err(AppError::new(
            Code::AddItem,
            format!("{main_slot} holds a two-handed weapon"),
        ));
    }
    if slot == main_slot && TWO_HANDED.contains(&class) {
        empty_off = off
            .as_deref()
            .is_some_and(|o| !(class == "Bow" && o == "Quiver"));
    }
    if class == "Quiver" && main.as_deref().is_some_and(|m| m != "Bow") {
        return Err(AppError::new(
            Code::AddItem,
            "a quiver goes with a bow only",
        ));
    }
    for entry in build.items.entries.typed_mut() {
        let ItemsEntry::Set(set) = entry else {
            continue;
        };
        if set.id != set_id {
            continue;
        }
        put(set, slot, Some(id));
        if empty_off {
            put(set, &off_slot, None);
        }
    }
    Ok(())
}

pub(crate) fn put(set: &mut ItemSet, slot: &str, item: Option<ItemId>) {
    if let Some(e) = set.slots.typed_mut().find(|e| e.slot.as_str() == slot) {
        e.item = item;
    } else if item.is_some() {
        set.slots.push(SlotEntry {
            slot: SlotId::new(slot.to_owned()),
            item,
            attributes: Attributes::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_should_take_the_classes_the_game_allows() {
        assert!(fits("Two Hand Mace", "Weapon 1"));
        assert!(fits("Two Hand Mace", "Weapon 1 Swap"));
        assert!(!fits("Two Hand Mace", "Weapon 2"));
        assert!(fits("Shield", "Weapon 2"));
        assert!(fits("Quiver", "Weapon 2"));
        assert!(fits("Claw", "Weapon 2"));
        assert!(fits("Ring", "Ring 2"));
        assert!(fits("UtilityFlask", "Flask 5"));
        assert!(fits("Body Armour", "Body Armour"));
        assert!(!fits("Helmet", "Gloves"));
        assert!(!fits("Jewel", "Ring 1"));
        assert!(!fits("Ring", "Ring 3"));
    }
}

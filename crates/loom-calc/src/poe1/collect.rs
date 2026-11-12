//! Collecting a Path of Exile build into the modifier store (Phase 3
//! plan, steps 6 and 8): the character's base values, the active spec's
//! passives and mastery effects, the items in use, and a reason for
//! everything the engine does not model yet (decision 1).
//!
//! What PoB v2.67.2 does, and what we follow (`Modules/CalcSetup.lua`):
//! - attributes start at the class's base values (`Str`, `Dex`, `Int`
//!   BASE from the tree's class data; the bundle's characters table, equal
//!   to PoB's in P3-2's data parity);
//! - life is `38 + life_per_level * level` and mana `34 +
//!   mana_per_level * level`, the level clamped to 1..100 (PoB writes 38
//!   and 34; the bundle carries them per class as `base_life` and
//!   `base_mana`, the per-level values in `Character.ot`);
//! - items: the active item set's slots of the weapon set in use, then
//!   jewels in allocated sockets; flasks and tinctures only act while
//!   active; an abyss jewel in an item is scaled by its parent.
//!
//! Not modelled yet, each a reason on every computed field: enabled gems
//! (auras, buffs, reservation: P3-9a, P3-12), config inputs (P3-12),
//! bandit and pantheon choices other than none, item base properties
//! (armour pieces and weapons: P3-7, P3-10), active flasks, abyssal
//! sockets, and slots other than the known ones.

use loom_domain::{
    Build, GameSpecific, Item, ItemId, ItemSet, LineKind, PassiveSpec, StatId, StatLine, StatValue,
};

use crate::ingest::Ingest;
use crate::input::{CalcInput, StatAmount};
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, UnsupportedReason};
use crate::store::{Conditions, Effect};

/// The value of an untyped attribute.
fn attribute<'a>(attributes: &'a loom_domain::Attributes, name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

/// A base effect from the rules.
fn base(quantity: Quantity, value: i64, source: Source) -> Effect {
    #[expect(
        clippy::cast_precision_loss,
        reason = "game integers are far below 2^53, where i64 to f64 is exact"
    )]
    let value = value as f64;
    Effect {
        quantity,
        op: Op::Base,
        value,
        conditions: Conditions::none(),
        source,
    }
}

/// Adds the build's content to `ingest`.
pub fn collect(input: &CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let build = input.build;
    character(input, ingest);
    choices(build, ingest);
    let spec = active_spec(build);
    if let Some(spec) = spec {
        passives(input, spec, ingest);
    }
    items(build, spec, ingest);
    gems(build, ingest);
    config(build, ingest);
}

/// The character's base values: class attributes, life and mana with
/// their per-level growth.
fn character(input: &CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let build = input.build;
    // PoB: `m_max(1, m_min(100, build.characterLevel))`; a build without a
    // level is level 1 (`Build.level` defaults to 1).
    let level = build
        .character
        .level
        .as_ref()
        .map_or(1, |l| *l.value())
        .clamp(1, 100);
    let class = build.character.class.as_ref().map_or("", |c| c.as_str());
    match input.data.character(class) {
        Some(c) => {
            ingest.effect(base(Quantity::Strength, c.strength, Source::Character));
            ingest.effect(base(Quantity::Dexterity, c.dexterity, Source::Character));
            ingest.effect(base(
                Quantity::Intelligence,
                c.intelligence,
                Source::Character,
            ));
            ingest.effect(base(Quantity::Life, c.base_life, Source::Character));
            ingest.effect(base(Quantity::Mana, c.base_mana, Source::Character));
        }
        None => ingest.indeterminate(IndeterminateReason::MissingData {
            table: "characters",
            key: class.to_owned(),
        }),
    }
    for (stat, quantity, rule) in [
        ("life_per_level", Quantity::Life, "life-per-level"),
        ("mana_per_level", Quantity::Mana, "mana-per-level"),
    ] {
        match input.data.character_constant(stat) {
            Some(per) => ingest.effect(base(quantity, per * i64::from(level), Source::Rule(rule))),
            None => ingest.indeterminate(IndeterminateReason::MissingData {
                table: "character_constants",
                key: stat.to_owned(),
            }),
        }
    }
}

/// Bandit and pantheon: `None` (or absent) has no effect on the slice
/// (PoB adds one passive point for no bandit); any other choice is not
/// modelled yet.
fn choices(build: &Build, ingest: &mut Ingest<'_>) {
    let GameSpecific::Poe1(c) = build.game_specific() else {
        return;
    };
    for (choice, value) in [
        ("bandit", &c.bandit),
        ("pantheon-major", &c.pantheon_major),
        ("pantheon-minor", &c.pantheon_minor),
    ] {
        if let Some(v) = value
            && v != "None"
        {
            ingest.reason(UnsupportedReason::UnmodelledChoice {
                choice,
                value: v.clone(),
            });
        }
    }
}

/// The active spec: `Tree.activeSpec` (default 1), else the first typed
/// one (as `calc-survey`).
#[must_use]
pub fn active_spec(build: &Build) -> Option<&PassiveSpec> {
    let p = &build.passives;
    let id = p.active.map_or(1, |s| s.0);
    p.specs
        .typed()
        .find(|s| s.id.0 == id)
        .or_else(|| p.specs.typed().next())
}

/// Stat amounts as stat lines with exact values.
fn lines(stats: &[StatAmount]) -> Vec<StatLine> {
    stats
        .iter()
        .map(|s| StatLine {
            stat: StatId::new(s.stat.as_str()),
            value: StatValue::Exact {
                min: s.value,
                max: s.value,
            },
        })
        .collect()
}

/// The bit PoB sets on the nodes of a cluster jewel's subgraph.
const CLUSTER_NODE_BIT: u32 = 0x1_0000;

/// The allocated nodes and chosen mastery effects of `spec`, after the
/// tree version check.
fn passives(input: &CalcInput<'_>, spec: &PassiveSpec, ingest: &mut Ingest<'_>) {
    let nodes = spec.nodes.value();
    let masteries = spec.mastery_effects.value();
    if nodes.is_empty() && masteries.is_empty() {
        return;
    }
    match &input.options.tree_version {
        None => ingest.reason(UnsupportedReason::UncheckedTreeVersion),
        Some(data) => {
            let build = spec.tree_version.as_ref().map_or("", |v| v.as_str());
            if build != data.as_str() {
                ingest.indeterminate(IndeterminateReason::TreeVersion {
                    build: build.to_owned(),
                    data: data.as_str().to_owned(),
                });
            }
        }
    }
    for node in nodes {
        // PoB numbers the nodes it generates from a socketed cluster jewel
        // with bit 16 set, above every tree node hash
        // (`Classes/PassiveSpec.lua` BuildSubgraph: "16: 1 (signal bit,
        // to prevent conflict with node hashes)"); their stats come from
        // the jewel, not the passives table.
        if node.0 & CLUSTER_NODE_BIT != 0 {
            ingest.reason(UnsupportedReason::UnmodelledPassive {
                source: Source::Passive { node: *node },
                what: "cluster-jewel-node",
            });
            continue;
        }
        match input.data.passive_stats(node.0) {
            Some(stats) => ingest.stats(&Source::Passive { node: *node }, &lines(stats)),
            None => ingest.indeterminate(IndeterminateReason::MissingData {
                table: "passives",
                key: node.0.to_string(),
            }),
        }
    }
    for m in masteries {
        // A mastery effect counts only on an allocated mastery (PoB keeps
        // `masteryEffects` of unallocated nodes in the XML but ignores
        // them).
        if !nodes.contains(&m.node) {
            continue;
        }
        match input.data.mastery_effect_stats(m.effect) {
            Some(stats) => ingest.stats(&Source::Passive { node: m.node }, &lines(stats)),
            None => ingest.indeterminate(IndeterminateReason::MissingData {
                table: "mastery_effects",
                key: m.effect.to_string(),
            }),
        }
    }
}

/// How an item in a slot acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    /// Its lines only (jewellery, jewels).
    Lines,
    /// Its lines, and base properties not modelled yet (armour pieces,
    /// weapons, shields, quivers).
    Properties,
    /// A flask or tincture: acts only while active.
    Flask,
    /// An abyss jewel in an item, scaled by the parent item.
    Abyssal,
    /// Another slot.
    Other,
}

fn slot_kind(slot: &str) -> SlotKind {
    if slot.contains("Abyssal Socket") {
        SlotKind::Abyssal
    } else if slot.starts_with("Flask") || slot.starts_with("Tincture") {
        SlotKind::Flask
    } else if slot.starts_with("Weapon")
        || matches!(slot, "Helmet" | "Body Armour" | "Gloves" | "Boots")
    {
        SlotKind::Properties
    } else if matches!(slot, "Amulet" | "Ring 1" | "Ring 2" | "Belt") {
        SlotKind::Lines
    } else {
        SlotKind::Other
    }
}

/// Whether a slot is in use for the item set's weapon set (PoB's
/// `slot.weaponSet`): `... Swap` slots belong to set 2, the other weapon
/// slots to set 1, every other slot to both.
fn slot_in_use(slot: &str, second_set: bool) -> bool {
    if slot.contains("Swap") {
        second_set
    } else if slot.starts_with("Weapon") {
        !second_set
    } else {
        true
    }
}

/// The active item set: `Items.activeItemSet` (default 1), else the first.
fn active_set(build: &Build) -> Option<&ItemSet> {
    let items = &build.items;
    items
        .active_set
        .and_then(|id| items.sets().find(|s| s.id == id))
        .or_else(|| items.sets().next())
}

/// The items in use: the active item set's slots, then jewels in
/// allocated sockets; each item once.
fn items(build: &Build, spec: Option<&PassiveSpec>, ingest: &mut Ingest<'_>) {
    let mut seen: Vec<ItemId> = Vec::new();
    if let Some(set) = active_set(build) {
        let second = attribute(&set.attributes, "useSecondWeaponSet") == Some("true");
        for entry in set.slots.typed() {
            let slot = entry.slot.as_str();
            let Some(id) = entry.item else { continue };
            // PoB writes `itemId="0"` for an empty slot.
            if id.0 == 0 || !slot_in_use(slot, second) || seen.contains(&id) {
                continue;
            }
            seen.push(id);
            let unmodelled = |what| UnsupportedReason::UnmodelledItem {
                item: id,
                slot: slot.to_owned(),
                what,
            };
            match slot_kind(slot) {
                SlotKind::Flask => {
                    if attribute(&entry.attributes, "active") == Some("true") {
                        ingest.reason(unmodelled("active-flask"));
                    }
                    continue;
                }
                SlotKind::Abyssal => {
                    ingest.reason(unmodelled("abyssal-socket"));
                    continue;
                }
                SlotKind::Properties => ingest.reason(unmodelled("base-properties")),
                SlotKind::Other => ingest.reason(unmodelled("slot")),
                SlotKind::Lines => {}
            }
            match build.items.item(id) {
                Some(item) => item_lines(item, ingest),
                None => ingest.reason(unmodelled("missing")),
            }
        }
    }
    let Some(spec) = spec else { return };
    let allocated = spec.nodes.value();
    for socket in spec.jewel_sockets.typed() {
        let Some(id) = socket.item else { continue };
        if id.0 == 0 || !allocated.contains(&socket.node) || seen.contains(&id) {
            continue;
        }
        seen.push(id);
        match build.items.item(id) {
            Some(item) => item_lines(item, ingest),
            None => ingest.reason(UnsupportedReason::UnmodelledItem {
                item: id,
                slot: format!("socket {}", socket.node),
                what: "missing",
            }),
        }
    }
}

/// An item's lines: modifier lines by their resolution; lines the import
/// did not read are reasons.
fn item_lines(item: &Item, ingest: &mut Ingest<'_>) {
    for (index, line) in item.body.lines.iter().enumerate() {
        match &line.kind {
            LineKind::Modifier(m) => ingest.item_line(item.id, index, &m.resolution),
            LineKind::Unrecognized | LineKind::Spec { known: false, .. } => {
                ingest.reason(UnsupportedReason::UnreadLine {
                    source: Source::Item {
                        item: item.id,
                        line: index,
                        adapted: None,
                    },
                });
            }
            LineKind::Header(_)
            | LineKind::Spec { known: true, .. }
            | LineKind::VariantBase { .. }
            | LineKind::Influence(_)
            | LineKind::Flag => {}
        }
    }
}

/// Every enabled gem of an enabled group of the active skill set: not
/// modelled yet.
fn gems(build: &Build, ingest: &mut Ingest<'_>) {
    let skills = &build.skills;
    let Some(set) = skills
        .active
        .and_then(|id| skills.sets.typed().find(|s| s.id == id))
        .or_else(|| skills.sets.typed().next())
    else {
        return;
    };
    if set.groups.opaque().next().is_some() {
        ingest.reason(UnsupportedReason::UnmodelledGem {
            source: Source::Rule("untyped-skill-group"),
        });
    }
    for (g, group) in set.groups.typed().enumerate() {
        if !group.enabled {
            continue;
        }
        for (i, gem) in group.gems.typed().enumerate() {
            if gem.enabled {
                ingest.reason(UnsupportedReason::UnmodelledGem {
                    source: Source::Gem { group: g, gem: i },
                });
            }
        }
    }
}

/// Every input of the active config set: not modelled yet.
fn config(build: &Build, ingest: &mut Ingest<'_>) {
    let config = &build.config;
    let Some(set) = config
        .active
        .and_then(|id| config.sets.typed().find(|s| s.id == id))
        .or_else(|| config.sets.typed().next())
    else {
        return;
    };
    if set.inputs.opaque().next().is_some() {
        ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: "<untyped>".to_owned(),
        });
    }
    // Every input, whatever its value: some of PoB's options default to
    // on, so even `false` can change a result.
    for input in set.inputs.typed() {
        ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: input.name.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_should_be_classified() {
        assert_eq!(slot_kind("Ring 1"), SlotKind::Lines);
        assert_eq!(slot_kind("Amulet"), SlotKind::Lines);
        assert_eq!(slot_kind("Belt"), SlotKind::Lines);
        assert_eq!(slot_kind("Helmet"), SlotKind::Properties);
        assert_eq!(slot_kind("Weapon 1 Swap"), SlotKind::Properties);
        assert_eq!(slot_kind("Flask 3"), SlotKind::Flask);
        assert_eq!(slot_kind("Belt Abyssal Socket 1"), SlotKind::Abyssal);
        assert_eq!(slot_kind("Ring 3"), SlotKind::Other);
        assert_eq!(slot_kind("Graft 1"), SlotKind::Other);
    }

    #[test]
    fn weapon_sets_should_pick_their_slots() {
        assert!(slot_in_use("Weapon 1", false));
        assert!(!slot_in_use("Weapon 1", true));
        assert!(slot_in_use("Weapon 2 Swap", true));
        assert!(!slot_in_use("Weapon 2 Swap", false));
        assert!(slot_in_use("Helmet", true));
    }
}

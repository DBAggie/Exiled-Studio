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
//!   active; an abyss jewel in an item is scaled by its parent;
//! - an armour piece (helmet, gloves, boots, body armour, a shield in the
//!   off hand) has its own defences from its base's ranges, its quality
//!   and base percentiles, and its local lines (`Classes/Item.lua`; P3-7);
//! - the character's base defences (maximum resistances, block and spell
//!   block maximums, base evasion) are character template stats, and
//!   the resistance penalty is the config's `resistancePenalty` or -60.
//!
//! Not modelled yet, each a reason on every computed field: enabled gems
//! (auras, buffs, reservation: P3-9a, P3-12), config inputs other than
//! the resistance penalty (P3-12), bandit and pantheon choices other than
//! none, weapons' base properties (P3-10), an armour piece's percentile
//! derived from its shown value, active flasks, abyssal sockets, and
//! slots other than the known ones.

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::{
    Build, ConfigSet, ConfigValue, GameSpecific, Item, ItemId, ItemSet, LineKind, NodeId,
    PassiveSpec, StatId, StatLine, StatValue,
};

use super::mapping::{PieceMapping, Poe1Mapping};
use super::{ArmourSlot, Defence, PieceValue, Poe1Quantity};
use crate::ingest::{Ingest, StatMapping};
use crate::input::{ArmourBase, CalcInput, ItemBase, PassiveLinks, StatAmount};
use crate::quantity::{DamageType, Op, Quantity};
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
    resistance_penalty(build, ingest);
    choices(build, ingest);
    let spec = active_spec(build);
    if let Some(spec) = spec {
        passives(input, spec, ingest);
    }
    items(input, spec, ingest);
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
    // The character's base defences are stats of the character template,
    // read through the mapping as an item's would be (PoB,
    // `Modules/CalcSetup.lua`: `FireResistMax` ... BASE
    // `base_maximum_all_resistances_%`, `BlockChanceMax` BASE
    // `maximum_block_%`, `SpellBlockChanceMax` BASE
    // `base_maximum_spell_block_%`, `Evasion` BASE `base_evasion_rating`).
    for stat in [
        "base_maximum_all_resistances_%",
        "maximum_block_%",
        "base_maximum_spell_block_%",
        "base_evasion_rating",
    ] {
        match input.data.character_constant(stat) {
            Some(value) => ingest.stat(
                &Source::Character,
                &StatLine {
                    stat: StatId::new(stat),
                    value: StatValue::Exact {
                        min: value,
                        max: value,
                    },
                },
            ),
            None => ingest.indeterminate(IndeterminateReason::MissingData {
                table: "character_constants",
                key: stat.to_owned(),
            }),
        }
    }
}

/// The resistance penalty PoB applies by default: the act 10 penalty,
/// -60% to fire, cold, lightning, and chaos resistance (PoB,
/// `Modules/CalcSetup.lua`: `<type>Resist` BASE
/// `env.configInput.resistancePenalty or -60`). The game data carries no
/// such row: it is a quest state, which PoB's config sets.
pub const RESISTANCE_PENALTY: i64 = -60;

/// The config input that sets the resistance penalty (PoB's
/// `resistancePenalty`: 0, -30, or -60).
pub const RESISTANCE_PENALTY_INPUT: &str = "resistancePenalty";

/// The resistance penalty: the active config set's `resistancePenalty`
/// when it is a number, else [`RESISTANCE_PENALTY`].
fn resistance_penalty(build: &Build, ingest: &mut Ingest<'_>) {
    let set = active_config(build);
    let configured = set.and_then(|s| {
        s.inputs
            .typed()
            .find(|i| i.name == RESISTANCE_PENALTY_INPUT)
            .and_then(|i| match &i.value {
                ConfigValue::Number(n) => Some(n.value().0),
                ConfigValue::Boolean(_) | ConfigValue::String(_) => None,
            })
    });
    let (value, source) = match configured {
        Some(v) => (
            v,
            Source::Config {
                input: RESISTANCE_PENALTY_INPUT.to_owned(),
            },
        ),
        #[expect(clippy::cast_precision_loss, reason = "a small constant, exact as f64")]
        None => (
            RESISTANCE_PENALTY as f64,
            Source::Rule("resistance-penalty"),
        ),
    };
    for t in [
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Lightning,
        DamageType::Chaos,
    ] {
        ingest.effect(Effect {
            quantity: Quantity::Resistance(t),
            op: Op::Base,
            value,
            conditions: Conditions::none(),
            source: source.clone(),
        });
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

/// The allocated tree nodes connected to an allocated root (a class or
/// ascendancy start) through allocated nodes (`Classes/PassiveSpec.lua`
/// `BuildAllDependsAndPaths`, `connectedToStart`). A node the data lacks
/// links nothing.
fn connected(input: &CalcInput<'_>, nodes: &[NodeId]) -> BTreeSet<u32> {
    let allocated: BTreeSet<u32> = nodes
        .iter()
        .map(|n| n.0)
        .filter(|n| n & CLUSTER_NODE_BIT == 0)
        .collect();
    let links: BTreeMap<u32, PassiveLinks> = allocated
        .iter()
        .filter_map(|&n| input.data.passive_links(n).map(|l| (n, l)))
        .collect();
    let mut seen: BTreeSet<u32> = links
        .iter()
        .filter(|(_, l)| l.root)
        .map(|(n, _)| *n)
        .collect();
    let mut queue: Vec<u32> = seen.iter().copied().collect();
    while let Some(n) = queue.pop() {
        for &m in links.get(&n).map_or(&[][..], |l| l.links.as_slice()) {
            if allocated.contains(&m) && seen.insert(m) {
                queue.push(m);
            }
        }
    }
    seen
}

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
    let connected = connected(input, nodes);
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
        let Some(stats) = input.data.passive_stats(node.0) else {
            ingest.indeterminate(IndeterminateReason::MissingData {
                table: "passives",
                key: node.0.to_string(),
            });
            continue;
        };
        // PoB drops allocated nodes not connected to a start when it
        // loads a build, and the game allows none; their stats do not
        // count, and the fields say so. Jewels that allocate without a
        // path (Thread of Hope, Intuitive Leap) are not modelled yet, so
        // their nodes land here too: partial, never a silent value.
        if !connected.contains(&node.0) {
            // A cluster jewel socket inside a subgraph (PoB's legacy ids
            // are tree ids) connects through the cluster jewel.
            let expansion = input
                .data
                .passive_links(node.0)
                .is_some_and(|l| l.expansion);
            ingest.reason(UnsupportedReason::UnmodelledPassive {
                source: Source::Passive { node: *node },
                what: if expansion {
                    "cluster-jewel-node"
                } else {
                    "unconnected-node"
                },
            });
            continue;
        }
        ingest.stats(&Source::Passive { node: *node }, &lines(stats));
    }
    for m in masteries {
        // A mastery effect counts only on an allocated mastery (PoB keeps
        // `masteryEffects` of unallocated nodes in the XML but ignores
        // them) that is connected: a mastery connects through an
        // allocated notable of its group (the unconnected node itself is
        // reported above).
        if !nodes.contains(&m.node) || !connected.contains(&m.node.0) {
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
fn items(input: &CalcInput<'_>, spec: Option<&PassiveSpec>, ingest: &mut Ingest<'_>) {
    let build = input.build;
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
                SlotKind::Properties => {
                    if let (Some(armour), Some(item)) = (ArmourSlot::of(slot), build.items.item(id))
                    {
                        armour_slot(input, item, armour, slot, ingest);
                        continue;
                    }
                    // A weapon's base properties (damage, speed, crit;
                    // dual wielding's block) are P3-10's.
                    ingest.reason(unmodelled("base-properties"));
                }
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

/// The item in an armour slot (a helmet, gloves, boots, a body armour, or
/// the off hand): an armour piece's base properties and local lines act
/// on the piece ([`armour_piece`]); a quiver's or jewellery's lines are
/// global; a weapon in the off hand (dual wielding) is P3-10's.
fn armour_slot(
    input: &CalcInput<'_>,
    item: &Item,
    slot: ArmourSlot,
    slot_name: &str,
    ingest: &mut Ingest<'_>,
) {
    let unmodelled = |what| UnsupportedReason::UnmodelledItem {
        item: item.id,
        slot: slot_name.to_owned(),
        what,
    };
    let Some(name) = input.options.item_bases.get(&item.id) else {
        ingest.reason(unmodelled("no-base-name"));
        item_lines(item, ingest);
        return;
    };
    match name.find(input.data) {
        Some(ItemBase::Armour(base)) => {
            if slot == ArmourSlot::Weapon2 && base.block.is_none() {
                // Every shield has a block chance; one the data lacks would
                // be a silent 0.
                ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "shield_types",
                    key: name.name.clone(),
                });
            }
            armour_piece(item, slot, slot_name, &base, ingest);
        }
        // A quiver in the off hand: its lines only.
        Some(ItemBase::Other) if slot == ArmourSlot::Weapon2 => item_lines(item, ingest),
        // A helmet, gloves, boots, or body armour whose base has no armour
        // properties: whatever it is, its defences are not known.
        Some(ItemBase::Other) => {
            ingest.reason(unmodelled("not-armour"));
            item_lines(item, ingest);
        }
        Some(ItemBase::Weapon) => {
            ingest.reason(unmodelled("base-properties"));
            item_lines(item, ingest);
        }
        Some(ItemBase::Ambiguous) => {
            ingest.reason(unmodelled("ambiguous-base"));
            item_lines(item, ingest);
        }
        None => {
            ingest.indeterminate(IndeterminateReason::MissingData {
                table: "base_items",
                key: name.name.clone(),
            });
            item_lines(item, ingest);
        }
    }
}

/// The leading number of a spec value, as PoB's `specToNumber` reads it
/// (`^[+-]?[%d.]+`).
fn spec_number(value: &str) -> Option<f64> {
    let value = value.trim();
    let sign = usize::from(value.starts_with(['+', '-']));
    let end = value[sign..]
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map_or(value.len(), |i| i + sign);
    let n: f64 = value[..end].trim_start_matches('+').parse().ok()?;
    Some(n)
}

/// The defence a spec key shows (PoB reads `Evasion Rating` as `Evasion`).
fn shown_defence(key: &str) -> Option<Defence> {
    match key {
        "Armour" => Some(Defence::Armour),
        "Evasion" | "Evasion Rating" => Some(Defence::Evasion),
        "Energy Shield" => Some(Defence::EnergyShield),
        _ => None,
    }
}

/// The defence whose base percentile a spec key gives.
fn percentile_defence(key: &str) -> Option<Defence> {
    match key {
        "ArmourBasePercentile" => Some(Defence::Armour),
        "EvasionBasePercentile" => Some(Defence::Evasion),
        "EnergyShieldBasePercentile" => Some(Defence::EnergyShield),
        _ => None,
    }
}

/// An armour piece: its base's defence ranges and block, its quality and
/// base percentiles from its spec lines, and its local lines, all on the
/// piece's own quantities ([`Poe1Quantity::Piece`]); its other lines are
/// global. PoB (`Classes/Item.lua`, the armour branch): a defence is
/// `round((min + local flat + range x percentile) x (1 + local inc / 100)
/// x (1 + quality / 100))`; a percentile comes from the item's
/// `<Defence>BasePercentile` line, else from its shown value (`Armour:
/// 120`) when it has one, else 1. Deriving it from a shown value is not
/// modelled (`base-percentile`); PoB writes both lines.
fn armour_piece(
    item: &Item,
    slot: ArmourSlot,
    slot_name: &str,
    base: &ArmourBase,
    ingest: &mut Ingest<'_>,
) {
    let piece = |value| Quantity::Poe1(Poe1Quantity::Piece(slot, value));
    let from_base = Source::ItemBase { item: item.id };
    let effect = |quantity, op, value, source: &Source| Effect {
        quantity,
        op,
        value,
        conditions: Conditions::none(),
        source: source.clone(),
    };
    ingest.effect(effect(
        piece(PieceValue::Equipped),
        Op::Flag,
        1.0,
        &from_base,
    ));
    let mut ranges = Vec::new();
    for (defence, range) in [
        (Defence::Armour, base.armour),
        (Defence::Evasion, base.evasion),
        (Defence::EnergyShield, base.energy_shield),
    ] {
        #[expect(
            clippy::cast_precision_loss,
            reason = "base values are far below 2^53, where i64 to f64 is exact"
        )]
        let (min, variance) = (range.min as f64, (range.max - range.min) as f64);
        if min != 0.0 {
            ingest.effect(effect(
                piece(PieceValue::Defence(defence)),
                Op::Base,
                min,
                &from_base,
            ));
        }
        if variance != 0.0 {
            ingest.effect(effect(
                piece(PieceValue::Variance(defence)),
                Op::Base,
                variance,
                &from_base,
            ));
        }
        ranges.push((defence, variance));
    }
    if let Some(block) = base.block {
        #[expect(
            clippy::cast_precision_loss,
            reason = "block chances are small integers"
        )]
        let block = block as f64;
        ingest.effect(effect(
            piece(PieceValue::Block),
            Op::Base,
            block,
            &from_base,
        ));
    }
    let mut shown = Vec::new();
    let mut percentiles = Vec::new();
    // The last line of each key counts, as PoB's `self.quality` and
    // `armourData[key]` are overwritten line by line.
    let mut specs: BTreeMap<PieceValue, (usize, &str)> = BTreeMap::new();
    for (index, line) in item.body.lines.iter().enumerate() {
        let LineKind::Spec { key, value, .. } = &line.kind else {
            continue;
        };
        if key == "Quality" {
            specs.insert(PieceValue::Quality, (index, value));
        } else if let Some(d) = percentile_defence(key) {
            percentiles.push(d);
            specs.insert(PieceValue::Percentile(d), (index, value));
        } else if let Some(d) = shown_defence(key) {
            shown.push(d);
        }
    }
    for (target, (index, value)) in specs {
        let at = Source::Item {
            item: item.id,
            line: index,
            adapted: None,
        };
        match spec_number(value) {
            Some(n) => ingest.effect(effect(piece(target), Op::Base, n, &at)),
            None => ingest.reason(UnsupportedReason::UnreadLine { source: at }),
        }
    }
    // A shown value with a range of 0 needs no percentile: PoB's
    // derivation (`Classes/Item.lua:2323-2325`) divides by the range,
    // giving +-inf (clamped to 1 or 0) or NaN when the shown value equals
    // the rest exactly; times a range of 0 the term is 0 either way but in
    // the NaN case, which a shown value PoB itself wrote never hits.
    for (defence, variance) in ranges {
        if variance != 0.0 && shown.contains(&defence) && !percentiles.contains(&defence) {
            ingest.reason(UnsupportedReason::UnmodelledItem {
                item: item.id,
                slot: slot_name.to_owned(),
                what: "base-percentile",
            });
        }
    }
    item_lines_with(&PieceMapping { slot }, item, ingest);
}

/// An item's lines: modifier lines by their resolution; lines the import
/// did not read are reasons.
fn item_lines(item: &Item, ingest: &mut Ingest<'_>) {
    item_lines_with(&Poe1Mapping, item, ingest);
}

/// [`item_lines`] through `mapping`.
fn item_lines_with(mapping: &dyn StatMapping, item: &Item, ingest: &mut Ingest<'_>) {
    for (index, line) in item.body.lines.iter().enumerate() {
        match &line.kind {
            LineKind::Modifier(m) => ingest.item_line_with(mapping, item.id, index, &m.resolution),
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

/// The active config set: `Config.activeConfigSet`, else the first.
fn active_config(build: &Build) -> Option<&ConfigSet> {
    let config = &build.config;
    config
        .active
        .and_then(|id| config.sets.typed().find(|s| s.id == id))
        .or_else(|| config.sets.typed().next())
}

/// Every input of the active config set the engine does not model:
/// all but a numeric `resistancePenalty` ([`resistance_penalty`]).
fn config(build: &Build, ingest: &mut Ingest<'_>) {
    let Some(set) = active_config(build) else {
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
        if input.name == RESISTANCE_PENALTY_INPUT && matches!(input.value, ConfigValue::Number(_)) {
            continue;
        }
        // The enemy model's inputs (`poe1::enemy`), when of the type it
        // reads: no computed field reads the enemy yet, and the hit fields
        // will take its problems as their reasons (P3-9b).
        if super::enemy::ENEMY_INPUTS.contains(&input.name.as_str()) {
            let modelled = match &input.value {
                ConfigValue::String(s) => {
                    input.name == "enemyIsBoss" && super::enemy::BossKind::of(s).is_some()
                }
                ConfigValue::Number(_) => input.name != "enemyIsBoss",
                ConfigValue::Boolean(_) => false,
            };
            if modelled {
                continue;
            }
        }
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

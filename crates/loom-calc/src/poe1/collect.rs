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
//! - the weapon in the main hand has its own damage, attack time, and
//!   critical strike chance from its base, its quality, and its local
//!   lines (`Classes/Item.lua`, the weapon branch; P3-10), which only an
//!   attack's hit reads;
//! - the character's base defences (maximum resistances, block and spell
//!   block maximums, base evasion) are character template stats, and
//!   the resistance penalty is the config's `resistancePenalty` or -60.
//!
//! The main skill's gems act through their stats when the engine computes
//! its hit (a self-cast spell, P3-9b; an attack, P3-10).
//!
//! Not modelled yet, each a reason on every computed field: other enabled
//! gems (auras, buffs, reservation: P3-12), config inputs other than
//! the resistance penalty, the enemy model's numeric inputs, and
//! [`super::config::NAMED_ONLY`] (P3-12a), and the ones with their own
//! effects ([`super::charges`], [`super::damage_taken`], [`super::buffs`],
//! the bandit and the pantheon: P3-12e to P3-12g), a
//! weapon's base properties in the off hand (dual wielding), an
//! armour piece's percentile
//! derived from its shown value, active flasks, abyssal sockets, and
//! slots other than the known ones.

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::{
    Build, ConfigInput, ConfigInputKind, ConfigLayout, ConfigSet, ConfigValue, GameSpecific, Item,
    ItemId, ItemSet, LineKind, NodeId, PassiveSpec, StatId, StatLine, StatValue,
};

use super::gem_levels::GemLevels;
use super::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
use super::skill::{self, InstanceProblem};
use super::{ArmourSlot, Defence, PieceValue, Poe1Quantity, WeaponValue, hit};
use crate::ingest::{Ingest, StatMapping};
use crate::input::{ArmourBase, CalcInput, ItemBase, PassiveLinks, StatAmount, WeaponBase};
use crate::quantity::{Bound, DamageType, Op, Quantity};
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
    choices(build, input.data, ingest);
    let spec = active_spec(build);
    if let Some(spec) = spec {
        passives(input, spec, ingest);
    }
    items(input, spec, ingest);
    // After the items: "Allocates <notable>" (P3-8n).
    granted_passives(input, spec, ingest);
    config(build, ingest);
    // After every source of gem level modifiers (P3-8i): PoB reads its
    // `GemProperty` modifiers once the passives, items, and config are in.
    gems(input, ingest);
    super::charges::charges(active_config(build), input.data, ingest);
    // Rage, Inspiration, Momentum, Intensity, spells cast recently (P3-8n):
    // after every source of their modifiers, before the enemy's damage
    // taken (rage's Intimidate).
    super::stacks::stacks(active_config(build), input.data, ingest);
    super::buffs::buffs(active_config(build), ingest);
    // After every source of minion modifiers (P3-8j).
    super::minions::minions(ingest);
    // After every source of exposure chances and extras, and of the shock
    // effect and minimum (P3-8k).
    super::exposure::exposure(active_config(build), ingest);
    super::damage_taken::damage_taken(build, active_config(build), input.data, ingest);
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
            .find(|i| i.kind == ConfigInputKind::Input && i.name == RESISTANCE_PENALTY_INPUT)
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

/// The config input of the bandit choice.
pub const BANDIT_INPUT: &str = "bandit";

/// The bandit choices and their effects on the slice (PoB v2.67.2,
/// `Modules/CalcSetup.lua` 663 to 672): Alira `ElementalResist` BASE 15,
/// Oak `Life` BASE 40; Kraityn (movement speed) and none (one passive
/// point) reach no slice field.
pub const BANDITS: [(&str, &[(Quantity, f64)]); 4] = [
    (
        "Alira",
        &[
            (Quantity::Resistance(DamageType::Fire), 15.0),
            (Quantity::Resistance(DamageType::Cold), 15.0),
            (Quantity::Resistance(DamageType::Lightning), 15.0),
        ],
    ),
    ("Kraityn", &[]),
    ("None", &[]),
    ("Oak", &[(Quantity::Life, 40.0)]),
];

/// Bandit and pantheon (P3-12f for the bandit, P3-12g for the pantheon).
///
/// PoB reads the bandit from the active config set's `bandit` input
/// (`env.configInput.bandit`), "None" when the set has none (each new set
/// holds the list's default, `Classes/ConfigTab.lua` 1290 to 1296). The
/// build element's `bandit` attribute is read only for a build without a
/// config element (`Modules/Build.lua` 648 to 651, before the sections
/// load; `ConfigTab:Load` replaces the sets). So with a config set the
/// engine applies the set's choice ([`BANDITS`]; another value is a
/// reason under the input), and ignores the attribute. With no set, the
/// build may have had no config element (the attribute counts) or an
/// empty one (it does not): the domain does not tell them apart, so an
/// attribute other than "None" stays a reason. With only untyped sets,
/// their `<untyped>` reason covers the choice.
///
/// The pantheon reads its two inputs the same way (`Modules/Build.lua`
/// 630 to 651 copies all three attributes; [`super::pantheon`], P3-12g):
/// with a set, the set's gods act and the attributes are ignored; with no
/// set, an attribute other than "None" stays a reason.
fn choices(build: &Build, data: &dyn crate::input::CalcData, ingest: &mut Ingest<'_>) {
    let GameSpecific::Poe1(c) = build.game_specific() else {
        return;
    };
    match active_config(build) {
        Some(set) => {
            bandit(set, ingest);
            super::pantheon::pantheon(set, data, ingest);
        }
        None if build.config.sets.entries().is_empty() => {
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
        None => {}
    }
}

/// The active set's bandit ([`choices`]): its last `bandit` input, "None"
/// when it has none.
fn bandit(set: &ConfigSet, ingest: &mut Ingest<'_>) {
    let name = match super::config::last_input(set, BANDIT_INPUT, ConfigInputKind::Input) {
        None => return,
        Some(ConfigValue::String(s)) => s.as_str(),
        Some(_) => "",
    };
    let Some((_, effects)) = BANDITS.iter().find(|(b, _)| *b == name) else {
        ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: BANDIT_INPUT.to_owned(),
        });
        return;
    };
    for &(quantity, value) in *effects {
        ingest.effect(Effect {
            quantity,
            op: Op::Base,
            value,
            conditions: Conditions::none(),
            source: Source::Config {
                input: BANDIT_INPUT.to_owned(),
            },
        });
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

/// The node hashes of an item's "Allocates <notable>" line (P3-8n): its
/// one stat `mod_granted_passive_hash`, its value the hash (one number) or
/// the hashes the description's lookup gives the name (several nodes can
/// share a name). `None` for any other line, which the mapping reads.
fn granted_passive_hashes(resolution: &loom_domain::LineResolution) -> Option<Vec<i64>> {
    let loom_domain::LineResolution::Resolved {
        result: loom_domain::Resolved::Stats(lines),
        ..
    } = resolution
    else {
        return None;
    };
    let [line] = lines.as_slice() else {
        return None;
    };
    if line.stat.as_str() != GRANTED_PASSIVE_STAT {
        return None;
    }
    match &line.value {
        StatValue::Exact { min, max } if min == max => Some(vec![*min]),
        StatValue::Lookup(hashes) => Some(hashes.clone()),
        _ => None,
    }
}

/// "Allocates <notable>".
const GRANTED_PASSIVE_STAT: &str = "mod_granted_passive_hash";

/// The nodes items grant ("Allocates <notable>", PoB's `GrantedPassive`,
/// `CalcSetup.lua` 1287 to 1297; P3-8n): each [`Poe1Quantity::GrantedPassive`]
/// effect's value is a candidate node hash, the key of the passives table;
/// a line's candidates share its source. PoB looks the node up by its name
/// among the tree's notables, then the ascendancy nodes
/// (`notableMap[name] or ascendancyMap[name]`; `Classes/PassiveTree.lua`
/// 551 to 563): here the one candidate the passives table has, or of
/// several the one tree notable, else the one ascendancy notable (none, or
/// two of a kind: indeterminate). PoB adds the
/// node to the allocated set, so a node the tree already allocates (and
/// connects) counts once; otherwise its stats count with the node as
/// source.
fn granted_passives(input: &CalcInput<'_>, spec: Option<&PassiveSpec>, ingest: &mut Ingest<'_>) {
    let allocated: BTreeSet<u32> = spec.map_or_else(BTreeSet::new, |spec| {
        connected(input, spec.nodes.value())
            .into_iter()
            .filter(|n| spec.nodes.value().iter().any(|a| a.0 == *n))
            .collect()
    });
    let mut granted = BTreeSet::new();
    let mut by_line: BTreeMap<Source, Vec<f64>> = BTreeMap::new();
    for e in ingest.effects() {
        if e.quantity == Quantity::Poe1(Poe1Quantity::GrantedPassive)
            && e.op == Op::Base
            && e.conditions == Conditions::none()
        {
            by_line.entry(e.source.clone()).or_default().push(e.value);
        }
    }
    for (source, hashes) in by_line {
        let known: Vec<u32> = hashes
            .iter()
            .filter_map(|&hash| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a node hash below 2^16, checked"
                )]
                let node = (hash.fract() == 0.0 && (0.0..65536.0).contains(&hash))
                    .then_some(hash as u32)?;
                input.data.passive_stats(node).map(|_| node)
            })
            .collect();
        // Of several, PoB's order: a tree notable, then an ascendancy node.
        let pick = |tree: bool| -> Vec<u32> {
            known
                .iter()
                .copied()
                .filter(|&n| {
                    input
                        .data
                        .passive_kind(n)
                        .is_some_and(|k| k.notable && k.ascendancy != tree)
                })
                .collect()
        };
        let known = if known.len() > 1 {
            let tree = pick(true);
            if tree.is_empty() { pick(false) } else { tree }
        } else {
            known
        };
        let [node] = known.as_slice() else {
            ingest.indeterminate(IndeterminateReason::MissingData {
                table: "passives",
                key: format!("granted-passive:{source:?}"),
            });
            continue;
        };
        let node = *node;
        if allocated.contains(&node) || !granted.insert(node) {
            continue;
        }
        if let Some(stats) = input.data.passive_stats(node) {
            ingest.stats(&Source::Passive { node: NodeId(node) }, &lines(stats));
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

/// The parent item's stat that gives it abyssal sockets ("Has # Abyssal
/// Sockets", PoB's `AbyssalSocketCount`, `Item.lua` 2589).
const ABYSS_SOCKETS: &str = "local_has_X_abyss_sockets";
/// A parent item's stat that scales its socketed abyss jewels: not
/// modelled, a reason.
const ABYSS_JEWEL_EFFECT: &str = "local_socketed_abyss_jewel_effect_+%";

/// Whether the abyssal slot `slot` ("<parent> Abyssal Socket <n>") holds
/// an active socket (close-out): PoB's `ItemSlotControl` makes socket `n`
/// inactive, and empties it, when the parent slot's item has fewer than
/// `n` abyssal sockets (`ItemSlotControl.lua` 98 to 111), so its jewel
/// counts only when `n` is at most the parent's count. `None` when that
/// is not known: a parent line that does not resolve or settle, or one
/// that scales its socketed abyss jewels.
fn abyssal_active(build: &Build, set: &ItemSet, slot: &str) -> Option<bool> {
    let (parent, n) = slot.split_once(" Abyssal Socket ")?;
    let n: i64 = n.trim().parse().ok()?;
    let Some(parent_id) = set
        .slots
        .typed()
        .find(|e| e.slot.as_str() == parent)
        .and_then(|e| e.item)
        .filter(|id| id.0 != 0)
    else {
        return Some(false);
    };
    let item = build.items.item(parent_id)?;
    let mut count = 0;
    for line in &item.body.lines {
        let LineKind::Modifier(m) = &line.kind else {
            continue;
        };
        match &m.resolution {
            loom_domain::LineResolution::Resolved {
                result: loom_domain::Resolved::Stats(stats),
                ..
            } => {
                for s in stats {
                    if s.stat.as_str() == ABYSS_JEWEL_EFFECT {
                        return None;
                    }
                    if s.stat.as_str() == ABYSS_SOCKETS {
                        #[expect(clippy::cast_possible_truncation, reason = "a socket count")]
                        let v = crate::ingest::settle(&s.value)? as i64;
                        count += v;
                    }
                }
            }
            loom_domain::LineResolution::Excluded(loom_domain::Exclusion::NotSelected)
            | loom_domain::LineResolution::Continues { .. }
            | loom_domain::LineResolution::Resolved { .. } => {}
            _ => return None,
        }
    }
    Some(n <= count)
}

/// Whether a slot is in use for the item set's weapon set (PoB's
/// `slot.weaponSet`): `... Swap` slots belong to set 2, the other weapon
/// slots to set 1, every other slot to both.
pub(super) fn slot_in_use(slot: &str, second_set: bool) -> bool {
    if slot.contains("Swap") {
        second_set
    } else if slot.starts_with("Weapon") {
        !second_set
    } else {
        true
    }
}

/// The active item set: `Items.activeItemSet` (default 1), else the first.
pub(super) fn active_item_set(build: &Build) -> Option<&ItemSet> {
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
    if let Some(set) = active_item_set(build) {
        let second = attribute(&set.attributes, "useSecondWeaponSet") == Some("true");
        for entry in set.slots.typed() {
            let slot = entry.slot.as_str();
            let Some(id) = entry.item else { continue };
            // PoB writes `itemId="0"` for an empty slot. An item a build
            // puts in two slots counts in each, as PoB counts it per slot
            // (`CalcSetup.lua` 701 to 722; close-out: before, it counted
            // once). The game cannot equip one item twice; the build's
            // slots are taken as written.
            if id.0 == 0 || !slot_in_use(slot, second) {
                continue;
            }
            if !seen.contains(&id) {
                seen.push(id);
            }
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
                SlotKind::Abyssal => match abyssal_active(build, set, slot) {
                    // Its lines act as any jewel's (close-out).
                    Some(true) => {}
                    // PoB empties an abyssal slot its parent lacks
                    // (`ItemSlotControl.lua` 98 to 111).
                    Some(false) => continue,
                    None => {
                        ingest.reason(unmodelled("abyssal-socket"));
                        continue;
                    }
                },
                SlotKind::Properties => {
                    if let (Some(armour), Some(item)) = (ArmourSlot::of(slot), build.items.item(id))
                    {
                        armour_slot(input, item, armour, slot, ingest);
                        continue;
                    }
                    if let (true, Some(item)) = (is_main_hand(slot), build.items.item(id)) {
                        weapon_slot(input, item, slot, ingest);
                        continue;
                    }
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
        if id.0 == 0 || !allocated.contains(&socket.node) {
            continue;
        }
        // A jewel a build puts in several sockets counts in each, as PoB
        // counts it per socket slot (`CalcSetup.lua` 744 to 837; close-out).
        // PoB disables a repeat of a jewel with a limit ("Limited to"),
        // which only unique jewels carry: not modelled, a reason.
        let repeat = seen.contains(&id);
        if !repeat {
            seen.push(id);
        }
        let item = build.items.item(id);
        if repeat && item.is_some_and(|i| i.body.rarity == Some(loom_domain::Rarity::Unique)) {
            ingest.reason(UnsupportedReason::UnmodelledItem {
                item: id,
                slot: format!("socket {}", socket.node),
                what: "jewel-limit",
            });
            continue;
        }
        match item {
            Some(item) => item_lines(item, ingest),
            None => ingest.reason(UnsupportedReason::UnmodelledItem {
                item: id,
                slot: format!("socket {}", socket.node),
                what: "missing",
            }),
        }
    }
}

/// Whether a slot in use is the main hand (`Weapon 1`, or `Weapon 1
/// Swap` when the second weapon set is in use).
fn is_main_hand(slot: &str) -> bool {
    slot.strip_suffix(" Swap").unwrap_or(slot) == "Weapon 1"
}

/// The hands of the weapon set in use: the main hand's weapon (its item
/// and base) when its base is a weapon, and whether the off hand holds a
/// weapon or an item whose base is not known (dual wielding, as far as
/// the engine can tell). The attack hit reads them
/// ([`hit::attack_skill`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hands {
    /// The main hand's weapon.
    pub main: Option<(ItemId, WeaponBase)>,
    /// The off hand holds a weapon, or an item whose base is not known.
    pub off_hand_weapon: bool,
    /// What the main hand holds (P3-8f).
    pub main_hand: Held,
    /// What the off hand holds (P3-8f).
    pub off_hand: Held,
}

/// What a hand holds, as far as the engine can tell (P3-8f: PoB's
/// weapon conditions).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Held {
    /// Nothing.
    #[default]
    Empty,
    /// A weapon of this item class.
    Weapon(String),
    /// A shield (an armour base with a chance to block).
    Shield,
    /// Another known base (a quiver, an armour base without block).
    Other,
    /// An item whose base is not known, or one of several bases.
    Unknown,
}

impl Held {
    fn of(base: Option<ItemBase>) -> Self {
        match base {
            Some(ItemBase::Weapon(w)) => Self::Weapon(w.class),
            Some(ItemBase::Armour(a)) if a.block.is_some() => Self::Shield,
            Some(ItemBase::Armour(_) | ItemBase::Other) => Self::Other,
            Some(ItemBase::Ambiguous) | None => Self::Unknown,
        }
    }
}

/// The weapon set in use's hands ([`Hands`]).
#[must_use]
pub fn hands(input: &CalcInput<'_>) -> Hands {
    let mut hands = Hands::default();
    let build = input.build;
    let Some(set) = active_item_set(build) else {
        return hands;
    };
    let second = attribute(&set.attributes, "useSecondWeaponSet") == Some("true");
    for entry in set.slots.typed() {
        let slot = entry.slot.as_str();
        let Some(id) = entry.item else { continue };
        if id.0 == 0 || !slot_in_use(slot, second) {
            continue;
        }
        let base = input
            .options
            .item_bases
            .get(&id)
            .and_then(|name| name.find(input.data));
        match slot.strip_suffix(" Swap").unwrap_or(slot) {
            "Weapon 1" => {
                hands.main_hand = Held::of(base.clone());
                if let Some(ItemBase::Weapon(w)) = base {
                    hands.main = Some((id, w));
                }
            }
            "Weapon 2" => {
                hands.off_hand_weapon =
                    !matches!(base, Some(ItemBase::Armour(_) | ItemBase::Other));
                hands.off_hand = Held::of(base);
            }
            _ => {}
        }
    }
    hands
}

/// The item in the main hand: a weapon's base properties and local lines
/// act on the weapon ([`weapon`]); another item's lines are global, and
/// its being in the main hand is a reason.
fn weapon_slot(input: &CalcInput<'_>, item: &Item, slot_name: &str, ingest: &mut Ingest<'_>) {
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
        Some(ItemBase::Weapon(base)) => weapon(item, &base, ingest),
        Some(ItemBase::Armour(_) | ItemBase::Other) => {
            ingest.reason(unmodelled("not-weapon"));
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

/// The weapon in the main hand: its base's physical damage, attack time,
/// and critical strike chance, its quality from its spec lines (the last
/// `Quality` line, as PoB's `self.quality`; none: 0), and its local lines,
/// all on the weapon's own quantities ([`Poe1Quantity::Weapon`]); its
/// other lines are global. The hit phase computes the weapon's values from
/// them as PoB does (`hit` module docs).
fn weapon(item: &Item, base: &WeaponBase, ingest: &mut Ingest<'_>) {
    let value = |v| Quantity::Poe1(Poe1Quantity::Weapon(v));
    let from_base = Source::ItemBase { item: item.id };
    let effect = |quantity, value: f64, source: &Source| Effect {
        quantity,
        op: Op::Base,
        value,
        conditions: Conditions::none(),
        source: source.clone(),
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "base values are small integers, exact as f64"
    )]
    let (min, max, time, crit) = (
        base.physical.min as f64,
        base.physical.max as f64,
        base.attack_time as f64,
        base.crit_chance as f64,
    );
    for (bound, v) in [(Bound::Min, min), (Bound::Max, max)] {
        if v != 0.0 {
            ingest.effect(effect(
                value(WeaponValue::Damage(DamageType::Physical, bound)),
                v,
                &from_base,
            ));
        }
    }
    ingest.effect(effect(value(WeaponValue::AttackTime), time, &from_base));
    // Hundredths of a percent: one division, the nearest double to the
    // decimal PoB's exporter writes (`CritChanceBase`).
    ingest.effect(effect(
        value(WeaponValue::CritChance),
        crit / 100.0,
        &from_base,
    ));
    let quality = item
        .body
        .lines
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, line)| match &line.kind {
            LineKind::Spec { key, value, .. } if key == "Quality" => Some((index, value)),
            _ => None,
        });
    if let Some((index, text)) = quality {
        let at = Source::Item {
            item: item.id,
            line: index,
            adapted: None,
        };
        match spec_number(text) {
            Some(n) => ingest.effect(effect(value(WeaponValue::Quality), n, &at)),
            None => ingest.reason(UnsupportedReason::UnreadLine { source: at }),
        }
    }
    item_lines_with(&WeaponMapping, item, ingest);
    // "Grants #% increased Elemental Damage per 2% Quality" (P3-8n): PoB's
    // `ElementalDamage` INC times `floor(QualityOn{SlotName} / 2 +
    // 0.0001)`, the character's (`ModParser.lua` 5759, `ModStore`'s
    // multiplier). Its own effect on the weapon is read by nothing else.
    let quality = ingest
        .effects()
        .iter()
        .filter(|e| e.quantity == value(WeaponValue::Quality) && e.op == Op::Base)
        .map(|e| e.value)
        .sum::<f64>();
    let per: Vec<Effect> = ingest
        .effects()
        .iter()
        .filter(|e| {
            e.quantity == value(WeaponValue::ElementalDamagePerQuality)
                && e.op == Op::Base
                && e.conditions == Conditions::none()
                && matches!(e.source, Source::Item { item: i, .. } if i == item.id)
        })
        .cloned()
        .collect();
    let steps = (quality / 2.0 + 0.0001).floor();
    for e in per {
        if steps * e.value != 0.0 {
            ingest.effect(Effect {
                quantity: Quantity::Damage,
                op: Op::Increased,
                value: e.value * steps,
                conditions: Conditions::none().with_any(crate::condition::Flags::ELEMENTAL),
                source: e.source,
            });
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
        Some(ItemBase::Weapon(_)) => {
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
            LineKind::Modifier(m) => {
                // "Allocates <notable>" (P3-8n): its value is the name's
                // node hashes; [`granted_passives`] picks the node.
                if let Some(hashes) = granted_passive_hashes(&m.resolution) {
                    let source = Source::Item {
                        item: item.id,
                        line: index,
                        adapted: None,
                    };
                    for hash in hashes {
                        #[expect(clippy::cast_precision_loss, reason = "a node hash, below 2^53")]
                        ingest.effect(Effect {
                            quantity: Quantity::Poe1(Poe1Quantity::GrantedPassive),
                            op: Op::Base,
                            value: hash as f64,
                            conditions: Conditions::none(),
                            source: source.clone(),
                        });
                    }
                    continue;
                }
                ingest.item_line_with(mapping, item.id, index, &m.resolution);
            }
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

/// The gems of the active skill set. When the main skill is a hit the
/// engine computes ([`hit::hit_skill`]: a self-cast spell or an attack),
/// its active gem and the supports that apply act through their stats on
/// the skill alone ([`SkillMapping`]: a stat with any other effect is
/// unmapped, a reason on every field), and their instance problems are
/// reasons. Every other skill of the set acts as [`super::gems`] models it
/// (inert skills, reservation, buffs), or is a reason on every field
/// (P3-12h; before, every other enabled gem was).
fn gems(input: &CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let build = input.build;
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
        // The groups' indexes are not certain: every gem stays a reason.
        for (g, group) in set.groups.typed().enumerate() {
            for (i, gem) in group.gems.typed().enumerate() {
                if group.enabled && gem.enabled {
                    ingest.reason(UnsupportedReason::UnmodelledGem {
                        source: Source::Gem { group: g, gem: i },
                    });
                }
            }
        }
        return;
    }
    let levels = GemLevels::from_effects(ingest.effects());
    let hit = hit::hit_skill(input, &levels);
    let modelled: Vec<_> = hit.as_ref().map_or_else(Vec::new, |h| {
        std::iter::once(&h.skill().active_gem)
            .chain(&h.skill().supports)
            .copied()
            .collect()
    });
    // The reservation and aura effect modifiers (P3-8l): every source of
    // them (passives, items, config) is in.
    let totals = super::skill_modifiers::Totals::read(ingest);
    if totals.unread {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: Source::Rule("skill-modifiers"),
            what: "unread-skill-modifier",
        });
    }
    super::gems::gems(
        input,
        hit.as_ref().map(|h| (h.group(), h.skill())),
        &levels,
        &totals,
        ingest,
    );
    let Some(hit) = hit else { return };
    // The main skill's own aura (Smite's) is scaled as any aura (P3-8l).
    let buff_scale = totals.buff_scale(true, &hit.skill().types, 0.0);
    for r in modelled {
        let source = Source::Gem {
            group: hit.group(),
            gem: r.gem,
        };
        let instance = skill::instance(r.effect, r.level, r.quality, input.data);
        for p in &instance.problems {
            match p {
                InstanceProblem::MissingStatSet { id } => {
                    ingest.indeterminate(IndeterminateReason::MissingData {
                        table: "stat_sets",
                        key: id.clone(),
                    });
                }
                _ => ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source: source.clone(),
                    what: p.name(),
                }),
            }
        }
        let stats: Vec<StatLine> = instance
            .stats
            .iter()
            .map(|(stat, value)| {
                let value =
                    super::skill_modifiers::main_skill_stat(&r.effect.id, stat, *value, buff_scale);
                StatLine {
                    stat: StatId::new(stat.as_str()),
                    value: StatValue::Exact {
                        min: value,
                        max: value,
                    },
                }
            })
            .collect();
        ingest.stats_with(&SkillMapping, &source, &stats);
    }
}

/// The active config set: `Config.activeConfigSet`, else the first.
pub(crate) fn active_config(build: &Build) -> Option<&ConfigSet> {
    let config = &build.config;
    config
        .active
        .and_then(|id| config.sets.typed().find(|s| s.id == id))
        .or_else(|| config.sets.typed().next())
}

/// Every input and placeholder of the active config set that can change a
/// computed field the engine does not model: all but a numeric
/// `resistancePenalty` ([`resistance_penalty`]), the enemy model's numeric
/// inputs and the placeholders it derives
/// ([`super::enemy::RESET_PLACEHOLDERS`]), the inputs that only set named
/// conditions, multipliers, or distances ([`super::config::NAMED_ONLY`]),
/// and the enemy's damage ([`super::config::ENEMY_DAMAGE`]). A
/// placeholder's reason has its input's name (`poe1::config`, module
/// docs). Each line of an enabled custom modifier block acts as a
/// character-wide modifier through its resolution ([`Ingest::custom_line`]:
/// a line that did not resolve is a reason); a disabled block changes
/// nothing (P3-12c).
/// The lines of PoB's legacy `customMods` input act the same way where
/// PoB moves them into a block ([`legacy_custom_mods`], P3-12d).
/// Untyped content (a block the reader kept whole, a placeholder PoB reads
/// as an input, content outside a config set beside set elements, which
/// PoB loads into set 1; PoB's layout with no set element is read as one
/// flat set, P3-12e)
/// is one `<untyped>` reason.
fn config(build: &Build, ingest: &mut Ingest<'_>) {
    let untyped = || UnsupportedReason::UnmodelledConfig {
        input: "<untyped>".to_owned(),
    };
    if build.config.sets.opaque().next().is_some() {
        ingest.reason(untyped());
    }
    let Some(set) = active_config(build) else {
        return;
    };
    if set.inputs.opaque().next().is_some() {
        ingest.reason(untyped());
    }
    // PoB applies only enabled blocks (`ConfigTab.lua`, `BuildModList`).
    for (b, block) in set.custom_modifiers.iter().enumerate() {
        if block.enabled {
            for (l, line) in block.lines.iter().enumerate() {
                ingest.custom_line(
                    Source::CustomModifier { block: b, line: l },
                    &line.resolution,
                );
            }
        }
    }
    match legacy_custom_mods(build, set) {
        Some(Legacy::Lines(input)) => {
            for (l, line) in input.lines.iter().enumerate() {
                ingest.custom_line(
                    Source::ConfigInputLine {
                        input: input.name.clone(),
                        line: l,
                    },
                    &line.resolution,
                );
            }
        }
        Some(Legacy::Unknown) => ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: LEGACY_CUSTOM_MODS.to_owned(),
        }),
        Some(Legacy::Nothing) | None => {}
    }
    // Every input, whatever its value: some of PoB's options default to
    // on, so even `false` can change a result.
    for input in set.inputs.typed() {
        // PoB's legacy custom modifiers, above.
        if is_legacy_custom_mods(input) {
            continue;
        }
        // Lines on any other input: no format applies them; a build
        // document can carry them, so they are a reason, never ignored.
        if !input.lines.is_empty() {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: input.name.clone(),
            });
            continue;
        }
        // Charges: their own effects (`poe1::charges`, P3-12e).
        if super::charges::is_charge_input(&input.name) || super::stacks::is_input(&input.name) {
            continue;
        }
        // The enemy's damage taken (shock, the boss kind, delirium),
        // Onslaught, the bandit, and the pantheon: their own effects or
        // reasons (`poe1::damage_taken`, `poe1::buffs`, [`choices`],
        // P3-12f, P3-12g).
        if super::damage_taken::handles(&input.name, &input.value)
            || super::conditions::handles(&input.name, &input.value)
            || super::exposure::handles(&input.name)
            || super::buffs::is_buff_input(&input.name)
            || input.name == BANDIT_INPUT
            || super::pantheon::is_input(&input.name)
            || super::gems::is_input(&input.name)
        {
            continue;
        }
        // Named conditions, multipliers, and distances only modifiers
        // read, and the enemy's damage: no computed field changes, for an
        // input or a placeholder (`poe1::config`, P3-12a, P3-12b).
        if super::config::named_only(&input.name) || super::config::enemy_damage(&input.name) {
            continue;
        }
        if input.kind == ConfigInputKind::Placeholder {
            // PoB's `enemyIsBoss` sets these on every run; the enemy
            // model derives them (P3-12b).
            if !super::enemy::RESET_PLACEHOLDERS.contains(&input.name.as_str()) {
                ingest.reason(UnsupportedReason::UnmodelledConfig {
                    input: input.name.clone(),
                });
            }
            continue;
        }
        if input.name == RESISTANCE_PENALTY_INPUT && matches!(input.value, ConfigValue::Number(_)) {
            continue;
        }
        // The enemy model's numeric inputs (`poe1::enemy`): the hit fields
        // take its problems as their reasons (P3-9b). `enemyIsBoss` as a
        // string is above; as a number it is a reason.
        if super::enemy::ENEMY_INPUTS.contains(&input.name.as_str())
            && input.name != super::damage_taken::BOSS_INPUT
            && matches!(input.value, ConfigValue::Number(_))
        {
            continue;
        }
        ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: input.name.clone(),
        });
    }
}

/// PoB's legacy custom modifier input (before custom modifier blocks).
const LEGACY_CUSTOM_MODS: &str = "customMods";

fn is_legacy_custom_mods(input: &ConfigInput) -> bool {
    input.name == LEGACY_CUSTOM_MODS && input.kind == ConfigInputKind::Input
}

/// What the active set's legacy `customMods` input does.
enum Legacy<'a> {
    /// Its lines act, as a block's.
    Lines(&'a ConfigInput),
    /// Nothing: PoB discards it.
    Nothing,
    /// Not known: a reason.
    Unknown,
}

/// PoB v2.67.2's migration (`Classes/ConfigTab.lua`, `Load`, 872 to 884):
/// a set's `customMods` value, when not empty, becomes the set's only
/// block (enabled) if the set has no block, or one block whose text is
/// empty (PoB's XML reader trims text, so whitespace only), even a
/// disabled one; otherwise PoB discards it. In every case PoB then clears
/// the input, so `BuildModList`'s fallback for it never runs after a load.
/// The last input of that name in the set is the one PoB keeps (its inputs
/// are a table). `None` when the set has none.
///
/// In PoB's layout before config sets ([`ConfigLayout::Flat`]), set 1
/// also holds PoB's empty default block before the blocks read.
///
/// Not known, so a reason: untyped content in the set, or outside any set
/// when the set is set 1 (PoB loads it there), can hold another input or
/// block of PoB's count; a value that is a number or `true` (PoB then
/// fails on the block's text). `false` is PoB's empty text (`input and
/// false or ""`): nothing.
fn legacy_custom_mods<'a>(build: &Build, set: &'a ConfigSet) -> Option<Legacy<'a>> {
    let input = set
        .inputs
        .typed()
        .filter(|i| is_legacy_custom_mods(i))
        .last()?;
    let untyped = set.inputs.opaque().next().is_some()
        || (set.id.0 == 1 && build.config.sets.opaque().next().is_some());
    if untyped {
        return Some(Legacy::Unknown);
    }
    let text = match &input.value {
        ConfigValue::String(s) => s,
        ConfigValue::Boolean(false) => return Some(Legacy::Nothing),
        ConfigValue::Boolean(true) | ConfigValue::Number(_) => return Some(Legacy::Unknown),
    };
    // In PoB's layout before config sets, set 1 already holds an empty
    // default block when `Load` appends the blocks it reads (`Load`, 890
    // to 900; `NewConfigSet`, 1281), so one block read makes two.
    let migrated = !text.is_empty()
        && match (build.config.layout, set.custom_modifiers.as_slice()) {
            (_, []) => true,
            (ConfigLayout::Sets, [only]) => only.text.trim_ascii().is_empty(),
            _ => false,
        };
    Some(if migrated {
        Legacy::Lines(input)
    } else {
        Legacy::Nothing
    })
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

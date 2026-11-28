//! Radius jewels that change the passives in their radius (Phase 6
//! P6-11gv): what PoB's jewel functions (`Modules/ModParser.lua`
//! `jewelOtherFuncs` and `jewelSelfUnallocFuncs`, run by
//! `Modules/CalcSetup.lua` `buildModListForNode`, 117 to 165) make of the
//! nodes in the radius of a jewel in an allocated socket.
//!
//! - **"Passive Skills in Radius also grant ..."** (the
//!   `local_unique_jewel_passives_in_radius_grant_<stat>` stats, PoB's
//!   functions at `ModParser.lua` 6217 to 6282): every allocated node in
//!   the radius but keystones, sockets, class starts, and masteries gains
//!   the global `<stat>` at the jewel's value. A node a Timeless jewel
//!   conquers is skipped (PoB's `conqueredBy` check, 6496 to 6511).
//! - **"Allocated Small Passive Skills in Radius grant nothing"**
//!   (`local_jewel_allocated_non_notable_passives_in_radius_grant_nothing`,
//!   6192): an allocated small passive (PoB's `Normal`) in the radius
//!   grants nothing, its additions from other jewels included (the wipe
//!   follows the first pass, 131).
//! - **"Grants all bonuses of Unallocated Small Passive Skills in Radius"**
//!   (`local_jewel_copy_stats_from_unallocated_non_notable_passives_in_radius`,
//!   6362): the unallocated small passives in the radius give their stats
//!   to the jewel, here the socket node's; a tattooed one gives its
//!   tattoo's (PoB applies the override first).
//!
//! - **"Increases and Reductions to Life in Radius are Transformed to apply
//!   to Energy Shield"** (Energy From Within,
//!   `local_unique_jewel_life_increases_applies_to_energy_shield`; PoB's
//!   `getSimpleConv({ "Life" }, "EnergyShield", "INC", true)`,
//!   `ModParser.lua` 6100, a first-pass function on every node in the
//!   radius but masteries): an allocated node's `maximum_life_+%` becomes
//!   `maximum_energy_shield_+%` at the same value, its other stats kept.
//!   Checked in the game (the maintainer, 2026-11-26: 6% increased maximum
//!   Life in the radius took 1,402 life to 1,323 and 36 energy shield to
//!   39). A node in the radius with a conditional life increase
//!   ([`LIFE_INCREASES_NOT_READ`]) is a reason on the socket.
//!
//! The jewel's own stats are `NoEffect` in the engine's mapping
//! (`loom_calc::poe1::mapping`). A jewel whose value does not settle, or
//! that both copies unallocated nodes and adds to the nodes of another
//! radius function, is a reason on its socket (`radius-jewel`).

use std::collections::{BTreeMap, BTreeSet};

use loom_calc::input::Conquest;
use loom_data::bundle::passives::StatAmount;
use loom_domain::{Build, LineKind, LineResolution, PassiveSpec, Resolved, StatValue};
use loom_tree::Tree;
use loom_tree::graph::Kind;

use crate::timeless::Rolls;

/// The prefix of the "also grant" stats; the rest is the global stat.
pub const ALSO_GRANT: &str = "local_unique_jewel_passives_in_radius_grant_";
/// "Allocated Small Passive Skills in Radius grant nothing".
pub const GRANT_NOTHING: &str =
    "local_jewel_allocated_non_notable_passives_in_radius_grant_nothing";
/// "Grants all bonuses of Unallocated Small Passive Skills in Radius".
pub const COPY_UNALLOCATED: &str =
    "local_jewel_copy_stats_from_unallocated_non_notable_passives_in_radius";

/// "Increases and Reductions to Life in Radius are Transformed to apply to
/// Energy Shield" (module docs).
pub const LIFE_TO_ENERGY_SHIELD: &str =
    "local_unique_jewel_life_increases_applies_to_energy_shield";
/// The plain life increase it transforms, and what it becomes.
const LIFE_INCREASE: (&str, &str) = ("maximum_life_+%", "maximum_energy_shield_+%");
/// Passive stats that are `Life` INC with a condition or a multiplier: in
/// the radius of [`LIFE_TO_ENERGY_SHIELD`] they are a reason (their energy
/// shield form is not read here).
pub const LIFE_INCREASES_NOT_READ: [&str; 3] = [
    "maximum_life_+%_per_abyssal_jewel_affecting_you",
    "maximum_life_+%_per_equipped_corrupted_item",
    "maximum_life_+%_while_no_gems_in_gloves",
];

/// What one socketed jewel does.
#[derive(Debug, Default)]
struct Functions {
    also_grant: Vec<StatAmount>,
    grant_nothing: bool,
    copy_unallocated: bool,
    life_to_energy_shield: bool,
    unsettled: bool,
}

/// A line's value: its roll, else a settled one.
fn value(v: &StatValue, rolled: Option<i64>) -> Option<i64> {
    if rolled.is_some() {
        return rolled;
    }
    match v {
        StatValue::Exact { min, max } if min == max => Some(*min),
        StatValue::Hidden { assumed } => *assumed,
        _ => None,
    }
}

/// The changes the radius jewels of `spec` make (module docs), by node,
/// leaving out the nodes in `conquered` (a Timeless jewel's).
#[must_use]
pub fn changes(
    build: &Build,
    spec: &PassiveSpec,
    tree: &Tree,
    rolls: &Rolls,
    conquered: &BTreeMap<u32, Conquest>,
    tattooed: &BTreeMap<u32, Conquest>,
) -> BTreeMap<u32, Conquest> {
    let allocated: BTreeSet<u32> = spec.nodes.value().iter().map(|n| n.0).collect();
    let mut adds: BTreeMap<u32, Vec<StatAmount>> = BTreeMap::new();
    let mut wiped: BTreeSet<u32> = BTreeSet::new();
    let mut socket_reasons: BTreeSet<u32> = BTreeSet::new();
    // A node's own stats with its life increases transformed (replacing them).
    let mut transformed: BTreeMap<u32, Vec<StatAmount>> = BTreeMap::new();
    for socket in spec.jewel_sockets.typed() {
        let at = socket.node.0;
        if !allocated.contains(&at) {
            continue;
        }
        let Some(item) = socket
            .item
            .and_then(|id| build.items.items().find(|i| i.id == id))
        else {
            continue;
        };
        let mut label = None;
        let mut f = Functions::default();
        let item_rolls = rolls.get(&item.id);
        for (n, line) in item.body.lines.iter().enumerate() {
            match &line.kind {
                LineKind::Spec { key, value, .. } if key == "Radius" => {
                    label = Some(value.as_str())
                }
                LineKind::Modifier(m) if m.selected => {
                    let LineResolution::Resolved {
                        result: Resolved::Stats(lines),
                        ..
                    } = &m.resolution
                    else {
                        continue;
                    };
                    let rolled = match item_rolls.and_then(|r| r.get(&n)) {
                        Some(loom_calc::input::LineValues::Rolled(v)) if v.len() == lines.len() => {
                            v.clone()
                        }
                        _ => vec![None; lines.len()],
                    };
                    for (s, roll) in lines.iter().zip(rolled) {
                        let id = s.stat.as_str();
                        if let Some(global) = id.strip_prefix(ALSO_GRANT) {
                            match value(&s.value, roll) {
                                Some(v) => f.also_grant.push(StatAmount {
                                    stat: global.to_owned(),
                                    value: v,
                                }),
                                None => f.unsettled = true,
                            }
                        } else if id == GRANT_NOTHING {
                            f.grant_nothing = true;
                        } else if id == COPY_UNALLOCATED {
                            f.copy_unallocated = true;
                        } else if id == LIFE_TO_ENERGY_SHIELD {
                            f.life_to_energy_shield = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if f.also_grant.is_empty()
            && !f.grant_nothing
            && !f.copy_unallocated
            && !f.life_to_energy_shield
            && !f.unsettled
        {
            continue;
        }
        let Some(ring) = label.and_then(|l| tree.radius_named(l)) else {
            socket_reasons.insert(at);
            continue;
        };
        if f.unsettled {
            socket_reasons.insert(at);
            continue;
        }
        let nodes = loom_tree::poe1::radius::in_ring(tree, at, ring, false);
        let mut copied: Vec<StatAmount> = Vec::new();
        for node in nodes {
            let Some(n) = tree.node(node) else { continue };
            if node & loom_tree::poe1::CLUSTER_NODE_BIT != 0 || n.ascendancy.is_some() {
                continue;
            }
            let taken = allocated.contains(&node);
            if taken && !conquered.contains_key(&node) {
                if matches!(n.kind, Kind::Notable | Kind::Normal) && !f.also_grant.is_empty() {
                    adds.entry(node)
                        .or_default()
                        .extend(f.also_grant.iter().cloned());
                }
                if f.grant_nothing && n.kind == Kind::Normal {
                    wiped.insert(node);
                }
                if f.life_to_energy_shield && n.kind != Kind::Mastery {
                    if n.stats
                        .iter()
                        .any(|s| LIFE_INCREASES_NOT_READ.contains(&s.stat.as_str()))
                        || tattooed.contains_key(&node)
                    {
                        socket_reasons.insert(at);
                    } else if n.stats.iter().any(|s| s.stat == LIFE_INCREASE.0) {
                        transformed.insert(
                            node,
                            n.stats
                                .iter()
                                .map(|s| StatAmount {
                                    stat: if s.stat == LIFE_INCREASE.0 {
                                        LIFE_INCREASE.1.to_owned()
                                    } else {
                                        s.stat.clone()
                                    },
                                    value: s.value,
                                })
                                .collect(),
                        );
                    }
                }
            }
            if !taken && f.copy_unallocated && n.kind == Kind::Normal {
                if conquered.contains_key(&node) {
                    socket_reasons.insert(at);
                }
                // A tattooed node gives the tattoo's stats (PoB applies the
                // override to the node's mods first).
                match tattooed.get(&node) {
                    Some(t) if t.reason.is_some() => {
                        socket_reasons.insert(at);
                    }
                    Some(t) => {
                        if !t.replaced {
                            copied.extend(n.stats.iter().cloned());
                        }
                        copied.extend(t.stats.iter().cloned());
                    }
                    None => copied.extend(n.stats.iter().cloned()),
                }
            }
        }
        if !copied.is_empty() {
            adds.entry(at).or_default().extend(copied);
        }
    }
    let mut out: BTreeMap<u32, Conquest> = BTreeMap::new();
    // A transformed node replaces its own stats; another jewel's additions
    // to it are kept.
    for (node, mut stats) in transformed {
        stats.extend(adds.remove(&node).unwrap_or_default());
        out.insert(
            node,
            Conquest {
                replaced: true,
                stats,
                reason: None,
            },
        );
    }
    for (node, stats) in adds {
        out.insert(
            node,
            Conquest {
                replaced: false,
                stats,
                reason: None,
            },
        );
    }
    for node in wiped {
        out.insert(
            node,
            Conquest {
                replaced: true,
                stats: Vec::new(),
                reason: None,
            },
        );
    }
    for node in socket_reasons {
        out.insert(
            node,
            Conquest {
                replaced: false,
                stats: Vec::new(),
                reason: Some("radius-jewel"),
            },
        );
    }
    out
}

//! Text to calc result (Phase 4 plan, decision 0): the one pipeline both
//! the application (`loom-app`) and the parity tools (`calc-run`) run, so
//! the Phase 3 parity evidence covers what the application shows.
//!
//! 1. [`read_pob`] reads PoB XML or a share code; [`read_document`] a
//!    build document of any version the migrations reach.
//! 2. [`resolve`] resolves the build's pending lines with the bundle and
//!    returns the exact ends of the ranges it read ([`RangeEnds`]).
//! 3. [`options`] gives the engine what the import adapter settles: each
//!    item's base name and PoB's line values (rolls, scalings) from those
//!    ends, the tree version the host knows for the data, and the active
//!    spec's passives resolved against the tree ([`resolve_passives`]:
//!    which nodes are connected; the engine does not walk the tree).
//! 4. [`calculate`] runs the engine with the host's cancellation hook;
//!    [`document`] writes the calc result document v1.
//!
//! [`resolve_item`] re-reads one item after an edit; [`reresolve`]
//! recovers the range ends of a build read with its resolutions.
//!
//! Core crate (ADR-016): no filesystem, clocks, threads, or environment.
//! Errors carry the failing layer's stable code (ADR-014) and the step.

use std::collections::{BTreeMap, BTreeSet};

pub mod radius_jewels;
pub mod timeless;

use loom_calc::input::Conquest;
use loom_calc::{
    BaseName, CalcData, CalcInput, CalcOptions, CalcResult, ClusterPassive, PackNode, PackStats,
    PassiveState, ResolvedPassives,
};
use loom_data::bundle::passives::StatAmount;
use loom_data::bundle::tattoos::Tattoos;
use loom_domain::{
    Build, ItemBody, ItemId, LineKind, LineResolution, NodeId, PassiveSpec, Resolved, StatValue,
    TreeVersion,
};
use loom_parser::import::{BundleData, ItemBaseName, ItemData as _, Resolver};
use loom_serialization::dto::calc::v1 as calc_dto;
use loom_serialization::legacy::{pob_item, pob_xml, share_code};
use loom_tree::Tree;
use loom_tree::poe1::cluster::{self, Jewel};
use loom_tree::poe1::radius::{Centre, Leap};

pub use loom_parser::import::{BuildRangeEnds as RangeEnds, LineRangeEnds};

/// Where the pipeline stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Reading the input.
    Read,
    /// Resolving its lines.
    Resolve,
    /// Calculating.
    Calculate,
    /// Writing the calc result document.
    Document,
}

impl Step {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Resolve => "resolve",
            Self::Calculate => "calculate",
            Self::Document => "document",
        }
    }
}

/// A step failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {code}: {message}", step.name())]
pub struct Error {
    /// The step.
    pub step: Step,
    /// The failing layer's stable code.
    pub code: &'static str,
    /// Detail in English.
    pub message: String,
}

impl Error {
    fn new(step: Step, code: &'static str, message: impl ToString) -> Self {
        Self {
            step,
            code,
            message: message.to_string(),
        }
    }
}

/// What a PoB input was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PobFormat {
    /// PoB's build XML.
    Xml,
    /// A share code (the XML, deflated, in URL-safe base64).
    ShareCode,
}

/// Which PoB format `text` is: XML when it starts with `<` (after
/// whitespace and a byte-order mark), else a share code.
#[must_use]
pub fn pob_format(text: &str) -> PobFormat {
    if text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('<')
    {
        PobFormat::Xml
    } else {
        PobFormat::ShareCode
    }
}

/// Reads PoB XML or a share code ([`pob_format`]).
///
/// # Errors
/// The import adapter's code ([`Step::Read`]).
pub fn read_pob(text: &str) -> Result<Build, Error> {
    match pob_format(text) {
        PobFormat::Xml => pob_xml::import(text).map_err(|e| Error::new(Step::Read, e.code(), &e)),
        PobFormat::ShareCode => {
            share_code::import(text.trim()).map_err(|e| Error::new(Step::Read, e.code(), &e))
        }
    }
}

/// Reads a build document of any version the shipped migrations reach.
///
/// # Errors
/// The failing layer's code (`MIGRATE_*`, `DTO_*`; [`Step::Read`]).
pub fn read_document(json: &str) -> Result<Build, Error> {
    loom_serialization::dto::read(json)
        .map(|loaded| loaded.build)
        .map_err(|e| Error::new(Step::Read, e.code(), &e))
}

/// The import adapter's base name of an item, as the resolver and the
/// engine take it.
fn base_name(body: &ItemBody, data: &BundleData<'_>) -> Result<ItemBaseName, String> {
    pob_item::item_base(body, data.base_names())
        .map(|(name, fallback)| ItemBaseName { name, fallback })
        .map_err(|e| e.name().to_owned())
}

/// Resolves the build's pending lines and records the bundle's data
/// version on it; the exact ends of every range the call resolved.
///
/// # Errors
/// The resolver's code (game or data version mismatch; [`Step::Resolve`]);
/// the build is then unchanged.
pub fn resolve(
    resolver: &Resolver<'_, '_>,
    data: &BundleData<'_>,
    build: &mut Build,
) -> Result<RangeEnds, Error> {
    resolver
        .resolve_build_ranged(build, |body| base_name(body, data))
        .map(|(_, ends)| ends)
        .map_err(|e| Error::new(Step::Resolve, e.code(), &e))
}

/// Resolves one item's pending lines (after an edit); the exact ends of
/// the ranges it read. The caller vouches that the item is of a build of
/// the bundle's game.
pub fn resolve_item(
    resolver: &Resolver<'_, '_>,
    data: &BundleData<'_>,
    body: &mut ItemBody,
) -> LineRangeEnds {
    let base = base_name(body, data);
    resolver.resolve_item_ranged(body, base.as_ref().map_err(String::as_str))
}

/// Sets every resolved or unresolved line (items and custom config lines)
/// back to pending; excluded lines keep their exclusion, as the import
/// adapter set them.
pub fn reset_resolutions(build: &mut Build) {
    let reset = |r: &mut LineResolution| {
        if !matches!(r, LineResolution::Excluded(_)) {
            *r = LineResolution::Pending;
        }
    };
    for entry in build.items.entries.typed_mut() {
        if let loom_domain::ItemsEntry::Item(item) = entry {
            for line in &mut item.body.lines {
                if let LineKind::Modifier(m) = &mut line.kind {
                    reset(&mut m.resolution);
                }
            }
        }
    }
    for set in build.config.sets.typed_mut() {
        for line in set.custom_modifiers.iter_mut().flat_map(|b| &mut b.lines) {
            reset(&mut line.resolution);
        }
        for line in set.inputs.typed_mut().flat_map(|i| &mut i.lines) {
            reset(&mut line.resolution);
        }
    }
}

/// What re-resolving a build read with its resolutions gave.
#[derive(Debug)]
pub enum Reresolved {
    /// The same resolutions: the range ends are the import's.
    Same(RangeEnds),
    /// Different resolutions (another resolver than the import's): the
    /// stored ones stand, and no ends are recovered.
    Changed,
}

/// Recovers the range ends of a build read with its resolutions (a build
/// document stores resolutions, not ends; Phase 4 plan, decision 7): a
/// clone with its lines reset to pending is resolved again and compared
/// with the build. The build itself is never changed.
///
/// # Errors
/// As [`resolve`] (another game or data version).
pub fn reresolve(
    resolver: &Resolver<'_, '_>,
    data: &BundleData<'_>,
    build: &Build,
) -> Result<Reresolved, Error> {
    let mut clone = build.clone();
    reset_resolutions(&mut clone);
    let ends = resolve(resolver, data, &mut clone)?;
    Ok(if clone == *build {
        Reresolved::Same(ends)
    } else {
        Reresolved::Changed
    })
}

/// "Passive Skills in Radius can be Allocated without being connected to
/// your tree" (Intuitive Leap, Thread of Hope).
const LEAP_STAT: &str = "local_unique_jewel_nearby_disconnected_passives_can_be_allocated";
/// The same for keystones only.
const LEAP_KEYSTONES_STAT: &str =
    "local_unique_jewel_nearby_disconnected_keystone_passives_can_be_allocated";
/// "Passive Skills in Radius of <keystone> can be Allocated without being
/// connected to your tree" (Impossible Escape): the keystone's hash.
const ESCAPE_STAT: &str =
    "local_unique_jewel_disconnected_passives_can_be_allocated_around_keystone_hash";
/// "Only affects Passives in <size> Ring" (Thread of Hope): the ring.
const RING_STAT: &str = "local_jewel_variable_ring_radius_value";

/// The one value a stat line settles, if it does.
fn settled(value: &StatValue) -> Vec<i64> {
    match value {
        StatValue::Exact { min, max } if min == max => vec![*min],
        StatValue::Hidden {
            assumed: Some(v), ..
        } => vec![*v],
        StatValue::Lookup(keys) => keys.clone(),
        _ => Vec::new(),
    }
}

/// The jewels socketed in `spec` that let nodes in their radius be
/// allocated without a path (passive tree plan, T-f), read from their
/// selected lines' stats (ADR-015): Intuitive Leap-like jewels around
/// their socket, Impossible Escape around its keystone (one leap per
/// keystone the line names). The radius is the ring the ring stat names,
/// else the radius the item's `Radius:` line names; a jewel whose radius
/// the tree's data does not have gives none (its nodes stay unconnected,
/// a reason, never a guess). A line that did not resolve gives nothing:
/// PoB's older wording ("Passives in Radius can be Allocated ...") is not
/// the game's text, and stays an `unresolved-line` reason.
#[must_use]
pub fn leaps(build: &Build, spec: &PassiveSpec, tree: &Tree) -> Vec<Leap> {
    let mut out = Vec::new();
    for socket in spec.jewel_sockets.typed() {
        let Some(item) = socket
            .item
            .and_then(|id| build.items.items().find(|i| i.id == id))
        else {
            continue;
        };
        let mut label = None;
        let (mut leap, mut keystones_only, mut ring) = (false, false, None);
        let mut keystones: Vec<u32> = Vec::new();
        for line in &item.body.lines {
            match &line.kind {
                LineKind::Spec { key, value, .. } if key == "Radius" => {
                    label = Some(value.as_str())
                }
                LineKind::Modifier(m) if m.selected => {
                    let LineResolution::Resolved {
                        result: Resolved::Stats(stats),
                        ..
                    } = &m.resolution
                    else {
                        continue;
                    };
                    for s in stats {
                        match s.stat.as_str() {
                            LEAP_STAT => leap = true,
                            LEAP_KEYSTONES_STAT => keystones_only = true,
                            ESCAPE_STAT => keystones.extend(
                                settled(&s.value)
                                    .into_iter()
                                    .filter_map(|k| u32::try_from(k).ok()),
                            ),
                            RING_STAT => ring = settled(&s.value).first().copied(),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        let Some(radius) = ring
            .and_then(|k| tree.ring(k))
            .or_else(|| label.and_then(|l| tree.radius_named(l)))
        else {
            continue;
        };
        if leap || keystones_only {
            out.push(Leap {
                socket: socket.node.0,
                ring: radius,
                centre: Centre::Socket,
                keystones_only: keystones_only && !leap,
            });
        }
        for k in keystones {
            out.push(Leap {
                socket: socket.node.0,
                ring: radius,
                centre: Centre::Keystone(k),
                keystones_only: false,
            });
        }
    }
    out
}

// The tree and the engine name cluster jewel nodes by the same bit.
const _: () = assert!(loom_tree::poe1::CLUSTER_NODE_BIT == PassiveState::CLUSTER_NODE_BIT);

/// "Adds # Passive Skills".
const CLUSTER_NODE_COUNT: &str = "local_jewel_expansion_passive_node_count";
/// "# Added Passive Skills are Jewel Sockets".
const CLUSTER_SOCKETS: &str = "local_jewel_expansion_jewels_count";
/// "Adds # Jewel Socket Passive Skills".
const CLUSTER_SOCKETS_OVERRIDE: &str = "local_jewel_expansion_jewels_count_override";
/// "Added Small Passive Skills grant: ...": the skill's row.
const CLUSTER_SKILL: &str = "local_jewel_expansion_passive_node_index";
/// "Adds # Small Passive Skills which grant nothing".
const CLUSTER_NOTHINGNESS: &str = "local_unique_jewel_grants_x_empty_passives";
/// "Added Small Passive Skills grant Nothing".
const CLUSTER_SMALLS_NOTHING: &str = "local_affliction_jewel_display_small_nodes_grant_nothing";
/// "Added Small Passive Skills have #% increased Effect".
const CLUSTER_EFFECT: &str = "local_affliction_jewel_small_nodes_have_effect_+%";
/// "Added Small Passive Skills also grant: ...": each such stat's prefix.
const CLUSTER_ALSO_GRANT: &str = "local_affliction_jewel_small_nodes_grant_";

/// The cluster jewels socketed in `spec` (passive tree plan, T-i), read
/// from their selected lines' stats (ADR-015), by socket: a jewel whose
/// base is a cluster jewel base in the tree's data. An "also grant" line
/// gives each small passive the stat the data pairs with it
/// ([`loom_data::bundle::poe1::ClusterJewels::grant`]). A jewel with a
/// line that did not resolve, or an "also grant" stat the data does not
/// pair, is left out: its nodes then stay a reason (`cluster-jewel-node`),
/// never a guess.
#[must_use]
pub fn cluster_jewels(build: &Build, spec: &PassiveSpec, tree: &Tree) -> BTreeMap<u32, Jewel> {
    let mut out = BTreeMap::new();
    let Some(data) = &tree.cluster_jewels else {
        return out;
    };
    for socket in spec.jewel_sockets.typed() {
        let Some(item) = socket
            .item
            .and_then(|id| build.items.items().find(|i| i.id == id))
        else {
            continue;
        };
        let names = tree.cluster_bases.keys().map(String::as_str);
        let Some(base) =
            pob_item::item_base(&item.body, names)
                .ok()
                .and_then(|(name, fallback)| {
                    tree.cluster_bases
                        .get(&name)
                        .or_else(|| fallback.and_then(|f| tree.cluster_bases.get(&f)))
                })
        else {
            continue;
        };
        if let Some(jewel) = cluster_jewel(item, base, data)
            && jewel.valid(data)
        {
            out.insert(socket.node.0, jewel);
        }
    }
    out
}

/// One cluster jewel's lines as a [`Jewel`] ([`cluster_jewels`]); `None`
/// for a line it cannot read.
fn cluster_jewel(
    item: &loom_domain::Item,
    base: &str,
    data: &loom_data::bundle::poe1::ClusterJewels,
) -> Option<Jewel> {
    let mut jewel = Jewel {
        base: base.to_owned(),
        ..Jewel::default()
    };
    for line in &item.body.lines {
        let LineKind::Modifier(m) = &line.kind else {
            continue;
        };
        if !m.selected {
            continue;
        }
        let LineResolution::Resolved {
            result: Resolved::Stats(stats),
            ..
        } = &m.resolution
        else {
            return None;
        };
        for s in stats {
            let one = settled(&s.value).first().copied();
            let count = one.and_then(|v| u32::try_from(v).ok());
            match s.stat.as_str() {
                CLUSTER_NODE_COUNT => jewel.node_count = count,
                CLUSTER_SOCKETS => jewel.socket_count = count,
                CLUSTER_SOCKETS_OVERRIDE => jewel.socket_override = count,
                CLUSTER_NOTHINGNESS => jewel.nothingness = count,
                CLUSTER_SKILL => jewel.skill = one.and_then(|v| usize::try_from(v).ok()),
                CLUSTER_SMALLS_NOTHING => jewel.smalls_nothing = true,
                CLUSTER_EFFECT => jewel.effect = one.unwrap_or(0),
                other if other.starts_with(CLUSTER_ALSO_GRANT) => {
                    // The granted stat at the line's value; a stat the
                    // data does not pair, or a value the line does not
                    // settle, leaves the jewel unbuilt.
                    let granted = data.grant(other)?;
                    jewel.added.push(StatAmount {
                        stat: granted.to_owned(),
                        value: one?,
                    });
                }
                other => match data.special(other) {
                    Some(sp) if sp.keystone => jewel.keystone = u32::try_from(sp.node).ok(),
                    Some(sp) => jewel.notables.extend(u32::try_from(sp.node).ok()),
                    None => {}
                },
            }
        }
    }
    Some(jewel)
}

/// The active spec's allocated nodes resolved against `tree` (passive
/// tree plan, T-e): connected to an allocated start
/// ([`loom_tree::poe1::connected_to_start`]) or held by a radius jewel
/// ([`leaps`], [`loom_tree::poe1::radius::held`]), a cluster jewel's
/// socket or node, or not connected. The socketed cluster jewels
/// ([`cluster_jewels`]) first add their nodes to the tree
/// ([`loom_tree::poe1::cluster::expand`], T-i): an allocated node one
/// adds resolves as any other, with its stats in
/// [`ResolvedPassives::cluster`]; one no jewel adds stays a cluster jewel
/// node (a reason). Without a tree, empty: the engine then counts every
/// node as not connected, each with the visible `unconnected-node` reason
/// (a bundle without passive tables has no node's stats either; the app
/// and `calc-run` pass the bundle's tree whenever it has one).
#[must_use]
pub fn resolve_passives(build: &Build, tree: Option<&Tree>) -> ResolvedPassives {
    resolve_passives_with(build, tree, None, None, &timeless::Rolls::new())
}

/// [`resolve_passives`] with the Timeless jewels' data the host read
/// ([`timeless::TimelessData`], P6-10a): the nodes a socketed Timeless
/// jewel conquers ([`timeless::conquests`]); without it, each such node is
/// a reason.
#[must_use]
pub fn resolve_passives_with(
    build: &Build,
    tree: Option<&Tree>,
    timeless: Option<&timeless::TimelessData<'_>>,
    tattoos: Option<&Tattoos>,
    rolls: &timeless::Rolls,
) -> ResolvedPassives {
    let (Some(tree), Some(spec)) = (tree, loom_calc::poe1::collect::active_spec(build)) else {
        return ResolvedPassives::default();
    };
    let pack = tree.version.is_some().then(|| pack_stats(tree));
    let legacy = spec.cluster_hash_format() < 2;
    let subgraphs = cluster::subgraphs_for(tree, &cluster_jewels(build, spec, tree), legacy);
    let expanded;
    let tree = if subgraphs.is_empty() {
        tree
    } else {
        expanded = cluster::expand(tree, &subgraphs);
        &expanded
    };
    let in_subgraph: BTreeMap<u32, &cluster::ClusterNode> = subgraphs
        .iter()
        .flat_map(|g| &g.nodes)
        .map(|n| (n.id, n))
        .collect();
    // A legacy spec's numbers as PoB renumbers them (T-i); the states are
    // given under the spec's own numbers.
    let map = cluster::legacy_map(&subgraphs);
    let now = |n: NodeId| cluster::remapped(tree, &map, n.0);
    let nodes = spec.nodes.value();
    let current: Vec<u32> = nodes.iter().map(|n| now(*n)).collect();
    let mut connected = loom_tree::poe1::connected_to_start(tree, current.iter());
    let allocated: std::collections::BTreeSet<u32> = current
        .iter()
        .copied()
        .filter(|n| n & loom_tree::poe1::CLUSTER_NODE_BIT == 0)
        .collect();
    let held =
        loom_tree::poe1::radius::held(tree, &leaps(build, spec, tree), &allocated, &connected);
    connected.extend(held);
    // Tattoos and the other overrides (P6-11av), by the spec's numbers.
    let overrides = overrides(spec);
    let tattooed: BTreeSet<u32> = overrides.keys().map(|n| now(NodeId(*n))).collect();
    let mut conquered = timeless::conquests(build, spec, tree, timeless, rolls, &tattooed);
    // Radius jewels that change the nodes in their radius (P6-11gv): not a
    // node a Timeless jewel conquers.
    // An unallocated tattooed node gives its tattoo's stats to a jewel that
    // copies it.
    let tattoo_stats: BTreeMap<u32, Conquest> = overrides
        .iter()
        .map(|(n, name)| (now(NodeId(*n)), tattoo(tattoos, name)))
        .collect();
    let radius = radius_jewels::changes(build, spec, tree, rolls, &conquered, &tattoo_stats);
    conquered.extend(radius);
    let is_cluster = |n: u32| n & loom_tree::poe1::CLUSTER_NODE_BIT != 0;
    ResolvedPassives {
        nodes: nodes
            .iter()
            .map(|n| {
                let c = now(*n);
                let state = if is_cluster(c) && !in_subgraph.contains_key(&c) {
                    PassiveState::ClusterNode
                } else if connected.contains(&c) {
                    PassiveState::Connected
                } else if tree.node(c).is_some_and(|x| x.expansion) && !in_subgraph.contains_key(&c)
                {
                    PassiveState::ClusterSocket
                } else {
                    PassiveState::Unconnected
                };
                (*n, state)
            })
            .collect(),
        cluster: nodes
            .iter()
            .filter(|n| is_cluster(now(**n)))
            .filter_map(|n| {
                let c = in_subgraph.get(&now(*n))?;
                Some((
                    *n,
                    ClusterPassive {
                        stats: c.stats.clone(),
                        effect: c.effect,
                    },
                ))
            })
            .collect(),
        pack,
        changed: nodes
            .iter()
            .filter_map(|n| {
                let tattoo = overrides.get(&n.0).map(|name| tattoo(tattoos, name));
                let change = match (tattoo, conquered.get(&now(*n)).cloned()) {
                    (None, None) => return None,
                    (Some(t), None) => t,
                    (None, Some(c)) => c,
                    (Some(t), Some(c)) => over_tattoo(t, c),
                };
                Some((*n, change))
            })
            .collect(),
    }
}

/// A spec's tattoos, runegrafts, and other overrides (PoB's `<Overrides>`,
/// which the XML reader keeps as read): each `<Override>`'s node and name
/// (`nodeId`, `dn`; PoB's `PassiveSpec.lua` 148 to 175). The text inside an
/// override is PoB's display, not read.
fn overrides(spec: &PassiveSpec) -> BTreeMap<u32, String> {
    use loom_domain::{Entry, OpaqueNode};
    let mut out = BTreeMap::new();
    for entry in spec.layout.entries() {
        let Entry::Opaque(OpaqueNode::Element { name, children, .. }) = entry else {
            continue;
        };
        if name != "Overrides" {
            continue;
        }
        for child in children {
            let OpaqueNode::Element {
                name, attributes, ..
            } = child
            else {
                continue;
            };
            if name != "Override" {
                continue;
            }
            let get = |k: &str| {
                attributes
                    .iter()
                    .find(|a| a.name == k)
                    .map(|a| a.value.as_str())
            };
            if let (Some(node), Some(dn)) =
                (get("nodeId").and_then(|v| v.trim().parse().ok()), get("dn"))
            {
                out.insert(node, dn.to_owned());
            }
        }
    }
    out
}

/// What the override named `name` makes of its node: its stats in place
/// of the node's, or the reason it is not known (module docs of
/// [`loom_data::bundle::tattoos`]).
fn tattoo(tattoos: Option<&Tattoos>, name: &str) -> Conquest {
    let reason = |what| Conquest {
        replaced: true,
        stats: Vec::new(),
        reason: Some(what),
    };
    let Some(t) = tattoos else {
        return reason("tattoo-data");
    };
    match t.named(name) {
        None => reason("tattoo"),
        Some(o) if o.grants_skill => reason("tattoo-skill"),
        Some(o) => Conquest {
            replaced: true,
            stats: o.stats.clone(),
            reason: None,
        },
    }
}

/// A Timeless jewel's conquest of a tattooed node: PoB applies the tattoo
/// first (`PassiveSpec.lua` 1187), so a replacement takes its place and an
/// addition adds to it.
fn over_tattoo(tattoo: Conquest, conquest: Conquest) -> Conquest {
    if tattoo.reason.is_some() {
        return tattoo;
    }
    if conquest.reason.is_some() || conquest.replaced {
        return conquest;
    }
    let mut stats = tattoo.stats;
    stats.extend(conquest.stats);
    Conquest {
        replaced: true,
        stats,
        reason: None,
    }
}

/// A pack tree's stats for the engine ([`ResolvedPassives::pack`]; passive
/// tree plan, T4): every node's and mastery effect's, with the lines the
/// pack could not read.
fn pack_stats(tree: &Tree) -> PackStats {
    let node = |stats: &[StatAmount],
                kind: loom_calc::input::PassiveKind,
                unresolved: Option<&Vec<String>>| PackNode {
        stats: stats.to_vec(),
        kind,
        unresolved: unresolved.cloned().unwrap_or_default(),
    };
    PackStats {
        passives: tree
            .nodes
            .iter()
            .map(|(n, x)| {
                let kind = loom_calc::input::PassiveKind {
                    notable: x.kind == loom_tree::Kind::Notable,
                    ascendancy: x.ascendancy.is_some(),
                    life_mastery: x.kind == loom_tree::Kind::Mastery
                        && x.name == loom_calc::input::LIFE_MASTERY,
                };
                (NodeId(*n), node(&x.stats, kind, tree.unresolved.get(n)))
            })
            .collect(),
        mastery_effects: tree
            .effect_stats
            .iter()
            .map(|(e, stats)| {
                let unresolved = tree.effect_unresolved.get(e);
                (
                    *e,
                    node(stats, loom_calc::input::PassiveKind::default(), unresolved),
                )
            })
            .collect(),
    }
}

/// The bundle's older tree versions ([`loom_data::bundle::tree_packs`]),
/// each built as a tree ([`Tree::from_pack`]) on `base`, the bundle's own
/// tree, by version.
#[must_use]
pub fn pack_trees(bundle: &loom_data::bundle::Bundle, base: &Tree) -> BTreeMap<String, Tree> {
    bundle
        .tree_packs
        .iter()
        .filter_map(|(v, pack)| Some((v.clone(), Tree::from_pack(bundle, base, pack)?)))
        .collect()
}

/// The tree to calculate `build` on and its version (passive tree plan,
/// T4): the data's own (`own`, of `version`), unless the active spec names
/// another version the bundle carries as a pack (`packs`, [`pack_trees`]),
/// then that one, so the spec reads the stats of its own tree as PoB does.
/// A spec of a version with no pack keeps the data's tree, and the
/// engine's version check gives its reason.
#[must_use]
pub fn pick_tree<'a>(
    build: &Build,
    own: Option<&'a Tree>,
    version: Option<TreeVersion>,
    packs: &'a BTreeMap<String, Tree>,
) -> (Option<&'a Tree>, Option<TreeVersion>) {
    let named = loom_calc::poe1::collect::active_spec(build).and_then(|s| s.tree_version.as_ref());
    match named {
        Some(v) if Some(v) != version.as_ref() => match packs.get(v.as_str()) {
            Some(tree) => (Some(tree), Some(v.clone())),
            None => (own, version),
        },
        _ => (own, version),
    }
}

/// Renumbers each spec of `build` numbered as PoB numbered cluster nodes
/// before its hash fix (`clusterHashFormatVersion` 1; passive tree plan,
/// T-i) as PoB does when it loads one (`PassiveSpec.lua` 1651 to 1720):
/// its allocated nodes and its jewel sockets take their current numbers
/// ([`loom_tree::poe1::cluster::remapped`]) and the spec is marked 2, in
/// one change (ADR-009). A tree without cluster jewel data renumbers
/// nothing, and a spec of another tree version than `version` (when both
/// are known) is left as it is: PoB then renumbers it when it reads the
/// export. Returns whether a spec changed.
pub fn migrate_cluster_hashes(
    build: &mut Build,
    tree: &Tree,
    version: Option<&TreeVersion>,
) -> bool {
    if tree.cluster_jewels.is_none() {
        return false;
    }
    let mut plans = Vec::new();
    for (k, spec) in build.passives.specs.typed().enumerate() {
        let other_version = matches!(
            (spec.tree_version.as_ref(), version),
            (Some(a), Some(b)) if a != b
        );
        if spec.cluster_hash_format() >= 2 || other_version {
            continue;
        }
        let subgraphs = cluster::subgraphs_for(tree, &cluster_jewels(build, spec, tree), true);
        let expanded = cluster::expand(tree, &subgraphs);
        let map = cluster::legacy_map(&subgraphs);
        let now = |n: NodeId| NodeId(cluster::remapped(&expanded, &map, n.0));
        let mut seen = std::collections::BTreeSet::new();
        let nodes: Vec<NodeId> = spec
            .nodes
            .value()
            .iter()
            .map(|n| now(*n))
            .filter(|n| seen.insert(*n))
            .collect();
        let sockets: Vec<NodeId> = spec.jewel_sockets.typed().map(|j| now(j.node)).collect();
        plans.push((k, nodes, sockets));
    }
    let changed = !plans.is_empty();
    for (k, nodes, sockets) in plans {
        let Some(spec) = build.passives.specs.typed_mut().nth(k) else {
            continue;
        };
        if nodes != *spec.nodes.value() {
            spec.nodes.set(nodes);
        }
        for (j, node) in spec.jewel_sockets.typed_mut().zip(sockets) {
            j.node = node;
        }
        spec.set_cluster_hash_format(2);
    }
    changed
}

/// The implicit tags of the bundle's unique mod rows, by their stats
/// (sorted, those with a value other than `(0, 0)`), each the union over
/// the rows with those stats ([`pob_item::line_values_full`]; the agent
/// list, S9). The unique rows are those of the generation types the item
/// rules fit to a unique's explicit lines. Empty for a bundle before data
/// schema 3, whose mods carry no tags.
fn unique_row_tags(bundle: &loom_data::bundle::Bundle) -> BTreeMap<Vec<String>, BTreeSet<String>> {
    let mut out: BTreeMap<Vec<String>, BTreeSet<String>> = BTreeMap::new();
    if bundle.manifest.data_schema < loom_calc::input::IMPLICIT_TAGS_SCHEMA {
        return out;
    }
    let generations: BTreeSet<i64> = bundle
        .item_rules
        .mods
        .fits
        .iter()
        .filter(|f| f.unique == Some(true) && f.section == loom_domain::ModSection::Explicit)
        .flat_map(|f| f.clauses.iter().flat_map(|c| c.generations.iter().copied()))
        .collect();
    for row in bundle
        .mods
        .iter()
        .filter(|m| generations.contains(&m.generation))
    {
        let mut key: Vec<String> = row
            .stats
            .iter()
            .filter(|s| (s.min, s.max) != (0, 0))
            .map(|s| s.stat.clone())
            .collect();
        key.sort_unstable();
        key.dedup();
        if key.is_empty() {
            continue;
        }
        out.entry(key)
            .or_default()
            .extend(row.implicit_tags.iter().cloned());
    }
    out
}

/// The engine's options for `build`: each item's base name and PoB's line
/// values from the range ends, the data's tree version, if the host knows
/// it, and the active spec's passives resolved against `tree`
/// ([`resolve_passives`]).
#[must_use]
pub fn options(
    build: &Build,
    data: &BundleData<'_>,
    ends: &RangeEnds,
    tree_version: Option<TreeVersion>,
    tree: Option<&Tree>,
) -> CalcOptions {
    options_with(build, data, ends, tree_version, tree, None)
}

/// [`options`] with the Timeless jewels' data the host read (P6-10a,
/// [`resolve_passives_with`]).
#[must_use]
pub fn options_with(
    build: &Build,
    data: &BundleData<'_>,
    ends: &RangeEnds,
    tree_version: Option<TreeVersion>,
    tree: Option<&Tree>,
    timeless: Option<&timeless::TimelessData<'_>>,
) -> CalcOptions {
    // The same base names the resolver used, for the engine's armour
    // pieces (the legacy format's naming rules stay in the adapter).
    let item_bases = build
        .items
        .items()
        .filter_map(|item| {
            pob_item::item_base(&item.body, data.base_names())
                .ok()
                .map(|(name, fallback)| (item.id, BaseName { name, fallback }))
        })
        .collect();
    // PoB's rules for values the text does not settle (rolls, scalings):
    // the adapter's, from the ranges' exact ends the resolver read, and
    // the stats' scalability for catalysts (P6-11ax).
    let scalability: BTreeMap<&str, bool> = data
        .bundle()
        .stats
        .iter()
        .filter_map(|s| Some((s.id.as_str(), s.scalable?)))
        .collect();
    let scalable = |stat: &str| scalability.get(stat).copied();
    // The unique rows' tags, read only when an untagged line on a
    // catalysed unique asks for them.
    let unique_rows = std::cell::OnceCell::new();
    let unique_tags = |stats: &[&str]| -> Option<Vec<String>> {
        let mut key: Vec<String> = stats.iter().map(|s| (*s).to_owned()).collect();
        key.sort_unstable();
        key.dedup();
        unique_rows
            .get_or_init(|| unique_row_tags(data.bundle()))
            .get(&key)
            .map(|t: &BTreeSet<String>| t.iter().cloned().collect())
    };
    let item_lines = build
        .items
        .items()
        .filter_map(|item| {
            let item_ends: pob_item::LineEnds = ends
                .get(&item.id)
                .map(|lines| {
                    lines
                        .iter()
                        .map(|(n, stats)| {
                            (
                                *n,
                                stats.iter().map(|e| e.map(|e| (e.from, e.to))).collect(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let values = pob_item::line_values_full(item, &item_ends, &scalable, &unique_tags);
            (!values.is_empty()).then_some((item.id, values))
        })
        .collect();
    let tattoos = data.bundle().poe1.as_ref().and_then(|p| p.tattoos.as_ref());
    let passives = resolve_passives_with(build, tree, timeless, tattoos, &item_lines);
    CalcOptions {
        tree_version,
        item_bases,
        item_lines,
        passives,
        ..CalcOptions::default()
    }
}

/// Calculates `build` with the engine, polling `cancel`.
///
/// # Errors
/// The engine's code (`CALC_*`, cancellation included; [`Step::Calculate`]).
pub fn calculate(
    build: &Build,
    data: &dyn CalcData,
    options: &CalcOptions,
    cancel: &dyn Fn() -> bool,
) -> Result<CalcResult, Error> {
    let input = CalcInput {
        build,
        data,
        options,
    };
    loom_calc::calculate_cancellable(&input, cancel)
        .map_err(|e| Error::new(Step::Calculate, e.code(), &e))
}

/// The values of the items `build` has in use ([`loom_calc::item_values`]:
/// the item card's defences, block, and weapon values).
///
/// # Errors
/// The engine's code ([`Step::Calculate`]).
pub fn item_values(
    build: &Build,
    data: &dyn CalcData,
    options: &CalcOptions,
) -> Result<loom_calc::ItemValues, Error> {
    let input = CalcInput {
        build,
        data,
        options,
    };
    loom_calc::item_values(&input).map_err(|e| Error::new(Step::Calculate, e.code(), &e))
}

/// The calc result document v1 of a result, as JSON.
///
/// # Errors
/// The document layer's code ([`Step::Document`]).
pub fn document(result: &CalcResult) -> Result<String, Error> {
    calc_dto::to_json(result).map_err(|e| Error::new(Step::Document, e.code(), &e))
}

/// Range ends with the items that no longer exist removed and one item's
/// replaced (after an item edit).
#[must_use]
pub fn with_item_ends(ends: &RangeEnds, item: ItemId, lines: LineRangeEnds) -> RangeEnds {
    let mut out: RangeEnds = ends.clone();
    if lines.is_empty() {
        out.remove(&item);
    } else {
        out.insert(item, lines);
    }
    out
}

/// Every item's base name, by ID (for display).
#[must_use]
pub fn base_names(build: &Build, data: &BundleData<'_>) -> BTreeMap<ItemId, String> {
    build
        .items
        .items()
        .filter_map(|item| {
            pob_item::item_base(&item.body, data.base_names())
                .ok()
                .map(|(name, _)| (item.id, name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_data::bundle::tattoos::Override;
    use loom_domain::{Attribute, OpaqueNode, SpecId};

    fn amount(stat: &str, value: i64) -> StatAmount {
        StatAmount {
            stat: stat.to_owned(),
            value,
        }
    }

    /// A spec's `<Overrides>`: each `<Override>`'s node and name, the
    /// display text inside it not read.
    #[test]
    fn overrides_should_read_each_node_and_name() {
        let mut spec = PassiveSpec::new(SpecId(1));
        let child = |node: &str, dn: &str| OpaqueNode::Element {
            name: "Override".to_owned(),
            attributes: vec![Attribute::new("nodeId", node), Attribute::new("dn", dn)],
            children: vec![OpaqueNode::Text("+6% to Fire Resistance".to_owned())],
        };
        spec.layout.push_opaque(OpaqueNode::Element {
            name: "Overrides".to_owned(),
            attributes: Vec::new(),
            children: vec![
                child("37671", "Tattoo of the Ngamahu Firewalker"),
                child("x", "bad"),
            ],
        });
        let got = overrides(&spec);
        assert_eq!(
            got.into_iter().collect::<Vec<_>>(),
            [(37671, "Tattoo of the Ngamahu Firewalker".to_owned())]
        );
    }

    /// A tattoo's stats replace the node's; a name the data does not hold,
    /// an override granting a skill, or no data is a reason; a conquest
    /// that replaces wins over the tattoo, one that adds goes on top.
    #[test]
    fn tattoos_should_replace_and_conquests_apply_after_them() {
        let o = |name: &str, skill| Override {
            id: name.to_owned(),
            name: name.to_owned(),
            kind: String::new(),
            stats: vec![amount("base_fire_damage_resistance_%", 6)],
            passive: None,
            grants_skill: skill,
        };
        let t = Tattoos {
            overrides: vec![o("Fire", false), o("Skill", true)],
        };
        let fire = tattoo(Some(&t), "Fire");
        assert!(fire.replaced && fire.reason.is_none());
        assert_eq!(tattoo(Some(&t), "Skill").reason, Some("tattoo-skill"));
        assert_eq!(tattoo(Some(&t), "None").reason, Some("tattoo"));
        assert_eq!(tattoo(None, "Fire").reason, Some("tattoo-data"));
        let added = Conquest {
            replaced: false,
            stats: vec![amount("base_strength", 2)],
            reason: None,
        };
        let both = over_tattoo(fire.clone(), added);
        assert!(both.replaced);
        assert_eq!(
            both.stats,
            vec![
                amount("base_fire_damage_resistance_%", 6),
                amount("base_strength", 2)
            ]
        );
        let replaced = Conquest {
            replaced: true,
            stats: vec![amount("base_devotion", 10)],
            reason: None,
        };
        assert_eq!(over_tattoo(fire, replaced.clone()), replaced);
    }
}

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

use std::collections::BTreeMap;

use loom_calc::{
    BaseName, CalcData, CalcInput, CalcOptions, CalcResult, ClusterPassive, PassiveState,
    ResolvedPassives,
};
use loom_domain::{
    Build, ItemBody, ItemId, LineKind, LineResolution, PassiveSpec, Resolved, StatValue,
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
/// "Added Small Passive Skills also grant: ...": the granted stat's line.
const CLUSTER_ALSO_GRANT: &str = "local_affliction_jewel_small_nodes_grant_";

/// The cluster jewels socketed in `spec` (passive tree plan, T-i), read
/// from their selected lines' stats (ADR-015), by socket: a jewel whose
/// base is a cluster jewel base in the tree's data. A jewel with a line
/// that did not resolve, or with an "also grant" line (the granted stat
/// is not yet read from the game's line), is left out: its nodes then
/// stay a reason (`cluster-jewel-node`), never a guess.
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
                other if other.starts_with(CLUSTER_ALSO_GRANT) => return None,
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
    let (Some(tree), Some(spec)) = (tree, loom_calc::poe1::collect::active_spec(build)) else {
        return ResolvedPassives::default();
    };
    let subgraphs = cluster::subgraphs(tree, &cluster_jewels(build, spec, tree));
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
    let nodes = spec.nodes.value();
    let mut connected = loom_tree::poe1::connected_to_start(tree, nodes.iter().map(|n| &n.0));
    let allocated: std::collections::BTreeSet<u32> = nodes
        .iter()
        .map(|n| n.0)
        .filter(|n| n & loom_tree::poe1::CLUSTER_NODE_BIT == 0)
        .collect();
    let held =
        loom_tree::poe1::radius::held(tree, &leaps(build, spec, tree), &allocated, &connected);
    connected.extend(held);
    let is_cluster = |n: u32| n & loom_tree::poe1::CLUSTER_NODE_BIT != 0;
    ResolvedPassives {
        nodes: nodes
            .iter()
            .map(|n| {
                let state = if is_cluster(n.0) && !in_subgraph.contains_key(&n.0) {
                    PassiveState::ClusterNode
                } else if connected.contains(&n.0) {
                    PassiveState::Connected
                } else if tree.node(n.0).is_some_and(|x| x.expansion)
                    && !in_subgraph.contains_key(&n.0)
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
            .filter(|n| is_cluster(n.0))
            .filter_map(|n| {
                let c = in_subgraph.get(&n.0)?;
                Some((
                    *n,
                    ClusterPassive {
                        stats: c.stats.clone(),
                        effect: c.effect,
                    },
                ))
            })
            .collect(),
    }
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
    // the adapter's, from the ranges' exact ends the resolver read.
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
            let values = pob_item::line_values(item, &item_ends);
            (!values.is_empty()).then_some((item.id, values))
        })
        .collect();
    CalcOptions {
        tree_version,
        item_bases,
        item_lines,
        passives: resolve_passives(build, tree),
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

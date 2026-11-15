//! The passive tree editor's application side (passive tree plan, T5a):
//! the loaded data's tree and its layout DTO, a build's spec as the rules
//! see it, and an edited spec written back.
//!
//! A spec's class and ascendancy are PoB's `classId` and `ascendClassId`,
//! typed by the domain's accessors ([`PassiveSpec::class`]) over the
//! attributes they are kept in and edited in place, so an unedited spec is
//! written byte for byte (the plan's T-h). An edited spec gets:
//! - its nodes and mastery effects written in ascending order, starts
//!   included, as PoB lists them;
//! - its `classId` and `ascendClassId` set;
//! - its `<URL>` child, when it has one, regenerated (`loom_tree::poe1::url`),
//!   or removed when the format cannot hold the spec;
//! - when it is the active spec, the build's class and ascendancy names
//!   set to match, as PoB's `Load` expects.

use std::collections::BTreeMap;

use loom_data::bundle::Bundle;
use loom_data::stat_desc::render::{NotRenderedReason, StatValue, VariantFilter};
use loom_data::stat_desc::scope::Scope;
use loom_domain::{
    AscendancyId, Attributes, Build, ClassId, Entry, ItemId, JewelSocket, MasteryEffect, NodeId,
    OpaqueNode, PassiveSpec, SpecId, SpecPart, TreeVersion,
};
use loom_serialization::dto::app::v1 as api;
use loom_tree::poe1::{self, EditError, Spec};
use loom_tree::{Kind, Tree};

use crate::error::{AppError, Code};

/// The stat descriptions the tree's lines are written with.
const PASSIVE_DESCRIPTIONS: &str = "passive_skill_stat_descriptions";

/// The ascendancy name PoB writes for none.
const NO_ASCENDANCY: &str = "None";

fn tree_error(e: &EditError) -> AppError {
    AppError::new(Code::Tree, e.to_string())
}

/// The spec `id` of the build.
///
/// # Errors
/// `APP_TREE` when the build has none.
pub fn spec_of(build: &Build, id: u32) -> Result<&PassiveSpec, AppError> {
    build
        .passives
        .specs
        .typed()
        .find(|s| s.id.0 == id)
        .ok_or_else(|| AppError::new(Code::Tree, format!("no passive spec {id}")))
}

/// Whether `spec` is the build's active one.
fn is_active(build: &Build, spec: SpecId) -> bool {
    loom_calc::poe1::collect::active_spec(build).is_some_and(|s| s.id == spec)
}

/// The spec's class: `classId`, else, for the active spec, the build's
/// class by name; `None` when neither names one of the tree's classes.
fn class_of(build: &Build, spec: &PassiveSpec, tree: &Tree) -> Option<u32> {
    spec.class()
        .or_else(|| {
            let name = build.character.class.as_ref()?;
            is_active(build, spec.id)
                .then(|| tree.classes.iter().find(|c| c.name == name.as_str()))
                .flatten()
                .map(|c| c.number)
        })
        .filter(|c| tree.class(*c).is_some())
}

/// The spec's ascendancy within `class`: `ascendClassId`, else, for the
/// active spec, the build's ascendancy by name, else 0 (none).
fn ascendancy_of(build: &Build, spec: &PassiveSpec, tree: &Tree, class: u32) -> u32 {
    spec.ascendancy()
        .or_else(|| {
            let name = build.character.ascendancy.as_ref()?;
            is_active(build, spec.id)
                .then(|| tree.class(class))
                .flatten()?
                .ascendancies
                .iter()
                .find(|a| a.name == name.as_str())
                .map(|a| a.number)
        })
        .unwrap_or(0)
}

/// Why spec `spec` cannot be edited against `tree` (of version `data`),
/// if it cannot: its tree version and the data's are both known and
/// differ (converting versions is a later step), or its class is not known.
#[must_use]
pub fn locked(
    build: &Build,
    spec: &PassiveSpec,
    tree: &Tree,
    data: Option<&TreeVersion>,
) -> Option<String> {
    if let (Some(own), Some(data)) = (spec.tree_version.as_ref(), data)
        && own != data
    {
        return Some(format!(
            "the spec is for tree version {}, the loaded data's is {}",
            own.as_str(),
            data.as_str()
        ));
    }
    class_of(build, spec, tree)
        .is_none()
        .then(|| "the spec's class is not known".to_owned())
}

/// The spec as the rules see it: its class (`class_of`, else 0), its
/// ascendancy (`ascendancy_of`), its nodes with the starts added.
#[must_use]
pub fn rules_spec(build: &Build, spec: &PassiveSpec, tree: &Tree) -> Spec {
    let class = class_of(build, spec, tree).unwrap_or(0);
    let mut out = Spec {
        class,
        ascendancy: ascendancy_of(build, spec, tree, class),
        nodes: spec.nodes.value().iter().map(|n| n.0).collect(),
        masteries: spec
            .mastery_effects
            .value()
            .iter()
            .map(|m| (m.node.0, m.effect))
            .collect(),
    };
    out.nodes.extend(poe1::starts(tree, &out));
    out
}

/// Writes the rules' spec `next` into spec `id` of `build` (module docs).
///
/// # Errors
/// `APP_TREE` when the build has no spec `id`.
pub fn write_spec(build: &mut Build, id: u32, tree: &Tree, next: &Spec) -> Result<(), AppError> {
    spec_of(build, id)?;
    let active = is_active(build, SpecId(id));
    let spec = build
        .passives
        .specs
        .typed_mut()
        .find(|s| s.id.0 == id)
        .ok_or_else(|| AppError::new(Code::Tree, format!("no passive spec {id}")))?;
    spec.nodes
        .set(next.nodes.iter().map(|n| NodeId(*n)).collect());
    spec.mastery_effects.set(
        next.masteries
            .iter()
            .map(|(node, effect)| MasteryEffect {
                node: NodeId(*node),
                effect: *effect,
            })
            .collect(),
    );
    spec.set_class(next.class);
    spec.set_ascendancy(next.ascendancy);
    let secondary = spec.secondary_ascendancy().unwrap_or(0);
    match poe1::url::encode(tree, next, secondary) {
        Some(url) => {
            for node in spec.layout.opaque_mut() {
                if let OpaqueNode::Element { name, children, .. } = node
                    && name == "URL"
                {
                    *children = vec![OpaqueNode::Text(url_text(children, &url))];
                }
            }
        }
        None => spec
            .layout
            .remove_opaque(|n| matches!(n, OpaqueNode::Element { name, .. } if name == "URL")),
    }
    if active {
        let class = tree.class(next.class);
        if let Some(c) = class {
            build.character.class = Some(ClassId::new(c.name.clone()));
            let asc = c
                .ascendancies
                .iter()
                .find(|a| a.number == next.ascendancy)
                .map_or(NO_ASCENDANCY, |a| a.name.as_str());
            build.character.ascendancy = Some(AscendancyId::new(asc));
        }
    }
    Ok(())
}

/// The URL element's new text: `url` in the old text's surrounding
/// whitespace (PoB indents it).
fn url_text(children: &[OpaqueNode], url: &str) -> String {
    let old: String = children
        .iter()
        .filter_map(|c| match c {
            OpaqueNode::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    let lead = &old[..old.len() - old.trim_start().len()];
    let trail = &old[old.trim_end().len()..];
    format!("{lead}{url}{trail}")
}

/// Whether an entry of the spec list is a `<Spec>` (typed, or kept
/// untyped): spec IDs count both, as the reader numbers them.
fn is_spec(e: &Entry<PassiveSpec>) -> bool {
    match e {
        Entry::Typed(_) => true,
        Entry::Opaque(OpaqueNode::Element { name, .. }) => name == "Spec",
        Entry::Opaque(_) => false,
    }
}

/// How many `<Spec>` elements the build has.
pub(crate) fn spec_count(build: &Build) -> u32 {
    let n = build
        .passives
        .specs
        .entries()
        .iter()
        .filter(|e| is_spec(e))
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn title(text: &str) -> Result<String, AppError> {
    let t = text.trim();
    if t.is_empty() {
        return Err(AppError::new(Code::Tree, "a passive tree needs a name"));
    }
    Ok(t.to_owned())
}

/// Adds a passive spec at the end, as PoB's tree list does, and returns its
/// ID. A copy (`copy`: a spec's ID) takes that spec whole, title aside. A
/// new spec is on the loaded data's tree version (`data`, else the active
/// spec's), with the active spec's class, ascendancy, and bloodline, and
/// only the starts allocated (PoB's "New"). Neither becomes the active
/// spec.
///
/// # Errors
/// `APP_TREE`: an empty name, or no spec `copy`.
pub fn new_spec(
    build: &mut Build,
    tree: &Tree,
    data: Option<&TreeVersion>,
    name: &str,
    copy: Option<u32>,
) -> Result<u32, AppError> {
    let name = title(name)?;
    let id = spec_count(build) + 1;
    let spec = if let Some(from) = copy {
        let mut s = spec_of(build, from)?.clone();
        s.id = SpecId(id);
        s.title = Some(name);
        s
    } else {
        let active = loom_calc::poe1::collect::active_spec(build);
        let rules = active.map(|a| rules_spec(build, a, tree));
        let mut s = PassiveSpec::new(SpecId(id));
        s.title = Some(name);
        s.tree_version = data
            .cloned()
            .or_else(|| active.and_then(|a| a.tree_version.clone()));
        if let Some(bloodline) = active.and_then(PassiveSpec::secondary_ascendancy) {
            s.set_secondary_ascendancy(bloodline);
        }
        let mut next = Spec {
            class: rules.as_ref().map_or(0, |r| r.class),
            ascendancy: rules.as_ref().map_or(0, |r| r.ascendancy),
            nodes: std::collections::BTreeSet::new(),
            masteries: BTreeMap::new(),
        };
        next.nodes.extend(poe1::starts(tree, &next));
        s.nodes.set(next.nodes.iter().map(|n| NodeId(*n)).collect());
        s.set_class(next.class);
        s.set_ascendancy(next.ascendancy);
        s
    };
    build.passives.specs.push(spec);
    Ok(id)
}

/// Renames spec `id`.
///
/// # Errors
/// `APP_TREE`: an empty name, or no spec `id`.
pub fn rename_spec(build: &mut Build, id: u32, name: &str) -> Result<(), AppError> {
    let name = title(name)?;
    let spec = build
        .passives
        .specs
        .typed_mut()
        .find(|s| s.id.0 == id)
        .ok_or_else(|| AppError::new(Code::Tree, format!("no passive spec {id}")))?;
    spec.title = Some(name);
    Ok(())
}

/// Deletes spec `id`, as PoB's tree list does: never the last one; when it
/// was the active spec, the one before it becomes active. The specs after
/// it are renumbered, as the reader numbers them.
///
/// # Errors
/// `APP_TREE`: the last spec, or no spec `id`.
pub fn delete_spec(build: &mut Build, id: u32) -> Result<(), AppError> {
    spec_of(build, id)?;
    if spec_count(build) <= 1 {
        return Err(AppError::new(
            Code::Tree,
            "a build keeps at least one passive tree",
        ));
    }
    let index = build
        .passives
        .specs
        .typed()
        .position(|s| s.id.0 == id)
        .ok_or_else(|| AppError::new(Code::Tree, format!("no passive spec {id}")))?;
    build.passives.specs.remove_typed(index);
    let mut n = 0;
    for e in build.passives.specs.entries_mut() {
        if is_spec(e) {
            n += 1;
            if let Entry::Typed(s) = e {
                s.id = SpecId(n);
            }
        }
    }
    let active = build.passives.active.map_or(1, |a| a.0);
    build.passives.active = Some(SpecId(if active == id {
        id.saturating_sub(1).max(1)
    } else if active > id {
        active - 1
    } else {
        active
    }));
    Ok(())
}

/// The item classes a tree socket takes (passive tree plan, A-a:
/// `SocketJewel`, cluster jewels aside).
const JEWEL_CLASSES: [&str; 2] = ["Jewel", "AbyssJewel"];

/// Sockets jewel `item` (an item's ID; `None` empties it) in socket `node`
/// of spec `id`. The socket must be an allocated jewel socket of the tree,
/// the item a jewel of the build (by its base's item class when `bundle`
/// is known), not a cluster jewel: cluster jewels' sockets and generated
/// nodes come with T3 and T5b. The spec's `<Sockets>` list gets the socket
/// if it lacks it.
///
/// # Errors
/// `APP_TREE`.
pub fn socket_jewel(
    build: &mut Build,
    id: u32,
    tree: &Tree,
    bundle: Option<&Bundle>,
    node: u32,
    item: Option<u32>,
) -> Result<(), AppError> {
    let spec = spec_of(build, id)?;
    let n = tree
        .node(node)
        .filter(|n| n.kind == Kind::JewelSocket && n.modelled)
        .ok_or_else(|| AppError::new(Code::Tree, format!("node {node} is not a jewel socket")))?;
    if !spec.nodes.value().contains(&NodeId(n.id)) {
        return Err(AppError::new(
            Code::Tree,
            format!("socket {node} is not allocated"),
        ));
    }
    let item = item.filter(|i| *i != 0).map(ItemId);
    if let Some(i) = item {
        let found = build
            .items
            .item(i)
            .ok_or_else(|| AppError::new(Code::Tree, format!("no item {}", i.0)))?;
        if let Some(b) = bundle {
            let class = crate::view::item_class(b, found);
            if !class.as_deref().is_some_and(|c| JEWEL_CLASSES.contains(&c)) {
                return Err(AppError::new(
                    Code::Tree,
                    format!("item {} is not a jewel", i.0),
                ));
            }
        }
        let base = found.body.base.as_deref().unwrap_or_default();
        if base.contains("Cluster Jewel") {
            return Err(AppError::new(
                Code::Tree,
                "cluster jewels are not socketed by the tree editor yet",
            ));
        }
    }
    let spec = build
        .passives
        .specs
        .typed_mut()
        .find(|s| s.id.0 == id)
        .ok_or_else(|| AppError::new(Code::Tree, format!("no passive spec {id}")))?;
    if let Some(socket) = spec.jewel_sockets.typed_mut().find(|s| s.node.0 == node) {
        socket.item = item;
    } else {
        spec.jewel_sockets.push(JewelSocket {
            node: NodeId(node),
            item,
            attributes: Attributes::new(),
        });
    }
    if !spec.layout.typed().any(|p| *p == SpecPart::JewelSockets) {
        spec.layout.push(SpecPart::JewelSockets);
    }
    Ok(())
}

/// The spec and edit of a requested tree change.
#[must_use]
pub fn change_edit(change: &api::TreeChange) -> (u32, Edit) {
    match *change {
        api::TreeChange::AllocateNode { spec, node } => (spec, Edit::Allocate(node)),
        api::TreeChange::DeallocateNode { spec, node } => (spec, Edit::Deallocate(node)),
        api::TreeChange::SetMasteryEffect { spec, node, effect } => {
            (spec, Edit::Mastery { node, effect })
        }
    }
}

/// The edits a command makes.
#[derive(Debug, Clone, Copy)]
pub enum Edit {
    /// Allocates a node and its path.
    Allocate(u32),
    /// Deallocates a node and its orphans.
    Deallocate(u32),
    /// Sets a mastery's effect.
    Mastery {
        /// The mastery.
        node: u32,
        /// The effect.
        effect: u32,
    },
    /// Changes the class.
    Class(u32),
    /// Changes the ascendancy.
    Ascendancy(u32),
    /// Clears the tree.
    Reset,
}

/// What an edit does: the next spec, the path allocated, the nodes
/// removed.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The spec afterwards.
    pub spec: Spec,
    /// Nodes allocated, from the allocated ones to the target.
    pub path: Vec<u32>,
    /// Nodes deallocated, ascending.
    pub removed: Vec<u32>,
}

/// Runs `edit` on spec `id` of `build` without writing it; `data` is the
/// loaded data's tree version.
///
/// # Errors
/// `APP_TREE`, also when the spec is `locked`.
pub fn run(
    build: &Build,
    id: u32,
    tree: &Tree,
    data: Option<&TreeVersion>,
    edit: Edit,
) -> Result<Outcome, AppError> {
    let own = spec_of(build, id)?;
    if let Some(why) = locked(build, own, tree, data) {
        return Err(AppError::new(Code::Tree, format!("not edited: {why}")));
    }
    let spec = rules_spec(build, own, tree);
    let removal = |r: poe1::Removal| Outcome {
        spec: r.spec,
        path: Vec::new(),
        removed: r.removed,
    };
    Ok(match edit {
        Edit::Allocate(node) => {
            let a = poe1::allocate(tree, &spec, node).map_err(|e| tree_error(&e))?;
            let removed = spec.nodes.difference(&a.spec.nodes).copied().collect();
            Outcome {
                spec: a.spec,
                path: a.path,
                removed,
            }
        }
        Edit::Deallocate(node) => {
            removal(poe1::deallocate(tree, &spec, node).map_err(|e| tree_error(&e))?)
        }
        Edit::Mastery { node, effect } => {
            let next = poe1::set_mastery(tree, &spec, node, effect).map_err(|e| tree_error(&e))?;
            let path = if spec.nodes.contains(&node) {
                Vec::new()
            } else {
                vec![node]
            };
            Outcome {
                spec: next,
                path,
                removed: Vec::new(),
            }
        }
        Edit::Class(class) => {
            removal(poe1::select_class(tree, &spec, class).map_err(|e| tree_error(&e))?)
        }
        Edit::Ascendancy(a) => {
            removal(poe1::select_ascendancy(tree, &spec, a).map_err(|e| tree_error(&e))?)
        }
        Edit::Reset => {
            let next = poe1::reset(tree, &spec);
            let removed = spec.nodes.difference(&next.nodes).copied().collect();
            Outcome {
                spec: next,
                path: Vec::new(),
                removed,
            }
        }
    })
}

/// The active spec as the editor sees it; `data` is the loaded data's
/// tree version.
#[must_use]
pub fn view(build: &Build, tree: &Tree, data: Option<&TreeVersion>) -> Option<api::TreeView> {
    let spec = loom_calc::poe1::collect::active_spec(build)?;
    let s = rules_spec(build, spec, tree);
    let points = poe1::points(tree, &s);
    Some(api::TreeView {
        spec: spec.id.0,
        class: s.class,
        ascendancy: s.ascendancy,
        nodes: s.nodes.iter().copied().collect(),
        masteries: s
            .masteries
            .iter()
            .map(|(node, effect)| api::MasteryChoice {
                node: *node,
                effect: *effect,
            })
            .collect(),
        kept: s
            .nodes
            .iter()
            .copied()
            .filter(|n| tree.node(*n).is_none_or(|x| !x.modelled))
            .collect(),
        points: points.passive,
        ascendancy_points: points.ascendancy,
        locked: locked(build, spec, tree, data),
        jewels: spec
            .jewel_sockets
            .typed()
            .filter_map(|j| {
                Some(api::TreeJewel {
                    node: j.node.0,
                    item: j.item?.0,
                })
            })
            .collect(),
    })
}

const fn kind(k: Kind) -> api::TreeNodeKind {
    match k {
        Kind::Normal => api::TreeNodeKind::Normal,
        Kind::Notable => api::TreeNodeKind::Notable,
        Kind::Keystone => api::TreeNodeKind::Keystone,
        Kind::Mastery => api::TreeNodeKind::Mastery,
        Kind::JewelSocket => api::TreeNodeKind::JewelSocket,
        Kind::ClusterSocket => api::TreeNodeKind::ClusterSocket,
        Kind::ClassStart => api::TreeNodeKind::ClassStart,
        Kind::AscendancyStart => api::TreeNodeKind::AscendancyStart,
    }
}

/// Stat lines as the game prints them; a stat the descriptions cannot
/// print, other than one the game declares without a description, is
/// shown as its ID and value, never dropped (ADR-015).
fn lines(
    bundle: &Bundle,
    scope: Option<&Scope<'_>>,
    stats: &[loom_data::bundle::passives::StatAmount],
) -> Vec<String> {
    if stats.is_empty() {
        return Vec::new();
    }
    let input: BTreeMap<String, StatValue> = stats
        .iter()
        .map(|s| (s.stat.clone(), StatValue::exact(s.value)))
        .collect();
    let Some(scope) = scope else {
        return stats
            .iter()
            .map(|s| format!("{} {}", s.stat, s.value))
            .collect();
    };
    let rendered = scope.render(&input, bundle, VariantFilter::default());
    let mut out: Vec<String> = rendered.lines.into_iter().map(|l| l.text).collect();
    out.extend(
        rendered
            .not_rendered
            .iter()
            // The game prints neither zeros nor stats it declares
            // without a description; anything else unprinted is shown raw.
            .filter(|n| {
                !matches!(
                    n.reason,
                    NotRenderedReason::Zero | NotRenderedReason::DeclaredNoDescription
                )
            })
            .map(|n| format!("{} {}", n.stat, input[&n.stat].min)),
    );
    out
}

/// The layout DTO of the tree (static while the data is loaded).
#[must_use]
pub fn layout(tree: &Tree, bundle: &Bundle, tree_version: Option<String>) -> api::TreeLayout {
    let scope = bundle
        .scope_root(PASSIVE_DESCRIPTIONS)
        .ok()
        .and_then(|root| bundle.stat_descriptions.scope(root).ok());
    let scope = scope.as_ref();
    let point = |(x, y): (f64, f64)| api::TreePoint { x, y };
    let effects: BTreeMap<u32, &[loom_data::bundle::passives::StatAmount]> = bundle
        .passives
        .as_ref()
        .map(|p| {
            p.mastery_effects
                .iter()
                .filter_map(|m| Some((u32::try_from(m.hash).ok()?, m.stats.as_slice())))
                .collect()
        })
        .unwrap_or_default();
    api::TreeLayout {
        tree_version,
        placed: tree.placed(),
        orbit_radii: tree.orbit_radii.clone(),
        groups: tree.groups.iter().map(|g| point((g.x, g.y))).collect(),
        nodes: tree
            .nodes
            .values()
            .map(|n| api::TreeNodeView {
                id: n.id,
                name: n.name.clone(),
                kind: kind(n.kind),
                group: u32::try_from(n.group).unwrap_or(u32::MAX),
                orbit: n.orbit,
                position: n.position.map(point),
                ascendancy: n.ascendancy.clone(),
                mastery_group: n.mastery_group.clone(),
                modelled: n.modelled,
                cluster_area: n.cluster_area,
                links: n.neighbours.iter().copied().filter(|m| *m > n.id).collect(),
                lines: lines(bundle, scope, &n.stats),
            })
            .collect(),
        classes: tree
            .classes
            .iter()
            .map(|c| api::TreeClassView {
                number: c.number,
                name: c.name.clone(),
                start: c.start,
                ascendancies: c
                    .ascendancies
                    .iter()
                    .map(|a| api::TreeAscendancyView {
                        number: a.number,
                        id: a.id.clone(),
                        name: a.name.clone(),
                        start: a.start,
                    })
                    .collect(),
            })
            .collect(),
        masteries: tree
            .mastery_effects
            .iter()
            .map(|(id, hashes)| api::MasteryGroupView {
                id: id.clone(),
                effects: hashes
                    .iter()
                    .map(|h| api::MasteryEffectView {
                        effect: *h,
                        lines: effects
                            .get(h)
                            .map(|s| lines(bundle, scope, s))
                            .unwrap_or_default(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

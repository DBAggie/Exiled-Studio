//! Path of Exile 1's allocation rules (passive tree plan, T-f, T-g) and its
//! tree URL ([`url`]).
//!
//! A [`Spec`] is the editor's view of one passive spec: the class, the
//! ascendancy, the allocated nodes (class and ascendancy starts included,
//! as PoB lists them), and the chosen mastery effects. Each edit takes a
//! spec and returns the next one, or an [`EditError`] and no change.
//!
//! Rules, as PoB's `PassiveSpec.lua` where they are deterministic:
//! - a node is reached from the allocated nodes connected to the class
//!   start or the ascendancy start; class and ascendancy starts, masteries,
//!   and nodes the editor does not model are never on a path; ascendancy
//!   nodes connect only within their ascendancy, and only the spec's
//!   ascendancy's nodes are reachable;
//! - the path allocated is a shortest one; of several, the one whose node
//!   list, from the allocated nodes to the target, is lexicographically
//!   smallest by node number (loom's tie rule: PoB's is Lua hash order);
//! - a mastery is allocated with an effect, when a node linked to it (a
//!   notable of its group) is connected; an effect is taken at most once
//!   per mastery group;
//! - an allocated ascendancy node reaches the main tree nodes it links to
//!   (the Ascendant's "Path of the ..." nodes, as PoB);
//! - deallocating removes the node and every node the edit disconnects; a
//!   node not connected before the edit (allocated through a radius jewel,
//!   as imported) stays; a mastery is connected while a node linked to it
//!   is;
//! - nodes the editor does not model, and nodes the tree does not have,
//!   count as connected and are kept by every edit except [`reset`] (T-g);
//! - a node in the radius of a socketed Intuitive Leap-like jewel
//!   ([`radius::Leap`], the spec's `leaps`) is allocated alone, without a
//!   path, and counts as connected while the jewel's socket does; no path
//!   starts from it (PoB's `intuitiveLeapLikesAffecting`).

pub mod cluster;
pub mod radius;
pub mod url;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::graph::{Kind, Tree};

/// One passive spec, as the rules see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spec {
    /// The class's number (PoB's `classId`).
    pub class: u32,
    /// The ascendancy's number within the class, 0 for none (PoB's
    /// `ascendClassId`).
    pub ascendancy: u32,
    /// The allocated nodes, starts included.
    pub nodes: BTreeSet<u32>,
    /// The chosen effect of each allocated mastery.
    pub masteries: BTreeMap<u32, u32>,
    /// The socketed jewels that let nodes in their radius be allocated
    /// without a path.
    pub leaps: Vec<radius::Leap>,
}

/// Why an edit was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// The tree has no such node.
    #[error("the tree has no node {0}")]
    UnknownNode(u32),
    /// The tree has no such class.
    #[error("the tree has no class {0}")]
    UnknownClass(u32),
    /// The class has no such ascendancy.
    #[error("the class has no ascendancy {0}")]
    UnknownAscendancy(u32),
    /// The node is not modelled by the editor (it is kept, not edited).
    #[error("node {0} is not one the editor allocates")]
    NotModelled(u32),
    /// Class and ascendancy starts are set by choosing the class or
    /// ascendancy.
    #[error("node {0} is a start; choose the class or ascendancy instead")]
    Start(u32),
    /// The node is already allocated.
    #[error("node {0} is already allocated")]
    Allocated(u32),
    /// The node is not allocated.
    #[error("node {0} is not allocated")]
    NotAllocated(u32),
    /// No path reaches the node.
    #[error("no path reaches node {0}")]
    Unreachable(u32),
    /// A mastery is allocated by choosing its effect.
    #[error("node {0} is a mastery; choose its effect")]
    Mastery(u32),
    /// The node is not a mastery.
    #[error("node {0} is not a mastery")]
    NotMastery(u32),
    /// The effect is not one of the mastery's.
    #[error("effect {effect} is not one of mastery {node}'s")]
    UnknownEffect {
        /// The mastery.
        node: u32,
        /// The effect.
        effect: u32,
    },
    /// Another mastery of the group has the effect.
    #[error("effect {effect} is taken by mastery {by}")]
    EffectTaken {
        /// The effect.
        effect: u32,
        /// The mastery that has it.
        by: u32,
    },
}

/// Points spent, as PoB's `CountAllocNodes`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Points {
    /// Passive points: nodes outside ascendancies, starts excepted.
    pub passive: u32,
    /// Ascendancy points: ascendancy nodes, starts and multiple choice
    /// options excepted.
    pub ascendancy: u32,
}

/// What an allocation would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocation {
    /// The nodes allocated, from the allocated set to the target.
    pub path: Vec<u32>,
    /// The spec afterwards.
    pub spec: Spec,
}

/// What a deallocation or a class change would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removal {
    /// The nodes deallocated, ascending.
    pub removed: Vec<u32>,
    /// The spec afterwards.
    pub spec: Spec,
}

/// The spec's ascendancy's ID, if it has one.
fn ascendancy_id<'t>(tree: &'t Tree, spec: &Spec) -> Option<&'t str> {
    tree.class(spec.class)?
        .ascendancies
        .iter()
        .find(|a| a.number == spec.ascendancy)
        .map(|a| a.id.as_str())
}

/// The spec's starts: the class's and the ascendancy's.
#[must_use]
pub fn starts(tree: &Tree, spec: &Spec) -> Vec<u32> {
    let Some(class) = tree.class(spec.class) else {
        return Vec::new();
    };
    let mut out = vec![class.start];
    if let Some(s) = class
        .ascendancies
        .iter()
        .find(|a| a.number == spec.ascendancy)
        .and_then(|a| a.start)
    {
        out.push(s);
    }
    out
}

/// Whether a node is kept as it is: one the tree lacks or the editor does
/// not model (T-g).
fn kept(tree: &Tree, id: u32) -> bool {
    tree.node(id).is_none_or(|n| !n.modelled)
}

/// Whether a path may enter `id` (unallocated): a modelled node that is not
/// a start or a mastery, outside ascendancies or in the spec's.
fn enterable(tree: &Tree, id: u32, ascendancy: Option<&str>) -> bool {
    tree.node(id).is_some_and(|n| {
        n.modelled
            && !matches!(
                n.kind,
                Kind::Mastery | Kind::ClassStart | Kind::AscendancyStart
            )
            && n.ascendancy
                .as_deref()
                .is_none_or(|a| Some(a) == ascendancy)
    })
}

/// Whether two nodes link for the rules: both in the tree, in the same
/// ascendancy or both outside one.
fn links(tree: &Tree, a: u32, b: u32) -> bool {
    match (tree.node(a), tree.node(b)) {
        (Some(x), Some(y)) => x.ascendancy == y.ascendancy,
        _ => false,
    }
}

/// Whether an allocated node `a` reaches its neighbour `b`: a link, or a
/// step from an ascendancy node out to the main tree (the Ascendant's
/// "Path of the ..." nodes; PoB's `BuildPathFromNode`, `curDist == 0 and
/// not other.ascendancyName`).
fn reaches(tree: &Tree, a: u32, b: u32) -> bool {
    links(tree, a, b)
        || match (tree.node(a), tree.node(b)) {
            (Some(x), Some(y)) => x.ascendancy.is_some() && y.ascendancy.is_none(),
            _ => false,
        }
}

/// `spec`, allocated on another tree version, converted to `tree` (plan
/// T5b, PoB's `ConvertToVersion`): its nodes `tree` has, with the class's
/// and ascendancy's starts, and only those still [`connected`]; its
/// ascendancy none when `tree`'s class lacks it. A mastery stays only
/// with a chosen effect `tree` gives it, as PoB's import allocates no
/// mastery without a selection (`ImportFromNodeList`) and drops one whose
/// effect its tree does not know (`BuildAllDependsAndPaths`).
#[must_use]
pub fn convert(tree: &Tree, spec: &Spec) -> Spec {
    let ascendancy = if tree
        .class(spec.class)
        .is_some_and(|c| c.ascendancies.iter().any(|a| a.number == spec.ascendancy))
    {
        spec.ascendancy
    } else {
        0
    };
    let mut next = Spec {
        class: spec.class,
        ascendancy,
        nodes: spec
            .nodes
            .iter()
            .copied()
            .filter(|n| tree.node(*n).is_some())
            .collect(),
        masteries: BTreeMap::new(),
        leaps: spec.leaps.clone(),
    };
    next.nodes.extend(starts(tree, &next));
    next.nodes = connected(tree, &next);
    next.masteries = spec
        .masteries
        .iter()
        .filter(|(node, effect)| {
            next.nodes.contains(node)
                && tree
                    .node(**node)
                    .and_then(|n| n.mastery_group.as_ref())
                    .and_then(|g| tree.mastery_effects.get(g))
                    .is_some_and(|effects| effects.contains(effect))
        })
        .map(|(n, e)| (*n, *e))
        .collect();
    let masteries = &next.masteries;
    next.nodes.retain(|n| {
        tree.node(*n)
            .is_none_or(|node| node.kind != Kind::Mastery || masteries.contains_key(n))
    });
    next
}

/// The allocated nodes connected to the spec's starts, with every kept
/// node (T-g), masteries of connected groups included.
#[must_use]
pub fn connected(tree: &Tree, spec: &Spec) -> BTreeSet<u32> {
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    for s in starts(tree, spec) {
        if spec.nodes.contains(&s) && seen.insert(s) {
            queue.push_back(s);
        }
    }
    for &n in &spec.nodes {
        if kept(tree, n) && seen.insert(n) {
            queue.push_back(n);
        }
    }
    while let Some(a) = queue.pop_front() {
        let Some(node) = tree.node(a) else { continue };
        for &b in &node.neighbours {
            if !spec.nodes.contains(&b) || seen.contains(&b) || !reaches(tree, a, b) {
                continue;
            }
            let Some(next) = tree.node(b) else { continue };
            if matches!(
                next.kind,
                Kind::Mastery | Kind::ClassStart | Kind::AscendancyStart
            ) {
                continue;
            }
            seen.insert(b);
            queue.push_back(b);
        }
    }
    // Nodes a radius jewel holds; no path goes on from them.
    let held = radius::held(tree, &spec.leaps, &spec.nodes, &seen);
    seen.extend(held);
    // A mastery hangs from the nodes linked to it (its group's notables).
    let masteries: Vec<u32> = spec
        .nodes
        .iter()
        .copied()
        .filter(|n| {
            tree.node(*n).is_some_and(|m| {
                m.kind == Kind::Mastery && m.neighbours.iter().any(|o| seen.contains(o))
            })
        })
        .collect();
    seen.extend(masteries);
    seen
}

/// The allocated nodes that count in a calculation: those connected to an
/// allocated root (any class or ascendancy start) through allocated nodes,
/// over every link of the tree (PoB's load-time rule,
/// `Classes/PassiveSpec.lua` `BuildAllDependsAndPaths`, `connectedToStart`;
/// PoB drops the others when it loads a build). Unlike [`connected`] (the
/// editor's), kept nodes do not count unless connected, and a node the
/// tree lacks links nothing; a node PoB numbers as a cluster jewel's
/// counts only on a tree its jewel's subgraph is on ([`cluster::expand`]).
/// The calculation's resolved passive set (`loom-pipeline`) reads this.
#[must_use]
pub fn connected_to_start<'a>(
    tree: &Tree,
    nodes: impl IntoIterator<Item = &'a u32>,
) -> BTreeSet<u32> {
    let allocated: BTreeSet<u32> = nodes
        .into_iter()
        .copied()
        .filter(|n| n & CLUSTER_NODE_BIT == 0 || tree.node(*n).is_some())
        .collect();
    let mut seen: BTreeSet<u32> = allocated
        .iter()
        .copied()
        .filter(|n| {
            tree.node(*n)
                .is_some_and(|x| matches!(x.kind, Kind::ClassStart | Kind::AscendancyStart))
        })
        .collect();
    let mut queue: Vec<u32> = seen.iter().copied().collect();
    while let Some(a) = queue.pop() {
        let Some(node) = tree.node(a) else { continue };
        for &b in &node.neighbours {
            if allocated.contains(&b) && seen.insert(b) {
                queue.push(b);
            }
        }
    }
    seen
}

/// The bit PoB sets on the nodes it generates from a socketed cluster
/// jewel (`Classes/PassiveSpec.lua` BuildSubgraph: "16: 1 (signal bit, to
/// prevent conflict with node hashes)").
pub const CLUSTER_NODE_BIT: u32 = 0x1_0000;

/// Removes every allocated node the edit from `before` left unconnected,
/// and the effects of removed masteries; returns what was removed. A node
/// that was not connected before (one allocated through a radius jewel, as
/// imported) stays: an edit removes only what it disconnects (T-g).
fn prune(tree: &Tree, before: &Spec, spec: &mut Spec) -> Vec<u32> {
    let was = connected(tree, before);
    let keep = connected(tree, spec);
    let removed: Vec<u32> = spec
        .nodes
        .iter()
        .copied()
        .filter(|n| was.contains(n) && !keep.contains(n))
        .collect();
    for n in &removed {
        spec.nodes.remove(n);
        spec.masteries.remove(n);
    }
    removed
}

/// The distances to `target` over unallocated enterable nodes, and the
/// first steps: each enterable node next to a connected one, with its
/// distance.
type Steps = (BTreeMap<u32, u32>, BTreeSet<(u32, u32)>);

/// Refuses a target that cannot be allocated whatever the path: unknown,
/// kept, a start, a mastery, allocated, or of another ascendancy.
fn check(tree: &Tree, spec: &Spec, target: u32) -> Result<(), EditError> {
    let node = tree.node(target).ok_or(EditError::UnknownNode(target))?;
    if !node.modelled {
        return Err(EditError::NotModelled(target));
    }
    match node.kind {
        Kind::ClassStart | Kind::AscendancyStart => return Err(EditError::Start(target)),
        Kind::Mastery => return Err(EditError::Mastery(target)),
        _ => {}
    }
    if spec.nodes.contains(&target) {
        return Err(EditError::Allocated(target));
    }
    if !enterable(tree, target, ascendancy_id(tree, spec)) {
        return Err(EditError::Unreachable(target));
    }
    Ok(())
}

/// Whether a radius jewel of the spec, in a connected socket, lets
/// `target` be allocated without a path.
fn leaped(tree: &Tree, spec: &Spec, target: u32) -> bool {
    if spec.leaps.is_empty() {
        return false;
    }
    let from = connected(tree, spec);
    spec.leaps
        .iter()
        .any(|l| from.contains(&l.socket) && radius::leaps_to(tree, l, target))
}

fn steps(tree: &Tree, spec: &Spec, target: u32) -> Result<Steps, EditError> {
    let node = tree.node(target).ok_or(EditError::UnknownNode(target))?;
    if !node.modelled {
        return Err(EditError::NotModelled(target));
    }
    match node.kind {
        Kind::ClassStart | Kind::AscendancyStart => return Err(EditError::Start(target)),
        Kind::Mastery => return Err(EditError::Mastery(target)),
        _ => {}
    }
    if spec.nodes.contains(&target) {
        return Err(EditError::Allocated(target));
    }
    let asc = ascendancy_id(tree, spec);
    if !enterable(tree, target, asc) {
        return Err(EditError::Unreachable(target));
    }
    let mut to_target: BTreeMap<u32, u32> = BTreeMap::new();
    to_target.insert(target, 0);
    let mut queue = VecDeque::from([target]);
    while let Some(a) = queue.pop_front() {
        let d = to_target[&a];
        for &b in &tree.nodes[&a].neighbours {
            if !to_target.contains_key(&b)
                && !spec.nodes.contains(&b)
                && enterable(tree, b, asc)
                && links(tree, a, b)
            {
                to_target.insert(b, d + 1);
                queue.push_back(b);
            }
        }
    }
    // Paths start from the nodes connected without the radius jewels.
    let mut plain = spec.clone();
    plain.leaps.clear();
    let from = connected(tree, &plain);
    let firsts: BTreeSet<(u32, u32)> = from
        .iter()
        .filter_map(|a| tree.node(*a))
        .filter(|a| a.kind != Kind::Mastery)
        .flat_map(|a| {
            a.neighbours
                .iter()
                .filter(|b| reaches(tree, a.id, **b))
                .filter_map(|b| to_target.get(b).map(|d| (*d, *b)))
        })
        .collect();
    if firsts.is_empty() {
        return Err(EditError::Unreachable(target));
    }
    Ok((to_target, firsts))
}

/// The nodes one step nearer `target` from `at` (at distance `d`).
fn nearer<'t>(
    tree: &'t Tree,
    to_target: &'t BTreeMap<u32, u32>,
    at: u32,
    d: u32,
) -> impl Iterator<Item = u32> + 't {
    tree.nodes[&at]
        .neighbours
        .iter()
        .filter(move |b| links(tree, at, **b) && to_target.get(*b).is_some_and(|e| *e + 1 == d))
        .copied()
}

/// The path that allocating `target` would allocate: a shortest path from
/// the connected nodes, the lexicographically smallest of equals (module
/// docs).
///
/// # Errors
/// When the node cannot be allocated: unknown, kept, a start, a mastery,
/// allocated, or out of reach.
pub fn path(tree: &Tree, spec: &Spec, target: u32) -> Result<Vec<u32>, EditError> {
    check(tree, spec, target)?;
    if leaped(tree, spec, target) {
        return Ok(vec![target]);
    }
    let (to_target, firsts) = steps(tree, spec, target)?;
    let first = *firsts.first().ok_or(EditError::Unreachable(target))?;
    let mut out = vec![first.1];
    let mut at = first;
    while at.0 > 0 {
        let next = nearer(tree, &to_target, at.1, at.0)
            .min()
            .ok_or(EditError::Unreachable(target))?;
        out.push(next);
        at = (at.0 - 1, next);
    }
    Ok(out)
}

/// How many shortest paths [`path`] chooses among, counted up to `cap`:
/// 1 when the shortest path is unique (the plan's T-f: only then is the
/// node list compared with PoB's).
///
/// # Errors
/// As [`path`].
pub fn shortest_path_count(
    tree: &Tree,
    spec: &Spec,
    target: u32,
    cap: usize,
) -> Result<usize, EditError> {
    check(tree, spec, target)?;
    if leaped(tree, spec, target) {
        return Ok(1);
    }
    let (to_target, firsts) = steps(tree, spec, target)?;
    let best = firsts.first().map_or(0, |f| f.0);
    // Paths from each node to the target, by increasing distance.
    let mut count: BTreeMap<u32, usize> = BTreeMap::new();
    let mut by_distance: Vec<(u32, u32)> = to_target.iter().map(|(n, d)| (*d, *n)).collect();
    by_distance.sort_unstable();
    for (d, n) in by_distance {
        let c = if d == 0 {
            1
        } else {
            nearer(tree, &to_target, n, d)
                .map(|m| count.get(&m).copied().unwrap_or(0))
                .fold(0, |a: usize, b| a.saturating_add(b).min(cap))
        };
        count.insert(n, c);
    }
    Ok(firsts
        .iter()
        .filter(|(d, _)| *d == best)
        .map(|(_, b)| count.get(b).copied().unwrap_or(0))
        .fold(0, |a: usize, b| a.saturating_add(b).min(cap)))
}

/// Allocates `target` and the path to it.
///
/// # Errors
/// As [`path`].
pub fn allocate(tree: &Tree, spec: &Spec, target: u32) -> Result<Allocation, EditError> {
    let path = path(tree, spec, target)?;
    let mut next = spec.clone();
    next.nodes.extend(path.iter().copied());
    // One option of a multiple choice at a time.
    if tree.node(target).is_some_and(|n| n.choice_option) {
        let parents = &tree.nodes[&target].neighbours;
        let others: Vec<u32> = next
            .nodes
            .iter()
            .copied()
            .filter(|o| {
                *o != target
                    && tree.node(*o).is_some_and(|n| {
                        n.choice_option && n.neighbours.iter().any(|p| parents.contains(p))
                    })
            })
            .collect();
        let before = next.clone();
        for o in others {
            next.nodes.remove(&o);
        }
        prune(tree, &before, &mut next);
    }
    Ok(Allocation { path, spec: next })
}

/// Deallocates `node` and every node it leaves unconnected.
///
/// # Errors
/// When the node is unknown, kept, a start, or not allocated.
pub fn deallocate(tree: &Tree, spec: &Spec, node: u32) -> Result<Removal, EditError> {
    let n = tree.node(node).ok_or(EditError::UnknownNode(node))?;
    if !n.modelled {
        return Err(EditError::NotModelled(node));
    }
    if matches!(n.kind, Kind::ClassStart | Kind::AscendancyStart) {
        return Err(EditError::Start(node));
    }
    if !spec.nodes.contains(&node) {
        return Err(EditError::NotAllocated(node));
    }
    let mut next = spec.clone();
    next.nodes.remove(&node);
    next.masteries.remove(&node);
    let mut removed = vec![node];
    removed.extend(prune(tree, spec, &mut next));
    removed.sort_unstable();
    Ok(Removal {
        removed,
        spec: next,
    })
}

/// Sets a mastery's effect, allocating the mastery when it is not.
///
/// # Errors
/// When the node is not a modelled mastery, no node linked to it is
/// connected, the effect is not the mastery's, or another mastery of the group
/// has it.
pub fn set_mastery(tree: &Tree, spec: &Spec, node: u32, effect: u32) -> Result<Spec, EditError> {
    let n = tree.node(node).ok_or(EditError::UnknownNode(node))?;
    if n.kind != Kind::Mastery {
        return Err(EditError::NotMastery(node));
    }
    if !n.modelled {
        return Err(EditError::NotModelled(node));
    }
    let group = n.mastery_group.as_deref().unwrap_or_default();
    if !tree
        .mastery_effects
        .get(group)
        .is_some_and(|e| e.contains(&effect))
    {
        return Err(EditError::UnknownEffect { node, effect });
    }
    if let Some((by, _)) = spec.masteries.iter().find(|(m, e)| {
        **m != node
            && **e == effect
            && tree
                .node(**m)
                .is_some_and(|x| x.mastery_group.as_deref() == Some(group))
    }) {
        return Err(EditError::EffectTaken { effect, by: *by });
    }
    let mut next = spec.clone();
    next.nodes.insert(node);
    next.masteries.insert(node, effect);
    if !connected(tree, &next).contains(&node) {
        return Err(EditError::Unreachable(node));
    }
    Ok(next)
}

/// Changes the class, as PoB's `SelectClass`: the old starts go, the new
/// class's start comes, no ascendancy, and nodes no longer connected go.
///
/// # Errors
/// When the tree has no such class.
pub fn select_class(tree: &Tree, spec: &Spec, class: u32) -> Result<Removal, EditError> {
    tree.class(class).ok_or(EditError::UnknownClass(class))?;
    let mut next = spec.clone();
    for s in starts(tree, spec) {
        next.nodes.remove(&s);
    }
    next.class = class;
    next.ascendancy = 0;
    next.nodes.extend(starts(tree, &next));
    let removed = prune(tree, spec, &mut next);
    Ok(Removal {
        removed,
        spec: next,
    })
}

/// Changes the ascendancy (0 for none), as PoB's `SelectAscendClass`: the
/// old start and its nodes go, the new start comes.
///
/// # Errors
/// When the class has no such ascendancy.
pub fn select_ascendancy(tree: &Tree, spec: &Spec, ascendancy: u32) -> Result<Removal, EditError> {
    let class = tree
        .class(spec.class)
        .ok_or(EditError::UnknownClass(spec.class))?;
    if ascendancy != 0 && !class.ascendancies.iter().any(|a| a.number == ascendancy) {
        return Err(EditError::UnknownAscendancy(ascendancy));
    }
    let mut next = spec.clone();
    for s in starts(tree, spec).into_iter().skip(1) {
        next.nodes.remove(&s);
    }
    next.ascendancy = ascendancy;
    next.nodes.extend(starts(tree, &next));
    let removed = prune(tree, spec, &mut next);
    Ok(Removal {
        removed,
        spec: next,
    })
}

/// Clears the tree, as PoB's `ResetNodes`: the starts stay, everything
/// else goes, kept nodes too.
#[must_use]
pub fn reset(tree: &Tree, spec: &Spec) -> Spec {
    Spec {
        class: spec.class,
        ascendancy: spec.ascendancy,
        nodes: starts(tree, spec).into_iter().collect(),
        masteries: BTreeMap::new(),
        leaps: spec.leaps.clone(),
    }
}

/// The points spent, as PoB's `CountAllocNodes`. A node the tree lacks
/// counts as a passive point.
#[must_use]
pub fn points(tree: &Tree, spec: &Spec) -> Points {
    let mut out = Points::default();
    for &id in &spec.nodes {
        match tree.node(id) {
            Some(n) if matches!(n.kind, Kind::ClassStart | Kind::AscendancyStart) => {}
            Some(n) if n.ascendancy.is_some() => {
                if !n.choice_option {
                    out.ascendancy += 1;
                }
            }
            _ => out.passive += 1,
        }
    }
    out
}

/// The level a character is expected to reach each act at, and the
/// passive points its quests have given by then (PoB v2.67.2,
/// `Modules/Build.lua` 711 to 746, `acts`).
const QUEST_POINTS: [(u32, u32); 11] = [
    (1, 0),
    (12, 2),
    (22, 4),
    (32, 6),
    (40, 7),
    (44, 9),
    (50, 12),
    (54, 15),
    (60, 18),
    (64, 20),
    (67, 23),
];

/// The passive points a character of `level` has: one a level after the
/// first, its quests' by the act PoB expects that level in
/// ([`QUEST_POINTS`]), and from the third act on one more when it helped
/// no bandit (`bandits_killed`; PoB's `actExtra`). An estimate below level
/// 67, where a character may be ahead of or behind its quests; at 67 and
/// above it is the game's count for a character that did every quest.
#[must_use]
pub fn points_at(level: u32, bandits_killed: bool) -> u32 {
    let level = level.clamp(1, 100);
    let act = QUEST_POINTS
        .iter()
        .rposition(|(from, _)| *from <= level)
        .unwrap_or(0);
    let bandit = u32::from(bandits_killed && act >= 2);
    level - 1 + QUEST_POINTS[act].1 + bandit
}

#[cfg(test)]
mod tests;

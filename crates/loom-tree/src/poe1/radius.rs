//! Radius jewels that let nodes be allocated without a path (passive tree
//! plan, T-f): Intuitive Leap and Thread of Hope around their socket,
//! Impossible Escape around a keystone (PoB's `intuitiveLeapLike` and
//! `impossibleEscapeKeystone`, `Classes/PassiveSpec.lua`).
//!
//! A node is in a jewel's radius when its distance from the centre is
//! from the ring's inner to its outer radius, both included, compared as
//! squares on the layout's positions (PoB's `nodesInRadius`,
//! `Classes/PassiveTree.lua` 645 to 657: `distSquared <= outerSquared and
//! innerSquared <= distSquared`). The centre itself, masteries, nodes of
//! the cluster areas, and nodes without a position are never in a radius;
//! around a keystone, jewel sockets are not either (PoB 672 to 676). A
//! tree without the layout has no node in any radius.
//!
//! The radii are the game's (`PassiveJewelRadii`, the bundle's
//! `poe1/jewel_radii.json`): a jewel's `Radius:` line names a row, and a
//! ring jewel's ring value `k` is the ring of the table's `k`-th row (the
//! data-parity table `jewel_radii` checks both against PoB's
//! `data.jewelRadii`).

use std::collections::BTreeSet;

use crate::graph::{Kind, Tree};

/// The distances a jewel reaches, in tree units, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ring {
    /// Inner radius (0 for a full radius).
    pub inner: i64,
    /// Outer radius.
    pub outer: i64,
}

/// Where a jewel's radius is centred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Centre {
    /// Its socket (Intuitive Leap, Thread of Hope).
    Socket,
    /// A keystone, by node (Impossible Escape).
    Keystone(u32),
}

/// A socketed jewel that lets the nodes in its radius be allocated without
/// being connected to the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leap {
    /// The socket it is in.
    pub socket: u32,
    /// Its radius.
    pub ring: Ring,
    /// Where the radius is centred.
    pub centre: Centre,
    /// Only keystones in the radius (PoB's `intuitiveLeapKeystoneOnly`).
    pub keystones_only: bool,
}

impl Leap {
    /// The nodes in its radius.
    #[must_use]
    pub fn nodes(&self, tree: &Tree) -> BTreeSet<u32> {
        match self.centre {
            Centre::Socket => in_ring(tree, self.socket, self.ring, true),
            Centre::Keystone(k) => in_ring(tree, k, self.ring, false),
        }
    }
}

/// The nodes at a distance in `ring` from `centre` (module docs);
/// `sockets`: whether jewel sockets count.
#[must_use]
pub fn in_ring(tree: &Tree, centre: u32, ring: Ring, sockets: bool) -> BTreeSet<u32> {
    let Some((cx, cy)) = tree.node(centre).and_then(|c| c.position) else {
        return BTreeSet::new();
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "radii are a few thousand tree units, exact in f64"
    )]
    let (inner, outer) = (ring.inner as f64, ring.outer as f64);
    tree.nodes
        .values()
        .filter(|n| {
            n.id != centre
                && n.kind != Kind::Mastery
                && !n.cluster_area
                && (sockets || !matches!(n.kind, Kind::JewelSocket | Kind::ClusterSocket))
        })
        .filter_map(|n| {
            let (x, y) = n.position?;
            let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            (d <= outer * outer && inner * inner <= d).then_some(n.id)
        })
        .collect()
}

/// Whether a leap lets `node` be allocated without a path: PoB's
/// `intuitiveLeapLikesAffecting` (`PassiveSpec.lua` 1145 to 1175): not a
/// start, a socket, or an ascendancy node; a keystone when the jewel takes
/// only keystones.
#[must_use]
pub fn leaps_to(tree: &Tree, leap: &Leap, node: u32) -> bool {
    tree.node(node).is_some_and(|n| {
        !matches!(
            n.kind,
            Kind::ClassStart | Kind::AscendancyStart | Kind::JewelSocket | Kind::ClusterSocket
        ) && n.ascendancy.is_none()
            && (!leap.keystones_only || n.kind == Kind::Keystone)
    }) && leap.nodes(tree).contains(&node)
}

/// The allocated nodes `leaps` hold beyond `counted`: each allocated node
/// in the radius of a leap whose socket counts, added until nothing
/// changes (a held socket's leap holds too). PoB keeps them when it loads a
/// build instead of pruning them (`PassiveSpec.lua` 1515 to 1545: the
/// radius alone, without the node kinds of [`leaps_to`]).
#[must_use]
pub fn held(
    tree: &Tree,
    leaps: &[Leap],
    allocated: &BTreeSet<u32>,
    counted: &BTreeSet<u32>,
) -> BTreeSet<u32> {
    let radii: Vec<(u32, BTreeSet<u32>)> =
        leaps.iter().map(|l| (l.socket, l.nodes(tree))).collect();
    let mut all = counted.clone();
    let mut out = BTreeSet::new();
    loop {
        let more: Vec<u32> = radii
            .iter()
            .filter(|(socket, _)| all.contains(socket))
            .flat_map(|(_, nodes)| nodes.iter().copied())
            .filter(|n| allocated.contains(n) && !all.contains(n))
            .collect();
        if more.is_empty() {
            return out;
        }
        all.extend(more.iter().copied());
        out.extend(more);
    }
}

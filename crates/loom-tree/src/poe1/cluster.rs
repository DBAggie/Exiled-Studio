//! Cluster jewels (passive tree plan, T-i): the nodes a cluster jewel adds
//! when socketed, as PoB's `Classes/PassiveSpec.lua` builds them
//! (`BuildClusterJewelGraphs` 1723, `BuildSubgraph` 1885, PoB v2.67.2), so
//! the node numbers equal PoB's (what builds list) and the game's.
//!
//! A [`Jewel`] is what a cluster jewel's lines say (the host reads them
//! from the item's stat IDs, ADR-015). [`subgraphs`] builds, for each large
//! socket on the tree holding a jewel that adds passives, its
//! [`Subgraph`], and recursively the subgraphs of the jewels socketed in
//! the sockets it adds. [`expand`] puts them on a copy of the tree, which
//! the allocation rules and the calculation's resolution then walk as any
//! other part of the tree.
//!
//! The rules, as PoB's:
//! - **Node numbers.** Bit 16 set (PoB's signal bit, [`super::CLUSTER_NODE_BIT`]),
//!   bits 6 to 8 the large socket's cluster index, bits 9 and 10 a nested
//!   medium socket's, bits 4 and 5 the jewel's size, bits 0 to 3 the node's
//!   place on the jewel's ring; a socket the jewel adds keeps the tree's
//!   own number of the nested socket it stands for; the jewel's mastery is
//!   the ring's 12.
//! - **Places.** Sockets first (a large jewel with one socket puts it at
//!   place 6, the socket with cluster index 1; else the base's socket
//!   places, the sockets with cluster indices 0, 2, 1 in that order), then
//!   notables in the base's notable places (sorted, the notables in their
//!   stats' order), then small passives in its small places; a medium
//!   jewel moves some places (two notables and no socket; four nodes).
//! - **The ring.** Each node's place is turned by the socket's start index
//!   for the jewel's size and translated from the base's ring to the
//!   orbit's slots; the nodes link around the ring in place order, closed
//!   for a medium or large jewel; the first place links to the socket.
//! - **Small passives** take the granted passive's name and stats, plus
//!   the jewel's "also grant" stats, with its "increased effect"
//!   ([`ClusterNode::effect`]); no granted passive (a jewel whose small
//!   passives grant nothing) gives none.
//! - A jewel adding a keystone adds only that node, at place 1 of the
//!   centre.
//! - What PoB refuses with an error (no socket for a place, more sockets
//!   than places, no first place, a notable the data lacks) builds
//!   nothing here: the host gives the jewel's nodes a reason.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::passives::StatAmount;
use loom_data::bundle::poe1::{ClusterJewelBase, ClusterJewels};

use super::CLUSTER_NODE_BIT;
use crate::graph::{Kind, Node, Tree};

/// What a socketed cluster jewel's lines say (module docs; PoB's
/// `jewelData.clusterJewel*`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Jewel {
    /// Its base item's ID.
    pub base: String,
    /// The small passive it grants: its row in
    /// [`ClusterJewels::skills`] (`local_jewel_expansion_passive_node_index`).
    pub skill: Option<usize>,
    /// "Adds # Passive Skills" (`local_jewel_expansion_passive_node_count`).
    pub node_count: Option<u32>,
    /// "# Added Passive Skills are Jewel Sockets"
    /// (`local_jewel_expansion_jewels_count`).
    pub socket_count: Option<u32>,
    /// "Adds # Jewel Socket Passive Skills"
    /// (`local_jewel_expansion_jewels_count_override`).
    pub socket_override: Option<u32>,
    /// "Adds # Small Passive Skills which grant nothing"
    /// (`local_unique_jewel_grants_x_empty_passives`).
    pub nothingness: Option<u32>,
    /// "Added Small Passive Skills grant Nothing"
    /// (`local_affliction_jewel_display_small_nodes_grant_nothing`).
    pub smalls_nothing: bool,
    /// The notables it adds, by node number.
    pub notables: Vec<u32>,
    /// The keystone it adds, by node number.
    pub keystone: Option<u32>,
    /// "Added Small Passive Skills also grant: ..." as stats.
    pub added: Vec<StatAmount>,
    /// "Added Small Passive Skills have #% increased Effect"
    /// (`local_affliction_jewel_small_nodes_have_effect_+%`).
    pub effect: i64,
}

impl Jewel {
    /// The small passive it grants, if it is one of its base's size (PoB
    /// drops a skill its base's size does not have).
    fn skill<'d>(
        &self,
        data: &'d ClusterJewels,
        base: &ClusterJewelBase,
    ) -> Option<&'d loom_data::bundle::poe1::ClusterSkill> {
        self.skill
            .and_then(|i| data.skills.get(i))
            .filter(|s| s.size == base.size)
    }

    /// Whether it adds passives (PoB's `clusterJewelValid`): a keystone,
    /// or a node count with a skill or nothing-granting small passives, or
    /// a socket count with nothing-granting passives.
    #[must_use]
    pub fn valid(&self, data: &ClusterJewels) -> bool {
        let Some(base) = data.jewel(&self.base) else {
            return false;
        };
        self.keystone.is_some()
            || ((self.skill(data, base).is_some() || self.smalls_nothing)
                && self.node_count.is_some())
            || (self.socket_override.is_some() && self.nothingness.is_some())
    }
}

/// A node a cluster jewel adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterNode {
    /// Its number (module docs).
    pub id: u32,
    /// Its kind: a small passive, notable, keystone, mastery, or socket.
    pub kind: Kind,
    /// Its name.
    pub name: String,
    /// Its stats (a small passive's with the jewel's "also grant" stats).
    pub stats: Vec<StatAmount>,
    /// Its stats' increased effect (a small passive's: the jewel's
    /// "increased Effect"; else 0).
    pub effect: i64,
    /// Its group: the socket's proxy's.
    pub group: usize,
    /// Its orbit.
    pub orbit: u32,
    /// Its slot on the orbit.
    pub orbit_index: u32,
}

/// The nodes one socketed cluster jewel adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subgraph {
    /// The socket the jewel is in.
    pub socket: u32,
    /// The subgraph's number (its nodes' numbers without their place).
    pub id: u32,
    /// Its nodes, in PoB's order (mastery, sockets, notables, small).
    pub nodes: Vec<ClusterNode>,
    /// Its links, the entrance's to the socket last.
    pub links: Vec<(u32, u32)>,
    /// The node that links to the socket.
    pub entrance: u32,
}

/// The subgraphs of the jewels socketed in `tree`'s large cluster sockets
/// (`jewels`, by socket), and of the jewels in the sockets those add,
/// recursively. Empty when the tree has no cluster jewel data.
#[must_use]
pub fn subgraphs(tree: &Tree, jewels: &BTreeMap<u32, Jewel>) -> Vec<Subgraph> {
    let Some(data) = &tree.cluster_jewels else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for slot in data.slots.iter().filter(|s| s.size == LARGE) {
        let Ok(socket) = u32::try_from(slot.socket) else {
            continue;
        };
        if tree.node(socket).is_none() {
            continue;
        }
        if let Some(jewel) = jewels.get(&socket) {
            build(tree, data, socket, jewel, None, jewels, &mut out);
        }
    }
    out
}

/// The sizes as PoB names them (`PassiveTreeExpansionJewelSizes`' rows).
const LARGE: u32 = 2;

fn size_named(data: &ClusterJewels, size: u32, name: &str) -> bool {
    usize::try_from(size)
        .ok()
        .and_then(|s| data.sizes.get(s))
        .is_some_and(|n| n == name)
}

/// The socket of `group` with cluster index `index` (PoB's
/// `FindClusterSocket`).
fn cluster_socket(tree: &Tree, data: &ClusterJewels, group: usize, index: i64) -> Option<u32> {
    tree.nodes
        .values()
        .filter(|n| n.group == group)
        .find(|n| data.slot(i64::from(n.id)).is_some_and(|s| s.index == index))
        .map(|n| n.id)
}

/// A place on a ring of `from` places as a slot of an orbit of `to` (PoB's
/// `TranslateClusterOrbitIndex`).
#[must_use]
pub fn translate(place: u32, from: u32, to: u32) -> u32 {
    const T12_16: [u32; 12] = [0, 1, 3, 4, 5, 7, 8, 9, 11, 12, 13, 15];
    const T16_12: [u32; 16] = [0, 1, 1, 2, 3, 4, 4, 5, 6, 7, 7, 8, 9, 10, 10, 11];
    const T6_16: [u32; 6] = [0, 3, 5, 8, 11, 13];
    const T16_6: [u32; 16] = [0, 0, 0, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 5, 5, 5];
    let table: Option<&[u32]> = match (from, to) {
        _ if from == to => return place,
        (12, 16) => Some(&T12_16),
        (16, 12) => Some(&T16_12),
        (6, 16) => Some(&T6_16),
        (16, 6) => Some(&T16_6),
        _ => None,
    };
    match table.and_then(|t| t.get(usize::try_from(place).ok()?)) {
        Some(p) => *p,
        // PoB's fallback for a pair it does not know.
        None => place.saturating_mul(to).checked_div(from).unwrap_or(0),
    }
}

/// The places taken, by place on the base's ring.
type Places = BTreeMap<u32, ClusterNode>;

#[expect(
    clippy::too_many_lines,
    reason = "one function per PoB's BuildSubgraph keeps the port checkable line by line"
)]
fn build(
    tree: &Tree,
    data: &ClusterJewels,
    socket: u32,
    jewel: &Jewel,
    id: Option<u32>,
    jewels: &BTreeMap<u32, Jewel>,
    out: &mut Vec<Subgraph>,
) {
    let (Some(slot), Some(base)) = (data.slot(i64::from(socket)), data.jewel(&jewel.base)) else {
        return;
    };
    if !jewel.valid(data) {
        return;
    }
    let Ok(index) = u32::try_from(slot.index) else {
        return;
    };
    let mut id = id.unwrap_or(CLUSTER_NODE_BIT);
    match slot.size {
        2 => id += index << 6,
        1 => id += index << 9,
        _ => {}
    }
    let node_id = id + (base.size << 4);
    let Some(proxy) = u32::try_from(slot.proxy).ok().and_then(|p| tree.node(p)) else {
        return;
    };
    let group = proxy.group;
    let template = |n: u32| {
        tree.templates
            .get(&n)
            .map(|t| (t.name.clone(), t.stats.clone()))
            .or_else(|| tree.node(n).map(|t| (t.name.clone(), t.stats.clone())))
    };
    let node =
        |id, kind, (name, stats): (String, Vec<StatAmount>), orbit, orbit_index| ClusterNode {
            id,
            kind,
            name,
            stats,
            effect: 0,
            group,
            orbit,
            orbit_index,
        };

    if let Some(k) = jewel.keystone {
        let Some(t) = template(k) else { return };
        out.push(Subgraph {
            socket,
            id: node_id,
            nodes: vec![node(node_id, Kind::Keystone, t, 0, 1)],
            links: vec![(node_id, socket)],
            entrance: node_id,
        });
        return;
    }

    let orbit = base.size + 1;
    let mut notables = Vec::new();
    for &n in &jewel.notables {
        // PoB drops the whole subgraph when a notable is not in the data.
        let (Some(t), Some(special)) = (
            template(n),
            data.specials.iter().find(|s| s.node == i64::from(n)),
        ) else {
            return;
        };
        notables.push((special.order, t));
    }
    notables.sort_by_key(|(order, _)| *order);

    let skill = jewel.skill(data, base);
    let small: (String, Vec<StatAmount>) = skill
        .and_then(|s| template(u32::try_from(s.node).ok()?))
        .unwrap_or_else(|| ("Nothingness".to_owned(), Vec::new()));
    let sockets = i64::from(jewel.socket_override.or(jewel.socket_count).unwrap_or(0));
    let notable_count = i64::try_from(notables.len()).unwrap_or(i64::MAX);
    let node_count = jewel.node_count.map_or_else(
        || sockets + notable_count + i64::from(jewel.nothingness.unwrap_or(0)),
        |n| i64::from(n.clamp(base.min_nodes, base.max_nodes)),
    );
    let small_count = node_count - sockets - notable_count;
    let large = size_named(data, base.size, "Large");
    let medium = size_named(data, base.size, "Medium");
    let ring_closes = !size_named(data, base.size, "Small");

    let mut nodes: Vec<ClusterNode> = Vec::new();
    if let Some(m) = skill
        .and_then(|s| s.mastery)
        .and_then(|m| u32::try_from(m).ok())
        .and_then(template)
    {
        nodes.push(node(node_id + 12, Kind::Mastery, m, 0, 0));
    }

    let mut places: Places = BTreeMap::new();
    // First pass: sockets.
    let add_socket = |place: u32, jewel_index: i64, places: &mut Places| -> bool {
        let Some(s) = cluster_socket(tree, data, group, jewel_index) else {
            return false;
        };
        let name = tree.node(s).map(|n| n.name.clone()).unwrap_or_default();
        places.insert(
            place,
            node(s, Kind::ClusterSocket, (name, Vec::new()), orbit, place),
        );
        true
    };
    if large && sockets == 1 {
        if !add_socket(6, 1, &mut places) {
            return;
        }
    } else {
        let order = [0, 2, 1];
        let Ok(count) = usize::try_from(sockets) else {
            return;
        };
        if count > base.socket_indices.len() || count > order.len() {
            return;
        }
        for (&place, index) in base.socket_indices.iter().zip(order).take(count) {
            if !add_socket(place, index, &mut places) {
                return;
            }
        }
    }
    let socket_places: Vec<u32> = places.keys().copied().collect();

    // Second pass: notables.
    let mut notable_places = Vec::new();
    for &p in &base.notable_indices {
        if i64::try_from(notable_places.len()).unwrap_or(i64::MAX) == notable_count {
            break;
        }
        let mut p = p;
        if medium {
            if sockets == 0 && notable_count == 2 {
                p = match p {
                    6 => 4,
                    10 => 8,
                    _ => p,
                };
            } else if node_count == 4 {
                p = match p {
                    10 => 9,
                    2 => 3,
                    _ => p,
                };
            }
        }
        if !places.contains_key(&p) {
            notable_places.push(p);
        }
    }
    notable_places.sort_unstable();
    let mut notable_ids = Vec::new();
    for ((_, t), &p) in notables.into_iter().zip(&notable_places) {
        places.insert(p, node(node_id + p, Kind::Notable, t, orbit, p));
        notable_ids.push(p);
    }

    // Third pass: small passives.
    let mut small_places = Vec::new();
    for &p in &base.small_indices {
        if i64::try_from(small_places.len()).unwrap_or(i64::MAX) == small_count {
            break;
        }
        let mut p = p;
        if medium {
            if node_count == 5 && p == 4 {
                p = 3;
            } else if node_count == 4 {
                p = match p {
                    8 => 9,
                    4 => 3,
                    _ => p,
                };
            }
        }
        if !places.contains_key(&p) {
            small_places.push(p);
        }
    }
    let mut small_ids = Vec::new();
    for &p in small_places
        .iter()
        .take(usize::try_from(small_count.max(0)).unwrap_or(0))
    {
        let mut stats = small.1.clone();
        stats.extend(jewel.added.iter().cloned());
        let mut n = node(
            node_id + p,
            Kind::Normal,
            (small.0.clone(), stats),
            orbit,
            p,
        );
        n.effect = jewel.effect;
        places.insert(p, n);
        small_ids.push(p);
    }

    let Some(entrance) = places.get(&0).map(|n| n.id) else {
        return;
    };

    // The ring: places turned by the socket's start index and translated
    // to the orbit's slots.
    let (Some(&per_orbit), Some(&start)) = (
        usize::try_from(orbit)
            .ok()
            .and_then(|o| tree.orbit_slots.get(o)),
        usize::try_from(base.size)
            .ok()
            .and_then(|s| slot.start_indices.get(s)),
    ) else {
        return;
    };
    if base.total_indices == 0 {
        return;
    }
    for n in places.values_mut() {
        let turned = (n.orbit_index + start) % base.total_indices;
        n.orbit_index = translate(turned, base.total_indices, per_orbit);
    }

    // Links around the ring in place order, closed unless small.
    let ring: Vec<u32> = places.values().map(|n| n.id).collect();
    let mut links: Vec<(u32, u32)> = ring.windows(2).map(|w| (w[1], w[0])).collect();
    if ring_closes
        && let (Some(&first), Some(&last)) = (ring.first(), ring.last())
        && first != last
    {
        links.push((first, last));
    }
    links.push((entrance, socket));

    // PoB's order: mastery, sockets, notables, small passives.
    for p in socket_places.iter().chain(&notable_ids).chain(&small_ids) {
        if let Some(n) = places.remove(p) {
            nodes.push(n);
        }
    }
    let nested: Vec<u32> = nodes
        .iter()
        .filter(|n| n.kind == Kind::ClusterSocket)
        .map(|n| n.id)
        .collect();
    out.push(Subgraph {
        socket,
        id: node_id,
        nodes,
        links,
        entrance,
    });
    for s in nested {
        if let Some(j) = jewels.get(&s) {
            build(tree, data, s, j, Some(id), jewels, out);
        }
    }
}

/// `tree` with `subgraphs` on it (module docs): the cluster areas' own
/// links cut (PoB never puts them on its graph), each subgraph's nodes
/// placed around its proxy's group and linked, a socket it adds standing
/// in for the tree's nested socket of that number. Nodes a jewel adds are
/// modelled (an edit allocates them), but for its mastery (drawn only).
/// The tree as it is when it has no cluster jewel data.
#[must_use]
pub fn expand(tree: &Tree, subgraphs: &[Subgraph]) -> Tree {
    let mut t = tree.clone();
    if tree.cluster_jewels.is_none() {
        return t;
    }
    let area: BTreeSet<u32> = t
        .nodes
        .values()
        .filter(|n| n.cluster_area)
        .map(|n| n.id)
        .collect();
    for n in t.nodes.values_mut() {
        if area.contains(&n.id) {
            n.neighbours.clear();
        } else {
            n.neighbours.retain(|m| !area.contains(m));
        }
    }
    for g in subgraphs {
        for c in &g.nodes {
            let position = t.layout.as_ref().and_then(|l| {
                let (dx, dy) = l.offset(
                    usize::try_from(c.orbit).ok()?,
                    usize::try_from(c.orbit_index).ok()?,
                )?;
                let centre = t.groups.get(c.group)?;
                Some((centre.x + dx, centre.y + dy))
            });
            let modelled = c.kind != Kind::Mastery;
            match t.nodes.get_mut(&c.id) {
                Some(n) => {
                    n.kind = c.kind;
                    n.group = c.group;
                    n.orbit = c.orbit;
                    n.orbit_index = c.orbit_index;
                    n.position = position;
                    n.neighbours.clear();
                    n.modelled = modelled;
                    n.cluster_area = false;
                }
                None => {
                    t.nodes.insert(
                        c.id,
                        Node {
                            id: c.id,
                            name: c.name.clone(),
                            kind: c.kind,
                            ascendancy: None,
                            class_start: None,
                            mastery_group: None,
                            group: c.group,
                            orbit: c.orbit,
                            orbit_index: c.orbit_index,
                            position,
                            neighbours: Vec::new(),
                            stats: c.stats.clone(),
                            choice_option: false,
                            modelled,
                            cluster_area: false,
                            expansion: true,
                        },
                    );
                }
            }
        }
    }
    for g in subgraphs {
        for &(a, b) in &g.links {
            for (x, y) in [(a, b), (b, a)] {
                if let Some(n) = t.nodes.get_mut(&x)
                    && !n.neighbours.contains(&y)
                {
                    n.neighbours.push(y);
                    n.neighbours.sort_unstable();
                }
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::translate;

    #[test]
    fn translate_should_map_rings_as_pob() {
        assert_eq!(translate(5, 12, 16), 7);
        assert_eq!(translate(11, 12, 16), 15);
        assert_eq!(translate(3, 16, 12), 2);
        assert_eq!(translate(4, 6, 16), 11);
        assert_eq!(translate(13, 16, 6), 5);
        assert_eq!(translate(4, 6, 6), 4);
        // PoB's fallback for a pair it does not know.
        assert_eq!(translate(3, 8, 16), 6);
    }
}

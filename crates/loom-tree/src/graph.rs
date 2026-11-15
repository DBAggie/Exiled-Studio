//! The tree's graph: nodes with kinds, groups, positions, classes.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::Bundle;
use loom_data::bundle::passives::{PassiveTables, StatAmount};
use loom_data::bundle::poe1::JewelRadius;
use loom_data::bundle::tree_layout::TreeLayout;

/// What a node is, for drawing and for the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A small passive.
    Normal,
    /// A notable.
    Notable,
    /// A keystone.
    Keystone,
    /// A mastery (takes one effect; attaches through its group).
    Mastery,
    /// A jewel socket.
    JewelSocket,
    /// A cluster jewel socket.
    ClusterSocket,
    /// A class's start.
    ClassStart,
    /// An ascendancy's start.
    AscendancyStart,
}

/// A node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Its number (the graph's ID; what builds list).
    pub id: u32,
    /// Its name.
    pub name: String,
    /// Its kind.
    pub kind: Kind,
    /// The ascendancy it belongs to (its ID), if any.
    pub ascendancy: Option<String>,
    /// For a class start: the class's number.
    pub class_start: Option<u32>,
    /// For a mastery: its mastery group's ID.
    pub mastery_group: Option<String>,
    /// Its group (an index into [`Tree::groups`]).
    pub group: usize,
    /// Its orbit.
    pub orbit: u32,
    /// Its slot on the orbit.
    pub orbit_index: u32,
    /// Where it is drawn, when the bundle has the layout.
    pub position: Option<(f64, f64)>,
    /// The nodes it connects to, ascending.
    pub neighbours: Vec<u32>,
    /// Its stats, as the bundle has them (stat IDs and values).
    pub stats: Vec<StatAmount>,
    /// Whether it is one option of a multiple choice (allocating it
    /// deallocates the other options; it costs no ascendancy point).
    pub choice_option: bool,
    /// Whether the editor models it (module docs); a node it does not is
    /// kept in a spec and never allocated by an edit.
    pub modelled: bool,
    /// Whether it exists only once a cluster jewel is socketed: a nested
    /// expansion socket, a node of a group holding one, or a position
    /// proxy linked to one (PoB's `PassiveSpec:Init` leaves these out of a
    /// spec: proxies, proxy groups, sockets with a parent). Never drawn,
    /// never modelled.
    pub cluster_area: bool,
    /// Whether the game marks it as a cluster jewel's (the passives
    /// table's `expansion`): a socket or node of the cluster areas.
    pub expansion: bool,
}

/// A group of nodes around one centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Group {
    /// Centre x.
    pub x: f64,
    /// Centre y (down is positive).
    pub y: f64,
}

/// An ascendancy of a class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AscendancyClass {
    /// Its number within the class (1 to 3; PoB's `ascendClassId`).
    pub number: u32,
    /// Its ID (what nodes name).
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its start node, if the tree has one.
    pub start: Option<u32>,
}

/// A class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    /// Its number (PoB's `classId`).
    pub number: u32,
    /// Its name.
    pub name: String,
    /// Its start node.
    pub start: u32,
    /// Its ascendancies, by number.
    pub ascendancies: Vec<AscendancyClass>,
}

/// The tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// Nodes by number.
    pub nodes: BTreeMap<u32, Node>,
    /// Groups.
    pub groups: Vec<Group>,
    /// Classes, by number.
    pub classes: Vec<Class>,
    /// Each mastery group's effects (their hashes, what builds name).
    pub mastery_effects: BTreeMap<String, Vec<u32>>,
    /// Each orbit's radius, when the bundle has the layout.
    pub orbit_radii: Vec<f64>,
    /// The jewel radii, when the bundle has them (`poe1/jewel_radii.json`).
    pub jewel_radii: Vec<JewelRadius>,
}

fn kind_of(p: &loom_data::bundle::passives::Passive, socket_outer: bool) -> Kind {
    if !p.characters.is_empty() {
        Kind::ClassStart
    } else if p.ascendancy_start {
        Kind::AscendancyStart
    } else if p.mastery_group.is_some() {
        Kind::Mastery
    } else if p.jewel_socket && p.expansion && socket_outer {
        Kind::ClusterSocket
    } else if p.jewel_socket {
        Kind::JewelSocket
    } else if p.keystone {
        Kind::Keystone
    } else if p.notable {
        Kind::Notable
    } else {
        Kind::Normal
    }
}

impl Tree {
    /// The tree of the bundle's passive tables, placed by `layout` when
    /// given (a layout that does not fit the orbits places nothing).
    #[must_use]
    pub fn from_tables(p: &PassiveTables, layout: Option<&TreeLayout>) -> Self {
        let passives: BTreeMap<i64, &loom_data::bundle::passives::Passive> =
            p.passives.iter().map(|x| (x.node, x)).collect();
        let layout = layout.filter(|l| l.fits(&p.tree.orbit_slots));
        let groups: Vec<Group> = p
            .tree
            .groups
            .iter()
            .map(|g| Group {
                x: f64::from(g.x),
                y: f64::from(g.y),
            })
            .collect();
        // Undirected links, each stored once in the graph.
        let mut links: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for g in &p.tree.groups {
            for n in &g.nodes {
                let Ok(a) = u32::try_from(n.node) else {
                    continue;
                };
                links.entry(a).or_default();
                for c in &n.connections {
                    if let Ok(b) = u32::try_from(c.node)
                        && a != b
                    {
                        links.entry(a).or_default().insert(b);
                        links.entry(b).or_default().insert(a);
                    }
                }
            }
        }
        let expansion = |n: u32| passives.get(&i64::from(n)).is_some_and(|x| x.expansion);
        let mut nodes = BTreeMap::new();
        for (gi, g) in p.tree.groups.iter().enumerate() {
            for n in &g.nodes {
                let Ok(id) = u32::try_from(n.node) else {
                    continue;
                };
                let Some(row) = passives.get(&n.node) else {
                    continue;
                };
                let neighbours: Vec<u32> = links
                    .get(&id)
                    .map(|s| s.iter().copied().collect())
                    .unwrap_or_default();
                // A cluster socket on the main tree has a neighbour outside
                // the cluster areas; one inside them does not.
                let outer = neighbours.iter().any(|m| !expansion(*m));
                let orbit = u32::try_from(n.orbit).unwrap_or(0);
                let orbit_index = u32::try_from(n.orbit_index).unwrap_or(0);
                let position = layout.and_then(|l| {
                    let (dx, dy) = l.offset(orbit as usize, orbit_index as usize)?;
                    Some((groups[gi].x + dx, groups[gi].y + dy))
                });
                nodes.insert(
                    id,
                    Node {
                        id,
                        name: row.name.clone(),
                        kind: kind_of(row, outer),
                        ascendancy: row.ascendancy.clone(),
                        class_start: None,
                        mastery_group: row.mastery_group.clone(),
                        group: gi,
                        orbit,
                        orbit_index,
                        position,
                        neighbours,
                        stats: row.stats.clone(),
                        choice_option: row.multiple_choice_option,
                        modelled: false,
                        cluster_area: false,
                        expansion: row.expansion,
                    },
                );
            }
        }
        let mut classes: Vec<Class> = p
            .characters
            .iter()
            .filter_map(|c| {
                let number = u32::try_from(c.class_number).ok()?;
                let start = p.passives.iter().find(|x| x.characters.contains(&c.id))?;
                let start = u32::try_from(start.node).ok()?;
                let mut ascendancies: Vec<AscendancyClass> = p
                    .ascendancies
                    .iter()
                    .filter(|a| a.character.as_deref() == Some(c.id.as_str()))
                    .filter_map(|a| {
                        Some(AscendancyClass {
                            number: u32::try_from(a.class_number).ok()?,
                            id: a.id.clone(),
                            name: a.name.clone(),
                            start: p
                                .passives
                                .iter()
                                .find(|x| {
                                    x.ascendancy_start && x.ascendancy.as_deref() == Some(&a.id)
                                })
                                .and_then(|x| u32::try_from(x.node).ok()),
                        })
                    })
                    .collect();
                ascendancies.sort_by_key(|a| a.number);
                Some(Class {
                    number,
                    name: c.name.clone(),
                    start,
                    ascendancies,
                })
            })
            .collect();
        classes.sort_by_key(|c| c.number);
        for c in &classes {
            if let Some(n) = nodes.get_mut(&c.start) {
                n.class_start = Some(c.number);
            }
        }
        let mastery_effects = p
            .mastery_groups
            .iter()
            .map(|g| {
                let hashes = g
                    .effects
                    .iter()
                    .filter_map(|e| p.mastery_effects.iter().find(|m| &m.id == e))
                    .filter_map(|m| u32::try_from(m.hash).ok())
                    .collect();
                (g.id.clone(), hashes)
            })
            .collect();
        // The cluster areas (`Node::cluster_area`): nested expansion
        // sockets, the groups holding one, and stat-less non-socket nodes
        // linked to one (the position proxies).
        let nested = |n: u32| {
            passives
                .get(&i64::from(n))
                .is_some_and(|x| x.expansion && x.jewel_socket)
        };
        let nested_groups: BTreeSet<usize> = nodes
            .values()
            .filter(|n| nested(n.id))
            .map(|n| n.group)
            .collect();
        for node in nodes.values_mut() {
            let proxy = node.stats.is_empty()
                && !passives
                    .get(&i64::from(node.id))
                    .is_some_and(|x| x.jewel_socket)
                && node.neighbours.iter().any(|m| nested(*m));
            node.cluster_area = nested_groups.contains(&node.group) || proxy;
        }
        let mut tree = Self {
            nodes,
            groups,
            classes,
            mastery_effects,
            orbit_radii: layout.map(|l| l.orbit_radii.clone()).unwrap_or_default(),
            jewel_radii: Vec::new(),
        };
        tree.mark_modelled();
        tree
    }

    /// Marks the nodes the editor models (module docs).
    fn mark_modelled(&mut self) {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut queue: Vec<u32> = Vec::new();
        for c in &self.classes {
            seen.insert(c.start);
            queue.push(c.start);
            for a in &c.ascendancies {
                if let Some(s) = a.start {
                    seen.insert(s);
                    queue.push(s);
                }
            }
        }
        while let Some(a) = queue.pop() {
            let Some(node) = self.nodes.get(&a) else {
                continue;
            };
            let from = node.ascendancy.clone();
            for &b in &node.neighbours {
                let Some(next) = self.nodes.get(&b) else {
                    continue;
                };
                let enters = matches!(next.kind, Kind::ClassStart | Kind::AscendancyStart)
                    || next.cluster_area
                    || next.mastery_group.is_some()
                    || next.ascendancy != from;
                if !enters && seen.insert(b) {
                    queue.push(b);
                }
            }
        }
        // Masteries linked to nodes the editor models.
        for node in self.nodes.values_mut() {
            node.modelled = seen.contains(&node.id)
                || (node.kind == Kind::Mastery && node.neighbours.iter().any(|n| seen.contains(n)));
        }
    }

    /// The tree of `bundle`: its passive tables placed by its layout, with
    /// its jewel radii; `None` without passive tables.
    #[must_use]
    pub fn from_bundle(bundle: &Bundle) -> Option<Self> {
        let mut tree = Self::from_tables(bundle.passives.as_ref()?, bundle.tree_layout.as_ref());
        tree.jewel_radii = bundle
            .poe1
            .as_ref()
            .and_then(|p| p.jewel_radii.clone())
            .unwrap_or_default();
        Some(tree)
    }

    /// The full radius named `name` (an item's `Radius:` line), if the
    /// bundle has it.
    #[must_use]
    pub fn radius_named(&self, name: &str) -> Option<crate::poe1::radius::Ring> {
        self.jewel_radii
            .iter()
            .find(|r| !r.name.is_empty() && r.name.eq_ignore_ascii_case(name))
            .map(|r| crate::poe1::radius::Ring {
                inner: 0,
                outer: r.radius,
            })
    }

    /// The ring with value `k` (a ring jewel's `local_jewel_variable_ring_radius_value`):
    /// the ring of the table's `k`-th row.
    #[must_use]
    pub fn ring(&self, k: i64) -> Option<crate::poe1::radius::Ring> {
        let r = self
            .jewel_radii
            .get(usize::try_from(k.checked_sub(1)?).ok()?)?;
        Some(crate::poe1::radius::Ring {
            inner: r.ring_inner,
            outer: r.ring_outer,
        })
    }

    /// The node `id`, if the tree has it.
    #[must_use]
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The class numbered `number`.
    #[must_use]
    pub fn class(&self, number: u32) -> Option<&Class> {
        self.classes.iter().find(|c| c.number == number)
    }

    /// Whether the tree has positions (the bundle has the layout).
    #[must_use]
    pub fn placed(&self) -> bool {
        !self.orbit_radii.is_empty()
    }
}

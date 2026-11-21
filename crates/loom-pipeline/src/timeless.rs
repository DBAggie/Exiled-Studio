//! Timeless jewels (Phase 6 P6-10a; plan
//! `dev/active/phase-6-mechanics/p6-10a-timeless-plan.md`): what a socketed
//! Timeless jewel makes of the nodes in its radius, as PoB's
//! `PassiveSpec.lua` (`BuildAllDependsAndPaths`, 1102 to 1376) does.
//!
//! **The jewel** ([`socketed`]): a Timeless Jewel in an allocated socket
//! (PoB conquers only from one, `allocNodes[socket]`), read from its
//! selected lines' stats: `local_unique_jewel_alternate_tree_version` (the
//! jewel type, PoB's `timelessJewelTypes` index), `..._seed` (the raw seed:
//! Elegant Hubris's line shows it times 20), `..._keystone` (the
//! conqueror's index) and `..._internal_revision` (its version; the
//! description's "1 and above" reads as its lowest value). Its radius is
//! its `Radius:` line's.
//!
//! **The nodes** ([`conquests`]): every node in the ring but class starts,
//! sockets, ascendancy nodes, masteries, and cluster jewels' generated
//! nodes (PoB's types `Keystone`, `Notable`, `Normal` of the tree itself):
//! - **a keystone** becomes the conqueror's keystone: the row of the jewel
//!   type with that conqueror index and version that replaces keystones;
//! - **a notable**, and a Glorious Vanity small passive: the results
//!   ([`loom_data::bundle::timeless::read`], PoB's `readLUT`): a
//!   replacement takes its row's stats (Glorious Vanity's rolls, else the
//!   row's fixed values); additions add theirs. Glorious Vanity's additions
//!   replace the node with Might or Legacy of the Vaal (PoB's bias: more
//!   additions with a global ID up to 21 is Might), each addition's roll
//!   summed per addition;
//! - **another small passive** (the other types): an attribute passive
//!   (named Strength, Dexterity, or Intelligence) is the small attribute
//!   type, else the small normal type. When the jewel type's tree version
//!   replaces that type, the node becomes the type's one replacement row;
//!   else it gains the type's one addition row (a tattooed node counts as
//!   an attribute one, as PoB's `node.isTattoo`). This is PoB's hand-written
//!   rule for each conqueror (+4 or +2 Strength or Dexterity, +5 Devotion
//!   or the devotion node, the blank node, 2% or 1% increased Ward), read
//!   from the game's rows.
//!
//! **The Abyss jewels** (types 7 to 11, [`abyss`]) have no radius: their
//! results list the nodes they change.
//! - Tecrod, Ulaman, Kurgal, and Amanamu: by socket and seed
//!   ([`loom_data::bundle::timeless::abyss::read_socket`]).
//! - Zorath: by node and seed, for the nodes of the allocated shortest path
//!   from its socket to the class start, and the node each ascendancy's
//!   section picks for the seed
//!   ([`loom_data::bundle::timeless::abyss::read_path`]).
//! - Each change's parts apply in order: a replacement takes its row's
//!   stats with the part's rolls (signed, the stats' values), and an
//!   addition adds its row's. Sockets are never changed.
//! - A node a radius jewel conquers is that jewel's, as in PoB, where the
//!   Abyss jewels are read first.
//!
//! The results files are large, so the host reads only the types a build
//! needs ([`needed`]) and hands their bytes over ([`TimelessData`]). A node
//! whose result is not known is a reason, never its old stats: no data
//! (`timeless-results`), a jewel whose lines do not read (`timeless-jewel`).
//! An Abyss jewel without its data is the reason on its socket, since the
//! nodes it changes are not known without it.

use std::collections::{BTreeMap, BTreeSet};

use loom_calc::input::{Conquest, LineValues};
use loom_domain::ItemId;

/// PoB's rolls of the items' lines, by item and line (the host's
/// `CalcOptions::item_lines`).
pub type Rolls = BTreeMap<ItemId, BTreeMap<usize, LineValues>>;
use loom_data::bundle::passives::StatAmount;
use loom_data::bundle::timeless::abyss::{ADDS, AbyssPart, REPLACES};
use loom_data::bundle::timeless::{
    FIRST_ABYSS, GLORIOUS_VANITY, LutResult, Timeless, TimelessLuts, read,
};
use loom_domain::{Build, LineKind, LineResolution, PassiveSpec, Resolved, StatValue};
use loom_tree::Tree;
use loom_tree::graph::Kind;

/// A Timeless Jewel's base.
pub const BASE: &str = "Timeless Jewel";
/// The jewel type stat.
const VERSION: &str = "local_unique_jewel_alternate_tree_version";
/// The seed stat.
const SEED: &str = "local_unique_jewel_alternate_tree_seed";
/// The conqueror index stat.
const KEYSTONE: &str = "local_unique_jewel_alternate_tree_keystone";
/// The conqueror version stat.
const REVISION: &str = "local_unique_jewel_alternate_tree_internal_revision";
/// Every Timeless jewel stat starts so.
const PREFIX: &str = "local_unique_jewel_alternate_tree_";
/// PoB's addition IDs up to this one count towards Might of the Vaal
/// (`PassiveSpec.lua` 1268 to 1279).
const MIGHT_BIAS: u32 = 21;
/// Might of the Vaal's and Legacy of the Vaal's rows (PoB's
/// `legionNodes[77]` and `[78]`, 1-based).
const MIGHT_ROW: u32 = 76;
const LEGACY_ROW: u32 = 77;
/// The passive types of the alternate tables (`PassiveSkillSizes`).
const SMALL_ATTRIBUTE: u32 = 1;
const SMALL_NORMAL: u32 = 2;
const KEYSTONE_TYPE: u32 = 4;
/// The names of attribute passives (PoB's `attributes`).
const ATTRIBUTES: [&str; 3] = ["Dexterity", "Intelligence", "Strength"];
/// Zorath, the Abyss jewel read along a path.
const ZORATH: u32 = 11;

/// A socketed Timeless jewel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socketed {
    /// Its socket.
    pub socket: u32,
    /// The nodes in its radius (none for an Abyss jewel).
    pub ring: BTreeSet<u32>,
    /// `(type, raw seed, conqueror, version)`; `None` when its lines do not
    /// read.
    pub jewel: Option<(u32, u32, u32, u32)>,
    /// Its type alone, when its lines name it (what the host reads; the
    /// seed may need the rolls).
    pub jewel_type: Option<u32>,
}

/// The results the host read, by jewel type ([`needed`]), with the
/// bundle's tables.
pub struct TimelessData<'a> {
    /// The extracted tables.
    pub timeless: &'a Timeless,
    /// How to read the results.
    pub luts: &'a TimelessLuts,
    /// Each type's results file's bytes.
    pub results: BTreeMap<u32, &'a [u8]>,
}

/// A stat's value: its roll (PoB's `{range:r}`, from the host's line
/// values) when it has one, else a settled value; the conqueror version
/// reads a range's lowest value (the description's "1 and above").
fn value(stat: &str, v: &StatValue, rolled: Option<i64>) -> Option<i64> {
    if rolled.is_some() {
        return rolled;
    }
    match v {
        StatValue::Exact { min, max } if min == max => Some(*min),
        StatValue::Exact { min, .. } | StatValue::Ranged { min, .. } if stat == REVISION => {
            Some(*min)
        }
        StatValue::Hidden { assumed } => *assumed,
        _ => None,
    }
}

/// The Timeless jewels socketed in `spec` (module docs), each in an
/// allocated socket.
#[must_use]
pub fn socketed(build: &Build, spec: &PassiveSpec, tree: &Tree, rolls: &Rolls) -> Vec<Socketed> {
    let allocated: BTreeSet<u32> = spec.nodes.value().iter().map(|n| n.0).collect();
    let mut out = Vec::new();
    for socket in spec.jewel_sockets.typed() {
        if !allocated.contains(&socket.node.0) {
            continue;
        }
        let Some(item) = socket
            .item
            .and_then(|id| build.items.items().find(|i| i.id == id))
        else {
            continue;
        };
        let mut label = None;
        let mut timeless = item.body.base.as_deref() == Some(BASE);
        let mut stats: BTreeMap<String, i64> = BTreeMap::new();
        let item_rolls = rolls.get(&item.id);
        for (n, line) in item.body.lines.iter().enumerate() {
            match &line.kind {
                LineKind::Spec { key, value, .. } if key == "Radius" => {
                    label = Some(value.as_str());
                }
                LineKind::Modifier(m) if m.selected => {
                    if let LineResolution::Resolved {
                        result: Resolved::Stats(lines),
                        ..
                    } = &m.resolution
                    {
                        let rolled = match item_rolls.and_then(|r| r.get(&n)) {
                            Some(LineValues::Rolled(v)) if v.len() == lines.len() => v.clone(),
                            _ => vec![None; lines.len()],
                        };
                        for (s, roll) in lines.iter().zip(rolled) {
                            if !s.stat.as_str().starts_with(PREFIX) {
                                continue;
                            }
                            timeless = true;
                            if let Some(v) = value(s.stat.as_str(), &s.value, roll) {
                                stats.insert(s.stat.as_str().to_owned(), v);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if !timeless {
            continue;
        }
        let get = |s: &str| stats.get(s).and_then(|v| u32::try_from(*v).ok());
        let abyss = get(VERSION).is_some_and(|t| t >= FIRST_ABYSS);
        let ring = if abyss {
            BTreeSet::new()
        } else {
            let Some(ring) = label.and_then(|l| tree.radius_named(l)) else {
                continue;
            };
            loom_tree::poe1::radius::in_ring(tree, socket.node.0, ring, false)
        };
        let jewel = match (get(VERSION), get(SEED), get(KEYSTONE)) {
            (Some(t), Some(seed), Some(c)) => Some((t, seed, c, get(REVISION).unwrap_or(0))),
            _ => None,
        };
        out.push(Socketed {
            jewel_type: get(VERSION),
            socket: socket.node.0,
            ring,
            jewel,
        });
    }
    out
}

/// The jewel types whose results the build's active spec needs (the host
/// reads only these files).
#[must_use]
pub fn needed(build: &Build, tree: &Tree) -> BTreeSet<u32> {
    let Some(spec) = loom_calc::poe1::collect::active_spec(build) else {
        return BTreeSet::new();
    };
    socketed(build, spec, tree, &Rolls::new())
        .into_iter()
        .filter_map(|s| s.jewel_type)
        .collect()
}

fn amounts(stats: &[loom_data::bundle::timeless::AltStat], rolls: &[i64]) -> Vec<StatAmount> {
    stats
        .iter()
        .enumerate()
        .map(|(i, s)| StatAmount {
            stat: s.stat.clone(),
            value: rolls.get(i).copied().unwrap_or(s.min),
        })
        .collect()
}

fn reason(what: &'static str) -> Conquest {
    Conquest {
        replaced: true,
        stats: Vec::new(),
        reason: Some(what),
    }
}

/// One node's conquest from the results (notables, Glorious Vanity's small
/// passives).
fn from_results(data: &TimelessData<'_>, jewel: u32, seed: u32, node: u32) -> Conquest {
    let (Some(lut), Some(&bytes)) = (data.luts.jewel(jewel), data.results.get(&jewel)) else {
        return reason("timeless-results");
    };
    let shown = seed.saturating_mul(lut.seed_divisor.max(1));
    let additions = data.luts.additions;
    match read(data.luts, lut, bytes, shown, node) {
        Some(LutResult::Replaced { global, rolls }) => {
            match data.timeless.passive(global, additions) {
                Some(p) => Conquest {
                    replaced: true,
                    stats: amounts(
                        &p.stats,
                        &rolls.iter().map(|r| i64::from(*r)).collect::<Vec<_>>(),
                    ),
                    reason: None,
                },
                None => reason("timeless-results"),
            }
        }
        Some(LutResult::Added { additions: list }) => {
            let mut summed: BTreeMap<u32, Option<u32>> = BTreeMap::new();
            for (id, roll) in &list {
                let e = summed.entry(*id).or_insert(None);
                *e = match (*e, roll) {
                    (Some(a), Some(b)) => Some(a + b),
                    (None, r) => *r,
                    (a, None) => a,
                };
            }
            let mut stats = Vec::new();
            for (id, roll) in summed {
                let Some(a) = data.timeless.addition(id) else {
                    return reason("timeless-results");
                };
                stats.extend(amounts(
                    &a.stats,
                    &roll.map(|r| vec![i64::from(r)]).unwrap_or_default(),
                ));
            }
            // Glorious Vanity's additions come on Might or Legacy of the Vaal,
            // which have no stats of their own.
            if jewel == GLORIOUS_VANITY {
                let might = list.iter().filter(|(id, _)| *id <= MIGHT_BIAS).count();
                let row = if 2 * might >= list.len() {
                    MIGHT_ROW
                } else {
                    LEGACY_ROW
                };
                let mut own = data
                    .timeless
                    .passives
                    .iter()
                    .find(|p| p.row == row)
                    .map(|p| amounts(&p.stats, &[]))
                    .unwrap_or_default();
                own.extend(stats);
                return Conquest {
                    replaced: true,
                    stats: own,
                    reason: None,
                };
            }
            Conquest {
                replaced: false,
                stats,
                reason: None,
            }
        }
        None => reason("timeless-results"),
    }
}

/// The nodes the Timeless jewels socketed in `spec` conquer (module docs).
/// Without `data`, every node in a radius is the `timeless-results` reason.
#[must_use]
pub fn conquests(
    build: &Build,
    spec: &PassiveSpec,
    tree: &Tree,
    data: Option<&TimelessData<'_>>,
    rolls: &Rolls,
    tattooed: &BTreeSet<u32>,
) -> BTreeMap<u32, Conquest> {
    let jewels = socketed(build, spec, tree, rolls);
    // The Abyss jewels first; a radius jewel's conquest replaces theirs.
    let mut out = abyss(spec, tree, data, &jewels);
    let mut by_radius = BTreeSet::new();
    for s in &jewels {
        for &node in &s.ring {
            let Some(n) = tree.node(node) else {
                continue;
            };
            // A cluster jewel's generated node is never conquered: PoB's
            // `nodesInRadius` holds only the tree's own nodes.
            if node & loom_tree::poe1::CLUSTER_NODE_BIT != 0
                || n.ascendancy.is_some()
                || !matches!(n.kind, Kind::Keystone | Kind::Notable | Kind::Normal)
            {
                continue;
            }
            let conquest = match (s.jewel, data) {
                (None, _) => reason("timeless-jewel"),
                (Some(_), None) => reason("timeless-results"),
                (Some((t, seed, conqueror, version)), Some(d)) => match n.kind {
                    Kind::Keystone => d
                        .timeless
                        .passives
                        .iter()
                        .find(|p| {
                            p.tree_version == t
                                && p.conqueror_index == conqueror
                                && p.conqueror_version == version
                                && p.passive_types.contains(&KEYSTONE_TYPE)
                        })
                        .map_or_else(
                            || reason("timeless-jewel"),
                            |p| Conquest {
                                replaced: true,
                                stats: amounts(&p.stats, &[]),
                                reason: None,
                            },
                        ),
                    Kind::Notable => from_results(d, t, seed, node),
                    _ if t == GLORIOUS_VANITY => from_results(d, t, seed, node),
                    // A tattooed small passive counts as an attribute
                    // one (PoB's `node.isTattoo`, `PassiveSpec.lua` 1355).
                    _ => small(
                        d.timeless,
                        t,
                        ATTRIBUTES.contains(&n.name.as_str()) || tattooed.contains(&node),
                    ),
                },
            };
            // Two jewels reaching one node: PoB keeps the last socket's; the
            // engine keeps the first and the order is the spec's.
            if by_radius.insert(node) {
                out.insert(node, conquest);
            }
        }
    }
    out
}

/// The nodes the Abyss jewels change (module docs); an Abyss jewel whose
/// lines or data are missing is a reason on its socket.
fn abyss(
    spec: &PassiveSpec,
    tree: &Tree,
    data: Option<&TimelessData<'_>>,
    jewels: &[Socketed],
) -> BTreeMap<u32, Conquest> {
    let allocated: BTreeSet<u32> = spec.nodes.value().iter().map(|n| n.0).collect();
    let mut out = BTreeMap::new();
    for s in jewels.iter().filter(|s| s.ring.is_empty()) {
        let Some((t, seed, ..)) = s.jewel else {
            out.insert(s.socket, reason("timeless-jewel"));
            continue;
        };
        let (Some(d), Some(lut)) = (data, data.and_then(|d| d.luts.jewel(t))) else {
            out.insert(s.socket, reason("timeless-results"));
            continue;
        };
        let Some(&bytes) = d.results.get(&t) else {
            out.insert(s.socket, reason("timeless-results"));
            continue;
        };
        let changes = if t == ZORATH {
            match shortest_path(tree, &allocated, s.socket) {
                Some(path) => {
                    loom_data::bundle::timeless::abyss::read_path(d.luts, lut, bytes, seed, &path)
                }
                None => BTreeMap::new(),
            }
        } else {
            loom_data::bundle::timeless::abyss::read_socket(d.luts, lut, bytes, seed, s.socket)
        };
        for (node, parts) in changes {
            if tree
                .node(node)
                .is_some_and(|n| matches!(n.kind, Kind::JewelSocket | Kind::ClusterSocket))
            {
                continue;
            }
            out.insert(node, apply(d, &parts));
        }
    }
    out
}

/// An Abyss change's parts, in order (module docs).
fn apply(data: &TimelessData<'_>, parts: &[AbyssPart]) -> Conquest {
    let mut conquest = Conquest {
        replaced: false,
        stats: Vec::new(),
        reason: None,
    };
    for part in parts {
        match part.kind {
            REPLACES => {
                let Some(row) = data.timeless.passive(part.global, data.luts.additions) else {
                    return reason("timeless-results");
                };
                conquest.replaced = true;
                conquest.stats = amounts(&row.stats, &part.rolls);
            }
            ADDS => {
                let Some(row) = data.timeless.addition(part.global) else {
                    return reason("timeless-results");
                };
                conquest.stats.extend(amounts(&row.stats, &part.rolls));
            }
            _ => return reason("timeless-results"),
        }
    }
    conquest
}

/// The allocated shortest path from `socket` to the class start, both
/// included (PoB's `GetShortestPathToClassStart`): a breadth-first search
/// through allocated nodes, not through a mastery, a class start (but the
/// target), or an ascendancy start. `None` when the socket is not
/// allocated or nothing joins it to the start. Between paths of the same
/// length, PoB's pick follows its tree's link order and the search here
/// takes neighbours in ascending order, so the two can differ.
fn shortest_path(tree: &Tree, allocated: &BTreeSet<u32>, socket: u32) -> Option<BTreeSet<u32>> {
    if !allocated.contains(&socket) {
        return None;
    }
    let start = allocated
        .iter()
        .copied()
        .find(|n| tree.node(*n).is_some_and(|x| x.kind == Kind::ClassStart))?;
    let mut parent: BTreeMap<u32, u32> = BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([socket]);
    while let Some(at) = queue.pop_front() {
        let node = tree.node(at)?;
        for &other in &node.neighbours {
            if other == start {
                let mut path = BTreeSet::from([socket, start]);
                let mut cur = Some(at);
                while let Some(c) = cur {
                    path.insert(c);
                    cur = parent.get(&c).copied();
                }
                return Some(path);
            }
            let Some(o) = tree.node(other) else { continue };
            if allocated.contains(&other)
                && node.kind != Kind::Mastery
                && !matches!(o.kind, Kind::ClassStart | Kind::AscendancyStart)
                && other != socket
                && !parent.contains_key(&other)
            {
                parent.insert(other, at);
                queue.push_back(other);
            }
        }
    }
    None
}

/// A small passive of a type other than Glorious Vanity (module docs).
fn small(t: &Timeless, jewel: u32, attribute: bool) -> Conquest {
    let size = if attribute {
        SMALL_ATTRIBUTE
    } else {
        SMALL_NORMAL
    };
    let replaced = t
        .tree_versions
        .iter()
        .find(|v| v.row == jewel)
        .is_some_and(|v| {
            if attribute {
                v.small_attribute_replaced
            } else {
                v.small_normal_replaced
            }
        });
    let stats = if replaced {
        let rows: Vec<_> = t
            .passives
            .iter()
            .filter(|p| p.tree_version == jewel && p.passive_types.contains(&size))
            .collect();
        let [row] = rows.as_slice() else {
            return reason("timeless-jewel");
        };
        amounts(&row.stats, &[])
    } else {
        let rows: Vec<_> = t
            .additions
            .iter()
            .filter(|a| a.tree_version == jewel && a.passive_types.contains(&size))
            .collect();
        let [row] = rows.as_slice() else {
            return reason("timeless-jewel");
        };
        amounts(&row.stats, &[])
    };
    Conquest {
        replaced,
        stats,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_data::bundle::timeless::{
        AltAddition, AltPassive, AltStat, AltTreeVersion, LutJewel, LutNode, LutSource,
    };

    fn stat(s: &str, v: i64) -> AltStat {
        AltStat {
            stat: s.to_owned(),
            min: v,
            max: v,
        }
    }

    fn tables() -> Timeless {
        let version = |row, a, n| AltTreeVersion {
            row,
            id: String::new(),
            small_attribute_replaced: a,
            small_normal_replaced: n,
        };
        let passive = |row, tv, types: Vec<u32>, stats| AltPassive {
            row,
            id: String::new(),
            name: String::new(),
            tree_version: tv,
            passive_types: types,
            stats,
            conqueror_index: 0,
            conqueror_version: 0,
        };
        let addition = |row, tv, types: Vec<u32>, stats| AltAddition {
            row,
            id: String::new(),
            tree_version: tv,
            passive_types: types,
            stats,
        };
        Timeless {
            tree_versions: vec![
                version(1, true, true),
                version(2, false, false),
                version(4, true, false),
            ],
            passives: vec![
                passive(MIGHT_ROW, 1, vec![3], Vec::new()),
                passive(LEGACY_ROW, 1, vec![3], Vec::new()),
                passive(90, 4, vec![1], vec![stat("base_devotion", 10)]),
            ],
            additions: vec![
                addition(5, 1, vec![3], vec![stat("fire_damage_+%", 7)]),
                addition(30, 1, vec![3], vec![stat("cold_damage_+%", 7)]),
                addition(38, 2, vec![1], vec![stat("base_strength", 2)]),
                addition(39, 2, vec![2], vec![stat("base_strength", 4)]),
                addition(92, 4, vec![2], vec![stat("base_devotion", 5)]),
            ],
        }
    }

    /// The small passive rule: the type's flags pick its one replacement or
    /// addition row by size (Lethal Pride adds 2 or 4 Strength, Militant
    /// Faith replaces attributes with the devotion node and adds 5 Devotion
    /// elsewhere).
    #[test]
    fn small_passives_should_follow_the_rows() {
        let t = tables();
        assert_eq!(
            small(&t, 2, true),
            Conquest {
                replaced: false,
                stats: vec![StatAmount {
                    stat: "base_strength".to_owned(),
                    value: 2
                }],
                reason: None
            }
        );
        assert_eq!(small(&t, 2, false).stats[0].value, 4);
        let devotion = small(&t, 4, true);
        assert!(devotion.replaced);
        assert_eq!(devotion.stats[0].value, 10);
        assert_eq!(small(&t, 4, false).stats[0].value, 5);
        assert!(!small(&t, 4, false).replaced);
    }

    /// Glorious Vanity's additions: summed per addition, on Might of the
    /// Vaal when at least half the IDs are up to 21, else Legacy.
    #[test]
    fn glorious_vanity_additions_should_pick_might_or_legacy() {
        let t = tables();
        let luts = TimelessLuts {
            source: LutSource {
                pob_version: String::new(),
                licence: String::new(),
            },
            additions: 337,
            node_count: 1,
            notable_count: 1,
            nodes: vec![LutNode {
                node: 7,
                index: 0,
                size: 8,
            }],
            local_to_global: Vec::new(),
            jewels: vec![LutJewel {
                jewel: 1,
                name: String::new(),
                file: String::new(),
                seed_min: 100,
                seed_max: 100,
                seed_divisor: 1,
                sha256: String::new(),
            }],
        };
        // One seed: 8 bytes, IDs 5, 5, 30, 30 with rolls 7, 8, 9, 10.
        let bytes = [8_u8, 5, 5, 30, 30, 7, 8, 9, 10];
        let data = TimelessData {
            timeless: &t,
            luts: &luts,
            results: [(1, &bytes[..])].into_iter().collect(),
        };
        let c = from_results(&data, 1, 100, 7);
        assert!(c.replaced);
        let values: Vec<(&str, i64)> = c.stats.iter().map(|s| (s.stat.as_str(), s.value)).collect();
        assert_eq!(values, [("fire_damage_+%", 15), ("cold_damage_+%", 19)]);
    }

    /// An Abyss change: its parts in order, a replacement's stats taking the
    /// part's rolls as the stats' raw values (a per-minute regeneration
    /// roll stays per minute; PoB rounds it to the second's tenth, DIV-0294
    /// and DIV-0295), an addition added, a replacement wiping what came
    /// before it.
    #[test]
    fn abyss_parts_should_apply_in_order_with_their_raw_rolls() {
        let mut t = tables();
        t.passives.push(AltPassive {
            row: 500,
            id: String::new(),
            name: String::new(),
            tree_version: 8,
            passive_types: vec![2],
            stats: vec![AltStat {
                stat: "base_mana_regeneration_rate_per_minute".to_owned(),
                min: 30,
                max: 60,
            }],
            conqueror_index: 0,
            conqueror_version: 0,
        });
        let luts = TimelessLuts {
            source: LutSource {
                pob_version: String::new(),
                licence: String::new(),
            },
            additions: 337,
            node_count: 0,
            notable_count: 0,
            nodes: Vec::new(),
            local_to_global: Vec::new(),
            jewels: Vec::new(),
        };
        let data = TimelessData {
            timeless: &t,
            luts: &luts,
            results: BTreeMap::new(),
        };
        let part = |kind, global, rolls: &[i64]| AbyssPart {
            kind,
            global,
            rolls: rolls.to_vec(),
        };
        let c = apply(
            &data,
            &[
                part(ADDS, 5, &[-3]),
                part(REPLACES, 837, &[51]),
                part(ADDS, 30, &[]),
            ],
        );
        assert!(c.replaced);
        let values: Vec<(&str, i64)> = c.stats.iter().map(|s| (s.stat.as_str(), s.value)).collect();
        assert_eq!(
            values,
            [
                ("base_mana_regeneration_rate_per_minute", 51),
                ("cold_damage_+%", 7)
            ]
        );
        let added = apply(&data, &[part(ADDS, 5, &[-15])]);
        assert!(!added.replaced);
        assert_eq!(added.stats[0].value, -15);
        assert!(apply(&data, &[part(3, 5, &[])]).reason.is_some());
    }
}

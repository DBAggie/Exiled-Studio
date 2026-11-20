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
//!   else it gains the type's one addition row. This is PoB's hand-written
//!   rule for each conqueror (+4 or +2 Strength or Dexterity, +5 Devotion
//!   or the devotion node, the blank node, 2% or 1% increased Ward), read
//!   from the game's rows.
//!
//! The results files are large, so the host reads only the types a build
//! needs ([`needed`]) and hands their bytes over ([`TimelessData`]). A node
//! whose result is not known is a reason, never its old stats: no data
//! (`timeless-results`), a jewel whose lines do not read (`timeless-jewel`),
//! an Abyss jewel (`abyss-jewel`, not ported yet).

use std::collections::{BTreeMap, BTreeSet};

use loom_calc::input::{Conquest, LineValues};
use loom_domain::ItemId;

/// PoB's rolls of the items' lines, by item and line (the host's
/// `CalcOptions::item_lines`).
pub type Rolls = BTreeMap<ItemId, BTreeMap<usize, LineValues>>;
use loom_data::bundle::passives::StatAmount;
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

/// A socketed Timeless jewel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socketed {
    /// Its socket.
    pub socket: u32,
    /// The nodes in its radius.
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
        let Some(ring) = label.and_then(|l| tree.radius_named(l)) else {
            continue;
        };
        let get = |s: &str| stats.get(s).and_then(|v| u32::try_from(*v).ok());
        let jewel = match (get(VERSION), get(SEED), get(KEYSTONE)) {
            (Some(t), Some(seed), Some(c)) => Some((t, seed, c, get(REVISION).unwrap_or(0))),
            _ => None,
        };
        out.push(Socketed {
            jewel_type: get(VERSION),
            socket: socket.node.0,
            ring: loom_tree::poe1::radius::in_ring(tree, socket.node.0, ring, false),
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
        .filter(|t| *t < FIRST_ABYSS)
        .collect()
}

fn amounts(stats: &[loom_data::bundle::timeless::AltStat], rolls: &[u32]) -> Vec<StatAmount> {
    stats
        .iter()
        .enumerate()
        .map(|(i, s)| StatAmount {
            stat: s.stat.clone(),
            value: rolls.get(i).map_or(s.min, |r| i64::from(*r)),
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
                    stats: amounts(&p.stats, &rolls),
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
                    &roll.map(|r| vec![r]).unwrap_or_default(),
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
) -> BTreeMap<u32, Conquest> {
    let mut out = BTreeMap::new();
    for s in socketed(build, spec, tree, rolls) {
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
                (Some((t, ..)), _) if t >= FIRST_ABYSS => reason("abyss-jewel"),
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
                    _ => small(d.timeless, t, ATTRIBUTES.contains(&n.name.as_str())),
                },
            };
            // Two jewels reaching one node: PoB keeps the last socket's; the
            // engine keeps the first and the order is the spec's.
            out.entry(node).or_insert(conquest);
        }
    }
    out
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
}

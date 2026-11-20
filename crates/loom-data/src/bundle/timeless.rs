//! Timeless jewels (Phase 6 P6-10a; plan
//! `dev/active/phase-6-mechanics/p6-10a-timeless-plan.md`, Q-P6-1).
//!
//! Three parts, by source:
//! - `poe1/timeless.json` ([`Timeless`], our extraction): the alternate
//!   passive skills, their additions, and the alternate tree versions
//!   (`AlternatePassiveSkills`, `AlternatePassiveAdditions`,
//!   `AlternateTreeVersions`), the rows the results index.
//! - `poe1/timeless_luts.json` ([`TimelessLuts`], from PoB's MIT data): how
//!   to read the results. The node index mapping and the local-to-global
//!   IDs (`Data/TimelessJewelData/NodeIndexMapping.lua`), and each jewel
//!   type's seed range and results file.
//! - `poe1/timeless/<Type>.bin`: each jewel type's results, PoB's lookup
//!   table (`Data/TimelessJewelData/<Type>.zip`) inflated, byte for byte.
//!
//! **Loading.** The results are large (466 MB for the 11 types), so
//! [`super::Bundle::load`] does not read them: the app reads one type's file
//! only when a build socketes that jewel, and hands the bytes to the
//! calculation (plan decision T2). The JSON parts load with the section.
//!
//! **Reading.** [`read`] is PoB's `readLUT`
//! (`Modules/DataLegionLookUpTableHelper.lua`, v2.67.2) for the six
//! non-Abyss types:
//! - Glorious Vanity: per node and seed, a length, then the bytes. Two or
//!   three bytes are a replacement (its local ID, then its rolls); six or
//!   eight are additions (their local IDs, then their rolls).
//! - The others: one byte per notable and seed, a local ID; at or above
//!   [`TimelessLuts::additions`] a replacement, else an addition.
//! - Elegant Hubris's seed is divided by 20 first.
//!
//! A local ID becomes a global one through the jewel's local-to-global
//! table. A global ID below `additions` is an addition's row; at or above,
//! a replacement's row is the ID minus `additions`. The Abyss jewels have
//! their own reader in PoB (`DataAbyssJewelLookUpTableHelper.lua`), not
//! ported yet: their files ship, and [`read`] returns `None` for them.

use serde::{Deserialize, Serialize};

/// The extracted tables.
pub const TIMELESS: &str = "poe1/timeless.json";
/// How to read the results (PoB's mapping).
pub const TIMELESS_LUTS: &str = "poe1/timeless_luts.json";
/// The folder of the results files.
pub const TIMELESS_DIR: &str = "poe1/timeless/";

/// One stat of an alternate passive or addition, with its roll range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltStat {
    /// `Stats.Id`.
    pub stat: String,
    /// The lowest roll.
    pub min: i64,
    /// The highest roll.
    pub max: i64,
}

/// An alternate passive skill (`AlternatePassiveSkills`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltPassive {
    /// The table row (what a result's ID names, minus the additions'
    /// count).
    pub row: u32,
    /// `Id`.
    pub id: String,
    /// `Name`.
    pub name: String,
    /// `AlternateTreeVersionsKey`: the jewel type.
    pub tree_version: u32,
    /// `PassiveType`: the passive sizes it replaces (1 small attribute, 2
    /// small, 3 notable, 4 keystone).
    pub passive_types: Vec<u32>,
    /// Its stats, in the row's order (`StatsKeys` with `Stat1` to `Stat6`).
    pub stats: Vec<AltStat>,
    /// `ConquerorIndex` (keystones; 0: none).
    #[serde(default)]
    pub conqueror_index: u32,
    /// `ConquerorVersion` (keystones).
    #[serde(default)]
    pub conqueror_version: u32,
}

/// An alternate passive addition (`AlternatePassiveAdditions`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltAddition {
    /// The table row (what a result's ID names).
    pub row: u32,
    /// `Id`.
    pub id: String,
    /// `AlternateTreeVersionsKey`: the jewel type.
    pub tree_version: u32,
    /// `PassiveType`.
    pub passive_types: Vec<u32>,
    /// Its stats (`StatsKeys` with `Stat1` to `Stat3`).
    pub stats: Vec<AltStat>,
}

/// An alternate tree version (`AlternateTreeVersions`): a jewel type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltTreeVersion {
    /// The table row; a jewel's `tree_version` names it.
    pub row: u32,
    /// `Id` (`Vaal`, `Karui`, ...).
    pub id: String,
    /// Small attribute passives are replaced.
    pub small_attribute_replaced: bool,
    /// Small non-attribute passives are replaced.
    pub small_normal_replaced: bool,
}

/// `poe1/timeless.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timeless {
    /// The jewel types.
    pub tree_versions: Vec<AltTreeVersion>,
    /// The alternate passives, by row.
    pub passives: Vec<AltPassive>,
    /// The additions, by row.
    pub additions: Vec<AltAddition>,
}

impl Timeless {
    /// The replacement of a global result ID ([`TimelessLuts::additions`]
    /// and above).
    #[must_use]
    pub fn passive(&self, global: u32, additions: u32) -> Option<&AltPassive> {
        let row = global.checked_sub(additions)?;
        self.passives.iter().find(|p| p.row == row)
    }

    /// The addition of a global result ID (below the additions' count).
    #[must_use]
    pub fn addition(&self, global: u32) -> Option<&AltAddition> {
        self.additions.iter().find(|a| a.row == global)
    }
}

/// A node of the results (PoB's `nodeIDList` entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LutNode {
    /// The passive's graph ID.
    pub node: u32,
    /// Its index in the results.
    pub index: u32,
    /// Its bytes in Glorious Vanity's results.
    pub size: u32,
}

/// A local-to-global result ID of one jewel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LutId {
    /// The jewel type (PoB's `timelessJewelTypes` index).
    pub jewel: u32,
    /// The local ID in its results.
    pub local: u32,
    /// The global ID.
    pub global: u32,
}

/// A jewel type's results file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LutJewel {
    /// PoB's `timelessJewelTypes` index (1 Glorious Vanity ... 11 Abyss
    /// Zorath).
    pub jewel: u32,
    /// Its name (`Glorious Vanity`).
    pub name: String,
    /// The file (`poe1/timeless/GloriousVanity.bin`).
    pub file: String,
    /// The lowest seed, as the results count it (Elegant Hubris's divided).
    pub seed_min: u32,
    /// The highest seed, likewise.
    pub seed_max: u32,
    /// What a jewel's seed is divided by first (20 for Elegant Hubris).
    pub seed_divisor: u32,
    /// The file's SHA-256 (the inflated bytes, PoB's own).
    pub sha256: String,
}

impl LutJewel {
    /// The seeds' count.
    #[must_use]
    pub const fn seed_count(&self) -> u64 {
        (self.seed_max as u64).saturating_sub(self.seed_min as u64) + 1
    }
}

/// Where the results came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LutSource {
    /// PoB's version (`2.67.2`).
    pub pob_version: String,
    /// PoB's licence.
    pub licence: String,
}

/// `poe1/timeless_luts.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelessLuts {
    /// Where it came from.
    pub source: LutSource,
    /// The additions' count (PoB's `timelessJewelAdditions`).
    pub additions: u32,
    /// The nodes Glorious Vanity's results hold (PoB's `nodeIDList.size`).
    pub node_count: u32,
    /// The notables the others' results hold (`nodeIDList.sizeNotable`):
    /// rows `0 .. notable_count`.
    pub notable_count: u32,
    /// The nodes, by index.
    pub nodes: Vec<LutNode>,
    /// The local-to-global IDs (a pair absent: the same ID).
    pub local_to_global: Vec<LutId>,
    /// The jewel types.
    pub jewels: Vec<LutJewel>,
}

/// Glorious Vanity (PoB's type 1).
pub const GLORIOUS_VANITY: u32 = 1;
/// Elegant Hubris (type 5).
pub const ELEGANT_HUBRIS: u32 = 5;
/// The first Abyss jewel (type 7); every later type is one.
pub const FIRST_ABYSS: u32 = 7;

impl TimelessLuts {
    /// A jewel type.
    #[must_use]
    pub fn jewel(&self, jewel: u32) -> Option<&LutJewel> {
        self.jewels.iter().find(|j| j.jewel == jewel)
    }

    /// The global ID of a local one.
    #[must_use]
    pub fn global(&self, jewel: u32, local: u32) -> u32 {
        self.local_to_global
            .iter()
            .find(|l| l.jewel == jewel && l.local == local)
            .map_or(local, |l| l.global)
    }

    /// The index of a node.
    #[must_use]
    pub fn index(&self, node: u32) -> Option<u32> {
        self.nodes.iter().find(|n| n.node == node).map(|n| n.index)
    }

    /// The length a jewel type's results file must have: Glorious
    /// Vanity's, every node's lengths then their bytes; the others', one
    /// byte per notable and seed. `None` for an Abyss jewel (another
    /// layout).
    #[must_use]
    pub fn expected_len(&self, jewel: &LutJewel) -> Option<u64> {
        let seeds = jewel.seed_count();
        match jewel.jewel {
            GLORIOUS_VANITY => {
                let data: u64 = self.nodes.iter().map(|n| u64::from(n.size)).sum();
                Some(u64::from(self.node_count) * seeds + data)
            }
            j if j >= FIRST_ABYSS => None,
            _ => Some(u64::from(self.notable_count) * seeds),
        }
    }
}

/// One result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LutResult {
    /// The node is replaced: the replacement's global ID and its rolls
    /// (Glorious Vanity's; empty for the others, whose rows have fixed
    /// values).
    Replaced {
        /// The global ID.
        global: u32,
        /// The rolls, in the replacement's stat order.
        rolls: Vec<u32>,
    },
    /// Additions: each global ID with its roll (Glorious Vanity's Might or
    /// Legacy of the Vaal; one addition and no roll for the others).
    Added {
        /// `(global ID, roll)` pairs, in the results' order.
        additions: Vec<(u32, Option<u32>)>,
    },
}

/// A jewel type's result for a seed and a node (module docs: PoB's
/// `readLUT`); `None` for a node the results do not hold, a seed out of
/// range, an Abyss jewel, or bytes shorter than the layout.
#[must_use]
pub fn read(
    luts: &TimelessLuts,
    jewel: &LutJewel,
    bytes: &[u8],
    seed: u32,
    node: u32,
) -> Option<LutResult> {
    if jewel.jewel >= FIRST_ABYSS {
        return None;
    }
    let seed = seed / jewel.seed_divisor.max(1);
    if seed < jewel.seed_min || seed > jewel.seed_max {
        return None;
    }
    let seed_off = usize::try_from(seed - jewel.seed_min).ok()?;
    let seeds = usize::try_from(jewel.seed_count()).ok()?;
    let index = usize::try_from(luts.index(node)?).ok()?;
    let id = |local: u8| luts.global(jewel.jewel, u32::from(local));
    if jewel.jewel == GLORIOUS_VANITY {
        let node_count = usize::try_from(luts.node_count).ok()?;
        let sizes_at = index * seeds;
        let n = usize::from(*bytes.get(sizes_at + seed_off)?);
        // The node's data: after every node's lengths, after the earlier
        // nodes' data, after this node's earlier seeds.
        let mut start = node_count * seeds;
        for other in &luts.nodes {
            if (other.index as usize) < index {
                start += other.size as usize;
            }
        }
        start += bytes
            .get(sizes_at..sizes_at + seed_off)?
            .iter()
            .map(|b| usize::from(*b))
            .sum::<usize>();
        let b = bytes.get(start..start + n)?;
        return match n {
            2 | 3 => {
                let (first, rolls) = b.split_first()?;
                Some(LutResult::Replaced {
                    global: id(*first),
                    rolls: rolls.iter().map(|r| u32::from(*r)).collect(),
                })
            }
            6 | 8 => {
                let (ids, rolls) = b.split_at(n / 2);
                Some(LutResult::Added {
                    additions: ids
                        .iter()
                        .zip(rolls)
                        .map(|(a, r)| (id(*a), Some(u32::from(*r))))
                        .collect(),
                })
            }
            _ => None,
        };
    }
    if index >= usize::try_from(luts.notable_count).ok()? {
        return None;
    }
    let local = *bytes.get(index * seeds + seed_off)?;
    let global = id(local);
    Some(if global >= luts.additions {
        LutResult::Replaced {
            global,
            rolls: Vec::new(),
        }
    } else {
        LutResult::Added {
            additions: vec![(global, None)],
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luts() -> TimelessLuts {
        TimelessLuts {
            source: LutSource {
                pob_version: "2.67.2".to_owned(),
                licence: "MIT".to_owned(),
            },
            additions: 10,
            node_count: 3,
            notable_count: 2,
            nodes: vec![
                LutNode {
                    node: 100,
                    index: 0,
                    size: 5,
                },
                LutNode {
                    node: 200,
                    index: 1,
                    size: 8,
                },
                LutNode {
                    node: 300,
                    index: 2,
                    size: 4,
                },
            ],
            local_to_global: vec![LutId {
                jewel: 1,
                local: 1,
                global: 11,
            }],
            jewels: Vec::new(),
        }
    }

    fn jewel(jewel: u32, seed_min: u32, seed_max: u32, divisor: u32) -> LutJewel {
        LutJewel {
            jewel,
            name: String::new(),
            file: String::new(),
            seed_min,
            seed_max,
            seed_divisor: divisor,
            sha256: String::new(),
        }
    }

    /// Glorious Vanity's layout: the lengths of every node's seeds, then
    /// each node's data by index; a replacement's ID through the local
    /// table, its rolls after it; additions then their rolls.
    #[test]
    fn glorious_vanity_should_read_as_pobs_read_lut() {
        let l = luts();
        let j = jewel(1, 100, 101, 1);
        // Lengths: node 0 seeds (2, 3), node 1 (8, 0), node 2 (2, 2).
        let mut bytes = vec![2, 3, 8, 0, 2, 2];
        // Node 0: seed 100 [1, 7], seed 101 [12, 4, 5].
        bytes.extend([1, 7, 12, 4, 5]);
        // Node 1: seed 100 additions [3, 3, 4, 5] rolls [1, 2, 3, 4].
        bytes.extend([3, 3, 4, 5, 1, 2, 3, 4]);
        // Node 2: seed 100 [13, 9], seed 101 [14, 9].
        bytes.extend([13, 9, 14, 9]);
        assert_eq!(
            l.expected_len(&LutJewel { ..j.clone() }),
            Some(bytes.len() as u64)
        );
        assert_eq!(
            read(&l, &j, &bytes, 100, 100),
            Some(LutResult::Replaced {
                global: 11,
                rolls: vec![7]
            })
        );
        assert_eq!(
            read(&l, &j, &bytes, 101, 100),
            Some(LutResult::Replaced {
                global: 12,
                rolls: vec![4, 5]
            })
        );
        assert_eq!(
            read(&l, &j, &bytes, 100, 200),
            Some(LutResult::Added {
                additions: vec![(3, Some(1)), (3, Some(2)), (4, Some(3)), (5, Some(4))]
            })
        );
        assert_eq!(read(&l, &j, &bytes, 101, 200), None);
        assert_eq!(
            read(&l, &j, &bytes, 101, 300),
            Some(LutResult::Replaced {
                global: 14,
                rolls: vec![9]
            })
        );
        assert_eq!(read(&l, &j, &bytes, 102, 100), None);
        assert_eq!(read(&l, &j, &bytes, 100, 999), None);
    }

    /// The other types: one byte per notable and seed; at or above the
    /// additions' count a replacement, else an addition; Elegant Hubris's
    /// seed divided by 20; a node past the notables none.
    #[test]
    fn the_others_should_read_one_byte_per_notable() {
        let l = luts();
        let j = jewel(5, 100, 102, 20);
        // Node 0 seeds 100..=102, node 1 likewise.
        let bytes = vec![12, 3, 10, 9, 15, 0];
        assert_eq!(l.expected_len(&j), Some(6));
        assert_eq!(
            read(&l, &j, &bytes, 2000, 100),
            Some(LutResult::Replaced {
                global: 12,
                rolls: Vec::new()
            })
        );
        assert_eq!(
            read(&l, &j, &bytes, 2020, 100),
            Some(LutResult::Added {
                additions: vec![(3, None)]
            })
        );
        assert_eq!(
            read(&l, &j, &bytes, 2040, 200),
            Some(LutResult::Added {
                additions: vec![(0, None)]
            })
        );
        assert_eq!(read(&l, &j, &bytes, 2000, 300), None);
        let abyss = jewel(FIRST_ABYSS, 100, 102, 1);
        assert_eq!(l.expected_len(&abyss), None);
        assert_eq!(read(&l, &abyss, &bytes, 100, 100), None);
    }
}

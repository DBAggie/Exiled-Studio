//! The Abyss jewels' results (types 7 to 11): PoB's
//! `Modules/DataAbyssJewelLookUpTableHelper.lua` (v2.67.2), ported.
//!
//! A file starts with a header: four format letters, the format version
//! (1), the jewel type, and the first seed, last seed, and the step between
//! seeds (little-endian `u16`s). Two formats follow:
//! - **`ABYS`** (Tecrod, Ulaman, Kurgal, Amanamu): the sockets' count, a
//!   byte PoB keeps but does not read, the sockets' IDs, then each socket's
//!   block, one record per seed: the nodes the jewel changes from that
//!   socket, each a node ID and a change.
//! - **`ABYN`** (Zorath): the nodes' count and IDs, then each node's block,
//!   one change per seed; then `ASCS`, the ascendancies' count, and each
//!   ascendancy's name and block, one record per seed: the ascendancy
//!   nodes the seed picks.
//!
//! A change is its parts' count, then each part: its kind (1 replaces the
//! node, 2 adds to it), its local ID, its rolls' count, and its rolls
//! (signed `u16`s, the stats' values). The records vary in length, so a
//! block is found by skipping every record before it, as PoB does.

use std::collections::{BTreeMap, BTreeSet};

use super::{LutJewel, TimelessLuts};

/// A part replaces the node (PoB's component type 1).
pub const REPLACES: u8 = 1;
/// A part adds to the node (type 2).
pub const ADDS: u8 = 2;

/// One part of an Abyss jewel's change to a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbyssPart {
    /// [`REPLACES`] or [`ADDS`] (another kind PoB does not handle either).
    pub kind: u8,
    /// The replacement's or addition's global ID.
    pub global: u32,
    /// Its rolls, in its stats' order.
    pub rolls: Vec<i64>,
}

/// A byte reader over a results file; every read is checked.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn u8(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(b)
    }

    fn u16(&mut self) -> Option<u16> {
        let lo = self.u8()?;
        let hi = self.u8()?;
        Some(u16::from_le_bytes([lo, hi]))
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let b = self.bytes.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(b)
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        let at = self.at.checked_add(n)?;
        (at <= self.bytes.len()).then(|| self.at = at)
    }

    /// Skips one change.
    fn skip_change(&mut self) -> Option<()> {
        let parts = self.u8()?;
        for _ in 0..parts {
            self.skip(2)?;
            let rolls = usize::from(self.u8()?);
            self.skip(rolls * 2)?;
        }
        Some(())
    }

    /// Skips one socket record.
    fn skip_socket_record(&mut self) -> Option<()> {
        let nodes = self.u8()?;
        for _ in 0..nodes {
            self.skip(2)?;
            self.skip_change()?;
        }
        Some(())
    }

    /// Skips one ascendancy record.
    fn skip_ascendancy_record(&mut self) -> Option<()> {
        let nodes = usize::from(self.u8()?);
        self.skip(nodes * 2)
    }

    /// Reads one change, its local IDs made global.
    fn change(&mut self, luts: &TimelessLuts, jewel: u32) -> Option<Vec<AbyssPart>> {
        let parts = self.u8()?;
        let mut out = Vec::with_capacity(usize::from(parts));
        for _ in 0..parts {
            let kind = self.u8()?;
            let local = self.u8()?;
            let count = self.u8()?;
            let mut rolls = Vec::with_capacity(usize::from(count));
            for _ in 0..count {
                // PoB: `roll >= 32768 and roll - 65536 or roll`.
                rolls.push(i64::from(i16::from_le_bytes(self.u16()?.to_le_bytes())));
            }
            out.push(AbyssPart {
                kind,
                global: luts.global(jewel, u32::from(local)),
                rolls,
            });
        }
        Some(out)
    }
}

/// The header's format and the seed's index, after checking the header
/// (PoB's `parseAbyssJewel` and `getSeedIndex`); `None` for a bad header
/// or a seed the file does not hold.
fn header<'a>(
    jewel: &LutJewel,
    bytes: &'a [u8],
    seed: u32,
) -> Option<(&'a [u8], usize, usize, Cursor<'a>)> {
    let mut c = Cursor { bytes, at: 0 };
    let format = c.take(4)?;
    let version = c.u8()?;
    let stored = c.u8()?;
    let first = u32::from(c.u16()?);
    let last = u32::from(c.u16()?);
    let step = u32::from(c.u16()?);
    if version != 1 || u32::from(stored) != jewel.jewel || step == 0 || last < first {
        return None;
    }
    let off = seed.checked_sub(first)?;
    if seed > last || off % step != 0 {
        return None;
    }
    let index = usize::try_from(off / step).ok()?;
    let seeds = usize::try_from((last - first) / step + 1).ok()?;
    Some((format, index, seeds, c))
}

/// The changes an Abyss jewel of type 7 to 10 makes from `socket` with
/// `seed` (PoB's `readAbyssJewelLUT`), by node; empty for a socket or seed
/// the file does not hold, or bytes that do not read.
#[must_use]
pub fn read_socket(
    luts: &TimelessLuts,
    jewel: &LutJewel,
    bytes: &[u8],
    seed: u32,
    socket: u32,
) -> BTreeMap<u32, Vec<AbyssPart>> {
    read_socket_inner(luts, jewel, bytes, seed, socket).unwrap_or_default()
}

fn read_socket_inner(
    luts: &TimelessLuts,
    jewel: &LutJewel,
    bytes: &[u8],
    seed: u32,
    socket: u32,
) -> Option<BTreeMap<u32, Vec<AbyssPart>>> {
    let (format, index, seeds, mut c) = header(jewel, bytes, seed)?;
    if format != b"ABYS" {
        return None;
    }
    let sockets = c.u8()?;
    c.skip(1)?;
    let mut ids = Vec::with_capacity(usize::from(sockets));
    for _ in 0..sockets {
        ids.push(u32::from(c.u16()?));
    }
    let at = ids.iter().position(|s| *s == socket)?;
    for _ in 0..at * seeds + index {
        c.skip_socket_record()?;
    }
    let nodes = c.u8()?;
    let mut out = BTreeMap::new();
    for _ in 0..nodes {
        let node = u32::from(c.u16()?);
        out.insert(node, c.change(luts, jewel.jewel)?);
    }
    Some(out)
}

/// The changes Zorath (type 11) makes with `seed` (PoB's
/// `readAbyssJewelLUT` with a path): each node of `path` the file holds a
/// change for, and in every ascendancy the nodes the seed picks, with their
/// changes. Empty for a seed the file does not hold, or bytes that do not
/// read.
#[must_use]
pub fn read_path(
    luts: &TimelessLuts,
    jewel: &LutJewel,
    bytes: &[u8],
    seed: u32,
    path: &BTreeSet<u32>,
) -> BTreeMap<u32, Vec<AbyssPart>> {
    read_path_inner(luts, jewel, bytes, seed, path).unwrap_or_default()
}

fn read_path_inner(
    luts: &TimelessLuts,
    jewel: &LutJewel,
    bytes: &[u8],
    seed: u32,
    path: &BTreeSet<u32>,
) -> Option<BTreeMap<u32, Vec<AbyssPart>>> {
    let (format, index, seeds, mut c) = header(jewel, bytes, seed)?;
    if format != b"ABYN" {
        return None;
    }
    let count = c.u16()?;
    let mut ids = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        ids.push(u32::from(c.u16()?));
    }
    // Each node's change for the seed, kept by node; the blocks are walked
    // once, to reach the ascendancy section after them.
    let mut changes: BTreeMap<u32, Vec<AbyssPart>> = BTreeMap::new();
    for id in &ids {
        for s in 0..seeds {
            if s == index {
                let change = c.change(luts, jewel.jewel)?;
                changes.insert(*id, change);
            } else {
                c.skip_change()?;
            }
        }
    }
    if c.take(4)? != b"ASCS" {
        return None;
    }
    let mut out: BTreeMap<u32, Vec<AbyssPart>> = path
        .iter()
        .filter_map(|n| Some((*n, changes.get(n).filter(|ch| !ch.is_empty())?.clone())))
        .collect();
    let ascendancies = c.u16()?;
    for _ in 0..ascendancies {
        let name = usize::from(c.u8()?);
        c.skip(name)?;
        for _ in 0..index {
            c.skip_ascendancy_record()?;
        }
        let picked = c.u8()?;
        for _ in 0..picked {
            let node = u32::from(c.u16()?);
            out.insert(node, changes.get(&node).cloned().unwrap_or_default());
        }
        for _ in index + 1..seeds {
            c.skip_ascendancy_record()?;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::timeless::{LutId, LutSource};

    fn u16le(v: i32) -> [u8; 2] {
        let v = if v < 0 { v + 65536 } else { v };
        u16::try_from(v).unwrap().to_le_bytes()
    }

    fn change(parts: &[(u8, u8, &[i32])]) -> Vec<u8> {
        let mut out = vec![u8::try_from(parts.len()).unwrap()];
        for (kind, id, rolls) in parts {
            out.extend([*kind, *id, u8::try_from(rolls.len()).unwrap()]);
            for r in *rolls {
                out.extend(u16le(*r));
            }
        }
        out
    }

    fn head(format: &[u8], jewel: u8, seed: i32) -> Vec<u8> {
        let mut out = format.to_vec();
        out.extend([1, jewel]);
        out.extend(u16le(seed));
        out.extend(u16le(seed));
        out.extend(u16le(1));
        out
    }

    fn luts(jewel: u32, map: &[(u32, u32)]) -> (TimelessLuts, LutJewel) {
        let j = LutJewel {
            jewel,
            name: String::new(),
            file: String::new(),
            seed_min: 100,
            seed_max: 8000,
            seed_divisor: 1,
            sha256: String::new(),
        };
        let l = TimelessLuts {
            source: LutSource {
                pob_version: String::new(),
                licence: String::new(),
            },
            additions: 94,
            node_count: 0,
            notable_count: 0,
            nodes: Vec::new(),
            local_to_global: map
                .iter()
                .map(|(local, global)| LutId {
                    jewel,
                    local: *local,
                    global: *global,
                })
                .collect(),
            jewels: vec![j.clone()],
        };
        (l, j)
    }

    /// PoB's own example (`spec/System/TestAbyssTimelessJewel_spec.lua`,
    /// "reads Kurgal seed 533 at socket 61419").
    #[test]
    fn kurgal_should_read_as_pobs_example() {
        let nodes: [(i32, Vec<u8>); 4] = [
            (4367, change(&[(1, 24, &[6])])),
            (15117, change(&[(1, 19, &[8])])),
            (20528, change(&[(2, 207, &[-15])])),
            (21958, change(&[(2, 211, &[12, 18])])),
        ];
        let mut bytes = head(b"ABYS", 9, 533);
        bytes.extend([1, 60]);
        bytes.extend(u16le(61419));
        bytes.push(4);
        for (id, ch) in &nodes {
            bytes.extend(u16le(*id));
            bytes.extend(ch);
        }
        let (l, j) = luts(9, &[(19, 622), (24, 627)]);
        let got = read_socket(&l, &j, &bytes, 533, 61419);
        let part = |kind, global, rolls: &[i64]| AbyssPart {
            kind,
            global,
            rolls: rolls.to_vec(),
        };
        assert_eq!(got.get(&15117), Some(&vec![part(1, 622, &[8])]));
        assert_eq!(got.get(&4367), Some(&vec![part(1, 627, &[6])]));
        assert_eq!(got.get(&20528), Some(&vec![part(2, 207, &[-15])]));
        assert_eq!(got.get(&21958), Some(&vec![part(2, 211, &[12, 18])]));
        assert!(!got.contains_key(&53279));
        assert!(read_socket(&l, &j, &bytes, 534, 61419).is_empty());
        assert!(read_socket(&l, &j, &bytes, 533, 2311).is_empty());
    }

    /// PoB's Zorath example ("zorathExampleData"): the path's nodes and
    /// each ascendancy's pick.
    #[test]
    fn zorath_should_read_the_path_and_the_ascendancy_picks() {
        let nodes: [(i32, Vec<u8>); 3] = [
            (23027, change(&[(1, 90, &[8])])),
            (50692, change(&[(1, 77, &[1])])),
            (53884, change(&[(1, 78, &[1])])),
        ];
        let mut bytes = head(b"ABYN", 11, 6564);
        bytes.extend(u16le(3));
        for (id, _) in &nodes {
            bytes.extend(u16le(*id));
        }
        for (_, ch) in &nodes {
            bytes.extend(ch);
        }
        bytes.extend(b"ASCS");
        bytes.extend(u16le(2));
        for (name, node) in [(&b"Inquisitor"[..], 53884), (&b"Chieftain"[..], 50692)] {
            bytes.push(u8::try_from(name.len()).unwrap());
            bytes.extend(name);
            bytes.push(1);
            bytes.extend(u16le(node));
        }
        let (l, j) = luts(11, &[]);
        let path: BTreeSet<u32> = [23027, 999].into();
        let got = read_path(&l, &j, &bytes, 6564, &path);
        assert_eq!(
            got.keys().copied().collect::<Vec<_>>(),
            vec![23027, 50692, 53884]
        );
        assert_eq!(
            got.get(&23027).and_then(|c| c.first()).map(|p| p.global),
            Some(90)
        );
    }
}

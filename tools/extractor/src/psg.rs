//! Passive skill graph files (`*.psg`, Spike A, PR A6b): the layout of a
//! passive tree (character tree, atlas tree, league and event trees). A
//! `PassiveSkills` row is on a tree only if its `PassiveSkillGraphId` is a
//! node of that tree's graph; `PassiveSkillTrees.PassiveSkillGraph` names the
//! graph of each tree.
//!
//! Layout (little-endian; confirmed on all 7 PoE1 and 5 PoE2 graphs of the
//! installs, which parse to their last byte; report section 16):
//!
//! ```text
//! u8   version                 3
//! u8   kind                    0 passive tree, 1 atlas, 2 league (observed)
//! u8   orbit count n
//! u8   slots[n]                nodes per orbit ring (PoE1 1,6,16,16,40,72,72)
//! u32  root count
//!      roots                   u32 (PoE1) or u64 (PoE2) node IDs
//! u32  group count, then per group:
//!   f32 x, f32 y
//!   u32 flags, u32 unknown1, u8 unknown2
//!   u32 node count, then per node:
//!     u32 id                   PassiveSkills.PassiveSkillGraphId
//!     u32 orbit                < n
//!     u32 orbit index          < slots[orbit]
//!     u32 connection count, then per connection:
//!       u32 id                 another node of the graph
//!       i32 orbit              PoE2 only (signed arc; i32::MAX observed)
//! ```
//!
//! PoB's `src/Export/psg.lua` (PoE1) reads an older layout (7-byte header,
//! bare node fields); its PoE2 `src/Export/Scripts/passivetree.lua` reads this
//! one. Parsing is pure: bytes in, typed errors out, no allocation sized by
//! an untrusted count.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::Short;
use crate::extract::{ExtractError, TableError};
use crate::install::hex;
use crate::reader::{Reader, widen};
use crate::schema::Game;

/// The only version read.
pub const VERSION: u8 = 3;

/// Folder of the graph files in the extract output.
pub const GRAPHS_DIR: &str = "passive_skill_graphs";

/// File extension of a graph in the index.
pub const EXTENSION: &str = ".psg";

/// Whether `path` (as listed in the index) is a passive skill graph.
#[must_use]
pub fn is_graph(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(EXTENSION)
}

/// Output path of the graph at `path` (lowercase index path plus `.json`).
#[must_use]
pub fn output_path(path: &str) -> String {
    format!("{GRAPHS_DIR}/{}.json", path.to_ascii_lowercase())
}

/// Errors from reading a graph. Each variant has a stable [`code`](Self::code).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PsgError {
    /// The file ends before a field.
    #[error(
        "graph truncated: {} at offset {} needs {} bytes, {} left",
        .0.field, .0.offset, .0.needed, .0.available
    )]
    Truncated(Short),
    /// A version other than [`VERSION`].
    #[error("graph version {0}, only {VERSION} is read")]
    Version(u8),
    /// A group position is NaN or infinite.
    #[error("group {group}: coordinate is not finite")]
    NonFinite {
        /// Group position in the file.
        group: usize,
    },
    /// A node's orbit or orbit index is outside the orbits in the header.
    #[error("node {node}: orbit {orbit} index {index} is outside the header's orbits")]
    Orbit {
        /// Node ID.
        node: u32,
        /// Orbit.
        orbit: u32,
        /// Orbit index.
        index: u32,
    },
    /// Bytes after the last group.
    #[error("{len} bytes after the last group at offset {offset}")]
    Trailing {
        /// Offset of the first extra byte.
        offset: usize,
        /// Number of extra bytes.
        len: usize,
    },
}

impl From<Short> for PsgError {
    fn from(s: Short) -> Self {
        Self::Truncated(s)
    }
}

impl PsgError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Truncated(_) => "EXTRACT_PSG_TRUNCATED",
            Self::Version(_) => "EXTRACT_PSG_VERSION",
            Self::NonFinite { .. } => "EXTRACT_PSG_NON_FINITE",
            Self::Orbit { .. } => "EXTRACT_PSG_ORBIT",
            Self::Trailing { .. } => "EXTRACT_PSG_TRAILING",
        }
    }
}

/// A connection to another node. Fields in alphabetical order (stable JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Connection {
    /// Node ID at the other end.
    pub id: u32,
    /// Arc of the drawn line (PoE2 only; `None` in PoE1 files).
    pub orbit: Option<i32>,
}

/// A node. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    /// Outgoing connections.
    pub connections: Vec<Connection>,
    /// `PassiveSkills.PassiveSkillGraphId`.
    pub id: u32,
    /// Orbit ring.
    pub orbit: u32,
    /// Slot on the ring.
    pub orbit_index: u32,
}

/// A group of nodes around one centre. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Group {
    /// Flags as stored.
    pub flags: u32,
    /// Nodes in file order.
    pub nodes: Vec<Node>,
    /// Unnamed `u32` after the flags, as stored.
    pub unknown1: u32,
    /// Unnamed byte before the node count, as stored.
    pub unknown2: u8,
    /// Centre x.
    pub x: f32,
    /// Centre y.
    pub y: f32,
}

/// A parsed graph. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Graph {
    /// Groups in file order.
    pub groups: Vec<Group>,
    /// Graph kind byte, as stored.
    pub kind: u8,
    /// Nodes per orbit ring.
    pub orbit_slots: Vec<u8>,
    /// Root node IDs (class starts).
    pub roots: Vec<u64>,
    /// Format version.
    pub version: u8,
}

impl Graph {
    /// Every node ID, in file order.
    pub fn node_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.groups
            .iter()
            .flat_map(|g| g.nodes.iter().map(|n| n.id))
    }

    /// Number of connections.
    #[must_use]
    pub fn connections(&self) -> usize {
        self.groups
            .iter()
            .flat_map(|g| &g.nodes)
            .map(|n| n.connections.len())
            .sum()
    }

    /// Connections whose far end is not a node of this graph.
    #[must_use]
    pub fn dangling_connections(&self) -> usize {
        let ids: BTreeSet<u32> = self.node_ids().collect();
        self.groups
            .iter()
            .flat_map(|g| &g.nodes)
            .flat_map(|n| &n.connections)
            .filter(|c| !ids.contains(&c.id))
            .count()
    }

    /// Roots that are not nodes of this graph.
    #[must_use]
    pub fn dangling_roots(&self) -> usize {
        let ids: BTreeSet<u64> = self.node_ids().map(u64::from).collect();
        self.roots.iter().filter(|r| !ids.contains(r)).count()
    }

    /// Node IDs that appear more than once.
    #[must_use]
    pub fn duplicate_nodes(&self) -> usize {
        let mut seen = BTreeSet::new();
        self.node_ids().filter(|id| !seen.insert(*id)).count()
    }
}

/// Reads `count` items with `read`, without trusting `count` for allocation:
/// every item reads at least one byte, so a false count ends in
/// [`PsgError::Truncated`] after at most one item per input byte. Memory stays
/// proportional to the input (a node takes about 40 bytes for its 16 input
/// bytes), which is fine for game files of a few hundred KiB.
fn repeat<T>(
    r: &mut Reader<'_>,
    count: u32,
    mut read: impl FnMut(&mut Reader<'_>) -> Result<T, PsgError>,
) -> Result<Vec<T>, PsgError> {
    let mut out = Vec::new();
    for _ in 0..count {
        out.push(read(r)?);
    }
    Ok(out)
}

fn f32_(r: &mut Reader<'_>, field: &'static str) -> Result<f32, PsgError> {
    Ok(f32::from_bits(r.u32(field)?))
}

/// Parses a graph of `game`.
///
/// # Errors
/// [`PsgError`] if the file is truncated, has another version, a non-finite
/// group position, a node outside the header's orbits, or trailing bytes.
pub fn parse(bytes: &[u8], game: Game) -> Result<Graph, PsgError> {
    let mut r = Reader::new(bytes);
    let version = r.u8("version")?;
    if version != VERSION {
        return Err(PsgError::Version(version));
    }
    let kind = r.u8("kind")?;
    let orbits = r.u8("orbit count")?;
    let orbit_slots = r.take("orbit slots", usize::from(orbits))?.to_vec();
    let root_count = r.u32("root count")?;
    let roots = repeat(&mut r, root_count, |r| {
        Ok(match game {
            Game::Poe1 => u64::from(r.u32("root")?),
            Game::Poe2 => r.u64("root")?,
        })
    })?;
    let group_count = r.u32("group count")?;
    let mut groups = Vec::new();
    for g in 0..widen(group_count) {
        let x = f32_(&mut r, "group x")?;
        let y = f32_(&mut r, "group y")?;
        if !x.is_finite() || !y.is_finite() {
            return Err(PsgError::NonFinite { group: g });
        }
        let flags = r.u32("group flags")?;
        let unknown1 = r.u32("group unknown1")?;
        let unknown2 = r.u8("group unknown2")?;
        let node_count = r.u32("node count")?;
        let nodes = repeat(&mut r, node_count, |r| {
            let id = r.u32("node id")?;
            let orbit = r.u32("node orbit")?;
            let orbit_index = r.u32("node orbit index")?;
            let slots = usize::try_from(orbit)
                .ok()
                .and_then(|o| orbit_slots.get(o))
                .copied();
            if slots.is_none_or(|s| orbit_index >= u32::from(s)) {
                return Err(PsgError::Orbit {
                    node: id,
                    orbit,
                    index: orbit_index,
                });
            }
            let connection_count = r.u32("connection count")?;
            let connections = repeat(r, connection_count, |r| {
                let id = r.u32("connection id")?;
                let orbit = match game {
                    Game::Poe1 => None,
                    Game::Poe2 => {
                        Some(i32::from_le_bytes(r.u32("connection orbit")?.to_le_bytes()))
                    }
                };
                Ok(Connection { id, orbit })
            })?;
            Ok(Node {
                connections,
                id,
                orbit,
                orbit_index,
            })
        })?;
        groups.push(Group {
            flags,
            nodes,
            unknown1,
            unknown2,
            x,
            y,
        });
    }
    if !r.is_empty() {
        return Err(PsgError::Trailing {
            offset: r.pos(),
            len: r.rest().len(),
        });
    }
    Ok(Graph {
        groups,
        kind,
        orbit_slots,
        roots,
        version,
    })
}

/// Renders a graph as JSON: header fields, then one group per line.
fn render(graph: &Graph) -> Result<Vec<u8>, ExtractError> {
    let mut out = String::from("{\n\"groups\": [");
    for (i, group) in graph.groups.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        out.push_str(&serde_json::to_string(group)?);
    }
    out.push_str(if graph.groups.is_empty() {
        "],\n"
    } else {
        "\n],\n"
    });
    out.push_str(&format!(
        "\"kind\": {},\n\"orbit_slots\": {},\n\"roots\": {},\n\"version\": {}\n}}\n",
        graph.kind,
        serde_json::to_string(&graph.orbit_slots)?,
        serde_json::to_string(&graph.roots)?,
        graph.version
    ));
    Ok(out.into_bytes())
}

/// Outcome of one graph file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphStatus {
    /// Parsed to the last byte.
    Ok,
    /// Did not parse: see `error`. No output file.
    Failed,
}

/// One graph in the manifest. Fields in alphabetical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphEntry {
    /// Connections.
    pub connections: Option<usize>,
    /// Connections to a node not in the graph (none on the installs; a
    /// connection is assumed to stay inside its graph, so one fails the run).
    pub dangling_connections: Option<usize>,
    /// Roots that are not nodes of the graph (recorded only).
    pub dangling_roots: Option<usize>,
    /// Node IDs listed more than once.
    pub duplicate_nodes: Option<usize>,
    /// Why the file did not parse.
    pub error: Option<TableError>,
    /// Groups.
    pub groups: Option<usize>,
    /// Graph kind byte.
    pub kind: Option<u8>,
    /// Nodes (with duplicates).
    pub nodes: Option<usize>,
    /// Output file path.
    pub output: Option<String>,
    /// Root node IDs.
    pub roots: Option<usize>,
    /// Path in the index.
    pub source: String,
    /// SHA-256 of the file as read from its (decoded) bundle.
    pub source_sha256: String,
    /// Outcome.
    pub status: GraphStatus,
}

/// The graph section of the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphManifest {
    /// Output folder.
    pub dir: &'static str,
    /// Every `*.psg` file in the index, by lowercase index path.
    pub files: BTreeMap<String, GraphEntry>,
}

impl GraphManifest {
    /// Problems that make the run fail: no graph at all, a graph that did
    /// not parse, or a connection to a node outside its graph.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.files.is_empty() {
            problems.push(format!("no {EXTENSION} files in the index"));
        }
        for (path, file) in &self.files {
            if file.status == GraphStatus::Failed {
                problems.push(format!("{path}: Failed"));
            }
            if let Some(n @ 1..) = file.dangling_connections {
                problems.push(format!("{path}: {n} dangling connections"));
            }
        }
        problems
    }
}

#[derive(Debug, Clone)]
struct Added {
    source: String,
    sha256: [u8; 32],
    result: Result<(Graph, Vec<u8>), TableError>,
}

/// Collects the graph files of one game in any order.
#[derive(Debug, Clone)]
pub struct Collection {
    game: Game,
    added: BTreeMap<String, Added>,
}

impl Collection {
    /// An empty collection for `game`.
    #[must_use]
    pub const fn new(game: Game) -> Self {
        Self {
            game,
            added: BTreeMap::new(),
        }
    }

    /// Parses and renders the graph at `path` (as listed in the index). A
    /// file that does not parse is recorded, not returned.
    ///
    /// # Errors
    /// [`ExtractError`] if `path` is not a graph, was added before, or JSON
    /// cannot be written.
    pub fn add(&mut self, path: &str, bytes: &[u8]) -> Result<(), ExtractError> {
        if !is_graph(path) {
            return Err(ExtractError::UnplannedFile {
                path: path.to_owned(),
            });
        }
        let key = path.to_ascii_lowercase();
        if self.added.contains_key(&key) {
            return Err(ExtractError::DuplicateFile {
                path: path.to_owned(),
            });
        }
        let result = if crate::statdesc::plain_relative(&key) {
            match parse(bytes, self.game) {
                Ok(graph) => {
                    let json = render(&graph)?;
                    Ok((graph, json))
                }
                Err(e) => Err(TableError {
                    code: e.code(),
                    message: e.to_string(),
                }),
            }
        } else {
            Err(TableError {
                code: "EXTRACT_PSG_BAD_PATH",
                message: format!("{path} is not a plain relative path"),
            })
        };
        self.added.insert(
            key,
            Added {
                source: path.to_owned(),
                sha256: Sha256::digest(bytes).into(),
                result,
            },
        );
        Ok(())
    }

    /// Builds the output files (by path inside the output directory) and the
    /// manifest section.
    #[must_use]
    pub fn finish(self) -> (BTreeMap<String, Vec<u8>>, GraphManifest) {
        let mut files = BTreeMap::new();
        let mut entries = BTreeMap::new();
        for (key, added) in self.added {
            let mut entry = GraphEntry {
                connections: None,
                dangling_connections: None,
                dangling_roots: None,
                duplicate_nodes: None,
                error: None,
                groups: None,
                kind: None,
                nodes: None,
                output: None,
                roots: None,
                source: added.source,
                source_sha256: hex(&added.sha256),
                status: GraphStatus::Failed,
            };
            match added.result {
                Ok((graph, json)) => {
                    let out = output_path(&key);
                    entry.connections = Some(graph.connections());
                    entry.dangling_connections = Some(graph.dangling_connections());
                    entry.dangling_roots = Some(graph.dangling_roots());
                    entry.duplicate_nodes = Some(graph.duplicate_nodes());
                    entry.groups = Some(graph.groups.len());
                    entry.kind = Some(graph.kind);
                    entry.nodes = Some(graph.node_ids().count());
                    entry.roots = Some(graph.roots.len());
                    entry.output = Some(out.clone());
                    entry.status = GraphStatus::Ok;
                    files.insert(out, json);
                }
                Err(error) => entry.error = Some(error),
            }
            entries.insert(key, entry);
        }
        (
            files,
            GraphManifest {
                dir: GRAPHS_DIR,
                files: entries,
            },
        )
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;

    /// A test node: `(id, orbit, index, connections)`.
    type TestNode<'a> = (u32, u32, u32, &'a [u32]);

    /// Builds a graph file: two orbits (1 and 6 slots), the given roots, and
    /// groups of nodes.
    fn file(game: Game, roots: &[u32], groups: &[&[TestNode<'_>]]) -> Vec<u8> {
        let mut b = vec![VERSION, 0, 2, 1, 6];
        b.extend(u32::try_from(roots.len()).unwrap().to_le_bytes());
        for &root in roots {
            match game {
                Game::Poe1 => b.extend(root.to_le_bytes()),
                Game::Poe2 => b.extend(u64::from(root).to_le_bytes()),
            }
        }
        b.extend(u32::try_from(groups.len()).unwrap().to_le_bytes());
        for nodes in groups {
            b.extend(1.5f32.to_le_bytes());
            b.extend((-2.0f32).to_le_bytes());
            b.extend(0u32.to_le_bytes());
            b.extend(7u32.to_le_bytes());
            b.push(1);
            b.extend(u32::try_from(nodes.len()).unwrap().to_le_bytes());
            for (id, orbit, index, connections) in *nodes {
                b.extend(id.to_le_bytes());
                b.extend(orbit.to_le_bytes());
                b.extend(index.to_le_bytes());
                b.extend(u32::try_from(connections.len()).unwrap().to_le_bytes());
                for c in *connections {
                    b.extend(c.to_le_bytes());
                    if game == Game::Poe2 {
                        b.extend((-1i32).to_le_bytes());
                    }
                }
            }
        }
        b
    }

    fn sample(game: Game) -> Vec<u8> {
        file(
            game,
            &[10],
            &[
                &[(10, 0, 0, &[20])],
                &[(20, 1, 5, &[10, 30]), (30, 1, 0, &[])],
            ],
        )
    }

    #[test]
    fn parse_should_read_poe1_layout() {
        let g = parse(&sample(Game::Poe1), Game::Poe1).unwrap();
        assert_eq!(g.version, 3);
        assert_eq!(g.orbit_slots, [1, 6]);
        assert_eq!(g.roots, [10]);
        assert_eq!(g.groups.len(), 2);
        assert_eq!((g.groups[0].x, g.groups[0].y), (1.5, -2.0));
        assert_eq!((g.groups[0].unknown1, g.groups[0].unknown2), (7, 1));
        assert_eq!(g.node_ids().collect::<Vec<_>>(), [10, 20, 30]);
        assert_eq!(
            g.groups[1].nodes[0].connections,
            [
                Connection {
                    id: 10,
                    orbit: None
                },
                Connection {
                    id: 30,
                    orbit: None
                }
            ]
        );
        assert_eq!(g.connections(), 3);
        assert_eq!(g.dangling_connections(), 0);
        assert_eq!(g.duplicate_nodes(), 0);
    }

    #[test]
    fn parse_should_read_poe2_wide_roots_and_connection_orbits() {
        let g = parse(&sample(Game::Poe2), Game::Poe2).unwrap();
        assert_eq!(g.roots, [10]);
        assert_eq!(
            g.groups[0].nodes[0].connections,
            [Connection {
                id: 20,
                orbit: Some(-1)
            }]
        );
    }

    #[test]
    fn parse_should_fail_on_the_other_games_layout() {
        // With roots or connections, the other game's widths do not fit the
        // bytes. A graph with neither reads the same either way (see
        // `parse_should_read_graphs_without_roots_or_connections_in_both_layouts`).
        assert!(parse(&sample(Game::Poe2), Game::Poe1).is_err());
        assert!(parse(&sample(Game::Poe1), Game::Poe2).is_err());
    }

    #[test]
    fn parse_should_report_truncation_at_every_length() {
        let bytes = sample(Game::Poe2);
        for len in 0..bytes.len() {
            let err = parse(&bytes[..len], Game::Poe2).unwrap_err();
            assert_eq!(err.code(), "EXTRACT_PSG_TRUNCATED", "length {len}");
        }
    }

    #[test]
    fn parse_should_reject_trailing_bytes() {
        let mut bytes = sample(Game::Poe1);
        let len = bytes.len();
        bytes.push(0);
        assert_eq!(
            parse(&bytes, Game::Poe1).unwrap_err(),
            PsgError::Trailing {
                offset: len,
                len: 1
            }
        );
    }

    #[test]
    fn parse_should_reject_other_versions() {
        let mut bytes = sample(Game::Poe1);
        bytes[0] = 2;
        assert_eq!(
            parse(&bytes, Game::Poe1).unwrap_err().code(),
            "EXTRACT_PSG_VERSION"
        );
    }

    #[test]
    fn parse_should_reject_nodes_outside_the_orbits() {
        let orbit = file(Game::Poe1, &[], &[&[(1, 2, 0, &[])]]);
        let index = file(Game::Poe1, &[], &[&[(1, 1, 6, &[])]]);
        for bytes in [orbit, index] {
            assert_eq!(
                parse(&bytes, Game::Poe1).unwrap_err().code(),
                "EXTRACT_PSG_ORBIT"
            );
        }
    }

    #[test]
    fn parse_should_reject_non_finite_positions() {
        let mut bytes = file(Game::Poe1, &[], &[&[]]);
        // Header 5 bytes, root count 4, group count 4: x starts at 13.
        bytes[13..17].copy_from_slice(&f32::NAN.to_le_bytes());
        assert_eq!(
            parse(&bytes, Game::Poe1).unwrap_err(),
            PsgError::NonFinite { group: 0 }
        );
    }

    #[test]
    fn parse_should_not_trust_huge_counts() {
        let mut bytes = vec![VERSION, 0, 0];
        bytes.extend(u32::MAX.to_le_bytes());
        assert_eq!(
            parse(&bytes, Game::Poe2).unwrap_err().code(),
            "EXTRACT_PSG_TRUNCATED"
        );
    }

    #[test]
    fn graph_should_count_dangling_connections_roots_and_duplicates() {
        let bytes = file(Game::Poe1, &[1, 5], &[&[(1, 0, 0, &[9]), (1, 1, 0, &[])]]);
        let g = parse(&bytes, Game::Poe1).unwrap();
        assert_eq!(g.dangling_connections(), 1);
        assert_eq!(g.dangling_roots(), 1);
        assert_eq!(g.duplicate_nodes(), 1);
    }

    #[test]
    fn parse_should_read_graphs_without_roots_or_connections_in_both_layouts() {
        // The widths only differ in roots and connections; without them
        // both games read the same bytes the same way.
        let bytes = file(Game::Poe1, &[], &[&[(1, 0, 0, &[])]]);
        assert_eq!(bytes, file(Game::Poe2, &[], &[&[(1, 0, 0, &[])]]));
        assert_eq!(
            parse(&bytes, Game::Poe1).unwrap(),
            parse(&bytes, Game::Poe2).unwrap()
        );
    }

    #[test]
    fn parse_should_keep_poe2_connection_orbits_as_stored() {
        let mut bytes = sample(Game::Poe2);
        // The first connection's orbit (-1) becomes the observed sentinel.
        let at = bytes.windows(4).position(|w| w == [0xff; 4]).unwrap();
        bytes[at..at + 4].copy_from_slice(&i32::MAX.to_le_bytes());
        let g = parse(&bytes, Game::Poe2).unwrap();
        assert_eq!(g.groups[0].nodes[0].connections[0].orbit, Some(i32::MAX));
        assert_eq!(g.groups[1].nodes[0].connections[0].orbit, Some(-1));
    }

    #[test]
    fn collection_should_refuse_paths_that_leave_the_output_folder() {
        let mut c = Collection::new(Game::Poe1);
        c.add("metadata/../x.psg", &sample(Game::Poe1)).unwrap();
        let (files, manifest) = c.finish();
        assert!(files.is_empty());
        let entry = &manifest.files["metadata/../x.psg"];
        assert_eq!(entry.status, GraphStatus::Failed);
        assert_eq!(entry.error.as_ref().unwrap().code, "EXTRACT_PSG_BAD_PATH");
    }

    #[test]
    fn render_should_write_header_fields_of_a_graph_without_groups() {
        let mut c = Collection::new(Game::Poe2);
        c.add("a.psg", &file(Game::Poe2, &[7], &[])).unwrap();
        let (files, _) = c.finish();
        assert_eq!(
            String::from_utf8(files["passive_skill_graphs/a.psg.json"].clone()).unwrap(),
            "{
\"groups\": [],
\"kind\": 0,
\"orbit_slots\": [1,6],
\"roots\": [7],
\"version\": 3
}
"
        );
    }

    #[test]
    fn collection_should_render_ok_and_record_failed_graphs() {
        let mut c = Collection::new(Game::Poe1);
        c.add("Metadata/PassiveSkillGraph.psg", &sample(Game::Poe1))
            .unwrap();
        c.add("metadata/broken.psg", &[2]).unwrap();
        assert_eq!(
            c.add("metadata/passiveskillgraph.psg", &[])
                .unwrap_err()
                .code(),
            "EXTRACT_DUPLICATE_FILE"
        );
        assert_eq!(
            c.add("metadata/x.txt", &[]).unwrap_err().code(),
            "EXTRACT_UNPLANNED_FILE"
        );
        let (files, manifest) = c.finish();
        let out = "passive_skill_graphs/metadata/passiveskillgraph.psg.json";
        assert_eq!(files.keys().collect::<Vec<_>>(), [out]);
        let text = String::from_utf8(files[out].clone()).unwrap();
        assert_eq!(
            text,
            "{\n\"groups\": [\n\
             {\"flags\":0,\"nodes\":[{\"connections\":[{\"id\":20,\"orbit\":null}],\"id\":10,\"orbit\":0,\"orbit_index\":0}],\"unknown1\":7,\"unknown2\":1,\"x\":1.5,\"y\":-2.0},\n\
             {\"flags\":0,\"nodes\":[{\"connections\":[{\"id\":10,\"orbit\":null},{\"id\":30,\"orbit\":null}],\"id\":20,\"orbit\":1,\"orbit_index\":5},{\"connections\":[],\"id\":30,\"orbit\":1,\"orbit_index\":0}],\"unknown1\":7,\"unknown2\":1,\"x\":1.5,\"y\":-2.0}\n\
             ],\n\"kind\": 0,\n\"orbit_slots\": [1,6],\n\"roots\": [10],\n\"version\": 3\n}\n"
        );
        let ok = &manifest.files["metadata/passiveskillgraph.psg"];
        assert_eq!(ok.status, GraphStatus::Ok);
        assert_eq!(
            (ok.nodes, ok.connections, ok.roots),
            (Some(3), Some(3), Some(1))
        );
        assert_eq!(ok.source, "Metadata/PassiveSkillGraph.psg");
        let failed = &manifest.files["metadata/broken.psg"];
        assert_eq!(failed.status, GraphStatus::Failed);
        assert_eq!(failed.error.as_ref().unwrap().code, "EXTRACT_PSG_VERSION");
        assert_eq!(manifest.problems(), ["metadata/broken.psg: Failed"]);
    }

    #[test]
    fn manifest_should_flag_no_graphs_and_dangling_connections() {
        let (_, empty) = Collection::new(Game::Poe1).finish();
        assert_eq!(empty.problems(), ["no .psg files in the index"]);
        let mut c = Collection::new(Game::Poe1);
        c.add("a.psg", &file(Game::Poe1, &[], &[&[(1, 0, 0, &[9])]]))
            .unwrap();
        assert_eq!(c.finish().1.problems(), ["a.psg: 1 dangling connections"]);
    }
}

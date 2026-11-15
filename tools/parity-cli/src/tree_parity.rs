//! `tree-parity`: the passive tree against PoB (passive tree plan, gate
//! items 1 and 2; T7's read-back). Maintainer-run: PoB's side is written by
//! `tools/oracle/data/pob_tree.lua` on the pinned oracle runtime
//! (`docs/tree/parity.md`).
//!
//! - `layout <pob-layout.json> <bundle>`: every node of the bundle's tree
//!   placed within 0.01 of PoB's position, its links equal to PoB's, and
//!   each class's and ascendancy's start equal.
//! - `alloc-requests <bundle> <count> <seed>`: writes seeded allocation
//!   requests for `pob_tree.lua alloc`.
//! - `alloc <requests.txt> <pob-alloc.json> <bundle>`: for each request,
//!   loom's rules and PoB's give the same path length and validity, the
//!   same path where it is unique, the same deallocation set, and the same
//!   point counts.
//! - `readback <build.xml> <pob-readback.json>`: PoB reads every spec of a
//!   loom export with the allocation loom wrote.
//!
//! Each prints a JSON record and exits 0 when everything matches, 1
//! otherwise.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use loom_data::bundle::Bundle;
use loom_tree::Tree;
use loom_tree::poe1::{self, Spec};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A list PoB's JSON writer gives as `{}` when empty (a Lua table).
fn list<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    match Value::deserialize(d)? {
        Value::Object(m) if m.is_empty() => Ok(Vec::new()),
        v => serde_json::from_value(v).map_err(serde::de::Error::custom),
    }
}

fn opt_list<'de, D, T>(d: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    list(d).map(Some)
}

/// A position's largest allowed difference (plan, gate 1).
const TOLERANCE: f64 = 0.01;

/// GGG's export (which PoB's `tree.lua` carries) writes group centres
/// rounded, 116 of 3_29's groups to one decimal (up to 0.05 off); the
/// game's psg, which the bundle carries, has them in full. An ascendancy
/// node is compared under its ascendancy's mean offset, which carries the
/// same rounding again. A node within this but not `TOLERANCE` is counted
/// as the export's rounding, not as a difference; the record lists them.
const ROUNDING: f64 = 0.1;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The bundle's tree and its manifest's SHA-256.
fn load_tree(bundle: &Path) -> Result<(Tree, String), String> {
    let source = crate::DirSource(bundle);
    let loaded = Bundle::load(&source).map_err(|e| format!("{}: {e}", bundle.display()))?;
    let tables = loaded
        .passives
        .as_ref()
        .ok_or("the bundle has no passive tree (format 1)")?;
    let manifest = read(&bundle.join("bundle.json"))?;
    Ok((
        Tree::from_tables(tables, loaded.tree_layout.as_ref()),
        sha256(&manifest),
    ))
}

#[derive(Deserialize)]
struct PobLayout {
    tree_version: String,
    nodes: BTreeMap<String, PobNode>,
    #[serde(default)]
    removed: BTreeMap<String, String>,
    classes: BTreeMap<String, PobClass>,
}

#[derive(Deserialize)]
struct PobNode {
    x: Option<f64>,
    y: Option<f64>,
    #[serde(default)]
    ascendancy: Option<String>,
    #[serde(deserialize_with = "list")]
    links: Vec<u32>,
}

#[derive(Deserialize)]
struct PobClass {
    name: String,
    start: Option<u32>,
    ascendancies: BTreeMap<String, PobAscendancy>,
}

#[derive(Deserialize)]
struct PobAscendancy {
    name: String,
    start: Option<u32>,
}

/// A difference, named by node or class.
#[derive(Serialize)]
struct Difference {
    what: String,
    loom: Value,
    pob: Value,
}

/// The layout record.
#[derive(Serialize)]
pub struct LayoutRecord {
    tree_version: String,
    pob_layout_sha256: String,
    bundle_manifest_sha256: String,
    nodes: usize,
    placed_within_tolerance: usize,
    largest_offset: f64,
    links_equal: usize,
    /// Links PoB has to nodes the bundle lacks, per bundle node (counted,
    /// not differences: the bundle carries the game's graph).
    links_to_nodes_not_carried: usize,
    pob_nodes_not_carried: usize,
    /// Ascendancy nodes compared under one offset per ascendancy (the
    /// mean over its nodes): PoB's `fix_ascendancy_positions.py` moves
    /// every group of an ascendancy by one offset (its start to a point of
    /// PoB's), so its `tree.lua` is not GGG's export there. The offsets,
    /// by ascendancy.
    ascendancy_offsets: BTreeMap<String, [f64; 2]>,
    /// Nodes PoB's tree file has and its `PassiveTree` class removes (the
    /// legacy alternate ascendancies), by ascendancy.
    removed_by_pob: BTreeMap<String, usize>,
    /// Nodes placed within `ROUNDING` but not `TOLERANCE` (the export's
    /// rounded group centres), each with its offset.
    placed_within_export_rounding: BTreeMap<u32, f64>,
    classes: usize,
    starts_equal: usize,
    differences: Vec<Difference>,
}

/// `tree-parity layout`.
///
/// # Errors
/// An unreadable input.
pub fn layout(pob: &Path, bundle: &Path) -> Result<(LayoutRecord, bool), String> {
    let pob_bytes = read(pob)?;
    let theirs: PobLayout =
        serde_json::from_slice(&pob_bytes).map_err(|e| format!("{}: {e}", pob.display()))?;
    let (tree, manifest) = load_tree(bundle)?;
    if !tree.placed() {
        return Err(format!(
            "{}: the bundle has no layout constants",
            bundle.display()
        ));
    }
    let mut differences = Vec::new();
    let (mut placed, mut links_equal, mut not_carried_links) = (0, 0, 0);
    let mut rounded: BTreeMap<u32, f64> = BTreeMap::new();
    let mut largest = 0.0_f64;
    // Each ascendancy's offset: the mean of its nodes' (PoB less loom),
    // so neither side's rounding of one node sets it.
    let mut sums: BTreeMap<&str, (f64, f64, f64)> = BTreeMap::new();
    for node in tree.nodes.values() {
        let Some(other) = theirs.nodes.get(&node.id.to_string()) else {
            continue;
        };
        if let (Some(asc), Some((x, y)), Some(px), Some(py)) =
            (other.ascendancy.as_deref(), node.position, other.x, other.y)
        {
            let e = sums.entry(asc).or_insert((0.0, 0.0, 0.0));
            *e = (e.0 + px - x, e.1 + py - y, e.2 + 1.0);
        }
    }
    let means: BTreeMap<&str, (f64, f64)> = sums
        .into_iter()
        .map(|(a, (x, y, n))| (a, (x / n, y / n)))
        .collect();
    let pob_ascendancies: BTreeSet<&str> = theirs
        .nodes
        .values()
        .filter_map(|n| n.ascendancy.as_deref())
        .collect();
    let mut offsets: BTreeMap<String, [f64; 2]> = BTreeMap::new();
    let mut removed: BTreeMap<String, usize> = BTreeMap::new();
    for node in tree.nodes.values() {
        let Some(other) = theirs.nodes.get(&node.id.to_string()) else {
            if let Some(asc) = theirs.removed.get(&node.id.to_string()) {
                *removed.entry(asc.clone()).or_default() += 1;
                continue;
            }
            differences.push(Difference {
                what: format!("node {}: not in PoB's tree", node.id),
                loom: Value::Bool(true),
                pob: Value::Null,
            });
            continue;
        };
        // An ascendancy node is compared relative to its start.
        let shift = match other
            .ascendancy
            .as_deref()
            .and_then(|a| means.get(a).map(|m| (a, *m)))
        {
            Some((asc, (dx, dy))) => {
                offsets.insert(asc.to_owned(), [dx, dy]);
                (dx, dy)
            }
            None => (0.0, 0.0),
        };
        match (node.position, other.x.zip(other.y)) {
            (Some((x, y)), Some((px, py))) => {
                let offset = (x + shift.0 - px).abs().max((y + shift.1 - py).abs());
                if offset <= TOLERANCE {
                    largest = largest.max(offset);
                    placed += 1;
                } else if offset <= ROUNDING {
                    rounded.insert(node.id, offset);
                } else {
                    largest = largest.max(offset);
                    differences.push(Difference {
                        what: format!("node {}: position", node.id),
                        loom: serde_json::json!([x, y]),
                        pob: serde_json::json!([px, py]),
                    });
                }
            }
            (ours, pobs) => differences.push(Difference {
                what: format!("node {}: placed", node.id),
                loom: serde_json::json!(ours),
                pob: serde_json::json!(pobs),
            }),
        }
        let ours: BTreeSet<u32> = node.neighbours.iter().copied().collect();
        let pobs: BTreeSet<u32> = other
            .links
            .iter()
            .copied()
            .filter(|l| {
                let carried = tree.nodes.contains_key(l);
                if !carried {
                    not_carried_links += 1;
                }
                carried
            })
            .collect();
        if ours == pobs {
            links_equal += 1;
        } else {
            differences.push(Difference {
                what: format!("node {}: links", node.id),
                loom: serde_json::json!(ours),
                pob: serde_json::json!(pobs),
            });
        }
    }
    let pob_not_carried = theirs
        .nodes
        .keys()
        .filter(|k| {
            k.parse::<u32>()
                .is_ok_and(|id| !tree.nodes.contains_key(&id))
        })
        .count();
    let mut starts = 0;
    let mut class_count = 0;
    for class in &tree.classes {
        class_count += 1;
        let Some(other) = theirs.classes.get(&class.number.to_string()) else {
            differences.push(Difference {
                what: format!("class {}: not in PoB's tree", class.number),
                loom: Value::String(class.name.clone()),
                pob: Value::Null,
            });
            continue;
        };
        let mut check = |what: String, ours: Option<u32>, pobs: Option<u32>| {
            if ours == pobs {
                starts += 1;
            } else {
                differences.push(Difference {
                    what,
                    loom: serde_json::json!(ours),
                    pob: serde_json::json!(pobs),
                });
            }
        };
        check(
            format!("class {} ({}): start", class.number, other.name),
            Some(class.start),
            other.start,
        );
        for asc in class
            .ascendancies
            .iter()
            .filter(|a| pob_ascendancies.contains(a.id.as_str()))
        {
            let pobs = other.ascendancies.values().find(|a| a.name == asc.name);
            check(
                format!("ascendancy {}: start", asc.name),
                asc.start,
                pobs.and_then(|a| a.start),
            );
        }
    }
    let ok = differences.is_empty();
    Ok((
        LayoutRecord {
            tree_version: theirs.tree_version,
            pob_layout_sha256: sha256(&pob_bytes),
            bundle_manifest_sha256: manifest,
            nodes: tree.nodes.len(),
            placed_within_tolerance: placed,
            largest_offset: largest,
            links_equal,
            links_to_nodes_not_carried: not_carried_links,
            pob_nodes_not_carried: pob_not_carried,
            ascendancy_offsets: offsets,
            removed_by_pob: removed,
            placed_within_export_rounding: rounded,
            classes: class_count,
            starts_equal: starts,
            differences,
        },
        ok,
    ))
}

/// One allocation request: a spec and a target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    class: u32,
    ascendancy: u32,
    target: u32,
    allocated: Vec<u32>,
}

impl Request {
    fn line(&self) -> String {
        let list: Vec<String> = self.allocated.iter().map(u32::to_string).collect();
        format!(
            "{} {} {} {}",
            self.class,
            self.ascendancy,
            self.target,
            list.join(",")
        )
    }

    fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(' ');
        let class = parts.next()?.parse().ok()?;
        let ascendancy = parts.next()?.parse().ok()?;
        let target = parts.next()?.parse().ok()?;
        let allocated = match parts.next() {
            None | Some("") => Vec::new(),
            Some(list) => list
                .split(',')
                .map(str::parse)
                .collect::<Result<_, _>>()
                .ok()?,
        };
        Some(Self {
            class,
            ascendancy,
            target,
            allocated,
        })
    }
}

/// A small deterministic generator (xorshift64*), so a seed names the
/// requests exactly.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n.max(1) as u64)).unwrap_or(0)
    }
}

/// `tree-parity alloc-requests`: `count` requests from `seed`. Each picks
/// a class and maybe an ascendancy, grows a connected spec by allocating
/// random modelled targets with loom's rules, then names one more target:
/// allocated (a deallocation case) or not (an allocation case), so PoB and
/// loom start from the same connected spec.
///
/// # Errors
/// An unreadable bundle.
pub fn alloc_requests(bundle: &Path, count: usize, seed: u64) -> Result<String, String> {
    let (tree, _) = load_tree(bundle)?;
    let mut rng = Rng(seed.max(1));
    let targets: Vec<u32> = tree
        .nodes
        .values()
        .filter(|n| {
            n.modelled
                && !matches!(
                    n.kind,
                    loom_tree::Kind::Mastery
                        | loom_tree::Kind::ClassStart
                        | loom_tree::Kind::AscendancyStart
                )
        })
        .map(|n| n.id)
        .collect();
    let mut out = String::new();
    for _ in 0..count {
        let class = &tree.classes[rng.below(tree.classes.len())];
        let ascendancy = if rng.below(2) == 0 {
            0
        } else {
            class.ascendancies[rng.below(class.ascendancies.len())].number
        };
        let mut spec = Spec {
            class: class.number,
            ascendancy,
            nodes: BTreeSet::new(),
            masteries: BTreeMap::new(),
        };
        spec.nodes.extend(poe1::starts(&tree, &spec));
        for _ in 0..rng.below(6) {
            let target = targets[rng.below(targets.len())];
            if let Ok(a) = poe1::allocate(&tree, &spec, target) {
                spec = a.spec;
            }
        }
        let target = if rng.below(3) == 0 && spec.nodes.len() > 1 {
            let allocated: Vec<u32> = spec.nodes.iter().copied().collect();
            allocated[rng.below(allocated.len())]
        } else {
            targets[rng.below(targets.len())]
        };
        let request = Request {
            class: class.number,
            ascendancy,
            target,
            allocated: spec.nodes.iter().copied().collect(),
        };
        out.push_str(&request.line());
        out.push('\n');
    }
    Ok(out)
}

#[derive(Deserialize)]
struct PobAlloc {
    tree_version: String,
    #[serde(deserialize_with = "list")]
    results: Vec<PobResult>,
}

#[derive(Deserialize)]
struct PobResult {
    #[serde(deserialize_with = "list")]
    allocated: Vec<u32>,
    points: u32,
    ascendancy_points: u32,
    #[serde(default)]
    missing: bool,
    #[serde(default, deserialize_with = "opt_list")]
    path: Option<Vec<u32>>,
    #[serde(default, deserialize_with = "opt_list")]
    depends: Option<Vec<u32>>,
}

/// The allocation record.
#[derive(Serialize, Default)]
pub struct AllocRecord {
    tree_version: String,
    requests_sha256: String,
    pob_alloc_sha256: String,
    bundle_manifest_sha256: String,
    requests: usize,
    allocations: usize,
    /// Allocation cases with one shortest path, compared node for node.
    unique_paths_equal: usize,
    /// Allocation cases with several shortest paths: length and validity
    /// compared, nodes not (plan T-f: PoB's choice is Lua hash order).
    tied_paths_length_equal: usize,
    unreachable_equal: usize,
    deallocations: usize,
    deallocations_equal: usize,
    points_equal: usize,
    differences: Vec<Difference>,
}

/// `tree-parity alloc`.
///
/// # Errors
/// An unreadable input, or a request line that does not parse.
pub fn alloc(requests: &Path, pob: &Path, bundle: &Path) -> Result<(AllocRecord, bool), String> {
    let request_bytes = read(requests)?;
    let pob_bytes = read(pob)?;
    let theirs: PobAlloc =
        serde_json::from_slice(&pob_bytes).map_err(|e| format!("{}: {e}", pob.display()))?;
    let (tree, manifest) = load_tree(bundle)?;
    let text = String::from_utf8(request_bytes.clone()).map_err(|e| e.to_string())?;
    let requests: Vec<Request> = text
        .lines()
        .map(|l| Request::parse(l).ok_or_else(|| format!("request {l:?}")))
        .collect::<Result<_, _>>()?;
    if requests.len() != theirs.results.len() {
        return Err(format!(
            "{} requests, {} PoB results",
            requests.len(),
            theirs.results.len()
        ));
    }
    let mut r = AllocRecord {
        tree_version: theirs.tree_version.clone(),
        requests_sha256: sha256(&request_bytes),
        pob_alloc_sha256: sha256(&pob_bytes),
        bundle_manifest_sha256: manifest,
        requests: requests.len(),
        ..AllocRecord::default()
    };
    for (i, (q, p)) in requests.iter().zip(&theirs.results).enumerate() {
        let name = |what: &str| format!("request {} ({}): {what}", i + 1, q.target);
        let spec = Spec {
            class: q.class,
            ascendancy: q.ascendancy,
            nodes: q.allocated.iter().copied().collect(),
            masteries: BTreeMap::new(),
        };
        let pob_set: BTreeSet<u32> = p.allocated.iter().copied().collect();
        if pob_set != spec.nodes {
            // PoB pruned or kept something else on import: the rest of the
            // case would compare different specs.
            r.differences.push(Difference {
                what: name("PoB's spec after import"),
                loom: serde_json::json!(spec.nodes),
                pob: serde_json::json!(pob_set),
            });
            continue;
        }
        let points = poe1::points(&tree, &spec);
        if (points.passive, points.ascendancy) == (p.points, p.ascendancy_points) {
            r.points_equal += 1;
        } else {
            r.differences.push(Difference {
                what: name("points"),
                loom: serde_json::json!([points.passive, points.ascendancy]),
                pob: serde_json::json!([p.points, p.ascendancy_points]),
            });
        }
        if p.missing {
            r.differences.push(Difference {
                what: name("target not in PoB's tree"),
                loom: Value::Bool(true),
                pob: Value::Null,
            });
        } else if spec.nodes.contains(&q.target) {
            r.deallocations += 1;
            let ours = poe1::deallocate(&tree, &spec, q.target)
                .map(|x| x.removed)
                .unwrap_or_default();
            let pobs = p.depends.clone().unwrap_or_default();
            let starts = poe1::starts(&tree, &spec);
            if starts.contains(&q.target) {
                // Starts are never deallocated by loom; PoB's depends of a
                // start is everything. Not a case.
                r.deallocations -= 1;
            } else if ours == pobs {
                r.deallocations_equal += 1;
            } else {
                r.differences.push(Difference {
                    what: name("deallocation"),
                    loom: serde_json::json!(ours),
                    pob: serde_json::json!(pobs),
                });
            }
        } else {
            r.allocations += 1;
            let ours = poe1::path(&tree, &spec, q.target).ok();
            // PoB's path runs from the target back; loom's to the target.
            let pobs: Option<Vec<u32>> = p.path.clone().map(|mut v| {
                v.reverse();
                v
            });
            match (&ours, &pobs) {
                (None, None) => r.unreachable_equal += 1,
                (Some(a), Some(b)) if a.len() == b.len() => {
                    if poe1::shortest_path_count(&tree, &spec, q.target, 2) == Ok(1) {
                        if a == b {
                            r.unique_paths_equal += 1;
                        } else {
                            r.differences.push(Difference {
                                what: name("unique path"),
                                loom: serde_json::json!(a),
                                pob: serde_json::json!(b),
                            });
                        }
                    } else {
                        r.tied_paths_length_equal += 1;
                    }
                }
                _ => r.differences.push(Difference {
                    what: name("path"),
                    loom: serde_json::json!(ours),
                    pob: serde_json::json!(pobs),
                }),
            }
        }
    }
    let ok = r.differences.is_empty();
    Ok((r, ok))
}

#[derive(Deserialize)]
struct PobReadback {
    class: Option<String>,
    ascendancy: Option<String>,
    #[serde(deserialize_with = "list")]
    specs: Vec<PobSpec>,
}

#[derive(Deserialize)]
struct PobSpec {
    title: Option<String>,
    class: u32,
    ascendancy: u32,
    #[serde(deserialize_with = "list")]
    nodes: Vec<u32>,
    #[serde(deserialize_with = "list")]
    masteries: Vec<PobMastery>,
    #[serde(default, deserialize_with = "list")]
    jewels: Vec<PobJewel>,
}

#[derive(Deserialize)]
struct PobJewel {
    node: u32,
    item: u32,
}

#[derive(Deserialize)]
struct PobMastery {
    node: u32,
    effect: u32,
}

/// The read-back record.
#[derive(Serialize)]
pub struct ReadbackRecord {
    build_sha256: String,
    pob_readback_sha256: String,
    specs: usize,
    specs_equal: usize,
    build_class_equal: bool,
    differences: Vec<Difference>,
}

/// `tree-parity readback`: the build loom wrote, read by loom's PoB XML
/// reader, against what PoB read from the same file.
///
/// # Errors
/// An unreadable input.
pub fn readback(build: &Path, pob: &Path) -> Result<(ReadbackRecord, bool), String> {
    let build_bytes = read(build)?;
    let pob_bytes = read(pob)?;
    let theirs: PobReadback =
        serde_json::from_slice(&pob_bytes).map_err(|e| format!("{}: {e}", pob.display()))?;
    let text = String::from_utf8(build_bytes.clone()).map_err(|e| e.to_string())?;
    let ours = loom_serialization::legacy::pob_xml::import(&text)
        .map_err(|e| format!("{}: {e}", build.display()))?;
    let mut differences = Vec::new();
    let specs: Vec<_> = ours.passives.specs.typed().collect();
    let mut equal = 0;
    for (i, spec) in specs.iter().enumerate() {
        let Some(p) = theirs.specs.get(i) else {
            differences.push(Difference {
                what: format!("spec {}: not read by PoB", i + 1),
                loom: Value::Bool(true),
                pob: Value::Null,
            });
            continue;
        };
        let attr = |name: &str| {
            spec.attributes
                .iter()
                .find(|a| a.name == name)
                .and_then(|a| a.value.trim().parse::<u32>().ok())
                .unwrap_or(0)
        };
        let nodes: BTreeSet<u32> = spec.nodes.value().iter().map(|n| n.0).collect();
        let masteries: BTreeMap<u32, u32> = spec
            .mastery_effects
            .value()
            .iter()
            .map(|m| (m.node.0, m.effect))
            .collect();
        let pob_nodes: BTreeSet<u32> = p.nodes.iter().copied().collect();
        let pob_masteries: BTreeMap<u32, u32> =
            p.masteries.iter().map(|m| (m.node, m.effect)).collect();
        let name = |what: &str| {
            format!(
                "spec {} ({}): {what}",
                i + 1,
                p.title.as_deref().unwrap_or("untitled")
            )
        };
        let before = differences.len();
        if (attr("classId"), attr("ascendClassId")) != (p.class, p.ascendancy) {
            differences.push(Difference {
                what: name("class and ascendancy"),
                loom: serde_json::json!([attr("classId"), attr("ascendClassId")]),
                pob: serde_json::json!([p.class, p.ascendancy]),
            });
        }
        if nodes != pob_nodes {
            differences.push(Difference {
                what: name("nodes"),
                loom: serde_json::json!(nodes.difference(&pob_nodes).collect::<Vec<_>>()),
                pob: serde_json::json!(pob_nodes.difference(&nodes).collect::<Vec<_>>()),
            });
        }
        let jewels: BTreeMap<u32, u32> = spec
            .jewel_sockets
            .typed()
            .filter_map(|j| Some((j.node.0, j.item?.0)))
            .collect();
        let pob_jewels: BTreeMap<u32, u32> = p.jewels.iter().map(|j| (j.node, j.item)).collect();
        if jewels != pob_jewels {
            differences.push(Difference {
                what: name("jewels"),
                loom: serde_json::json!(jewels),
                pob: serde_json::json!(pob_jewels),
            });
        }
        if masteries != pob_masteries {
            differences.push(Difference {
                what: name("masteries"),
                loom: serde_json::json!(masteries),
                pob: serde_json::json!(pob_masteries),
            });
        }
        if differences.len() == before {
            equal += 1;
        }
    }
    let class = ours.character.class.as_ref().map(|c| c.as_str().to_owned());
    let asc = ours
        .character
        .ascendancy
        .as_ref()
        .map(|c| c.as_str().to_owned());
    let class_equal = class == theirs.class && asc == theirs.ascendancy;
    if !class_equal {
        differences.push(Difference {
            what: "build class and ascendancy".to_owned(),
            loom: serde_json::json!([class, asc]),
            pob: serde_json::json!([theirs.class, theirs.ascendancy]),
        });
    }
    let ok = differences.is_empty();
    Ok((
        ReadbackRecord {
            build_sha256: sha256(&build_bytes),
            pob_readback_sha256: sha256(&pob_bytes),
            specs: specs.len(),
            specs_equal: equal,
            build_class_equal: class_equal,
            differences,
        },
        ok,
    ))
}

#[cfg(test)]
mod tests {
    use super::Request;
    use serde_json::Value;

    /// The committed records (`docs/tree/records/`), by file name.
    fn records() -> Vec<(String, Value)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/tree/records");
        let mut out: Vec<(String, Value)> = std::fs::read_dir(&dir)
            .expect("docs/tree/records")
            .map(|e| e.expect("entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .map(|p| {
                let text = std::fs::read_to_string(&p).expect("record");
                let name = p.file_name().expect("name").to_string_lossy().into_owned();
                (name, serde_json::from_str(&text).expect("record JSON"))
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn count(r: &Value, key: &str) -> u64 {
        r[key]
            .as_u64()
            .unwrap_or_else(|| panic!("{key} is a count"))
    }

    fn hash(r: &Value, key: &str) {
        let h = r[key]
            .as_str()
            .unwrap_or_else(|| panic!("{key} is a string"));
        assert!(
            h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()),
            "{key} is a SHA-256"
        );
    }

    /// CI cannot re-run the records (their inputs are local); it checks
    /// that each one matches and that its counts account for every case.
    #[test]
    fn the_committed_tree_records_should_match_and_add_up() {
        let records = records();
        let names: Vec<&str> = records.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"layout.json") && names.contains(&"alloc.json"),
            "{names:?}"
        );
        assert!(
            names.iter().filter(|n| n.starts_with("readback-")).count() >= 3,
            "{names:?}"
        );
        for (name, r) in &records {
            assert_eq!(r["differences"].as_array().map(Vec::len), Some(0), "{name}");
            match name.as_str() {
                "layout.json" => {
                    hash(r, "pob_layout_sha256");
                    hash(r, "bundle_manifest_sha256");
                    let rounded = r["placed_within_export_rounding"]
                        .as_object()
                        .expect("map")
                        .len() as u64;
                    let removed: u64 = r["removed_by_pob"]
                        .as_object()
                        .expect("map")
                        .values()
                        .map(|v| v.as_u64().expect("count"))
                        .sum();
                    assert_eq!(
                        count(r, "placed_within_tolerance") + rounded + removed,
                        count(r, "nodes"),
                        "{name}"
                    );
                    assert_eq!(
                        count(r, "links_equal") + removed,
                        count(r, "nodes"),
                        "{name}"
                    );
                }
                "alloc.json" => {
                    for k in [
                        "requests_sha256",
                        "pob_alloc_sha256",
                        "bundle_manifest_sha256",
                    ] {
                        hash(r, k);
                    }
                    assert_eq!(
                        count(r, "unique_paths_equal")
                            + count(r, "tied_paths_length_equal")
                            + count(r, "unreachable_equal"),
                        count(r, "allocations"),
                        "{name}"
                    );
                    assert_eq!(
                        count(r, "deallocations_equal"),
                        count(r, "deallocations"),
                        "{name}"
                    );
                    assert_eq!(count(r, "points_equal"), count(r, "requests"), "{name}");
                }
                n if n.starts_with("readback-") => {
                    hash(r, "build_sha256");
                    hash(r, "pob_readback_sha256");
                    assert!(count(r, "specs") > 0, "{name}");
                    assert_eq!(count(r, "specs_equal"), count(r, "specs"), "{name}");
                    assert_eq!(r["build_class_equal"], Value::Bool(true), "{name}");
                }
                other => panic!("an unknown record {other}"),
            }
        }
    }

    #[test]
    fn a_request_line_should_read_back() {
        let r = Request {
            class: 3,
            ascendancy: 1,
            target: 42,
            allocated: vec![1, 2],
        };
        assert_eq!(Request::parse(&r.line()), Some(r));
        let empty = Request {
            class: 0,
            ascendancy: 0,
            target: 7,
            allocated: Vec::new(),
        };
        assert_eq!(Request::parse(&empty.line()), Some(empty));
    }
}

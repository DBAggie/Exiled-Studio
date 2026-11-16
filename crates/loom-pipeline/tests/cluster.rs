//! Cluster jewels through the pipeline (passive tree plan, T-i) on the
//! synthetic tree bundle (made-up rows and texts;
//! `crates/loom-data/tests/fixtures/make_bundles.py`, `cluster_jewels`):
//! a large cluster jewel's lines resolved by the import, read as the
//! jewel ([`loom_pipeline::cluster_jewels`]), its nodes allocated from
//! the class start, and resolved with their stats
//! ([`loom_pipeline::resolve_passives`]).

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the fixtures into memory; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a test over fixed fixtures; a failure should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use loom_calc::PassiveState;
use loom_data::bundle::{Bundle, MemorySource, validate};
use loom_domain::{Build, NodeId};
use loom_parser::import::{BundleData, Resolver};
use loom_tree::Tree;
use loom_tree::poe1::{self, Spec, cluster};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn bundle() -> Bundle {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../loom-data/tests/fixtures/bundle-poe1-tree");
    let mut paths = Vec::new();
    files(&dir, &mut paths);
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        src.insert(rel, fs::read(&p).unwrap());
    }
    assert!(validate::validate(&src).is_valid());
    Bundle::load(&src).unwrap()
}

/// A Test Brute with `nodes` allocated and a large cluster jewel in the
/// large socket 5000: eight passives, two sockets, both notables, the
/// lightning small passive (row 2), 25% increased small passive effect.
fn xml(nodes: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="1" targetVersion="3_0" className="Test Brute" ascendClassName="None" mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>
	<Tree activeSpec="1">
		<Spec treeVersion="3_29" classId="1" ascendClassId="0" nodes="{nodes}">
			<Sockets>
				<Socket nodeId="5000" itemId="1"/>
			</Sockets>
		</Spec>
	</Tree>
	<Skills/>
	<Items activeItemSet="1" useSecondWeaponSet="false">
		<Item id="1">
Rarity: RARE
Loom Test Cluster
Large Test Cluster Jewel
Implicits: 0
Adds +8 Test Cluster Passive Skills
+2 Test Cluster Jewel Sockets
+2 Test Cluster Skill Row
+1 Test Cluster Notable A
+1 Test Cluster Notable B
+25% Test Cluster Small Effect
		</Item>
		<ItemSet id="1" useSecondWeaponSet="false"/>
	</Items>
</PathOfBuilding>
"#
    )
}

fn import(bundle: &Bundle, nodes: &str) -> Build {
    let data = BundleData::new(bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut build = loom_pipeline::read_pob(&xml(nodes)).unwrap();
    loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    build
}

fn joined(nodes: impl IntoIterator<Item = u32>) -> String {
    nodes
        .into_iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn a_socketed_cluster_jewel_should_add_nodes_the_calculation_resolves() {
    let bundle = bundle();
    let tree = Tree::from_bundle(&bundle).unwrap();

    // The jewel as its lines say.
    let build = import(&bundle, "1000");
    let spec = loom_calc::poe1::collect::active_spec(&build).unwrap();
    let jewels = loom_pipeline::cluster_jewels(&build, spec, &tree);
    let jewel = &jewels[&5000];
    assert_eq!(jewel.base, "Test/Items/ClusterLarge");
    assert_eq!(
        (
            jewel.node_count,
            jewel.socket_count,
            jewel.skill,
            jewel.effect
        ),
        (Some(8), Some(2), Some(2), 25)
    );
    assert_eq!(jewel.notables, vec![6010, 6011]);

    // Allocate notable A (65574) and a small passive past it from the
    // Brute's start, through the large socket, on the expanded tree.
    let expanded = cluster::expand(&tree, &cluster::subgraphs(&tree, &jewels));
    let start = Spec {
        class: 1,
        nodes: [1000].into_iter().collect(),
        ..Spec::default()
    };
    let to_notable = poe1::allocate(&expanded, &start, 65574).unwrap();
    let both = poe1::allocate(&expanded, &to_notable.spec, 65575).unwrap();
    let build = import(&bundle, &joined(both.spec.nodes.iter().copied()));

    let resolved = loom_pipeline::resolve_passives(&build, Some(&tree));
    for n in [5000, 65568, 65574, 65575] {
        assert_eq!(resolved.state(NodeId(n)), PassiveState::Connected, "{n}");
    }
    // The notable's stats as the passive's, its effect not scaled; the
    // small passive's with the jewel's 25% increased effect.
    let notable = &resolved.cluster[&NodeId(65574)];
    assert_eq!(
        (
            notable.stats[0].stat.as_str(),
            notable.stats[0].value,
            notable.effect
        ),
        ("test_passive_life", 30, 0)
    );
    let small = &resolved.cluster[&NodeId(65575)];
    assert_eq!(
        (
            small.stats[0].stat.as_str(),
            small.stats[0].value,
            small.effect
        ),
        ("test_passive_life", 5, 25)
    );
    // The socket itself is a tree node, not a cluster node.
    assert!(!resolved.cluster.contains_key(&NodeId(5000)));

    // The calculation takes the nodes' stats: no node is left a reason.
    let data = BundleData::new(&bundle);
    let options = loom_pipeline::options(&build, &data, &Default::default(), None, Some(&tree));
    let result = loom_pipeline::calculate(&build, &bundle, &options, &|| false).unwrap();
    let document = loom_pipeline::document(&result).unwrap();
    assert!(!document.contains("cluster-jewel-node"), "{document}");
    assert!(!document.contains("unconnected-node"), "{document}");
}

#[test]
fn cluster_nodes_without_their_jewel_should_stay_a_reason() {
    let bundle = bundle();
    let tree = Tree::from_bundle(&bundle).unwrap();
    // A cluster node allocated with the socket empty (PoB writes
    // `itemId="0"`).
    let data = BundleData::new(&bundle);
    let resolver = Resolver::new(&data).unwrap();
    let text = xml("1000,65574").replace(r#"itemId="1""#, r#"itemId="0""#);
    let mut build = loom_pipeline::read_pob(&text).unwrap();
    loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    let resolved = loom_pipeline::resolve_passives(&build, Some(&tree));
    assert_eq!(resolved.state(NodeId(65574)), PassiveState::ClusterNode);
    assert!(resolved.cluster.is_empty());
}

//! Cluster jewels on the synthetic tree bundle (passive tree plan, T-i,
//! gate 3's CI part): the subgraphs' node numbers, places, and links as
//! PoB's `BuildSubgraph` gives them for the bundle's made-up tables, worked
//! by hand from PoB's rules (`Classes/PassiveSpec.lua` 1885 to 2271), and
//! the rules walking them. Runs in CI.

#![expect(
    clippy::disallowed_methods,
    clippy::expect_used,
    reason = "a host-side test reads a bundle folder; never part of a core build"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, MemorySource, validate};
use loom_tree::poe1::cluster::{self, Jewel};
use loom_tree::poe1::{self, Spec};
use loom_tree::{Kind, Tree};

fn read_dir(root: &Path, at: &Path, out: &mut MemorySource) {
    for entry in std::fs::read_dir(at).unwrap_or_else(|e| panic!("{}: {e}", at.display())) {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            read_dir(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, std::fs::read(&path).expect("readable"));
        }
    }
}

fn tree() -> Tree {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../loom-data/tests/fixtures/bundle-poe1-tree");
    let mut source = MemorySource::new();
    read_dir(&dir, &dir, &mut source);
    assert!(
        validate::validate(&source).is_valid(),
        "the bundle is valid"
    );
    let bundle = Bundle::load(&source).expect("the bundle loads");
    Tree::from_bundle(&bundle).expect("a passive tree")
}

/// A large jewel with two sockets and both notables; a medium jewel in
/// its first socket (5001) with one socket and a notable; a small jewel in
/// that one's socket (5003) with a notable.
fn jewels() -> BTreeMap<u32, Jewel> {
    BTreeMap::from([
        (
            5000,
            Jewel {
                base: "Test/Items/ClusterLarge".to_owned(),
                skill: Some(2),
                node_count: Some(8),
                socket_count: Some(2),
                notables: vec![6011, 6010],
                ..Jewel::default()
            },
        ),
        (
            5001,
            Jewel {
                base: "Test/Items/ClusterMedium".to_owned(),
                skill: Some(1),
                node_count: Some(5),
                socket_count: Some(1),
                notables: vec![6011],
                effect: 25,
                ..Jewel::default()
            },
        ),
        (
            5003,
            Jewel {
                base: "Test/Items/ClusterSmall".to_owned(),
                skill: Some(0),
                node_count: Some(3),
                notables: vec![6010],
                ..Jewel::default()
            },
        ),
    ])
}

fn ids(g: &cluster::Subgraph) -> Vec<(u32, Kind, u32)> {
    g.nodes
        .iter()
        .map(|n| (n.id, n.kind, n.orbit_index))
        .collect()
}

#[test]
fn subgraphs_should_number_and_place_nodes_as_pob() {
    let t = tree();
    let g = cluster::subgraphs(&t, &jewels());
    assert_eq!(g.len(), 3);

    // Large: id 0x10000 + index 0 << 6, size 2 << 4 = 65568; start index 5,
    // the 12 places on the 16-slot orbit 3.
    let large = &g[0];
    assert_eq!(
        (large.socket, large.id, large.entrance),
        (5000, 65568, 65568)
    );
    assert_eq!(
        ids(large),
        vec![
            (65580, Kind::Mastery, 0),
            (5001, Kind::ClusterSocket, 12),
            (5008, Kind::ClusterSocket, 1),
            (65574, Kind::Notable, 15),
            (65578, Kind::Notable, 4),
            (65568, Kind::Normal, 7),
            (65570, Kind::Normal, 9),
            (65575, Kind::Normal, 0),
            (65573, Kind::Normal, 13),
        ]
    );
    // The notables in their stats' order: A (order 10) first.
    assert_eq!(large.nodes[3].name, "Test Cluster Notable A");
    assert_eq!(large.nodes[5].name, "Test Small Lightning");
    let mut links = large.links.clone();
    links.sort_unstable();
    assert_eq!(
        links,
        vec![
            (5001, 65570),
            (65568, 5000),
            (65568, 65578),
            (65570, 65568),
            (65573, 5001),
            (65574, 65573),
            (65575, 65574),
            (5008, 65575),
            (65578, 5008),
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
    );

    // Medium in 5001: id 0x10000 + 0 << 9, size 1 << 4 = 65552; five nodes
    // move the small place 4 to 3; start index 3.
    let medium = &g[1];
    assert_eq!((medium.socket, medium.id), (5001, 65552));
    assert_eq!(
        ids(medium),
        vec![
            (65552 + 12, Kind::Mastery, 0),
            (5003, Kind::ClusterSocket, 12),
            (65562, Kind::Notable, 1),
            (65552, Kind::Normal, 4),
            (65560, Kind::Normal, 15),
            (65555, Kind::Normal, 8),
        ]
    );
    assert!(
        medium
            .nodes
            .iter()
            .all(|n| n.kind != Kind::Normal || n.effect == 25)
    );

    // Small in 5003: no size bits; a ring of 6 on the 6-slot orbit 1, not
    // closed.
    let small = &g[2];
    assert_eq!((small.socket, small.id), (5003, 65536));
    assert_eq!(
        ids(small),
        vec![
            (65540, Kind::Notable, 5),
            (65536, Kind::Normal, 1),
            (65538, Kind::Normal, 3),
        ]
    );
    assert_eq!(
        small.links,
        vec![(65538, 65536), (65540, 65538), (65536, 5003)]
    );
}

#[test]
fn a_keystone_jewel_should_add_only_its_keystone() {
    let t = tree();
    let jewels = BTreeMap::from([(
        5000,
        Jewel {
            base: "Test/Items/ClusterLarge".to_owned(),
            keystone: Some(6020),
            ..Jewel::default()
        },
    )]);
    let g = cluster::subgraphs(&t, &jewels);
    assert_eq!(g.len(), 1);
    assert_eq!(ids(&g[0]), vec![(65568, Kind::Keystone, 1)]);
    assert_eq!(g[0].links, vec![(65568, 5000)]);
}

#[test]
fn a_jewel_that_adds_nothing_or_names_a_missing_notable_should_build_nothing() {
    let t = tree();
    let no_count = Jewel {
        base: "Test/Items/ClusterLarge".to_owned(),
        skill: Some(2),
        ..Jewel::default()
    };
    assert!(cluster::subgraphs(&t, &BTreeMap::from([(5000, no_count)])).is_empty());
    // A skill of another size: PoB drops it, so the jewel is not valid.
    let wrong_size = Jewel {
        base: "Test/Items/ClusterLarge".to_owned(),
        skill: Some(0),
        node_count: Some(8),
        ..Jewel::default()
    };
    assert!(cluster::subgraphs(&t, &BTreeMap::from([(5000, wrong_size)])).is_empty());
    let missing = Jewel {
        base: "Test/Items/ClusterLarge".to_owned(),
        skill: Some(2),
        node_count: Some(8),
        notables: vec![9999],
        ..Jewel::default()
    };
    assert!(cluster::subgraphs(&t, &BTreeMap::from([(5000, missing)])).is_empty());
    // A jewel in a nested socket alone is never built (only from a large).
    let nested = jewels().remove(&5001).expect("medium");
    assert!(cluster::subgraphs(&t, &BTreeMap::from([(5001, nested)])).is_empty());
}

#[test]
fn smalls_that_grant_nothing_should_have_no_stats_and_a_node_count_clamped() {
    let t = tree();
    let jewel = Jewel {
        base: "Test/Items/ClusterLarge".to_owned(),
        node_count: Some(40),
        smalls_nothing: true,
        ..Jewel::default()
    };
    let g = cluster::subgraphs(&t, &BTreeMap::from([(5000, jewel)]));
    // Clamped to the base's 12; no skill, so no mastery.
    assert_eq!(g[0].nodes.len(), 12);
    assert!(
        g[0].nodes
            .iter()
            .all(|n| n.kind == Kind::Normal && n.stats.is_empty() && n.name == "Nothingness")
    );
}

#[test]
fn the_expanded_tree_should_place_link_and_allocate_cluster_nodes() {
    let t = tree();
    let g = cluster::subgraphs(&t, &jewels());
    let x = cluster::expand(&t, &g);
    // The cluster area's own links are gone; the large socket links to the
    // entrance.
    assert!(x.node(5005).expect("proxy").neighbours.is_empty());
    assert!(x.node(5000).expect("socket").neighbours.contains(&65568));
    // A socket the jewel adds stands in for the tree's nested socket.
    let s = x.node(5001).expect("socket");
    assert!(s.modelled && !s.cluster_area);
    assert_eq!(s.kind, Kind::ClusterSocket);
    assert_eq!(s.neighbours, vec![65552, 65570, 65573]);
    // Placed on orbit 3 of the large proxy's group (0, 1300), radius 335,
    // slot 7 of 16 at 157.5 degrees.
    let (px, py) = x.node(65568).expect("entrance").position.expect("placed");
    let a = 157.5_f64.to_radians();
    assert!((px - 335.0 * a.sin()).abs() < 1e-9 && (py - (1300.0 - 335.0 * a.cos())).abs() < 1e-9);
    assert!(!x.node(65580).expect("mastery").modelled);

    // From the Brute's start through the large socket into the cluster:
    // allocating notable A takes the shortest path into the ring.
    let spec = Spec {
        class: 1,
        ascendancy: 1,
        nodes: [1000, 3000].into_iter().collect(),
        ..Spec::default()
    };
    let a = poe1::allocate(&x, &spec, 65574).expect("reachable");
    assert!(a.path.contains(&5000) && a.path.contains(&65568));
    assert_eq!(a.path.last(), Some(&65574));
    // The calculation's rule reaches it too.
    let c = poe1::connected_to_start(&x, a.spec.nodes.iter());
    assert!(c.contains(&65574));
    // Deallocating the large socket removes the cluster nodes after it.
    let r = poe1::deallocate(&x, &a.spec, 5000).expect("allocated");
    assert!(r.removed.contains(&65574));
}

#[test]
fn a_tree_without_cluster_data_should_expand_to_itself() {
    let mut t = tree();
    t.cluster_jewels = None;
    assert!(cluster::subgraphs(&t, &jewels()).is_empty());
    assert_eq!(cluster::expand(&t, &[]), t);
}

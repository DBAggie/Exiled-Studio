//! The rules on the synthetic tree bundle (passive tree plan, T1a):
//! `crates/loom-data/tests/fixtures/bundle-poe1-tree`, written by
//! `make_bundles.py`. Runs in CI.

#![expect(
    clippy::disallowed_methods,
    clippy::expect_used,
    reason = "a host-side test reads a bundle folder; never part of a core build"
)]

use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, MemorySource, validate};
use loom_tree::poe1::radius::{Centre, Leap, Ring};
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

fn brute(nodes: &[u32]) -> Spec {
    Spec {
        class: 1,
        ascendancy: 1,
        nodes: [1000, 3000]
            .into_iter()
            .chain(nodes.iter().copied())
            .collect(),
        masteries: Default::default(),
        leaps: Vec::new(),
    }
}

#[test]
fn the_synthetic_tree_should_have_every_kind_placed_and_modelled() {
    let t = tree();
    assert!(t.placed());
    assert_eq!(t.classes.len(), 2);
    assert_eq!(t.class(1).expect("Test Brute").ascendancies.len(), 2);
    let kind = |id: u32| t.node(id).map(|n| n.kind);
    assert_eq!(kind(1000), Some(Kind::ClassStart));
    assert_eq!(kind(3000), Some(Kind::AscendancyStart));
    assert_eq!(kind(11040), Some(Kind::Keystone));
    assert_eq!(kind(10023), Some(Kind::Notable));
    assert_eq!(kind(10029), Some(Kind::Mastery));
    assert_eq!(kind(11028), Some(Kind::JewelSocket));
    // The large socket is a plain socket on the tree; the nested ones,
    // with a proxy next to them, are cluster sockets.
    assert_eq!(kind(5000), Some(Kind::JewelSocket));
    assert_eq!(kind(5001), Some(Kind::ClusterSocket));
    // The cluster area: nested sockets and proxies, never modelled.
    for id in [5001, 5002, 5003, 5004] {
        let n = t.node(id).expect("in the tree");
        assert!(n.cluster_area && !n.modelled, "{id}");
    }
    assert!(t.node(5000).expect("socket").modelled);
    assert!(t.nodes.values().all(|n| n.position.is_some()));
    assert_eq!(t.jewel_radii.len(), 2);
}

#[test]
fn allocation_should_cross_the_tree_and_take_a_mastery() {
    let t = tree();
    let s = brute(&[]);
    // From the Brute's start to the notable of group (0, 2).
    let a = poe1::allocate(&t, &s, 10023).expect("reachable");
    assert_eq!(a.path.last(), Some(&10023));
    let m = poe1::set_mastery(&t, &a.spec, 10029, 4243).expect("a connected group");
    assert_eq!(m.masteries.get(&10029), Some(&4243));
    // The Sage's start is a start, never a target.
    assert!(poe1::allocate(&t, &s, 2000).is_err());
    // A choice option replaces the other.
    let one = poe1::allocate(&t, &s, 3004).expect("the Champion's option A");
    let other = poe1::allocate(&t, &one.spec, 3005).expect("option B");
    assert!(other.spec.nodes.contains(&3005) && !other.spec.nodes.contains(&3004));
    // Deallocating a node on the path drops what hangs from it.
    let r = poe1::deallocate(&t, &m, a.path[0]).expect("allocated");
    assert!(!r.spec.nodes.contains(&10023) && !r.spec.masteries.contains_key(&10029));
}

#[test]
fn conversion_should_keep_only_what_the_tree_gives() {
    let t = tree();
    let a = poe1::allocate(&t, &brute(&[]), 10023).expect("reachable");
    let m = poe1::set_mastery(&t, &a.spec, 10029, 4243).expect("a connected group");
    // A spec the tree has whole converts to itself, its mastery kept.
    assert_eq!(poe1::convert(&t, &m), m);
    // A mastery whose effect the tree does not know goes, node and choice.
    let mut unknown = m.clone();
    unknown.masteries.insert(10029, 9999);
    let c = poe1::convert(&t, &unknown);
    assert!(!c.nodes.contains(&10029) && c.masteries.is_empty());
    // So does a mastery with no choice.
    let mut bare = m.clone();
    bare.masteries.clear();
    let c = poe1::convert(&t, &bare);
    assert!(!c.nodes.contains(&10029));
    assert_eq!(c.nodes.len(), m.nodes.len() - 1);
    // A node the tree lacks goes, and so does what only it connected.
    let mut gone = m.clone();
    gone.nodes.insert(999_999);
    gone.nodes.remove(&a.path[0]);
    let c = poe1::convert(&t, &gone);
    assert!(!c.nodes.contains(&999_999) && !c.nodes.contains(&10023));
    assert!(c.nodes.contains(&1000));
    // An ascendancy the class lacks is cleared, its start with it.
    let mut other = m.clone();
    other.ascendancy = 99;
    let c = poe1::convert(&t, &other);
    assert_eq!(c.ascendancy, 0);
    assert!(!c.nodes.contains(&3000));
}

#[test]
fn a_radius_jewel_should_allocate_a_node_near_its_socket_alone() {
    let t = tree();
    let to_socket = poe1::allocate(&t, &brute(&[]), 11028).expect("the socket");
    let mut s = to_socket.spec;
    // The neighbouring group's nodes are 300 away; the made-up Small
    // radius is 960 tree units (800 in the table, times the tree scale).
    let target = 11034;
    let before = poe1::path(&t, &s, target).expect("reachable");
    assert!(before.len() > 1, "{before:?}");
    s.leaps.push(Leap {
        socket: 11028,
        ring: t.radius_named("Small").expect("the bundle's radius"),
        centre: Centre::Socket,
        keystones_only: false,
    });
    assert_eq!(
        poe1::path(&t, &s, target).expect("in the radius"),
        vec![target]
    );
    assert_eq!(
        t.ring(2),
        Some(Ring {
            inner: 1320,
            outer: 1680
        })
    );
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(256))]

    /// Review B3, plan T-g and gate item 4: no edit but a reset removes a
    /// node the editor keeps (one the tree lacks, or a cluster area node
    /// it does not model), whatever edits come before it.
    #[test]
    fn no_edit_should_remove_a_kept_node(
        edits in proptest::collection::vec((0_u8..5, proptest::prelude::any::<proptest::sample::Index>()), 1..24)
    ) {
        let t = tree();
        let ids: Vec<u32> = t.nodes.keys().copied().collect();
        let effects: Vec<u32> = t.mastery_effects.values().flatten().copied().collect();
        // Kept: two nodes no tree has, and the unmodelled cluster area.
        let kept: Vec<u32> = [900_001, 900_002]
            .into_iter()
            .chain(t.nodes.values().filter(|n| !n.modelled).map(|n| n.id))
            .collect();
        proptest::prop_assert!(kept.len() > 2);
        let mut spec = brute(&kept);
        for (op, at) in edits {
            let node = *at.get(&ids);
            let next = match op {
                0 => poe1::allocate(&t, &spec, node).map(|a| a.spec),
                1 => poe1::deallocate(&t, &spec, node).map(|r| r.spec),
                2 => poe1::set_mastery(&t, &spec, node, *at.get(&effects)),
                3 => poe1::select_ascendancy(&t, &spec, node % 3).map(|r| r.spec),
                _ => poe1::select_class(&t, &spec, 1 + node % 2).map(|r| r.spec),
            };
            if let Ok(next) = next {
                spec = next;
            }
            for k in &kept {
                proptest::prop_assert!(spec.nodes.contains(k), "{k} removed by edit {op} on {node}");
            }
        }
    }
}

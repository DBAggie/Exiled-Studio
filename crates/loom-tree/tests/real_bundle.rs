//! The rules on a real bundle (maintainer-run: `LOOM_TREE_BUNDLE`, else
//! `local/tree/bundle-poe1-tree-a`, made by `loom-parity-cli tree-layout`).
//!
//! `cargo test -p loom-tree --test real_bundle -- --ignored`

#![expect(
    clippy::disallowed_methods,
    clippy::expect_used,
    reason = "a host-side test reads a bundle folder; never part of a core build"
)]

use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, MemorySource};
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
    let dir = std::env::var_os("LOOM_TREE_BUNDLE").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/tree/bundle-poe1-tree-a"),
        PathBuf::from,
    );
    let mut source = MemorySource::new();
    read_dir(&dir, &dir, &mut source);
    let bundle = Bundle::load(&source).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let tables = bundle.passives.as_ref().expect("a passive tree");
    Tree::from_tables(tables, bundle.tree_layout.as_ref())
}

#[test]
#[ignore = "needs a real bundle with the layout (maintainer-run)"]
fn the_real_tree_should_be_placed_modelled_and_allocatable_from_every_class() {
    let t = tree();
    assert!(t.placed(), "the bundle has the layout");
    assert_eq!(t.classes.len(), 7);
    let modelled = t.nodes.values().filter(|n| n.modelled).count();
    let kept = t.nodes.len() - modelled;
    println!("nodes {}, modelled {modelled}, kept {kept}", t.nodes.len());
    // Blight's anointment notables: flagged, placed, never allocated.
    let anoint: Vec<_> = t.nodes.values().filter(|n| n.anoint_only).collect();
    assert!(!anoint.is_empty(), "the 3.29 tree has anointment notables");
    assert!(
        anoint
            .iter()
            .all(|n| !n.modelled && n.neighbours.is_empty())
    );
    assert!(
        t.nodes.get(&62596).is_some_and(|n| n.anoint_only),
        "Mystic Talents"
    );
    assert!(t.nodes.values().all(|n| n.position.is_some()));
    let kinds = |k: Kind| {
        t.nodes
            .values()
            .filter(|n| n.kind == k && n.modelled)
            .count()
    };
    println!(
        "keystones {}, notables {}, masteries {}, sockets {}, cluster sockets {}",
        kinds(Kind::Keystone),
        kinds(Kind::Notable),
        kinds(Kind::Mastery),
        kinds(Kind::JewelSocket),
        kinds(Kind::ClusterSocket)
    );
    // Every keystone is reachable from every class, and its path is
    // connected at each step.
    let keystones: Vec<u32> = t
        .nodes
        .values()
        .filter(|n| n.kind == Kind::Keystone && n.modelled && n.ascendancy.is_none())
        .map(|n| n.id)
        .collect();
    for class in &t.classes {
        let spec = Spec {
            class: class.number,
            ascendancy: 0,
            nodes: [class.start].into_iter().collect(),
            masteries: Default::default(),
            leaps: Vec::new(),
        };
        for &k in &keystones {
            let a = poe1::allocate(&t, &spec, k)
                .unwrap_or_else(|e| panic!("{}: keystone {k}: {e}", class.name));
            assert_eq!(a.path.last(), Some(&k));
            let back = poe1::deallocate(&t, &a.spec, a.path[0]).expect("allocated");
            assert_eq!(
                back.spec, spec,
                "deallocating the first step drops the path"
            );
        }
        for asc in &class.ascendancies {
            let s = poe1::select_ascendancy(&t, &spec, asc.number).expect("its ascendancy");
            let notables: Vec<u32> = t
                .nodes
                .values()
                .filter(|n| {
                    n.ascendancy.as_deref() == Some(asc.id.as_str()) && n.kind == Kind::Notable
                })
                .map(|n| n.id)
                .collect();
            assert!(!notables.is_empty(), "{} has notables", asc.name);
            for n in notables {
                poe1::allocate(&t, &s.spec, n).unwrap_or_else(|e| panic!("{}: {n}: {e}", asc.name));
            }
        }
        assert!(poe1::url::encode(&t, &spec, 0).is_some());
    }
}

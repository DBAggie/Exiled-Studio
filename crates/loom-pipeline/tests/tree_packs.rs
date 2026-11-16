//! Older tree versions through the pipeline (passive tree plan, T4) on the
//! synthetic tree bundle, whose `trees/3_28.json` is the bundle's tree with
//! node 10000 granting 7 instead of 5 and node 10001's line unread
//! (`crates/loom-data/tests/fixtures/make_bundles.py`, `tree_pack`): a
//! spec of 3_28 is calculated on the pack ([`loom_pipeline::pick_tree`]),
//! one of the data's version on the data's tree.

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

use loom_data::bundle::{Bundle, MemorySource, validate};
use loom_domain::{Build, NodeId, TreeVersion};
use loom_parser::import::{BundleData, Resolver};
use loom_tree::Tree;

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

/// A Test Brute of tree version `version` with the path from its start to
/// node 10000 allocated (1000, 11004, 11005, 10002, 10001, 10000).
fn import(bundle: &Bundle, version: &str) -> Build {
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="1" targetVersion="3_0" className="Test Brute" ascendClassName="None" mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>
	<Tree activeSpec="1">
		<Spec treeVersion="{version}" classId="1" ascendClassId="0" clusterHashFormatVersion="2" nodes="1000,11004,11005,10002,10001,10000"/>
	</Tree>
	<Skills/>
	<Items activeItemSet="1" useSecondWeaponSet="false">
		<ItemSet id="1" useSecondWeaponSet="false"/>
	</Items>
</PathOfBuilding>
"#
    );
    let data = BundleData::new(bundle);
    let resolver = Resolver::new(&data).unwrap();
    let mut build = loom_pipeline::read_pob(&text).unwrap();
    loom_pipeline::resolve(&resolver, &data, &mut build).unwrap();
    build
}

/// The calculation document of `build` on the tree the pipeline picks.
fn calculated(
    bundle: &Bundle,
    build: &Build,
    own: &Tree,
    packs: &std::collections::BTreeMap<String, Tree>,
) -> String {
    let data = BundleData::new(bundle);
    let (tree, version) =
        loom_pipeline::pick_tree(build, Some(own), Some(TreeVersion::new("3_29")), packs);
    let options = loom_pipeline::options(build, &data, &Default::default(), version, tree);
    let result = loom_pipeline::calculate(build, bundle, &options, &|| false).unwrap();
    loom_pipeline::document(&result).unwrap()
}

#[test]
fn the_bundle_should_carry_its_pack_as_a_tree() {
    let bundle = bundle();
    assert_eq!(bundle.tree_packs.keys().collect::<Vec<_>>(), ["3_28"]);
    let own = Tree::from_bundle(&bundle).unwrap();
    let packs = loom_pipeline::pack_trees(&bundle, &own);
    let tree = &packs["3_28"];
    assert_eq!(tree.version.as_deref(), Some("3_28"));
    assert_eq!(tree.nodes[&10000].stats[0].value, 7);
    assert_eq!(tree.unresolved[&10001], ["Test Old Line"]);
    // The classes are the bundle's.
    assert_eq!(tree.classes, own.classes);
    assert_eq!(own.version, None);
}

#[test]
fn a_spec_of_the_packs_version_should_calculate_on_the_pack() {
    let bundle = bundle();
    let own = Tree::from_bundle(&bundle).unwrap();
    let packs = loom_pipeline::pack_trees(&bundle, &own);
    let build = import(&bundle, "3_28");

    let (tree, version) =
        loom_pipeline::pick_tree(&build, Some(&own), Some(TreeVersion::new("3_29")), &packs);
    assert_eq!(version, Some(TreeVersion::new("3_28")));
    let resolved = loom_pipeline::resolve_passives(&build, tree);
    let pack = resolved.pack.as_ref().unwrap();
    assert_eq!(pack.passives[&NodeId(10000)].stats[0].value, 7);
    assert_eq!(pack.passives[&NodeId(10001)].unresolved, ["Test Old Line"]);
    assert_eq!(pack.mastery_effects[&4243].stats[0].value, 22);

    // The pack's version passes the engine's check; its unread line is a
    // reason on its node.
    let document = calculated(&bundle, &build, &own, &packs);
    assert!(!document.contains("tree-version"), "{document}");
    assert!(document.contains("unresolved-tree-line"), "{document}");
    assert!(!document.contains("unconnected-node"), "{document}");
}

#[test]
fn a_spec_of_another_version_should_keep_the_datas_tree() {
    let bundle = bundle();
    let own = Tree::from_bundle(&bundle).unwrap();
    let packs = loom_pipeline::pack_trees(&bundle, &own);

    // The data's own version: its tree, no pack stats.
    let build = import(&bundle, "3_29");
    let (tree, version) =
        loom_pipeline::pick_tree(&build, Some(&own), Some(TreeVersion::new("3_29")), &packs);
    assert_eq!(version, Some(TreeVersion::new("3_29")));
    assert!(loom_pipeline::resolve_passives(&build, tree).pack.is_none());
    let document = calculated(&bundle, &build, &own, &packs);
    assert!(!document.contains("unresolved-tree-line"), "{document}");
    assert!(!document.contains("tree-version"), "{document}");

    // A version the bundle has no pack of: the data's tree, and the
    // engine's version check gives its reason.
    let build = import(&bundle, "3_25");
    let (tree, version) =
        loom_pipeline::pick_tree(&build, Some(&own), Some(TreeVersion::new("3_29")), &packs);
    assert!(std::ptr::eq(tree.unwrap(), &own));
    assert_eq!(version, Some(TreeVersion::new("3_29")));
    let document = calculated(&bundle, &build, &own, &packs);
    assert!(document.contains(r#""tree-version""#), "{document}");
}

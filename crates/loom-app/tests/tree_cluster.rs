//! Cluster sockets and version conversion in the tree editor (passive tree
//! plan, T5b). Cluster jewels on the synthetic tree bundle
//! (`crates/loom-data/tests/fixtures/make_bundles.py`, `tree_bundle` and
//! `cluster_jewels`): its large socket 5000 takes the made-up "Large Test
//! Cluster Jewel", whose nodes then allocate as any other and go when the
//! jewel does. Conversion on the plain synthetic bundle: a 3_25 spec
//! copied onto the data's 3_29 as PoB's `ConvertToVersion` does.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};

use loom_app::tree::Edit;
use loom_app::{Session, api};
use loom_domain::TreeVersion;
use loom_persistence::Store;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn view(s: &Session) -> api::TreeView {
    s.snapshot(api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    })
    .build
    .unwrap()
    .tree
    .unwrap()
}

/// A Test Brute with only its start allocated and two jewels: a large
/// cluster jewel (item 1) and a medium one (item 2).
const CLUSTER_BUILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="1" targetVersion="3_0" className="Test Brute" ascendClassName="None" mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>
	<Tree activeSpec="1">
		<Spec treeVersion="3_29" classId="1" ascendClassId="0" clusterHashFormatVersion="2" nodes="1000"/>
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
		<Item id="2">
Rarity: RARE
Loom Test Medium
Medium Test Cluster Jewel
Implicits: 0
Adds +4 Test Cluster Passive Skills
		</Item>
		<ItemSet id="1" useSecondWeaponSet="false"/>
	</Items>
</PathOfBuilding>
"#;

fn cluster_session() -> Session {
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(
        &root().join("crates/loom-data/tests/fixtures/bundle-poe1-tree"),
    ));
    s.import_text(CLUSTER_BUILD, None).unwrap();
    s
}

#[test]
fn a_socketed_cluster_jewel_should_add_nodes_the_editor_allocates() {
    let mut s = cluster_session();
    // The large socket, allocated with its path; nothing generated yet.
    s.edit_tree(1, Edit::Allocate(5000)).unwrap();
    let before = view(&s);
    assert!(before.cluster_nodes.is_empty());
    assert!(
        before
            .cluster_sockets
            .iter()
            .any(|c| c.node == 5000 && c.size == api::ClusterSize::Large)
    );

    s.socket_jewel(1, 5000, Some(1)).unwrap();
    let socketed = view(&s);
    assert!(!socketed.cluster_nodes.is_empty());
    let generated: Vec<&api::TreeNodeView> = socketed
        .cluster_nodes
        .iter()
        .filter(|n| n.id >= 65536)
        .collect();
    assert!(!generated.is_empty());
    assert!(
        generated
            .iter()
            .all(|n| n.position.is_some() && n.modelled || n.kind == api::TreeNodeKind::Mastery)
    );
    // The nested medium sockets the jewel places take medium jewels.
    assert!(
        socketed
            .cluster_sockets
            .iter()
            .any(|c| c.size == api::ClusterSize::Medium)
    );

    // Notable A (65574) allocates through the jewel's nodes.
    let notable = generated
        .iter()
        .find(|n| n.kind == api::TreeNodeKind::Notable)
        .unwrap()
        .id;
    s.edit_tree(1, Edit::Allocate(notable)).unwrap();
    let allocated = view(&s);
    assert!(allocated.nodes.contains(&notable));
    assert!(
        allocated.kept.iter().all(|n| *n < 65536),
        "{:?}",
        allocated.kept
    );

    // The jewel out: its nodes go, the main tree's stay.
    s.socket_jewel(1, 5000, None).unwrap();
    let emptied = view(&s);
    assert!(
        emptied.nodes.iter().all(|n| *n < 65536),
        "{:?}",
        emptied.nodes
    );
    assert!(emptied.nodes.contains(&5000));
    assert!(emptied.cluster_nodes.is_empty());

    // Each change is one undo step.
    s.undo().unwrap();
    assert!(view(&s).nodes.contains(&notable));
}

#[test]
fn a_cluster_jewel_should_go_only_where_its_size_fits() {
    let mut s = cluster_session();
    s.edit_tree(1, Edit::Allocate(5000)).unwrap();
    s.socket_jewel(1, 5000, Some(1)).unwrap();
    let medium = view(&s)
        .cluster_sockets
        .iter()
        .find(|c| c.size == api::ClusterSize::Medium)
        .unwrap()
        .node;
    s.edit_tree(1, Edit::Allocate(medium)).unwrap();
    // A large jewel does not fit a medium socket; a medium one does.
    let err = s.socket_jewel(1, medium, Some(1)).unwrap_err();
    assert_eq!(err.code, "APP_TREE");
    assert!(err.message.contains("size"), "{}", err.message);
    s.socket_jewel(1, medium, Some(2)).unwrap();
    assert!(
        view(&s)
            .jewels
            .iter()
            .any(|j| j.node == medium && j.item == 2)
    );
    // A cluster jewel never goes in a plain jewel socket (11028, on row 1).
    s.edit_tree(1, Edit::Allocate(11028)).unwrap();
    let err = s.socket_jewel(1, 11028, Some(2)).unwrap_err();
    assert!(
        err.message.contains("does not take cluster jewels"),
        "{}",
        err.message
    );
}

fn plain_session(version: &str) -> Session {
    let text = std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap()
    .replace(r#"className="Scion""#, r#"className="Test Brute""#)
    .replace(
        r#"<Spec treeVersion="3_29" classId="0" ascendClassId="0" nodes=""/>"#,
        &format!(
            r#"<Spec title="Old" treeVersion="{version}" classId="1" ascendClassId="0" nodes="100,101,102,999"/>"#
        ),
    );
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(
        &root().join("crates/loom-data/tests/fixtures/bundle-poe1"),
    ));
    s.import_text(&text, None).unwrap();
    s
}

#[test]
fn converting_a_spec_should_copy_it_onto_the_datas_version() {
    let s = plain_session("3_25");
    let build = &s.state().unwrap().build;
    let tree = s.tree().unwrap();
    let data = TreeVersion::new("3_29");
    // The preview: the node the tree lacks goes, the rest connect.
    let (copy, dropped) = loom_app::tree::converted(build, 1, &tree, Some(&data)).unwrap();
    assert_eq!(dropped, [999]);
    assert_eq!(copy.tree_version, Some(data.clone()));
    let nodes: Vec<u32> = copy.nodes.value().iter().map(|n| n.0).collect();
    assert_eq!(nodes, [100, 101, 102]);

    // The command: the copy after the old spec, active; the old one kept.
    let mut build = build.clone();
    let (id, dropped) = loom_app::tree::convert_spec(&mut build, 1, &tree, Some(&data)).unwrap();
    assert_eq!((id, dropped), (2, vec![999]));
    let specs: Vec<(u32, Option<String>)> = build
        .passives
        .specs
        .typed()
        .map(|s| {
            (
                s.id.0,
                s.tree_version.as_ref().map(|v| v.as_str().to_owned()),
            )
        })
        .collect();
    assert_eq!(
        specs,
        [(1, Some("3_25".to_owned())), (2, Some("3_29".to_owned()))]
    );
    assert_eq!(build.passives.active.map(|a| a.0), Some(2));
    assert_eq!(
        loom_app::tree::spec_of(&build, 2).unwrap().title.as_deref(),
        Some("Old")
    );

    // Converting a spec that is not the last: the copy follows it and
    // takes the next ID, the later spec moves up one.
    let mut again = build.clone();
    let (id, _) = loom_app::tree::convert_spec(&mut again, 1, &tree, Some(&data)).unwrap();
    assert_eq!(id, 2);
    assert_eq!(again.passives.active.map(|a| a.0), Some(2));
    let ids: Vec<u32> = again.passives.specs.typed().map(|s| s.id.0).collect();
    assert_eq!(ids, [1, 2, 3]);
    let active = loom_app::tree::spec_of(&again, 2).unwrap();
    assert_eq!(active.tree_version, Some(data.clone()));

    // Nothing to convert on the data's version, or with it unknown.
    let err = loom_app::tree::converted(&build, 2, &tree, Some(&data)).unwrap_err();
    assert_eq!(err.code, "APP_TREE");
    assert!(loom_app::tree::converted(&build, 1, &tree, None).is_err());
}

/// [`plain_session`] with the data's tree version set to 3_29 (the
/// fixture's data version names none).
fn known_session(version: &str) -> Session {
    let mut s = plain_session(version);
    let mut data =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe1"))
            .unwrap();
    data.tree_version = Some(TreeVersion::new("3_29"));
    s.set_data(Ok(data));
    s
}

#[test]
fn the_view_should_offer_conversion_only_for_another_version() {
    assert_eq!(
        view(&known_session("3_25")).convert,
        Some(api::TreeConversion {
            version: "3_29".to_owned(),
            dropped: vec![999],
        })
    );
    assert_eq!(view(&known_session("3_29")).convert, None);
    // A data version not known: nothing to offer.
    assert_eq!(view(&plain_session("3_25")).convert, None);
}

#[test]
fn the_convert_command_should_be_one_undoable_change() {
    let mut s = known_session("3_25");
    s.convert_tree_version(1).unwrap();
    let converted = view(&s);
    assert_eq!(converted.spec, 2);
    assert_eq!(converted.locked, None);
    assert_eq!(converted.nodes, [100, 101, 102]);
    // The converted spec edits.
    s.edit_tree(2, Edit::Deallocate(102)).unwrap();
    s.undo().unwrap();
    s.undo().unwrap();
    let back = view(&s);
    assert_eq!(back.spec, 1);
    assert!(back.convert.is_some());

    // The data's version not known: refused.
    let mut s = plain_session("3_25");
    let err = s.convert_tree_version(1).unwrap_err();
    assert_eq!(err.code, "APP_TREE");
    assert!(err.message.contains("not known"), "{}", err.message);
}

/// The e2e's cluster steps (`ui/e2e/workflow.mjs`) on the build its
/// run scripts make: the synthetic build as Test Brute with a large
/// cluster jewel as item 5. The export keeps the socket and the node.
#[test]
fn the_e2e_cluster_steps_should_export_the_socket_and_the_node() {
    let cluster = "<Item id=\"5\">\nRarity: RARE\nLoom Test Cluster\nLarge Test Cluster Jewel\nImplicits: 0\nAdds +8 Test Cluster Passive Skills\n+2 Test Cluster Jewel Sockets\n+2 Test Cluster Skill Row\n+1 Test Cluster Notable A\n+1 Test Cluster Notable B\n+25% Test Cluster Small Effect\n</Item>\n\t\t<ItemSet id=\"1\"";
    let text = std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap()
    .replace(r#"className="Scion""#, r#"className="Test Brute""#)
    .replace(
        r#"classId="0" ascendClassId="0" nodes="""#,
        r#"classId="1" ascendClassId="0" nodes="1000""#,
    )
    .replace(r#"<ItemSet id="1""#, cluster);
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(
        &root().join("crates/loom-data/tests/fixtures/bundle-poe1-tree"),
    ));
    s.import_text(&text, None).unwrap();
    s.edit_tree(1, Edit::Allocate(5000)).unwrap();
    s.socket_jewel(1, 5000, Some(5)).unwrap();
    let notable = view(&s)
        .cluster_nodes
        .iter()
        .find(|n| n.name == "Test Cluster Notable A")
        .unwrap()
        .id;
    s.edit_tree(1, Edit::Allocate(notable)).unwrap();
    let xml = loom_serialization::legacy::pob_xml::export(&s.state().unwrap().build);
    assert!(xml.contains(r#"<Socket nodeId="5000" itemId="5""#), "{xml}");
    assert!(xml.contains(&notable.to_string()), "{xml}");
}

//! The passive tree editor's commands (passive tree plan, T5a) on the
//! synthetic PoE1 bundle: each command as one undoable change, the spec's
//! class, ascendancy, and `<URL>` kept consistent, a node the tree lacks
//! kept by every edit but a reset (T-g), the preview never committing, and
//! the export reading back.
//!
//! The bundle's tree: start 100 (class 1, "Test Brute") - 101 - 102
//! (keystone); 101 - 103 (a mastery, effect 4242); ascendancy start 200
//! ("Test Champion", number 1). Node 999 is not in the tree.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};

use loom_app::{Session, api};
use loom_persistence::Store;
use loom_serialization::legacy::pob_xml;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build_xml() -> String {
    let text = std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap();
    text.replace(
        r#"className="Scion""#,
        r#"className="Test Brute""#,
    )
    .replace(
        r#"<Spec treeVersion="3_29" classId="0" ascendClassId="0" nodes=""/>"#,
        "<Spec treeVersion=\"3_29\" classId=\"1\" ascendClassId=\"0\" nodes=\"100,999\">\n\t\t\t<URL>\n\t\t\t\thttps://www.pathofexile.com/passive-skill-tree/old\n\t\t\t</URL>\n\t\t</Spec>",
    )
}

fn session() -> Session {
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(
        &root().join("crates/loom-data/tests/fixtures/bundle-poe1"),
    ));
    s.import_text(&build_xml(), None).unwrap();
    s
}

fn tree(s: &Session) -> api::TreeView {
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

fn spec_xml(s: &Session) -> String {
    let xml = pob_xml::export(&s.state().unwrap().build);
    let start = xml.find("<Spec ").unwrap();
    let end = xml[start..]
        .find("</Spec>")
        .map_or(xml.len(), |e| start + e);
    xml[start..end].to_owned()
}

#[test]
fn the_layout_should_carry_kinds_links_lines_and_classes() {
    let s = session();
    let data =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe1"))
            .unwrap();
    let layout = data.tree_layout.unwrap();
    assert!(!layout.placed, "the fixture bundle has no layout constants");
    let node = |id: u32| layout.nodes.iter().find(|n| n.id == id).unwrap();
    assert_eq!(node(100).kind, api::TreeNodeKind::ClassStart);
    assert_eq!(node(102).kind, api::TreeNodeKind::Keystone);
    assert_eq!(node(103).kind, api::TreeNodeKind::Mastery);
    assert_eq!(node(200).kind, api::TreeNodeKind::AscendancyStart);
    assert_eq!(node(100).links, [101]);
    assert!(!node(101).lines.is_empty(), "a node's stats are shown");
    assert_eq!(
        layout.classes.len(),
        1,
        "a class with no start node is not offered"
    );
    assert_eq!(layout.classes[0].ascendancies[0].name, "Test Champion");
    assert_eq!(layout.masteries[0].effects[0].effect, 4242);
    let view = tree(&s);
    assert_eq!(view.class, 1);
    assert_eq!(view.nodes, [100, 999]);
    assert_eq!(view.kept, [999]);
    assert_eq!(view.points, 1, "the unknown node counts as PoB counts it");
}

#[test]
fn tree_commands_should_be_undoable_and_keep_the_unknown_node() {
    use loom_app::tree::Edit;
    let mut s = session();
    let before = s.state().unwrap().build.clone();

    s.edit_tree(1, Edit::Allocate(102)).unwrap().unwrap();
    assert_eq!(tree(&s).nodes, [100, 101, 102, 999]);
    let xml = spec_xml(&s);
    assert!(xml.contains(r#"nodes="100,101,102,999""#), "{xml}");
    assert!(!xml.contains("/old"), "the URL is regenerated: {xml}");
    assert!(
        xml.contains(
            "\t\t\t\thttps://www.pathofexile.com/passive-skill-tree/AAAABgEAAwBlAGYD5wAA\n"
        ),
        "{xml}"
    );
    // Allocating it again changes nothing and is refused.
    assert_eq!(
        s.edit_tree(1, Edit::Allocate(102)).unwrap_err().code,
        "APP_TREE"
    );

    s.edit_tree(
        1,
        Edit::Mastery {
            node: 103,
            effect: 4242,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(tree(&s).masteries.len(), 1);
    assert!(spec_xml(&s).contains(r#"masteryEffects="{103,4242}""#));

    s.edit_tree(1, Edit::Deallocate(101)).unwrap().unwrap();
    assert_eq!(
        tree(&s).nodes,
        [100, 999],
        "orphans go, the unknown node stays"
    );
    assert!(tree(&s).masteries.is_empty());

    s.edit_tree(1, Edit::Ascendancy(1)).unwrap().unwrap();
    assert_eq!(tree(&s).nodes, [100, 200, 999]);
    let build = &s.state().unwrap().build;
    assert_eq!(
        build.character.ascendancy.as_ref().map(|a| a.as_str()),
        Some("Test Champion"),
        "the build's ascendancy follows the active spec"
    );
    assert!(spec_xml(&s).contains(r#"ascendClassId="1""#));

    assert_eq!(
        s.edit_tree(1, Edit::Class(3)).unwrap_err().code,
        "APP_TREE",
        "class 3 has no start node in this tree"
    );
    // The same class again: the ascendancy goes, as PoB's `SelectClass`.
    s.edit_tree(1, Edit::Class(1)).unwrap().unwrap();
    let v = tree(&s);
    assert_eq!(
        (v.class, v.ascendancy, v.nodes.clone()),
        (1, 0, vec![100, 999])
    );
    assert_eq!(
        s.state()
            .unwrap()
            .build
            .character
            .ascendancy
            .as_ref()
            .map(|a| a.as_str()),
        Some("None")
    );
    s.edit_tree(1, Edit::Reset).unwrap().unwrap();
    assert_eq!(tree(&s).nodes, [100], "a reset drops the unknown node too");

    for _ in 0..8 {
        if s.undo().is_none() {
            break;
        }
    }
    assert_eq!(s.state().unwrap().build, before, "undo restores the import");
    assert_eq!(
        s.edit_tree(7, Edit::Reset).unwrap_err().code,
        "APP_TREE",
        "no such spec"
    );
}

#[test]
fn the_preview_should_never_commit_and_the_export_should_read_back() {
    use loom_app::tree::Edit;
    let mut s = session();
    let state = s.state().unwrap().clone();
    let data =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe1"))
            .unwrap();
    let p = loom_app::preview_tree(&state, &data, 1, Edit::Allocate(102), None, &|| false).unwrap();
    assert_eq!(p.path, [101, 102]);
    assert!(p.removed.is_empty());
    assert!(!p.fields.is_empty(), "the synthetic build calculates");
    assert_eq!(tree(&s).nodes, [100, 999], "nothing committed");

    s.edit_tree(1, Edit::Allocate(102)).unwrap().unwrap();
    let xml = pob_xml::export(&s.state().unwrap().build);
    let again = pob_xml::import(&xml).unwrap();
    assert!(pob_xml::xml_equal(&xml, &pob_xml::export(&again)).unwrap());
}

/// The layout of a real bundle with the layout constants (maintainer-run:
/// `LOOM_TREE_BUNDLE`, else `local/tree/bundle-poe1-tree-a`).
#[test]
#[ignore = "needs a real bundle with the layout (maintainer-run)"]
fn a_real_layout_should_be_placed_and_print_every_line() {
    let dir = std::env::var_os("LOOM_TREE_BUNDLE").map_or_else(
        || root().join("local/tree/bundle-poe1-tree-a"),
        PathBuf::from,
    );
    let data = loom_app::GameData::load(&dir).unwrap();
    let layout = data.tree_layout.unwrap();
    assert!(layout.placed);
    assert!(layout.nodes.iter().all(|n| n.position.is_some()));
    // A stat no description prints shows as its ID and value.
    let raw: Vec<&String> = layout
        .nodes
        .iter()
        .flat_map(|n| &n.lines)
        .chain(
            layout
                .masteries
                .iter()
                .flat_map(|g| g.effects.iter().flat_map(|e| &e.lines)),
        )
        .filter(|l| {
            l.split_once(' ').is_some_and(|(id, v)| {
                id.contains('_')
                    && id.chars().all(|c| {
                        c.is_ascii_lowercase()
                            || c.is_ascii_digit()
                            || c == '_'
                            || c == '%'
                            || c == '+'
                    })
                    && v.parse::<i64>().is_ok()
            })
        })
        .collect();
    let lines = layout.nodes.iter().map(|n| n.lines.len()).sum::<usize>();
    println!(
        "lines {lines}, raw {}: {:?}",
        raw.len(),
        raw.iter().take(10).collect::<Vec<_>>()
    );
    let node = |name: &str| layout.nodes.iter().find(|n| n.name == name).unwrap();
    println!("{:?}", node("Elemental Overload").lines);
    println!("{:?}", node("Heart of Ice").lines);
    let size = serde_json::to_vec(&*layout).unwrap().len();
    println!("layout JSON {size} bytes");
}

fn session_with(edit: impl Fn(String) -> String) -> Session {
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(
        &root().join("crates/loom-data/tests/fixtures/bundle-poe1"),
    ));
    s.import_text(&edit(build_xml()), None).unwrap();
    s
}

#[test]
fn a_spec_of_another_tree_version_should_be_shown_not_edited() {
    use loom_app::tree::{Edit, locked, run};
    use loom_domain::TreeVersion;
    let s = session_with(|x| {
        x.replace(
            r#"treeVersion="3_29" classId="1""#,
            r#"treeVersion="3_25" classId="1""#,
        )
    });
    let build = &s.state().unwrap().build;
    let tree = s.tree().unwrap();
    let data = TreeVersion::new("3_29");
    let spec = loom_app::tree::spec_of(build, 1).unwrap();
    assert_eq!(
        locked(build, spec, &tree, Some(&data)).as_deref(),
        Some("the spec is for tree version 3_25, the loaded data's is 3_29")
    );
    let err = run(build, 1, &tree, Some(&data), Edit::Allocate(101)).unwrap_err();
    assert_eq!(err.code, "APP_TREE");
    // The same version, or a data version not known, edits.
    assert!(
        run(
            build,
            1,
            &tree,
            Some(&TreeVersion::new("3_25")),
            Edit::Allocate(101)
        )
        .is_ok()
    );
    assert!(run(build, 1, &tree, None, Edit::Allocate(101)).is_ok());
}

#[test]
fn a_spec_whose_class_is_not_known_should_be_shown_not_edited() {
    use loom_app::tree::Edit;
    let mut s = session_with(|x| x.replace(r#"classId="1""#, r#"classId="7""#));
    assert_eq!(
        tree(&s).locked.as_deref(),
        Some("the spec's class is not known")
    );
    let before = spec_xml(&s);
    assert_eq!(
        s.edit_tree(1, Edit::Allocate(101)).unwrap_err().code,
        "APP_TREE"
    );
    assert_eq!(spec_xml(&s), before, "a refused edit writes nothing");
}

#[test]
fn the_active_spec_should_take_its_class_and_ascendancy_from_the_build_names() {
    let s = session_with(|x| {
        x.replace(r#" classId="1" ascendClassId="0""#, "").replace(
            r#"ascendClassName="None""#,
            r#"ascendClassName="Test Champion""#,
        )
    });
    let view = tree(&s);
    assert_eq!((view.class, view.ascendancy), (1, 1));
    assert_eq!(view.locked, None);
    assert!(view.nodes.contains(&200), "the ascendancy's start is added");
}

/// Edits real builds through the application's commands and exports them
/// for PoB to read back (passive tree plan, T7; maintainer-run: the real
/// bundle, then `pob_tree.lua readback` and `tree-parity readback`,
/// `docs/tree/parity.md`). Writes `local/tree/pob/readback/<name>.xml`.
#[test]
#[ignore = "needs a real bundle with the layout (maintainer-run)"]
fn edited_builds_should_be_exported_for_pob_to_read_back() {
    use loom_app::tree::Edit;
    use loom_tree::Kind;
    use loom_tree::poe1;
    let dir = std::env::var_os("LOOM_TREE_BUNDLE").map_or_else(
        || root().join("local/tree/bundle-poe1-tree-a"),
        PathBuf::from,
    );
    let out = root().join("local/tree/pob/readback");
    std::fs::create_dir_all(&out).unwrap();
    for name in [
        "ice-nova-elementalist-campaign-lab1",
        "facebreaker-luminary",
        "animate-guardian-guardian",
    ] {
        let mut s = Session::new(Store::new());
        s.set_data(loom_app::GameData::load(&dir));
        let text = std::fs::read_to_string(
            root().join(format!("tools/oracle/fixtures/community/{name}.xml")),
        )
        .unwrap();
        s.import_text(&text, None).unwrap();
        let tree = s.tree().unwrap();
        let rules = |s: &Session| {
            let build = &s.state().unwrap().build;
            let spec = loom_calc::poe1::collect::active_spec(build).unwrap();
            (spec.id.0, loom_app::tree::rules_spec(build, spec, &tree))
        };
        let (id, spec) = rules(&s);
        // The two keystones with the shortest paths.
        let mut keystones: Vec<(usize, u32)> = tree
            .nodes
            .values()
            .filter(|n| n.kind == Kind::Keystone && n.ascendancy.is_none())
            .filter_map(|n| Some((poe1::path(&tree, &spec, n.id).ok()?.len(), n.id)))
            .collect();
        keystones.sort_unstable();
        let first_path = poe1::path(&tree, &spec, keystones[0].1).unwrap();
        for (_, k) in keystones.iter().take(2) {
            s.edit_tree(id, Edit::Allocate(*k)).unwrap();
        }
        // A mastery effect on a mastery the spec reaches, not yet chosen.
        let (_, spec) = rules(&s);
        let connected = poe1::connected(&tree, &spec);
        let mastery = tree.nodes.values().find(|n| {
            n.kind == Kind::Mastery
                && n.modelled
                && !spec.nodes.contains(&n.id)
                && n.neighbours.iter().any(|m| connected.contains(m))
        });
        if let Some(m) = mastery {
            let taken: Vec<u32> = spec.masteries.values().copied().collect();
            let effect = tree.mastery_effects[m.mastery_group.as_ref().unwrap()]
                .iter()
                .copied()
                .find(|e| !taken.contains(e))
                .unwrap();
            s.edit_tree(id, Edit::Mastery { node: m.id, effect })
                .unwrap();
        }
        // Remove the first keystone's path from its first step.
        s.edit_tree(id, Edit::Deallocate(first_path[0])).unwrap();
        // Another ascendancy, and its nearest notable.
        let (_, spec) = rules(&s);
        let class = tree.class(spec.class).unwrap();
        if let Some(other) = class
            .ascendancies
            .iter()
            .find(|a| a.number != spec.ascendancy && a.start.is_some())
        {
            s.edit_tree(id, Edit::Ascendancy(other.number)).unwrap();
            let (_, spec) = rules(&s);
            let notable = tree
                .nodes
                .values()
                .filter(|n| n.kind == Kind::Notable && n.ascendancy.as_deref() == Some(&other.id))
                .filter_map(|n| Some((poe1::path(&tree, &spec, n.id).ok()?.len(), n.id)))
                .min();
            if let Some((_, n)) = notable {
                s.edit_tree(id, Edit::Allocate(n)).unwrap();
            }
        }
        // A socketed jewel moved to an allocated empty socket, when the
        // spec has both.
        let build = &s.state().unwrap().build;
        let spec = loom_calc::poe1::collect::active_spec(build).unwrap();
        let filled = spec.jewel_sockets.typed().find(|j| {
            j.item.is_some_and(|i| {
                build
                    .items
                    .item(i)
                    .and_then(|x| x.body.base.as_deref())
                    .is_some_and(|b| !b.contains("Cluster"))
            })
        });
        let empty = tree.nodes.values().find(|n| {
            n.kind == Kind::JewelSocket
                && n.modelled
                && spec.nodes.value().iter().any(|a| a.0 == n.id)
                && !spec
                    .jewel_sockets
                    .typed()
                    .any(|j| j.node.0 == n.id && j.item.is_some())
        });
        if let (Some(from), Some(to)) = (filled, empty) {
            let (from, item, to) = (from.node.0, from.item.map(|i| i.0), to.id);
            s.socket_jewel(id, from, None).unwrap();
            s.socket_jewel(id, to, item).unwrap();
            println!("{name}: jewel {item:?} moved from {from} to {to}");
        }
        s.export_xml(&out.join(format!("{name}.xml"))).unwrap();
        println!("{name}: spec {id} exported");
    }
}

#[test]
fn passive_trees_should_be_added_copied_renamed_and_deleted_undoably() {
    let mut s = session();
    let before = s.state().unwrap().build.clone();
    let specs = |s: &Session| -> Vec<(u32, Option<String>)> {
        s.state()
            .unwrap()
            .build
            .passives
            .specs
            .typed()
            .map(|p| (p.id.0, p.title.clone()))
            .collect()
    };
    assert_eq!(specs(&s).len(), 1);
    s.new_spec("Mapping", None).unwrap().unwrap();
    s.new_spec("Mapping copy", Some(1)).unwrap().unwrap();
    assert_eq!(
        specs(&s)[1..],
        [
            (2, Some("Mapping".to_owned())),
            (3, Some("Mapping copy".to_owned()))
        ]
    );
    let xml = pob_xml::export(&s.state().unwrap().build);
    assert!(
        xml.contains(r#"title="Mapping" "#) || xml.contains(r#"title="Mapping""#),
        "{xml}"
    );
    // The new spec has the class and only its start; the copy has spec 1's
    // nodes, the unknown one too.
    let new = loom_app::tree::spec_of(&s.state().unwrap().build, 2).unwrap();
    assert_eq!(
        new.nodes.value().iter().map(|n| n.0).collect::<Vec<_>>(),
        [100]
    );
    let copy = loom_app::tree::spec_of(&s.state().unwrap().build, 3).unwrap();
    assert_eq!(
        copy.nodes.value().iter().map(|n| n.0).collect::<Vec<_>>(),
        [100, 999]
    );
    assert_eq!(
        s.new_spec("  ", None).unwrap_err().code,
        "APP_TREE",
        "a name is needed"
    );

    s.rename_spec(2, "Endgame").unwrap().unwrap();
    s.edit_tree(3, loom_app::tree::Edit::Allocate(101))
        .unwrap()
        .unwrap();
    // Deleting spec 2 renumbers the copy to 2; the active spec stays 1.
    s.delete_spec(2).unwrap().unwrap();
    assert_eq!(specs(&s)[1..], [(2, Some("Mapping copy".to_owned()))]);
    assert_eq!(
        s.state().unwrap().build.passives.active.map(|a| a.0),
        Some(1)
    );
    // Deleting the active spec makes the one before it active.
    s.use_loadout(2).unwrap();
    s.delete_spec(2).unwrap().unwrap();
    assert_eq!(
        s.state().unwrap().build.passives.active.map(|a| a.0),
        Some(1)
    );
    assert_eq!(
        s.delete_spec(1).unwrap_err().code,
        "APP_TREE",
        "the last spec stays"
    );
    while s.undo().is_some() {}
    assert_eq!(
        s.state().unwrap().build,
        before,
        "undo goes back byte for byte"
    );
}

#[test]
fn a_bundle_with_sprites_should_give_them_as_data_urls() {
    // The synthetic tree bundle carries a made-up sheet (make_bundles.py,
    // `tree_sprites`); the plain fixture bundle has none.
    let data =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe1-tree"))
            .unwrap();
    let sprites = data.tree_sprites().unwrap();
    assert!((sprites.zoom - 0.5).abs() < f64::EPSILON);
    assert_eq!(sprites.sheets.len(), 1);
    assert_eq!(sprites.sheets[0].file, "test-sheet.png");
    assert!(
        sprites.sheets[0]
            .image
            .starts_with("data:image/png;base64,iVBORw0KGgo"),
        "the PNG's signature, base64"
    );
    let frame = sprites
        .categories
        .iter()
        .find(|c| c.name == "frame")
        .unwrap();
    assert!(frame.sprites.iter().any(|s| s.name == "PSSkillFrame"));
    let mastery = sprites.nodes.iter().find(|n| n.node == 10029).unwrap();
    assert_eq!(mastery.inactive_icon.as_deref(), Some("Test/Mastery"));
    let plain =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe1"))
            .unwrap();
    assert!(plain.tree_sprites().is_none());
}

#[test]
fn the_layout_should_carry_the_trees_flavour_texts() {
    // B-2 item evidence (E10): the synthetic tree bundle's made-up texts
    // (make_bundles.py): keystone 11040's lines and Test Champion's quote.
    let data =
        loom_app::GameData::load(&root().join("crates/loom-data/tests/fixtures/bundle-poe1-tree"))
            .unwrap();
    let layout = data.tree_layout.clone().unwrap();
    let keystone = layout.nodes.iter().find(|n| n.id == 11040).unwrap();
    assert_eq!(
        keystone.flavour.as_deref(),
        Some("A made-up keystone line.\nA second one.")
    );
    assert!(
        layout
            .nodes
            .iter()
            .filter(|n| n.id != 11040)
            .all(|n| n.flavour.is_none())
    );
    let champion = layout
        .classes
        .iter()
        .flat_map(|c| &c.ascendancies)
        .find(|a| a.id == "TestChampion")
        .unwrap();
    assert_eq!(champion.flavour.as_deref(), Some("A made-up quote."));
    assert_eq!(champion.flavour_colour.as_deref(), Some("200,180,120"));
}

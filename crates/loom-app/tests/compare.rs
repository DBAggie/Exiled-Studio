//! The read-only queries behind "compare": the numbers of another build
//! file (`loom_app::compare`), calculated without opening it, and another
//! stage's tree and equipment (`loom_app::stage_view`), read without
//! choosing it. On the synthetic PoE1 bundles and builds.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]
#![expect(clippy::panic, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use loom_app::tree::Edit;
use loom_app::{App, Config, HostCommand, Session, api};
use loom_persistence::Store;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle_dir() -> PathBuf {
    root().join("crates/loom-data/tests/fixtures/bundle-poe1")
}

fn tree_bundle_dir() -> PathBuf {
    root().join("crates/loom-data/tests/fixtures/bundle-poe1-tree")
}

fn synthetic() -> String {
    std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap()
}

/// The synthetic build with no resistance penalty: other numbers than
/// the plain one's (its resistances).
fn other() -> String {
    let xml = synthetic().replace(
        "<Config/>",
        "<Config activeConfigSet=\"1\"><ConfigSet id=\"1\" title=\"Default\">\
         <Input name=\"resistancePenalty\" number=\"0\"/></ConfigSet></Config>",
    );
    assert_ne!(xml, synthetic(), "the build was not changed");
    xml
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("loom-app-compare-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn empty_session(bundle: &Path) -> Session {
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(bundle));
    s
}

fn idle() -> api::CalcStatus {
    api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    }
}

/// The session's own calculation of its build, as the query gives fields.
fn calculated(s: &Session) -> Vec<api::WhatIfField> {
    let result = loom_app::calculate(s.state().unwrap(), s.data().unwrap(), &|| false).unwrap();
    result
        .fields
        .iter()
        .map(|(field, f)| api::WhatIfField {
            field: field.to_string(),
            value: f.value.filter(|v| v.is_finite()),
            state: f.state.kind().name().to_owned(),
        })
        .collect()
}

fn fields_of(path: &Path, s: &Session) -> Result<api::BuildFields, loom_app::AppError> {
    loom_app::fields_of_file(path, s.data().unwrap(), &|| false)
}

struct Running {
    app: App,
    events: Receiver<api::Event>,
}

fn start(data_dir: &Path, bundle: Option<PathBuf>) -> Running {
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let sink: loom_app::Sink = Arc::new(move |e| {
        let _ = tx.lock().unwrap().send(e);
    });
    let mut config = Config::new(data_dir.to_owned(), bundle).with_test_keys(true);
    config.instance = "compare".to_owned();
    config.autosave = Duration::from_secs(3600);
    Running {
        app: App::start(config, sink).unwrap(),
        events,
    }
}

impl Running {
    fn run(&self, c: api::Command) -> u64 {
        match self.app.execute(HostCommand::Api(c)).unwrap() {
            api::Outcome::Done { version } | api::Outcome::Text { version, .. } => version,
            api::Outcome::Nothing => panic!("nothing done"),
        }
    }

    /// Waits until the calculation of `version` finished.
    fn calculated(&self, version: u64) {
        loop {
            match self.events.recv_timeout(Duration::from_secs(30)) {
                Ok(e)
                    if e.version == version
                        && matches!(e.kind, api::EventKind::CalcFinished { .. }) =>
                {
                    return;
                }
                Ok(_) => {}
                Err(_) => panic!("no calc-finished event"),
            }
        }
    }
}

#[test]
fn a_pob_file_should_give_the_fields_its_import_calculates() {
    let dir = Scratch::new("pob");
    let path = dir.write("Other Build.xml", &other());
    let mut imported = empty_session(&bundle_dir());
    imported.import_file(&path).unwrap();

    let answer = fields_of(&path, &imported).unwrap();
    assert_eq!(answer.name, "Other Build");
    assert!(!answer.fields.is_empty());
    assert_eq!(answer.fields, calculated(&imported));

    // Another build than the plain one: the numbers are the file's.
    let mut plain = empty_session(&bundle_dir());
    plain.import_text(&synthetic(), None).unwrap();
    assert_ne!(answer.fields, calculated(&plain));
}

#[test]
fn a_build_document_should_give_the_fields_its_opening_calculates() {
    let dir = Scratch::new("document");
    let path = dir.0.join("saved.json");
    let mut source = empty_session(&bundle_dir());
    source.import_text(&other(), None).unwrap();
    source.save_as(&path).unwrap();
    let mut opened = empty_session(&bundle_dir());
    opened.open(&path).unwrap();

    let answer = fields_of(&path, &opened).unwrap();
    assert_eq!(answer.name, "saved");
    assert_eq!(answer.fields, calculated(&opened));
    // The saved build calculates as the import it was saved from.
    assert_eq!(answer.fields, calculated(&source));

    // Leading whitespace does not make it another kind of file.
    let text = std::fs::read_to_string(&path).unwrap();
    let padded = dir.write("padded.json", &format!("\n  \t{text}"));
    assert_eq!(fields_of(&padded, &opened).unwrap().fields, answer.fields);
    // The extension does not decide: the content does.
    let renamed = dir.write("document.xml", &text);
    assert_eq!(fields_of(&renamed, &opened).unwrap().fields, answer.fields);
    let pob = dir.write("pob.json", &other());
    assert_eq!(fields_of(&pob, &opened).unwrap().fields, answer.fields);
}

#[test]
fn a_file_that_is_neither_should_give_its_readers_error() {
    let dir = Scratch::new("neither");
    let s = empty_session(&bundle_dir());

    // Not JSON: PoB's reader refuses it, as an import of the file does.
    let notes = dir.write("notes.txt", "not a build at all");
    let e = fields_of(&notes, &s).unwrap_err();
    let mut import = empty_session(&bundle_dir());
    assert_eq!(e, import.import_file(&notes).unwrap_err());

    // JSON that is no build document: the store refuses it, as opening
    // the file does.
    let json = dir.write("other.json", r#"{"dashboards":[]}"#);
    let e = fields_of(&json, &s).unwrap_err();
    let mut open = empty_session(&bundle_dir());
    assert_eq!(e, open.open(&json).unwrap_err());
    assert!(
        e.code.starts_with("PERSIST_")
            || e.code.starts_with("DTO_")
            || e.code.starts_with("MIGRATE_"),
        "{e}"
    );

    // No file.
    let e = fields_of(&dir.0.join("missing.xml"), &s).unwrap_err();
    assert_eq!(e.code, "APP_READ");
    // A PoB file that is not text.
    let binary = dir.0.join("binary.xml");
    std::fs::write(&binary, [b'<', 0xFF, 0xFE]).unwrap();
    assert_eq!(fields_of(&binary, &s).unwrap_err().code, "APP_READ");
}

#[test]
fn a_build_of_other_data_should_be_refused_as_a_calculation_of_it_is() {
    let dir = Scratch::new("data");
    let path = dir.0.join("saved.json");
    let mut source = empty_session(&bundle_dir());
    source.import_text(&synthetic(), None).unwrap();
    source.save_as(&path).unwrap();

    // The same document, pinned to another data version than the loaded
    // one.
    let text = std::fs::read_to_string(&path).unwrap();
    let pinned = text.replacen("\"schema_version\": 4", "\"schema_version\": 3", 1);
    assert_ne!(pinned, text, "the data version was not changed");
    let path = dir.write("pinned.json", &pinned);
    let mut opened = empty_session(&bundle_dir());
    opened.open(&path).unwrap();
    let expected = opened.calc_input().unwrap_err();
    assert_eq!(expected.code, "APP_DATA_VERSION");
    assert_eq!(fields_of(&path, &opened).unwrap_err(), expected);
}

#[test]
fn comparing_a_file_should_need_only_data_and_leave_the_open_build_alone() {
    let dir = Scratch::new("app");
    let path = dir.write("other.xml", &other());
    let mut reference = empty_session(&bundle_dir());
    reference.import_file(&path).unwrap();
    let expected = calculated(&reference);

    // Without data: refused.
    let bare = start(&dir.0.join("bare"), None);
    assert_eq!(
        bare.app.fields_of_file(&path).unwrap_err().code,
        "APP_NO_DATA"
    );
    bare.app.stop();

    // With data and no build open.
    let running = start(&dir.0.join("data"), Some(bundle_dir()));
    assert!(running.app.snapshot().build.is_none());
    let answer = running.app.fields_of_file(&path).unwrap();
    assert_eq!(answer.name, "other");
    assert_eq!(answer.fields, expected);
    assert!(running.app.snapshot().build.is_none());

    // With a build open and changed: its version, dirtiness, history,
    // name, and the recent list stay as they are.
    running.run(api::Command::ImportText { text: synthetic() });
    let version = running.run(api::Command::SetDashboards {
        dashboards: Some(api::Dashboards {
            active: "overview".to_owned(),
            list: vec![api::Dashboard {
                id: "overview".to_owned(),
                name: "Overview".to_owned(),
                components: Vec::new(),
            }],
        }),
    });
    running.calculated(version);
    let before = running.app.snapshot();
    assert!(before.dirty);
    assert!(before.history.can_undo);
    let own = running.app.calc_document().unwrap();

    assert_eq!(running.app.fields_of_file(&path).unwrap().fields, expected);
    let e = running
        .app
        .fields_of_file(&dir.0.join("missing.json"))
        .unwrap_err();
    assert_eq!(e.code, "APP_READ");

    assert_eq!(running.app.snapshot(), before);
    assert_eq!(running.app.calc_document().unwrap(), own);
    assert!(running.app.recent().is_empty());
    running.app.stop();
}

/// A Test Brute on the synthetic tree bundle with three passive specs and
/// two item sets: stages 1 and 2 linked to an item set each, stage 3 to
/// none. Spec 2 gets the large socket 5000 and its path allocated.
const STAGED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="1" targetVersion="3_0" className="Test Brute" ascendClassName="None" mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>
	<Tree activeSpec="1">
		<Spec title="^4Leveling {1}" treeVersion="3_29" classId="1" ascendClassId="0" clusterHashFormatVersion="2" nodes="1000"/>
		<Spec title="Endgame {2}" treeVersion="3_29" classId="1" ascendClassId="0" clusterHashFormatVersion="2" nodes="1000"/>
		<Spec title="Side {9}" treeVersion="3_29" classId="1" ascendClassId="0" clusterHashFormatVersion="2" nodes="1000"/>
	</Tree>
	<Skills/>
	<Items activeItemSet="1" useSecondWeaponSet="false">
		<Item id="1">
Rarity: RARE
Loom Test Cluster
Large Test Cluster Jewel
Implicits: 0
Adds +8 Test Cluster Passive Skills
		</Item>
		<Item id="2">
Rarity: RARE
Loom Test Medium
Medium Test Cluster Jewel
Implicits: 0
Adds +4 Test Cluster Passive Skills
		</Item>
		<Item id="3">
Rarity: RARE
Loom Test Spare
Medium Test Cluster Jewel
Implicits: 0
Adds +4 Test Cluster Passive Skills
		</Item>
		<ItemSet id="1" title="Leveling {1}" useSecondWeaponSet="false">
			<Slot name="Ring 1" itemId="1"/>
			<Slot name="Ring 2" itemId="0"/>
		</ItemSet>
		<ItemSet id="2" title="Endgame {2}" useSecondWeaponSet="false">
			<Slot name="Ring 1" itemId="2"/>
			<Slot name="Amulet" itemId="1"/>
		</ItemSet>
	</Items>
</PathOfBuilding>
"#;

fn staged_session() -> Session {
    let mut s = empty_session(&tree_bundle_dir());
    s.import_text(STAGED, None).unwrap();
    s.edit_tree(2, Edit::Allocate(5000)).unwrap().unwrap();
    s
}

/// What the build's view shows of the tree and the equipment: the active
/// spec's nodes and each item's slot.
fn shown(s: &Session) -> (Vec<u32>, Vec<api::StageSlot>) {
    let build = s.snapshot(idle()).build.unwrap();
    let slots = build
        .items
        .iter()
        .filter_map(|i| {
            Some(api::StageSlot {
                slot: i.slot.clone()?,
                item: i.id,
            })
        })
        .collect();
    (build.tree.unwrap().nodes, slots)
}

fn stage(s: &Session, spec: u32) -> Result<api::StageView, loom_app::AppError> {
    loom_app::stage_view(&s.state().unwrap().build, spec, s.data().map(|d| &**d))
}

fn slot(slot: &str, item: u32) -> api::StageSlot {
    api::StageSlot {
        slot: slot.to_owned(),
        item,
    }
}

#[test]
fn the_active_stage_should_be_what_the_snapshot_shows() {
    let s = staged_session();
    let view = stage(&s, 1).unwrap();
    assert_eq!(view.spec, 1);
    assert_eq!(view.title, "Leveling");
    let (nodes, slots) = shown(&s);
    assert_eq!(view.nodes, nodes);
    assert_eq!(view.slots, slots);
    assert_eq!(view.nodes, [1000]);
    assert_eq!(view.slots, [slot("Ring 1", 1)]);
}

#[test]
fn another_stage_should_be_what_the_snapshot_shows_once_it_is_chosen() {
    let s = staged_session();
    let version = s.version();
    let before = Arc::clone(s.state().unwrap());

    for spec in [1, 2, 3] {
        let view = stage(&s, spec).unwrap();
        let mut chosen = staged_session();
        let _ = chosen.use_loadout(spec).unwrap();
        let (nodes, slots) = shown(&chosen);
        assert_eq!(view.nodes, nodes, "stage {spec}");
        assert_eq!(view.slots, slots, "stage {spec}");
        assert_eq!(view.spec, spec);
    }

    // The second stage: its own tree and its linked item set.
    let endgame = stage(&s, 2).unwrap();
    assert_eq!(endgame.title, "Endgame");
    assert!(endgame.nodes.contains(&5000), "{:?}", endgame.nodes);
    assert!(endgame.nodes.len() > 1);
    assert!(endgame.nodes.is_sorted());
    assert_eq!(endgame.slots, [slot("Amulet", 1), slot("Ring 1", 2)]);
    // The third has no item set of its own: the one the build keeps.
    let side = stage(&s, 3).unwrap();
    assert_eq!(side.title, "Side");
    assert_eq!(side.nodes, [1000]);
    assert_eq!(side.slots, [slot("Ring 1", 1)]);

    // Nothing was chosen.
    assert_eq!(s.version(), version);
    assert!(Arc::ptr_eq(&before, s.state().unwrap()));
    assert_eq!(shown(&s), (vec![1000], vec![slot("Ring 1", 1)]));
}

#[test]
fn a_stage_without_tree_data_should_give_the_specs_own_nodes() {
    let mut s = empty_session(&bundle_dir());
    s.import_text(
        &synthetic().replace(" nodes=\"\"/>", " nodes=\"300,7,7,45\"/>"),
        None,
    )
    .unwrap();
    // Without game data there is no tree to read the spec with.
    let view = loom_app::stage_view(&s.state().unwrap().build, 1, None).unwrap();
    assert_eq!(view.nodes, [7, 45, 300]);
    assert_eq!(view.slots, [slot("Ring 1", 1)]);
}

#[test]
fn an_unknown_stage_should_be_refused_with_use_loadouts_code() {
    let mut s = staged_session();
    let e = stage(&s, 4).unwrap_err();
    assert_eq!(e.code, "APP_LOADOUT");
    assert_eq!(e, s.use_loadout(4).unwrap_err());
}

#[test]
fn stage_view_should_read_the_applications_latest_state() {
    let dir = Scratch::new("stage");
    let running = start(&dir.0.join("data"), Some(tree_bundle_dir()));
    assert_eq!(running.app.stage_view(1).unwrap_err().code, "APP_NO_BUILD");

    running.run(api::Command::ImportText {
        text: STAGED.to_owned(),
    });
    let version = running.run(api::Command::AllocateNode {
        spec: 2,
        node: 5000,
    });
    let before = running.app.snapshot();
    let reference = staged_session();
    for spec in [1, 2, 3] {
        assert_eq!(
            running.app.stage_view(spec).unwrap(),
            stage(&reference, spec).unwrap()
        );
    }
    assert_eq!(running.app.stage_view(7).unwrap_err().code, "APP_LOADOUT");
    let after = running.app.snapshot();
    assert_eq!(after.version, version);
    assert_eq!(after.build, before.build);
    assert_eq!(after.history, before.history);
    running.app.stop();
}

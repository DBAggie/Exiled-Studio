//! The build's dashboards (the layout its author chose to show it with,
//! typed and checked by the core; maintainer decision 2026-11-16): set and
//! read back, undo and redo, kept by a save, every limit, and PoB's
//! formats unaffected. On the synthetic PoE1 bundle and build.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]
#![expect(clippy::panic, reason = "tests; a failure should panic")]
#![expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use loom_app::{
    App, Config, HostCommand, MAX_DASHBOARD_COMPONENTS, MAX_DASHBOARD_FIELDS, MAX_DASHBOARD_NAME,
    MAX_DASHBOARD_TEXT, MAX_DASHBOARDS, Session, api,
};
use loom_persistence::Store;
use loom_serialization::legacy::pob_xml;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle_dir() -> PathBuf {
    root().join("crates/loom-data/tests/fixtures/bundle-poe1")
}

fn synthetic() -> String {
    std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("loom-app-dashboards-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn session() -> Session {
    let mut s = Session::new(Store::new());
    s.set_data(loom_app::GameData::load(&bundle_dir()));
    s.import_text(&synthetic(), None).unwrap();
    s
}

fn snapshot(s: &Session) -> api::Snapshot {
    s.snapshot(api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    })
}

fn dashboards(s: &Session) -> Option<api::Dashboards> {
    snapshot(s).build.unwrap().dashboards
}

/// The dashboards as the UI writes them: a stat, a progression chart, and
/// a fight timeline.
const DOCUMENT: &str = r#"{"active":"overview","list":[{"id":"overview","name":"Overview","components":[{"id":"c1","kind":"stat","width":"third","field":"hit-dps"},{"id":"c2","kind":"progression","width":"full","fields":["life","hit-dps"]},{"id":"c3","kind":"timeline","width":"half","target":5000000}]}]}"#;

fn document() -> api::Dashboards {
    serde_json::from_str(DOCUMENT).unwrap()
}

fn component(id: &str) -> api::DashboardComponent {
    api::DashboardComponent {
        id: id.to_owned(),
        kind: "stat".to_owned(),
        width: "third".to_owned(),
        field: Some("life".to_owned()),
        fields: Vec::new(),
        vary: None,
        input: None,
        target: None,
    }
}

fn dashboard(id: &str, components: Vec<api::DashboardComponent>) -> api::Dashboard {
    api::Dashboard {
        id: id.to_owned(),
        name: format!("Dashboard {id}"),
        components,
    }
}

/// `count` dashboards `d0`, `d1`, ... of one component each, the first
/// active.
fn several(count: usize) -> api::Dashboards {
    api::Dashboards {
        active: "d0".to_owned(),
        list: (0..count)
            .map(|i| dashboard(&format!("d{i}"), vec![component(&format!("c{i}"))]))
            .collect(),
    }
}

#[test]
fn the_dashboards_should_be_set_read_back_and_undone_as_one_change() {
    let mut s = session();
    assert_eq!(dashboards(&s), None);
    let imported = s.version();

    let set = s.set_dashboards(Some(document())).unwrap().unwrap();
    assert!(set > imported);
    assert_eq!(dashboards(&s), Some(document()));
    // The view writes what was sent.
    assert_eq!(
        serde_json::to_string(&dashboards(&s).unwrap()).unwrap(),
        DOCUMENT
    );
    let history = snapshot(&s).history;
    assert_eq!(
        history.undo,
        Some(api::HistoryEntry {
            kind: api::ChangeKind::Dashboards,
            item: 0,
            request: None,
        })
    );

    // The same dashboards again are no change: no version, no history step.
    assert_eq!(s.set_dashboards(Some(document())).unwrap(), None);
    assert_eq!(s.version(), set);

    // Other dashboards are a second step; undo and redo walk them.
    let other = several(2);
    s.set_dashboards(Some(other.clone())).unwrap().unwrap();
    assert_eq!(dashboards(&s), Some(other.clone()));
    s.undo().unwrap();
    assert_eq!(dashboards(&s), Some(document()));
    s.undo().unwrap();
    assert_eq!(dashboards(&s), None);
    assert!(!snapshot(&s).history.can_undo);
    s.redo().unwrap();
    s.redo().unwrap();
    assert_eq!(dashboards(&s), Some(other));

    // None clears them; clearing nothing is no change.
    s.set_dashboards(None).unwrap().unwrap();
    assert_eq!(dashboards(&s), None);
    assert_eq!(s.set_dashboards(None).unwrap(), None);
}

#[test]
fn the_dashboards_should_be_kept_by_a_save_and_left_out_when_there_are_none() {
    let dir = Scratch::new("save");
    let mut s = session();

    // Without any the document has no such field (an old reader reads it).
    let plain = dir.0.join("plain.json");
    s.save_as(&plain).unwrap();
    let text = std::fs::read_to_string(&plain).unwrap();
    assert!(!text.contains("\"dashboards\""), "{text}");
    let mut reopened = Session::new(Store::new());
    reopened.open(&plain).unwrap();
    assert_eq!(dashboards(&reopened), None);

    s.set_dashboards(Some(document())).unwrap().unwrap();
    assert!(s.dirty());
    let kept = dir.0.join("kept.json");
    s.save_as(&kept).unwrap();
    assert!(!s.dirty());
    let text = std::fs::read_to_string(&kept).unwrap();
    let file: serde_json::Value = serde_json::from_str(&text).unwrap();
    // The build document's section is the API's shape.
    assert_eq!(
        file["build"]["dashboards"],
        serde_json::from_str::<serde_json::Value>(DOCUMENT).unwrap()
    );
    let mut reopened = Session::new(Store::new());
    reopened.open(&kept).unwrap();
    assert_eq!(dashboards(&reopened), Some(document()));
    // The rest of the build is what was saved.
    assert_eq!(reopened.state().unwrap().build, s.state().unwrap().build);
}

#[test]
fn a_file_whose_dashboards_break_the_limits_should_still_open() {
    let dir = Scratch::new("open");
    let mut s = session();
    s.set_dashboards(Some(document())).unwrap().unwrap();
    let path = dir.0.join("build.json");
    s.save_as(&path).unwrap();

    // Edited by hand: no dashboards at all, and an active one that is not
    // there. Setting it is refused; opening the build never is.
    let mut file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    file["build"]["dashboards"] = serde_json::json!({"active": "gone", "list": []});
    std::fs::write(&path, file.to_string()).unwrap();
    let mut reopened = Session::new(Store::new());
    reopened.open(&path).unwrap();
    let read = dashboards(&reopened).unwrap();
    assert_eq!(read.active, "gone");
    assert!(read.list.is_empty());
    assert_eq!(
        reopened.set_dashboards(Some(read)).unwrap_err().code,
        "APP_DASHBOARDS"
    );
}

#[test]
fn dashboards_outside_a_limit_should_be_refused_and_change_nothing() {
    let mut s = session();
    s.set_dashboards(Some(document())).unwrap().unwrap();
    let version = s.version();
    let before = Arc::clone(s.state().unwrap());

    assert_eq!(
        (
            MAX_DASHBOARDS,
            MAX_DASHBOARD_COMPONENTS,
            MAX_DASHBOARD_FIELDS,
            MAX_DASHBOARD_NAME,
            MAX_DASHBOARD_TEXT
        ),
        (8, 24, 6, 40, 64)
    );
    let long = "a".repeat(MAX_DASHBOARD_TEXT + 1);
    let edit = |change: &dyn Fn(&mut api::Dashboards)| {
        let mut d = several(2);
        change(&mut d);
        d
    };
    let refused: Vec<(&str, api::Dashboards)> = vec![
        ("no dashboards", several(0)),
        ("too many dashboards", several(MAX_DASHBOARDS + 1)),
        (
            "a repeated dashboard ID",
            edit(&|d| d.list[1].id = "d0".to_owned()),
        ),
        (
            "an active dashboard not in the list",
            edit(&|d| d.active = "d2".to_owned()),
        ),
        ("an empty active dashboard", edit(&|d| d.active.clear())),
        (
            "too many components",
            edit(&|d| {
                d.list[0].components = (0..=MAX_DASHBOARD_COMPONENTS)
                    .map(|i| component(&format!("x{i}")))
                    .collect();
            }),
        ),
        (
            "a component ID repeated in one dashboard",
            edit(&|d| d.list[0].components.push(component("c0"))),
        ),
        (
            "a component ID repeated across dashboards",
            edit(&|d| d.list[1].components[0].id = "c0".to_owned()),
        ),
        (
            "too many fields",
            edit(&|d| {
                d.list[0].components[0].fields = (0..=MAX_DASHBOARD_FIELDS)
                    .map(|i| format!("field-{i}"))
                    .collect();
            }),
        ),
        ("an empty name", edit(&|d| d.list[0].name.clear())),
        (
            "a long name",
            edit(&|d| d.list[0].name = "n".repeat(MAX_DASHBOARD_NAME + 1)),
        ),
        ("an empty dashboard ID", edit(&|d| d.list[1].id.clear())),
        (
            "a long dashboard ID",
            edit(&|d| {
                d.list[0].id.clone_from(&long);
                d.active.clone_from(&long);
            }),
        ),
        (
            "an empty component ID",
            edit(&|d| d.list[0].components[0].id.clear()),
        ),
        (
            "a long component ID",
            edit(&|d| d.list[0].components[0].id.clone_from(&long)),
        ),
        (
            "an empty kind",
            edit(&|d| d.list[0].components[0].kind.clear()),
        ),
        (
            "a long kind",
            edit(&|d| d.list[0].components[0].kind.clone_from(&long)),
        ),
        (
            "an empty width",
            edit(&|d| d.list[0].components[0].width.clear()),
        ),
        (
            "a long width",
            edit(&|d| d.list[0].components[0].width.clone_from(&long)),
        ),
        (
            "an empty field",
            edit(&|d| d.list[0].components[0].field = Some(String::new())),
        ),
        (
            "a long field",
            edit(&|d| d.list[0].components[0].field = Some(long.clone())),
        ),
        (
            "an empty entry of fields",
            edit(&|d| d.list[0].components[0].fields = vec!["life".to_owned(), String::new()]),
        ),
        (
            "a long entry of fields",
            edit(&|d| d.list[0].components[0].fields = vec![long.clone()]),
        ),
        (
            "an empty vary",
            edit(&|d| d.list[0].components[0].vary = Some(String::new())),
        ),
        (
            "a long vary",
            edit(&|d| d.list[0].components[0].vary = Some(long.clone())),
        ),
        (
            "an empty input",
            edit(&|d| d.list[0].components[0].input = Some(String::new())),
        ),
        (
            "a long input",
            edit(&|d| d.list[0].components[0].input = Some(long.clone())),
        ),
    ];
    for (what, d) in refused {
        let e = s.set_dashboards(Some(d)).unwrap_err();
        assert_eq!(e.code, "APP_DASHBOARDS", "{what}: {e:?}");
        assert_eq!(s.version(), version, "{what}");
        assert!(Arc::ptr_eq(&before, s.state().unwrap()), "{what}");
        assert_eq!(dashboards(&s), Some(document()), "{what}");
    }

    // Exactly at every limit is kept. The limits count characters, not
    // bytes; a dashboard may be empty; a kind this application does not
    // know is kept as it is.
    let text = "é".repeat(MAX_DASHBOARD_TEXT);
    let mut full = several(MAX_DASHBOARDS);
    full.list[0].id.clone_from(&text);
    full.active.clone_from(&text);
    full.list[0].name = "ü".repeat(MAX_DASHBOARD_NAME);
    full.list[0].components = (0..MAX_DASHBOARD_COMPONENTS)
        .map(|i| component(&format!("x{i}")))
        .collect();
    full.list[1].components.clear();
    full.list[2].components[0] = api::DashboardComponent {
        id: text.clone(),
        kind: text.clone(),
        width: text.clone(),
        field: Some(text.clone()),
        fields: vec![text.clone(); MAX_DASHBOARD_FIELDS],
        vary: Some(text.clone()),
        input: Some(text),
        target: Some(u64::MAX),
    };
    full.list[3].components[0].kind = "a-kind-of-later".to_owned();
    s.set_dashboards(Some(full.clone())).unwrap().unwrap();
    assert_eq!(dashboards(&s), Some(full));
}

#[test]
fn the_dashboards_should_not_reach_pobs_formats() {
    let dir = Scratch::new("export");
    let mut s = session();
    let xml = pob_xml::export(&s.state().unwrap().build);
    let code = s.share_code().unwrap();

    s.set_dashboards(Some(document())).unwrap().unwrap();
    assert_eq!(pob_xml::export(&s.state().unwrap().build), xml);
    assert_eq!(s.share_code().unwrap(), code);
    let path = dir.0.join("export.xml");
    s.export_xml(&path).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert_eq!(written, xml);
    assert!(!written.contains("overview"));

    // An import of the export has no dashboards: PoB's format has no place
    // for them.
    let mut again = Session::new(Store::new());
    again.set_data(loom_app::GameData::load(&bundle_dir()));
    again.import_file(&path).unwrap();
    assert_eq!(dashboards(&again), None);
}

#[test]
fn set_dashboards_should_run_as_a_command_of_the_application() {
    let dir = Scratch::new("command");
    let sink: loom_app::Sink = Arc::new(|_| {});
    let mut config = Config::new(dir.0.join("data"), Some(bundle_dir())).with_test_keys(true);
    config.instance = "dashboards".to_owned();
    config.autosave = Duration::from_secs(3600);
    let app = App::start(config, sink).unwrap();
    let run = |c: api::Command| app.execute(HostCommand::Api(c));
    let set = |dashboards: Option<api::Dashboards>| run(api::Command::SetDashboards { dashboards });

    // No build: nothing to keep them in.
    assert_eq!(set(Some(document())).unwrap_err().code, "APP_NO_BUILD");

    run(api::Command::ImportText { text: synthetic() }).unwrap();
    let api::Outcome::Done { version } = set(Some(document())).unwrap() else {
        panic!("the dashboards were not set");
    };
    let snap = app.snapshot();
    assert_eq!(snap.version, version);
    assert_eq!(snap.build.as_ref().unwrap().dashboards, Some(document()));
    assert_eq!(
        snap.history.undo.map(|e| e.kind),
        Some(api::ChangeKind::Dashboards)
    );
    assert_eq!(set(Some(document())).unwrap(), api::Outcome::Nothing);
    assert_eq!(
        set(Some(several(MAX_DASHBOARDS + 1))).unwrap_err().code,
        "APP_DASHBOARDS"
    );
    assert_eq!(app.snapshot().version, version);

    run(api::Command::Undo).unwrap();
    assert_eq!(app.snapshot().build.unwrap().dashboards, None);
    run(api::Command::Redo).unwrap();
    assert!(matches!(set(None).unwrap(), api::Outcome::Done { .. }));
    assert_eq!(app.snapshot().build.unwrap().dashboards, None);
    app.stop();
}

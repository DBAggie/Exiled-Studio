//! The build's workspace document (the UI's dashboards and kept snapshots,
//! opaque to the application; maintainer approval 2026-11-16): set and
//! read back, undo and redo, kept by a save, the size limit, and PoB's
//! formats unaffected. On the synthetic PoE1 bundle and build.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]
#![expect(clippy::panic, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use loom_app::{App, Config, HostCommand, MAX_WORKSPACE, Session, api};
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
            std::env::temp_dir().join(format!("loom-app-workspace-{test}-{}", std::process::id()));
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

fn workspace(s: &Session) -> Option<String> {
    snapshot(s).build.unwrap().workspace
}

const DOCUMENT: &str =
    r#"{"dashboards":[{"name":"Overview"}],"snapshots":[{"name":"Before","life":60}]}"#;

#[test]
fn the_workspace_should_be_set_read_back_and_undone_as_one_change() {
    let mut s = session();
    assert_eq!(workspace(&s), None);
    let imported = s.version();

    let set = s.set_workspace(Some(DOCUMENT.to_owned())).unwrap().unwrap();
    assert!(set > imported);
    assert_eq!(workspace(&s).as_deref(), Some(DOCUMENT));
    let history = snapshot(&s).history;
    assert_eq!(
        history.undo,
        Some(api::HistoryEntry {
            kind: api::ChangeKind::Workspace,
            item: 0,
            request: None,
        })
    );

    // The same document again is no change: no version, no history step.
    assert_eq!(s.set_workspace(Some(DOCUMENT.to_owned())).unwrap(), None);
    assert_eq!(s.version(), set);

    // Another document is a second step; undo and redo walk them.
    let other = r#"{"dashboards":[]}"#;
    s.set_workspace(Some(other.to_owned())).unwrap().unwrap();
    assert_eq!(workspace(&s).as_deref(), Some(other));
    s.undo().unwrap();
    assert_eq!(workspace(&s).as_deref(), Some(DOCUMENT));
    s.undo().unwrap();
    assert_eq!(workspace(&s), None);
    assert!(!snapshot(&s).history.can_undo);
    s.redo().unwrap();
    s.redo().unwrap();
    assert_eq!(workspace(&s).as_deref(), Some(other));

    // Null clears it; clearing nothing is no change.
    s.set_workspace(None).unwrap().unwrap();
    assert_eq!(workspace(&s), None);
    assert_eq!(s.set_workspace(None).unwrap(), None);
    // An empty document is a document.
    s.set_workspace(Some(String::new())).unwrap().unwrap();
    assert_eq!(workspace(&s).as_deref(), Some(""));
}

#[test]
fn the_workspace_should_be_kept_by_a_save_and_left_out_when_there_is_none() {
    let dir = Scratch::new("save");
    let mut s = session();

    // Without one the document has no such field (an old reader reads it).
    let plain = dir.0.join("plain.json");
    s.save_as(&plain).unwrap();
    let text = std::fs::read_to_string(&plain).unwrap();
    assert!(!text.contains("\"workspace\""), "{text}");
    let mut reopened = Session::new(Store::new());
    reopened.open(&plain).unwrap();
    assert_eq!(workspace(&reopened), None);

    s.set_workspace(Some(DOCUMENT.to_owned())).unwrap().unwrap();
    assert!(s.dirty());
    let kept = dir.0.join("kept.json");
    s.save_as(&kept).unwrap();
    assert!(!s.dirty());
    let text = std::fs::read_to_string(&kept).unwrap();
    assert!(text.contains("\"workspace\""), "{text}");
    let mut reopened = Session::new(Store::new());
    reopened.open(&kept).unwrap();
    assert_eq!(workspace(&reopened).as_deref(), Some(DOCUMENT));
    // The rest of the build is what was saved.
    assert_eq!(reopened.state().unwrap().build, s.state().unwrap().build);
}

#[test]
fn a_workspace_over_the_limit_should_be_refused_and_change_nothing() {
    let mut s = session();
    let version = s.version();
    let before = Arc::clone(s.state().unwrap());

    let e = s
        .set_workspace(Some("a".repeat(MAX_WORKSPACE + 1)))
        .unwrap_err();
    assert_eq!(e.code, "APP_WORKSPACE");
    assert_eq!(s.version(), version);
    assert!(Arc::ptr_eq(&before, s.state().unwrap()));
    assert!(!snapshot(&s).history.can_undo);

    // The limit is in bytes, and a document of exactly the limit is kept.
    assert_eq!(MAX_WORKSPACE, 262_144);
    let e = s
        .set_workspace(Some("é".repeat(MAX_WORKSPACE / 2 + 1)))
        .unwrap_err();
    assert_eq!(e.code, "APP_WORKSPACE");
    let full = "a".repeat(MAX_WORKSPACE);
    s.set_workspace(Some(full.clone())).unwrap().unwrap();
    assert_eq!(workspace(&s), Some(full));
}

#[test]
fn the_workspace_should_not_reach_pobs_formats() {
    let dir = Scratch::new("export");
    let mut s = session();
    let xml = pob_xml::export(&s.state().unwrap().build);
    let code = s.share_code().unwrap();

    s.set_workspace(Some(DOCUMENT.to_owned())).unwrap().unwrap();
    assert_eq!(pob_xml::export(&s.state().unwrap().build), xml);
    assert_eq!(s.share_code().unwrap(), code);
    let path = dir.0.join("export.xml");
    s.export_xml(&path).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert_eq!(written, xml);
    assert!(!written.contains("dashboards"));

    // An import of the export has no workspace: PoB's format has no place
    // for it.
    let mut again = Session::new(Store::new());
    again.set_data(loom_app::GameData::load(&bundle_dir()));
    again.import_file(&path).unwrap();
    assert_eq!(workspace(&again), None);
}

#[test]
fn set_workspace_should_run_as_a_command_of_the_application() {
    let dir = Scratch::new("command");
    let sink: loom_app::Sink = Arc::new(|_| {});
    let mut config = Config::new(dir.0.join("data"), Some(bundle_dir())).with_test_keys(true);
    config.instance = "workspace".to_owned();
    config.autosave = Duration::from_secs(3600);
    let app = App::start(config, sink).unwrap();
    let run = |c: api::Command| app.execute(HostCommand::Api(c));
    let set = |document: Option<&str>| {
        run(api::Command::SetWorkspace {
            document: document.map(str::to_owned),
        })
    };

    // No build: nothing to keep it in.
    assert_eq!(set(Some(DOCUMENT)).unwrap_err().code, "APP_NO_BUILD");

    run(api::Command::ImportText { text: synthetic() }).unwrap();
    let api::Outcome::Done { version } = set(Some(DOCUMENT)).unwrap() else {
        panic!("the workspace was not set");
    };
    let snap = app.snapshot();
    assert_eq!(snap.version, version);
    assert_eq!(
        snap.build.as_ref().unwrap().workspace.as_deref(),
        Some(DOCUMENT)
    );
    assert_eq!(
        snap.history.undo.map(|e| e.kind),
        Some(api::ChangeKind::Workspace)
    );
    assert_eq!(set(Some(DOCUMENT)).unwrap(), api::Outcome::Nothing);
    assert_eq!(
        set(Some(&"a".repeat(MAX_WORKSPACE + 1))).unwrap_err().code,
        "APP_WORKSPACE"
    );
    assert_eq!(app.snapshot().version, version);

    run(api::Command::Undo).unwrap();
    assert_eq!(app.snapshot().build.unwrap().workspace, None);
    run(api::Command::Redo).unwrap();
    assert!(matches!(set(None).unwrap(), api::Outcome::Done { .. }));
    assert_eq!(app.snapshot().build.unwrap().workspace, None);
    app.stop();
}

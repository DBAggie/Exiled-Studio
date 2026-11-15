//! The application's end-to-end workflow (Phase 4 plan, gate 1, decision
//! 14 a) on the synthetic PoE1 bundle and build: import, background
//! calculation, an item edit with its preview, undo and redo, save,
//! reopen, export, share code, reopening without data, recovery, and MCP
//! change requests. Runs on Windows and Linux in CI's `rust` job.

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]
#![expect(clippy::panic, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use loom_app::{App, Config, GroupPlace, HostCommand, Session, api};
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

/// The pipeline's golden document for the synthetic build (written by
/// `calc-run` before the pipeline moved; `loom-pipeline` tests).
fn golden() -> String {
    std::fs::read_to_string(
        root().join("crates/loom-pipeline/tests/testdata/synthetic-poe1.calc.json"),
    )
    .unwrap()
    .replace("\r\n", "\n")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("loom-app-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Running {
    app: App,
    events: Receiver<api::Event>,
}

fn start(data_dir: &Path, bundle: Option<PathBuf>, instance: &str) -> Running {
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let sink: loom_app::Sink = Arc::new(move |e| {
        let _ = tx.lock().unwrap().send(e);
    });
    let mut config = Config::new(data_dir.to_owned(), bundle);
    config.instance = instance.to_owned();
    config.autosave = Duration::from_millis(20);
    Running {
        app: App::start(config, sink).unwrap(),
        events,
    }
}

impl Running {
    /// Waits for an event the predicate accepts.
    fn wait(&self, what: &str, f: impl Fn(&api::Event) -> bool) -> api::Event {
        loop {
            match self.events.recv_timeout(Duration::from_secs(30)) {
                Ok(e) if f(&e) => return e,
                Ok(_) => {}
                Err(_) => panic!("no {what} event"),
            }
        }
    }

    /// Waits until the calculation of `version` finished; its document.
    fn calculated(&self, version: u64) -> String {
        self.wait("calc-finished", |e| {
            e.version == version && matches!(e.kind, api::EventKind::CalcFinished { .. })
        });
        let (v, doc) = self.app.calc_document().unwrap();
        assert_eq!(v, version);
        let snap = self.app.snapshot();
        assert_eq!(snap.calc.state, api::CalcState::Done);
        assert_eq!(snap.calc.version, version);
        doc.to_string()
    }

    fn run(&self, c: api::Command) -> Result<api::Outcome, loom_app::AppError> {
        self.app.execute(HostCommand::Api(c))
    }
}

fn version(o: &api::Outcome) -> u64 {
    match o {
        api::Outcome::Done { version } | api::Outcome::Text { version, .. } => *version,
        api::Outcome::Nothing => panic!("nothing done"),
    }
}

/// An edit of item 1 replacing `from` with `to`.
fn replace(app: &App, from: &str, to: &str) -> api::ItemEdit {
    let item = app.item(1).unwrap();
    let line = item.lines.iter().find(|l| l.text == from).unwrap();
    api::ItemEdit {
        item: 1,
        ops: vec![api::LineOp::Replace {
            line: line.index,
            text: to.to_owned(),
        }],
        expect: vec![api::ExpectedLine {
            line: line.index,
            text: from.to_owned(),
        }],
    }
}

#[test]
fn workflow_should_import_edit_recalculate_undo_save_reopen_and_export() {
    let dir = Scratch::new("workflow");
    let r = start(&dir.path("data"), Some(bundle_dir()), "workflow");

    // Import: the build is shown and calculated in the background.
    let v1 = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    let imported = r.calculated(v1);
    assert_eq!(imported, golden(), "the app calculates as calc-run does");
    let snap = r.app.snapshot();
    let build = snap.build.unwrap();
    assert!(snap.dirty);
    assert_eq!(build.game, "poe1");
    assert!(
        build.lines.warnings > 0,
        "the synthetic build has warning lines"
    );
    assert_eq!(build.warnings.len() as u32, build.lines.warnings);

    // Out-of-scope content: every field has a state, and every field
    // without a value says why (never a blank or a zero; finding 6).
    let doc: serde_json::Value = serde_json::from_str(&imported).unwrap();
    for f in doc["result"]["fields"].as_array().unwrap() {
        match f["state"].as_str().unwrap() {
            "unsupported" => assert!(f["reason"].is_object() || f["reason"].is_string(), "{f}"),
            "indeterminate" => assert!(!f["indeterminate"].as_array().unwrap().is_empty(), "{f}"),
            "complete" | "partial" => assert!(f["value"].is_number(), "{f}"),
            other => panic!("{other}"),
        }
    }

    // Preview, then apply an item edit as one undoable step.
    let mut edit = replace(&r.app, "+15 to maximum Life", "+25 to maximum Life");
    // The synthetic bundle maps no stat to a field, so a value edit alone
    // changes no number; removing a line changes the result's reasons.
    let accuracy = r
        .app
        .item(1)
        .unwrap()
        .lines
        .into_iter()
        .find(|l| l.text == "+12 to Accuracy")
        .unwrap();
    edit.ops.push(api::LineOp::Remove {
        line: accuracy.index,
    });
    edit.expect.push(api::ExpectedLine {
        line: accuracy.index,
        text: accuracy.text,
    });
    let preview = r.app.preview(&edit).unwrap();
    assert!(!preview.fields.is_empty());
    let edited_line = preview
        .item
        .lines
        .iter()
        .find(|l| l.text == "+25 to maximum Life")
        .unwrap();
    assert_eq!(edited_line.status, api::LineStatus::Matched);
    assert_eq!(edited_line.stats[0].value, "25");
    let v2 = version(&r.run(api::Command::EditItem(edit.clone())).unwrap());
    assert!(v2 > v1);
    let edited = r.calculated(v2);
    assert_ne!(edited, imported, "the edit changes the calculation");
    let history = r.app.snapshot().history;
    assert!(history.can_undo);
    assert_eq!(
        history.undo.unwrap(),
        api::HistoryEntry {
            kind: api::ChangeKind::ItemEdit,
            item: 1,
            request: None
        }
    );

    // The same edit again is stale: its line is no longer the one it names.
    let stale = r.run(api::Command::EditItem(edit)).unwrap_err();
    assert_eq!(stale.code, "APP_STALE_EDIT");

    // Undo and redo give the exact calculations back (finding 3).
    let v3 = version(&r.run(api::Command::Undo).unwrap());
    assert_eq!(r.calculated(v3), imported);
    let v4 = version(&r.run(api::Command::Redo).unwrap());
    assert_eq!(r.calculated(v4), edited);
    assert!(matches!(
        r.run(api::Command::Redo).unwrap(),
        api::Outcome::Nothing
    ));

    // Save, then reopen: the same build and the same calculation.
    let saved = dir.path("build.json");
    r.app.execute(HostCommand::SaveAs(saved.clone())).unwrap();
    r.wait("saved", |e| matches!(e.kind, api::EventKind::Saved { .. }));
    let snap = r.app.snapshot();
    assert!(!snap.dirty);
    assert_eq!(snap.build.unwrap().name, "build");
    assert_eq!(r.app.recent().len(), 1);
    let v5 = version(&r.app.execute(HostCommand::Open(saved.clone())).unwrap());
    assert_eq!(
        r.calculated(v5),
        edited,
        "reopen calculates as before (decision 7)"
    );
    assert!(r.app.snapshot().notices.is_empty());

    // Save in place works; a recent build opens by its ID.
    version(&r.run(api::Command::Save).unwrap());
    let id = r.app.recent()[0].id.clone();
    let v6 = version(&r.run(api::Command::OpenRecent { id }).unwrap());
    assert_eq!(r.calculated(v6), edited);
    let unknown = r
        .run(api::Command::OpenRecent {
            id: "../../x".to_owned(),
        })
        .unwrap_err();
    assert_eq!(unknown.code, "APP_NOT_FOUND");

    // A recent build can be taken off the list; its file stays (UX-20).
    let id = r.app.recent()[0].id.clone();
    let file = PathBuf::from(&r.app.recent()[0].path);
    r.run(api::Command::ForgetRecent { id: id.clone() })
        .unwrap();
    assert!(r.app.recent().is_empty());
    assert!(file.exists());
    let gone = r.run(api::Command::ForgetRecent { id }).unwrap_err();
    assert_eq!(gone.code, "APP_NOT_FOUND");

    // Export PoB XML: it reads back with the edited line and the rest as
    // the export wrote it.
    let xml_path = dir.path("export.xml");
    r.app
        .execute(HostCommand::ExportXml(xml_path.clone()))
        .unwrap();
    let xml = std::fs::read_to_string(&xml_path).unwrap();
    assert!(xml.contains("+25 to maximum Life"));
    let again = pob_xml::import(&xml).unwrap();
    assert!(pob_xml::xml_equal(&xml, &pob_xml::export(&again)).unwrap());

    // The share code imports to the same calculation.
    let code = match r.run(api::Command::ExportShareCode).unwrap() {
        api::Outcome::Text { text, .. } => text,
        other => panic!("{other:?}"),
    };
    let v7 = version(&r.run(api::Command::ImportText { text: code }).unwrap());
    assert_eq!(r.calculated(v7), edited);

    // Close.
    r.run(api::Command::Close).unwrap();
    assert!(r.app.snapshot().build.is_none());
}

#[test]
fn import_from_the_original_file_should_never_be_overwritten() {
    let dir = Scratch::new("origin");
    let r = start(&dir.path("data"), Some(bundle_dir()), "origin");
    let original = dir.path("original.xml");
    std::fs::write(&original, synthetic()).unwrap();
    let v = version(
        &r.app
            .execute(HostCommand::ImportFile(original.clone()))
            .unwrap(),
    );
    r.calculated(v);
    assert_eq!(r.app.snapshot().build.unwrap().name, "original");
    let e = r
        .app
        .execute(HostCommand::ExportXml(original.clone()))
        .unwrap_err();
    assert_eq!(e.code, "APP_EXPORT_ORIGIN");
    let e = r
        .app
        .execute(HostCommand::SaveAs(original.clone()))
        .unwrap_err();
    assert_eq!(e.code, "PERSIST_IMPORT_SOURCE");
    assert_eq!(std::fs::read_to_string(&original).unwrap(), synthetic());
}

#[test]
fn opening_without_data_should_show_the_build_and_say_why_nothing_is_calculated() {
    let dir = Scratch::new("nodata");
    let saved = dir.path("build.json");
    {
        let r = start(&dir.path("data"), Some(bundle_dir()), "nodata-1");
        let v = version(
            &r.run(api::Command::ImportText { text: synthetic() })
                .unwrap(),
        );
        r.calculated(v);
        r.app.execute(HostCommand::SaveAs(saved.clone())).unwrap();
    }
    // The synchronous core: open needs no data.
    let mut session = Session::new(Store::new());
    session.open(&saved).unwrap();
    assert!(session.snapshot(idle()).build.is_some());
    assert_eq!(session.calc_input().unwrap_err().code, "APP_NO_DATA");
    // The application: the build is shown and the calculation fails with
    // the code, as a state.
    let r = start(&dir.path("data"), None, "nodata-2");
    r.app.execute(HostCommand::Open(saved)).unwrap();
    let e = r.wait("calc-failed", |e| {
        matches!(e.kind, api::EventKind::CalcFailed { .. })
    });
    match e.kind {
        api::EventKind::CalcFailed { error } => assert_eq!(error.code, "APP_NO_DATA"),
        _ => unreachable!(),
    }
    let snap = r.app.snapshot();
    assert!(snap.build.is_some());
    assert_eq!(snap.calc.state, api::CalcState::Failed);
    assert_eq!(snap.calc.error.unwrap().code, "APP_NO_DATA");
    // Import needs data.
    let e = r
        .run(api::Command::ImportText { text: synthetic() })
        .unwrap_err();
    assert_eq!(e.code, "APP_NO_DATA");
}

fn idle() -> api::CalcStatus {
    api::CalcStatus {
        state: api::CalcState::Idle,
        version: 0,
        result_state: None,
        error: None,
    }
}

#[test]
fn unsaved_changes_should_be_recoverable_after_the_app_ends() {
    let dir = Scratch::new("recovery");
    let data = dir.path("data");
    let edited;
    let original;
    {
        let r = start(&data, Some(bundle_dir()), "first");
        let v = version(
            &r.run(api::Command::ImportText { text: synthetic() })
                .unwrap(),
        );
        original = r.calculated(v);
        let edit = replace(&r.app, "+15 to maximum Life", "+40 to maximum Life");
        let v = version(&r.run(api::Command::EditItem(edit)).unwrap());
        edited = r.calculated(v);
        // The app ends with unsaved changes (dropped: no save, no close).
    }
    let r = start(&data, Some(bundle_dir()), "second");
    let found = r.app.recovery();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "first");
    let v = version(
        &r.run(api::Command::Restore {
            id: "first".to_owned(),
        })
        .unwrap(),
    );
    assert_eq!(
        r.calculated(v),
        edited,
        "a restore goes through the open path"
    );
    assert!(r.app.snapshot().dirty);
    assert!(r.app.recovery().is_empty());
    // The undo history came back with it (UX-19): undo reaches the import.
    let history = r.app.snapshot().history;
    assert!(history.can_undo, "the edit can be undone after a restore");
    let v = version(&r.run(api::Command::Undo).unwrap());
    assert_eq!(r.calculated(v), original, "undo reaches the imported build");
    assert!(!r.app.snapshot().history.can_undo);
    assert_eq!(
        r.run(api::Command::Restore {
            id: "first".to_owned()
        })
        .unwrap_err()
        .code,
        "APP_NOT_FOUND"
    );
}

#[test]
fn a_saved_build_should_leave_no_recovery_snapshot() {
    let dir = Scratch::new("saved-recovery");
    let data = dir.path("data");
    {
        let r = start(&data, Some(bundle_dir()), "one");
        let v = version(
            &r.run(api::Command::ImportText { text: synthetic() })
                .unwrap(),
        );
        r.calculated(v);
        std::thread::sleep(Duration::from_millis(100));
        r.app
            .execute(HostCommand::SaveAs(dir.path("b.json")))
            .unwrap();
    }
    let r = start(&data, Some(bundle_dir()), "two");
    assert!(r.app.recovery().is_empty());
}

#[test]
fn rapid_changes_should_end_with_the_latest_versions_calculation() {
    let dir = Scratch::new("rapid");
    let r = start(&dir.path("data"), Some(bundle_dir()), "rapid");
    let mut last = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    for _ in 0..10 {
        last = version(
            &r.run(api::Command::ImportText { text: synthetic() })
                .unwrap(),
        );
    }
    r.calculated(last);
    // No finished calculation is ever older than a change already made
    // (stale results are discarded).
    let snap = r.app.snapshot();
    assert_eq!(snap.version, last);
    assert_eq!(snap.calc.version, last);
}

#[test]
fn change_requests_should_follow_the_users_permissions() {
    let dir = Scratch::new("requests");
    let r = start(&dir.path("data"), Some(bundle_dir()), "requests");
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);
    let edit = replace(&r.app, "+15 to maximum Life", "+20 to maximum Life");

    // Read-only by default.
    let e = r
        .app
        .request_change("client", edit.clone(), None)
        .unwrap_err();
    assert_eq!(e.code, "APP_REQUESTS_OFF");

    // Requests on: the request waits, with the effect the app calculated.
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: false,
    });
    r.wait("mcp", |e| matches!(e.kind, api::EventKind::McpChanged));
    let req = r
        .app
        .request_change("client", edit.clone(), Some("why".to_owned()))
        .unwrap();
    assert_eq!(req.status, api::RequestStatus::Pending);
    assert!(req.preview.is_some());
    assert_eq!(r.app.snapshot().version, v, "nothing applied yet");

    // Approve: applied through the edit command, undoable, in the history.
    let v2 = version(&r.run(api::Command::Approve { id: req.id }).unwrap());
    r.calculated(v2);
    assert_eq!(
        r.app.request(req.id).unwrap().status,
        api::RequestStatus::Applied
    );
    assert_eq!(
        r.app.snapshot().history.undo.unwrap(),
        api::HistoryEntry {
            kind: api::ChangeKind::RequestApplied,
            item: 1,
            request: Some(req.id)
        }
    );
    assert_eq!(
        r.run(api::Command::Approve { id: req.id })
            .unwrap_err()
            .code,
        "APP_REQUEST"
    );

    // A request made against lines that changed meanwhile is stale.
    let stale = r.app.request_change("client", edit, None);
    let stale = match stale {
        Ok(r) => panic!("{r:?}"),
        Err(e) => e,
    };
    assert_eq!(stale.code, "APP_STALE_EDIT");

    // Decline.
    let edit = replace(&r.app, "+20 to maximum Life", "+21 to maximum Life");
    let req = r.app.request_change("client", edit.clone(), None).unwrap();
    r.run(api::Command::Decline { id: req.id }).unwrap();
    assert_eq!(
        r.app.request(req.id).unwrap().status,
        api::RequestStatus::Declined
    );

    // A pending request whose lines change before approval fails as stale.
    let req = r.app.request_change("client", edit.clone(), None).unwrap();
    let v3 = version(&r.run(api::Command::EditItem(edit.clone())).unwrap());
    r.calculated(v3);
    assert_eq!(
        r.run(api::Command::Approve { id: req.id })
            .unwrap_err()
            .code,
        "APP_STALE_EDIT"
    );
    assert_eq!(
        r.app.request(req.id).unwrap().status,
        api::RequestStatus::Failed
    );

    // The 17th pending request is refused.
    let edit = replace(&r.app, "+21 to maximum Life", "+22 to maximum Life");
    for _ in 0..loom_app::MAX_PENDING_REQUESTS {
        r.app.request_change("client", edit.clone(), None).unwrap();
    }
    assert_eq!(
        r.app
            .request_change("client", edit.clone(), None)
            .unwrap_err()
            .code,
        "APP_REQUESTS_FULL"
    );
    let pending: Vec<u64> = r
        .app
        .mcp_status()
        .requests
        .iter()
        .filter(|q| q.status == api::RequestStatus::Pending)
        .map(|q| q.id)
        .collect();
    for id in pending {
        r.run(api::Command::Decline { id }).unwrap();
    }

    // A standing grant (only while the server runs) applies at once;
    // revoking it takes effect next.
    r.app
        .set_endpoint(Some(("http://127.0.0.1:1/mcp".to_owned(), "t".to_owned())));
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: true,
    });
    r.wait("mcp", |e| matches!(e.kind, api::EventKind::McpChanged));
    let req = r.app.request_change("client", edit, None).unwrap();
    assert_eq!(req.status, api::RequestStatus::Applied);
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: false,
    });
    r.wait("mcp", |e| matches!(e.kind, api::EventKind::McpChanged));
    let edit = replace(&r.app, "+22 to maximum Life", "+23 to maximum Life");
    let req = r.app.request_change("client", edit, None).unwrap();
    assert_eq!(req.status, api::RequestStatus::Pending);

    // Stopping the server ends the standing grant.
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: true,
    });
    r.app
        .set_endpoint(Some(("http://127.0.0.1:1/mcp".to_owned(), "t".to_owned())));
    r.app.set_endpoint(None);
    r.wait("mcp", |e| matches!(e.kind, api::EventKind::McpChanged));
    r.wait("mcp", |e| matches!(e.kind, api::EventKind::McpChanged));
    r.wait("mcp", |e| matches!(e.kind, api::EventKind::McpChanged));
    assert!(!r.app.mcp_status().permissions.standing_grant);
}

#[test]
fn a_roll_edit_should_keep_the_range_through_export() {
    let dir = Scratch::new("roll");
    let r = start(&dir.path("data"), Some(bundle_dir()), "roll");
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);
    let add = api::ItemEdit {
        item: 1,
        ops: vec![api::LineOp::Add {
            text: "+(10-20) to maximum Life".to_owned(),
        }],
        expect: Vec::new(),
    };
    let v = version(&r.run(api::Command::EditItem(add)).unwrap());
    r.calculated(v);
    let line = r
        .app
        .item(1)
        .unwrap()
        .lines
        .into_iter()
        .find(|l| l.text == "+(10-20) to maximum Life")
        .unwrap();
    let roll = api::ItemEdit {
        item: 1,
        ops: vec![api::LineOp::SetRange {
            line: line.index,
            value: 0.25,
        }],
        expect: vec![api::ExpectedLine {
            line: line.index,
            text: line.text,
        }],
    };
    let v = version(&r.run(api::Command::EditItem(roll)).unwrap());
    r.calculated(v);
    let xml_path = dir.path("roll.xml");
    r.app
        .execute(HostCommand::ExportXml(xml_path.clone()))
        .unwrap();
    let xml = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        xml.contains("{range:0.25}+(10-20) to maximum Life"),
        "{xml}"
    );
    // Re-imported, the line keeps its range and roll.
    let v = version(&r.run(api::Command::ImportText { text: xml }).unwrap());
    r.calculated(v);
    let item = r.app.item(1).unwrap();
    assert!(
        item.lines
            .iter()
            .any(|l| l.text == "{range:0.25}+(10-20) to maximum Life"
                && l.status == api::LineStatus::Matched)
    );
}

#[test]
fn a_pending_request_should_fail_when_another_build_is_opened() {
    let dir = Scratch::new("generation");
    let r = start(&dir.path("data"), Some(bundle_dir()), "generation");
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);
    r.app
        .set_endpoint(Some(("http://127.0.0.1:1/mcp".to_owned(), "t".to_owned())));
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: false,
    });
    let edit = replace(&r.app, "+15 to maximum Life", "+20 to maximum Life");
    let req = r.app.request_change("client", edit, None).unwrap();
    // The same text imported again is another build with identical lines.
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);
    let after = r.app.request(req.id).unwrap();
    assert_eq!(after.status, api::RequestStatus::Failed);
    assert_eq!(after.error.unwrap().code, "APP_STALE_EDIT");
    assert_eq!(
        r.run(api::Command::Approve { id: req.id })
            .unwrap_err()
            .code,
        "APP_REQUEST"
    );
}

#[test]
fn a_grant_should_not_outlive_the_server() {
    let dir = Scratch::new("grant");
    let r = start(&dir.path("data"), Some(bundle_dir()), "grant");
    // No server: a grant is not kept.
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: true,
    });
    assert!(!r.app.mcp_status().permissions.standing_grant);
    r.app
        .set_endpoint(Some(("http://127.0.0.1:1/mcp".to_owned(), "t".to_owned())));
    r.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: true,
    });
    assert!(r.app.mcp_status().permissions.standing_grant);
    // A restart ends it.
    r.app
        .set_endpoint(Some(("http://127.0.0.1:2/mcp".to_owned(), "u".to_owned())));
    assert!(!r.app.mcp_status().permissions.standing_grant);
}

#[test]
fn an_edit_should_refuse_lines_that_change_the_items_structure() {
    let dir = Scratch::new("structure");
    let r = start(&dir.path("data"), Some(bundle_dir()), "structure");
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);
    for text in ["Corrupted".to_owned(), "x".repeat(501)] {
        let add = api::ItemEdit {
            item: 1,
            ops: vec![api::LineOp::Add { text }],
            expect: Vec::new(),
        };
        assert_eq!(
            r.run(api::Command::EditItem(add)).unwrap_err().code,
            "APP_EDIT_LINE"
        );
    }
    let many = api::ItemEdit {
        item: 1,
        ops: (0..51)
            .map(|i| api::LineOp::Add {
                text: format!("+{i} to maximum Life"),
            })
            .collect(),
        expect: Vec::new(),
    };
    assert_eq!(
        r.run(api::Command::EditItem(many)).unwrap_err().code,
        "APP_EDIT_LINE"
    );
}

#[test]
fn a_save_should_not_recalculate() {
    let dir = Scratch::new("save-no-calc");
    let r = start(&dir.path("data"), Some(bundle_dir()), "save-no-calc");
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);
    r.app
        .execute(HostCommand::SaveAs(dir.path("b.json")))
        .unwrap();
    let snap = r.app.snapshot();
    assert_eq!(snap.version, v);
    assert_eq!(snap.calc.state, api::CalcState::Done);
    assert!(!snap.dirty);
}

/// UX walkthrough, UX-02 and UX-03: the view says which items and
/// warnings are in use, lists the item sets, and the user can make
/// another set active as one undoable change.
#[test]
fn item_sets_should_be_listed_and_switchable() {
    let xml = synthetic().replace(
        "</ItemSet>",
        "</ItemSet><ItemSet id=\"2\" title=\"Bare\" useSecondWeaponSet=\"false\">\
         <Slot name=\"Ring 1\" itemId=\"0\"/></ItemSet>",
    );
    assert!(
        xml.contains("title=\"Bare\""),
        "the second set was not added"
    );
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();

    let build = session.snapshot(idle()).build.unwrap();
    let sets: Vec<(u32, bool)> = build.item_sets.iter().map(|s| (s.id, s.active)).collect();
    assert_eq!(sets, [(1, true), (2, false)]);
    let ring = build.items.iter().find(|i| i.id == 1).unwrap();
    assert!(ring.in_use);
    assert_eq!(ring.sets, [1]);
    assert!(build.warnings.iter().all(|w| w.in_use == (w.item == 1)));

    let v = session.use_item_set(2).unwrap().unwrap();
    let build = session.snapshot(idle()).build.unwrap();
    assert!(build.item_sets.iter().any(|s| s.id == 2 && s.active));
    assert!(!build.items.iter().find(|i| i.id == 1).unwrap().in_use);
    // No item in use, so no warning in use: complete for the calculation,
    // whatever the import's own completeness.
    assert!(build.warnings.iter().all(|w| !w.in_use));
    assert_eq!(build.in_use_completeness, "complete");
    let snap = session.snapshot(idle());
    assert!(snap.dirty);
    assert_eq!(snap.version, v);
    assert_eq!(
        snap.history.undo.as_ref().map(|e| (e.kind.clone(), e.item)),
        Some((api::ChangeKind::ItemSet, 2))
    );
    // The active set again: no change.
    assert_eq!(session.use_item_set(2).unwrap(), None);
    assert_eq!(session.use_item_set(9).unwrap_err().code, "APP_ITEM_SET");

    session.undo().unwrap();
    let build = session.snapshot(idle()).build.unwrap();
    assert!(build.item_sets.iter().any(|s| s.id == 1 && s.active));
}

/// The synthetic build with two stages (PoB's loadouts): two specs, two
/// skill sets, and two item sets linked by `{1}` and `{2}`.
fn staged() -> String {
    let skill = |name: &str, level: u32| {
        format!(
            "<Skill enabled=\"true\" slot=\"Weapon 1\" mainActiveSkill=\"1\">\
             <Gem nameSpec=\"{name}\" skillId=\"{name}\" level=\"{level}\" quality=\"0\" \
             enabled=\"true\"/></Skill>"
        )
    };
    let xml = synthetic()
        .replace(
            "<Spec treeVersion=\"3_29\" classId=\"0\" ascendClassId=\"0\" nodes=\"\"/>",
            "<Spec title=\"^4Leveling {1}\" treeVersion=\"3_29\" classId=\"0\" ascendClassId=\"0\" nodes=\"\"/>\
             <Spec title=\"Endgame {2}\" treeVersion=\"3_29\" classId=\"0\" ascendClassId=\"0\" nodes=\"\"/>",
        )
        .replace(
            "<Skills/>",
            &format!(
                "<Skills activeSkillSet=\"1\"><SkillSet id=\"1\" title=\"Leveling {{1}}\">{}{}</SkillSet>\
                 <SkillSet id=\"2\" title=\"Endgame {{2}}\">{}</SkillSet></Skills>",
                skill("Smite", 1),
                skill("Leap Slam", 1),
                skill("Smite", 20),
            ),
        )
        .replace("<ItemSet id=\"1\" ", "<ItemSet id=\"1\" title=\"Leveling {1}\" ")
        .replace(
            "</ItemSet>",
            "</ItemSet><ItemSet id=\"2\" title=\"Endgame {2}\" useSecondWeaponSet=\"false\">\
             <Slot name=\"Ring 1\" itemId=\"0\"/></ItemSet>",
        );
    assert!(
        xml.contains("Endgame {2}\" tree"),
        "the second spec was not added"
    );
    assert!(
        xml.contains("SkillSet id=\"2\""),
        "the skill sets were not added"
    );
    assert!(
        xml.contains("ItemSet id=\"1\" title"),
        "the item set was not titled"
    );
    xml
}

/// The maintainer's global stage choice (2026-11-14): one stage sets the
/// tree, the item set, and the skill set together, as one undoable change.
#[test]
fn a_stage_should_switch_the_tree_items_and_skills_together() {
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&staged(), None).unwrap();

    let build = session.snapshot(idle()).build.unwrap();
    let stages: Vec<_> = build
        .loadouts
        .iter()
        .map(|l| (l.spec, l.title.as_str(), l.item_set, l.skill_set, l.active))
        .collect();
    assert_eq!(
        stages,
        [
            (1, "Leveling", Some(1), Some(1), true),
            (2, "Endgame", Some(2), Some(2), false),
        ]
    );
    assert_eq!(build.skill_groups.len(), 2);

    session.use_loadout(2).unwrap().unwrap();
    let build = session.snapshot(idle()).build.unwrap();
    assert!(build.loadouts.iter().any(|l| l.spec == 2 && l.active));
    assert!(build.specs.iter().any(|s| s.id == 2 && s.active));
    assert!(build.item_sets.iter().any(|s| s.id == 2 && s.active));
    assert!(build.skill_sets.iter().any(|s| s.id == 2 && s.active));
    assert_eq!(build.skill_groups.len(), 1);
    assert_eq!(build.skill_groups[0].gems[0].level, 20);
    assert_eq!(
        session
            .snapshot(idle())
            .history
            .undo
            .map(|e| (e.kind, e.item)),
        Some((api::ChangeKind::Loadout, 2))
    );
    assert_eq!(session.use_loadout(2).unwrap(), None);
    assert_eq!(session.use_loadout(7).unwrap_err().code, "APP_LOADOUT");

    session.undo().unwrap();
    let build = session.snapshot(idle()).build.unwrap();
    assert!(build.loadouts.iter().any(|l| l.spec == 1 && l.active));
}

/// The Skills page draws each slot's sockets: an item's link groups from
/// its `Sockets` line, each gem's socket (a slot's groups fill its sockets
/// in order, as PoB does), and which actives each support supports.
#[test]
fn sockets_and_support_links_should_be_shown() {
    let gem = |id: &str, skill: &str| {
        format!(
            "<Gem nameSpec=\"{skill}\" gemId=\"{id}\" skillId=\"{skill}\" level=\"1\" \
             quality=\"0\" enabled=\"true\"/>"
        )
    };
    let bolt = gem("Test/Gems/Bolt", "TestBolt");
    let support = gem("Test/Gems/SupportDamage", "SupportTestDamage");
    let xml = synthetic()
        .replace("Plain Test Ring\n", "Plain Test Ring\nSockets: B-R G\n")
        .replace(
            "<Skills/>",
            &format!(
                "<Skills activeSkillSet=\"1\"><SkillSet id=\"1\">\
                 <Skill enabled=\"true\" slot=\"Ring 1\" mainActiveSkill=\"1\">{bolt}{support}</Skill>\
                 <Skill enabled=\"true\" slot=\"Ring 1\" mainActiveSkill=\"1\">{support}{bolt}</Skill>\
                 <Skill enabled=\"true\" mainActiveSkill=\"1\">{bolt}</Skill>\
                 </SkillSet></Skills>"
            ),
        );
    assert!(xml.contains("Sockets: B-R G"), "the sockets were not added");
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();
    let build = session.snapshot(idle()).build.unwrap();

    let ring = build.items.iter().find(|i| i.id == 1).unwrap();
    assert_eq!(ring.sockets, ["BR", "G"]);
    let other = build.items.iter().find(|i| i.id == 2).unwrap();
    assert!(other.sockets.is_empty());

    let groups = &build.skill_groups;
    let view = |g: usize| {
        groups[g]
            .gems
            .iter()
            .map(|gem| (gem.support, gem.socket, gem.supports.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        view(0),
        [
            (Some(false), Some(0), vec![]),
            (Some(true), Some(1), vec![0])
        ]
    );
    // The second group goes on: socket 2 is the last; its support names
    // the active by its index in its own group.
    assert_eq!(
        view(1),
        [(Some(true), Some(2), vec![1]), (Some(false), None, vec![])]
    );
    // No slot, no socket.
    assert_eq!(view(2), [(Some(false), None, vec![])]);
}

/// A build planner adds and removes gems and socket groups, picks gems
/// from the game data's catalog, and moves or renames a group; the main
/// skill stays on its gem, and every change reaches the export.
#[test]
fn gems_and_groups_should_be_added_and_removed() {
    let bolt = "<Gem nameSpec=\"Test Bolt\" gemId=\"Test/Gems/Bolt\" skillId=\"TestBolt\" \
                level=\"1\" quality=\"0\" enabled=\"true\"/>";
    let xml = synthetic().replace(
        "<Skills/>",
        &format!(
            "<Skills activeSkillSet=\"1\"><SkillSet id=\"1\">\
             <Skill enabled=\"true\" slot=\"Ring 1\" mainActiveSkill=\"1\">{bolt}</Skill>\
             </SkillSet></Skills>"
        ),
    );
    let dir = Scratch::new("gem-add");
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();
    let bundle = session.data().unwrap().bundle;

    // The catalog: both gems, the support marked as fitting the group.
    let group = session
        .state()
        .unwrap()
        .build
        .skills
        .sets
        .typed()
        .next()
        .unwrap();
    let group = group.groups.typed().next().unwrap().clone();
    let catalog = loom_app::gems::catalog(bundle, Some(&group));
    let entry = |id: &str| catalog.iter().find(|e| e.id == id).unwrap();
    assert_eq!(entry("TestBolt").name, "Test Bolt");
    assert_eq!(entry("TestBolt").fits, None);
    assert!(entry("SupportTestDamage").support);
    assert_eq!(entry("SupportTestDamage").fits, Some(true));
    assert!(
        loom_app::gems::catalog(bundle, None)
            .iter()
            .all(|e| e.fits.is_none())
    );

    // Add a support at the end and an active in front: the main skill
    // stays on the first bolt, now the second gem.
    session
        .add_gem(0, "SupportTestDamage", None)
        .unwrap()
        .unwrap();
    session.add_gem(0, "TestBolt", Some(0)).unwrap().unwrap();
    let groups = session.snapshot(idle()).build.unwrap().skill_groups;
    let gems: Vec<_> = groups[0]
        .gems
        .iter()
        .map(|g| (g.name.as_deref().unwrap_or(""), g.level, g.main))
        .collect();
    assert_eq!(
        gems,
        [
            ("Test Bolt", 2, false),
            ("Test Bolt", 1, true),
            ("Test Damage Support", 1, false)
        ]
    );
    assert_eq!(
        session.add_gem(0, "NoSuchGem", None).unwrap_err().code,
        "APP_SKILL"
    );

    // Remove the main gem: the main skill falls to the first active.
    session.remove_gem(0, 1).unwrap().unwrap();
    let groups = session.snapshot(idle()).build.unwrap().skill_groups;
    assert_eq!(groups[0].gems.len(), 2);
    assert!(groups[0].gems[0].main);
    assert_eq!(session.remove_gem(0, 9).unwrap_err().code, "APP_SKILL");

    // A new group, moved and renamed; then removed.
    session
        .add_socket_group(Some("Ring 2".to_owned()), None)
        .unwrap()
        .unwrap();
    session
        .edit_socket_group(
            1,
            None,
            None,
            None,
            GroupPlace {
                slot: Some(String::new()),
                label: Some("Auras".to_owned()),
            },
        )
        .unwrap()
        .unwrap();
    let groups = session.snapshot(idle()).build.unwrap().skill_groups;
    assert_eq!(groups.len(), 2);
    assert_eq!(
        (groups[1].label.as_deref(), groups[1].slot.as_deref()),
        (Some("Auras"), None)
    );
    let out = dir.path("added.xml");
    session.export_xml(&out).unwrap();
    let xml = std::fs::read_to_string(&out).unwrap();
    assert!(xml.contains("gemId=\"Test/Gems/SupportDamage\""), "{xml}");
    assert!(xml.contains("label=\"Auras\""), "{xml}");
    session.remove_socket_group(1).unwrap().unwrap();
    assert_eq!(
        session.snapshot(idle()).build.unwrap().skill_groups.len(),
        1
    );
    assert_eq!(
        session.remove_socket_group(5).unwrap_err().code,
        "APP_SKILL"
    );

    // Each was one undoable step.
    session.undo().unwrap();
    assert_eq!(
        session.snapshot(idle()).build.unwrap().skill_groups.len(),
        2
    );
}

/// A build planner edits gems: level, quality, enabled, the group's
/// state, and the main skill; each is undoable and reaches the export.
#[test]
fn gems_and_groups_should_be_editable() {
    let dir = Scratch::new("gem-edits");
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&staged(), None).unwrap();
    let build = session.snapshot(idle()).build.unwrap();
    assert_eq!(build.main_skill.as_deref(), Some("Smite"));
    // Imported from text, so no file name: named from the build (UX-15;
    // the ascendancy "None" is no name).
    assert_eq!(build.name, "Smite Scion 1");
    let groups = build.skill_groups;
    assert!(groups[0].main && !groups[1].main);
    assert!(groups[0].gems[0].main && !groups[1].gems[0].main);
    assert_eq!(
        session
            .edit_socket_group(0, None, None, Some(4), GroupPlace::default())
            .unwrap_err()
            .code,
        "APP_SKILL"
    );

    session
        .edit_gem(0, 0, Some(21), Some(23), Some(false))
        .unwrap()
        .unwrap();
    session
        .edit_socket_group(1, Some(false), Some(true), None, GroupPlace::default())
        .unwrap()
        .unwrap();
    let groups = session.snapshot(idle()).build.unwrap().skill_groups;
    let gem = &groups[0].gems[0];
    assert_eq!((gem.level, gem.quality, gem.enabled), (21, 23, false));
    assert!(!groups[0].main && groups[1].main && !groups[1].enabled);
    assert!(groups[1].gems[0].main);
    assert_eq!(
        session.snapshot(idle()).history.undo.map(|e| e.kind),
        Some(api::ChangeKind::SkillEdit)
    );
    // The same values again change nothing; bad indexes and level 0 fail.
    assert_eq!(session.edit_gem(0, 0, Some(21), None, None).unwrap(), None);
    assert_eq!(
        session
            .edit_gem(0, 5, Some(2), None, None)
            .unwrap_err()
            .code,
        "APP_SKILL"
    );
    assert_eq!(
        session
            .edit_gem(9, 0, Some(2), None, None)
            .unwrap_err()
            .code,
        "APP_SKILL"
    );
    assert_eq!(
        session
            .edit_gem(0, 0, Some(0), None, None)
            .unwrap_err()
            .code,
        "APP_SKILL"
    );

    let out = dir.path("out.xml");
    session.export_xml(&out).unwrap();
    let xml = std::fs::read_to_string(&out).unwrap();
    assert!(
        xml.contains("level=\"21\" quality=\"23\" enabled=\"false\""),
        "{xml}"
    );
    assert!(xml.contains("mainSocketGroup=\"2\""));

    session.undo().unwrap();
    session.undo().unwrap();
    let groups = session.snapshot(idle()).build.unwrap().skill_groups;
    assert_eq!(groups[0].gems[0].level, 1);
    assert!(groups[0].main && groups[1].enabled);
}

#[test]
fn gem_names_should_fall_back_to_the_gem_item_and_skip_placeholders() {
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    let mut bundle = session.data().unwrap().bundle.clone();
    let skills = bundle.skills.as_mut().unwrap();
    for effect in &mut skills.gem_effects {
        match effect.id.as_str() {
            // Most gem effects of the real data leave their name to the gem item.
            "TestBolt" => effect.name.clear(),
            "SupportTestDamage" => effect.name = "[DNT] Test Damage".to_owned(),
            _ => {}
        }
    }
    // A later effect of the same gem with the same name (a Vaal gem's
    // leftover `AltX`) is not listed again.
    let mut alt = skills
        .gem_effects
        .iter()
        .find(|e| e.id == "TestBolt")
        .unwrap()
        .clone();
    alt.id = "TestBoltAltX".to_owned();
    skills.gem_effects.push(alt);
    skills
        .gems
        .iter_mut()
        .find(|g| g.effects.iter().any(|e| e == "TestBolt"))
        .unwrap()
        .effects
        .push("TestBoltAltX".to_owned());
    let catalog = loom_app::gems::catalog(&bundle, None);
    assert!(catalog.iter().all(|e| e.id != "TestBoltAltX"));
    let bolt = catalog.iter().find(|e| e.id == "TestBolt").unwrap();
    assert_eq!(bolt.name, "Test Bolt");
    assert!(catalog.iter().all(|e| e.id != "SupportTestDamage"));
    assert_eq!(
        loom_app::gems::new_gem(&bundle, "TestBolt")
            .unwrap()
            .name
            .as_deref(),
        Some("Test Bolt")
    );
    assert!(loom_app::gems::new_gem(&bundle, "SupportTestDamage").is_none());
}

#[test]
fn config_inputs_and_custom_blocks_should_be_edited() {
    use loom_serialization::dto::app::v1::{ConfigValueView, InputEffect, LineStatus};
    let dir = Scratch::new("config");
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&synthetic(), None).unwrap();
    let view = |s: &Session| {
        loom_app::build_view(&s.state().unwrap().build, String::new(), None)
            .config
            .unwrap()
    };

    // A build with no config set gets one; each input says what it does.
    session
        .set_config_input("resistancePenalty", &ConfigValueView::Number(-60.0))
        .unwrap()
        .unwrap();
    session
        .set_config_input("conditionFullLife", &ConfigValueView::Boolean(true))
        .unwrap();
    session
        .set_config_input("somethingNew", &ConfigValueView::Text("x".to_owned()))
        .unwrap();
    let config = view(&session);
    let effect = |name: &str| {
        config
            .inputs
            .iter()
            .find(|i| i.name == name)
            .map(|i| i.effect.clone())
    };
    assert_eq!(effect("resistancePenalty"), Some(InputEffect::Applied));
    assert_eq!(effect("conditionFullLife"), Some(InputEffect::NoEffect));
    assert_eq!(effect("somethingNew"), Some(InputEffect::Unmodelled));
    // The same value again changes nothing; the legacy input is refused.
    assert_eq!(
        session
            .set_config_input("conditionFullLife", &ConfigValueView::Boolean(true))
            .unwrap(),
        None
    );
    assert_eq!(
        session
            .set_config_input("customMods", &ConfigValueView::Text(String::new()))
            .unwrap_err()
            .code,
        "APP_CONFIG"
    );
    session
        .remove_config_input("somethingNew")
        .unwrap()
        .unwrap();
    assert_eq!(view(&session).inputs.len(), 2);
    assert_eq!(
        session
            .remove_config_input("somethingNew")
            .unwrap_err()
            .code,
        "APP_CONFIG"
    );

    // A custom block: its lines read and resolved; a changed text again.
    // A line with no digit is a note (ADR-015 Amendment 2); one with a
    // number that matches nothing is a warning.
    session
        .add_custom_block("+15 to maximum Life\n\nA comment\n+7 to nothing known")
        .unwrap()
        .unwrap();
    let block = &view(&session).blocks[0];
    assert!(block.enabled);
    let lines: Vec<_> = block
        .lines
        .iter()
        .map(|l| (l.text.as_str(), l.status.clone(), l.typed.as_deref()))
        .collect();
    assert_eq!(
        lines,
        [
            ("+15 to maximum Life", LineStatus::Matched, None),
            ("A comment", LineStatus::Matched, Some("custom-note")),
            ("+7 to nothing known", LineStatus::Warning, None),
        ]
    );
    session
        .edit_custom_block(0, Some(false), Some("+15 to maximum Life"))
        .unwrap()
        .unwrap();
    let block = &view(&session).blocks[0];
    assert!(!block.enabled);
    assert_eq!(block.lines.len(), 1);
    assert_eq!(block.lines[0].status, LineStatus::Matched);
    assert_eq!(
        session
            .edit_custom_block(3, Some(true), None)
            .unwrap_err()
            .code,
        "APP_CONFIG"
    );

    // Each change is one undo step, and the export carries them.
    session.undo().unwrap();
    assert!(view(&session).blocks[0].enabled);
    session.redo().unwrap();
    let xml = dir.0.join("config.xml");
    session.export_xml(&xml).unwrap();
    let text = std::fs::read_to_string(&xml).unwrap();
    assert!(text.contains("resistancePenalty"), "{text}");
    assert!(text.contains("+15 to maximum Life"), "{text}");
    session.remove_custom_block(0).unwrap().unwrap();
    assert!(view(&session).blocks.is_empty());
}

#[test]
fn an_item_card_readable_lines_and_a_variant_pick_should_be_shown() {
    use loom_serialization::dto::app::v1::{ItemEdit, LineOp, LineStatus};
    let item = "<Item id=\"9\">\nRarity: UNIQUE\nTest Circle\nPlain Test Ring\n\
                Variant: Old\nVariant: Current\nSelected Variant: 1\nQuality: 20\n\
                Sockets: R-G B\nLevelReq: 30\nImplicits: 0\n\
                {variant:1}{range:0.5}+(10-20) to maximum Life\n\
                {variant:2}{range:1}+(20-30) to maximum Life\n\
                {crafted}+12 to Accuracy\nCorrupted\n</Item>\n";
    let xml = synthetic().replacen("<ItemSet ", &format!("{item}<ItemSet "), 1);
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();

    let view = session.item(9).unwrap();
    let card = &view.card;
    assert_eq!(card.variants, ["Old", "Current"]);
    assert_eq!(card.selected_variant, Some(1));
    assert_eq!(card.quality.as_deref(), Some("20"));
    assert_eq!(card.level_req.as_deref(), Some("30"));
    assert_eq!(card.sockets, ["RG", "B"]);
    assert_eq!(card.flags, ["Corrupted"]);
    let modifier = |v: &loom_serialization::dto::app::v1::ItemView, n: usize| {
        v.lines
            .iter()
            .filter(|l| l.kind == "modifier")
            .nth(n)
            .cloned()
            .unwrap()
    };
    // The roll applied, no markup; the variants and marks named.
    let old = modifier(&view, 0);
    assert_eq!(old.shown.as_deref(), Some("+15 to maximum Life"));
    assert_eq!(old.variants, [1]);
    assert_eq!(old.status, LineStatus::Matched);
    assert_eq!(modifier(&view, 1).status, LineStatus::Excluded);
    let crafted = modifier(&view, 2);
    assert_eq!(crafted.shown.as_deref(), Some("+12 to Accuracy"));
    assert_eq!(crafted.marks, ["crafted"]);

    // Picking the other variant: one undoable edit, the lines read again.
    let pick = |variant| ItemEdit {
        item: 9,
        ops: vec![LineOp::SelectVariant { variant }],
        expect: Vec::new(),
    };
    session.edit(&pick(2), None).unwrap();
    let view = session.item(9).unwrap();
    assert_eq!(view.card.selected_variant, Some(2));
    assert_eq!(modifier(&view, 0).status, LineStatus::Excluded);
    assert_eq!(
        modifier(&view, 1).shown.as_deref(),
        Some("+30 to maximum Life")
    );
    assert_eq!(modifier(&view, 1).status, LineStatus::Matched);
    assert_eq!(
        session.edit(&pick(3), None).unwrap_err().code,
        "APP_EDIT_LINE"
    );
    session.undo().unwrap();
    assert_eq!(session.item(9).unwrap().card.selected_variant, Some(1));

    // An item with no `Selected Variant` line gets one after its variants.
    let xml = xml.replace("Selected Variant: 1\n", "");
    session.import_text(&xml, None).unwrap();
    assert_eq!(session.item(9).unwrap().card.selected_variant, None);
    session.edit(&pick(1), None).unwrap();
    let view = session.item(9).unwrap();
    assert_eq!(view.card.selected_variant, Some(1));
    assert_eq!(modifier(&view, 1).status, LineStatus::Excluded);
}

#[test]
fn a_uniques_card_should_carry_its_flavour_text_when_the_data_has_one() {
    use loom_serialization::dto::app::v1::FlavourLine;
    let unique = |id: u32, name: &str, rarity: &str| {
        format!(
            "<Item id=\"{id}\">\nRarity: {rarity}\n{name}\nPlain Test Ring\n\
             Implicits: 0\n+10 to maximum Life\n</Item>\n"
        )
    };
    let items = [
        unique(9, "Test Flavoured", "UNIQUE"),
        unique(10, "Test Variants", "UNIQUE"),
        unique(11, "Test Flavoured", "RARE"),
        unique(12, "Not In The Data", "UNIQUE"),
        unique(13, "Foulborn Test Flavoured", "UNIQUE"),
        unique(14, "Test Same", "UNIQUE"),
    ]
    .concat();
    let xml = synthetic().replacen("<ItemSet ", &format!("{items}<ItemSet "), 1);
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();

    let line = |text: &str, default_colour| FlavourLine {
        text: text.to_owned(),
        default_colour,
    };
    assert_eq!(
        session.item(9).unwrap().card.flavour,
        [
            line("A made-up line.", false),
            line("Another one.", false),
            line("", false),
            line("A made-up note", true),
        ]
    );
    // Two variants with their own text: which one is not known, so none.
    assert!(session.item(10).unwrap().card.flavour.is_empty());
    // Only a unique has flavour text; a name the data lacks has none.
    assert!(session.item(11).unwrap().card.flavour.is_empty());
    assert!(session.item(12).unwrap().card.flavour.is_empty());
    // Two variants with the same text: that text.
    assert_eq!(
        session.item(14).unwrap().card.flavour,
        [line("One text.", false)]
    );
    // A Foulborn unique reads its plain version's text.
    assert_eq!(
        session.item(13).unwrap().card.flavour,
        session.item(9).unwrap().card.flavour
    );
}

#[test]
fn the_league_should_flag_legacy_versions_and_be_saved() {
    use loom_serialization::dto::app::v1::{League, VariantEra};
    let item = "<Item id=\"9\">\nRarity: UNIQUE\nTest Circle\nPlain Test Ring\n\
                Variant: Pre 3.7.0\nVariant: Current\nSelected Variant: 1\nImplicits: 0\n\
                {variant:1}+10 to maximum Life\n{variant:2}+20 to maximum Life\n</Item>\n";
    let xml = synthetic().replacen("<ItemSet ", &format!("{item}<ItemSet "), 1);
    let dir = Scratch::new("league");
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();
    let view = |s: &Session| loom_app::build_view(&s.state().unwrap().build, String::new(), None);

    // Not chosen reads as a challenge league; the legacy version is named,
    // with the current one to switch to.
    assert_eq!(view(&session).league, League::Challenge);
    let summary = |s: &Session| view(s).items.into_iter().find(|i| i.id == 9).unwrap();
    assert!(summary(&session).legacy);
    let card = session.item(9).unwrap().card;
    assert_eq!(card.variant_eras, [VariantEra::Legacy, VariantEra::Current]);
    assert_eq!(card.current_variant, Some(2));

    // The league is one undoable change, kept in the saved document.
    session.set_league(&League::Standard).unwrap().unwrap();
    assert_eq!(session.set_league(&League::Standard).unwrap(), None);
    let path = dir.0.join("league.json");
    session.save_as(&path).unwrap();
    let mut reopened = Session::new(Store::new());
    reopened.open(&path).unwrap();
    assert_eq!(view(&reopened).league, League::Standard);
    session.undo().unwrap();
    assert_eq!(view(&session).league, League::Challenge);

    // Switching to the current version clears the flag.
    session
        .edit(
            &loom_serialization::dto::app::v1::ItemEdit {
                item: 9,
                ops: vec![loom_serialization::dto::app::v1::LineOp::SelectVariant { variant: 2 }],
                expect: Vec::new(),
            },
            None,
        )
        .unwrap();
    assert!(!summary(&session).legacy);
    assert_eq!(session.item(9).unwrap().card.current_variant, None);
}

#[test]
fn the_weapon_set_should_switch_as_one_undoable_change() {
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&synthetic(), None).unwrap();
    let view = |s: &Session| loom_app::build_view(&s.state().unwrap().build, String::new(), None);
    assert!(!view(&session).second_weapon_set);
    // The first set when nothing is written: no change.
    assert_eq!(session.use_weapon_set(false).unwrap(), None);
    session.use_weapon_set(true).unwrap().unwrap();
    assert!(view(&session).second_weapon_set);
    assert_eq!(session.use_weapon_set(true).unwrap(), None);
    session.undo().unwrap();
    assert!(!view(&session).second_weapon_set);
}

#[test]
fn the_card_should_name_pob_affixes_and_the_base_requirements() {
    use loom_serialization::dto::app::v1::AffixKind;
    let item = "<Item id=\"9\">\nRarity: RARE\nTest Steady Band\nPlain Test Ring\n\
                Prefix: {range:0.5}TestAccuracy1\nPrefix: None\nSuffix: None\n\
                Implicits: 0\n+15 to Accuracy\n</Item>\n";
    let xml = synthetic().replacen("<ItemSet ", &format!("{item}<ItemSet "), 1);
    let mut session = Session::new(Store::new());
    session.set_data(loom_app::GameData::load(&bundle_dir()));
    session.import_text(&xml, None).unwrap();
    let card = session.item(9).unwrap().card;
    let a = &card.affixes;
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].kind, AffixKind::Prefix);
    assert_eq!(a[0].name.as_deref(), Some("Steady"));
    assert_eq!(a[0].tags, ["Attack"]);
    // The accuracy line is the item's last line.
    let view = session.item(9).unwrap();
    let last = view.lines.iter().rfind(|l| l.kind == "modifier").unwrap();
    assert_eq!(a[0].lines, [last.index]);
    // No `LevelReq`: the base's drop level; the base's dexterity.
    let req = card.requirements.unwrap();
    assert_eq!((req.level, req.strength, req.dexterity), (Some(5), 0, 10));
}

#[test]
fn a_new_build_should_start_empty_calculate_save_and_export() {
    let dir = Scratch::new("new-build");
    // Without data there is nothing to make it with.
    let none = start(&dir.path("data0"), None, "new-build-0");
    let err = none.run(api::Command::NewBuild).unwrap_err();
    assert_eq!(err.code, "APP_NO_DATA");
    none.app.stop();

    let r = start(&dir.path("data"), Some(bundle_dir()), "new-build");
    let v = version(&r.run(api::Command::NewBuild).unwrap());
    r.calculated(v);
    let snap = r.app.snapshot();
    assert!(!snap.dirty, "a new build is not dirty until edited");
    let build = snap.build.unwrap();
    assert_eq!(build.game, "poe1");
    assert_eq!(build.class.as_deref(), Some("Scion"));
    assert_eq!(build.level, Some(1));
    assert_eq!(build.ascendancy, None, "PoB's None is no ascendancy");
    assert_eq!(build.name, "Scion 1", "named from itself (UX-15)");
    assert!(build.items.is_empty());
    assert!(build.warnings.is_empty());
    assert_eq!(build.item_sets.len(), 1);
    assert_eq!(build.skill_sets.len(), 1);
    assert_eq!(build.specs.len(), 1);
    assert_eq!(build.config_sets.len(), 1);

    // It saves and exports as any build.
    let saved = dir.path("new.json");
    r.app.execute(HostCommand::SaveAs(saved.clone())).unwrap();
    assert!(saved.exists());
    // The recent list says what it is (UX-20).
    let recent = r.app.recent();
    assert_eq!(recent[0].class.as_deref(), Some("Scion"));
    assert_eq!(recent[0].level, Some(1));
    assert_eq!(recent[0].ascendancy, None, "PoB's None is no ascendancy");
    let xml = dir.path("new.xml");
    r.app.execute(HostCommand::ExportXml(xml.clone())).unwrap();
    let text = std::fs::read_to_string(&xml).unwrap();
    assert!(pob_xml::import(&text).is_ok(), "the export reads back");
    assert!(text.contains(r#"className="Scion""#));
    r.app.stop();
}

/// Copies the synthetic bundle into `<data>/bundles/poe1/<name>/` with its
/// manifest's Steam build set to `steam` (the manifest is not hashed, so the
/// copy is valid; its data version differs).
fn install(data: &Path, name: &str, steam: u64) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let dir = data.join("bundles/poe1").join(name);
    copy(&bundle_dir(), &dir);
    let path = dir.join("bundle.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["source"]["steam_build"] = serde_json::json!(steam);
    std::fs::write(&path, manifest.to_string()).unwrap();
    dir
}

#[test]
fn installed_bundles_should_be_found_switched_remembered_and_offered_for_a_pinned_build() {
    let dir = Scratch::new("installed");
    let data = dir.path("data");
    let old = install(&data, "100-s4", 100);
    install(&data, "200-s4", 200);

    // No folder named, nothing remembered: the newest installed bundle.
    let r = start(&data, None, "installed-1");
    let snap = r.app.snapshot();
    assert!(snap.data.error.is_none());
    assert_eq!(snap.data.active.as_deref(), Some("poe1/200-s4"));
    let ids: Vec<&str> = snap.data.installed.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["poe1/200-s4", "poe1/100-s4"]);
    assert_eq!(snap.data.pinned, None);

    // A build imported now is pinned to it (ADR-010).
    let v = version(
        &r.run(api::Command::ImportText { text: synthetic() })
            .unwrap(),
    );
    r.calculated(v);

    // An unknown ID changes nothing.
    let e = r
        .run(api::Command::UseBundle {
            id: "poe1/1-s4".to_owned(),
        })
        .unwrap_err();
    assert_eq!(e.code, "APP_NO_BUNDLE");
    assert_eq!(r.app.snapshot().data.active.as_deref(), Some("poe1/200-s4"));

    // Switching: the build's data is now another installed bundle, which
    // is offered, never taken silently.
    r.run(api::Command::UseBundle {
        id: "poe1/100-s4".to_owned(),
    })
    .unwrap();
    let e = r.wait("calc-failed", |e| {
        matches!(e.kind, api::EventKind::CalcFailed { .. })
    });
    match e.kind {
        api::EventKind::CalcFailed { error } => assert_eq!(error.code, "APP_DATA_VERSION"),
        _ => unreachable!(),
    }
    let snap = r.app.snapshot();
    assert_eq!(snap.data.active.as_deref(), Some("poe1/100-s4"));
    assert_eq!(snap.data.pinned.as_deref(), Some("poe1/200-s4"));
    // Taking the offer calculates again.
    let v = version(
        &r.run(api::Command::UseBundle {
            id: "poe1/200-s4".to_owned(),
        })
        .unwrap(),
    );
    r.calculated(v);
    assert_eq!(r.app.snapshot().data.pinned, None);
    // Back to the old one; the choice is remembered.
    r.run(api::Command::UseBundle {
        id: "poe1/100-s4".to_owned(),
    })
    .unwrap();
    r.app.stop();
    let settings = std::fs::read_to_string(data.join("settings.json")).unwrap();
    assert!(
        settings.contains(r#""bundle-id": "poe1/100-s4""#),
        "{settings}"
    );

    let r = start(&data, None, "installed-2");
    assert_eq!(r.app.snapshot().data.active.as_deref(), Some("poe1/100-s4"));
    r.app.stop();

    // The remembered bundle breaks: the newest installed one is taken,
    // and the snapshot says so.
    std::fs::remove_file(old.join("data/stats.json")).unwrap();
    let r = start(&data, None, "installed-3");
    let snap = r.app.snapshot();
    assert!(snap.data.error.is_none());
    assert_eq!(snap.data.active.as_deref(), Some("poe1/200-s4"));
    assert_eq!(
        snap.data.notice.map(|n| n.code).as_deref(),
        Some("APP_BUNDLE_FALLBACK")
    );
    r.app.stop();

    // `LOOM_BUNDLE_DIR` wins and is not an installed one.
    let r = start(&data, Some(bundle_dir()), "installed-4");
    let snap = r.app.snapshot();
    assert!(snap.data.error.is_none());
    assert_eq!(snap.data.active, None);
    assert_eq!(snap.data.installed.len(), 2);
    r.app.stop();
}

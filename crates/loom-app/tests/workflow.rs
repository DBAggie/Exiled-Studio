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

use loom_app::{App, Config, HostCommand, Session, api};
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
    {
        let r = start(&data, Some(bundle_dir()), "first");
        let v = version(
            &r.run(api::Command::ImportText { text: synthetic() })
                .unwrap(),
        );
        r.calculated(v);
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

    // A standing grant applies at once; revoking it takes effect next.
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

//! The workflow through the real Tauri commands on Tauri's mock runtime
//! (Phase 4 plan, gate 1, decision 14 b), with the file dialogs replaced by
//! a fake picker; the command list against `ui/src/api/commands.json`; and
//! the capability file's permissions (finding 11c).

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests; a failure should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use loom_tauri::{FilePicker, Paths, Pick};
use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{Listener, Manager};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The app's own origin, as the webview loads it: a remote origin gets no
/// command (Tauri's ACL). WebView2 serves custom protocols over http.
#[cfg(windows)]
const APP_ORIGIN: &str = "http://tauri.localhost";
#[cfg(not(windows))]
const APP_ORIGIN: &str = "tauri://localhost";

struct FakePicker(PathBuf);

impl FilePicker for FakePicker {
    fn pick(&self, what: Pick, _name: &str) -> Option<PathBuf> {
        let file = match what {
            // The build to compare with is the file a test put there:
            // `compare.json`, else `compare.xml`; with neither, the user
            // cancelled.
            Pick::CompareBuild => {
                return ["compare.json", "compare.xml"]
                    .iter()
                    .map(|f| self.0.join(f))
                    .find(|p| p.exists());
            }
            Pick::OpenPob => "import.xml",
            Pick::OpenBuild | Pick::SaveBuild => "saved.json",
            Pick::ExportPob => "export.xml",
            Pick::Diagnostics => "diagnostics.zip",
            Pick::BundleFolder | Pick::BundlePack => return None,
        };
        Some(self.0.join(file))
    }
}

struct Shell {
    _app: tauri::App<MockRuntime>,
    webview: tauri::WebviewWindow<MockRuntime>,
    events: Receiver<Value>,
    dir: PathBuf,
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn shell(test: &str) -> Shell {
    let dir = std::env::temp_dir().join(format!("loom-tauri-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
        dir.join("import.xml"),
    )
    .unwrap();
    let data = dir.join("data");
    let picks = dir.clone();
    let mut app = loom_tauri::build(
        mock_builder(),
        move |_| Paths {
            data,
            bundle: Some(root().join("crates/loom-data/tests/fixtures/bundle-poe1")),
            keep_token: false,
            shipped: None,
            load_in_background: false,
        },
        move |_| Box::new(FakePicker(picks)),
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    // `setup` (which starts the application) runs on the first iteration;
    // one iteration of the mock runtime, not a loop.
    #[expect(
        deprecated,
        reason = "one iteration runs setup; never called in a loop"
    )]
    app.run_iteration(|_, _| {});
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    app.listen_any(loom_tauri::EVENT, move |e| {
        let _ = tx
            .lock()
            .unwrap()
            .send(serde_json::from_str(e.payload()).unwrap());
    });
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    Shell {
        _app: app,
        webview,
        events,
        dir,
    }
}

impl Shell {
    fn invoke(&self, cmd: &str, body: Value) -> Result<Value, Value> {
        get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: APP_ORIGIN.parse().unwrap(),
                body: InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<Value>().unwrap())
    }

    fn ok(&self, cmd: &str, body: Value) -> Value {
        self.invoke(cmd, body).unwrap()
    }

    fn execute(&self, command: Value) -> Value {
        self.ok("execute", json!({"command": command}))
    }

    /// Waits for `calc-finished` of the outcome's version.
    fn calculated(&self, outcome: &Value) -> String {
        let version = outcome["done"]["version"].as_u64().unwrap();
        loop {
            let e = self
                .events
                .recv_timeout(Duration::from_secs(30))
                .expect("no calc-finished event");
            if e["version"] == version && e["kind"].get("calc-finished").is_some() {
                break;
            }
        }
        let doc = self.ok("calc_document", json!({}));
        assert_eq!(doc[0], version);
        doc[1].as_str().unwrap().to_owned()
    }
}

#[test]
fn workflow_should_run_through_the_tauri_commands() {
    let s = shell("workflow");
    let snap = s.ok("snapshot", json!({}));
    assert!(snap["build"].is_null());
    assert!(snap["data"]["error"].is_null(), "{snap}");

    // Import a file the user picks.
    let imported = s.calculated(&s.ok("import_file", json!({})));
    let snap = s.ok("snapshot", json!({}));
    assert_eq!(snap["build"]["name"], "import");
    assert_eq!(snap["calc"]["state"], "done");

    // Inspect an item, preview and apply an edit, undo it.
    let item = s.ok("item", json!({"id": 1}));
    let life = item["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["text"] == "+15 to maximum Life")
        .unwrap()
        .clone();
    let accuracy = item["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["text"] == "+12 to Accuracy")
        .unwrap()
        .clone();
    let edit = json!({
        "item": 1,
        "ops": [{"replace": {"line": life["index"], "text": "+25 to maximum Life"}}, {"remove": {"line": accuracy["index"]}}],
        "expect": [{"line": life["index"], "text": life["text"]}, {"line": accuracy["index"], "text": accuracy["text"]}]
    });
    let preview = s.ok("preview", json!({"edit": edit}));
    assert!(!preview["fields"].as_array().unwrap().is_empty());
    let edited = s.calculated(&s.execute(json!({"edit-item": edit})));
    assert_ne!(edited, imported);
    let stale = s
        .invoke("execute", json!({"command": {"edit-item": edit}}))
        .unwrap_err();
    assert_eq!(stale["code"], "APP_STALE_EDIT");
    assert_eq!(s.calculated(&s.execute(json!("undo"))), imported);
    assert_eq!(s.calculated(&s.execute(json!("redo"))), edited);

    // Save, reopen, export.
    s.ok("save_as", json!({}));
    assert!(s.dir.join("saved.json").exists());
    assert_eq!(s.ok("recent", json!({})).as_array().unwrap().len(), 1);
    assert_eq!(s.calculated(&s.ok("open_file", json!({}))), edited);
    s.ok("export_xml", json!({}));
    let xml = std::fs::read_to_string(s.dir.join("export.xml")).unwrap();
    assert!(xml.contains("+25 to maximum Life"));
    let code = s.execute(json!("export-share-code"));
    assert!(code["text"]["text"].as_str().unwrap().len() > 20);

    // The MCP server starts and stops from the shell.
    let status = s.ok("mcp_start", json!({}));
    assert_eq!(status["running"], true);
    assert!(
        status["url"]
            .as_str()
            .unwrap()
            .starts_with("http://127.0.0.1:")
    );
    let status = s.ok(
        "set_permissions",
        json!({"permissions": {"requests": true, "standing-grant": false}}),
    );
    assert_eq!(status["permissions"]["requests"], true);
    // Rotate (ADR-017 Amendment 1): a new token on the same address; with
    // no protected store here, the token is per session.
    let before = s.ok("mcp_status", json!({}));
    assert_eq!(before["token-kept"], false);
    let rotated = s.ok("mcp_rotate", json!({}));
    assert_eq!(rotated["running"], true);
    assert_eq!(rotated["url"], before["url"]);
    assert_ne!(rotated["token"], before["token"]);
    let status = s.ok("mcp_stop", json!({}));
    assert_eq!(status["running"], false);

    // A picker that is cancelled does nothing.
    assert_eq!(s.ok("choose_bundle", json!({})), json!("nothing"));
}

#[test]
fn command_list_should_match_the_uis_and_every_command_should_be_registered() {
    let file = root().join("ui/src/api/commands.json");
    let names: Vec<String> = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(names, loom_tauri::COMMANDS.map(str::to_owned).to_vec());
    let s = shell("commands");
    // No command may reach the network from a test (the update check and
    // the data feed): strict offline refuses both before connecting.
    s.ok("set_update_mode", json!({"mode": "strict-offline"}));
    for name in &names {
        if name == "set_update_mode" {
            continue;
        }
        if let Err(e) = s.invoke(name, json!({})) {
            let text = e.to_string();
            assert!(!text.contains("not found"), "{name}: {text}");
        }
    }
    let unknown = s.invoke("no_such_command", json!({})).unwrap_err();
    assert!(unknown.to_string().contains("not found"), "{unknown}");
}

#[test]
fn capabilities_should_equal_the_committed_allow_list() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let allow: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("capability-allow-list.json")).unwrap(),
    )
    .unwrap();
    let mut seen = Vec::new();
    for entry in std::fs::read_dir(root.join("capabilities")).unwrap() {
        let cap: Value =
            serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap();
        let id = cap["identifier"].as_str().unwrap().to_owned();
        let mut granted: Vec<String> = cap["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                p.as_str()
                    .or_else(|| p["identifier"].as_str())
                    .unwrap()
                    .to_owned()
            })
            .collect();
        granted.sort();
        let mut allowed: Vec<String> = allow[&id]
            .as_array()
            .unwrap_or_else(|| panic!("capability {id} is not in the allow-list"))
            .iter()
            .map(|p| p.as_str().unwrap().to_owned())
            .collect();
        allowed.sort();
        assert_eq!(granted, allowed, "capability {id}");
        seen.push(id);
    }
    let listed = allow
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !k.starts_with('$'))
        .count();
    assert_eq!(seen.len(), listed, "every allow-listed capability exists");
}

#[test]
fn capabilities_should_grant_no_dialog_or_file_system_permission() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    let mut files = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        let cap: Value = serde_json::from_str(&text).unwrap();
        for p in cap["permissions"].as_array().unwrap() {
            let id = p.as_str().or_else(|| p["identifier"].as_str()).unwrap();
            assert!(
                !id.starts_with("dialog:")
                    && !id.starts_with("fs:")
                    && !id.starts_with("shell:")
                    && !id.starts_with("updater:"),
                "{id}"
            );
        }
        files += 1;
    }
    assert!(files >= 1);
}

#[test]
fn a_dropped_file_should_be_kept_on_the_rust_side_and_opened_by_kind() {
    let s = shell("drop");
    let handle = s.webview.app_handle().clone();
    let (tx, drops) = mpsc::channel();
    let tx = Mutex::new(tx);
    handle.listen_any(loom_tauri::DROP_EVENT, move |e| {
        let _ = tx
            .lock()
            .unwrap()
            .send(serde_json::from_str::<Value>(e.payload()).unwrap());
    });
    let next = || drops.recv_timeout(Duration::from_secs(10)).unwrap();

    // Nothing dropped: nothing to open.
    assert_eq!(s.ok("open_dropped", json!({})), json!("nothing"));

    // Another kind of file: the webview hears its kind, never its path.
    loom_tauri::dropped(&handle, &[s.dir.join("notes.txt")]);
    let notice = next();
    assert_eq!(
        notice,
        json!({"state": "dropped", "kind": "other", "name": "notes.txt"})
    );
    let err = s.invoke("open_dropped", json!({})).unwrap_err();
    assert_eq!(err["code"], "UI_DROP");

    // A PoB build: imported.
    loom_tauri::dropped(&handle, &[s.dir.join("import.xml")]);
    assert_eq!(next()["kind"], "pob");
    s.calculated(&s.ok("open_dropped", json!({})));
    let snap = s.ok("snapshot", json!({}));
    assert_eq!(snap["build"]["name"], "import");
    // Taken once.
    assert_eq!(s.ok("open_dropped", json!({})), json!("nothing"));
}

#[test]
fn the_workspace_and_the_comparison_queries_should_run_through_the_commands() {
    let s = shell("compare");
    // No file picked: the user cancelled.
    assert_eq!(s.ok("compare_file", json!({})), Value::Null);
    let err = s.invoke("stage_view", json!({"spec": 1})).unwrap_err();
    assert_eq!(err["code"], "APP_NO_BUILD");

    let imported = s.ok("import_file", json!({}));
    let document: Value = serde_json::from_str(&s.calculated(&imported)).unwrap();
    let life = |fields: &Value| {
        fields
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["field"] == "life")
            .unwrap()["value"]
            .clone()
    };
    let own = life(&document["result"]["fields"]);
    assert!(own.is_number(), "{own}");

    // The build's workspace document: one undoable change, in the view.
    let text = r#"{"dashboards":[{"name":"Overview"}]}"#;
    let set = s.execute(json!({"set-workspace": {"document": text}}));
    let version = set["done"]["version"].as_u64().unwrap();
    let snap = s.ok("snapshot", json!({}));
    assert_eq!(snap["build"]["workspace"], json!(text));
    assert_eq!(snap["history"]["undo"]["kind"], "workspace");
    assert_eq!(
        s.execute(json!({"set-workspace": {"document": text}})),
        json!("nothing")
    );
    let big = "a".repeat(262_145);
    let err = s
        .invoke(
            "execute",
            json!({"command": {"set-workspace": {"document": big}}}),
        )
        .unwrap_err();
    assert_eq!(err["code"], "APP_WORKSPACE");

    // Another build file, a PoB build then a build document, calculated
    // without opening it: the open build stays as it is.
    std::fs::copy(s.dir.join("import.xml"), s.dir.join("compare.xml")).unwrap();
    let pob = s.ok("compare_file", json!({}));
    assert_eq!(pob["name"], "compare");
    assert_eq!(life(&pob["fields"]), own);
    s.ok("save_as", json!({}));
    std::fs::copy(s.dir.join("saved.json"), s.dir.join("compare.json")).unwrap();
    let saved = s.ok("compare_file", json!({}));
    assert_eq!(saved["name"], "compare");
    assert_eq!(saved["fields"], pob["fields"]);
    std::fs::write(s.dir.join("compare.json"), "{}").unwrap();
    let err = s.invoke("compare_file", json!({})).unwrap_err();
    assert!(err["code"].is_string(), "{err}");
    let snap = s.ok("snapshot", json!({}));
    assert_eq!(snap["version"], version);
    assert_eq!(snap["build"]["name"], "saved");

    // A stage's tree and equipment, read without choosing it.
    let stage = s.ok("stage_view", json!({"spec": 1}));
    assert_eq!(stage["spec"], 1);
    assert_eq!(stage["title"], "Tree 1");
    assert_eq!(stage["nodes"], json!([]));
    assert_eq!(stage["slots"], json!([{"slot": "Ring 1", "item": 1}]));
    let err = s.invoke("stage_view", json!({"spec": 9})).unwrap_err();
    assert_eq!(err["code"], "APP_LOADOUT");
    assert_eq!(s.ok("snapshot", json!({}))["version"], version);
}

#[test]
fn drop_kind_should_follow_the_extension() {
    use loom_tauri::{DropKind, drop_kind};
    assert_eq!(drop_kind(Path::new("a/Build.XML")), DropKind::Pob);
    assert_eq!(drop_kind(Path::new("b.json")), DropKind::Build);
    assert_eq!(drop_kind(Path::new("c.txt")), DropKind::Other);
    assert_eq!(drop_kind(Path::new("noext")), DropKind::Other);
}

#[test]
fn ui_prefs_should_round_trip_and_refuse_what_is_not_json() {
    let s = shell("prefs");
    assert_eq!(s.ok("ui_prefs", json!({})), Value::Null);
    let text = r#"{"dashboards":[{"id":"a","name":"Overview","components":[]}]}"#;
    s.ok("set_ui_prefs", json!({"text": text}));
    assert_eq!(s.ok("ui_prefs", json!({})), json!(text));
    let e = s
        .invoke("set_ui_prefs", json!({"text": "{not json"}))
        .unwrap_err();
    assert_eq!(e["code"], "UI_PREFS");
    let big = format!(r#"{{"x":"{}"}}"#, "a".repeat(loom_tauri::MAX_UI_PREFS));
    let e = s.invoke("set_ui_prefs", json!({"text": big})).unwrap_err();
    assert_eq!(e["code"], "UI_PREFS");
    // The saved document is unchanged by the refusals.
    assert_eq!(s.ok("ui_prefs", json!({})), json!(text));
}

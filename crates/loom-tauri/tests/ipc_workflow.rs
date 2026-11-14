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
use tauri::Listener;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;

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
            Pick::OpenPob => "import.xml",
            Pick::OpenBuild | Pick::SaveBuild => "saved.json",
            Pick::ExportPob => "export.xml",
            Pick::BundleFolder => return None,
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
    for name in &names {
        if let Err(e) = s.invoke(name, json!({})) {
            let text = e.to_string();
            assert!(!text.contains("not found"), "{name}: {text}");
        }
    }
    let unknown = s.invoke("no_such_command", json!({})).unwrap_err();
    assert!(unknown.to_string().contains("not found"), "{unknown}");
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
                !id.starts_with("dialog:") && !id.starts_with("fs:") && !id.starts_with("shell:"),
                "{id}"
            );
        }
        files += 1;
    }
    assert!(files >= 1);
}

//! Logs, crash records, and the diagnostic package through the running
//! application (Phase 5, P-f; ADR-014): a command's log line and its
//! calculation's share the command's correlation ID; an import is logged by
//! size and SHA-256, never its text; the exported zip holds exactly the
//! previewed files. One test in its own binary: it installs the process's
//! global subscriber.

#![expect(clippy::panic, reason = "tests; a failure should panic")]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use loom_app::logs::{Logs, Redactor};
use loom_app::{App, Config, HostCommand, api, crash, diagnostics};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn logs_should_correlate_redact_and_package_what_the_preview_shows() {
    let data = std::env::temp_dir().join(format!("loom-app-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(&data).unwrap();
    // The data folder stands in for the home folder, so every path the
    // logs hold is redacted.
    let logs = Logs::open(&data, Redactor::new(Some(&data), None)).unwrap();
    tracing::subscriber::set_global_default(logs.subscriber()).unwrap();

    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let sink: loom_app::Sink = Arc::new(move |e| {
        let _ = tx.lock().unwrap().send(e);
    });
    let mut config = Config::new(
        data.clone(),
        Some(root().join("crates/loom-data/tests/fixtures/bundle-poe1")),
    );
    config.instance = "logs".to_owned();
    let app = App::start(config, sink).unwrap();
    let text = std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap();
    let outcome = app
        .execute(HostCommand::Api(api::Command::ImportText {
            text: text.clone(),
        }))
        .unwrap();
    let api::Outcome::Done { version } = outcome else {
        panic!("{outcome:?}");
    };
    loop {
        let e = events.recv_timeout(Duration::from_secs(30)).unwrap();
        if e.version == version && matches!(e.kind, api::EventKind::CalcFinished { .. }) {
            break;
        }
    }
    // A second command, logged with its own correlation.
    let _ = app.execute(HostCommand::Api(api::Command::Undo));

    let log = std::fs::read_to_string(data.join("logs").join("loom.log")).unwrap();
    let lines: Vec<serde_json::Value> = log
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let import = lines
        .iter()
        .find(|l| l["command"] == "import-text")
        .unwrap_or_else(|| panic!("{log}"));
    let correlation = &import["span"]["correlation"];
    assert!(correlation.is_u64(), "{import}");
    assert_eq!(import["outcome"], "ok");
    assert_eq!(import["size"], text.len());
    assert_eq!(import["sha256"], loom_app::logs::digest(text.as_bytes()).1);
    let calc = lines
        .iter()
        .find(|l| l["target"] == "loom::calc" && &l["span"]["correlation"] == correlation)
        .unwrap_or_else(|| panic!("no calculation with the import's correlation: {log}"));
    assert_eq!(calc["span"]["version"], version);
    assert!(lines.iter().any(|l| l["message"] == "start"));
    // No build text: not the build's name, an item's name, or its XML.
    for secret in ["Loom Test Band", "<PathOfBuilding", "maximum Life"] {
        assert!(!log.contains(secret), "{secret} in the log");
    }

    // A crash record, listed and in the package.
    let record = crash::record(&Redactor::default(), 0, "main", "a test panic", None);
    crash::write(&data, &record).unwrap();
    assert_eq!(app.crashes().len(), 1);

    let preview = app.diagnostics();
    let names: Vec<&str> = preview.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names[0], "about.json");
    assert!(names.contains(&"logs/loom.log"), "{names:?}");
    assert!(names.contains(&format!("crashes/{}.json", record.id).as_str()));
    assert!(
        !preview
            .iter()
            .any(|f| f.text.contains(&data.display().to_string())),
        "no file names the data folder"
    );
    // More events after the preview do not reach the package.
    let _ = app.execute(HostCommand::Api(api::Command::Undo));
    // A failed write, then two saves: each writes the previewed files.
    assert!(
        app.execute(HostCommand::ExportDiagnostics(
            data.join("missing").join("x.zip")
        ))
        .is_err()
    );
    for name in ["out.zip", "again.zip"] {
        let zip = data.join(name);
        app.execute(HostCommand::ExportDiagnostics(zip.clone()))
            .unwrap();
        let written = diagnostics::unzip(&std::fs::read(&zip).unwrap()).unwrap();
        assert_eq!(written.len(), preview.len());
        for (w, p) in written.iter().zip(&preview) {
            assert_eq!(w.name, p.name);
            assert_eq!(w.text, p.text);
            assert_eq!(w.text.len() as u64, p.bytes);
        }
    }

    app.dismiss_crash(&record.id).unwrap();
    assert!(app.crashes().is_empty());
    assert!(app.dismiss_crash(&record.id).is_err());
    app.stop();
    let _ = std::fs::remove_dir_all(&data);
}

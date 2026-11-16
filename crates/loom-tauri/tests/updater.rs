//! The update check against a loopback fake feed (Phase 5, P3;
//! `docs/phase-5/updates.md`): an offer for a greater version, none for an
//! equal or lower one (no downgrade), and the package's signature checked
//! with the update key on download. `tests/fixtures/updater/` holds a test
//! key made by `tauri signer generate` (public: it signs nothing real) and a
//! payload signed with it by `tauri signer sign`.
#![expect(
    clippy::unwrap_used,
    reason = "test: fixtures from disk, a loopback server thread, and panics as failures"
)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::json;
use tauri::Url;
use tauri::test::{mock_builder, mock_context, noop_assets};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/updater")
            .join(name),
    )
    .unwrap()
}

/// A loopback HTTP server: `/latest.json` answers the current feed,
/// `/payload.bin` the signed payload, `/tampered.bin` it changed by a byte.
struct Feed {
    base: String,
    feed: Arc<Mutex<String>>,
}

impl Feed {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let feed = Arc::new(Mutex::new(String::new()));
        let served = Arc::clone(&feed);
        let payload = fixture("payload.bin");
        let mut tampered = payload.clone();
        tampered[0] ^= 1;
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut line = String::new();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                // The rest of the request head.
                let mut header = String::new();
                while reader.read_line(&mut header).is_ok_and(|n| n > 2) {
                    header.clear();
                }
                let path = line.split_whitespace().nth(1).unwrap_or("/");
                let (status, body): (&str, Vec<u8>) = match path {
                    "/latest.json" => ("200 OK", served.lock().unwrap().clone().into_bytes()),
                    "/payload.bin" => ("200 OK", payload.clone()),
                    "/tampered.bin" => ("200 OK", tampered.clone()),
                    _ => ("404 Not Found", Vec::new()),
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        Self { base, feed }
    }

    /// Serves a feed for `version` whose package is `file`.
    fn serve(&self, version: &str, file: &str) {
        let signature = String::from_utf8(fixture("payload.bin.sig")).unwrap();
        let target = tauri_plugin_updater::target().unwrap();
        let entry =
            json!({ "signature": signature.trim(), "url": format!("{}/{file}", self.base) });
        *self.feed.lock().unwrap() = json!({
            "version": version,
            "notes": "Test notes",
            "pub_date": "2026-11-15T12:00:00Z",
            "platforms": { target: entry }
        })
        .to_string();
    }

    fn endpoint(&self) -> Vec<Url> {
        vec![Url::parse(&format!("{}/latest.json", self.base)).unwrap()]
    }
}

/// A mock application (version 0.1.0) with the updater plugin.
fn app() -> tauri::App<tauri::test::MockRuntime> {
    let mut context = mock_context(noop_assets());
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".to_owned(), json!({ "pubkey": "" }));
    mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap()
}

fn pubkey() -> String {
    String::from_utf8(fixture("test.key.pub")).unwrap()
}

#[test]
fn a_greater_version_is_offered_and_its_signed_package_verifies() {
    let feed = Feed::start();
    feed.serve("0.2.0", "payload.bin");
    let app = app();
    let update = tauri::async_runtime::block_on(loom_tauri::updates::check(
        app.handle(),
        &pubkey(),
        Some(feed.endpoint()),
    ))
    .unwrap()
    .expect("an offer");
    let offer = loom_tauri::updates::offer(&update);
    assert_eq!(offer.version, "0.2.0");
    assert_eq!(offer.notes.as_deref(), Some("Test notes"));
    assert_eq!(offer.date.as_deref(), Some("2026-11-15T12:00:00Z"));
    assert!(offer.url.ends_with("/releases/tag/v0.2.0"));
    let bytes =
        tauri::async_runtime::block_on(update.download(|_, _| {}, || {})).expect("verified");
    assert_eq!(bytes, fixture("payload.bin"));
}

#[test]
fn a_changed_package_is_refused() {
    let feed = Feed::start();
    feed.serve("0.2.0", "tampered.bin");
    let app = app();
    let update = tauri::async_runtime::block_on(loom_tauri::updates::check(
        app.handle(),
        &pubkey(),
        Some(feed.endpoint()),
    ))
    .unwrap()
    .expect("an offer");
    let result = tauri::async_runtime::block_on(update.download(|_, _| {}, || {}));
    assert!(
        result.is_err(),
        "a package whose signature fails is refused"
    );
}

#[test]
fn an_equal_or_lower_version_is_no_update() {
    let feed = Feed::start();
    let app = app();
    for version in ["0.1.0", "0.0.9", "0.1.0-rc.1"] {
        feed.serve(version, "payload.bin");
        let update = tauri::async_runtime::block_on(loom_tauri::updates::check(
            app.handle(),
            &pubkey(),
            Some(feed.endpoint()),
        ))
        .unwrap();
        assert!(update.is_none(), "{version} must not be offered to 0.1.0");
    }
}

#[test]
fn an_unreachable_feed_is_an_error_not_an_offer() {
    let app = app();
    // A port nothing listens on.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/latest.json", listener.local_addr().unwrap());
    drop(listener);
    let result = tauri::async_runtime::block_on(loom_tauri::updates::check(
        app.handle(),
        &pubkey(),
        Some(vec![Url::parse(&url).unwrap()]),
    ));
    assert!(result.is_err());
}

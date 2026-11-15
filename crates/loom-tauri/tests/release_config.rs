//! The packaging configuration (Phase 5, P1; `docs/phase-5/packaging.md`):
//! one version for the whole application, and the bundler's targets.
#![expect(
    clippy::unwrap_used,
    reason = "test helpers read committed files; a failure is the test's"
)]

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(root().join(path)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The version policy: the workspace version is the application's; the UI
/// shows `ui/package.json`'s, which must equal it, and the Tauri config
/// names none (so the bundler takes the crate's).
#[test]
fn the_application_should_have_one_version() {
    let ui = json("ui/package.json");
    assert_eq!(ui["version"], env!("CARGO_PKG_VERSION"), "ui/package.json");
    let conf = json("crates/loom-tauri/tauri.conf.json");
    assert!(
        conf.get("version").is_none(),
        "tauri.conf.json must not name a version"
    );
}

/// The installers and packages the plan names (P-a), the license page with
/// GGG's notice, and WebView2: the bootstrapper by default, the offline
/// installer in the NSIS setup.
#[test]
fn the_bundler_should_make_the_planned_artifacts() {
    let conf = json("crates/loom-tauri/tauri.conf.json");
    let bundle = &conf["bundle"];
    assert_eq!(bundle["active"], true);
    assert_eq!(
        bundle["targets"],
        serde_json::json!(["nsis", "msi", "appimage", "deb"])
    );
    assert_eq!(bundle["windows"]["nsis"]["installMode"], "currentUser");
    let license = std::fs::read_to_string(
        root()
            .join("crates/loom-tauri")
            .join(bundle["licenseFile"].as_str().unwrap()),
    )
    .unwrap();
    assert!(license.contains(
        "This product isn't affiliated with or endorsed by Grinding Gear Games in any way."
    ));
    let nsis = json("crates/loom-tauri/tauri.nsis.conf.json");
    assert_eq!(
        nsis["bundle"]["windows"]["webviewInstallMode"]["type"],
        "offlineInstaller"
    );
}

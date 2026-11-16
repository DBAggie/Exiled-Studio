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

/// Merges `patch` into `base` as Tauri merges a `--config` file (JSON merge
/// patch: objects merge, anything else replaces).
fn merged(mut base: serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
    if let (Some(b), Some(p)) = (base.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            let next = merged(b.get(k).cloned().unwrap_or_default(), v);
            b.insert(k.clone(), next);
        }
        base
    } else {
        patch.clone()
    }
}

/// The experimental channel (`docs/release/channels.md`): another
/// application beside the stable one (its own name, binary, identifier, so
/// its own install folder, MSI upgrade code, and data folder), the same
/// window but for its title, and updates from the project site.
#[test]
fn the_experimental_application_should_stand_beside_the_stable_one() {
    let stable = json("crates/loom-tauri/tauri.conf.json");
    let experimental = merged(
        stable.clone(),
        &json("crates/loom-tauri/tauri.experimental.conf.json"),
    );
    for key in ["productName", "mainBinaryName", "identifier"] {
        assert_ne!(stable[key], experimental[key], "{key}");
        assert!(
            experimental[key]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("experimental"),
            "{key}"
        );
    }
    let mut window = stable["app"]["windows"][0].clone();
    window["title"] = experimental["productName"].clone();
    assert_eq!(experimental["app"]["windows"], serde_json::json!([window]));
    assert_eq!(
        stable["plugins"]["updater"]["endpoints"],
        serde_json::json!([
            "https://github.com/DBAggie/Exiled-Studio/releases/latest/download/latest.json"
        ])
    );
    assert_eq!(
        experimental["plugins"]["updater"]["endpoints"],
        serde_json::json!([format!(
            "{}/experimental/latest.json",
            loom_tauri::release::SITE
        )])
    );
    // Everything else, the update key and the install mode included, is
    // the stable application's.
    assert_eq!(
        experimental["plugins"]["updater"]["windows"],
        stable["plugins"]["updater"]["windows"]
    );
    assert_eq!(experimental["bundle"], stable["bundle"]);
}

/// The data feed a build checks is its channel's: the stable one on the
/// latest release, the experimental one on the project site.
#[test]
fn each_channel_should_check_its_own_data_feed() {
    use loom_tauri::release::{Channel, SITE};
    let (feed, signature) = Channel::Stable.data_feed();
    assert!(feed.starts_with("https://github.com/DBAggie/Exiled-Studio/releases/latest/download/"));
    assert_eq!(signature, format!("{feed}.sig"));
    let (feed, signature) = Channel::Experimental.data_feed();
    assert_eq!(feed, format!("{SITE}/experimental/data-feed.json"));
    assert_eq!(signature, format!("{feed}.sig"));
    assert_ne!(
        Channel::Stable.token_service(),
        Channel::Experimental.token_service()
    );
    assert_eq!(Channel::CURRENT, Channel::Stable, "tests build stable");
}

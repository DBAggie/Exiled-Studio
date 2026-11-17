//! `loom-bundles` on its own: an installed folder's status, the scan, the
//! full check, and the folder source. `loom-app`'s install tests cover the
//! same code through its re-exports, with packs and installs.

#![expect(
    clippy::unwrap_used,
    reason = "a test over folders it writes; a failure should panic"
)]

use std::path::{Path, PathBuf};

use loom_bundles::{
    ACCEPTED_FILE, AcceptedRecord, BUNDLES_DIR, DirSource, Quarantine, Status, VerifyError,
    bundle_id, scan, sha256_hex, status_of, trust_list, verify_full,
};
use loom_data::bundle::{BundleSource, MANIFEST};
use loom_data::signature::{KeyPair, SIGNATURE_FILE, TrustList};
use loom_domain::{DataVersion, Game};

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tempdir(name: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!("loom-bundles-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

fn trust() -> TrustList {
    trust_list(true).unwrap()
}

const DATA: &[u8] = b"{\"rows\":[]}";

/// A bundle folder with one data file, its manifest, and (when `signed`)
/// a signature by the public test key, which `trust()` lists.
fn bundle(dir: &Path, steam: Option<u64>, schema: u32, signed: bool) -> Vec<u8> {
    std::fs::create_dir_all(dir.join("poe1")).unwrap();
    std::fs::write(dir.join("poe1/table.json"), DATA).unwrap();
    let manifest = serde_json::json!({
        "format": 2,
        "data_schema": schema,
        "game": "poe1",
        "source": {
            "steam_build": steam,
            "index_sha256": "",
            "extractor_commit": "",
            "decoder": "",
            "dat_schema": ""
        },
        "files": [{"path": "poe1/table.json", "sha256": sha256_hex(DATA), "size": DATA.len()}],
        "signatures": []
    })
    .to_string()
    .into_bytes();
    std::fs::write(dir.join(MANIFEST), &manifest).unwrap();
    if signed {
        let sig = KeyPair::test_key().sign(&manifest);
        std::fs::write(dir.join(SIGNATURE_FILE), serde_json::to_vec(&sig).unwrap()).unwrap();
    }
    manifest
}

fn accept(dir: &Path, manifest_sha256: String) {
    let record = AcceptedRecord { manifest_sha256 };
    std::fs::write(
        dir.join(ACCEPTED_FILE),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
}

#[test]
fn a_test_signed_folder_should_be_trusted_with_its_id_and_sizes() {
    let tmp = tempdir("trusted");
    let manifest = bundle(&tmp.0, Some(26_530_219), 4, true);
    let found = status_of(&tmp.0, &trust()).unwrap();
    assert!(matches!(found.status, Status::Trusted { .. }));
    assert!(found.status.loads());
    assert_eq!(found.id, "poe1/26530219-s4");
    assert_eq!(found.manifest_sha256, sha256_hex(&manifest));
    assert_eq!((found.files, found.bytes), (1, DATA.len() as u64));
    assert!(found.key_id.is_some());
}

#[test]
fn a_folder_without_a_readable_manifest_should_not_be_a_bundle() {
    let tmp = tempdir("none");
    assert!(status_of(&tmp.0, &trust()).is_none());
    std::fs::write(tmp.0.join(MANIFEST), "{").unwrap();
    assert!(status_of(&tmp.0, &trust()).is_none());
}

#[test]
fn an_unsigned_folder_should_load_only_once_its_manifest_is_accepted() {
    let tmp = tempdir("unsigned");
    let manifest = bundle(&tmp.0, Some(1), 4, false);
    let status = || status_of(&tmp.0, &trust()).unwrap().status;
    assert_eq!(status(), Status::Quarantined(Quarantine::Unsigned));
    assert!(!status().loads());
    // A record of another manifest does not count.
    accept(&tmp.0, sha256_hex(b"other"));
    assert_eq!(status(), Status::Quarantined(Quarantine::Unsigned));
    accept(&tmp.0, sha256_hex(&manifest));
    assert_eq!(status(), Status::Accepted(Quarantine::Unsigned));
    assert!(status().loads());
}

#[test]
fn a_signed_folder_should_be_unknown_key_without_the_test_keys() {
    let tmp = tempdir("unknown-key");
    bundle(&tmp.0, Some(1), 4, true);
    let release = trust_list(false).unwrap();
    let found = status_of(&tmp.0, &release).unwrap();
    assert_eq!(found.status, Status::Quarantined(Quarantine::UnknownKey));
    assert!(found.key_id.is_some());
}

#[test]
fn a_manifest_changed_after_signing_should_be_quarantined_for_good() {
    let tmp = tempdir("bad-signature");
    let manifest = bundle(&tmp.0, Some(1), 4, true);
    let changed = String::from_utf8(manifest)
        .unwrap()
        .replace("\"data_schema\":4", "\"data_schema\":5");
    std::fs::write(tmp.0.join(MANIFEST), &changed).unwrap();
    accept(&tmp.0, sha256_hex(changed.as_bytes()));
    let found = status_of(&tmp.0, &trust()).unwrap();
    assert_eq!(found.status, Status::Quarantined(Quarantine::BadSignature));
    assert!(!Quarantine::BadSignature.acceptable());
}

#[test]
fn a_file_of_another_size_should_quarantine_even_an_accepted_folder() {
    let tmp = tempdir("damaged");
    let manifest = bundle(&tmp.0, Some(1), 4, false);
    accept(&tmp.0, sha256_hex(&manifest));
    std::fs::write(tmp.0.join("poe1/table.json"), b"{}").unwrap();
    let found = status_of(&tmp.0, &trust()).unwrap();
    assert_eq!(found.status, Status::Quarantined(Quarantine::Damaged));
    std::fs::remove_file(tmp.0.join("poe1/table.json")).unwrap();
    let found = status_of(&tmp.0, &trust()).unwrap();
    assert_eq!(found.status, Status::Quarantined(Quarantine::Damaged));
}

#[test]
fn scan_should_list_folders_newest_first_skipping_hidden_and_duplicates() {
    let tmp = tempdir("scan");
    let root = tmp.0.join(BUNDLES_DIR);
    bundle(&root.join("poe1/100-s4"), Some(100), 4, true);
    bundle(&root.join("poe1/200-s3"), Some(200), 3, true);
    bundle(&root.join("poe1/200-s4"), Some(200), 4, false);
    // The same ID as 100-s4 under a later name: the first by path stays.
    bundle(&root.join("poe1/zz-copy"), Some(100), 4, true);
    // A temporary folder, and a hidden game folder: skipped.
    bundle(&root.join("poe1/.tmp-install"), Some(900), 4, true);
    bundle(&root.join(".hidden/x"), Some(901), 4, true);
    // Not a bundle: skipped.
    std::fs::create_dir_all(root.join("poe1/empty")).unwrap();
    let found = scan(&tmp.0, &trust());
    let ids: Vec<&str> = found.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["poe1/200-s4", "poe1/200-s3", "poe1/100-s4"]);
    assert_eq!(found[2].dir, root.join("poe1/100-s4"));
    // Quarantined folders are listed too, with their status.
    assert_eq!(found[0].status, Status::Quarantined(Quarantine::Unsigned));
}

#[test]
fn scan_should_be_empty_without_a_bundles_folder() {
    let tmp = tempdir("scan-empty");
    assert!(scan(&tmp.0, &trust()).is_empty());
}

#[test]
fn bundle_id_should_name_game_build_and_schema() {
    let v = DataVersion {
        game: Game::Poe1,
        schema_version: 4,
        steam_build: Some(26_530_219),
    };
    assert_eq!(bundle_id(&v), "poe1/26530219-s4");
    let none = DataVersion {
        steam_build: None,
        ..v
    };
    assert_eq!(bundle_id(&none), "poe1/unknown-s4");
}

#[test]
fn verify_full_should_find_a_file_of_the_right_size_and_other_bytes() {
    let tmp = tempdir("verify");
    bundle(&tmp.0, Some(1), 4, true);
    assert!(verify_full(&tmp.0).is_ok());
    let other: Vec<u8> = DATA.iter().map(|b| b ^ 1).collect();
    std::fs::write(tmp.0.join("poe1/table.json"), other).unwrap();
    // The sizes still match, so the quick status does not see it.
    assert!(status_of(&tmp.0, &trust()).unwrap().status.loads());
    assert!(matches!(
        verify_full(&tmp.0),
        Err(VerifyError::Damaged(p)) if p == "poe1/table.json"
    ));
    std::fs::remove_file(tmp.0.join(MANIFEST)).unwrap();
    assert!(matches!(verify_full(&tmp.0), Err(VerifyError::Read { .. })));
}

#[test]
fn dir_source_should_list_read_and_size_the_folders_files() {
    let tmp = tempdir("source");
    bundle(&tmp.0, Some(1), 4, true);
    let source = DirSource(&tmp.0);
    assert_eq!(
        source.list().unwrap(),
        [MANIFEST, "poe1/table.json", SIGNATURE_FILE]
    );
    assert_eq!(source.read("poe1/table.json").unwrap(), DATA);
    assert_eq!(source.size("poe1/table.json").unwrap(), DATA.len() as u64);
    assert!(source.size("poe1").is_err());
    assert!(source.read("missing.json").is_err());
}

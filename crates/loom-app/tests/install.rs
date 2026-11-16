//! Installing data bundle packs (Phase 5, P4; `docs/phase-5/data-bundles.md`):
//! signature, quarantine, acceptance, revocation, and the hostile-input
//! checks (zip slip, links, a zip bomb's sizes, changed content), each
//! leaving nothing installed.
#![expect(
    clippy::unwrap_used,
    reason = "test: temporary folders and fixtures on disk"
)]

use std::io::Write;
use std::path::{Path, PathBuf};

use loom_app::install::{self, Quarantine, Status};
use loom_data::signature::{KeyPair, Revocation, TRUST_FORMAT, TrustList};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../loom-data/tests/fixtures/bundle-poe1")
}

struct Temp(PathBuf);

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp(name: &str) -> Temp {
    let dir = std::env::temp_dir().join(format!("loom-install-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Temp(dir)
}

fn key() -> KeyPair {
    KeyPair::from_seed(&[7; 32])
}

fn trust() -> TrustList {
    TrustList {
        format: TRUST_FORMAT,
        keys: vec![key().trusted("test")],
        revoked: Vec::new(),
    }
}

fn manifest_bytes(dir: &Path) -> Vec<u8> {
    std::fs::read(dir.join("bundle.json")).unwrap()
}

fn signature(pair: &KeyPair, manifest: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&pair.sign(manifest)).unwrap()
}

/// A pack of the fixture signed by `pair` (none: unsigned).
fn pack(tmp: &Path, pair: Option<&KeyPair>) -> PathBuf {
    let out = tmp.join("bundle.loombundle");
    let sig = pair.map(|p| signature(p, &manifest_bytes(&fixture())));
    install::write_pack(&fixture(), sig.as_deref(), &out).unwrap();
    out
}

fn bundles(data: &Path) -> Vec<String> {
    install::scan(data, &trust())
        .into_iter()
        .map(|f| f.id)
        .collect()
}

/// Nothing but the bundles folder (no temporary folder left behind).
fn nothing_installed(data: &Path) {
    let root = data.join("bundles");
    let left: Vec<_> = walk(&root);
    assert!(left.is_empty(), "left behind: {left:?}");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .flat_map(|e| {
            let p = e.path();
            if p.is_dir() {
                let inner = walk(&p);
                if inner.is_empty() { Vec::new() } else { inner }
            } else {
                vec![p]
            }
        })
        .collect()
}

#[test]
fn a_signed_pack_installs_and_loads() {
    let tmp = temp("signed");
    let data = tmp.0.join("data");
    let installed = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust()).unwrap();
    assert!(!installed.already);
    assert!(matches!(installed.found.status, Status::Trusted { .. }));
    assert_eq!(bundles(&data), std::slice::from_ref(&installed.found.id));
    install::verify_full(&installed.found.dir).unwrap();
    // The same pack again: nothing written.
    let again = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust()).unwrap();
    assert!(again.already);
}

#[test]
fn an_unsigned_or_unknown_key_pack_is_quarantined_until_accepted() {
    for (name, pair, reason) in [
        ("unsigned", None, Quarantine::Unsigned),
        (
            "unknown",
            Some(KeyPair::from_seed(&[9; 32])),
            Quarantine::UnknownKey,
        ),
    ] {
        let tmp = temp(name);
        let data = tmp.0.join("data");
        let found = install::install_pack(&data, &pack(&tmp.0, pair.as_ref()), &trust())
            .unwrap()
            .found;
        assert_eq!(found.status, Status::Quarantined(reason), "{name}");
        // A hash other than the one shown: refused.
        let e = install::accept(&found, &"0".repeat(64)).unwrap_err();
        assert_eq!(e.code, "APP_QUARANTINED");
        install::accept(&found, &found.manifest_sha256).unwrap();
        let after = install::status_of(&found.dir, &trust()).unwrap();
        assert_eq!(after.status, Status::Accepted(reason), "{name}");
    }
}

#[test]
fn a_changed_manifest_or_a_revoked_key_is_refused() {
    let tmp = temp("refused");
    let data = tmp.0.join("data");
    // Signed over other bytes: the manifest changed after signing.
    let out = tmp.0.join("changed.loombundle");
    let sig = signature(&key(), b"{\"another\":\"manifest\"}");
    install::write_pack(&fixture(), Some(&sig), &out).unwrap();
    let e = install::install_pack(&data, &out, &trust()).unwrap_err();
    assert_eq!(e.code, "PACK_SIGNATURE");
    nothing_installed(&data);
    // Revoked.
    let mut revoked = trust();
    revoked.revoked.push(Revocation {
        key_id: key().trusted("").id,
        reason: "leaked".to_owned(),
    });
    let e = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &revoked).unwrap_err();
    assert_eq!(e.code, "PACK_SIGNATURE");
    nothing_installed(&data);
}

#[test]
fn revoking_an_installed_bundles_key_quarantines_it_for_good() {
    let tmp = temp("revoke-installed");
    let data = tmp.0.join("data");
    let found = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust())
        .unwrap()
        .found;
    let mut revoked = trust();
    revoked.revoked.push(Revocation {
        key_id: key().trusted("").id,
        reason: "leaked".to_owned(),
    });
    let after = install::status_of(&found.dir, &revoked).unwrap();
    assert_eq!(after.status, Status::Quarantined(Quarantine::Revoked));
    assert_eq!(
        install::accept(&after, &after.manifest_sha256)
            .unwrap_err()
            .code,
        "APP_QUARANTINED"
    );
}

/// A zip of the fixture's files plus `extra` entries (name, bytes),
/// signed.
fn crafted(
    tmp: &Path,
    name: &str,
    replace: Option<(&str, &[u8])>,
    extra: &[(&str, &[u8])],
) -> PathBuf {
    let dir = fixture();
    let manifest = manifest_bytes(&dir);
    let parsed: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    let out = tmp.join(name);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&out).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("bundle.json", options).unwrap();
    zip.write_all(&manifest).unwrap();
    zip.start_file("signature.json", options).unwrap();
    zip.write_all(&signature(&key(), &manifest)).unwrap();
    for f in parsed["files"].as_array().unwrap() {
        let path = f["path"].as_str().unwrap();
        zip.start_file(path, options).unwrap();
        match replace {
            Some((p, bytes)) if p == path => zip.write_all(bytes).unwrap(),
            _ => zip
                .write_all(&std::fs::read(dir.join(path)).unwrap())
                .unwrap(),
        }
    }
    for (n, bytes) in extra {
        zip.start_file(*n, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    out
}

fn first_file() -> (String, u64) {
    let parsed: serde_json::Value = serde_json::from_slice(&manifest_bytes(&fixture())).unwrap();
    let f = &parsed["files"][0];
    (
        f["path"].as_str().unwrap().to_owned(),
        f["size"].as_u64().unwrap(),
    )
}

#[test]
fn zip_slip_entries_are_refused_before_anything_is_written() {
    let tmp = temp("slip");
    let data = tmp.0.join("data");
    for (i, name) in [
        "../../evil.json",
        "/abs.json",
        "data\\..\\x.json",
        "C:/x.json",
    ]
    .iter()
    .enumerate()
    {
        let out = crafted(
            &tmp.0,
            &format!("slip{i}.loombundle"),
            None,
            &[(name, b"x")],
        );
        let e = install::install_pack(&data, &out, &trust()).unwrap_err();
        assert!(
            e.code == "PACK_PATH" || e.code == "PACK_ZIP",
            "{name}: {}",
            e.code
        );
        nothing_installed(&data);
        assert!(!tmp.0.join("evil.json").exists());
    }
}

#[test]
fn an_extra_entry_or_a_bomb_sized_file_is_refused() {
    let tmp = temp("bomb");
    let data = tmp.0.join("data");
    let out = crafted(
        &tmp.0,
        "extra.loombundle",
        None,
        &[("data/extra.json", b"{}")],
    );
    assert_eq!(
        install::install_pack(&data, &out, &trust())
            .unwrap_err()
            .code,
        "PACK_UNLISTED"
    );
    // A manifest file far larger than the manifest says (deflated zeros:
    // a small pack, a large file). Refused by its declared size, before
    // extracting.
    let (path, size) = first_file();
    let zeros = vec![0u8; usize::try_from(size).unwrap() + 8 * 1024 * 1024];
    let out = crafted(&tmp.0, "bomb.loombundle", Some((&path, &zeros)), &[]);
    assert!(
        std::fs::metadata(&out).unwrap().len() < 1024 * 1024,
        "the pack is small"
    );
    assert_eq!(
        install::install_pack(&data, &out, &trust())
            .unwrap_err()
            .code,
        "PACK_SIZE"
    );
    nothing_installed(&data);
}

#[test]
fn a_file_of_the_right_size_and_other_bytes_is_refused_while_extracting() {
    let tmp = temp("content");
    let data = tmp.0.join("data");
    let (path, size) = first_file();
    let other = vec![b' '; usize::try_from(size).unwrap()];
    let out = crafted(&tmp.0, "content.loombundle", Some((&path, &other)), &[]);
    assert_eq!(
        install::install_pack(&data, &out, &trust())
            .unwrap_err()
            .code,
        "PACK_CONTENT"
    );
    nothing_installed(&data);
}

#[test]
fn other_content_with_the_same_id_is_replaced_only_by_a_trusted_pack() {
    let tmp = temp("exists");
    let data = tmp.0.join("data");
    let found = install::install_pack(&data, &pack(&tmp.0, None), &trust())
        .unwrap()
        .found;
    // Change the installed manifest (as another bundle with this ID would
    // differ).
    let mut m = manifest_bytes(&found.dir);
    m.push(b' ');
    std::fs::write(found.dir.join("bundle.json"), &m).unwrap();
    // An unsigned or unknown-key pack does not replace it.
    let e = install::install_pack(&data, &pack(&tmp.0, None), &trust()).unwrap_err();
    assert_eq!(e.code, "APP_PACK_EXISTS");
    let stranger = KeyPair::from_seed(&[9; 32]);
    let e = install::install_pack(&data, &pack(&tmp.0, Some(&stranger)), &trust()).unwrap_err();
    assert_eq!(e.code, "APP_PACK_EXISTS");
    assert_eq!(manifest_bytes(&found.dir), m);
    // A trusted one does, whole, and leaves no temporary folder.
    let replaced = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust()).unwrap();
    assert!(!replaced.already);
    assert!(replaced.found.status.loads());
    assert_eq!(manifest_bytes(&found.dir), manifest_bytes(&fixture()));
    install::verify_full(&found.dir).unwrap();
    assert!(walk(&data).iter().all(|p| {
        let name = p.to_string_lossy();
        !name.contains(".incoming-") && !name.contains(".outgoing-")
    }));
}

#[test]
fn installing_again_repairs_damaged_files() {
    let tmp = temp("repair");
    let data = tmp.0.join("data");
    let found = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust())
        .unwrap()
        .found;
    let (path, _) = first_file();
    std::fs::write(found.dir.join(&path), b"short").unwrap();
    assert!(
        !install::status_of(&found.dir, &trust())
            .unwrap()
            .status
            .loads()
    );
    // Even an unsigned pack of the same manifest repairs it, and the
    // trusted signature stays.
    let again = install::install_pack(&data, &pack(&tmp.0, None), &trust()).unwrap();
    assert!(!again.already);
    assert!(again.found.status.loads(), "{:?}", again.found.status);
    install::verify_full(&found.dir).unwrap();
}

#[test]
fn a_weaker_signature_never_replaces_a_trusted_one() {
    let tmp = temp("weaker");
    let data = tmp.0.join("data");
    install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust()).unwrap();
    let stranger = KeyPair::from_seed(&[9; 32]);
    let again = install::install_pack(&data, &pack(&tmp.0, Some(&stranger)), &trust()).unwrap();
    assert!(again.already);
    assert!(again.found.status.loads(), "{:?}", again.found.status);
    let unsigned = install::install_pack(&data, &pack(&tmp.0, None), &trust()).unwrap();
    assert!(unsigned.found.status.loads(), "{:?}", unsigned.found.status);
}

#[test]
fn a_temporary_folder_left_by_another_process_is_swept() {
    let tmp = temp("sweep");
    let data = tmp.0.join("data");
    let found = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust())
        .unwrap()
        .found;
    let game = found.dir.parent().unwrap().to_owned();
    let stale = game.join(".incoming-1-2");
    std::fs::create_dir_all(stale.join("data")).unwrap();
    let _ = install::scan(&data, &trust());
    assert!(!stale.exists());
}

/// The start loads an installed bundle checking sizes only (plan decision
/// P-e; `docs/phase-5/performance.md`): a same-size change still loads
/// there, where Verify and any folder not installed hash every file.
#[test]
fn the_start_checks_an_installed_bundles_sizes_and_other_folders_hashes() {
    let tmp = temp("start-sizes");
    let data = tmp.0.join("data");
    let found = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust())
        .unwrap()
        .found;
    let (path, _) = first_file();
    let file = found.dir.join(&path);
    let mut bytes = std::fs::read(&file).unwrap();
    let at = bytes.iter().position(|b| *b == b' ').unwrap();
    bytes[at] = b'\t';
    std::fs::write(&file, &bytes).unwrap();
    let mut library = loom_app::bundles::Library::new(&data, trust());
    library.load(&found.dir).unwrap();
    assert_eq!(
        loom_app::GameData::load(&found.dir).unwrap_err().code,
        "APP_BUNDLE"
    );
    assert_eq!(
        install::verify_full(&found.dir).unwrap_err().code,
        "APP_BUNDLE_DAMAGED"
    );
    // The same folder copied elsewhere is not installed: hashed.
    let copy = tmp.0.join("copy");
    for f in walk(&found.dir) {
        let to = copy.join(f.strip_prefix(&found.dir).unwrap());
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&f, &to).unwrap();
    }
    assert_eq!(library.load(&copy).unwrap_err().code, "APP_BUNDLE");
}

/// A shipped pack at start (plan decision P-e): installed once; at each
/// later start, a trusted copy of the same manifest with every size right
/// is left as it is, its files not hashed (a same-size change stays for
/// Verify to find); damage the sizes show is repaired; an accepted
/// unsigned copy gets the shipped pack's trusted signature.
#[test]
fn a_shipped_pack_installs_once_and_is_then_checked_by_sizes() {
    let tmp = temp("shipped");
    let data = tmp.0.join("data");
    let signed = pack(&tmp.0, Some(&key()));
    let first = install::install_shipped(&data, &signed, &trust()).unwrap();
    assert!(!first.already);
    let (path, _) = first_file();
    let file = first.found.dir.join(&path);
    let mut bytes = std::fs::read(&file).unwrap();
    let at = bytes.iter().position(|b| *b == b' ').unwrap();
    bytes[at] = b'\t';
    std::fs::write(&file, &bytes).unwrap();
    let again = install::install_shipped(&data, &signed, &trust()).unwrap();
    assert!(again.already);
    assert!(
        install::verify_full(&first.found.dir).is_err(),
        "not hashed"
    );
    // Another size: repaired.
    std::fs::write(&file, b"short").unwrap();
    let repaired = install::install_shipped(&data, &signed, &trust()).unwrap();
    assert!(!repaired.already);
    install::verify_full(&first.found.dir).unwrap();

    let tmp = temp("shipped-accepted");
    let data = tmp.0.join("data");
    let unsigned = install::install_pack(&data, &pack(&tmp.0, None), &trust())
        .unwrap()
        .found;
    install::accept(&unsigned, &unsigned.manifest_sha256).unwrap();
    let trusted = install::install_shipped(&data, &pack(&tmp.0, Some(&key())), &trust()).unwrap();
    assert!(
        matches!(trusted.found.status, Status::Trusted { .. }),
        "{:?}",
        trusted.found.status
    );
}

#[test]
fn a_damaged_installed_bundle_is_quarantined_or_fails_verification() {
    let tmp = temp("damaged");
    let data = tmp.0.join("data");
    let found = install::install_pack(&data, &pack(&tmp.0, Some(&key())), &trust())
        .unwrap()
        .found;
    let (path, size) = first_file();
    // Same size, other bytes: the start's size check passes; the full
    // verification does not.
    std::fs::write(
        found.dir.join(&path),
        vec![b' '; usize::try_from(size).unwrap()],
    )
    .unwrap();
    assert!(
        install::status_of(&found.dir, &trust())
            .unwrap()
            .status
            .loads()
    );
    assert_eq!(
        install::verify_full(&found.dir).unwrap_err().code,
        "APP_BUNDLE_DAMAGED"
    );
    // Another size: quarantined at the next start.
    std::fs::write(found.dir.join(&path), b"short").unwrap();
    assert_eq!(
        install::status_of(&found.dir, &trust()).unwrap().status,
        Status::Quarantined(Quarantine::Damaged)
    );
}

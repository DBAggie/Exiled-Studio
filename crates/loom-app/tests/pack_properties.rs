//! Properties of installing a pack (Phase 5, P6; `docs/phase-5/security.md`):
//! arbitrary bytes and single-byte changes to a real signed pack are
//! refused or installed whole, never a panic, never a half-written folder,
//! and whatever installs matches its manifest file by file. The fuzz target
//! `install_pack` in `fuzz/` drives the same function with coverage
//! guidance; these run in CI on every change.
#![expect(clippy::unwrap_used, reason = "test: temporary folders and fixtures")]

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use loom_app::install;
use loom_data::signature::{KeyPair, TRUST_FORMAT, TrustList};
use proptest::prelude::*;

fn trust() -> TrustList {
    TrustList {
        format: TRUST_FORMAT,
        keys: vec![KeyPair::test_key().trusted("test")],
        revoked: Vec::new(),
    }
}

struct Temp(PathBuf);

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp() -> Temp {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("loom-pack-prop-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Temp(dir)
}

/// The fixture bundle as a pack signed with the test key.
static PACK: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../loom-data/tests/fixtures/bundle-poe1");
    let manifest = std::fs::read(fixture.join("bundle.json")).unwrap();
    let sig = serde_json::to_vec(&KeyPair::test_key().sign(&manifest)).unwrap();
    let tmp = temp();
    let out = tmp.0.join("p.loombundle");
    install::write_pack(&fixture, Some(&sig), &out).unwrap();
    std::fs::read(out).unwrap()
});

/// No temporary extraction folder is left anywhere under `dir`.
fn no_incoming(dir: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return true;
    };
    read.flatten().all(|e| {
        let name = e.file_name();
        !name.to_string_lossy().starts_with(".incoming-")
            && (!e.path().is_dir() || no_incoming(&e.path()))
    })
}

/// Installs `bytes`; whatever installs must verify file by file.
fn check(bytes: Vec<u8>) -> Result<(), TestCaseError> {
    let tmp = temp();
    match install::install_from(&tmp.0, Cursor::new(bytes), &trust()) {
        Ok(installed) => {
            prop_assert!(install::verify_full(&installed.found.dir).is_ok());
        }
        Err(e) => {
            prop_assert!(
                e.code.starts_with("PACK_") || e.code.starts_with("APP_"),
                "{e}"
            );
        }
    }
    prop_assert!(no_incoming(&tmp.0));
    Ok(())
}

#[test]
fn the_unchanged_pack_should_install_trusted() {
    let tmp = temp();
    let installed = install::install_from(&tmp.0, Cursor::new(PACK.clone()), &trust()).unwrap();
    assert!(installed.found.status.loads());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn any_bytes_should_be_refused_without_panic_or_residue(
        bytes in proptest::collection::vec(any::<u8>(), 0..4096)
    ) {
        check(bytes)?;
    }

    #[test]
    fn a_changed_pack_should_be_refused_or_install_whole(
        changes in proptest::collection::vec(
            (any::<prop::sample::Index>(), any::<u8>()), 1..4
        )
    ) {
        let mut bytes = PACK.clone();
        for (at, to) in &changes {
            let i = at.index(bytes.len());
            if let Some(b) = bytes.get_mut(i) {
                *b = *to;
            }
        }
        check(bytes)?;
    }
}

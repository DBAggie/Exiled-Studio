//! A `.loombundle` pack (`install::install_from`: the zip, the signature,
//! the listing checks, extraction), into a fresh folder each run. Whatever
//! installs must verify file by file, and no temporary folder may remain.
#![no_main]

use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};

use libfuzzer_sys::fuzz_target;
use loom_app::install;
use loom_data::signature::{KeyPair, TRUST_FORMAT, TrustList};

static RUN: AtomicUsize = AtomicUsize::new(0);

fuzz_target!(|data: &[u8]| {
    let trust = TrustList {
        format: TRUST_FORMAT,
        keys: vec![KeyPair::test_key().trusted("test")],
        revoked: Vec::new(),
    };
    let dir = std::env::temp_dir().join(format!(
        "loom-fuzz-pack-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(installed) = install::install_from(&dir, Cursor::new(data), &trust) {
        assert!(install::verify_full(&installed.found.dir).is_ok());
    }
    let incoming = walk(&dir).any(|p| {
        p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with(".incoming-"))
    });
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!incoming, "a temporary folder was left");
});

fn walk(dir: &std::path::Path) -> impl Iterator<Item = std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p.clone());
            }
            out.push(p);
        }
    }
    out.into_iter()
}

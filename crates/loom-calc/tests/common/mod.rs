//! The synthetic PoE1 fixture bundle of `loom-data` (made-up rows), read
//! from its folder as the app reads a bundle.

use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, MemorySource};

#[expect(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "a test helper reads the fixture bundle (the crate itself does no I/O); a failure fails the test"
)]
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The synthetic PoE1 bundle.
pub fn bundle() -> Bundle {
    bundle_at(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../loom-data/tests/fixtures/bundle-poe1"),
    )
}

/// The bundle in the folder `root`.
#[expect(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "a test helper reads a bundle (the crate itself does no I/O); a failure fails the test"
)]
pub fn bundle_at(root: &Path) -> Bundle {
    let root = root.to_path_buf();
    let mut paths = Vec::new();
    files(&root, &mut paths);
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        src.insert(rel, std::fs::read(&p).unwrap());
    }
    Bundle::load(&src).unwrap()
}

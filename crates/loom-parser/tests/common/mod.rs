//! The synthetic fixture bundles of `loom-data`
//! (`crates/loom-data/tests/fixtures/`, made-up rows; plan decision 10),
//! validated before use, as the app does.

use std::fs;
use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, MemorySource, validate};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The bundle folder `root` as a memory source.
pub fn source(root: &Path) -> MemorySource {
    let mut paths = Vec::new();
    files(root, &mut paths);
    let mut src = MemorySource::new();
    for p in paths {
        let rel = p
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        src.insert(rel, fs::read(&p).unwrap());
    }
    src
}

/// The validated fixture bundle `name`.
pub fn bundle(name: &str) -> Bundle {
    let src = source(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../loom-data/tests/fixtures")
            .join(name),
    );
    let report = validate::validate(&src);
    assert!(report.is_valid(), "{name}: {:?}", report.codes());
    Bundle::load(&src).unwrap()
}

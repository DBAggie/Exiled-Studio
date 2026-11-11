//! Phase 2 gate 3c (plan, "Gate"): shared core crates name no PoE1-only
//! identifier outside the places allowed to (ADR-001; plan decision 5).
//!
//! Scanned: the `src/` trees of every core crate, except unit-test files
//! named `tests.rs` (which may build PoE1 data on purpose). Allowed by
//! design: any `poe1` module (`poe1.rs` or `poe1/`) of a core crate, which
//! is where PoE1-only content belongs (today `loom-domain/src/poe1.rs`),
//! and `loom-serialization/src/legacy/` (PoB's formats).
//!
//! A heuristic: it matches substrings, so it cannot see a renamed
//! constant, and prose naming a listed word fails it too (rephrase, or
//! move the text to a `poe1` module). The list grows with each lift PR.
//!
//! The list: PoE1-only build choices, PoE1-only table names, and the names
//! the Spike B harness gave to PoE1 data constants (mod domains,
//! generation types, influences, essence columns), which belong in the
//! data bundle's `item_rules` (plan decision 5).

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the workspace's own sources; never part of a core build"
)]

use std::fs;
use std::path::{Path, PathBuf};

const CORE_CRATES: [&str; 5] = [
    "loom-domain",
    "loom-data",
    "loom-parser",
    "loom-calc",
    "loom-serialization",
];

const POE1_ONLY: [&str; 15] = [
    "Bandit",
    "bandit",
    "Pantheon",
    "pantheon",
    "Essences",
    "AlternateQualityTypes",
    "Brequel",
    "IndexableNonActiveSupportGems",
    "IndexableSkillGems",
    "DOMAIN_SCOPES",
    "ModGenerationType",
    "ESSENCE_COLUMNS",
    "INFLUENCE_TYPES",
    "tincture_stat_descriptions",
    "graft_stat_descriptions",
];

fn allowed(path: &Path) -> bool {
    let p = path.to_string_lossy().replace('\\', "/");
    p.contains("/loom-serialization/src/legacy/")
        || p.ends_with("/poe1.rs")
        || p.contains("/poe1/")
        || p.ends_with("/tests.rs")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        entries.push(entry?.path());
    }
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn shared_core_sources_should_name_no_poe1_only_identifier() -> std::io::Result<()> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for name in CORE_CRATES {
        rust_files(&crates.join(name).join("src"), &mut files)?;
    }
    assert!(files.len() > 20, "too few sources found: {}", files.len());
    let mut found = Vec::new();
    for file in files.iter().filter(|f| !allowed(f)) {
        let text = fs::read_to_string(file)?;
        for (n, line) in text.lines().enumerate() {
            for word in POE1_ONLY.iter().filter(|w| line.contains(**w)) {
                found.push(format!("{}:{}: {word}", file.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "PoE1-only identifiers:\n{}",
        found.join("\n")
    );
    Ok(())
}

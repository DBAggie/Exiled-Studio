//! Build document v1 on every public fixture (Phase 2 plan, step 7):
//! `Build -> DTO -> JSON -> DTO -> Build` is equal and the JSON bytes are
//! the same on a second run.

#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the repository's public fixtures; never part of a core build"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a test reads fixed fixture files; a failure should panic"
)]

use std::fs;
use std::path::{Path, PathBuf};

use loom_serialization::dto::v1::{from_json, to_json};
use loom_serialization::legacy::pob_xml::import;

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/oracle/fixtures");
    let mut out = Vec::new();
    for dir in [root.clone(), root.join("community")] {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "xml")
                && p.file_name().is_some_and(|n| n != "malformed.xml")
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn public_fixtures_should_round_trip_through_json() {
    let mut checked = 0;
    let mut failed = Vec::new();
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let a = import(&fs::read_to_string(&path).unwrap()).unwrap();
        let json = match to_json(&a) {
            Ok(j) => j,
            Err(e) => {
                failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        if to_json(&a).unwrap() != json {
            failed.push(format!("{name}: a second run differs"));
        }
        match from_json(&json) {
            Ok(b) if b == a => {}
            Ok(_) => failed.push(format!("{name}: the build read back differs")),
            Err(e) => failed.push(format!("{name}: {e}")),
        }
        checked += 1;
    }
    assert!(checked >= 24, "only {checked} fixtures");
    assert!(failed.is_empty(), "{failed:#?}");
}

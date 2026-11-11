//! The PoB XML adapter on every public fixture (Phase 2 plan, step 5):
//! `xml_equal(export(import(x)), x)` and `import(export(import(x))) ==
//! import(x)`; the malformed fixture gives its code.

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

use loom_domain::{BuildPart, Entry};
use loom_serialization::legacy::pob_xml::{export, import, xml_equal};

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/oracle/fixtures");
    let mut out = Vec::new();
    for dir in [root.clone(), root.join("community")] {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "xml") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn public_fixtures_should_round_trip() {
    let mut checked = 0;
    let mut failed = Vec::new();
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name == "malformed.xml" {
            continue;
        }
        let xml = fs::read_to_string(&path).unwrap();
        let a = import(&xml).unwrap();
        let out = export(&a);
        if !xml_equal(&out, &xml).unwrap() {
            failed.push(format!("{name}: xml_equal"));
        }
        if import(&out).unwrap() != a {
            failed.push(format!("{name}: re-import"));
        }
        // The sections were typed, not kept whole by the fallback.
        let typed: Vec<BuildPart> = a
            .layout
            .entries()
            .iter()
            .filter_map(|e| match e {
                Entry::Typed(p) => Some(*p),
                Entry::Opaque(_) => None,
            })
            .collect();
        let want = [
            BuildPart::Character,
            BuildPart::Passives,
            BuildPart::Skills,
            BuildPart::Config,
        ];
        if !want.iter().all(|p| typed.contains(p)) {
            failed.push(format!("{name}: typed sections {typed:?}"));
        }
        // Every community build has skills and items (our synthetic
        // fixtures may have none).
        let community = path.parent().is_some_and(|d| d.ends_with("community"));
        if community && a.skills.sets.typed().count() == 0 {
            failed.push(format!("{name}: no typed skill set"));
        }
        checked += 1;
    }
    assert!(checked >= 24, "only {checked} fixtures");
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn malformed_fixture_should_give_an_import_code() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/oracle/fixtures/malformed.xml");
    let xml = fs::read_to_string(path).unwrap();
    let code = import(&xml).err().map(|e| e.code());
    assert!(
        matches!(code, Some("IMPORT_XML" | "IMPORT_ROOT")),
        "{code:?}"
    );
}

/// Maintainer-run: every `*.xml` directly in the folder `LOOM_XML_DIR`
/// round-trips (the corpus is local, ADR-007; only the counts are
/// printed). `LOOM_XML_DIR=<dir> cargo test -p loom-serialization --test
/// pob_xml_fixtures -- --ignored --nocapture`.
#[test]
#[ignore = "needs LOOM_XML_DIR (local builds)"]
#[expect(
    clippy::print_stdout,
    reason = "a maintainer-run test reports its counts"
)]
fn folder_should_round_trip() {
    let dir = std::env::var("LOOM_XML_DIR").unwrap();
    let (mut ok, mut failed) = (0, 0);
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("xml")) {
            continue;
        }
        let Ok(xml) = fs::read_to_string(&p) else {
            continue;
        };
        let Ok(a) = import(&xml) else {
            failed += 1;
            continue;
        };
        let out = export(&a);
        if xml_equal(&out, &xml).unwrap() && import(&out).unwrap() == a {
            ok += 1;
        } else {
            failed += 1;
        }
    }
    println!("round-trip: {ok} ok, {failed} failed");
    assert_eq!(failed, 0);
}

//! Share codes on the public fixtures and on codes made by PoB v2.67.2
//! (Phase 2 plan, step 10): a code imports to the same build as its XML.

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

use loom_serialization::legacy::{pob_xml, share_code};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/oracle/fixtures")
}

fn files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut out: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == extension))
        .collect();
    out.sort();
    out
}

/// Accept, first half: a code made by our exporter from each public
/// fixture imports to the same build as the fixture's XML; a fixture that
/// does not import gives the same code through a share code.
#[test]
fn our_codes_of_the_public_fixtures_should_import_as_their_xml() {
    let root = fixtures_dir();
    let (mut imported, mut refused) = (0, Vec::new());
    let fixtures: Vec<_> = files(&root, "xml")
        .into_iter()
        .chain(files(&root.join("community"), "xml"))
        .collect();
    for path in &fixtures {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let xml = fs::read_to_string(path).unwrap();
        match pob_xml::import(&xml) {
            Ok(build) => {
                let code = share_code::export(&build);
                assert_eq!(share_code::import(&code).unwrap(), build, "{name}");
                // The fixture's own text as a code reads the same too.
                let raw = share_code::encode(&xml);
                assert_eq!(share_code::decode(&raw).unwrap(), xml, "{name}");
                assert_eq!(share_code::import(&raw).unwrap(), build, "{name}");
                imported += 1;
            }
            Err(e) => {
                let through_code = share_code::import(&share_code::encode(&xml)).unwrap_err();
                assert_eq!(through_code.code(), e.code(), "{name}");
                refused.push(name);
            }
        }
    }
    // Every fixture but the malformed one imports, however many there are
    // (a community build can be removed on request, SOURCES.md).
    assert_eq!(refused, ["malformed.xml"]);
    assert_eq!(imported, fixtures.len() - 1);
}

/// Accept, second half: codes made by PoB v2.67.2 on the oracle runtime
/// (`tools/oracle/adapter/share_code.lua`), each recorded with the XML
/// PoB made it from, decode to that XML exactly and import to its build.
#[test]
fn codes_made_by_pob_should_import_as_their_xml() {
    let dir = fixtures_dir().join("share-codes");
    // Whatever codes are there (a community build's are removed with it);
    // ours is always there.
    let codes = files(&dir, "code");
    assert!(
        codes.iter().any(|p| p.ends_with("minimal-level1.code")),
        "{codes:?}"
    );
    for path in codes {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let code = fs::read_to_string(&path).unwrap();
        let xml = fs::read_to_string(path.with_extension("pob.xml")).unwrap();
        // PoB's alphabet: no `+` or `/`.
        assert!(!code.contains(['+', '/']), "{name}");
        assert_eq!(share_code::decode(&code).unwrap(), xml, "{name}");
        let build = pob_xml::import(&xml).unwrap();
        // The same items as the fixture the code was made from.
        let source = ["", "community"]
            .iter()
            .map(|d| fixtures_dir().join(d).join(name.replace(".code", ".xml")))
            .find(|p| p.exists())
            .unwrap();
        let fixture = pob_xml::import(&fs::read_to_string(source).unwrap()).unwrap();
        assert_eq!(
            build.items.items().count(),
            fixture.items.items().count(),
            "{name}"
        );
        assert_eq!(share_code::import(&code).unwrap(), build, "{name}");
        // Our code of the same build reads back to it too.
        assert_eq!(
            share_code::import(&share_code::export(&build)).unwrap(),
            build,
            "{name}"
        );
    }
}

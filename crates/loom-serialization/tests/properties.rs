//! Properties of the readers of files a user opens or pastes (Phase 5, P6;
//! `docs/phase-5/security.md`): PoB XML, share codes, the build document.
//! Arbitrary input and single-byte changes to real documents are read or
//! refused, never a panic. The fuzz targets in `fuzz/` drive the same
//! functions with coverage guidance; these run in CI on every change.
#![expect(
    clippy::disallowed_methods,
    reason = "a test reads the repository's public fixtures; never part of a core build"
)]
#![expect(clippy::unwrap_used, reason = "test: fixture files")]

use std::path::Path;
use std::sync::LazyLock;

use loom_serialization::dto::v1;
use loom_serialization::legacy::{pob_xml, share_code};
use proptest::prelude::*;

fn read(relative: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

static XML: LazyLock<Vec<u8>> =
    LazyLock::new(|| read("../../tools/oracle/fixtures/minimal-level1.xml"));
static BUILD: LazyLock<Vec<u8>> = LazyLock::new(|| read("src/dto/v1/testdata/poe1.json"));

/// `bytes` with up to four bytes replaced.
fn mutated(bytes: &[u8], changes: &[(prop::sample::Index, u8)]) -> String {
    let mut out = bytes.to_vec();
    for (at, to) in changes {
        let i = at.index(out.len());
        if let Some(b) = out.get_mut(i) {
            *b = *to;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn changes() -> impl Strategy<Value = Vec<(prop::sample::Index, u8)>> {
    proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..4)
}

proptest! {
    #[test]
    fn any_text_should_import_as_pob_xml_or_be_refused(text in ".{0,512}") {
        let _ = pob_xml::import(&text);
    }

    #[test]
    fn a_changed_pob_document_should_import_or_be_refused(changes in changes()) {
        let _ = pob_xml::import(&mutated(&XML, &changes));
    }

    #[test]
    fn any_text_should_import_as_a_share_code_or_be_refused(text in "[A-Za-z0-9_=+/-]{0,512}") {
        let _ = share_code::import(&text);
    }

    #[test]
    fn a_share_code_should_decode_to_the_text_it_encodes(text in ".{0,2048}") {
        prop_assert_eq!(share_code::decode(&share_code::encode(&text)).unwrap(), text);
    }

    #[test]
    fn a_changed_share_code_should_import_or_be_refused(changes in changes()) {
        let code = share_code::encode(&String::from_utf8_lossy(&XML));
        let _ = share_code::import(&mutated(code.as_bytes(), &changes));
    }

    #[test]
    fn any_text_should_read_as_a_build_document_or_be_refused(text in ".{0,512}") {
        let _ = v1::from_json(&text);
    }

    #[test]
    fn a_changed_build_document_should_read_or_be_refused(changes in changes()) {
        let _ = v1::from_json(&mutated(&BUILD, &changes));
    }
}

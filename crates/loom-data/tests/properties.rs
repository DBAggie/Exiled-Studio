//! Properties of the hostile-input entry points (Phase 5, P6;
//! `docs/phase-5/security.md`): the pack manifest and listing, the
//! signature, the data feed. The fuzz targets in `fuzz/` drive the same
//! functions with coverage guidance; these run in CI on every change.
#![expect(clippy::unwrap_used, reason = "test: fixtures built in the test")]

use loom_data::pack::{self, Entry};
use loom_data::signature::{self, KeyPair, Signature, TRUST_FORMAT, TrustList};
use proptest::prelude::*;

fn trust() -> TrustList {
    TrustList {
        format: TRUST_FORMAT,
        keys: vec![KeyPair::test_key().trusted("test")],
        revoked: Vec::new(),
    }
}

const MANIFEST: &str = r#"{"format":2,"data_schema":4,"game":"poe1","source":{"steam_build":1,"index_sha256":"","extractor_commit":"","decoder":"","dat_schema":""},"files":[{"path":"data/a.json","sha256":"00","size":3}],"signatures":[]}"#;

fn entry(name: &str, size: u64) -> Entry {
    Entry {
        name: name.to_owned(),
        size,
        is_file: true,
    }
}

fn segment() -> impl Strategy<Value = String> {
    "[a-z0-9_.-]{1,8}".prop_filter("a plain segment", |s| s != "." && s != "..")
}

/// Entry names that would escape the bundle folder or are not plain.
fn hostile_name() -> impl Strategy<Value = String> {
    (segment(), segment(), 0..8_u8).prop_map(|(a, b, kind)| match kind {
        0 => format!("../{a}"),
        1 => format!("{a}/../{b}"),
        2 => format!("/{a}"),
        3 => format!("C:/{a}"),
        4 => format!("{a}\\{b}"),
        5 => format!("{a}/./{b}"),
        6 => format!("{a}//{b}"),
        _ => format!("{a}/"),
    })
}

fn feed(sequence: u64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "format": 1,
        "sequence": sequence,
        "bundles": [{
            "id": "poe1/1-s4",
            "url": "https://example.invalid/a.loombundle",
            "sha256": "0".repeat(64),
            "size": 1
        }]
    }))
    .unwrap()
}

proptest! {
    #[test]
    fn any_manifest_bytes_should_be_read_or_refused_without_panic(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048)
    ) {
        let _ = pack::manifest(&bytes);
    }

    #[test]
    fn any_listing_should_be_checked_without_panic(
        entries in proptest::collection::vec(
            (".{0,24}", any::<u64>(), any::<bool>()), 0..12
        )
    ) {
        let manifest = pack::manifest(MANIFEST.as_bytes()).unwrap();
        let entries: Vec<Entry> = entries
            .into_iter()
            .map(|(name, size, is_file)| Entry { name, size, is_file })
            .collect();
        let _ = pack::check_listing(&manifest, &entries);
    }

    #[test]
    fn a_hostile_entry_name_should_be_refused_as_a_path(name in hostile_name()) {
        let manifest = pack::manifest(MANIFEST.as_bytes()).unwrap();
        let entries = vec![entry("bundle.json", 100), entry("data/a.json", 3), entry(&name, 1)];
        let error = pack::check_listing(&manifest, &entries).unwrap_err();
        prop_assert_eq!(error.code(), "PACK_PATH", "{}", name);
    }

    #[test]
    fn a_declared_size_other_than_the_manifests_should_be_refused(size in any::<u64>()) {
        prop_assume!(size != 3);
        let manifest = pack::manifest(MANIFEST.as_bytes()).unwrap();
        let entries = vec![entry("bundle.json", 100), entry("data/a.json", size)];
        prop_assert!(pack::check_listing(&manifest, &entries).is_err());
    }

    #[test]
    fn a_manifest_changed_after_signing_should_never_be_trusted(
        at in any::<prop::sample::Index>(),
        to in any::<u8>()
    ) {
        let original = MANIFEST.as_bytes();
        let sig = serde_json::to_vec(&KeyPair::test_key().sign(original)).unwrap();
        let mut changed = original.to_vec();
        let i = at.index(changed.len());
        prop_assume!(changed.get(i) != Some(&to));
        if let Some(b) = changed.get_mut(i) {
            *b = to;
        }
        let trusted = pack::trust_of(&changed, Some(&sig), &trust())
            .is_ok_and(|t| t.is_trusted());
        prop_assert!(!trusted);
    }

    #[test]
    fn any_signature_should_be_checked_without_panic(
        message in proptest::collection::vec(any::<u8>(), 0..256),
        format in 0..3_u32,
        algorithm in prop_oneof![Just("ed25519".to_owned()), ".{0,12}"],
        known_key in any::<bool>(),
        hex in "[0-9a-fA-F]{0,140}"
    ) {
        let key_id = if known_key {
            signature::key_id(&KeyPair::test_key().public_key())
        } else {
            "ed25519:0000000000000000".to_owned()
        };
        let sig = Signature { format, algorithm, key_id, signature: hex };
        prop_assert!(signature::verify(&message, &sig, &trust()).is_err());
    }

    #[test]
    fn any_feed_and_signature_bytes_should_be_refused_without_panic(
        feed in proptest::collection::vec(any::<u8>(), 0..1024),
        sig in proptest::collection::vec(any::<u8>(), 0..512),
        seen in any::<Option<u64>>()
    ) {
        prop_assert!(pack::verify_feed(&feed, &sig, &trust(), seen).is_err());
    }

    #[test]
    fn a_signed_feed_should_pass_exactly_when_not_below_the_highest_seen(
        sequence in 0..1_000_u64,
        seen in 0..1_000_u64
    ) {
        let bytes = feed(sequence);
        let sig = serde_json::to_vec(&KeyPair::test_key().sign(&bytes)).unwrap();
        let result = pack::verify_feed(&bytes, &sig, &trust(), Some(seen));
        if sequence >= seen {
            prop_assert_eq!(result.map(|f| f.sequence).ok(), Some(sequence));
        } else {
            prop_assert_eq!(result.map_err(|e| e.code()).err(), Some("FEED_ROLLBACK"));
        }
    }
}

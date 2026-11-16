//! The data keys the application trusts (Phase 5, P2;
//! `docs/release/signing.md`): the release trust list compiled in, and the
//! public test key's list only when the host asks for it (debug and `e2e`
//! builds; `loom-tauri` decides). Verification itself is
//! `loom_data::signature`.

use loom_data::signature::{self, Signature, SignatureError, TrustList, TrustedKey};

/// `release/trust.json`: the production data keys and revocations.
const RELEASE: &[u8] = include_bytes!("../../../release/trust.json");

/// `release/trust-test.json`: the public test key (anyone can sign with it).
const TEST: &[u8] = include_bytes!("../../../release/trust-test.json");

/// The trust list: the release keys, plus the test key when `test_keys`.
///
/// # Errors
/// A compiled-in list that does not parse (a test guards both).
pub fn trust_list(test_keys: bool) -> Result<TrustList, SignatureError> {
    let release = TrustList::parse(RELEASE)?;
    if test_keys {
        Ok(release.with(&TrustList::parse(TEST)?))
    } else {
        Ok(release)
    }
}

/// Verifies a manifest's exact bytes against its `signature.json` bytes.
///
/// # Errors
/// [`SignatureError::Malformed`] when the signature file does not parse; the
/// errors of [`signature::verify`] otherwise.
pub fn verify_manifest<'t>(
    manifest: &[u8],
    signature_json: &[u8],
    trust: &'t TrustList,
) -> Result<&'t TrustedKey, SignatureError> {
    let sig: Signature = serde_json::from_slice(signature_json)
        .map_err(|e| SignatureError::Malformed(format!("signature file: {e}")))?;
    signature::verify(manifest, &sig, trust)
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_data::signature::KeyPair;

    fn signed(message: &[u8], pair: &KeyPair) -> Vec<u8> {
        serde_json::to_vec(&pair.sign(message)).unwrap_or_default()
    }

    #[test]
    fn the_compiled_lists_parse_and_the_test_key_is_only_in_the_test_list() {
        let test = KeyPair::test_key().trusted("");
        let release = trust_list(false).ok();
        let with_test = trust_list(true).ok();
        assert!(release.is_some_and(|l| l.keys.iter().all(|k| k.id != test.id)));
        let with_test = with_test.unwrap_or_default();
        let key = with_test.keys.iter().find(|k| k.id == test.id);
        // `release/trust-test.json` is `loom-release test-trust`'s output.
        assert_eq!(
            key.map(|k| k.public_key.as_str()),
            Some(test.public_key.as_str())
        );
    }

    #[test]
    fn a_test_signed_manifest_verifies_only_with_test_keys() {
        let manifest = br#"{"format":1}"#;
        let sig = signed(manifest, &KeyPair::test_key());
        let release = trust_list(false).unwrap_or_default();
        let with_test = trust_list(true).unwrap_or_default();
        assert!(matches!(
            verify_manifest(manifest, &sig, &release),
            Err(SignatureError::UnknownKey(_))
        ));
        assert!(verify_manifest(manifest, &sig, &with_test).is_ok());
        assert!(matches!(
            verify_manifest(br#"{"format":2}"#, &sig, &with_test),
            Err(SignatureError::Invalid)
        ));
        assert!(matches!(
            verify_manifest(manifest, b"not json", &with_test),
            Err(SignatureError::Malformed(_))
        ));
    }
}

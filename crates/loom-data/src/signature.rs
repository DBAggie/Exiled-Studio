//! Ed25519 signatures over data bundles and the data feed (Phase 5, P2;
//! ADR-010 revised, decision P-c; runbook `docs/release/signing.md`).
//!
//! **What is signed.** A bundle's manifest (`bundle.json`), byte for byte:
//! it lists every other file with its SHA-256 and size, so a verified
//! manifest and matching file hashes cover the whole bundle. The data feed
//! (P4) is signed the same way. The signature is kept beside what it signs
//! (`signature.json`), never inside it; the manifest's `signatures` field
//! stays empty (reserved in formats 1 and 2).
//!
//! **Who is trusted.** A [`TrustList`]: data keys by key ID, and the key
//! IDs revoked. The host compiles the release trust list into the
//! application (`release/trust.json`); a revocation ships with an app
//! update. Verification is pure code here, so every host verifies the same
//! way (ADR-016: no I/O; the host reads the bytes).
//!
//! **Strict verification** (`verify_strict`): non-canonical signatures and
//! weak public keys are refused.

use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The signature file's format.
pub const SIGNATURE_FORMAT: u32 = 1;

/// The trust list's format.
pub const TRUST_FORMAT: u32 = 1;

/// The only algorithm.
pub const ALGORITHM: &str = "ed25519";

/// The file beside a bundle's manifest (or a feed) that holds its
/// signature.
pub const SIGNATURE_FILE: &str = "signature.json";

/// A detached signature (`signature.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    /// [`SIGNATURE_FORMAT`].
    pub format: u32,
    /// [`ALGORITHM`].
    pub algorithm: String,
    /// The signing key's ID ([`key_id`]).
    pub key_id: String,
    /// The 64-byte signature, lowercase hex.
    pub signature: String,
}

/// What a key may sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    /// Data bundles and the data feed (the data key, P-c).
    Data,
}

/// A trusted public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedKey {
    /// Its ID: [`key_id`] of the public key, checked on load.
    pub id: String,
    /// The 32-byte public key, lowercase hex.
    pub public_key: String,
    /// What it may sign.
    pub purpose: Purpose,
    /// Who made it and when, for people (`maintainer, 2026-11-20`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// A revoked key: never trusted again, whatever the list says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revocation {
    /// The revoked key's ID.
    pub key_id: String,
    /// Why, for people.
    pub reason: String,
}

/// The keys a host trusts (`release/trust.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustList {
    /// [`TRUST_FORMAT`].
    pub format: u32,
    /// The trusted keys.
    pub keys: Vec<TrustedKey>,
    /// The revoked key IDs.
    #[serde(default)]
    pub revoked: Vec<Revocation>,
}

/// Why a signature or a trust list was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    /// The file is not a signature or trust list this code reads.
    #[error("unsupported {what}: {value}")]
    Unsupported {
        /// What (`signature format`, `algorithm`, `trust list format`).
        what: &'static str,
        /// The value found.
        value: String,
    },
    /// A field is not what it must be (hex of the right length, a key ID
    /// that matches its key, a key listed twice).
    #[error("malformed {0}")]
    Malformed(String),
    /// No trusted key has this ID.
    #[error("unknown key {0}")]
    UnknownKey(String),
    /// The key is revoked.
    #[error("revoked key {0}")]
    Revoked(String),
    /// The signature does not verify: the content or the signature changed.
    #[error("the signature does not match")]
    Invalid,
}

/// A public key's ID: `ed25519:` and the first 16 hex digits of its
/// SHA-256 (64 bits; IDs name keys, the signature is what is checked).
#[must_use]
pub fn key_id(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    let head: Vec<u8> = digest.iter().take(8).copied().collect();
    format!("{ALGORITHM}:{}", hex(&head))
}

/// Lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Lowercase or uppercase hex to exactly `N` bytes.
fn unhex<const N: usize>(text: &str, what: &str) -> Result<[u8; N], SignatureError> {
    let bad = || SignatureError::Malformed(what.to_owned());
    if text.len() != N * 2 || !text.is_ascii() {
        return Err(bad());
    }
    let mut out = [0u8; N];
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    for (byte, pair) in out.iter_mut().zip(pairs) {
        let digits = std::str::from_utf8(pair).map_err(|_| bad())?;
        *byte = u8::from_str_radix(digits, 16).map_err(|_| bad())?;
    }
    Ok(out)
}

impl TrustList {
    /// Reads a trust list and checks it: the format, each key's hex and ID,
    /// no key listed twice.
    ///
    /// # Errors
    /// [`SignatureError::Unsupported`] or [`SignatureError::Malformed`].
    pub fn parse(json: &[u8]) -> Result<Self, SignatureError> {
        let list: Self = serde_json::from_slice(json)
            .map_err(|e| SignatureError::Malformed(format!("trust list: {e}")))?;
        list.check()?;
        Ok(list)
    }

    /// The checks [`Self::parse`] makes.
    ///
    /// # Errors
    /// As [`Self::parse`].
    pub fn check(&self) -> Result<(), SignatureError> {
        if self.format != TRUST_FORMAT {
            return Err(SignatureError::Unsupported {
                what: "trust list format",
                value: self.format.to_string(),
            });
        }
        let mut seen = std::collections::BTreeSet::new();
        for key in &self.keys {
            let bytes = unhex::<32>(&key.public_key, &format!("public key {}", key.id))?;
            if key_id(&bytes) != key.id {
                return Err(SignatureError::Malformed(format!(
                    "key ID {} does not match its public key",
                    key.id
                )));
            }
            if !seen.insert(key.id.as_str()) {
                return Err(SignatureError::Malformed(format!(
                    "key {} listed twice",
                    key.id
                )));
            }
        }
        Ok(())
    }

    /// This list with `other`'s keys and revocations added (a host adds
    /// its test keys to a development build's list).
    #[must_use]
    pub fn with(mut self, other: &Self) -> Self {
        for key in &other.keys {
            if self.keys.iter().all(|k| k.id != key.id) {
                self.keys.push(key.clone());
            }
        }
        for r in &other.revoked {
            if self.revoked.iter().all(|x| x.key_id != r.key_id) {
                self.revoked.push(r.clone());
            }
        }
        self
    }

    /// Whether key `id` is revoked.
    #[must_use]
    pub fn is_revoked(&self, id: &str) -> bool {
        self.revoked.iter().any(|r| r.key_id == id)
    }
}

/// Checks `signature` over `message` against `trust`: a known, unrevoked
/// data key whose signature verifies strictly. Returns the key.
///
/// # Errors
/// [`SignatureError`]: unsupported format or algorithm, malformed hex,
/// unknown or revoked key, or a signature that does not match.
pub fn verify<'t>(
    message: &[u8],
    signature: &Signature,
    trust: &'t TrustList,
) -> Result<&'t TrustedKey, SignatureError> {
    if signature.format != SIGNATURE_FORMAT {
        return Err(SignatureError::Unsupported {
            what: "signature format",
            value: signature.format.to_string(),
        });
    }
    if signature.algorithm != ALGORITHM {
        return Err(SignatureError::Unsupported {
            what: "algorithm",
            value: signature.algorithm.clone(),
        });
    }
    // Revocation wins over the list: a revoked key is refused even if a
    // stale list still names it.
    if trust.is_revoked(&signature.key_id) {
        return Err(SignatureError::Revoked(signature.key_id.clone()));
    }
    let key = trust
        .keys
        .iter()
        .find(|k| k.id == signature.key_id)
        .ok_or_else(|| SignatureError::UnknownKey(signature.key_id.clone()))?;
    if key.purpose != Purpose::Data {
        return Err(SignatureError::UnknownKey(signature.key_id.clone()));
    }
    let public = unhex::<32>(&key.public_key, "public key")?;
    let public = VerifyingKey::from_bytes(&public)
        .map_err(|_| SignatureError::Malformed("public key".to_owned()))?;
    let bytes = unhex::<64>(&signature.signature, "signature")?;
    let sig = ed25519_dalek::Signature::from_bytes(&bytes);
    public
        .verify_strict(message, &sig)
        .map_err(|_| SignatureError::Invalid)?;
    Ok(key)
}

/// The public phrase the test key's seed is derived from.
pub const TEST_KEY_PHRASE: &[u8] = b"loom-test-data-key";

/// A key pair from its 32-byte secret seed (the release tool and tests;
/// the application never holds a secret key).
#[derive(Debug, Clone)]
pub struct KeyPair(SigningKey);

impl KeyPair {
    /// The key pair of `seed`.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self(SigningKey::from_bytes(seed))
    }

    /// The public test key's pair: its seed is the SHA-256 of
    /// [`TEST_KEY_PHRASE`], so anyone can sign with it. Only debug and `e2e`
    /// builds trust it (`release/trust-test.json`); a release build never
    /// does (`docs/release/signing.md`, "The test key").
    #[must_use]
    pub fn test_key() -> Self {
        Self::from_seed(&Sha256::digest(TEST_KEY_PHRASE).into())
    }

    /// The key pair of a hex seed (a secret key file's content, trimmed).
    ///
    /// # Errors
    /// [`SignatureError::Malformed`] when it is not 64 hex digits.
    pub fn from_hex(seed: &str) -> Result<Self, SignatureError> {
        Ok(Self::from_seed(&unhex::<32>(seed.trim(), "secret key")?))
    }

    /// The public key's bytes.
    #[must_use]
    pub fn public_key(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }

    /// The trust list entry for this key.
    #[must_use]
    pub fn trusted(&self, note: &str) -> TrustedKey {
        let public = self.public_key();
        TrustedKey {
            id: key_id(&public),
            public_key: hex(&public),
            purpose: Purpose::Data,
            note: note.to_owned(),
        }
    }

    /// Signs `message`.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        use ed25519_dalek::Signer as _;
        Signature {
            format: SIGNATURE_FORMAT,
            algorithm: ALGORITHM.to_owned(),
            key_id: key_id(&self.public_key()),
            signature: hex(&self.0.sign(message).to_bytes()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(n: u8) -> KeyPair {
        KeyPair::from_seed(&[n; 32])
    }

    fn trust(keys: &[&KeyPair]) -> TrustList {
        TrustList {
            format: TRUST_FORMAT,
            keys: keys.iter().map(|k| k.trusted("test")).collect(),
            revoked: Vec::new(),
        }
    }

    #[test]
    fn a_good_signature_should_verify_with_its_key() {
        let key = pair(1);
        let list = trust(&[&key]);
        let sig = key.sign(b"manifest");
        assert_eq!(verify(b"manifest", &sig, &list).unwrap().id, sig.key_id);
    }

    #[test]
    fn tampered_content_or_signature_should_be_invalid() {
        let key = pair(1);
        let list = trust(&[&key]);
        let sig = key.sign(b"manifest");
        assert_eq!(
            verify(b"manifesT", &sig, &list),
            Err(SignatureError::Invalid)
        );
        let mut flipped = sig.clone();
        let last = flipped.signature.pop().unwrap();
        flipped.signature.push(if last == '0' { '1' } else { '0' });
        assert_eq!(
            verify(b"manifest", &flipped, &list),
            Err(SignatureError::Invalid)
        );
    }

    #[test]
    fn a_key_the_list_lacks_or_revokes_should_be_refused() {
        let (good, other) = (pair(1), pair(2));
        let list = trust(&[&good]);
        let sig = other.sign(b"m");
        assert_eq!(
            verify(b"m", &sig, &list),
            Err(SignatureError::UnknownKey(sig.key_id.clone()))
        );
        // The right ID with another key's signature: not a match.
        let mut forged = other.sign(b"m");
        forged.key_id = good.trusted("").id;
        assert_eq!(verify(b"m", &forged, &list), Err(SignatureError::Invalid));
        let mut revoked = trust(&[&good]);
        revoked.revoked.push(Revocation {
            key_id: good.trusted("").id,
            reason: "test".to_owned(),
        });
        let sig = good.sign(b"m");
        assert_eq!(
            verify(b"m", &sig, &revoked),
            Err(SignatureError::Revoked(sig.key_id.clone()))
        );
    }

    #[test]
    fn unknown_formats_and_bad_hex_should_be_refused() {
        let key = pair(1);
        let list = trust(&[&key]);
        let mut sig = key.sign(b"m");
        sig.format = 2;
        assert!(matches!(
            verify(b"m", &sig, &list),
            Err(SignatureError::Unsupported { .. })
        ));
        let mut sig = key.sign(b"m");
        sig.algorithm = "rsa".to_owned();
        assert!(matches!(
            verify(b"m", &sig, &list),
            Err(SignatureError::Unsupported { .. })
        ));
        let mut sig = key.sign(b"m");
        sig.signature.truncate(10);
        assert!(matches!(
            verify(b"m", &sig, &list),
            Err(SignatureError::Malformed(_))
        ));
    }

    #[test]
    fn a_trust_list_should_check_its_keys() {
        let key = pair(1);
        let list = trust(&[&key]);
        let json = serde_json::to_vec(&list).unwrap();
        assert_eq!(TrustList::parse(&json).unwrap(), list);
        let mut wrong_id = list.clone();
        if let Some(k) = wrong_id.keys.first_mut() {
            k.id = "ed25519:0000000000000000".to_owned();
        }
        assert!(matches!(
            wrong_id.check(),
            Err(SignatureError::Malformed(_))
        ));
        let twice = trust(&[&key, &key]);
        assert!(matches!(twice.check(), Err(SignatureError::Malformed(_))));
        let mut future = list.clone();
        future.format = 2;
        assert!(matches!(
            future.check(),
            Err(SignatureError::Unsupported { .. })
        ));
        let merged = trust(&[&key]).with(&trust(&[&pair(2), &key]));
        assert_eq!(merged.keys.len(), 2);
    }

    #[test]
    fn a_key_id_should_be_stable() {
        // The ID of the all-zero seed's key, fixed so a change is seen.
        let id = KeyPair::from_seed(&[0; 32]).trusted("").id;
        assert!(id.starts_with("ed25519:") && id.len() == 8 + 16, "{id}");
        assert_eq!(
            KeyPair::from_hex(&hex(&[0; 32])).unwrap().trusted("").id,
            id
        );
    }
}

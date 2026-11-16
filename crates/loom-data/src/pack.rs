//! Data bundle packs and the data feed (Phase 5, P4; plan decision P-e;
//! `docs/phase-5/data-bundles.md`): the checks that need no filesystem.
//!
//! A pack (`.loombundle`) is a zip of a bundle: `bundle.json`, an optional
//! detached `signature.json` over its exact bytes (`crate::signature`), and
//! every file the manifest lists, nothing else. **Unpacking is hostile-input
//! code.** The host reads `bundle.json` and `signature.json` first, decides
//! the pack's [`Trust`], then checks the archive's listing with
//! [`check_listing`] before anything is extracted; it then extracts each
//! file checking its size and hash as it goes (`loom_app::install`).
//!
//! The data feed (`data-feed.json`, signed like a manifest) lists the packs
//! the project publishes, with a sequence number that only grows
//! ([`verify_feed`]): a lower one is a replayed old feed and is refused.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::bundle::{MANIFEST, Manifest, plain_path};
use crate::signature::{self, Signature, SignatureError, TrustList};

/// The pack file extension.
pub const PACK_EXTENSION: &str = "loombundle";

/// The most a pack may unpack to, in bytes (1 GiB; the real bundle is a
/// small fraction of it).
pub const MAX_UNPACKED: u64 = 1 << 30;

/// The most `bundle.json`, `signature.json`, or the feed may be, in bytes.
pub const MAX_SMALL_FILE: u64 = 16 << 20;

/// The files an installed bundle folder holds beside its manifest and
/// listed files: the signature, and the user's acceptance of a quarantined
/// bundle (`loom_app::install`). The validator does not count them as
/// unlisted.
pub const SIDE_FILES: [&str; 2] = [signature::SIGNATURE_FILE, "accepted.json"];

/// The most files a pack may hold.
pub const MAX_FILES: usize = 10_000;

/// Who vouches for a pack or an installed bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trust {
    /// Signed by a trusted, unrevoked key: installed and loaded.
    Trusted {
        /// The key's ID.
        key_id: String,
    },
    /// Signed by a key this build does not know: quarantined.
    UnknownKey {
        /// The key's ID.
        key_id: String,
    },
    /// No signature: quarantined.
    Unsigned,
}

impl Trust {
    /// Whether the bundle loads without the user's acceptance.
    #[must_use]
    pub fn is_trusted(&self) -> bool {
        matches!(self, Self::Trusted { .. })
    }
}

/// The trust of a manifest and its signature file. A signature that does
/// not verify, a revoked key, or a malformed file is an error: the pack is
/// refused, not quarantined.
///
/// # Errors
/// [`SignatureError`] but [`SignatureError::UnknownKey`], which is
/// [`Trust::UnknownKey`].
pub fn trust_of(
    manifest: &[u8],
    signature_json: Option<&[u8]>,
    trust: &TrustList,
) -> Result<Trust, SignatureError> {
    let Some(bytes) = signature_json else {
        return Ok(Trust::Unsigned);
    };
    let sig: Signature = serde_json::from_slice(bytes)
        .map_err(|e| SignatureError::Malformed(format!("signature file: {e}")))?;
    match signature::verify(manifest, &sig, trust) {
        Ok(key) => Ok(Trust::Trusted {
            key_id: key.id.clone(),
        }),
        Err(SignatureError::UnknownKey(key_id)) => Ok(Trust::UnknownKey { key_id }),
        Err(e) => Err(e),
    }
}

/// One entry of a pack's zip listing, as the archive declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its name.
    pub name: String,
    /// Its declared uncompressed size.
    pub size: u64,
    /// Whether it is a regular file (not a folder, link, or other kind).
    pub is_file: bool,
}

/// Why a pack is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackError {
    /// The manifest does not parse or lists a path that is not plain or is
    /// listed twice.
    #[error("bad manifest: {0}")]
    Manifest(String),
    /// An entry that is not a plain relative path to a regular file (zip
    /// slip: `..`, an absolute path, a drive, a backslash; a link; a
    /// folder).
    #[error("{0}: not a plain file path")]
    Path(String),
    /// An entry listed twice.
    #[error("{0}: listed twice")]
    Duplicate(String),
    /// An entry the manifest does not list.
    #[error("{0}: not in the manifest")]
    Unlisted(String),
    /// A manifest file the pack lacks.
    #[error("{0}: missing")]
    Missing(String),
    /// An entry whose declared size is not the manifest's.
    #[error("{name}: {size} bytes, the manifest says {expected}")]
    Size {
        /// The entry.
        name: String,
        /// Its declared size.
        size: u64,
        /// The manifest's.
        expected: u64,
    },
    /// Too much: more than [`MAX_UNPACKED`] bytes or [`MAX_FILES`] files
    /// (zip bomb).
    #[error("too large: {0}")]
    TooLarge(String),
    /// A file's bytes are not the manifest's (size or SHA-256), found while
    /// extracting.
    #[error("{0}: content does not match the manifest")]
    Content(String),
}

impl PackError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Manifest(_) => "PACK_MANIFEST",
            Self::Path(_) => "PACK_PATH",
            Self::Duplicate(_) => "PACK_DUPLICATE",
            Self::Unlisted(_) => "PACK_UNLISTED",
            Self::Missing(_) => "PACK_MISSING",
            Self::Size { .. } => "PACK_SIZE",
            Self::TooLarge(_) => "PACK_TOO_LARGE",
            Self::Content(_) => "PACK_CONTENT",
        }
    }
}

/// Whether a path names the same file on every file system the
/// application runs on: no segment ending in `.` or a space (Windows drops
/// them), and no Windows device name (`con`, `nul`, `com1`, ..., with or
/// without an extension).
fn portable(path: &str) -> bool {
    path.split('/').all(|seg| {
        let stem = seg.split('.').next().unwrap_or(seg).to_ascii_lowercase();
        let device = matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
            || ((stem.starts_with("com") || stem.starts_with("lpt"))
                && stem.len() == 4
                && stem.ends_with(|c: char| c.is_ascii_digit()));
        !seg.ends_with(['.', ' ']) && !device
    })
}

/// Parses a manifest and checks its file list: plain, portable paths,
/// unique even ignoring case, none named like the pack's own files in any
/// case, at most [`MAX_FILES`], at most [`MAX_UNPACKED`] bytes in all.
///
/// # Errors
/// [`PackError::Manifest`], [`PackError::TooLarge`].
pub fn manifest(bytes: &[u8]) -> Result<Manifest, PackError> {
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|e| PackError::Manifest(e.to_string()))?;
    if manifest.game().is_none() {
        return Err(PackError::Manifest(format!(
            "unknown game {}",
            manifest.game
        )));
    }
    if manifest.files.len() > MAX_FILES {
        return Err(PackError::TooLarge(format!(
            "{} files",
            manifest.files.len()
        )));
    }
    let mut seen = BTreeSet::new();
    let mut total: u64 = 0;
    for f in &manifest.files {
        let folded = f.path.to_ascii_lowercase();
        if !plain_path(&f.path)
            || !portable(&f.path)
            || folded == MANIFEST
            || SIDE_FILES.contains(&folded.as_str())
        {
            return Err(PackError::Manifest(format!(
                "{}: not a plain file path",
                f.path
            )));
        }
        // Case-insensitive file systems (Windows, macOS) would merge them.
        if !seen.insert(folded) {
            return Err(PackError::Manifest(format!("{}: listed twice", f.path)));
        }
        total = total.saturating_add(f.size);
    }
    if total > MAX_UNPACKED {
        return Err(PackError::TooLarge(format!("{total} bytes")));
    }
    Ok(manifest)
}

/// Checks a pack's listing against its manifest before anything is
/// extracted: every entry a regular file at a plain path, listed once,
/// either `bundle.json`, `signature.json`, or a manifest file of the
/// manifest's size; every manifest file present.
///
/// # Errors
/// The first [`PackError`] found.
pub fn check_listing(manifest: &Manifest, entries: &[Entry]) -> Result<(), PackError> {
    if entries.len() > MAX_FILES.saturating_add(2) {
        return Err(PackError::TooLarge(format!("{} entries", entries.len())));
    }
    let mut seen = BTreeSet::new();
    for e in entries {
        if !e.is_file || !plain_path(&e.name) {
            return Err(PackError::Path(e.name.clone()));
        }
        if !seen.insert(e.name.as_str()) {
            return Err(PackError::Duplicate(e.name.clone()));
        }
        let expected = if e.name == MANIFEST || e.name == signature::SIGNATURE_FILE {
            if e.size > MAX_SMALL_FILE {
                return Err(PackError::TooLarge(e.name.clone()));
            }
            continue;
        } else {
            manifest
                .files
                .iter()
                .find(|f| f.path == e.name)
                .ok_or_else(|| PackError::Unlisted(e.name.clone()))?
                .size
        };
        if e.size != expected {
            return Err(PackError::Size {
                name: e.name.clone(),
                size: e.size,
                expected,
            });
        }
    }
    if !seen.contains(MANIFEST) {
        return Err(PackError::Missing(MANIFEST.to_owned()));
    }
    if let Some(f) = manifest
        .files
        .iter()
        .find(|f| !seen.contains(f.path.as_str()))
    {
        return Err(PackError::Missing(f.path.clone()));
    }
    Ok(())
}

/// The data feed's format.
pub const FEED_FORMAT: u32 = 1;

/// `data-feed.json`: the packs the project publishes, signed with the data
/// key (detached, `data-feed.json.sig`, the [`Signature`] format).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feed {
    /// [`FEED_FORMAT`].
    pub format: u32,
    /// Grows with every published feed; a lower one than the last seen is
    /// refused (no feed rollback).
    pub sequence: u64,
    /// The packs, newest first.
    pub bundles: Vec<FeedBundle>,
}

/// One pack of the feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedBundle {
    /// The bundle's ID, `<game>/<steam build>-s<schema>`.
    pub id: String,
    /// Where the pack is (HTTPS).
    pub url: String,
    /// The pack file's SHA-256, lowercase hex.
    pub sha256: String,
    /// The pack file's size in bytes.
    pub size: u64,
}

/// Why a feed is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedError {
    /// Its signature does not verify (unknown or revoked key, changed).
    #[error("feed signature: {0}")]
    Signature(#[from] SignatureError),
    /// It does not parse, or is a format this build does not read.
    #[error("bad feed: {0}")]
    Malformed(String),
    /// Its sequence is lower than one seen before: a replayed old feed.
    #[error("feed sequence {got} is lower than {seen}, seen before")]
    Rollback {
        /// The highest seen before.
        seen: u64,
        /// This feed's.
        got: u64,
    },
}

impl FeedError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Signature(_) => "FEED_SIGNATURE",
            Self::Malformed(_) => "FEED_MALFORMED",
            Self::Rollback { .. } => "FEED_ROLLBACK",
        }
    }
}

/// Verifies a feed: its signature with a trusted key (an unknown key is
/// refused here: a feed is never quarantined), its format, its packs'
/// fields, and its sequence against the highest seen before.
///
/// # Errors
/// [`FeedError`].
pub fn verify_feed(
    feed: &[u8],
    signature_json: &[u8],
    trust: &TrustList,
    seen: Option<u64>,
) -> Result<Feed, FeedError> {
    let sig: Signature = serde_json::from_slice(signature_json)
        .map_err(|e| SignatureError::Malformed(format!("signature file: {e}")))?;
    signature::verify(feed, &sig, trust)?;
    let parsed: Feed =
        serde_json::from_slice(feed).map_err(|e| FeedError::Malformed(e.to_string()))?;
    if parsed.format != FEED_FORMAT {
        return Err(FeedError::Malformed(format!("format {}", parsed.format)));
    }
    for b in &parsed.bundles {
        let hex = b.sha256.len() == 64
            && b.sha256
                .bytes()
                .all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'));
        if !hex || !b.url.starts_with("https://") || b.size == 0 || b.size > MAX_UNPACKED {
            return Err(FeedError::Malformed(format!("bundle {}", b.id)));
        }
    }
    if let Some(seen) = seen
        && parsed.sequence < seen
    {
        return Err(FeedError::Rollback {
            seen,
            got: parsed.sequence,
        });
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::KeyPair;

    fn manifest_json(files: &[(&str, u64)]) -> Vec<u8> {
        let files: Vec<_> = files
            .iter()
            .map(|(p, s)| serde_json::json!({ "path": p, "sha256": "00", "size": s }))
            .collect();
        serde_json::to_vec(&serde_json::json!({
            "format": 2, "data_schema": 4, "game": "poe1",
            "source": { "steam_build": 1, "index_sha256": "", "extractor_commit": "", "decoder": "", "dat_schema": "" },
            "files": files, "signatures": []
        }))
        .unwrap_or_default()
    }

    fn entry(name: &str, size: u64) -> Entry {
        Entry {
            name: name.to_owned(),
            size,
            is_file: true,
        }
    }

    fn listing_error(entries: &[Entry]) -> Option<&'static str> {
        let m = manifest(&manifest_json(&[("data/a.json", 3), ("data/b.json", 5)])).ok()?;
        check_listing(&m, entries).err().map(|e| e.code())
    }

    fn good() -> Vec<Entry> {
        vec![
            entry("bundle.json", 400),
            entry("signature.json", 200),
            entry("data/a.json", 3),
            entry("data/b.json", 5),
        ]
    }

    #[test]
    fn a_listing_of_exactly_the_manifest_passes() {
        assert_eq!(listing_error(&good()), None);
        // The signature is optional (an unsigned pack is quarantined).
        let unsigned: Vec<Entry> = good()
            .into_iter()
            .filter(|e| e.name != "signature.json")
            .collect();
        assert_eq!(listing_error(&unsigned), None);
    }

    #[test]
    fn names_that_alias_on_some_file_system_are_refused() {
        for files in [
            vec![("Signature.JSON", 1)],
            vec![("ACCEPTED.json", 1)],
            vec![("Bundle.json", 1)],
            vec![("data/a.json.", 1)],
            vec![("data/a.json ", 1)],
            vec![("data/con.json", 1)],
            vec![("lpt1/a.json", 1)],
            vec![("data/a.json", 1), ("Data/A.json", 1)],
        ] {
            assert!(manifest(&manifest_json(&files)).is_err(), "{files:?}");
        }
        // Names that only look like those pass.
        assert!(
            manifest(&manifest_json(&[
                ("data/console.json", 1),
                ("com10/a.json", 1)
            ]))
            .is_ok()
        );
    }

    #[test]
    fn zip_slip_paths_links_and_folders_are_refused() {
        for name in [
            "../evil.json",
            "/etc/passwd",
            "C:/x.json",
            "data\\a.json",
            "data/./a.json",
            "data//a.json",
        ] {
            let mut e = good();
            e.push(entry(name, 1));
            assert_eq!(listing_error(&e), Some("PACK_PATH"), "{name}");
        }
        let mut link = good();
        if let Some(e) = link.get_mut(2) {
            e.is_file = false;
        }
        assert_eq!(listing_error(&link), Some("PACK_PATH"), "a link or folder");
    }

    #[test]
    fn extra_missing_duplicate_and_resized_entries_are_refused() {
        let mut extra = good();
        extra.push(entry("data/c.json", 1));
        assert_eq!(listing_error(&extra), Some("PACK_UNLISTED"));
        let missing: Vec<Entry> = good()
            .into_iter()
            .filter(|e| e.name != "data/b.json")
            .collect();
        assert_eq!(listing_error(&missing), Some("PACK_MISSING"));
        let no_manifest: Vec<Entry> = good()
            .into_iter()
            .filter(|e| e.name != "bundle.json")
            .collect();
        assert_eq!(listing_error(&no_manifest), Some("PACK_MISSING"));
        let mut dup = good();
        dup.push(entry("data/a.json", 3));
        assert_eq!(listing_error(&dup), Some("PACK_DUPLICATE"));
        let mut resized = good();
        if let Some(e) = resized.get_mut(3) {
            e.size = 5_000_000_000;
        }
        assert_eq!(
            listing_error(&resized),
            Some("PACK_SIZE"),
            "a bomb's declared size"
        );
    }

    #[test]
    fn a_manifest_over_the_caps_or_with_bad_paths_is_refused() {
        let big = manifest_json(&[("a", MAX_UNPACKED), ("b", 1)]);
        assert_eq!(
            manifest(&big).err().map(|e| e.code()),
            Some("PACK_TOO_LARGE")
        );
        for bad in [
            &[("../x", 1)][..],
            &[("a", 1), ("a", 1)],
            &[("bundle.json", 1)],
            &[("signature.json", 1)],
        ] {
            assert_eq!(
                manifest(&manifest_json(bad)).err().map(|e| e.code()),
                Some("PACK_MANIFEST")
            );
        }
        let mut huge_small = good();
        if let Some(e) = huge_small.get_mut(0) {
            e.size = MAX_SMALL_FILE + 1;
        }
        assert_eq!(listing_error(&huge_small), Some("PACK_TOO_LARGE"));
    }

    #[test]
    fn trust_is_trusted_unknown_unsigned_or_refused() {
        let m = manifest_json(&[]);
        let key = KeyPair::from_seed(&[1; 32]);
        let other = KeyPair::from_seed(&[2; 32]);
        let mut trust = TrustList {
            format: signature::TRUST_FORMAT,
            keys: vec![key.trusted("k")],
            revoked: Vec::new(),
        };
        let sig = serde_json::to_vec(&key.sign(&m)).unwrap_or_default();
        assert!(trust_of(&m, Some(&sig), &trust).is_ok_and(|t| t.is_trusted()));
        assert_eq!(trust_of(&m, None, &trust), Ok(Trust::Unsigned));
        let unknown = serde_json::to_vec(&other.sign(&m)).unwrap_or_default();
        assert!(matches!(
            trust_of(&m, Some(&unknown), &trust),
            Ok(Trust::UnknownKey { .. })
        ));
        // Changed after signing: refused, never quarantined.
        let mut changed = m.clone();
        changed.push(b' ');
        assert_eq!(
            trust_of(&changed, Some(&sig), &trust),
            Err(SignatureError::Invalid)
        );
        // Revoked: refused.
        trust.revoked.push(signature::Revocation {
            key_id: key.trusted("").id,
            reason: "test".to_owned(),
        });
        assert!(matches!(
            trust_of(&m, Some(&sig), &trust),
            Err(SignatureError::Revoked(_))
        ));
    }

    fn feed(sequence: u64) -> Vec<u8> {
        serde_json::to_vec(&Feed {
            format: FEED_FORMAT,
            sequence,
            bundles: vec![FeedBundle {
                id: "poe1/1-s4".to_owned(),
                url: "https://example.invalid/a.loombundle".to_owned(),
                sha256: "a".repeat(64),
                size: 10,
            }],
        })
        .unwrap_or_default()
    }

    #[test]
    fn a_feed_verifies_and_a_lower_sequence_is_a_rollback() {
        let key = KeyPair::from_seed(&[3; 32]);
        let trust = TrustList {
            format: signature::TRUST_FORMAT,
            keys: vec![key.trusted("k")],
            revoked: Vec::new(),
        };
        let sign = |f: &[u8]| serde_json::to_vec(&key.sign(f)).unwrap_or_default();
        let f5 = feed(5);
        assert!(verify_feed(&f5, &sign(&f5), &trust, None).is_ok());
        assert!(
            verify_feed(&f5, &sign(&f5), &trust, Some(5)).is_ok(),
            "the same feed again"
        );
        let f4 = feed(4);
        assert_eq!(
            verify_feed(&f4, &sign(&f4), &trust, Some(5))
                .err()
                .map(|e| e.code()),
            Some("FEED_ROLLBACK")
        );
        assert_eq!(
            verify_feed(&f5, &sign(&f4), &trust, None)
                .err()
                .map(|e| e.code()),
            Some("FEED_SIGNATURE")
        );
        let unknown =
            serde_json::to_vec(&KeyPair::from_seed(&[4; 32]).sign(&f5)).unwrap_or_default();
        assert_eq!(
            verify_feed(&f5, &unknown, &trust, None)
                .err()
                .map(|e| e.code()),
            Some("FEED_SIGNATURE"),
            "a feed is never quarantined"
        );
    }
}

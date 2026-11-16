//! Each installed folder's status (moved from `loom-app::install`): its
//! signature against the trust list, its files' sizes and, on request,
//! hashes against the manifest. A bundle signed by a trusted key loads in
//! the app; an unsigned one, or one signed by a key this build does not
//! know, is quarantined until the user accepts it; a bad signature, a
//! revoked key, or damaged files quarantine it for good.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use loom_data::bundle::{MANIFEST, Manifest};
use loom_data::pack::{self, PackError, Trust};
use loom_data::signature::{SIGNATURE_FILE, SignatureError, TrustList, hex};
use loom_domain::DataVersion;
use sha2::{Digest, Sha256};

/// The folder, under the application's data folder, that holds installed
/// bundles.
pub const BUNDLES_DIR: &str = "bundles";

/// The ID of a data version: `<game>/<steam build>-s<schema>` (`unknown`
/// for a bundle without a Steam build).
#[must_use]
pub fn bundle_id(version: &DataVersion) -> String {
    let build = version
        .steam_build
        .map_or_else(|| "unknown".to_owned(), |b| b.to_string());
    format!(
        "{}/{build}-s{}",
        version.game.name(),
        version.schema_version
    )
}

/// The file in a bundle folder that records the user's acceptance of a
/// quarantined bundle: the manifest SHA-256 they accepted.
pub const ACCEPTED_FILE: &str = "accepted.json";

/// Why an installed bundle does not load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quarantine {
    /// No signature (the user may accept it).
    Unsigned,
    /// Signed by a key this build does not know (the user may accept it).
    UnknownKey,
    /// The signature does not verify: changed after signing.
    BadSignature,
    /// Signed by a revoked key.
    Revoked,
    /// A file is missing or not its manifest size (or hash, when checked).
    Damaged,
}

impl Quarantine {
    /// Its name in the API (`unsigned`, `unknown-key`, ...).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unsigned => "unsigned",
            Self::UnknownKey => "unknown-key",
            Self::BadSignature => "bad-signature",
            Self::Revoked => "revoked",
            Self::Damaged => "damaged",
        }
    }

    /// Whether the user may accept a bundle quarantined for this reason.
    #[must_use]
    pub const fn acceptable(self) -> bool {
        matches!(self, Self::Unsigned | Self::UnknownKey)
    }
}

/// Whether an installed bundle loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Signed by a trusted key.
    Trusted {
        /// The key's ID.
        key_id: String,
    },
    /// Quarantined, then accepted by the user.
    Accepted(Quarantine),
    /// Not loaded.
    Quarantined(Quarantine),
}

impl Status {
    /// Whether it loads.
    #[must_use]
    pub fn loads(&self) -> bool {
        !matches!(self, Self::Quarantined(_))
    }
}

/// An installed bundle folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// `<game>/<steam build>-s<schema>`.
    pub id: String,
    /// Its folder.
    pub dir: PathBuf,
    /// Its data version.
    pub version: DataVersion,
    /// Whether it loads, and why not.
    pub status: Status,
    /// The SHA-256 of its `bundle.json`, lowercase hex (shown when the
    /// user is asked to accept it).
    pub manifest_sha256: String,
    /// The signing key's ID, when signed.
    pub key_id: Option<String>,
    /// Its files and their total size.
    pub files: usize,
    /// Bytes.
    pub bytes: u64,
}

/// SHA-256 of `bytes`, lowercase hex.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Reads at most `cap` bytes of a file; more is an error.
///
/// # Errors
/// The file does not read, or holds more than `cap` bytes.
pub fn read_capped(path: &Path, cap: u64) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    File::open(path)?
        .take(cap.saturating_add(1))
        .read_to_end(&mut out)?;
    if out.len() as u64 > cap {
        return Err(std::io::Error::other("too large"));
    }
    Ok(out)
}

/// A manifest's data version, when its game is known.
#[must_use]
pub fn version_of(manifest: &Manifest) -> Option<DataVersion> {
    Some(DataVersion {
        game: manifest.game()?,
        schema_version: manifest.data_schema,
        steam_build: manifest.source.steam_build,
    })
}

/// The record in [`ACCEPTED_FILE`]: the manifest SHA-256 the user accepted.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AcceptedRecord {
    /// The accepted manifest's SHA-256, lowercase hex.
    pub manifest_sha256: String,
}

/// The status of one bundle folder, if it is one (its manifest reads).
#[must_use]
pub fn status_of(dir: &Path, trust: &TrustList) -> Option<Found> {
    let manifest_bytes = read_capped(&dir.join(MANIFEST), pack::MAX_SMALL_FILE).ok()?;
    let manifest = pack::manifest(&manifest_bytes).ok()?;
    let version = version_of(&manifest)?;
    let signature = read_capped(&dir.join(SIGNATURE_FILE), pack::MAX_SMALL_FILE).ok();
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let (mut quarantine, key_id) =
        match pack::trust_of(&manifest_bytes, signature.as_deref(), trust) {
            Ok(Trust::Trusted { key_id }) => (None, Some(key_id)),
            Ok(Trust::UnknownKey { key_id }) => (Some(Quarantine::UnknownKey), Some(key_id)),
            Ok(Trust::Unsigned) => (Some(Quarantine::Unsigned), None),
            Err(SignatureError::Revoked(key_id)) => (Some(Quarantine::Revoked), Some(key_id)),
            Err(_) => (Some(Quarantine::BadSignature), None),
        };
    let sizes_ok = manifest.files.iter().all(|f| {
        std::fs::symlink_metadata(dir.join(&f.path))
            .is_ok_and(|m| m.file_type().is_file() && m.len() == f.size)
    });
    if !sizes_ok {
        quarantine = Some(Quarantine::Damaged);
    }
    let status = match quarantine {
        None => Status::Trusted {
            key_id: key_id.clone().unwrap_or_default(),
        },
        Some(q) => {
            let accepted = q.acceptable()
                && read_capped(&dir.join(ACCEPTED_FILE), 4096)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<AcceptedRecord>(&b).ok())
                    .is_some_and(|a| a.manifest_sha256 == manifest_sha256);
            if accepted {
                Status::Accepted(q)
            } else {
                Status::Quarantined(q)
            }
        }
    };
    Some(Found {
        id: bundle_id(&version),
        dir: dir.to_owned(),
        version,
        status,
        manifest_sha256,
        key_id,
        files: manifest.files.len(),
        bytes: manifest.files.iter().map(|f| f.size).sum(),
    })
}

/// Every bundle folder under `<data_dir>/bundles/*/*/` (temporary folders,
/// whose names start with `.`, skipped), with its status; newest first
/// (highest Steam build, then schema, then ID). Two folders with the same
/// ID keep the first by path. Read only: `loom-app`'s `install::scan` also
/// sweeps temporary folders a crashed install left.
#[must_use]
pub fn scan(data_dir: &Path, trust: &TrustList) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    let Ok(games) = std::fs::read_dir(data_dir.join(BUNDLES_DIR)) else {
        return out;
    };
    let hidden = |e: &std::fs::DirEntry| e.file_name().to_string_lossy().starts_with('.');
    let mut dirs: Vec<PathBuf> = games
        .flatten()
        .filter(|g| !hidden(g) && g.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|g| std::fs::read_dir(g.path()).ok())
        .flat_map(|d| d.flatten())
        .filter(|b| !hidden(b) && b.file_type().is_ok_and(|t| t.is_dir()))
        .map(|b| b.path())
        .collect();
    dirs.sort();
    for dir in dirs {
        if let Some(found) = status_of(&dir, trust)
            && out.iter().all(|f| f.id != found.id)
        {
            out.push(found);
        }
    }
    out.sort_by(|a, b| {
        (b.version.steam_build, b.version.schema_version)
            .cmp(&(a.version.steam_build, a.version.schema_version))
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// Why [`verify_full`] failed.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// `bundle.json` does not read.
    #[error("{path}: {source}")]
    Read {
        /// The folder.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// `bundle.json` is not a manifest a pack may carry.
    #[error(transparent)]
    Manifest(#[from] PackError),
    /// A file is missing or does not match its manifest size and SHA-256.
    #[error("{0}: does not match the manifest")]
    Damaged(String),
}

/// Checks every file's SHA-256 against the manifest (the user's "verify
/// data" action in the app; the knowledge MCP at startup).
///
/// # Errors
/// [`VerifyError`], naming the first file that does not match.
pub fn verify_full(dir: &Path) -> Result<(), VerifyError> {
    let manifest_bytes =
        read_capped(&dir.join(MANIFEST), pack::MAX_SMALL_FILE).map_err(|source| {
            VerifyError::Read {
                path: dir.to_owned(),
                source,
            }
        })?;
    let manifest = pack::manifest(&manifest_bytes)?;
    for f in &manifest.files {
        let path = dir.join(&f.path);
        let ok = File::open(&path).is_ok_and(|mut file| {
            let mut hasher = Sha256::new();
            std::io::copy(&mut file, &mut hasher).is_ok_and(|n| n == f.size)
                && hex(&hasher.finalize()) == f.sha256
        });
        if !ok {
            return Err(VerifyError::Damaged(f.path.clone()));
        }
    }
    Ok(())
}

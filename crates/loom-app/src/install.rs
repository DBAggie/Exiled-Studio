//! Installing and checking data bundles (Phase 5, P4; plan decision P-e;
//! `docs/phase-5/data-bundles.md`).
//!
//! **Install** ([`install_pack`]): a pack (`.loombundle`) is hostile input.
//! `bundle.json` and `signature.json` are read from the archive first and
//! the pack's trust decided (`loom_data::pack::trust_of`): a bad signature
//! or a revoked key refuses it. The archive's listing is checked against
//! the manifest (`check_listing`: zip slip, links, extra, missing, and
//! resized entries, the size caps) before anything is extracted. Each file
//! is then extracted into a temporary folder beside the target, its size
//! and SHA-256 checked as it is written; any mismatch removes the folder.
//! Last, one rename moves it to `bundles/<game>/<steam build>-s<schema>/`.
//! An ID already installed: the same manifest with intact files is a
//! no-op (the signature rewritten only when not weaker); damaged files are
//! extracted again; another manifest is replaced only by a pack signed by a
//! trusted key (extracted first, then swapped, the old folder kept until
//! the new one is in place).
//!
//! **Status** ([`scan`]): each installed folder's signature is checked
//! against the trust list and its files' sizes against the manifest at
//! every start; [`verify_full`] checks hashes (at install, on the user's
//! Verify; the bundle that loads also goes through the bundle validator,
//! which hashes every file). A bundle signed by a trusted key loads. An unsigned
//! one, or one signed by a key this build does not know, is
//! **quarantined**: stored, listed, not loaded, until the user accepts it
//! with its manifest's SHA-256 shown ([`accept`]). A bad signature, a
//! revoked key, or damaged files quarantine it for good.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use loom_data::bundle::{MANIFEST, Manifest};
use loom_data::pack::{self, Entry, PackError, Trust};
use loom_data::signature::{SIGNATURE_FILE, SignatureError, TrustList, hex};
use loom_domain::DataVersion;
use sha2::{Digest, Sha256};

use crate::bundles::{BUNDLES_DIR, bundle_id};
use crate::error::{AppError, Code};

pub use loom_data::pack::PACK_EXTENSION;

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

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Reads at most `cap` bytes of a file; more is an error.
fn read_capped(path: &Path, cap: u64) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    File::open(path)?
        .take(cap.saturating_add(1))
        .read_to_end(&mut out)?;
    if out.len() as u64 > cap {
        return Err(std::io::Error::other("too large"));
    }
    Ok(out)
}

fn version_of(manifest: &Manifest) -> Option<DataVersion> {
    Some(DataVersion {
        game: manifest.game()?,
        schema_version: manifest.data_schema,
        steam_build: manifest.source.steam_build,
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Accepted {
    manifest_sha256: String,
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
                    .and_then(|b| serde_json::from_slice::<Accepted>(&b).ok())
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
/// ID keep the first by path.
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
    for game in dirs.iter().filter_map(|d| d.parent()) {
        sweep_stale(game);
    }
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

fn pack_error(e: &PackError) -> AppError {
    AppError::layer(e.code(), e)
}

fn io(path: &Path, e: &std::io::Error) -> AppError {
    AppError::new(Code::Read, format!("{}: {e}", path.display()))
}

/// Reads one small entry (`bundle.json`, `signature.json`) whole, capped.
fn small_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<Option<Vec<u8>>, AppError> {
    let mut file = match archive.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(AppError::layer("PACK_ZIP", e)),
    };
    if file.size() > pack::MAX_SMALL_FILE {
        return Err(pack_error(&PackError::TooLarge(name.to_owned())));
    }
    let mut out = Vec::new();
    (&mut file)
        .take(pack::MAX_SMALL_FILE + 1)
        .read_to_end(&mut out)
        .map_err(|e| AppError::layer("PACK_ZIP", e))?;
    if out.len() as u64 > pack::MAX_SMALL_FILE {
        return Err(pack_error(&PackError::TooLarge(name.to_owned())));
    }
    Ok(Some(out))
}

/// The name prefix of a pack's temporary extraction folder.
const INCOMING: &str = ".incoming-";
/// The name prefix of a replaced bundle folder before it is removed.
const OUTGOING: &str = ".outgoing-";

/// Removes temporary folders another process left in a game's bundles
/// folder (a crash or kill mid-install). This process's own are left: one
/// may be in use.
fn sweep_stale(game_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(game_dir) else {
        return;
    };
    let own = format!("-{}-", std::process::id());
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = name
            .strip_prefix(INCOMING)
            .or_else(|| name.strip_prefix(OUTGOING))
        else {
            continue;
        };
        if !format!("-{rest}").starts_with(&own) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Removes a folder when dropped, unless kept.
struct TempFolder(Option<PathBuf>);

impl Drop for TempFolder {
    fn drop(&mut self) {
        if let Some(dir) = self.0.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// What installing a pack did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The bundle, with its status (quarantined when unsigned or signed by
    /// an unknown key).
    pub found: Found,
    /// Whether it was installed before, with the same manifest (nothing
    /// was written).
    pub already: bool,
}

/// Installs a pack (the module's steps).
///
/// # Errors
/// `APP_READ` (the file), `PACK_ZIP` (not a zip), `PACK_SIGNATURE` (a bad
/// signature, a revoked key, a malformed signature file), the `PACK_*`
/// codes of `loom_data::pack`, `APP_PACK_EXISTS` (the ID is installed with
/// another manifest).
pub fn install_pack(
    data_dir: &Path,
    pack_path: &Path,
    trust: &TrustList,
) -> Result<Installed, AppError> {
    let file = File::open(pack_path).map_err(|e| io(pack_path, &e))?;
    install_from(data_dir, std::io::BufReader::new(file), trust)
}

/// Installs a pack the application ships, at start (plan decision P-e):
/// [`install_pack`], but a pack whose exact manifest is installed already,
/// signed by a trusted key with every file at its listed size, is left as
/// it is without hashing its files. That is the start's own check
/// ([`status_of`]); **Verify** hashes them. Anything else, a first install,
/// another manifest, damage the sizes show, or an installed copy that is
/// not trusted, goes through [`install_pack`].
///
/// # Errors
/// As [`install_pack`].
pub fn install_shipped(
    data_dir: &Path,
    pack_path: &Path,
    trust: &TrustList,
) -> Result<Installed, AppError> {
    if let Some(found) = shipped_already(data_dir, pack_path, trust) {
        return Ok(Installed {
            found,
            already: true,
        });
    }
    install_pack(data_dir, pack_path, trust)
}

/// The installed bundle a shipped pack names, when [`install_shipped`] can
/// leave it as it is.
fn shipped_already(data_dir: &Path, pack_path: &Path, trust: &TrustList) -> Option<Found> {
    let file = File::open(pack_path).ok()?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file)).ok()?;
    let manifest_bytes = small_entry(&mut archive, MANIFEST).ok()??;
    let version = version_of(&pack::manifest(&manifest_bytes).ok()?)?;
    let target = data_dir.join(BUNDLES_DIR).join(bundle_id(&version));
    let installed = read_capped(&target.join(MANIFEST), pack::MAX_SMALL_FILE).ok()?;
    if installed != manifest_bytes {
        return None;
    }
    status_of(&target, trust).filter(|f| matches!(f.status, Status::Trusted { .. }))
}

/// Installs a pack read from `pack`: [`install_pack`] without the file
/// (the fuzz target and the properties give bytes).
///
/// # Errors
/// As [`install_pack`], without `APP_READ` for the pack.
pub fn install_from<R: Read + std::io::Seek>(
    data_dir: &Path,
    pack: R,
    trust: &TrustList,
) -> Result<Installed, AppError> {
    let mut archive = zip::ZipArchive::new(pack).map_err(|e| AppError::layer("PACK_ZIP", e))?;
    let manifest_bytes = small_entry(&mut archive, MANIFEST)?
        .ok_or_else(|| pack_error(&PackError::Missing(MANIFEST.to_owned())))?;
    let signature = small_entry(&mut archive, SIGNATURE_FILE)?;
    // The signature first: a changed or revoked pack is refused before its
    // listing is even read.
    let new_trust = pack::trust_of(&manifest_bytes, signature.as_deref(), trust)
        .map_err(|e| AppError::layer("PACK_SIGNATURE", e))?;
    let manifest = pack::manifest(&manifest_bytes).map_err(|e| pack_error(&e))?;
    let version = version_of(&manifest).ok_or_else(|| {
        pack_error(&PackError::Manifest(format!(
            "unknown game {}",
            manifest.game
        )))
    })?;
    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let f = archive
            .by_index_raw(i)
            .map_err(|e| AppError::layer("PACK_ZIP", e))?;
        entries.push(Entry {
            name: f
                .name()
                .map_err(|e| AppError::layer("PACK_ZIP", e))?
                .into_owned(),
            size: f.size(),
            is_file: f.is_file() && !f.is_symlink(),
        });
    }
    pack::check_listing(&manifest, &entries).map_err(|e| pack_error(&e))?;

    let id = bundle_id(&version);
    let target = data_dir.join(BUNDLES_DIR).join(&id);
    // The signature the folder gets: the pack's, unless the installed one
    // is trusted and the pack's is not (never weaker).
    let mut written_signature = signature.clone();
    let replacing = target.exists();
    if replacing {
        let existing_manifest = read_capped(&target.join(MANIFEST), pack::MAX_SMALL_FILE).ok();
        let existing_signature =
            read_capped(&target.join(SIGNATURE_FILE), pack::MAX_SMALL_FILE).ok();
        let same = existing_manifest.as_deref() == Some(manifest_bytes.as_slice());
        // Whether the installed signature verifies with a trusted key
        // (whatever the files' state: a damaged bundle keeps it).
        let existing_trusted = existing_manifest.as_deref().is_some_and(|m| {
            pack::trust_of(m, existing_signature.as_deref(), trust).is_ok_and(|t| t.is_trusted())
        });
        if same && existing_trusted && !new_trust.is_trusted() {
            written_signature = existing_signature;
        }
        if same && verify_full(&target).is_ok() {
            if let Some(sig) = &signature
                && (new_trust.is_trusted() || !existing_trusted)
            {
                std::fs::write(target.join(SIGNATURE_FILE), sig).map_err(|e| io(&target, &e))?;
            }
            let found = status_of(&target, trust)
                .ok_or_else(|| AppError::new(Code::BundleDamaged, format!("{id}: manifest")))?;
            return Ok(Installed {
                found,
                already: true,
            });
        }
        // Damaged files of the same manifest are extracted again; other
        // content only from a trusted key.
        if !same && !new_trust.is_trusted() {
            return Err(AppError::new(
                Code::PackExists,
                format!("{id} is installed with other content"),
            ));
        }
    }
    let parent = target
        .parent()
        .ok_or_else(|| AppError::new(Code::Read, "no bundles folder"))?;
    std::fs::create_dir_all(parent).map_err(|e| io(parent, &e))?;
    sweep_stale(parent);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let temp = parent.join(format!("{INCOMING}{}-{nonce}", std::process::id()));
    std::fs::create_dir(&temp).map_err(|e| io(&temp, &e))?;
    let mut guard = TempFolder(Some(temp.clone()));
    for f in &manifest.files {
        let path = temp.join(&f.path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| io(dir, &e))?;
        }
        let mut entry = archive
            .by_name(&f.path)
            .map_err(|e| AppError::layer("PACK_ZIP", e))?;
        let mut out = File::create_new(&path).map_err(|e| io(&path, &e))?;
        let mut hasher = Sha256::new();
        let mut written: u64 = 0;
        let mut buf = vec![0u8; 64 * 1024];
        // Never more than the manifest's size plus one byte, whatever the
        // archive claims (zip bomb).
        let mut limited = (&mut entry).take(f.size.saturating_add(1));
        loop {
            let n = limited
                .read(&mut buf)
                .map_err(|e| AppError::layer("PACK_ZIP", e))?;
            if n == 0 {
                break;
            }
            let chunk = buf.get(..n).unwrap_or_default();
            hasher.update(chunk);
            out.write_all(chunk).map_err(|e| io(&path, &e))?;
            written += n as u64;
        }
        if written != f.size || hex(&hasher.finalize()) != f.sha256 {
            return Err(pack_error(&PackError::Content(f.path.clone())));
        }
    }
    std::fs::write(temp.join(MANIFEST), &manifest_bytes).map_err(|e| io(&temp, &e))?;
    if let Some(sig) = &written_signature {
        std::fs::write(temp.join(SIGNATURE_FILE), sig).map_err(|e| io(&temp, &e))?;
    }
    if replacing {
        // The old folder aside, the new one in, then the old one removed;
        // if the new one cannot move in, the old one goes back.
        let old = parent.join(format!("{OUTGOING}{}-{nonce}", std::process::id()));
        std::fs::rename(&target, &old).map_err(|e| io(&target, &e))?;
        if let Err(e) = std::fs::rename(&temp, &target) {
            let _ = std::fs::rename(&old, &target);
            return Err(io(&target, &e));
        }
        let _ = std::fs::remove_dir_all(&old);
    } else {
        std::fs::rename(&temp, &target).map_err(|e| io(&target, &e))?;
    }
    guard.0 = None;
    let found = status_of(&target, trust)
        .ok_or_else(|| AppError::new(Code::BundleDamaged, format!("{id}: manifest")))?;
    tracing::info!(id = %found.id, loads = found.status.loads(), "bundle installed");
    Ok(Installed {
        found,
        already: false,
    })
}

/// Checks every file's SHA-256 against the manifest (the user's "verify
/// data" action; the start checks sizes only).
///
/// # Errors
/// `APP_BUNDLE_DAMAGED` naming the first file that does not match.
pub fn verify_full(dir: &Path) -> Result<(), AppError> {
    let manifest_bytes =
        read_capped(&dir.join(MANIFEST), pack::MAX_SMALL_FILE).map_err(|e| io(dir, &e))?;
    let manifest = pack::manifest(&manifest_bytes).map_err(|e| pack_error(&e))?;
    for f in &manifest.files {
        let path = dir.join(&f.path);
        let ok = File::open(&path).is_ok_and(|mut file| {
            let mut hasher = Sha256::new();
            std::io::copy(&mut file, &mut hasher).is_ok_and(|n| n == f.size)
                && hex(&hasher.finalize()) == f.sha256
        });
        if !ok {
            return Err(AppError::new(
                Code::BundleDamaged,
                format!("{}: does not match the manifest", f.path),
            ));
        }
    }
    Ok(())
}

/// Accepts a quarantined bundle: the user saw `manifest_sha256` and
/// approved it. Refused when it differs from the folder's (the bundle
/// changed since it was shown) or when the reason is not acceptable.
///
/// # Errors
/// `APP_QUARANTINED`, `APP_READ`.
pub fn accept(found: &Found, manifest_sha256: &str) -> Result<(), AppError> {
    let Status::Quarantined(q) = found.status else {
        return Ok(());
    };
    if !q.acceptable() {
        return Err(AppError::new(
            Code::Quarantined,
            format!("{}: {} bundles cannot be accepted", found.id, q.name()),
        ));
    }
    if manifest_sha256 != found.manifest_sha256 {
        return Err(AppError::new(
            Code::Quarantined,
            format!("{}: the bundle changed since it was shown", found.id),
        ));
    }
    let text = serde_json::to_vec_pretty(&Accepted {
        manifest_sha256: manifest_sha256.to_owned(),
    })
    .unwrap_or_default();
    std::fs::write(found.dir.join(ACCEPTED_FILE), text).map_err(|e| io(&found.dir, &e))
}

/// Writes a pack of the bundle folder `dir`: `bundle.json`, then
/// `signature_json` as `signature.json` when given, then every manifest
/// file, deflated (the release tool and tests; the application only
/// installs).
///
/// # Errors
/// `APP_READ` for a file that does not read or write; the `PACK_*` codes
/// for a manifest that does not pass [`pack::manifest`].
pub fn write_pack(dir: &Path, signature_json: Option<&[u8]>, out: &Path) -> Result<(), AppError> {
    let manifest_bytes = std::fs::read(dir.join(MANIFEST)).map_err(|e| io(dir, &e))?;
    let manifest = pack::manifest(&manifest_bytes).map_err(|e| pack_error(&e))?;
    let file = File::create(out).map_err(|e| io(out, &e))?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(false);
    let zip_err = |e: zip::result::ZipError| AppError::layer("PACK_ZIP", e);
    zip.start_file(MANIFEST, options).map_err(zip_err)?;
    zip.write_all(&manifest_bytes).map_err(|e| io(out, &e))?;
    if let Some(sig) = signature_json {
        zip.start_file(SIGNATURE_FILE, options).map_err(zip_err)?;
        zip.write_all(sig).map_err(|e| io(out, &e))?;
    }
    for f in &manifest.files {
        let path = dir.join(&f.path);
        let mut src = File::open(&path).map_err(|e| io(&path, &e))?;
        zip.start_file(f.path.as_str(), options).map_err(zip_err)?;
        std::io::copy(&mut src, &mut zip).map_err(|e| io(&path, &e))?;
    }
    zip.finish().map_err(zip_err)?;
    Ok(())
}

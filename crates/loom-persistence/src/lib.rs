//! File-first build storage (ADR-009; Phase 2 plan, decision 13). Host
//! layer: filesystem access is allowed here and injected into the core
//! from this crate. [`recovery`] holds the autosave snapshots and
//! [`recent`] the recent builds list (Phase 4 plan, decision 8); the SQLite
//! index waits for search.
//!
//! - [`Store::load`] reads a build document of any version the migrations
//!   reach ([`loom_serialization::dto::read_with`]) and returns the
//!   migration report. Loading never writes: a migrated build reaches disk
//!   only when it is saved.
//! - [`Store::save`] writes the current document to a temporary file in
//!   the target's folder, flushes it, then renames it over the target, so
//!   the target is either the old file or the whole new one. It never
//!   writes over the file a build was imported from, and replaces an
//!   existing file only when asked to and only when that file is a build
//!   document this program can read (ADR-009: originals are not silently
//!   overwritten).
//!
//! The file operations go through [`FileOps`], so tests can make any of
//! them fail ([`StdFiles`] is the real one).
//!
//! **Errors (plan decision 15).** [`Error::code`] is the code of the layer
//! that failed: `PERSIST_*` for files ([`PersistError`]), `MIGRATE_*` and
//! `DTO_*` from `loom-serialization` unchanged. No layer wraps or renames
//! another's code.

pub mod recent;
pub mod recovery;

use std::ffi::{OsStr, OsString};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use loom_domain::Build;
use loom_serialization::dto::{self, Loaded, ReadError, v1::DtoError};
use loom_serialization::migrate::{self, MigrateError, Migrations};

/// The file operations a [`Store`] makes.
pub trait FileOps {
    /// The file's bytes.
    ///
    /// # Errors
    /// The I/O error.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Whether something exists at `path` (a link is not followed).
    ///
    /// # Errors
    /// The I/O error when it cannot be told.
    fn exists(&self, path: &Path) -> io::Result<bool>;

    /// Creates the file at `path`, which must not exist, writes `bytes`,
    /// and flushes them to the device.
    ///
    /// # Errors
    /// [`io::ErrorKind::AlreadyExists`] when `path` exists; the I/O error
    /// otherwise. The file may exist after an error.
    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    /// Renames `from` to `to`, replacing `to`.
    ///
    /// # Errors
    /// The I/O error.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Makes `to` name the file `from` names, failing if `to` exists, in
    /// one step (a hard link): what publishes a save to a new target.
    /// `from` is left in place.
    ///
    /// # Errors
    /// [`io::ErrorKind::AlreadyExists`] when `to` exists; another I/O
    /// error when it cannot be done (a file system without hard links,
    /// say), after which [`Store::save`] falls back to checking and
    /// renaming.
    fn link_new(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes the file at `path`.
    ///
    /// # Errors
    /// The I/O error.
    fn remove(&self, path: &Path) -> io::Result<()>;
}

impl<T: FileOps + ?Sized> FileOps for &T {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        (**self).read(path)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        (**self).exists(path)
    }

    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        (**self).write_new(path, bytes)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        (**self).rename(from, to)
    }

    fn link_new(&self, from: &Path, to: &Path) -> io::Result<()> {
        (**self).link_new(from, to)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        (**self).remove(path)
    }
}

/// The file operations of `std::fs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFiles;

impl FileOps for StdFiles {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)?;
        sync_dir(to);
        Ok(())
    }

    /// Hard links differ by file system (measured 2026-11-11 unless noted):
    /// - NTFS (Windows), ext4 (Linux), and WSL 2's `/mnt/<drive>` (9p over
    ///   NTFS) have them; linking onto an existing name fails with
    ///   `AlreadyExists` on each, which is what makes a new-target save
    ///   atomic;
    /// - FAT32 and exFAT have none, and network shares depend on the
    ///   server (not measured): the error kind there is not one we can
    ///   rely on, so [`Store::save`] treats every error but
    ///   `AlreadyExists` as "no hard links" and falls back to a second
    ///   check and a rename, where a new-target save is not atomic.
    fn link_new(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::hard_link(from, to)?;
        sync_dir(to);
        Ok(())
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }
}

/// Makes a new name in `path`'s folder durable where the folder can be
/// flushed (Unix; best effort: the change has happened either way). On
/// Windows the folder is not flushed.
fn sync_dir(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        let dir = if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        };
        if let Ok(dir) = std::fs::File::open(dir) {
            let _ = dir.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Whether [`Store::save`] may replace a file that exists at the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overwrite {
    /// Never: an existing target gives `PERSIST_EXISTS`. Atomic where the
    /// file system has hard links: a target that appears while the save
    /// runs is kept and gives `PERSIST_EXISTS` too.
    #[default]
    Never,
    /// Only a build document this program reads in full, such as the file
    /// the build was loaded from: one that is no build document gives
    /// `PERSIST_NOT_A_BUILD`, and one that does not read (damaged, or a
    /// version this program does not read) gives its own `DTO_*` or
    /// `MIGRATE_*` code, since writing over it would lose what this program
    /// cannot read.
    ///
    /// The check and the rename are two steps; another process changing
    /// the target between them is not guarded (the app's single-writer
    /// build task owns saves). A link at the target is replaced by the
    /// file, not followed. The new file has the folder's default
    /// permissions, not the old file's.
    BuildDocument,
}

/// How [`Store::save`] writes.
#[derive(Debug, Clone, Copy, Default)]
pub struct SaveOptions<'a> {
    /// Whether an existing target may be replaced.
    pub overwrite: Overwrite,
    /// The file the build was imported from (a PoB XML file, say), which
    /// is never written: a target naming it gives `PERSIST_IMPORT_SOURCE`
    /// whatever `overwrite` says. Paths are compared after resolving
    /// links, `.` and `..`, and letter case where the file system ignores
    /// it; another hard link to the same file is not detected.
    pub imported_from: Option<&'a Path>,
}

/// A file-level failure (`PERSIST_*`).
#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    /// The file could not be read.
    #[error("PERSIST_READ: {path}: {source}", path = path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// The file is not UTF-8 text without a byte order mark (what
    /// [`Store::save`] writes).
    #[error("PERSIST_ENCODING: {path}: {reason}", path = path.display())]
    Encoding {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        reason: &'static str,
    },
    /// The target names no file in a folder.
    #[error("PERSIST_PATH: {path}: {reason}", path = path.display())]
    Path {
        /// The target.
        path: PathBuf,
        /// What is wrong.
        reason: &'static str,
    },
    /// The target is the file the build was imported from.
    #[error("PERSIST_IMPORT_SOURCE: {path} is the file the build was imported from", path = path.display())]
    ImportSource {
        /// The target.
        path: PathBuf,
    },
    /// The target exists and [`Overwrite::Never`] was asked for.
    #[error("PERSIST_EXISTS: {path}", path = path.display())]
    Exists {
        /// The target.
        path: PathBuf,
    },
    /// The target exists and is not a build document.
    #[error("PERSIST_NOT_A_BUILD: {path} is not a build document", path = path.display())]
    NotABuild {
        /// The target.
        path: PathBuf,
    },
    /// The temporary file could not be written. The target is unchanged.
    #[error("PERSIST_WRITE: {path}: {source}{}", leftover_note(*leftover), path = path.display())]
    Write {
        /// The temporary file.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
        /// Whether the temporary file is still there (removing it failed).
        leftover: bool,
    },
    /// The temporary file could not be renamed over the target. The target
    /// is unchanged.
    #[error("PERSIST_RENAME: {} to {}: {source}{}", from.display(), to.display(), leftover_note(*leftover))]
    Rename {
        /// The temporary file.
        from: PathBuf,
        /// The target.
        to: PathBuf,
        /// The I/O error.
        source: io::Error,
        /// Whether the temporary file is still there (removing it failed).
        leftover: bool,
    },
}

fn leftover_note(leftover: bool) -> &'static str {
    if leftover {
        " (the temporary file could not be removed)"
    } else {
        ""
    }
}

impl PersistError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Read { .. } => "PERSIST_READ",
            Self::Encoding { .. } => "PERSIST_ENCODING",
            Self::Path { .. } => "PERSIST_PATH",
            Self::ImportSource { .. } => "PERSIST_IMPORT_SOURCE",
            Self::Exists { .. } => "PERSIST_EXISTS",
            Self::NotABuild { .. } => "PERSIST_NOT_A_BUILD",
            Self::Write { .. } => "PERSIST_WRITE",
            Self::Rename { .. } => "PERSIST_RENAME",
        }
    }
}

/// Why a load or save failed: each layer with its own code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file-level failure.
    #[error(transparent)]
    Persist(#[from] PersistError),
    /// The document's version is not one this program reads, or a
    /// migration step failed.
    #[error(transparent)]
    Migrate(#[from] MigrateError),
    /// The document is malformed, or the build cannot be written as one.
    #[error(transparent)]
    Dto(#[from] DtoError),
}

impl Error {
    /// Stable code (ADR-014): the failing layer's own.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Persist(e) => e.code(),
            Self::Migrate(e) => e.code(),
            Self::Dto(e) => e.code(),
        }
    }
}

impl From<ReadError> for Error {
    fn from(e: ReadError) -> Self {
        match e {
            ReadError::Migrate(e) => Self::Migrate(e),
            ReadError::Dto(e) => Self::Dto(e),
        }
    }
}

/// Loads and saves build documents through some [`FileOps`].
#[derive(Debug, Clone)]
pub struct Store<F = StdFiles> {
    files: F,
    migrations: Migrations,
}

impl Store {
    /// A store over `std::fs` with the shipped migrations.
    #[must_use]
    pub fn new() -> Self {
        Self::with(StdFiles, Migrations::shipped())
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

/// How many names a save tries for its temporary file before giving up.
const TEMP_ATTEMPTS: u32 = 100;

/// Makes temporary names unique within this process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl<F: FileOps> Store<F> {
    /// A store over `files` with `migrations`.
    pub const fn with(files: F, migrations: Migrations) -> Self {
        Self { files, migrations }
    }

    /// Reads the build document at `path`, migrated to the current
    /// version, with the migration report. Nothing is written.
    ///
    /// # Errors
    /// `PERSIST_READ`, `PERSIST_ENCODING`, then the codes of
    /// [`dto::read_with`] (`MIGRATE_*`, `DTO_*`).
    pub fn load(&self, path: &Path) -> Result<Loaded, Error> {
        let text = self.read_text(path)?;
        Ok(dto::read_with(&text, &self.migrations)?)
    }

    fn read_text(&self, path: &Path) -> Result<String, PersistError> {
        let bytes = self.files.read(path).map_err(|source| PersistError::Read {
            path: path.to_owned(),
            source,
        })?;
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            return Err(PersistError::Encoding {
                path: path.to_owned(),
                reason: "starts with a byte order mark",
            });
        }
        String::from_utf8(bytes).map_err(|_| PersistError::Encoding {
            path: path.to_owned(),
            reason: "not UTF-8",
        })
    }

    /// Writes `build` as a current document to `target`, atomically: a
    /// temporary file in the target's folder, flushed, then published: to
    /// a new target by a hard link that fails if the target has appeared
    /// meanwhile (where the file system has none, by a second check and a
    /// rename), over an existing target by a rename. On any failure the
    /// target is unchanged and the temporary file removed.
    ///
    /// # Errors
    /// In the order checked: `PERSIST_PATH`, `PERSIST_IMPORT_SOURCE`, the
    /// `DTO_*` codes of [`dto::write`], `PERSIST_EXISTS` or (with
    /// [`Overwrite::BuildDocument`]) the codes of reading the existing
    /// target (`PERSIST_READ`, `PERSIST_ENCODING`, `PERSIST_NOT_A_BUILD`,
    /// `MIGRATE_*`, `DTO_*`), then `PERSIST_WRITE`, `PERSIST_EXISTS` (the
    /// target appeared), and `PERSIST_RENAME`. Nothing is written before
    /// `PERSIST_WRITE`.
    pub fn save(
        &self,
        build: &Build,
        target: &Path,
        options: &SaveOptions<'_>,
    ) -> Result<(), Error> {
        let (dir, name) = split(target)?;
        if let Some(source) = options.imported_from
            && same_file(target, source)
        {
            return Err(PersistError::ImportSource {
                path: target.to_owned(),
            }
            .into());
        }
        let json = dto::write(build)?;
        let exists = self
            .files
            .exists(target)
            .map_err(|source| PersistError::Read {
                path: target.to_owned(),
                source,
            })?;
        if exists {
            match options.overwrite {
                Overwrite::Never => {
                    return Err(PersistError::Exists {
                        path: target.to_owned(),
                    }
                    .into());
                }
                Overwrite::BuildDocument => self.check_replaceable(target)?,
            }
        }
        let temp = self.write_temp(&dir, &name, json.as_bytes())?;
        if !exists {
            match self.files.link_new(&temp, target) {
                Ok(()) => {
                    // Published. A temporary name that cannot be removed
                    // is a second name of the saved file, not a copy:
                    // deleting it later loses nothing.
                    self.discard(&temp);
                    return Ok(());
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    self.discard(&temp);
                    return Err(PersistError::Exists {
                        path: target.to_owned(),
                    }
                    .into());
                }
                // No hard links here: check again, then rename.
                Err(_) => {
                    if self.files.exists(target).unwrap_or(true) {
                        self.discard(&temp);
                        return Err(PersistError::Exists {
                            path: target.to_owned(),
                        }
                        .into());
                    }
                }
            }
        }
        self.files.rename(&temp, target).map_err(|source| {
            let leftover = !self.discard(&temp);
            PersistError::Rename {
                from: temp.clone(),
                to: target.to_owned(),
                source,
                leftover,
            }
        })?;
        Ok(())
    }

    /// Writes `bytes` to `target` atomically, replacing it if it exists:
    /// for the application's own files (recovery snapshots, the recent
    /// list), never for a user's build ([`Store::save`] guards those).
    ///
    /// # Errors
    /// `PERSIST_PATH`, `PERSIST_WRITE`, `PERSIST_RENAME`; the target is then
    /// unchanged.
    pub fn write_replacing(&self, target: &Path, bytes: &[u8]) -> Result<(), PersistError> {
        let (dir, name) = split(target)?;
        let temp = self.write_temp(&dir, &name, bytes)?;
        self.files.rename(&temp, target).map_err(|source| {
            let leftover = !self.discard(&temp);
            PersistError::Rename {
                from: temp.clone(),
                to: target.to_owned(),
                source,
                leftover,
            }
        })
    }

    /// Reads a text file this store wrote (UTF-8, no byte order mark).
    ///
    /// # Errors
    /// `PERSIST_READ`, `PERSIST_ENCODING`.
    pub fn read_file(&self, path: &Path) -> Result<String, PersistError> {
        self.read_text(path)
    }

    /// Whether the existing `target` may be replaced: a build document
    /// that reads in full through this store's migrations.
    fn check_replaceable(&self, target: &Path) -> Result<(), Error> {
        let text = self.read_text(target)?;
        let is_build = serde_json::from_str::<serde_json::Value>(&text)
            .is_ok_and(|value| migrate::document_version(&value).is_some());
        if !is_build {
            return Err(PersistError::NotABuild {
                path: target.to_owned(),
            }
            .into());
        }
        dto::read_with(&text, &self.migrations)?;
        Ok(())
    }

    /// Writes `bytes` to a new temporary file next to the target and
    /// returns its path.
    fn write_temp(&self, dir: &Path, name: &OsStr, bytes: &[u8]) -> Result<PathBuf, PersistError> {
        let mut last = None;
        for _ in 0..TEMP_ATTEMPTS {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let mut temp_name = OsString::from(".");
            temp_name.push(name);
            temp_name.push(format!(".{}-{n}.loom-tmp", std::process::id()));
            let temp = dir.join(temp_name);
            match self.files.write_new(&temp, bytes) {
                Ok(()) => return Ok(temp),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some((temp, e)),
                Err(source) => {
                    let leftover = !self.discard(&temp);
                    return Err(PersistError::Write {
                        path: temp,
                        source,
                        leftover,
                    });
                }
            }
        }
        let (path, source) = last.unwrap_or_else(|| {
            (
                dir.to_owned(),
                io::Error::other("no temporary file name was tried"),
            )
        });
        Err(PersistError::Write {
            path,
            source,
            leftover: false,
        })
    }

    /// Removes a temporary file; whether it is gone.
    fn discard(&self, temp: &Path) -> bool {
        match self.files.remove(temp) {
            Ok(()) => true,
            Err(e) => e.kind() == io::ErrorKind::NotFound,
        }
    }
}

/// The target's folder and file name.
fn split(target: &Path) -> Result<(PathBuf, OsString), PersistError> {
    let bad = |reason| PersistError::Path {
        path: target.to_owned(),
        reason,
    };
    let name = target
        .file_name()
        .ok_or_else(|| bad("no file name"))?
        .to_owned();
    let dir = match target.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_owned(),
        _ => PathBuf::from("."),
    };
    Ok((dir, name))
}

/// Whether `a` and `b` name the same file: compared after resolving links,
/// `.` and `..`, and (on Windows) letter case of existing parts. A path
/// that does not exist is resolved through its folder.
fn same_file(a: &Path, b: &Path) -> bool {
    resolve(a) == resolve(b)
}

fn resolve(path: &Path) -> PathBuf {
    if let Ok(p) = std::fs::canonicalize(path) {
        return p;
    }
    match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) => {
            let dir = if dir.as_os_str().is_empty() {
                Path::new(".")
            } else {
                dir
            };
            std::fs::canonicalize(dir).map_or_else(|_| path.to_owned(), |d| d.join(name))
        }
        _ => path.to_owned(),
    }
}

#[cfg(test)]
mod tests;

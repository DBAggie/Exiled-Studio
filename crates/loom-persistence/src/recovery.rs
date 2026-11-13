//! Recovery snapshots (ADR-009; Phase 4 plan, decision 8): while a build
//! has unsaved changes, the application writes it here; a save or a clean
//! close deletes the snapshot, so what is left at start is a session that
//! ended without one.
//!
//! Each running instance owns one snapshot, `<instance>.json`, and holds an
//! exclusive lock on `<instance>.lock` while it runs. [`Recovery::list`]
//! shows only snapshots whose lock is free (their instance has ended), so
//! a second instance never offers, restores, or deletes a live one.
//!
//! The file is versioned (`format: loom-recovery`, `version: 1`) and holds
//! a header and the build document, so it reads through the document's
//! migrations like any saved build.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

use loom_domain::Build;
use loom_serialization::dto;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, PersistError, Store};

/// The snapshot file's format name.
pub const FORMAT: &str = "loom-recovery";
/// The snapshot file's version.
pub const VERSION: u32 = 1;

/// What a snapshot says about its build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// When it was written, in seconds since the Unix epoch.
    pub saved_at: u64,
    /// The build's display name.
    pub name: String,
    /// Where the build was saved, if it was.
    pub path: Option<String>,
    /// The file it was imported from, if any.
    pub origin: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File1 {
    format: String,
    version: u32,
    #[serde(flatten)]
    header: Header,
    build: Value,
}

/// A snapshot left by an instance that has ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Its ID (the instance's).
    pub id: String,
    /// Its header.
    pub header: Header,
}

/// This instance's recovery folder.
#[derive(Debug)]
pub struct Recovery {
    dir: PathBuf,
    instance: String,
    /// Held while the instance runs; dropping it releases the lock.
    _lock: File,
}

/// Whether `id` can name a snapshot: what [`Recovery::open`] issues
/// (ASCII letters, digits, `-`), so an ID from the UI never leaves the
/// folder.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn bad_id(id: &str) -> PersistError {
    PersistError::Path {
        path: PathBuf::from(id),
        reason: "not a recovery snapshot ID",
    }
}

impl Recovery {
    /// Opens `dir` (created if missing) for an instance named `instance`
    /// (a [`valid_id`]), and takes its lock.
    ///
    /// # Errors
    /// The I/O error; [`io::ErrorKind::InvalidInput`] for a bad name, and
    /// [`io::ErrorKind::WouldBlock`] when another instance holds that name.
    pub fn open(dir: &Path, instance: &str) -> io::Result<Self> {
        if !valid_id(instance) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a recovery snapshot ID",
            ));
        }
        std::fs::create_dir_all(dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(format!("{instance}.lock")))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "another instance holds this recovery ID",
                ));
            }
            Err(TryLockError::Error(e)) => return Err(e),
        }
        Ok(Self {
            dir: dir.to_owned(),
            instance: instance.to_owned(),
            _lock: lock,
        })
    }

    /// This instance's ID.
    #[must_use]
    pub fn instance(&self) -> &str {
        &self.instance
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Writes this instance's snapshot of `build`, replacing the last one.
    ///
    /// # Errors
    /// The `DTO_*` codes of writing the document, then the store's.
    pub fn write(&self, store: &Store, header: &Header, build: &Build) -> Result<(), Error> {
        let json = dto::write(build)?;
        let build: Value = serde_json::from_str(&json).map_err(|_| PersistError::Path {
            path: self.file(&self.instance),
            reason: "the build document is not JSON",
        })?;
        let file = File1 {
            format: FORMAT.to_owned(),
            version: VERSION,
            header: header.clone(),
            build,
        };
        let mut text = serde_json::to_string(&file).map_err(|_| PersistError::Path {
            path: self.file(&self.instance),
            reason: "the snapshot does not serialize",
        })?;
        text.push('\n');
        store.write_replacing(&self.file(&self.instance), text.as_bytes())?;
        Ok(())
    }

    /// Deletes this instance's snapshot (after a save or a clean close).
    pub fn clear(&self) {
        let _ = std::fs::remove_file(self.file(&self.instance));
    }

    /// The snapshots of instances that have ended, oldest first; ones that
    /// do not read are skipped (they stay on disk for the user to find).
    #[must_use]
    pub fn list(&self, store: &Store) -> Vec<Found> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if id == self.instance || !valid_id(id) || self.live(id) {
                continue;
            }
            if let Ok((header, _)) = self.read_header(store, id) {
                out.push(Found {
                    id: id.to_owned(),
                    header,
                });
            }
        }
        out.sort_by(|a, b| (a.header.saved_at, &a.id).cmp(&(b.header.saved_at, &b.id)));
        out
    }

    /// Whether another running instance holds `id`'s lock.
    fn live(&self, id: &str) -> bool {
        let Ok(lock) = OpenOptions::new()
            .write(true)
            .open(self.dir.join(format!("{id}.lock")))
        else {
            // No lock file: its instance is gone.
            return false;
        };
        match lock.try_lock() {
            Ok(()) => {
                let _ = lock.unlock();
                false
            }
            Err(_) => true,
        }
    }

    fn read_header(&self, store: &Store, id: &str) -> Result<(Header, Value), Error> {
        let path = self.file(id);
        let text = store.read_file(&path)?;
        let file: File1 = serde_json::from_str(&text)
            .map_err(|_| PersistError::NotABuild { path: path.clone() })?;
        if file.format != FORMAT || file.version != VERSION {
            return Err(PersistError::NotABuild { path }.into());
        }
        Ok((file.header, file.build))
    }

    /// Reads an ended instance's snapshot: its header and build (through
    /// the document's migrations).
    ///
    /// # Errors
    /// `PERSIST_PATH` for an ID that is not one, or a live one; the read's
    /// and the document's codes.
    pub fn read(&self, store: &Store, id: &str) -> Result<(Header, Build), Error> {
        if !valid_id(id) || id == self.instance || self.live(id) {
            return Err(bad_id(id).into());
        }
        let (header, build) = self.read_header(store, id)?;
        let loaded = dto::read(&build.to_string())?;
        Ok((header, loaded.build))
    }

    /// Deletes an ended instance's snapshot and lock file.
    ///
    /// # Errors
    /// `PERSIST_PATH` for an ID that is not one, or a live one.
    pub fn discard(&self, id: &str) -> Result<(), PersistError> {
        if !valid_id(id) || id == self.instance || self.live(id) {
            return Err(bad_id(id));
        }
        let _ = std::fs::remove_file(self.file(id));
        let _ = std::fs::remove_file(self.dir.join(format!("{id}.lock")));
        Ok(())
    }
}

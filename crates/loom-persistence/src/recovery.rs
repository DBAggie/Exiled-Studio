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
//!
//! **Undo history (UX-19).** Beside the snapshot, `<instance>.history/`
//! keeps the latest [`HISTORY_KEEP`] undo steps: one build document per
//! step, `<step>.json`, written once (a step never changes), and
//! `index.json` (`format: loom-recovery-history`, `version: 1`), the steps
//! oldest first with what each change was. A write costs one document per
//! new change, not the whole history; the snapshot file itself is
//! unchanged, so an older application still reads it (without the
//! history). A restore reads the newest steps that read and drops the rest.

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

/// The history's index format name.
pub const HISTORY_FORMAT: &str = "loom-recovery-history";
/// The history index's version.
pub const HISTORY_VERSION: u32 = 1;
/// How many undo steps a snapshot keeps (the session keeps more; a
/// restore needs only the latest, and each is a whole build document).
pub const HISTORY_KEEP: usize = 25;
const HISTORY_INDEX: &str = "index.json";

/// An undo step to keep: its ID (unique within the instance, never reused
/// for another state), what the change was (the application's history
/// entry, opaque here), and the build before it.
#[derive(Debug, Clone, Copy)]
pub struct StepRef<'a> {
    /// The step's ID.
    pub id: u64,
    /// What the change was.
    pub entry: &'a Value,
    /// The build before the change.
    pub build: &'a Build,
}

/// A step read back: what the change was, and the build before it.
#[derive(Debug, Clone)]
pub struct Step {
    /// What the change was.
    pub entry: Value,
    /// The build before the change.
    pub build: Build,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexStep {
    id: u64,
    entry: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    format: String,
    version: u32,
    steps: Vec<IndexStep>,
}

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
    /// What the build is (class, ascendancy, level, main skill), so
    /// snapshots of builds with the same name are told apart. Absent in a
    /// snapshot written before 2026-11-25, and left out when not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<crate::recent::Summary>,
}

#[derive(Deserialize)]
struct HeaderOnly {
    format: String,
    version: u32,
    #[serde(flatten)]
    header: Header,
    #[serde(rename = "build")]
    _build: serde::de::IgnoredAny,
}

// Not strict: a key this version does not know (one a later version
// added to the header) is ignored, so its snapshot is still restored
// (RV-13). Versions up to 0.9.0 were strict here and refuse a snapshot
// that carries `summary`, though they list it.
#[derive(Serialize, Deserialize)]
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

    fn history_dir(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.history"))
    }

    /// Keeps the latest [`HISTORY_KEEP`] of `steps` (oldest first) as this
    /// instance's undo history: writes the steps not written yet, then the
    /// index, then removes the steps it no longer names.
    ///
    /// # Errors
    /// The `DTO_*` codes of writing a document, then the store's.
    pub fn write_history(&self, store: &Store, steps: &[StepRef<'_>]) -> Result<(), Error> {
        let dir = self.history_dir(&self.instance);
        let steps = &steps[steps.len().saturating_sub(HISTORY_KEEP)..];
        if steps.is_empty() {
            let _ = std::fs::remove_dir_all(&dir);
            return Ok(());
        }
        std::fs::create_dir_all(&dir).map_err(|_| PersistError::Path {
            path: dir.clone(),
            reason: "the history folder cannot be made",
        })?;
        for step in steps {
            let file = dir.join(format!("{}.json", step.id));
            if !file.exists() {
                store.write_replacing(&file, dto::write(step.build)?.as_bytes())?;
            }
        }
        let index = Index {
            format: HISTORY_FORMAT.to_owned(),
            version: HISTORY_VERSION,
            steps: steps
                .iter()
                .map(|s| IndexStep {
                    id: s.id,
                    entry: s.entry.clone(),
                })
                .collect(),
        };
        let text = serde_json::to_string(&index).map_err(|_| PersistError::Path {
            path: dir.join(HISTORY_INDEX),
            reason: "the history index does not serialize",
        })?;
        store.write_replacing(&dir.join(HISTORY_INDEX), text.as_bytes())?;
        let kept: std::collections::HashSet<u64> = steps.iter().map(|s| s.id).collect();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let stale = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .is_some_and(|id| !kept.contains(&id));
                if stale {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
    }

    /// An ended instance's undo history, oldest first: the newest steps
    /// that read, stopping at the first that does not (a gap would make
    /// undo skip states). Empty when there is none or it does not read.
    #[must_use]
    pub fn read_history(&self, store: &Store, id: &str) -> Vec<Step> {
        if !valid_id(id) || id == self.instance || self.live(id) {
            return Vec::new();
        }
        let dir = self.history_dir(id);
        let Ok(text) = store.read_file(&dir.join(HISTORY_INDEX)) else {
            return Vec::new();
        };
        let Ok(index) = serde_json::from_str::<Index>(&text) else {
            return Vec::new();
        };
        if index.format != HISTORY_FORMAT || index.version != HISTORY_VERSION {
            return Vec::new();
        }
        let mut out = Vec::new();
        for step in index.steps.into_iter().rev().take(HISTORY_KEEP) {
            let Ok(text) = store.read_file(&dir.join(format!("{}.json", step.id))) else {
                break;
            };
            let Ok(loaded) = dto::read(&text) else {
                break;
            };
            out.push(Step {
                entry: step.entry,
                build: loaded.build,
            });
        }
        out.reverse();
        out
    }

    /// Deletes this instance's snapshot and undo history (after a save or
    /// a clean close).
    pub fn clear(&self) {
        let _ = std::fs::remove_file(self.file(&self.instance));
        let _ = std::fs::remove_dir_all(self.history_dir(&self.instance));
    }

    /// At a clean exit with nothing to recover: releases the lock and
    /// deletes the lock file, so lock files do not accumulate.
    pub fn release(&self) {
        let _ = self._lock.unlock();
        let _ = std::fs::remove_file(self.dir.join(format!("{}.lock", self.instance)));
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
            if let Ok(header) = self.header(store, id) {
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

    /// A snapshot's header, without parsing its build.
    fn header(&self, store: &Store, id: &str) -> Result<Header, Error> {
        let path = self.file(id);
        let text = store.read_file(&path)?;
        let file: HeaderOnly = serde_json::from_str(&text)
            .map_err(|_| PersistError::NotABuild { path: path.clone() })?;
        if file.format != FORMAT || file.version != VERSION {
            return Err(PersistError::NotABuild { path }.into());
        }
        Ok(file.header)
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
        let _ = std::fs::remove_dir_all(self.history_dir(id));
        let _ = std::fs::remove_file(self.dir.join(format!("{id}.lock")));
        Ok(())
    }
}

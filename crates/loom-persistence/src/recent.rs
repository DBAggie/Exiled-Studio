//! The recent builds list (ADR-009; Phase 4 plan, decision 8): a small
//! JSON index in the application's data folder. It is a cache: losing it
//! loses no build, and an entry whose file is gone is dropped when it is
//! opened. Entries are named by an ID derived from the path, so the UI
//! opens one without ever sending a path.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{PersistError, Store};

/// The file's format name.
pub const FORMAT: &str = "loom-recent";
/// The file's version.
pub const VERSION: u32 = 1;
/// How many builds the list keeps.
pub const LIMIT: usize = 10;

/// One recent build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The build document's path.
    pub path: String,
    /// The display name.
    pub name: String,
    /// When it was last opened or saved, in seconds since the Unix epoch.
    pub at: u64,
}

impl Entry {
    /// The entry's ID: FNV-1a of the path, in hex (stable across runs).
    #[must_use]
    pub fn id(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for b in self.path.bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{hash:016x}")
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File1 {
    format: String,
    version: u32,
    entries: Vec<Entry>,
}

/// The recent list at one file.
#[derive(Debug, Clone)]
pub struct Recent {
    file: PathBuf,
}

impl Recent {
    /// The list stored at `file`.
    #[must_use]
    pub fn at(file: &Path) -> Self {
        Self {
            file: file.to_owned(),
        }
    }

    /// The entries, newest first; empty when the file is missing or does
    /// not read (it is a cache).
    #[must_use]
    pub fn entries(&self, store: &Store) -> Vec<Entry> {
        store
            .read_file(&self.file)
            .ok()
            .and_then(|text| serde_json::from_str::<File1>(&text).ok())
            .filter(|f| f.format == FORMAT && f.version == VERSION)
            .map(|f| f.entries)
            .unwrap_or_default()
    }

    /// The entry with this ID.
    #[must_use]
    pub fn find(&self, store: &Store, id: &str) -> Option<Entry> {
        self.entries(store).into_iter().find(|e| e.id() == id)
    }

    /// Puts `entry` first (replacing an entry for the same path) and keeps
    /// [`LIMIT`].
    ///
    /// # Errors
    /// The store's write codes.
    pub fn touch(&self, store: &Store, entry: Entry) -> Result<(), PersistError> {
        let mut entries = self.entries(store);
        entries.retain(|e| e.path != entry.path);
        entries.insert(0, entry);
        entries.truncate(LIMIT);
        self.write(store, entries)
    }

    /// Removes the entry with this ID.
    ///
    /// # Errors
    /// The store's write codes.
    pub fn remove(&self, store: &Store, id: &str) -> Result<(), PersistError> {
        let mut entries = self.entries(store);
        entries.retain(|e| e.id() != id);
        self.write(store, entries)
    }

    fn write(&self, store: &Store, entries: Vec<Entry>) -> Result<(), PersistError> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(|source| PersistError::Write {
                path: dir.to_owned(),
                source,
                leftover: false,
            })?;
        }
        let file = File1 {
            format: FORMAT.to_owned(),
            version: VERSION,
            entries,
        };
        let mut text = serde_json::to_string_pretty(&file).map_err(|_| PersistError::Path {
            path: self.file.clone(),
            reason: "the list does not serialize",
        })?;
        text.push('\n');
        store.write_replacing(&self.file, text.as_bytes())
    }
}

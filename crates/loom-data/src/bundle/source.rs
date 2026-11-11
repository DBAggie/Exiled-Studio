//! Where a bundle's bytes come from (ADR-016): core code reads bytes
//! through [`BundleSource`]; a host implements it over files, an archive,
//! or memory.

use std::collections::BTreeMap;

/// A source of bundle files.
pub trait BundleSource {
    /// Every file path in the source, sorted (the validator checks that no
    /// file is unlisted).
    ///
    /// # Errors
    /// [`SourceError`] when the source cannot be listed.
    fn list(&self) -> Result<Vec<String>, SourceError>;

    /// The bytes of one file.
    ///
    /// # Errors
    /// [`SourceError`] when the file is absent or unreadable.
    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError>;
}

/// A source could not give a file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// No such file.
    #[error("{0}: not in the bundle")]
    NotFound(String),
    /// The host could not read it.
    #[error("{path}: {message}")]
    Unreadable {
        /// The file.
        path: String,
        /// The host's message.
        message: String,
    },
}

impl SourceError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "BUNDLE_SOURCE_NOT_FOUND",
            Self::Unreadable { .. } => "BUNDLE_SOURCE_UNREADABLE",
        }
    }
}

/// Bundle files held in memory (tests; a web host).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemorySource {
    files: BTreeMap<String, Vec<u8>>,
}

impl MemorySource {
    /// An empty source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a file.
    pub fn insert(&mut self, path: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files.insert(path.into(), bytes.into());
    }

    /// Removes a file.
    pub fn remove(&mut self, path: &str) -> Option<Vec<u8>> {
        self.files.remove(path)
    }
}

impl BundleSource for MemorySource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        Ok(self.files.keys().cloned().collect())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| SourceError::NotFound(path.to_owned()))
    }
}

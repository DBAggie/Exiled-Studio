//! Filesystem edge: reads a game install's Bundles2 index. Parsers stay in
//! [`crate::bundle`] and [`crate::index`] and never touch files.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::bundle::{Bundle, BundleError};
use crate::decompress::Decompressor;
use crate::index::{Index, IndexError};

/// Location of the index inside an install.
pub const INDEX_PATH: &str = "Bundles2/_.index.bin";

/// The index of one install, with the facts needed to tie output to input.
#[derive(Debug, Clone)]
pub struct InstallIndex {
    /// SHA-256 of `_.index.bin` as stored on disk.
    pub sha256: [u8; 32],
    /// Size of `_.index.bin` on disk.
    pub file_len: usize,
    /// Size of the decoded index.
    pub decoded_len: usize,
    /// The parsed index.
    pub index: Index,
}

/// Errors from reading an install. Each variant has a stable [`code`](Self::code).
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// File that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// `_.index.bin` is not a valid bundle.
    #[error("{INDEX_PATH}: {0}")]
    Bundle(#[from] BundleError),
    /// The decoded index is invalid.
    #[error("{INDEX_PATH}: {0}")]
    Index(#[from] IndexError),
}

impl InstallError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Read { .. } => "EXTRACT_INSTALL_READ",
            Self::Bundle(inner) => inner.code(),
            Self::Index(inner) => inner.code(),
        }
    }
}

/// Reads, hashes, decodes, and parses `<install>/Bundles2/_.index.bin`.
///
/// # Errors
/// [`InstallError`] if the file cannot be read or is invalid.
pub fn read_index<D>(install: &Path, decompressor: &D) -> Result<InstallIndex, InstallError>
where
    D: Decompressor + ?Sized,
{
    let path = install.join(INDEX_PATH);
    let bytes = std::fs::read(&path).map_err(|source| InstallError::Read { path, source })?;
    let decoded = Bundle::parse(&bytes)?.decode(decompressor)?;
    let index = Index::parse(&decoded, decompressor)?;
    Ok(InstallIndex {
        sha256: Sha256::digest(&bytes).into(),
        file_len: bytes.len(),
        decoded_len: decoded.len(),
        index,
    })
}

/// Lowercase hex of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompress::Identity;

    #[test]
    fn hex_should_format_lowercase_pairs() {
        assert_eq!(hex(&[0x00, 0xab, 0x7f]), "00ab7f");
    }

    #[test]
    fn read_index_should_report_missing_index() {
        let err = read_index(Path::new("loom-no-such-install"), &Identity).unwrap_err();
        assert_eq!(err.code(), "EXTRACT_INSTALL_READ");
    }
}

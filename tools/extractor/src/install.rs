//! Filesystem edge: reads a game install's Bundles2 index and the files it
//! lists. Parsers stay in [`crate::bundle`], [`crate::index`], and
//! [`crate::dat`] and never touch files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::bundle::{Bundle, BundleError};
use crate::decompress::Decompressor;
use crate::index::{FileRecord, Index, IndexError};
use crate::reader::widen;

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
    /// A data bundle is not valid.
    #[error("bundle {name}: {source}")]
    DataBundle {
        /// Bundle name from the index.
        name: String,
        /// What is wrong.
        #[source]
        source: BundleError,
    },
    /// A data bundle decoded to a different size than the index records.
    #[error("bundle {name}: decoded {actual} bytes, index says {expected}")]
    BundleLength {
        /// Bundle name from the index.
        name: String,
        /// Size from the index.
        expected: u32,
        /// Decoded size.
        actual: usize,
    },
    /// A file record does not fit in its bundle, or names no bundle (only
    /// possible for an [`Index`] not built by [`Index::parse`]).
    #[error("file record {record:?} is outside its bundle")]
    RecordOutOfRange {
        /// The record.
        record: FileRecord,
    },
    /// A bundle name from the index is not a plain relative path, so it
    /// could point outside `Bundles2/`.
    #[error("bundle name {name:?} is not a plain relative path")]
    BadBundleName {
        /// The name.
        name: String,
    },
}

impl InstallError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Read { .. } => "EXTRACT_INSTALL_READ",
            Self::Bundle(inner) => inner.code(),
            Self::Index(inner) => inner.code(),
            Self::DataBundle { source, .. } => source.code(),
            Self::BundleLength { .. } => "EXTRACT_INSTALL_BUNDLE_LENGTH",
            Self::RecordOutOfRange { .. } => "EXTRACT_INSTALL_RECORD_OUT_OF_RANGE",
            Self::BadBundleName { .. } => "EXTRACT_INSTALL_BAD_BUNDLE_NAME",
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

/// Path of bundle `name` (from the index, so untrusted) inside `install`.
/// Only `/`-separated names whose parts are not empty, `.`, or `..` and hold
/// no `\` or `:` are accepted.
fn bundle_path(install: &Path, name: &str) -> Result<PathBuf, InstallError> {
    let plain = name.split('/').all(|part| {
        !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':', '\0'])
    });
    if !plain {
        return Err(InstallError::BadBundleName {
            name: name.to_owned(),
        });
    }
    Ok(install.join("Bundles2").join(format!("{name}.bundle.bin")))
}

/// SHA-256 of each bundle file read, by bundle name from the index.
pub type BundleHashes = BTreeMap<String, [u8; 32]>;

/// Reads the files of `records` from `<install>/Bundles2`, reading and
/// decoding each bundle once, and calls `visit` with each record's position in
/// `records` and its bytes. Files are visited bundle by bundle (in bundle
/// order, not `records` order), so only one decoded bundle is in memory at a
/// time. An error from `visit` stops the walk and is returned. Returns the
/// SHA-256 of every bundle file read, as stored on disk.
///
/// # Errors
/// `E` from `visit`, or [`InstallError`] (converted) if a bundle name is not a
/// plain relative path, a bundle cannot be read or decoded, decodes to a size
/// other than the index records, or a record is outside its bundle.
pub fn read_files<D, F, E>(
    install: &Path,
    index: &Index,
    records: &[FileRecord],
    decompressor: &D,
    mut visit: F,
) -> Result<BundleHashes, E>
where
    D: Decompressor + ?Sized,
    F: FnMut(usize, &[u8]) -> Result<(), E>,
    E: From<InstallError>,
{
    let mut hashes = BundleHashes::new();
    let mut by_bundle: BTreeMap<u32, Vec<(usize, FileRecord)>> = BTreeMap::new();
    for (position, &record) in records.iter().enumerate() {
        by_bundle
            .entry(record.bundle)
            .or_default()
            .push((position, record));
    }
    for (bundle, files) in by_bundle {
        // Each group has at least one record.
        let Some(&(_, first)) = files.first() else {
            continue;
        };
        let entry = index
            .bundles
            .get(widen(bundle))
            .ok_or(InstallError::RecordOutOfRange { record: first })?;
        let path = bundle_path(install, &entry.name)?;
        let bytes = std::fs::read(&path).map_err(|source| InstallError::Read { path, source })?;
        hashes.insert(entry.name.clone(), Sha256::digest(&bytes).into());
        let decoded = Bundle::parse(&bytes)
            .and_then(|b| b.decode(decompressor))
            .map_err(|source| InstallError::DataBundle {
                name: entry.name.clone(),
                source,
            })?;
        if decoded.len() != widen(entry.decompressed_len) {
            return Err(InstallError::BundleLength {
                name: entry.name.clone(),
                expected: entry.decompressed_len,
                actual: decoded.len(),
            }
            .into());
        }
        for (position, record) in files {
            let start = widen(record.offset);
            let file = start
                .checked_add(widen(record.size))
                .and_then(|end| decoded.get(start..end))
                .ok_or(InstallError::RecordOutOfRange { record })?;
            visit(position, file)?;
        }
    }
    Ok(hashes)
}

/// The Steam build ID of the install, read from the `appmanifest_*.acf` in
/// the Steam library's `steamapps` folder (two levels above an install in
/// `steamapps/common/<folder>`) whose `installdir` is the install's folder
/// name. `None` if there is no such manifest, for example outside Steam.
#[must_use]
pub fn steam_build_id(install: &Path) -> Option<String> {
    let folder = install.file_name()?.to_str()?;
    let steamapps = install.parent()?.parent()?;
    let mut found: Vec<String> = std::fs::read_dir(steamapps)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("appmanifest_") && name.ends_with(".acf")
        })
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter(|text| acf_value(text, "installdir") == Some(folder))
        .filter_map(|text| acf_value(&text, "buildid").map(str::to_owned))
        .collect();
    // Two manifests for one folder would be ambiguous.
    found.dedup();
    match found.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// The first `"key" "value"` pair named `key` in a Steam `.acf` text.
fn acf_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let mut parts = line.trim().split('"');
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(""), Some(k), Some(gap), Some(value)) if k == key && gap.trim().is_empty() => {
                Some(value)
            }
            _ => None,
        }
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
    use crate::bundle::build_identity_bundle;
    use crate::decompress::Identity;
    use crate::index::BundleEntry;

    /// A scratch install with `Bundles2/<name>.bundle.bin` files, removed on drop.
    struct FakeInstall(PathBuf);

    impl FakeInstall {
        fn new(test: &str, bundles: &[(&str, &[u8])]) -> Self {
            let root =
                std::env::temp_dir().join(format!("loom-install-{test}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("Bundles2")).unwrap();
            for (name, data) in bundles {
                let path = root.join("Bundles2").join(format!("{name}.bundle.bin"));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, build_identity_bundle(data, 4)).unwrap();
            }
            Self(root)
        }
    }

    impl Drop for FakeInstall {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn index(bundles: &[(&str, u32)]) -> Index {
        Index {
            bundles: bundles
                .iter()
                .map(|&(name, len)| BundleEntry {
                    name: name.to_owned(),
                    decompressed_len: len,
                })
                .collect(),
            files: Vec::new(),
            path_reps: Vec::new(),
            path_data: Vec::new(),
        }
    }

    const fn record(bundle: u32, offset: u32, size: u32) -> FileRecord {
        FileRecord {
            path_hash: 0,
            bundle,
            offset,
            size,
        }
    }

    fn collect(
        install: &FakeInstall,
        index: &Index,
        records: &[FileRecord],
    ) -> Result<Vec<(usize, Vec<u8>)>, InstallError> {
        let mut seen = Vec::new();
        read_files(&install.0, index, records, &Identity, |i, bytes| {
            seen.push((i, bytes.to_vec()));
            Ok::<_, InstallError>(())
        })?;
        Ok(seen)
    }

    #[test]
    fn read_files_should_hash_each_bundle_read() {
        let install = FakeInstall::new("hash", &[("a", b"0123"), ("b", b"xy"), ("c", b"z")]);
        let index = index(&[("a", 4), ("b", 2), ("c", 1)]);
        let records = [record(2, 0, 1), record(0, 0, 1), record(0, 1, 1)];
        let hashes = read_files(&install.0, &index, &records, &Identity, |_, _| {
            Ok::<_, InstallError>(())
        })
        .unwrap();
        let expected: BundleHashes = ["a", "c"]
            .into_iter()
            .map(|name| {
                let bytes =
                    std::fs::read(install.0.join(format!("Bundles2/{name}.bundle.bin"))).unwrap();
                (name.to_owned(), Sha256::digest(bytes).into())
            })
            .collect();
        assert_eq!(hashes, expected);
    }

    /// A Steam `appmanifest` text with `pairs`, tab-separated as Steam writes it.
    fn acf(pairs: &[(&str, &str)]) -> String {
        let body: String = pairs
            .iter()
            .map(|(k, v)| format!("\t\"{k}\"\t\t\"{v}\"\n"))
            .collect();
        format!("\"AppState\"\n{{\n{body}}}\n")
    }

    #[test]
    fn acf_value_should_read_quoted_pairs() {
        let text = acf(&[
            ("appid", "238960"),
            ("installdir", "Path of Exile"),
            ("buildid", "26474023"),
        ]);
        let text = text.as_str();
        assert_eq!(acf_value(text, "buildid"), Some("26474023"));
        assert_eq!(acf_value(text, "installdir"), Some("Path of Exile"));
        assert_eq!(acf_value(text, "AppState"), None);
        assert_eq!(acf_value(text, "missing"), None);
    }

    #[test]
    fn steam_build_id_should_match_install_folder() {
        let root = std::env::temp_dir().join(format!("loom-steam-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let install = root.join("steamapps/common/Game A");
        std::fs::create_dir_all(&install).unwrap();
        for (id, dir, build) in [("1", "Game A", "100"), ("2", "Game B", "200")] {
            std::fs::write(
                root.join(format!("steamapps/appmanifest_{id}.acf")),
                acf(&[("installdir", dir), ("buildid", build)]),
            )
            .unwrap();
        }
        assert_eq!(steam_build_id(&install).as_deref(), Some("100"));
        assert_eq!(steam_build_id(&root.join("steamapps/common/Other")), None);
        assert_eq!(steam_build_id(Path::new("loom-no-such-install")), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn read_files_should_slice_files_bundle_by_bundle() {
        let install = FakeInstall::new("slice", &[("a", b"0123456789"), ("dir/b", b"abcdef")]);
        let index = index(&[("a", 10), ("dir/b", 6)]);
        let records = [
            record(1, 2, 3),
            record(0, 0, 4),
            record(1, 0, 0),
            record(0, 8, 2),
        ];
        let seen = collect(&install, &index, &records).unwrap();
        assert_eq!(
            seen,
            [
                (1, b"0123".to_vec()),
                (3, b"89".to_vec()),
                (0, b"cde".to_vec()),
                (2, Vec::new()),
            ]
        );
    }

    #[test]
    fn read_files_should_report_bad_bundles_and_records() {
        let install = FakeInstall::new("errors", &[("a", b"0123456789")]);
        let cases = [
            (
                index(&[("a", 10)]),
                record(0, 8, 3),
                "EXTRACT_INSTALL_RECORD_OUT_OF_RANGE",
            ),
            (
                index(&[("a", 10)]),
                record(1, 0, 1),
                "EXTRACT_INSTALL_RECORD_OUT_OF_RANGE",
            ),
            (
                index(&[("a", 11)]),
                record(0, 0, 1),
                "EXTRACT_INSTALL_BUNDLE_LENGTH",
            ),
            (
                index(&[("missing", 1)]),
                record(0, 0, 1),
                "EXTRACT_INSTALL_READ",
            ),
        ];
        for (index, record, code) in cases {
            let err = collect(&install, &index, &[record]).unwrap_err();
            assert_eq!(err.code(), code, "{err}");
        }
        for name in [
            "../a", "a/../a", "/a", "a//b", "./a", "C:/a", "a\\b", "", "a/",
        ] {
            let err = collect(&install, &index(&[(name, 10)]), &[record(0, 0, 1)]).unwrap_err();
            assert_eq!(err.code(), "EXTRACT_INSTALL_BAD_BUNDLE_NAME", "{name}");
        }
        std::fs::write(install.0.join("Bundles2/bad.bundle.bin"), [1, 2, 3]).unwrap();
        let err = collect(&install, &index(&[("bad", 1)]), &[record(0, 0, 1)]).unwrap_err();
        assert_eq!(err.code(), "EXTRACT_BUNDLE_TRUNCATED", "{err}");
    }

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

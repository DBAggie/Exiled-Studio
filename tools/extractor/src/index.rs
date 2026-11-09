//! The Bundles2 index (`Bundles2/_.index.bin`, itself a bundle): which bundle
//! holds each file, and the file paths. Layout in
//! `docs/spikes/spike-a-extraction.md`, section 4.
//!
//! The index stores files by path hash only. Paths live in an embedded
//! bundle, split into one block ("path rep") per directory record and encoded
//! as fragments: [`Index::paths`] rebuilds them, and [`Index::list`] hashes
//! every path, matches it to its file record, and lists whatever does not
//! match on either side instead of dropping it (ADR-015).

use std::collections::BTreeSet;

use crate::bundle::{Bundle, BundleError};
use crate::decompress::Decompressor;
use crate::hash::path_hash;
use crate::reader::{Reader, Short, widen};

/// Bytes per file record and per path rep.
const RECORD_LEN: usize = 20;

/// Longest prefix or path accepted, in bytes. Real paths are far shorter;
/// the cap keeps memory linear in the input when a block chains prefixes.
pub const MAX_PATH_LEN: usize = 4096;

/// A bundle named in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleEntry {
    /// Name relative to `Bundles2/`, without the `.bundle.bin` extension.
    pub name: String,
    /// Size of the bundle's decoded data.
    pub decompressed_len: u32,
}

/// Where one file is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileRecord {
    /// [`path_hash`] of the file path.
    pub path_hash: u64,
    /// Position of the bundle in [`Index::bundles`].
    pub bundle: u32,
    /// Offset of the file in the decoded bundle.
    pub offset: u32,
    /// Size of the file.
    pub size: u32,
}

/// A directory record and the block of path data that lists its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathRep {
    /// [`path_hash`] of the directory path.
    pub hash: u64,
    /// Offset of the block in the decoded path data.
    pub offset: u32,
    /// Size of the block.
    pub size: u32,
    /// Size of this block plus those of its subdirectories (not used here).
    pub recursive_size: u32,
}

/// A parsed index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// Bundles, in index order (file records refer to them by position).
    pub bundles: Vec<BundleEntry>,
    /// File records, in index order.
    pub files: Vec<FileRecord>,
    /// Directory records, in index order.
    pub path_reps: Vec<PathRep>,
    /// Decoded path data of the embedded path bundle.
    pub path_data: Vec<u8>,
}

/// A file whose path matched its record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ListedFile {
    /// Full path as stored in the index (observed lowercase), `/`-separated.
    pub path: String,
    /// The record whose hash matches the path.
    pub record: FileRecord,
}

/// Result of matching rebuilt paths against the index records.
///
/// Nothing is dropped: every rebuilt path occurrence and every record appears
/// in `files`, `paths_without_record`, or `records_without_path`. A path that
/// matches several records is listed once per record, and a path rebuilt
/// twice (from two blocks) is listed twice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// Matched files, sorted by path.
    pub files: Vec<ListedFile>,
    /// Rebuilt paths whose hash is not in any file record, sorted.
    pub paths_without_record: Vec<String>,
    /// File records whose hash no rebuilt path produces, sorted by hash.
    pub records_without_path: Vec<FileRecord>,
    /// Directory records whose hash matches no directory of a rebuilt path,
    /// sorted by hash.
    pub path_reps_without_directory: Vec<PathRep>,
}

/// Errors from reading an index. Each variant has a stable [`code`](Self::code).
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// The decoded index ends before a field.
    #[error(
        "index truncated: {} at offset {} needs {} bytes, {} left",
        .0.field, .0.offset, .0.needed, .0.available
    )]
    Truncated(Short),
    /// A bundle name is not UTF-8.
    #[error("index bundle {bundle}: name is not UTF-8")]
    BundleNameNotUtf8 {
        /// Position of the bundle.
        bundle: usize,
    },
    /// A file record names a bundle that does not exist.
    #[error("index file record {record}: bundle {bundle} out of range ({bundles} bundles)")]
    BundleOutOfRange {
        /// Position of the record.
        record: usize,
        /// Bundle it names.
        bundle: u32,
        /// Number of bundles.
        bundles: usize,
    },
    /// A file record's range runs past the end of its bundle.
    #[error(
        "index file record {record}: bytes {offset}+{size} exceed bundle {bundle} ({bundle_len} bytes)"
    )]
    FileOutOfBundle {
        /// Position of the record.
        record: usize,
        /// Bundle it names.
        bundle: u32,
        /// Offset in the bundle.
        offset: u32,
        /// Size of the file.
        size: u32,
        /// Decoded size of the bundle.
        bundle_len: u32,
    },
    /// The embedded path bundle could not be read.
    /// The embedded path bundle could not be read. Its code is the inner
    /// bundle or decoder code; the message says it was the path bundle.
    #[error("index path bundle: {0}")]
    PathBundle(#[source] BundleError),
    /// A prefix or path is longer than [`MAX_PATH_LEN`].
    #[error("index path rep {rep}: path at path-data offset {offset} exceeds {MAX_PATH_LEN} bytes")]
    PathTooLong {
        /// Position of the directory record.
        rep: usize,
        /// Offset of the fragment that made it too long.
        offset: usize,
    },
    /// A directory record's block runs past the end of the path data.
    #[error("index path rep {rep}: bytes {offset}+{size} exceed path data ({data_len} bytes)")]
    PathRepOutOfRange {
        /// Position of the directory record.
        rep: usize,
        /// Offset of its block.
        offset: u32,
        /// Size of its block.
        size: u32,
        /// Size of the path data.
        data_len: usize,
    },
    /// A path block ends inside a fragment.
    #[error(
        "index path rep {rep}: {} at path-data offset {} needs {} bytes, {} left in the block",
        .short.field, .short.offset, .short.needed, .short.available
    )]
    PathDataTruncated {
        /// Position of the directory record.
        rep: usize,
        /// The short read.
        short: Short,
    },
    /// A fragment refers to a prefix that has not been defined.
    #[error(
        "index path rep {rep}: fragment at path-data offset {offset} uses prefix {prefix}, only {defined} defined"
    )]
    FragmentOutOfRange {
        /// Position of the directory record.
        rep: usize,
        /// Offset of the fragment.
        offset: usize,
        /// Prefix number it uses (1-based).
        prefix: u32,
        /// Prefixes defined so far.
        defined: usize,
    },
    /// A rebuilt path is not UTF-8.
    #[error("index path rep {rep}: path at path-data offset {offset} is not UTF-8")]
    PathNotUtf8 {
        /// Position of the directory record.
        rep: usize,
        /// Offset of the path's last fragment.
        offset: usize,
    },
}

impl From<Short> for IndexError {
    fn from(short: Short) -> Self {
        Self::Truncated(short)
    }
}

impl IndexError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Truncated(_) => "EXTRACT_INDEX_TRUNCATED",
            Self::BundleNameNotUtf8 { .. } | Self::PathNotUtf8 { .. } => {
                "EXTRACT_INDEX_INVALID_UTF8"
            }
            Self::BundleOutOfRange { .. } => "EXTRACT_INDEX_BUNDLE_OUT_OF_RANGE",
            Self::FileOutOfBundle { .. } => "EXTRACT_INDEX_FILE_OUT_OF_BUNDLE",
            Self::PathBundle(inner) => inner.code(),
            Self::PathTooLong { .. } => "EXTRACT_INDEX_PATH_TOO_LONG",
            Self::PathRepOutOfRange { .. } => "EXTRACT_INDEX_PATH_REP_OUT_OF_RANGE",
            Self::PathDataTruncated { .. } => "EXTRACT_INDEX_PATH_DATA_TRUNCATED",
            Self::FragmentOutOfRange { .. } => "EXTRACT_INDEX_PATH_FRAGMENT_OUT_OF_RANGE",
        }
    }
}

/// Reads `count` fixed-size records after checking that their bytes exist,
/// so a corrupt count cannot cause a huge allocation.
fn records<'a, T>(
    r: &mut Reader<'a>,
    field: &'static str,
    count: u32,
    mut read: impl FnMut(&mut Reader<'a>) -> Result<T, Short>,
) -> Result<Vec<T>, Short> {
    let block = r.take(field, widen(count).saturating_mul(RECORD_LEN))?;
    let mut block = Reader::new(block);
    (0..count).map(|_| read(&mut block)).collect()
}

impl Index {
    /// Parses decoded `_.index.bin` bytes and decodes the embedded path
    /// bundle with `decompressor`. Checks that every file record names an
    /// existing bundle and fits inside it, and that every path block lies
    /// inside the path data.
    ///
    /// # Errors
    /// Any [`IndexError`] except the path-rebuilding ones.
    pub fn parse<D>(decoded: &[u8], decompressor: &D) -> Result<Self, IndexError>
    where
        D: Decompressor + ?Sized,
    {
        let mut r = Reader::new(decoded);

        let bundle_count = r.u32("bundle count")?;
        let mut bundles = Vec::new();
        for position in 0..widen(bundle_count) {
            let name_len = r.u32("bundle name length")?;
            let name = r.take("bundle name", widen(name_len))?;
            let name = std::str::from_utf8(name)
                .map_err(|_| IndexError::BundleNameNotUtf8 { bundle: position })?
                .to_owned();
            let decompressed_len = r.u32("bundle size")?;
            bundles.push(BundleEntry {
                name,
                decompressed_len,
            });
        }

        let file_count = r.u32("file count")?;
        let files = records(&mut r, "file records", file_count, |b| {
            Ok(FileRecord {
                path_hash: b.u64("path hash")?,
                bundle: b.u32("bundle")?,
                offset: b.u32("offset")?,
                size: b.u32("size")?,
            })
        })?;
        for (position, file) in files.iter().enumerate() {
            let bundle = bundles
                .get(widen(file.bundle))
                .ok_or(IndexError::BundleOutOfRange {
                    record: position,
                    bundle: file.bundle,
                    bundles: bundles.len(),
                })?;
            if u64::from(file.offset) + u64::from(file.size) > u64::from(bundle.decompressed_len) {
                return Err(IndexError::FileOutOfBundle {
                    record: position,
                    bundle: file.bundle,
                    offset: file.offset,
                    size: file.size,
                    bundle_len: bundle.decompressed_len,
                });
            }
        }

        let rep_count = r.u32("path rep count")?;
        let path_reps = records(&mut r, "path reps", rep_count, |b| {
            Ok(PathRep {
                hash: b.u64("directory hash")?,
                offset: b.u32("offset")?,
                size: b.u32("size")?,
                recursive_size: b.u32("recursive size")?,
            })
        })?;

        // The rest of the index is the path bundle, which must end exactly at
        // the end of the index (`Bundle::parse` rejects trailing bytes).
        let path_data = Bundle::parse(r.rest())
            .and_then(|bundle| bundle.decode(decompressor))
            .map_err(IndexError::PathBundle)?;
        for (position, rep) in path_reps.iter().enumerate() {
            if u64::from(rep.offset) + u64::from(rep.size) > path_data.len() as u64 {
                return Err(IndexError::PathRepOutOfRange {
                    rep: position,
                    offset: rep.offset,
                    size: rep.size,
                    data_len: path_data.len(),
                });
            }
        }

        Ok(Self {
            bundles,
            files,
            path_reps,
            path_data,
        })
    }

    /// Rebuilds every file path, in index order.
    ///
    /// Each block is a sequence of `u32 n` values. `n = 0` toggles between
    /// the prefix phase and the path phase (the block starts in the path phase,
    /// and entering the prefix phase clears the prefixes). Any other `n` is
    /// followed by a NUL-terminated fragment: if `n - 1` names an existing
    /// prefix, the fragment is appended to it, and `n - 1` equal to the prefix
    /// count starts a new one. In the prefix phase the result becomes the next
    /// prefix; in the path phase it is a file path. Anything else is an error,
    /// as is a block that does not end exactly at a fragment boundary.
    ///
    /// # Errors
    /// [`IndexError::PathDataTruncated`], [`IndexError::FragmentOutOfRange`],
    /// or [`IndexError::PathNotUtf8`].
    pub fn paths(&self) -> Result<Vec<String>, IndexError> {
        let mut paths = Vec::new();
        for (position, rep) in self.path_reps.iter().enumerate() {
            let truncated = |short| IndexError::PathDataTruncated {
                rep: position,
                short,
            };
            let start = widen(rep.offset);
            let end = start.saturating_add(widen(rep.size));
            // `parse` checked that the block lies inside the path data.
            let block = self.path_data.get(..end).unwrap_or_default();
            let mut r = Reader::new(block);
            r.take("path rep start", start).map_err(truncated)?;

            let mut in_prefix_phase = false;
            let mut prefixes: Vec<Vec<u8>> = Vec::new();
            while !r.is_empty() {
                let offset = r.pos();
                let n = r.u32("path fragment number").map_err(truncated)?;
                if n == 0 {
                    in_prefix_phase = !in_prefix_phase;
                    if in_prefix_phase {
                        prefixes.clear();
                    }
                    continue;
                }
                let fragment = r.cstr("path fragment").map_err(truncated)?;
                let prefix_number = widen(n - 1);
                let mut text = match prefix_number.cmp(&prefixes.len()) {
                    std::cmp::Ordering::Less => {
                        prefixes.get(prefix_number).cloned().unwrap_or_default()
                    }
                    std::cmp::Ordering::Equal => Vec::new(),
                    std::cmp::Ordering::Greater => {
                        return Err(IndexError::FragmentOutOfRange {
                            rep: position,
                            offset,
                            prefix: n,
                            defined: prefixes.len(),
                        });
                    }
                };
                if text.len().saturating_add(fragment.len()) > MAX_PATH_LEN {
                    return Err(IndexError::PathTooLong {
                        rep: position,
                        offset,
                    });
                }
                text.extend_from_slice(fragment);
                if in_prefix_phase {
                    prefixes.push(text);
                } else {
                    let path = String::from_utf8(text).map_err(|_| IndexError::PathNotUtf8 {
                        rep: position,
                        offset,
                    })?;
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }

    /// Rebuilds the paths and matches them to the file and directory records.
    ///
    /// # Errors
    /// Any error from [`Index::paths`].
    pub fn list(&self) -> Result<Listing, IndexError> {
        let paths = self.paths()?;

        // Directory hashes: every ancestor directory of every path, root included.
        let mut directories: BTreeSet<&str> = BTreeSet::new();
        directories.insert("");
        let mut previous_dir = None;
        for path in &paths {
            let dir = path.rfind('/').map_or("", |i| path.get(..i).unwrap_or(""));
            if previous_dir == Some(dir) {
                continue;
            }
            previous_dir = Some(dir);
            for (i, _) in dir.match_indices('/') {
                directories.insert(dir.get(..i).unwrap_or(""));
            }
            directories.insert(dir);
        }
        let mut dir_hashes: Vec<u64> = directories.into_iter().map(path_hash).collect();
        dir_hashes.sort_unstable();
        let mut path_reps_without_directory: Vec<PathRep> = self
            .path_reps
            .iter()
            .filter(|rep| dir_hashes.binary_search(&rep.hash).is_err())
            .copied()
            .collect();
        path_reps_without_directory.sort_unstable();

        let mut records = self.files.clone();
        records.sort_unstable();
        let mut matched = vec![false; records.len()];
        let mut listing = Listing {
            path_reps_without_directory,
            ..Listing::default()
        };
        for path in paths {
            let hash = path_hash(&path);
            let first = records.partition_point(|rec| rec.path_hash < hash);
            let count = records
                .get(first..)
                .unwrap_or_default()
                .iter()
                .take_while(|rec| rec.path_hash == hash)
                .count();
            if count == 0 {
                listing.paths_without_record.push(path);
                continue;
            }
            for (record, seen) in records
                .iter()
                .zip(matched.iter_mut())
                .skip(first)
                .take(count)
            {
                *seen = true;
                listing.files.push(ListedFile {
                    path: path.clone(),
                    record: *record,
                });
            }
        }
        listing.records_without_path = records
            .into_iter()
            .zip(matched)
            .filter_map(|(record, seen)| (!seen).then_some(record))
            .collect();
        listing.files.sort_unstable();
        listing.paths_without_record.sort_unstable();
        Ok(listing)
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic buffers; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::bundle::build_identity_bundle;
    use crate::decompress::Identity;

    /// Path data writer for synthetic indexes.
    #[derive(Default)]
    struct PathData(Vec<u8>);

    impl PathData {
        fn toggle(&mut self) -> &mut Self {
            self.0.extend_from_slice(&0_u32.to_le_bytes());
            self
        }

        fn fragment(&mut self, n: u32, text: &str) -> &mut Self {
            self.0.extend_from_slice(&n.to_le_bytes());
            self.0.extend_from_slice(text.as_bytes());
            self.0.push(0);
            self
        }

        fn len(&self) -> u32 {
            u32::try_from(self.0.len()).unwrap()
        }
    }

    fn rep(dir: &str, offset: u32, size: u32) -> PathRep {
        PathRep {
            hash: path_hash(dir),
            offset,
            size,
            recursive_size: 0,
        }
    }

    fn file(path: &str, bundle: u32, offset: u32, size: u32) -> FileRecord {
        FileRecord {
            path_hash: path_hash(path),
            bundle,
            offset,
            size,
        }
    }

    /// Encodes decoded index bytes, with the path data as an embedded identity bundle.
    fn encode(
        bundles: &[(&str, u32)],
        files: &[FileRecord],
        reps: &[PathRep],
        path_data: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let n = |len: usize| u32::try_from(len).unwrap().to_le_bytes();
        out.extend_from_slice(&n(bundles.len()));
        for (name, size) in bundles {
            out.extend_from_slice(&n(name.len()));
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&size.to_le_bytes());
        }
        out.extend_from_slice(&n(files.len()));
        for f in files {
            out.extend_from_slice(&f.path_hash.to_le_bytes());
            out.extend_from_slice(&f.bundle.to_le_bytes());
            out.extend_from_slice(&f.offset.to_le_bytes());
            out.extend_from_slice(&f.size.to_le_bytes());
        }
        out.extend_from_slice(&n(reps.len()));
        for r in reps {
            out.extend_from_slice(&r.hash.to_le_bytes());
            out.extend_from_slice(&r.offset.to_le_bytes());
            out.extend_from_slice(&r.size.to_le_bytes());
            out.extend_from_slice(&r.recursive_size.to_le_bytes());
        }
        out.extend_from_slice(&build_identity_bundle(path_data, 32));
        out
    }

    /// Path data for the sample index: a root block with no paths, a block
    /// under `data/`, and a block under `art/` with a two-level prefix.
    fn sample_path_data() -> (PathData, Vec<PathRep>) {
        let mut pd = PathData::default();
        pd.toggle().toggle();
        let root = rep("", 0, pd.len());

        let start = pd.len();
        pd.toggle()
            .fragment(1, "data/")
            .toggle()
            .fragment(1, "stats.datc64")
            .fragment(1, "mods.datc64")
            .fragment(2, "readme.txt");
        let data = rep("data", start, pd.len() - start);

        let start = pd.len();
        pd.toggle()
            .fragment(1, "art/")
            .fragment(1, "2dart/")
            .toggle()
            .fragment(2, "a.dds")
            .fragment(1, "b.dds");
        let art = rep("art", start, pd.len() - start);

        (pd, vec![root, data, art])
    }

    const BUNDLES: &[(&str, u32)] = &[("Data/Bundle0", 100), ("Art/Bundle1", 50)];

    fn sample_index_bytes() -> Vec<u8> {
        let (pd, mut reps) = sample_path_data();
        reps.push(PathRep {
            hash: 0xdead,
            offset: 0,
            size: 0,
            recursive_size: 0,
        });
        let files = [
            file("data/stats.datc64", 0, 0, 10),
            file("data/mods.datc64", 0, 10, 20),
            file("readme.txt", 0, 30, 5),
            file("art/2dart/a.dds", 1, 0, 50),
            FileRecord {
                path_hash: 0x1234,
                bundle: 1,
                offset: 0,
                size: 0,
            },
        ];
        encode(BUNDLES, &files, &reps, &pd.0)
    }

    #[test]
    fn paths_should_rebuild_fragments_in_index_order() {
        let index = Index::parse(&sample_index_bytes(), &Identity).unwrap();
        assert_eq!(
            index.paths().unwrap(),
            [
                "data/stats.datc64",
                "data/mods.datc64",
                "readme.txt",
                "art/2dart/a.dds",
                "art/b.dds",
            ]
        );
        assert_eq!(index.bundles[1].name, "Art/Bundle1");
        assert_eq!(index.bundles[1].decompressed_len, 50);
    }

    #[test]
    fn list_should_match_paths_and_keep_every_unmatched_entry() {
        let index = Index::parse(&sample_index_bytes(), &Identity).unwrap();
        let listing = index.list().unwrap();
        let listed: Vec<(&str, u32, u32)> = listing
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.record.bundle, f.record.offset))
            .collect();
        assert_eq!(
            listed,
            [
                ("art/2dart/a.dds", 1, 0),
                ("data/mods.datc64", 0, 10),
                ("data/stats.datc64", 0, 0),
                ("readme.txt", 0, 30),
            ]
        );
        assert_eq!(listing.paths_without_record, ["art/b.dds"]);
        assert_eq!(listing.records_without_path.len(), 1);
        assert_eq!(listing.records_without_path[0].path_hash, 0x1234);
        assert_eq!(listing.path_reps_without_directory.len(), 1);
        assert_eq!(listing.path_reps_without_directory[0].hash, 0xdead);
    }

    #[test]
    fn list_should_list_a_path_once_per_matching_record() {
        let (pd, reps) = sample_path_data();
        let files = [file("readme.txt", 0, 0, 1), file("readme.txt", 1, 0, 1)];
        let index = Index::parse(&encode(BUNDLES, &files, &reps, &pd.0), &Identity).unwrap();
        let listing = index.list().unwrap();
        let readme: Vec<u32> = listing
            .files
            .iter()
            .filter(|f| f.path == "readme.txt")
            .map(|f| f.record.bundle)
            .collect();
        assert_eq!(readme, [0, 1]);
        assert!(listing.records_without_path.is_empty());
        assert_eq!(listing.paths_without_record.len(), 4);
    }

    fn paths_error(pd: &PathData) -> IndexError {
        let reps = [rep("", 0, pd.len())];
        let index = Index::parse(&encode(BUNDLES, &[], &reps, &pd.0), &Identity).unwrap();
        index.paths().unwrap_err()
    }

    #[test]
    fn paths_should_reject_undefined_prefix() {
        let mut pd = PathData::default();
        pd.toggle().fragment(1, "a/").toggle().fragment(3, "x");
        assert_eq!(
            paths_error(&pd).code(),
            "EXTRACT_INDEX_PATH_FRAGMENT_OUT_OF_RANGE"
        );
    }

    #[test]
    fn paths_should_reject_block_ending_inside_fragment() {
        let mut pd = PathData::default();
        pd.fragment(1, "abc");
        pd.0.pop(); // drop the NUL
        assert_eq!(paths_error(&pd).code(), "EXTRACT_INDEX_PATH_DATA_TRUNCATED");

        let mut pd = PathData::default();
        pd.fragment(1, "abc");
        pd.0.extend_from_slice(&[1, 0]); // half a fragment number
        assert_eq!(paths_error(&pd).code(), "EXTRACT_INDEX_PATH_DATA_TRUNCATED");
    }

    #[test]
    fn paths_should_clear_prefixes_when_reentering_prefix_phase() {
        let mut pd = PathData::default();
        pd.toggle()
            .fragment(1, "a/")
            .toggle()
            .fragment(1, "x")
            .toggle()
            .fragment(1, "b/")
            .toggle()
            .fragment(1, "y");
        let reps = [rep("", 0, pd.len())];
        let index = Index::parse(&encode(BUNDLES, &[], &reps, &pd.0), &Identity).unwrap();
        assert_eq!(index.paths().unwrap(), ["a/x", "b/y"]);

        // After re-entering, the old prefix count no longer applies.
        let mut pd = PathData::default();
        pd.toggle()
            .fragment(1, "a/")
            .fragment(2, "b/")
            .toggle()
            .toggle()
            .fragment(2, "c/");
        assert_eq!(
            paths_error(&pd).code(),
            "EXTRACT_INDEX_PATH_FRAGMENT_OUT_OF_RANGE"
        );
    }

    #[test]
    fn paths_should_reject_overlong_path() {
        let long = "a".repeat(MAX_PATH_LEN / 2);
        let mut pd = PathData::default();
        pd.toggle().fragment(1, &long).toggle().fragment(1, &long);
        let reps = [rep("", 0, pd.len())];
        let index = Index::parse(&encode(BUNDLES, &[], &reps, &pd.0), &Identity).unwrap();
        assert_eq!(index.paths().unwrap()[0].len(), MAX_PATH_LEN);

        // One byte more than the cap: prefix plus a fragment one byte longer.
        pd.fragment(1, &format!("{long}b"));
        assert_eq!(paths_error(&pd).code(), "EXTRACT_INDEX_PATH_TOO_LONG");
    }

    #[test]
    fn parse_should_reject_file_range_overflowing_u32() {
        let (pd, reps) = sample_path_data();
        let bundles = [("Big", u32::MAX)];
        let overflow = [file("readme.txt", 0, u32::MAX, 1)];
        assert_eq!(
            parse_code(&encode(&bundles, &overflow, &reps, &pd.0)),
            "EXTRACT_INDEX_FILE_OUT_OF_BUNDLE"
        );
    }

    #[test]
    fn paths_should_reject_non_utf8_path() {
        let mut pd = PathData::default();
        pd.0.extend_from_slice(&1_u32.to_le_bytes());
        pd.0.extend_from_slice(&[0xff, 0xfe, 0]);
        assert_eq!(paths_error(&pd).code(), "EXTRACT_INDEX_INVALID_UTF8");
    }

    fn parse_code(bytes: &[u8]) -> &'static str {
        Index::parse(bytes, &Identity).unwrap_err().code()
    }

    #[test]
    fn parse_should_reject_bad_file_records() {
        let (pd, reps) = sample_path_data();
        let out_of_range = [file("readme.txt", 2, 0, 1)];
        assert_eq!(
            parse_code(&encode(BUNDLES, &out_of_range, &reps, &pd.0)),
            "EXTRACT_INDEX_BUNDLE_OUT_OF_RANGE"
        );
        let past_end = [file("readme.txt", 1, 40, 11)];
        assert_eq!(
            parse_code(&encode(BUNDLES, &past_end, &reps, &pd.0)),
            "EXTRACT_INDEX_FILE_OUT_OF_BUNDLE"
        );
        let at_end = [file("readme.txt", 1, 40, 10)];
        assert!(Index::parse(&encode(BUNDLES, &at_end, &reps, &pd.0), &Identity).is_ok());
    }

    #[test]
    fn parse_should_reject_path_rep_outside_path_data() {
        let (pd, mut reps) = sample_path_data();
        reps.push(rep("x", pd.len() - 1, 2));
        assert_eq!(
            parse_code(&encode(BUNDLES, &[], &reps, &pd.0)),
            "EXTRACT_INDEX_PATH_REP_OUT_OF_RANGE"
        );
    }

    #[test]
    fn parse_should_reject_non_utf8_bundle_name() {
        let mut bytes = sample_index_bytes();
        bytes[8] = 0xff; // first byte of the first bundle name
        assert_eq!(parse_code(&bytes), "EXTRACT_INDEX_INVALID_UTF8");
    }

    #[test]
    fn parse_should_reject_trailing_bytes_after_path_bundle() {
        let mut bytes = sample_index_bytes();
        bytes.push(0);
        assert_eq!(parse_code(&bytes), "EXTRACT_BUNDLE_TRAILING_BYTES");
    }

    #[test]
    fn parse_should_reject_every_truncation() {
        let bytes = sample_index_bytes();
        for len in 0..bytes.len() {
            assert!(
                Index::parse(&bytes[..len], &Identity).is_err(),
                "truncated to {len}"
            );
        }
    }

    #[test]
    fn parse_and_list_should_not_panic_on_any_single_byte_corruption() {
        let bytes = sample_index_bytes();
        for offset in 0..bytes.len() {
            for value in [0x00, 0x01, 0x7f, 0x80, 0xff, bytes[offset].wrapping_add(1)] {
                let mut corrupt = bytes.clone();
                corrupt[offset] = value;
                if let Ok(index) = Index::parse(&corrupt, &Identity) {
                    // Either an error or a listing; never a panic.
                    let _ = index.list();
                }
            }
        }
    }

    #[test]
    fn parse_should_reject_huge_counts_without_allocating() {
        // Bundle count, then file count (after the two bundle entries).
        for count_offset in [0, 4 + (4 + 12 + 4) + (4 + 11 + 4)] {
            let mut bytes = sample_index_bytes();
            bytes[count_offset..count_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Index::parse(&bytes, &Identity).is_err());
        }
    }
}

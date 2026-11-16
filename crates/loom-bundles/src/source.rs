//! A bundle folder as a [`BundleSource`] (moved from `loom-app::data`).

use std::path::Path;

use loom_data::bundle::{BundleSource, SourceError};

/// A bundle folder on disk.
pub struct DirSource<'d>(pub &'d Path);

impl BundleSource for DirSource<'_> {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                // A link could point out of the bundle: refused.
                if entry.file_type()?.is_symlink() {
                    return Err(std::io::Error::other(format!(
                        "{}: a symbolic link",
                        path.display()
                    )));
                }
                if path.is_dir() {
                    walk(root, &path, out)?;
                } else if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
            Ok(())
        }
        let mut out = Vec::new();
        walk(self.0, self.0, &mut out).map_err(|e| SourceError::Unreadable {
            path: self.0.display().to_string(),
            message: e.to_string(),
        })?;
        out.sort();
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        std::fs::read(self.0.join(path)).map_err(|e| SourceError::Unreadable {
            path: path.to_owned(),
            message: e.to_string(),
        })
    }

    /// The file's length from its metadata; a link or a folder is not a file.
    fn size(&self, path: &str) -> Result<u64, SourceError> {
        let unreadable = |message: String| SourceError::Unreadable {
            path: path.to_owned(),
            message,
        };
        let meta =
            std::fs::symlink_metadata(self.0.join(path)).map_err(|e| unreadable(e.to_string()))?;
        if !meta.file_type().is_file() {
            return Err(unreadable("not a plain file".to_owned()));
        }
        Ok(meta.len())
    }
}

//! The game data the application calculates with (Phase 4 plan,
//! decision 1): one validated bundle folder.

use std::path::{Path, PathBuf};

use loom_data::bundle::{Bundle, BundleSource, SourceError, validate};
use loom_domain::{DataVersion, Game, TreeVersion};
use loom_parser::import::{BundleData, Resolver};

use crate::error::{AppError, Code};

/// The passive tree version of each bundle the project has evidence for,
/// by game and Steam build (`docs/phase-3/calc-scope-poe1.json`; a test
/// keeps the two in step). Another bundle calculates without one, which
/// the engine reports (`unchecked-tree-version`).
pub const TREE_VERSIONS: &[(Game, u64, &str)] = &[(Game::Poe1, 26_530_219, "3_29")];

/// The tree version for a data version, if the table has it.
#[must_use]
pub fn tree_version(version: &DataVersion) -> Option<TreeVersion> {
    let steam = version.steam_build?;
    TREE_VERSIONS
        .iter()
        .find(|(game, build, _)| *game == version.game && *build == steam)
        .map(|(_, _, tree)| TreeVersion::new(*tree))
}

/// A bundle folder on disk.
struct DirSource<'d>(&'d Path);

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
}

/// A loaded bundle with its resolver.
///
/// The bundle, its import view, and the resolver borrow each other, so
/// they are leaked to `'static`: a bundle is loaded once per run (or when
/// the user picks another folder, which leaks the previous one; bounded by
/// what the user does, and far simpler than a self-referential type).
pub struct GameData {
    /// The folder.
    pub dir: PathBuf,
    /// The bundle.
    pub bundle: &'static Bundle,
    /// Its import view.
    pub data: &'static BundleData<'static>,
    /// Its resolver.
    pub resolver: &'static Resolver<'static, 'static>,
    /// Its data version.
    pub version: DataVersion,
    /// The tree version for it, if known.
    pub tree_version: Option<TreeVersion>,
}

impl std::fmt::Debug for GameData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameData")
            .field("dir", &self.dir)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

impl GameData {
    /// Validates and loads the bundle folder `dir`.
    ///
    /// # Errors
    /// `APP_BUNDLE` when the folder is not a valid bundle.
    pub fn load(dir: &Path) -> Result<Self, AppError> {
        let source = DirSource(dir);
        let report = validate::validate(&source);
        if !report.is_valid() {
            return Err(AppError::new(
                Code::Bundle,
                format!("not a valid bundle ({})", report.codes().join(", ")),
            ));
        }
        let bundle = Bundle::load(&source)
            .map_err(|e| AppError::new(Code::Bundle, format!("{}: {e}", e.code())))?;
        let bundle: &'static Bundle = Box::leak(Box::new(bundle));
        let data: &'static BundleData<'static> = Box::leak(Box::new(BundleData::new(bundle)));
        let resolver =
            Resolver::new(data).map_err(|e| AppError::new(Code::Bundle, e.to_string()))?;
        let resolver: &'static Resolver<'static, 'static> = Box::leak(Box::new(resolver));
        let version = loom_parser::import::data_version(data);
        Ok(Self {
            dir: dir.to_owned(),
            bundle,
            data,
            resolver,
            tree_version: tree_version(&version),
            version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_versions_should_match_the_scope_file() {
        let scope = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/phase-3/calc-scope-poe1.json"),
        )
        .unwrap();
        let scope: serde_json::Value = serde_json::from_str(&scope).unwrap();
        let steam = scope["steam_build"].as_u64().unwrap();
        let tree = scope["tree_version"].as_str().unwrap();
        assert!(TREE_VERSIONS.contains(&(Game::Poe1, steam, tree)));
        let version = DataVersion {
            game: Game::Poe1,
            schema_version: 3,
            steam_build: Some(steam),
        };
        assert_eq!(tree_version(&version), Some(TreeVersion::new(tree)));
        let other = DataVersion {
            game: Game::Poe2,
            ..version
        };
        assert_eq!(tree_version(&other), None);
    }
}

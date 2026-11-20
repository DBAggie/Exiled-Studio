//! The game data the application calculates with (Phase 4 plan,
//! decision 1): one validated bundle folder.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use loom_bundles::DirSource;
pub use loom_data::bundle::validate::Integrity;
use loom_data::bundle::{Bundle, BundleSource, validate};
use loom_domain::{DataVersion, Game, TreeVersion};
use loom_parser::import::{BundleData, Resolver};
use sha2::{Digest, Sha256};

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
    /// Its passive tree, when it has one (the tree editor).
    pub tree: Option<Arc<loom_tree::Tree>>,
    /// Its older tree versions, by version (tree packs, T4): a build whose
    /// spec names one is calculated on it ([`loom_pipeline::pick_tree`]).
    pub trees: Arc<std::collections::BTreeMap<String, loom_tree::Tree>>,
    /// The tree's layout DTO, made once.
    pub tree_layout: Option<Arc<loom_serialization::dto::app::v1::TreeLayout>>,
    /// The tree's sprites DTO, made on first use ([`GameData::tree_sprites`]).
    sprites: OnceLock<Option<Arc<loom_serialization::dto::app::v1::TreeSprites>>>,
    /// Each Timeless jewel type's results read so far, by type
    /// ([`GameData::timeless_results`]; P6-10a): `None` when the file did
    /// not read or check.
    timeless: std::sync::Mutex<std::collections::BTreeMap<u32, Option<Arc<Vec<u8>>>>>,
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
    /// Validates and loads the bundle folder `dir`, every file's hash
    /// included.
    ///
    /// # Errors
    /// `APP_BUNDLE` when the folder is not a valid bundle.
    pub fn load(dir: &Path) -> Result<Self, AppError> {
        Self::load_checked(dir, Integrity::Hashes)
    }

    /// Validates and loads the bundle folder `dir`, the tables once. With
    /// [`Integrity::Sizes`] its files' hashes are not checked: for an
    /// installed bundle, checked at install and on Verify (plan decision
    /// P-e). A folder chosen by hand is checked with
    /// [`Integrity::Hashes`].
    ///
    /// # Errors
    /// `APP_BUNDLE` when the folder is not a valid bundle.
    pub fn load_checked(dir: &Path, integrity: Integrity) -> Result<Self, AppError> {
        let source = DirSource(dir);
        // Codes only: a message could name a path, and the error reaches
        // MCP clients (decision 11: no file paths).
        let bundle = validate::check(&source, integrity).map_err(|report| {
            AppError::new(
                Code::Bundle,
                format!("not a valid bundle ({})", report.codes().join(", ")),
            )
        })?;
        let bundle: &'static Bundle = Box::leak(Box::new(bundle));
        let data: &'static BundleData<'static> = Box::leak(Box::new(BundleData::new(bundle)));
        let resolver =
            Resolver::new(data).map_err(|e| AppError::new(Code::Bundle, e.code().to_owned()))?;
        let resolver: &'static Resolver<'static, 'static> = Box::leak(Box::new(resolver));
        let version = loom_parser::import::data_version(data);
        let tree_version = tree_version(&version);
        let tree = loom_tree::Tree::from_bundle(bundle);
        let trees = tree
            .as_ref()
            .map(|t| loom_pipeline::pack_trees(bundle, t))
            .unwrap_or_default();
        let tree_layout = tree.as_ref().map(|t| {
            Arc::new(crate::tree::layout(
                t,
                bundle,
                tree_version.as_ref().map(|v| v.as_str().to_owned()),
            ))
        });
        Ok(Self {
            dir: dir.to_owned(),
            bundle,
            data,
            resolver,
            tree_version,
            version,
            tree: tree.map(Arc::new),
            trees: Arc::new(trees),
            tree_layout,
            sprites: OnceLock::new(),
            timeless: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        })
    }

    /// The Timeless jewels' results of `types` (P6-10a, plan decision T2):
    /// each type's file is read from the folder on its first use, checked
    /// against the manifest's SHA-256 and size, then kept. A type whose
    /// file is missing or does not check is left out, so its nodes stay a
    /// reason (`timeless-results`).
    pub fn timeless_results(
        &self,
        types: &std::collections::BTreeSet<u32>,
    ) -> std::collections::BTreeMap<u32, Arc<Vec<u8>>> {
        use sha2::{Digest, Sha256};
        let Some(luts) = self
            .bundle
            .poe1
            .as_ref()
            .and_then(|p| p.timeless_luts.as_ref())
        else {
            return std::collections::BTreeMap::new();
        };
        let mut cache = self
            .timeless
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out = std::collections::BTreeMap::new();
        for t in types {
            let entry = cache.entry(*t).or_insert_with(|| {
                let jewel = luts.jewel(*t)?;
                let listed = self
                    .bundle
                    .manifest
                    .files
                    .iter()
                    .find(|f| f.path == jewel.file)?;
                let bytes = std::fs::read(self.dir.join(&jewel.file)).ok()?;
                let sha: String = Sha256::digest(&bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                (bytes.len() as u64 == listed.size && sha == listed.sha256).then(|| Arc::new(bytes))
            });
            if let Some(bytes) = entry {
                out.insert(*t, Arc::clone(bytes));
            }
        }
        out
    }

    /// The tree's sprites DTO, when the bundle has sprites and the tree is
    /// placed: made on the first call (the sheets are read only when the
    /// tree screen asks), then kept. Each sheet is read from the folder
    /// again and must still have the manifest's SHA-256 and size, else
    /// there are no sprites (the screen draws its own shapes).
    pub fn tree_sprites(&self) -> Option<Arc<loom_serialization::dto::app::v1::TreeSprites>> {
        self.sprites
            .get_or_init(|| {
                let index = self.bundle.tree_sprites.as_ref()?;
                if !self.tree.as_ref().is_some_and(|t| t.placed()) {
                    return None;
                }
                let source = DirSource(&self.dir);
                let manifest = &self.bundle.manifest;
                crate::tree::sprites(index, |path| {
                    let entry = manifest.files.iter().find(|f| f.path == path)?;
                    let bytes = source.read(path).ok()?;
                    let sha: String = Sha256::digest(&bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    (bytes.len() as u64 == entry.size && sha == entry.sha256).then_some(bytes)
                })
                .map(Arc::new)
            })
            .clone()
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

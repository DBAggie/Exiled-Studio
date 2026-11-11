//! Data bundle format v1 (Phase 2 plan, step 3 and decision 8): the
//! canonical game data the app reads, in our own schema. A bundle is a set
//! of files read through a [`BundleSource`] (ADR-016: core reads bytes,
//! hosts read files):
//!
//! | Path | Content |
//! |---|---|
//! | `bundle.json` | [`Manifest`]: format, game, provenance, every other file with its SHA-256 and size; `signatures` reserved (empty in v1) |
//! | `data/stats.json` | [`Stat`]s |
//! | `data/mods.json` | [`ModEntry`]s (mod rows, by ID) |
//! | `data/base_items.json` | [`BaseItem`]s (with their tags precomputed) |
//! | `data/item_classes.json` | item class IDs |
//! | `data/client_texts.json` | [`ClientText`]s the item rules name |
//! | `data/lookups.json` | [`LookupTable`]s for stat-description handlers |
//! | `data/item_rules.json` | the game's [`ItemRules`] |
//! | `stat_descriptions/*.json` | stat-description files (the extractor's format, [`StatDescFile`]) |
//! | `poe1/*.json` | the Path of Exile only section ([`poe1::Poe1Section`]); never in a PoE2 bundle |
//!
//! JSON files have sorted keys and a stable row order; unknown fields are
//! errors. The loader parses; checking hashes, references, and domain
//! rules is the validator's job (plan step 4). The compressed distribution
//! container and real signatures come in Phase 5 (ADR-010).

pub mod poe1;
mod source;

use loom_domain::Game;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub use self::source::{BundleSource, MemorySource, SourceError};
use crate::rules::ItemRules;
use crate::stat_desc::{LoadError, Lookup, LookupHandler, StatDescFile, StatDescriptions};

/// The bundle formats this loader reads.
pub const SUPPORTED_FORMATS: [u32; 1] = [1];

/// The manifest's path.
pub const MANIFEST: &str = "bundle.json";
/// Stats.
pub const STATS: &str = "data/stats.json";
/// Mod rows.
pub const MODS: &str = "data/mods.json";
/// Base items.
pub const BASE_ITEMS: &str = "data/base_items.json";
/// Item classes.
pub const ITEM_CLASSES: &str = "data/item_classes.json";
/// Client texts.
pub const CLIENT_TEXTS: &str = "data/client_texts.json";
/// Lookup tables.
pub const LOOKUPS: &str = "data/lookups.json";
/// Item rules.
pub const ITEM_RULES: &str = "data/item_rules.json";
/// The folder of stat-description files.
pub const STAT_DESCRIPTIONS_DIR: &str = "stat_descriptions/";

/// The files every bundle has, besides stat descriptions.
pub const REQUIRED_FILES: [&str; 7] = [
    STATS,
    MODS,
    BASE_ITEMS,
    ITEM_CLASSES,
    CLIENT_TEXTS,
    LOOKUPS,
    ITEM_RULES,
];

/// `bundle.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Bundle format version ([`SUPPORTED_FORMATS`]).
    pub format: u32,
    /// Version of the canonical data schema.
    pub data_schema: u32,
    /// `poe1` or `poe2`.
    pub game: String,
    /// Where the data came from.
    pub source: Provenance,
    /// Every file of the bundle but this one, sorted by path.
    pub files: Vec<FileEntry>,
    /// Reserved for signatures (Phase 5); empty in format 1.
    pub signatures: Vec<serde_json::Value>,
}

/// Where a bundle's data came from (ADR-010: builds record their data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Steam build of the install, if known.
    pub steam_build: Option<u64>,
    /// SHA-256 of the install's bundle index.
    pub index_sha256: String,
    /// The extractor's commit.
    pub extractor_commit: String,
    /// The decoder the extractor used, with its version.
    pub decoder: String,
    /// Revision of the community dat schema the extractor read.
    pub dat_schema: String,
}

/// One file of a bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    /// Path inside the bundle (`/`-separated).
    pub path: String,
    /// SHA-256 of the bytes, lowercase hex.
    pub sha256: String,
    /// Size in bytes.
    pub size: u64,
}

impl Manifest {
    /// The bundle's game, if the name is known.
    #[must_use]
    pub fn game(&self) -> Option<Game> {
        [Game::Poe1, Game::Poe2]
            .into_iter()
            .find(|g| g.name() == self.game)
    }
}

/// A stat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stat {
    /// Stat ID.
    pub id: String,
    /// Changes the item's own properties.
    #[serde(default)]
    pub local: bool,
    /// Changes the item's weapon properties.
    #[serde(default)]
    pub weapon_local: bool,
}

/// One stat of a mod row and its range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModStat {
    /// Stat ID.
    pub stat: String,
    /// Lowest value.
    pub min: i64,
    /// Highest value.
    pub max: i64,
}

/// A spawn weight of a mod row on a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnWeight {
    /// Tag.
    pub tag: String,
    /// Weight.
    pub weight: i64,
}

/// A mod row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModEntry {
    /// Mod ID.
    pub id: String,
    /// Domain (the game's number; its meaning is in the item rules).
    pub domain: i64,
    /// Generation type (likewise).
    pub generation: i64,
    /// Mod type, by ID.
    #[serde(default)]
    pub mod_type: Option<String>,
    /// Stats in row order; a `(0, 0)` stat is kept (its value is set
    /// outside the row).
    pub stats: Vec<ModStat>,
    /// Spawn weights in row order.
    #[serde(default)]
    pub spawn: Vec<SpawnWeight>,
    /// Influence value, if any.
    #[serde(default)]
    pub influence: Option<i64>,
    /// Item classes a crafted row is restricted to (empty: none).
    #[serde(default)]
    pub craft_classes: Vec<String>,
    /// Only an essence puts this row on an item.
    #[serde(default)]
    pub essence_only: bool,
    /// Item classes an essence puts this row on, when the data says.
    #[serde(default)]
    pub essence_classes: Option<Vec<String>>,
}

/// A base item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseItem {
    /// Base item ID.
    pub id: String,
    /// Display name (several rows may share one).
    pub name: String,
    /// Mod domain.
    pub domain: i64,
    /// Item class, by ID.
    #[serde(default)]
    pub item_class: Option<String>,
    /// Tags (item type chain plus the row's own), when known.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Implicit mods, by ID.
    #[serde(default)]
    pub implicits: Vec<String>,
}

/// A client text the item rules name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientText {
    /// Text ID.
    pub id: String,
    /// Text.
    pub text: String,
}

/// The names a stat-description lookup handler shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookupTable {
    /// The handler's name as stat-description files write it.
    pub handler: String,
    /// `(key, name)`, sorted by key.
    pub entries: Vec<(i64, String)>,
}

/// A loaded bundle.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// The manifest.
    pub manifest: Manifest,
    /// The game.
    pub game: Game,
    /// Stats.
    pub stats: Vec<Stat>,
    /// Mod rows.
    pub mods: Vec<ModEntry>,
    /// Base items.
    pub base_items: Vec<BaseItem>,
    /// Item class IDs.
    pub item_classes: Vec<String>,
    /// Client texts.
    pub client_texts: Vec<ClientText>,
    /// Lookup tables.
    pub lookups: Vec<LookupTable>,
    /// Item rules.
    pub item_rules: ItemRules,
    /// Stat-description files.
    pub stat_descriptions: StatDescriptions,
    /// The Path of Exile only section, when present.
    pub poe1: Option<poe1::Poe1Section>,
}

/// Why a bundle did not load.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    /// The source could not list or read a file.
    #[error("{0}")]
    Source(#[from] SourceError),
    /// A file is not valid JSON of its type.
    #[error("{path}: {error}")]
    Json {
        /// The file.
        path: String,
        /// The parse error.
        error: serde_json::Error,
    },
    /// A bundle format this loader does not read.
    #[error("bundle format {0} is not supported (supported: {SUPPORTED_FORMATS:?})")]
    UnsupportedFormat(u32),
    /// A game name this loader does not know.
    #[error("unknown game {0:?}")]
    UnknownGame(String),
    /// A file the format requires is not in the manifest.
    #[error("{0}: required, not in the manifest")]
    MissingFile(String),
    /// A stat-description file does not load.
    #[error("{path}: {error}")]
    StatDescription {
        /// The file.
        path: String,
        /// Why.
        error: LoadError,
    },
}

impl BundleError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Source(e) => e.code(),
            Self::Json { .. } => "BUNDLE_JSON",
            Self::UnsupportedFormat(_) => "BUNDLE_FORMAT",
            Self::UnknownGame(_) => "BUNDLE_GAME",
            Self::MissingFile(_) => "BUNDLE_MISSING_FILE",
            Self::StatDescription { .. } => "BUNDLE_STAT_DESCRIPTION",
        }
    }
}

fn json<T: DeserializeOwned>(source: &dyn BundleSource, path: &str) -> Result<T, BundleError> {
    let bytes = source.read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| BundleError::Json {
        path: path.to_owned(),
        error,
    })
}

impl Bundle {
    /// Loads a bundle: the manifest first (format, then game), then every
    /// file it lists. Files the manifest does not list are not read.
    ///
    /// # Errors
    /// [`BundleError`], with the first problem found.
    pub fn load(source: &dyn BundleSource) -> Result<Self, BundleError> {
        #[derive(Deserialize)]
        struct Format {
            format: u32,
        }
        let Format { format } = json(source, MANIFEST)?;
        if !SUPPORTED_FORMATS.contains(&format) {
            return Err(BundleError::UnsupportedFormat(format));
        }
        let manifest: Manifest = json(source, MANIFEST)?;
        let game = manifest
            .game()
            .ok_or_else(|| BundleError::UnknownGame(manifest.game.clone()))?;
        let listed = |path: &str| manifest.files.iter().any(|f| f.path == path);
        if let Some(missing) = REQUIRED_FILES.iter().find(|p| !listed(p)) {
            return Err(BundleError::MissingFile((*missing).to_owned()));
        }
        let mut stat_descriptions = StatDescriptions::new();
        for f in manifest
            .files
            .iter()
            .filter(|f| f.path.starts_with(STAT_DESCRIPTIONS_DIR))
        {
            let bytes = source.read(&f.path)?;
            let file =
                StatDescFile::from_json(&bytes).map_err(|error| BundleError::StatDescription {
                    path: f.path.clone(),
                    error,
                })?;
            stat_descriptions.insert(file);
        }
        let poe1 = if manifest.files.iter().any(|f| f.path.starts_with(poe1::DIR)) {
            Some(poe1::Poe1Section::load(source, &manifest)?)
        } else {
            None
        };
        Ok(Self {
            stats: json(source, STATS)?,
            mods: json(source, MODS)?,
            base_items: json(source, BASE_ITEMS)?,
            item_classes: json(source, ITEM_CLASSES)?,
            client_texts: json(source, CLIENT_TEXTS)?,
            lookups: json(source, LOOKUPS)?,
            item_rules: json(source, ITEM_RULES)?,
            stat_descriptions,
            poe1,
            game,
            manifest,
        })
    }
}

impl Lookup for Bundle {
    fn name(&self, handler: LookupHandler, value: i64) -> Option<String> {
        self.lookups
            .iter()
            .find(|t| t.handler == handler.name())?
            .entries
            .iter()
            .find(|(k, _)| *k == value)
            .map(|(_, n)| n.clone())
    }

    fn entries(&self, handler: LookupHandler) -> Vec<(i64, String)> {
        self.lookups
            .iter()
            .find(|t| t.handler == handler.name())
            .map(|t| t.entries.clone())
            .unwrap_or_default()
    }
}

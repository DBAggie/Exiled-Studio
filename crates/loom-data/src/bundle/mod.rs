//! Data bundle formats 1 and 2 (Phase 2 plan, step 3 and decision 8; Phase 3
//! plan, step 1): the canonical game data the app reads, in our own schema.
//! A bundle is a set of files read through a [`BundleSource`] (ADR-016: core
//! reads bytes, hosts read files). Format 1 holds what item import reads;
//! format 2 adds the calc tables (step 1a: [`skills`]; step 1b:
//! [`passives`]; step 1c: [`properties`]). Item import reads
//! both; the calc needs format 2. Format 2 grew by whole sections in
//! Phase 3 steps 1a to 1c (bundles made before 1c do not load); its
//! sections are complete with 1c, and a later change to them is a new
//! format.
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
//! | `data/skill_types.json`, ... | format 2: the skill tables ([`skills`]) |
//! | `data/characters.json`, ... | format 2: the character and passive tables ([`passives`]) |
//! | `data/weapon_types.json`, ... | format 2: the item property and monster tables ([`properties`]) |
//! | `data/passive_tree_layout.json` | optional: the tree's layout constants ([`tree_layout`]) |
//! | `data/passive_tree_sprites.json`, `trees/assets/*` | optional: the tree's sprites ([`tree_sprites`]) |
//!
//! JSON files have sorted keys and a stable row order; unknown fields are
//! errors. The loader parses; checking hashes, references, and domain
//! rules is the validator's job (`validate`, plan step 4). The compressed distribution
//! container and real signatures come in Phase 5 (ADR-010).

pub mod passives;
pub mod poe1;
pub mod properties;
pub mod skills;
mod source;
pub mod tree_layout;
pub mod tree_sprites;
pub mod validate;

use loom_domain::Game;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize, Serializer};

pub use self::source::{BundleSource, MemorySource, SourceError};
use crate::rules::ItemRules;
use crate::stat_desc::{LoadError, Lookup, LookupHandler, StatDescFile, StatDescriptions};

/// The bundle formats this loader reads.
pub const SUPPORTED_FORMATS: [u32; 2] = [1, 2];

/// The format with the calc tables.
pub const CALC_FORMAT: u32 = 2;

/// Every file the calc sections add in [`CALC_FORMAT`].
pub fn calc_files() -> impl Iterator<Item = &'static str> {
    skills::FILES
        .into_iter()
        .chain(passives::FILES)
        .chain(properties::FILES)
}

/// Writes an `f32` as its shortest text (`2.921`, not the widened
/// `2.9210000038146973`), so the bundle shows the extract's value; it reads
/// back to the same `f32`.
pub(crate) fn short_f32<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    let wide: f64 = v.to_string().parse().map_err(serde::ser::Error::custom)?;
    s.serialize_f64(wide)
}

/// [`short_f32`] for an optional value.
#[expect(
    clippy::ref_option,
    reason = "serde's `serialize_with` passes the field by reference"
)]
pub(crate) fn short_f32_opt<S: Serializer>(v: &Option<f32>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => short_f32(v, s),
        None => s.serialize_none(),
    }
}

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
/// Tag display names (data schema 4; optional, listed when present).
pub const TAGS: &str = "data/tags.json";

/// The first data schema whose mods carry their affix name and level, whose
/// base items carry their drop level and attribute requirements, and that
/// has [`TAGS`] (B-2's tooltip). Readers gate on the manifest's
/// `data_schema`, not on empty fields: an empty name is a mod without one.
pub const AFFIX_SCHEMA: u32 = 4;
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
    /// Reserved for signatures (Phase 5); empty in formats 1 and 2.
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
    /// The mod's implicit tags, by tag ID (the game's `ImplicitTagsKeys`:
    /// `life` makes it a "Life Modifier"). Carried since data schema 3
    /// (P3-8n); empty in an older bundle.
    #[serde(default)]
    pub implicit_tags: Vec<String>,
    /// The affix name the game shows (`Urchin's`, `of the Lost`); empty
    /// when it has none. Carried since data schema 4 ([`AFFIX_SCHEMA`]).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// The mod's level (the lowest item level it rolls on). Since data
    /// schema 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<i64>,
}

/// A base item's attribute requirements (the game's
/// `ComponentAttributeRequirements`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeRequirements {
    /// Strength.
    pub strength: i64,
    /// Dexterity.
    pub dexterity: i64,
    /// Intelligence.
    pub intelligence: i64,
}

/// A tag's display name, as the tooltip's modifier headers print it
/// (`Life`, `Defences`): PoE1's `Tags.Name`, where a tag the Alt view hides
/// has none (Q-B2-2). Bundles built before that read `Tags.DisplayString`,
/// which holds monster labels only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagName {
    /// Tag ID.
    pub id: String,
    /// Display name.
    pub display: String,
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
    /// The level it drops from. Since data schema 4 ([`AFFIX_SCHEMA`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_level: Option<i64>,
    /// Its attribute requirements, when the game lists any. Since data
    /// schema 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<AttributeRequirements>,
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
    /// Tag display names, sorted by ID (data schema 4; empty without
    /// [`TAGS`]).
    pub tags: Vec<TagName>,
    /// Stat-description files.
    pub stat_descriptions: StatDescriptions,
    /// The Path of Exile only section, when present.
    pub poe1: Option<poe1::Poe1Section>,
    /// The skill tables (format 2; `None` in format 1).
    pub skills: Option<skills::SkillTables>,
    /// The character and passive tables (format 2; `None` in format 1).
    pub passives: Option<passives::PassiveTables>,
    /// The item property and monster tables (format 2; `None` in format 1).
    pub properties: Option<properties::PropertyTables>,
    /// The passive tree's layout constants, when listed
    /// ([`tree_layout::TREE_LAYOUT`]; optional).
    pub tree_layout: Option<tree_layout::TreeLayout>,
    /// The passive tree's sprite index, when listed
    /// ([`tree_sprites::TREE_SPRITES`]; optional). Its sheets are read by
    /// path, not loaded.
    pub tree_sprites: Option<tree_sprites::TreeSprites>,
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
    /// A manifest path that is not a plain relative path (`..`, a leading
    /// `/`, a `\\`, a drive, an empty segment): a source never reads it.
    #[error("{0}: not a plain relative bundle path")]
    BadPath(String),
    /// A path listed twice in the manifest.
    #[error("{0}: listed twice")]
    DuplicatePath(String),
    /// A section of another game (a `poe1/` file in a PoE2 bundle).
    #[error("{0}: a {1} bundle has no such section")]
    GameSection(String, &'static str),
    /// A file of a later format (a calc table in a format-1 bundle).
    #[error("{0}: a format-{1} bundle has no such file")]
    FormatFile(String, u32),
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
            Self::BadPath(_) => "BUNDLE_BAD_PATH",
            Self::DuplicatePath(_) => "BUNDLE_DUPLICATE_PATH",
            Self::GameSection(..) => "BUNDLE_GAME_SECTION",
            Self::FormatFile(..) => "BUNDLE_FORMAT_FILE",
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

/// Whether `path` is a plain relative bundle path: `/`-separated segments,
/// none empty, `.` or `..`, and no `\\` or `:` anywhere.
fn plain_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

impl Bundle {
    /// Loads a bundle: the manifest first (format, then game, then its
    /// paths), then every file it lists. Files the manifest does not list
    /// are not read.
    ///
    /// **Unverified:** this parses; it does not check hashes, sizes,
    /// references, or unlisted files. The app loads a bundle only after
    /// the validator accepts it (plan step 4; required before step 6 uses
    /// bundles).
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
        let mut seen = std::collections::BTreeSet::new();
        for f in &manifest.files {
            if !plain_path(&f.path) {
                return Err(BundleError::BadPath(f.path.clone()));
            }
            if !seen.insert(f.path.as_str()) {
                return Err(BundleError::DuplicatePath(f.path.clone()));
            }
            if game != Game::Poe1 && f.path.starts_with(poe1::DIR) {
                return Err(BundleError::GameSection(f.path.clone(), game.name()));
            }
            if format < CALC_FORMAT && calc_files().any(|c| c == f.path) {
                return Err(BundleError::FormatFile(f.path.clone(), format));
            }
        }
        let listed = |path: &str| manifest.files.iter().any(|f| f.path == path);
        let calc: Vec<&str> = if format >= CALC_FORMAT {
            calc_files().collect()
        } else {
            Vec::new()
        };
        if let Some(missing) = REQUIRED_FILES.iter().chain(&calc).find(|p| !listed(p)) {
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
        let (skills, passives, properties) = if format >= CALC_FORMAT {
            (
                Some(skills::SkillTables::load(source)?),
                Some(passives::PassiveTables::load(source)?),
                Some(properties::PropertyTables::load(source)?),
            )
        } else {
            (None, None, None)
        };
        Ok(Self {
            stats: json(source, STATS)?,
            mods: json(source, MODS)?,
            base_items: json(source, BASE_ITEMS)?,
            item_classes: json(source, ITEM_CLASSES)?,
            client_texts: json(source, CLIENT_TEXTS)?,
            lookups: json(source, LOOKUPS)?,
            item_rules: json(source, ITEM_RULES)?,
            tags: if manifest.files.iter().any(|f| f.path == TAGS) {
                json(source, TAGS)?
            } else {
                Vec::new()
            },
            stat_descriptions,
            poe1,
            skills,
            passives,
            properties,
            tree_layout: if listed(tree_layout::TREE_LAYOUT) {
                Some(json(source, tree_layout::TREE_LAYOUT)?)
            } else {
                None
            },
            tree_sprites: if listed(tree_sprites::TREE_SPRITES) {
                Some(json(source, tree_sprites::TREE_SPRITES)?)
            } else {
                None
            },
            game,
            manifest,
        })
    }
}

/// Why a scope of the item rules has no root file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScopeRootError {
    /// No stat-description file has the scope's name.
    #[error("no stat-description file")]
    None,
    /// Several files have it.
    #[error("{0} files have the name")]
    Several(usize),
}

/// The level a base requires, by PoB's exporter's rule
/// (`v2.67.2:src/Export/Scripts/bases.lua:277-293`), which the game's
/// tooltips agree with (B-2 item evidence: a jewel requires none): the drop
/// level when above 4 for a weapon or armour and above 2 for a flask, else
/// 1; raised to `floor(level x 0.8)` of each implicit; none when 1.
#[must_use]
pub fn required_level(
    drop_level: i64,
    weapon_or_armour: bool,
    flask: bool,
    implicit_levels: impl IntoIterator<Item = i64>,
) -> Option<i64> {
    let mut level = 1;
    if weapon_or_armour && drop_level > 4 {
        level = drop_level;
    }
    if flask && drop_level > 2 {
        level = drop_level;
    }
    for implicit in implicit_levels {
        // Lua's math.floor(mod.Level * 0.8), in the same f64 arithmetic.
        #[expect(clippy::cast_precision_loss, reason = "game levels")]
        #[expect(clippy::cast_possible_truncation, reason = "floored game levels")]
        let scaled = (implicit as f64 * 0.8).floor() as i64;
        level = level.max(scaled);
    }
    (level > 1).then_some(level)
}

impl Bundle {
    /// The level `base` requires ([`required_level`]): a weapon or armour
    /// base is one with a row in the property tables; a flask one whose
    /// item class ends in `Flask` (as PoB's exporter tests the game's
    /// flask table). None when the bundle has no drop levels (before data
    /// schema 4).
    #[must_use]
    pub fn required_level(&self, base: &BaseItem) -> Option<i64> {
        let drop_level = base.drop_level?;
        let weapon_or_armour = self.properties.as_ref().is_some_and(|p| {
            p.weapon_types.iter().any(|w| w.base_item == base.id)
                || p.armour_types.iter().any(|a| a.base_item == base.id)
        });
        let flask = base
            .item_class
            .as_deref()
            .is_some_and(|c| c.ends_with("Flask"));
        let implicits = base
            .implicits
            .iter()
            .filter_map(|id| self.mods.iter().find(|m| m.id == *id).and_then(|m| m.level));
        required_level(drop_level, weapon_or_armour, flask, implicits)
    }

    /// The source path of the stat-description file whose name, without
    /// folder or extension, is `scope` in any case (PoE1 writes `.txt`
    /// sources, PoE2 `.csd`): the root of the scope's chain.
    ///
    /// # Errors
    /// [`ScopeRootError`] unless exactly one file has the name.
    pub fn scope_root(&self, scope: &str) -> Result<&str, ScopeRootError> {
        let stem = |source: &str| -> String {
            let name = source.rsplit('/').next().unwrap_or(source);
            name.split_once('.')
                .map_or(name, |(s, _)| s)
                .to_ascii_lowercase()
        };
        let want = scope.to_ascii_lowercase();
        let roots: Vec<&str> = self
            .stat_descriptions
            .files()
            .map(|(_, f)| f.source.as_str())
            .filter(|s| stem(s) == want)
            .collect();
        match roots.as_slice() {
            [root] => Ok(root),
            [] => Err(ScopeRootError::None),
            more => Err(ScopeRootError::Several(more.len())),
        }
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

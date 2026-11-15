//! The Path of Exile only section of a bundle (`poe1/`; ADR-001, Phase 2
//! plan decision 5): data the PoE1 item pipeline reads that PoE2 does not
//! have. Never required by the loader; a PoE2 bundle has none (the
//! validator refuses it there, plan step 4).
//!
//! - `poe1/quality_types.json`: the alternate quality types
//!   (`AlternateQualityTypes`), each with its catalyst mod's fixed stats.
//! - `poe1/grafts.json`: the graft bases (`BrequelGrafts`) with the skill
//!   each grants, and the allowed skill levels
//!   (`BrequelGraftGrantedSkillLevels`). Essences need no file: their item
//!   classes are precomputed into the mod rows (`essence_classes`).
//! - `poe1/pantheons.json`: the pantheon's gods (`PantheonPanelLayout`,
//!   rows not disabled), each with its souls and their stats (P3-12g; the
//!   calc applies a chosen god's souls).
//! - `poe1/jewel_radii.json`: the jewel radii (`PassiveJewelRadii`), each
//!   with its name, radius, and ring (passive tree plan, T-f: which nodes
//!   a radius jewel reaches).
//! - `poe1/unique_flavour.json`: unique items' flavour text (`FlavourText`,
//!   found through `UniqueStashLayout`; B-2's item card, Q-B2-1), each with
//!   the unique's name and the text's ID, as the game writes it.

use serde::{Deserialize, Serialize};

use super::{BundleError, BundleSource, Manifest, json};

/// The section's folder.
pub const DIR: &str = "poe1/";
/// Quality types.
pub const QUALITY_TYPES: &str = "poe1/quality_types.json";
/// Grafts.
pub const GRAFTS: &str = "poe1/grafts.json";
/// Pantheons.
pub const PANTHEONS: &str = "poe1/pantheons.json";
/// Jewel radii.
pub const JEWEL_RADII: &str = "poe1/jewel_radii.json";
/// Unique flavour text.
pub const UNIQUE_FLAVOUR: &str = "poe1/unique_flavour.json";

/// A unique item's flavour text. The game links them by a name convention,
/// not a key (`docs/ui/b2-data/card-data.md`): the unique's art
/// (`ItemVisualIdentity.Id`) names the `FlavourText` row, and a per-game
/// list names the rest (upgraded versions that share their base's art).
/// A name may have several rows: a unique whose variants have their own
/// text (Grand Spectrum).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueFlavour {
    /// The unique's name (`Words.Text2`).
    pub name: String,
    /// The `FlavourText` row's ID.
    pub id: String,
    /// The text as the game stores it, with line breaks as `\n` (the
    /// table's `\r\n` made `\n`) and the game's markup (`<default>{...}`,
    /// `<<...>>` glyphs) kept.
    pub text: String,
}

/// A jewel radius (`PassiveJewelRadii`), in the game table's units: the
/// tree before 3.16's, which `loom_tree::poe1::radius::scaled` turns into
/// the layout's tree units (times 1.2). A jewel with this radius reaches
/// the nodes whose distance from its socket is at most `radius`; a ring
/// jewel (Thread of Hope) the nodes from `ring_inner` to `ring_outer`,
/// both included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JewelRadius {
    /// ID.
    pub id: String,
    /// The name an item's `Radius:` line shows (`ClientStrings`); empty
    /// when the extract has none.
    pub name: String,
    /// The radius.
    pub radius: i64,
    /// The ring's inner radius.
    pub ring_inner: i64,
    /// The ring's outer radius.
    pub ring_outer: i64,
}

/// A god of the pantheon (`PantheonPanelLayout`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pantheon {
    /// ID (`TheBrineKing`, ...): the name PoB's config stores.
    pub id: String,
    /// A major god (else a minor one).
    pub major: bool,
    /// The god's souls with stats, in the table's order (the god's own
    /// soul first, then its upgrades).
    pub souls: Vec<PantheonSoul>,
}

/// One soul of a god: its name and stats (`Effect<n>_StatsKeys` with
/// `Effect<n>_Values`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PantheonSoul {
    /// The soul's name (`GodName<n>`).
    pub name: String,
    /// The soul's stats (`(stat, value)`), in the table's order.
    pub stats: Vec<(String, i64)>,
}

/// An alternate quality type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityType {
    /// ID.
    pub id: String,
    /// The description an item line shows.
    pub description: String,
    /// The catalyst mod's fixed stats (`(stat, value)`).
    #[serde(default)]
    pub stats: Vec<(String, i64)>,
}

/// A graft base and the skill it grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraftBase {
    /// The base item's name.
    pub base: String,
    /// The graft's ID.
    pub graft: String,
    /// The granted skill gem's ID.
    pub gem: String,
    /// The granted skill's name (the gem's base item name).
    pub skill: String,
}

/// `poe1/grafts.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grafts {
    /// Graft bases.
    pub bases: Vec<GraftBase>,
    /// Allowed skill levels.
    pub levels: Vec<u32>,
}

/// The section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poe1Section {
    /// Quality types (`None` when the file is absent).
    pub quality_types: Option<Vec<QualityType>>,
    /// Grafts (`None` when the file is absent).
    pub grafts: Option<Grafts>,
    /// Pantheons (`None` when the file is absent).
    pub pantheons: Option<Vec<Pantheon>>,
    /// Jewel radii (`None` when the file is absent).
    pub jewel_radii: Option<Vec<JewelRadius>>,
    /// Unique flavour text (`None` when the file is absent).
    pub unique_flavour: Option<Vec<UniqueFlavour>>,
}

impl Poe1Section {
    /// The stats the section's build choices (the pantheon's souls) name,
    /// each with the row it is on: the validator checks each is a stat.
    pub(super) fn choice_stats(&self) -> Vec<(String, &str)> {
        let mut out = Vec::new();
        for g in self.pantheons.iter().flatten() {
            for (s, _) in g.souls.iter().flat_map(|soul| &soul.stats) {
                out.push((format!("{PANTHEONS}:{}", g.id), s.as_str()));
            }
        }
        out
    }

    /// The pantheon's domain checks for the validator: the file, its IDs
    /// (each must be unique and not empty), and the rows whose shape is
    /// wrong (a soul without a name or a stat: the converter keeps only
    /// souls with stats, as PoB's export does).
    pub(super) fn choice_domain(&self) -> (&'static str, Vec<&str>, Vec<(String, &'static str)>) {
        let gods = self.pantheons.as_deref().unwrap_or_default();
        let ids = gods.iter().map(|g| g.id.as_str()).collect();
        let bad = gods
            .iter()
            .filter(|g| {
                g.souls
                    .iter()
                    .any(|s| s.name.is_empty() || s.stats.is_empty())
            })
            .map(|g| {
                (
                    format!("{PANTHEONS}:{}", g.id),
                    "a soul without a name or a stat",
                )
            })
            .collect();
        (PANTHEONS, ids, bad)
    }

    /// The jewel radii's domain checks for the validator: unique IDs, and
    /// the rows whose values are out of range (a negative radius, or a
    /// ring whose inner radius is above its outer one).
    pub(super) fn radius_domain(&self) -> (Vec<&str>, Vec<(String, &'static str)>) {
        let radii = self.jewel_radii.as_deref().unwrap_or_default();
        let ids = radii.iter().map(|r| r.id.as_str()).collect();
        let bad = radii
            .iter()
            .filter(|r| r.radius < 0 || r.ring_inner < 0 || r.ring_inner > r.ring_outer)
            .map(|r| {
                (
                    format!("{JEWEL_RADII}:{}", r.id),
                    "a negative radius, or a ring whose inner radius is above its outer one",
                )
            })
            .collect();
        (ids, bad)
    }

    /// Unique flavour text's domain checks for the validator: each row's
    /// key (`name:id`, each must be unique), the number of rows without a
    /// name or an ID, and the rows with no text.
    pub(super) fn flavour_domain(&self) -> (Vec<String>, usize, Vec<(String, &'static str)>) {
        let rows = self.unique_flavour.as_deref().unwrap_or_default();
        let keys = rows
            .iter()
            .map(|r| format!("{}:{}", r.name, r.id))
            .collect();
        let empty = rows
            .iter()
            .filter(|r| r.name.is_empty() || r.id.is_empty())
            .count();
        let bad = rows
            .iter()
            .filter(|r| r.text.trim().is_empty())
            .map(|r| (format!("{UNIQUE_FLAVOUR}:{}:{}", r.name, r.id), "no text"))
            .collect();
        (keys, empty, bad)
    }

    /// The flavour text rows of the unique named `name`, in the file's
    /// order (none without the file).
    pub fn flavour_of<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a UniqueFlavour> {
        self.unique_flavour
            .iter()
            .flatten()
            .filter(move |r| r.name == name)
    }

    /// Reads the section's files the manifest lists.
    ///
    /// # Errors
    /// [`BundleError`] when a listed file does not read or parse.
    pub(super) fn load(
        source: &dyn BundleSource,
        manifest: &Manifest,
    ) -> Result<Self, BundleError> {
        let listed = |path: &str| manifest.files.iter().any(|f| f.path == path);
        Ok(Self {
            quality_types: if listed(QUALITY_TYPES) {
                Some(json(source, QUALITY_TYPES)?)
            } else {
                None
            },
            grafts: if listed(GRAFTS) {
                Some(json(source, GRAFTS)?)
            } else {
                None
            },
            pantheons: if listed(PANTHEONS) {
                Some(json(source, PANTHEONS)?)
            } else {
                None
            },
            jewel_radii: if listed(JEWEL_RADII) {
                Some(json(source, JEWEL_RADII)?)
            } else {
                None
            },
            unique_flavour: if listed(UNIQUE_FLAVOUR) {
                Some(json(source, UNIQUE_FLAVOUR)?)
            } else {
                None
            },
        })
    }
}

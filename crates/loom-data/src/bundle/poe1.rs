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
        })
    }
}

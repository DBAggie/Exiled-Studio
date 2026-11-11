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

use serde::{Deserialize, Serialize};

use super::{BundleError, BundleSource, Manifest, json};

/// The section's folder.
pub const DIR: &str = "poe1/";
/// Quality types.
pub const QUALITY_TYPES: &str = "poe1/quality_types.json";
/// Grafts.
pub const GRAFTS: &str = "poe1/grafts.json";

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
}

impl Poe1Section {
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
        })
    }
}

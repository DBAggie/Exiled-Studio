//! Tattoos, runegrafts, and the other passive skill overrides (Phase 6
//! P6-11av): `poe1/tattoos.json`, from the game's `PassiveSkillOverrides`
//! (our extraction; PoB v2.67.2 exports `Data/TattooPassives.lua` from the
//! same table, `src/Export/Scripts/tattooPassives.lua`).
//!
//! A spec's `<Override nodeId=".." dn="..">` names one by its name (PoB's
//! `dn`, the table's `Name`): the node takes the override's stats in place
//! of its own. A keystone tattoo's name and stats are its passive skill's
//! (`AllocatedPassiveSkill`), as PoB's exporter reads them.

use serde::{Deserialize, Serialize};

use super::passives::StatAmount;

/// The file.
pub const TATTOOS: &str = "poe1/tattoos.json";

/// One passive skill override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    /// Its ID (`Id`).
    pub id: String,
    /// Its name (`Name`; a keystone tattoo's is its passive's), what a
    /// spec's override names.
    pub name: String,
    /// Its type (`Type`, a `PassiveSkillOverrideTypes` ID).
    pub kind: String,
    /// Its stats and values (`Stats`, `StatValues`; a keystone tattoo's
    /// passive's).
    pub stats: Vec<StatAmount>,
    /// The passive skill a keystone tattoo grants (`AllocatedPassiveSkill`,
    /// a `PassiveSkills` ID), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passive: Option<String>,
    /// Whether it grants a skill (`Effect` set); the calculation does not
    /// model one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub grants_skill: bool,
}

/// The overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tattoos {
    /// In the table's order.
    pub overrides: Vec<Override>,
}

impl Tattoos {
    /// The override a spec names; the last of that name, as PoB's
    /// exporter keys its table by name in the table's order.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Override> {
        self.overrides.iter().rev().find(|o| o.name == name)
    }
}

//! Older passive tree versions (ADR-005 Amendment 2; passive tree plan,
//! T-d and T4): a bundle carries the game's own tree in its passive tables
//! and older versions as **tree packs**, each converted from GGG's tree
//! export at the commit pinned for its version in `tools/tree/manifest.json`
//! (`loom-parity-cli tree-pack`).
//!
//! | Path | Content |
//! |---|---|
//! | `trees/<version>.json` | a [`TreePack`]: one version's passives, graph, and mastery effects, with the export it came from |
//!
//! Optional, each listed when present; `<version>` is a tree version as
//! builds name it (`3_28`). A pack holds what changes between versions:
//! - the passives, with their stats as stat IDs, resolved from the
//!   export's text through the bundle's passive stat descriptions
//!   (ADR-015);
//! - the graph (groups, nodes, links, class starts);
//! - the masteries and their effects.
//!
//! The classes, ascendancies, and object templates are the bundle's own:
//! a pack names an ascendancy by the bundle's ID. A line that did not
//! resolve to one reading is kept as text in [`TreePack::unresolved`]: the
//! calculation gives its node a reason, never a guess.

use serde::{Deserialize, Serialize};

use super::passives::{MasteryEffect, MasteryGroup, Passive, PassiveTree};
use super::tree_layout::ExportFile;

/// The packs' folder.
pub const DIR: &str = "trees/";

/// The path of `version`'s pack.
#[must_use]
pub fn path(version: &str) -> String {
    format!("{DIR}{version}.json")
}

/// The version a listed path is the pack of, if it is one: a file directly
/// in [`DIR`] named `<major>_<minor>.json`.
#[must_use]
pub fn version_of(path: &str) -> Option<&str> {
    let name = path.strip_prefix(DIR)?.strip_suffix(".json")?;
    let (major, minor) = name.split_once('_')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(major) && digits(minor)).then_some(name)
}

/// Where a pack came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackSource {
    /// The export's repository.
    pub repository: String,
    /// The tag read.
    pub tag: String,
    /// The commit read (the pinned one).
    pub commit: String,
    /// The file read, with its SHA-256.
    pub file: ExportFile,
}

/// A line of the export a pack could not read as stats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackLine {
    /// The passive's node number, or the mastery effect's hash.
    pub node: i64,
    /// Whether it is a mastery effect's line.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mastery_effect: bool,
    /// The line as the export writes it.
    pub text: String,
    /// Why it did not read: `no-template`, `ambiguous`, or `unsettled` (a
    /// value the text does not settle).
    pub reason: String,
}

/// One tree version (module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreePack {
    /// The version, as builds name it (`3_28`).
    pub version: String,
    /// Where it came from.
    pub source: PackSource,
    /// Its passives, cluster templates included.
    pub passives: Vec<Passive>,
    /// Its masteries' groups.
    pub mastery_groups: Vec<MasteryGroup>,
    /// Its masteries' effects.
    pub mastery_effects: Vec<MasteryEffect>,
    /// Its graph.
    pub tree: PassiveTree,
    /// The lines that did not read, in export order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<PackLine>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_of_should_read_only_pack_paths() {
        assert_eq!(version_of("trees/3_28.json"), Some("3_28"));
        assert_eq!(version_of(&path("3_25")), Some("3_25"));
        for p in [
            "trees/assets/a.json",
            "trees/3_28.png",
            "trees/x_1.json",
            "trees/3_.json",
            "data/3_28.json",
        ] {
            assert_eq!(version_of(p), None, "{p}");
        }
    }
}

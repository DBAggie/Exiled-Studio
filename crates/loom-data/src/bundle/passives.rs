//! The character and passive tables of bundle format 2 (Phase 3 plan, step
//! 1b), shared by both games, in our own types and names:
//!
//! | Path | Content |
//! |---|---|
//! | `data/characters.json` | [`Character`]s (classes) |
//! | `data/ascendancies.json` | [`Ascendancy`]s |
//! | `data/object_templates.json` | [`ObjectTemplate`]s: the `Stats` block of the character and monster templates (per-level growth, caps, ...) |
//! | `data/passives.json` | [`Passive`]s (every `PassiveSkills` row) |
//! | `data/mastery_groups.json` | [`MasteryGroup`]s |
//! | `data/mastery_effects.json` | [`MasteryEffect`]s |
//! | `data/passive_tree.json` | the [`PassiveTree`]: the game's main passive graph |
//!
//! Values are the game's, unscaled. A template's chain (`extends`) is not
//! resolved here; the calc applies it (a class template overrides
//! `Metadata/Characters/Character`); the validator refuses a chain that
//! loops. A key repeated in one `Stats` block is kept in file order (none
//! in PoE1 26530219 or PoE2 26476662). PoE2 classes have no own template.
//!
//! The bundle holds one passive tree, the one of the game build in the
//! manifest's provenance (`source.steam_build`); that build is the tree's
//! identity. Which PoB tree version label it answers to is decided with
//! evidence where the calc needs it (P3-3). Connections are undirected and
//! stored once, as the graph file stores them (PoE1 3,359, PoE2 6,070; one
//! PoE2 pair both ways). Node positions need orbit radii, which are not in
//! the game data read here; a rule that needs distances brings them with
//! their evidence (connectivity needs none).
//!
//! Not carried (no Phase 3 calc reads them; adding one is a new field): the
//! templates' other blocks (movement speed in `Pathfinding`, render and
//! action blocks), passive buffs and passive-granted skills, PoE1 Ruthless
//! passive values, PoE2 attribute and weapon-set passives' extra columns,
//! alternate ascendancy and atlas graphs, the graph's unnamed group fields.

use serde::{Deserialize, Serialize};

use super::{BundleError, BundleSource, json, short_f32};

/// Characters.
pub const CHARACTERS: &str = "data/characters.json";
/// Ascendancies.
pub const ASCENDANCIES: &str = "data/ascendancies.json";
/// Object templates.
pub const OBJECT_TEMPLATES: &str = "data/object_templates.json";
/// Passives.
pub const PASSIVES: &str = "data/passives.json";
/// Mastery groups.
pub const MASTERY_GROUPS: &str = "data/mastery_groups.json";
/// Mastery effects.
pub const MASTERY_EFFECTS: &str = "data/mastery_effects.json";
/// The passive tree.
pub const PASSIVE_TREE: &str = "data/passive_tree.json";

/// The section's files, each required in format 2.
pub const FILES: [&str; 7] = [
    CHARACTERS,
    ASCENDANCIES,
    OBJECT_TEMPLATES,
    PASSIVES,
    MASTERY_GROUPS,
    MASTERY_EFFECTS,
    PASSIVE_TREE,
];

/// A stat and its value (not `stat_desc::StatValue`, a rendering input).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatAmount {
    /// Stat ID.
    pub stat: String,
    /// Value.
    pub value: i64,
}

/// A character class (`Characters`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    /// ID (the game's object path, `Metadata/Characters/Str/Str`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// The class number builds name it by (PoB's `classId`).
    pub class_number: i64,
    /// Base maximum life.
    pub base_life: i64,
    /// Base maximum mana.
    pub base_mana: i64,
    /// Base strength.
    pub strength: i64,
    /// Base dexterity.
    pub dexterity: i64,
    /// Base intelligence.
    pub intelligence: i64,
    /// The class's own object template, by ID, when the game has one
    /// (PoE1); its chain leads to the character template.
    #[serde(default)]
    pub template: Option<String>,
}

/// An ascendancy (`Ascendancy`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ascendancy {
    /// ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// The character class, by ID.
    #[serde(default)]
    pub character: Option<String>,
    /// The ascendancy's number within its class (PoB's `ascendClassId`).
    pub class_number: i64,
}

/// A `Stats` entry of an object template that is no stat of the bundle,
/// or no integer, kept as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateValue {
    /// Key.
    pub key: String,
    /// Value as written.
    pub value: String,
}

/// An object template (`*.ot`): its `Stats` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectTemplate {
    /// ID: the lowercase path without extension
    /// (`metadata/characters/character`).
    pub id: String,
    /// The template it extends, by ID (`None` at the root).
    #[serde(default)]
    pub extends: Option<String>,
    /// `Stats` entries that are stats with integer values, in file order.
    #[serde(default)]
    pub stats: Vec<StatAmount>,
    /// Other `Stats` entries, in file order.
    #[serde(default)]
    pub values: Vec<TemplateValue>,
}

/// A passive skill (`PassiveSkills`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Passive {
    /// ID.
    pub id: String,
    /// The node number builds and the tree name it by
    /// (`PassiveSkillGraphId`).
    pub node: i64,
    /// Display name.
    pub name: String,
    /// Stats and values.
    #[serde(default)]
    pub stats: Vec<StatAmount>,
    /// A keystone.
    #[serde(default)]
    pub keystone: bool,
    /// A notable.
    #[serde(default)]
    pub notable: bool,
    /// A jewel socket.
    #[serde(default)]
    pub jewel_socket: bool,
    /// A cluster-jewel (expansion) passive.
    #[serde(default)]
    pub expansion: bool,
    /// Only granted by an anointment.
    #[serde(default)]
    pub anoint_only: bool,
    /// A choice node (its options are `multiple_choice_option`).
    #[serde(default)]
    pub multiple_choice: bool,
    /// An option of a choice node.
    #[serde(default)]
    pub multiple_choice_option: bool,
    /// Passive skill points it grants.
    #[serde(default)]
    pub skill_points: i64,
    /// Its mastery group, by ID.
    #[serde(default)]
    pub mastery_group: Option<String>,
    /// Its ascendancy, by ID.
    #[serde(default)]
    pub ascendancy: Option<String>,
    /// The ascendancy's start node.
    #[serde(default)]
    pub ascendancy_start: bool,
    /// The classes it is a start node of, by ID.
    #[serde(default)]
    pub characters: Vec<String>,
}

/// A mastery group (`PassiveSkillMasteryGroups`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MasteryGroup {
    /// ID.
    pub id: String,
    /// Its effects, by ID.
    #[serde(default)]
    pub effects: Vec<String>,
}

/// A mastery effect (`PassiveSkillMasteryEffects`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MasteryEffect {
    /// ID.
    pub id: String,
    /// The number builds name it by (`HASH16`).
    pub hash: i64,
    /// Stats and values.
    #[serde(default)]
    pub stats: Vec<StatAmount>,
}

/// A connection from a tree node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeConnection {
    /// The other node.
    pub node: i64,
    /// The arc's orbit as the graph stores it (`None`: straight).
    #[serde(default)]
    pub orbit: Option<i64>,
}

/// A node of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNode {
    /// The node number (a [`Passive::node`]).
    pub node: i64,
    /// Orbit.
    pub orbit: i64,
    /// Position on the orbit.
    pub orbit_index: i64,
    /// Connections, in graph order.
    #[serde(default)]
    pub connections: Vec<TreeConnection>,
}

/// A group of tree nodes around one centre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeGroup {
    /// Centre x.
    #[serde(serialize_with = "short_f32")]
    pub x: f32,
    /// Centre y.
    #[serde(serialize_with = "short_f32")]
    pub y: f32,
    /// Flags as stored.
    #[serde(default)]
    pub flags: i64,
    /// Nodes.
    #[serde(default)]
    pub nodes: Vec<TreeNode>,
}

/// The game's main passive tree (`PassiveSkillTrees` row `Default` and its
/// graph file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveTree {
    /// The `PassiveSkillTrees` ID.
    pub tree: String,
    /// The graph file it names.
    pub graph: String,
    /// Positions per orbit.
    #[serde(default)]
    pub orbit_slots: Vec<i64>,
    /// Root nodes (class starts), by node number.
    #[serde(default)]
    pub roots: Vec<i64>,
    /// Groups.
    #[serde(default)]
    pub groups: Vec<TreeGroup>,
}

/// The character and passive tables.
#[derive(Debug, Clone, PartialEq)]
pub struct PassiveTables {
    /// Characters.
    pub characters: Vec<Character>,
    /// Ascendancies.
    pub ascendancies: Vec<Ascendancy>,
    /// Object templates.
    pub object_templates: Vec<ObjectTemplate>,
    /// Passives.
    pub passives: Vec<Passive>,
    /// Mastery groups.
    pub mastery_groups: Vec<MasteryGroup>,
    /// Mastery effects.
    pub mastery_effects: Vec<MasteryEffect>,
    /// The passive tree.
    pub tree: PassiveTree,
}

impl PassiveTables {
    /// Reads the section's files (the loader has checked they are listed).
    ///
    /// # Errors
    /// [`BundleError`] when a file does not read or parse.
    pub(super) fn load(source: &dyn BundleSource) -> Result<Self, BundleError> {
        Ok(Self {
            characters: json(source, CHARACTERS)?,
            ascendancies: json(source, ASCENDANCIES)?,
            object_templates: json(source, OBJECT_TEMPLATES)?,
            passives: json(source, PASSIVES)?,
            mastery_groups: json(source, MASTERY_GROUPS)?,
            mastery_effects: json(source, MASTERY_EFFECTS)?,
            tree: json(source, PASSIVE_TREE)?,
        })
    }
}

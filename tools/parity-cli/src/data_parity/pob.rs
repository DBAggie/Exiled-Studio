//! The PoB data export (`tools/oracle/data/pob_data.lua`, format 1): the
//! values PoB v2.67.2 holds after startup, as PoB holds them. The reader is
//! strict: an unknown field or a missing one fails the run, so a changed
//! script cannot be compared silently.

use std::collections::BTreeMap;

use serde::de::{self, IgnoredAny};
use serde::{Deserialize, Deserializer};

/// The export format this reader understands.
pub const FORMAT: u32 = 12;

/// The whole export.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Export {
    /// [`FORMAT`].
    pub format: u32,
    /// PoB's version, as its manifest names it.
    pub pob: Version,
    /// The tree version read (`TreeData/<version>/tree.lua`).
    pub tree_version: String,
    #[expect(
        dead_code,
        reason = "read so the export is checked whole; not compared (data_parity::NOT_COMPARED)"
    )]
    /// `SkillType` names by number (`src/Data/Global.lua`).
    #[serde(deserialize_with = "list")]
    pub skill_types: Vec<String>,
    /// `data.skills`, by granted effect ID.
    pub skills: BTreeMap<String, Skill>,
    /// `data.gems`, by PoB's key.
    pub gems: BTreeMap<String, Gem>,
    /// `data.itemBases` with weapon or armour properties, by name.
    pub bases: BTreeMap<String, Base>,
    /// Every `data.itemBases` entry's requirements, by name (format 6).
    pub base_requirements: BTreeMap<String, BaseRequirements>,
    /// Every flask base's `flask` data, by name (format 11, P6-4b).
    pub flasks: BTreeMap<String, FlaskBase>,
    /// Every minion and spectre, by the monster variety's ID (format 12,
    /// P6-7a).
    pub minions: BTreeMap<String, Minion>,
    /// The mods of the keyed `data.itemMods` files, by mod ID (format 6).
    pub mods: BTreeMap<String, ItemMod>,
    /// `data.monster*Table`, by table name; entry `i` is level `i + 1`.
    pub monster_tables: BTreeMap<String, Vec<f64>>,
    /// `data.gameConstants`.
    pub game_constants: BTreeMap<String, f64>,
    /// `data.characterConstants`.
    pub character_constants: BTreeMap<String, f64>,
    /// `data.monsterConstants`.
    pub monster_constants: BTreeMap<String, f64>,
    /// `data.bosses`, by PoB's name (format 2).
    pub bosses: BTreeMap<String, Boss>,
    /// data.pantheons, by god ID (format 3, P3-12g).
    pub pantheons: BTreeMap<String, God>,
    /// data.jewelRadius, in PoB's order: the full radii, then the rings
    /// (format 7, passive tree plan T-f).
    #[serde(deserialize_with = "list")]
    pub jewel_radii: Vec<JewelRadius>,
    /// `data.flavourText`, in PoB's order (format 8, B-2).
    #[serde(deserialize_with = "list")]
    pub flavour_text: Vec<FlavourEntry>,
    /// `data.clusterJewels` (format 9, passive tree plan T-i).
    pub cluster_jewels: ClusterJewels,
    /// The tree.
    pub tree: Tree,
}

/// `data.clusterJewels` (`src/Data/ClusterJewels.lua`).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterJewels {
    /// Each jewel, by its base's name.
    pub jewels: BTreeMap<String, ClusterJewel>,
    /// `notableSortOrder`: each notable's stat row (1-based), by name.
    pub notable_sort_order: BTreeMap<String, i64>,
    /// The keystones' names.
    #[serde(deserialize_with = "list")]
    pub keystones: Vec<String>,
    /// `orbitOffsets`: each proxy's start indices, by size.
    pub orbit_offsets: BTreeMap<String, Vec<i64>>,
}

/// A jewel of `data.clusterJewels.jewels`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterJewel {
    /// `sizeIndex`.
    pub size_index: u32,
    /// `minNodes`.
    pub min_nodes: u32,
    /// `maxNodes`.
    pub max_nodes: u32,
    /// `smallIndicies`.
    #[serde(deserialize_with = "list")]
    pub small: Vec<u32>,
    /// `notableIndicies`.
    #[serde(deserialize_with = "list")]
    pub notable: Vec<u32>,
    /// `socketIndicies`.
    #[serde(deserialize_with = "list")]
    pub socket: Vec<u32>,
    /// `totalIndicies`.
    pub total: u32,
    /// Its small passives, by passive ID.
    #[serde(default)]
    pub skills: BTreeMap<String, ClusterJewelSkill>,
}

/// A small passive of a [`ClusterJewel`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterJewelSkill {
    /// Its name (" (Legacy)" added for a tag PoB marks old).
    pub name: String,
    /// Its tag.
    pub tag: String,
}

/// An entry of `data.flavourText`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlavourEntry {
    /// The `FlavourText` ID, trailing underscores dropped.
    pub id: String,
    /// The unique's name.
    pub name: String,
    /// The lines as PoB's exporter cleaned them.
    #[serde(deserialize_with = "list")]
    pub text: Vec<String>,
}

/// An entry of `data.jewelRadius`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JewelRadius {
    /// The inner radius (0 for a full radius).
    pub inner: f64,
    /// The outer radius.
    pub outer: f64,
    /// `Small` ... `Massive`, or `Variable` for a ring.
    pub label: String,
}

/// A boss of `data.bosses`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boss {
    /// `armourMult`, percent.
    #[serde(rename = "armourMult")]
    pub armour_mult: f64,
    /// `evasionMult`, percent.
    #[serde(rename = "evasionMult")]
    pub evasion_mult: f64,
    /// `isUber`.
    #[serde(rename = "isUber")]
    pub is_uber: bool,
}

/// A minion or spectre of `data.minions` (`src/Data/Minions.lua`,
/// `Spectres.lua`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Minion {
    /// PoB's key (the variety's ID, or a minion's short name).
    #[expect(
        dead_code,
        reason = "read so the export is checked whole; the variety's ID keys the row"
    )]
    pub key: String,
    /// The name.
    pub name: String,
    /// Life multiplier.
    pub life: f64,
    #[serde(default, rename = "energyShield")]
    pub energy_shield: Option<f64>,
    #[serde(default)]
    pub armour: Option<f64>,
    #[serde(default)]
    pub evasion: Option<f64>,
    #[serde(rename = "fireResist")]
    pub fire_resist: f64,
    #[serde(rename = "coldResist")]
    pub cold_resist: f64,
    #[serde(rename = "lightningResist")]
    pub lightning_resist: f64,
    #[serde(rename = "chaosResist")]
    pub chaos_resist: f64,
    /// Damage multiplier.
    pub damage: f64,
    #[serde(rename = "damageSpread")]
    pub damage_spread: f64,
    /// Attack time, seconds.
    #[serde(rename = "attackTime")]
    pub attack_time: f64,
    #[serde(rename = "attackRange")]
    pub attack_range: f64,
    pub accuracy: f64,
    #[serde(default, rename = "damageFixup")]
    pub damage_fixup: Option<f64>,
    #[serde(default, rename = "lifeScaling")]
    pub life_scaling: Option<String>,
    #[serde(rename = "baseDamageIgnoresAttackSpeed")]
    pub base_damage_ignores_attack_speed: bool,
    #[serde(default, rename = "weaponType1")]
    pub weapon_type1: Option<String>,
    #[serde(default, rename = "weaponType2")]
    pub weapon_type2: Option<String>,
    #[serde(rename = "monsterTags", deserialize_with = "list")]
    pub monster_tags: Vec<String>,
    #[serde(rename = "skillList", deserialize_with = "list")]
    pub skill_list: Vec<String>,
}

/// A flask base's `flask` table (`src/Data/Bases/flask.lua`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlaskBase {
    /// Life per use, when the flask recovers life.
    #[serde(default)]
    pub life: Option<f64>,
    /// Mana per use, when the flask recovers mana.
    #[serde(default)]
    pub mana: Option<f64>,
    /// The duration, seconds (`RecoveryTime / 10`).
    pub duration: f64,
    /// The buff's lines, as the exporter rendered them.
    #[serde(deserialize_with = "list")]
    pub buff: Vec<String>,
}

/// A god of data.pantheons.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct God {
    /// isMajorGod.
    #[serde(rename = "isMajorGod")]
    pub is_major_god: bool,
    /// The souls, in PoB's order.
    #[serde(deserialize_with = "list")]
    pub souls: Vec<Soul>,
}

/// A soul of a god.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Soul {
    /// The soul's name.
    pub name: String,
    /// Its mods, in PoB's order.
    #[serde(deserialize_with = "list")]
    pub mods: Vec<SoulMod>,
}

/// A soul's mod: the line PoB's exporter rendered from the stat and its
/// value (describeStats), which PoB's parser reads.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoulMod {
    /// The line.
    pub line: String,
    /// The stat's values.
    #[serde(deserialize_with = "list")]
    pub value: Vec<f64>,
}

/// PoB's version.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    /// `launch.versionNumber`.
    #[serde(default)]
    pub version: Option<String>,
    /// The runtime (`jit.version`).
    #[serde(default)]
    pub runtime: Option<String>,
    #[expect(
        dead_code,
        reason = "read so the export is checked whole; not recorded"
    )]
    /// `launch.versionBranch`.
    #[serde(default)]
    pub branch: Option<String>,
}

/// A stat with a value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatValue {
    /// Stat ID.
    pub stat: String,
    /// Value as PoB holds it.
    pub value: f64,
}

/// One `data.skills` entry. `name` and `support` are read so the export
/// is checked whole; the bundle's own support flag decides.
#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "name and support are not compared")]
#[serde(deny_unknown_fields)]
pub struct Skill {
    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
    /// `castTime`, seconds (active skills only).
    #[serde(default, rename = "castTime")]
    pub cast_time: Option<f64>,
    /// `baseEffectiveness`, written when not 1.
    #[serde(default, rename = "baseEffectiveness")]
    pub base_effectiveness: Option<f64>,
    /// `incrementalEffectiveness`, written when not 0.
    #[serde(default, rename = "incrementalEffectiveness")]
    pub incremental_effectiveness: Option<f64>,
    /// `support`.
    #[serde(default)]
    pub support: Option<bool>,
    /// `color` (format 5, P3-12i): the game's `Attribute`, as PoB's
    /// exporter writes it (1 strength, 2 dexterity, 3 intelligence, 4
    /// none); absent for a skill PoB defines without one.
    #[serde(default)]
    pub color: Option<i64>,
    /// Stat IDs in level-value order.
    #[serde(deserialize_with = "list")]
    pub stats: Vec<String>,
    /// `SkillType` names, sorted.
    #[serde(deserialize_with = "list")]
    pub types: Vec<String>,
    /// `constantStats`.
    #[serde(deserialize_with = "list")]
    pub constant_stats: Vec<StatValue>,
    /// `qualityStats`, value per 1% quality.
    #[serde(deserialize_with = "list")]
    pub quality_stats: Vec<StatValue>,
    /// `levels`, by level number as text.
    pub levels: BTreeMap<String, Level>,
    /// The mods each stat gives through the skill's `statMap` or PoB's
    /// `SkillStatMap.lua`, and the base mods (stat `""`), format 4
    /// (P3-12h).
    #[serde(deserialize_with = "list")]
    pub mods: Vec<StatMods>,
    /// `minionList` (format 4).
    #[serde(deserialize_with = "list")]
    pub minions: Vec<String>,
    /// A support's `addSkillTypes`, by name (format 4).
    #[serde(deserialize_with = "list")]
    pub add_types: Vec<String>,
    /// A support's `addMinionList` (format 4).
    #[serde(deserialize_with = "list")]
    pub add_minions: Vec<String>,
    /// A support's `isTrigger` (format 4).
    pub is_trigger: bool,
    /// PoB's hand-written base flags (format 10; read by `pob-skills`, not
    /// compared here).
    #[serde(default, deserialize_with = "list")]
    pub base_flags: Vec<String>,
    /// PoB's hand-written parts (format 10; read by `pob-skills`).
    #[serde(default)]
    pub parts: IgnoredAny,
    /// The names of the skill's functions (format 10; read by
    /// `pob-skills`).
    #[serde(default, deserialize_with = "list")]
    pub functions: Vec<String>,
}

/// The mods one stat of a skill gives (format 4): PoB's
/// `mergeSkillInstanceMods` reads `statMap[stat]`, else
/// `data.skillStatMap[stat]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatMods {
    /// The stat (`""` for the skill's base mods).
    pub stat: String,
    #[expect(
        dead_code,
        reason = "read so the export is checked whole; not compared"
    )]
    /// From the skill's own `statMap` (else PoB's global map).
    pub own: bool,
    /// The map's `div`, when it has one.
    #[serde(default)]
    pub div: Option<f64>,
    /// The map's `mult`, when it has one.
    #[serde(default)]
    pub mult: Option<f64>,
    /// The mods.
    #[serde(deserialize_with = "list")]
    pub mods: Vec<ModRecord>,
}

/// One mod of a skill's stat map (format 4), plain fields only.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModRecord {
    /// The mod's name (`Accuracy`, `CritChance`, `SkillData`, ...).
    pub name: String,
    /// Its type (`BASE`, `INC`, `MORE`, `FLAG`, `LIST`, ...).
    #[serde(rename = "type")]
    pub kind: String,
    /// `ModFlag` bits.
    #[serde(default)]
    pub flags: Option<u64>,
    /// `KeywordFlag` bits.
    #[serde(default, rename = "keywordFlags")]
    pub keyword_flags: Option<u64>,
    /// A constant value, when the mod has one.
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// A `LIST` mod's value fields (`key` and `value` for `SkillData`).
    #[serde(default)]
    pub value_fields: Option<BTreeMap<String, serde_json::Value>>,
    /// The tags, in order, each with its plain fields.
    #[serde(deserialize_with = "list")]
    pub tags: Vec<BTreeMap<String, serde_json::Value>>,
}

impl ModRecord {
    /// The `GlobalEffect` tag's fields, if the mod has one.
    #[must_use]
    pub fn global_effect(&self) -> Option<&BTreeMap<String, serde_json::Value>> {
        self.tags
            .iter()
            .find(|t| t.get("type").and_then(serde_json::Value::as_str) == Some("GlobalEffect"))
    }
}

/// One level of a skill.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    /// Values in the skill's `stats` order (stats past the last value have
    /// none).
    #[serde(deserialize_with = "list")]
    pub values: Vec<f64>,
    /// `cost`, by resource (Vaal souls left out by the script).
    pub cost: BTreeMap<String, f64>,
    /// `critChance`, percent.
    #[serde(default, rename = "critChance")]
    pub crit_chance: Option<f64>,
    /// `damageEffectiveness`.
    #[serde(default, rename = "damageEffectiveness")]
    pub damage_effectiveness: Option<f64>,
    /// `baseMultiplier`.
    #[serde(default, rename = "baseMultiplier")]
    pub base_multiplier: Option<f64>,
    /// `manaMultiplier` (cost multiplier minus 100).
    #[serde(default, rename = "manaMultiplier")]
    pub mana_multiplier: Option<f64>,
    /// `attackSpeedMultiplier`.
    #[serde(default, rename = "attackSpeedMultiplier")]
    pub attack_speed_multiplier: Option<f64>,
    /// `attackTime`, milliseconds.
    #[serde(default, rename = "attackTime")]
    pub attack_time: Option<f64>,
    /// `cooldown`, seconds.
    #[serde(default)]
    pub cooldown: Option<f64>,
    /// `storedUses`.
    #[serde(default, rename = "storedUses")]
    pub stored_uses: Option<f64>,
    /// `manaReservationFlat`.
    #[serde(default, rename = "manaReservationFlat")]
    pub mana_reservation_flat: Option<f64>,
    /// `manaReservationPercent`, percent.
    #[serde(default, rename = "manaReservationPercent")]
    pub mana_reservation_percent: Option<f64>,
    /// `lifeReservationFlat`.
    #[serde(default, rename = "lifeReservationFlat")]
    pub life_reservation_flat: Option<f64>,
    /// `lifeReservationPercent`, percent.
    #[serde(default, rename = "lifeReservationPercent")]
    pub life_reservation_percent: Option<f64>,
}

/// One `data.gems` entry.
#[derive(Debug, Deserialize)]
#[expect(
    dead_code,
    reason = "name and natural maximum level are not compared (NOT_COMPARED)"
)]
#[serde(deny_unknown_fields)]
pub struct Gem {
    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
    /// The base item.
    #[serde(rename = "gameId")]
    pub game_id: String,
    /// The game's gem effect ID.
    #[serde(rename = "variantId")]
    pub variant_id: String,
    /// The granted effect.
    #[serde(rename = "grantedEffectId")]
    pub granted_effect_id: String,
    /// The second granted effect (the base skill of a Vaal gem).
    #[serde(default, rename = "secondaryGrantedEffectId")]
    pub secondary_granted_effect_id: Option<String>,
    /// `vaalGem`.
    #[serde(default, rename = "vaalGem")]
    pub vaal_gem: Option<bool>,
    /// Attribute requirement weights.
    #[serde(rename = "reqStr")]
    pub req_str: f64,
    /// Dexterity.
    #[serde(rename = "reqDex")]
    pub req_dex: f64,
    /// Intelligence.
    #[serde(rename = "reqInt")]
    pub req_int: f64,
    /// `naturalMaxLevel`.
    #[serde(default, rename = "naturalMaxLevel")]
    pub natural_max_level: Option<f64>,
    /// Gem tags, sorted.
    #[serde(deserialize_with = "list")]
    pub tags: Vec<String>,
}

/// One item base with weapon or armour properties.
#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "PoB's item type names are not game data")]
#[serde(deny_unknown_fields)]
pub struct Base {
    /// PoB's item type.
    #[serde(rename = "type")]
    pub kind: String,
    /// PoB's sub-type.
    #[serde(default, rename = "subType")]
    pub sub_type: Option<String>,
    /// `weapon` (PoB's field names).
    #[serde(default)]
    pub weapon: Option<BTreeMap<String, f64>>,
    /// `armour` (PoB's field names).
    #[serde(default)]
    pub armour: Option<BTreeMap<String, f64>>,
}

/// An item base's `req` (format 6): the level PoB's exporter derived
/// (`v2.67.2:src/Export/Scripts/bases.lua:276-293`) and the attributes it
/// copied when not 0 (`:294-305`). Absent fields were not written.
#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "PoB's item type names are not game data")]
#[serde(deny_unknown_fields)]
pub struct BaseRequirements {
    /// PoB's item type.
    #[serde(rename = "type")]
    pub kind: String,
    /// Whether the base is a flask (the exporter's other level rule).
    pub flask: bool,
    /// The required level.
    #[serde(default)]
    pub level: Option<f64>,
    /// Strength.
    #[serde(default)]
    pub str: Option<f64>,
    /// Dexterity.
    #[serde(default)]
    pub dex: Option<f64>,
    /// Intelligence.
    #[serde(default)]
    pub int: Option<f64>,
}

/// One mod of the keyed `data.itemMods` files (format 6), as PoB's
/// exporter wrote it (`v2.67.2:src/Export/Scripts/mods.lua:83-114`).
#[derive(Debug, Deserialize)]
#[expect(
    dead_code,
    reason = "the kind is not compared (data_parity::NOT_COMPARED)"
)]
#[serde(deny_unknown_fields)]
pub struct ItemMod {
    /// `affix` (the game's `Mods.Name`; `""` when it has none).
    pub affix: String,
    /// `level` (the game's `Mods.Level`).
    #[serde(default)]
    pub level: Option<f64>,
    /// `type` (`Prefix`, `Suffix`, `Corrupted`, ...), when written.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// The `data.itemMods` files holding the ID, in the export's order.
    #[serde(deserialize_with = "list")]
    pub tables: Vec<String>,
    /// Two of those files disagree on the affix or level.
    #[serde(default)]
    pub conflict: bool,
}

/// The tree file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tree {
    /// Classes in the file's order.
    #[serde(deserialize_with = "list")]
    pub classes: Vec<Class>,
    /// Nodes by key (the node number as text).
    pub nodes: BTreeMap<String, Node>,
}

/// A class of the tree file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Class {
    /// Class name.
    pub name: String,
    /// Base strength.
    pub base_str: f64,
    /// Base dexterity.
    pub base_dex: f64,
    /// Base intelligence.
    pub base_int: f64,
    /// Ascendancy IDs.
    #[serde(deserialize_with = "list")]
    pub ascendancies: Vec<String>,
}

/// A mastery effect of a node.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MasteryEffect {
    /// The effect's hash.
    pub effect: i64,
    /// Display lines.
    #[serde(deserialize_with = "list")]
    pub stats: Vec<String>,
}

/// A node of the tree file (the flags PoB reads; absent means false).
/// The start, choice, proxy, bloodline, and granted-attribute fields are
/// read so the export is checked whole; P3-3 uses them for tree rules.
#[derive(Debug, Deserialize)]
#[expect(
    dead_code,
    reason = "tree-rule fields are not compared here (NOT_COMPARED)"
)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Node {
    /// Node number.
    pub skill: i64,
    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display lines.
    #[serde(deserialize_with = "list")]
    pub stats: Vec<String>,
    /// Keystone.
    #[serde(default)]
    pub is_keystone: bool,
    /// Notable.
    #[serde(default)]
    pub is_notable: bool,
    /// Mastery.
    #[serde(default)]
    pub is_mastery: bool,
    /// Jewel socket.
    #[serde(default)]
    pub is_jewel_socket: bool,
    /// Ascendancy (or bloodline) name.
    #[serde(default)]
    pub ascendancy_name: Option<String>,
    /// Ascendancy start.
    #[serde(default)]
    pub is_ascendancy_start: bool,
    /// Class start index.
    #[serde(default)]
    pub class_start_index: Option<i64>,
    /// Multiple choice.
    #[serde(default)]
    pub is_multiple_choice: bool,
    /// Multiple choice option.
    #[serde(default)]
    pub is_multiple_choice_option: bool,
    /// Blighted (anoint only).
    #[serde(default)]
    pub is_blighted: bool,
    /// Proxy (cluster jewel placeholder).
    #[serde(default)]
    pub is_proxy: bool,
    /// Bloodline node.
    #[serde(default)]
    pub is_bloodline: bool,
    /// Passive points granted.
    #[serde(default)]
    pub granted_passive_points: Option<i64>,
    /// Strength granted.
    #[serde(default)]
    pub granted_strength: Option<i64>,
    /// Dexterity granted.
    #[serde(default)]
    pub granted_dexterity: Option<i64>,
    /// Intelligence granted.
    #[serde(default)]
    pub granted_intelligence: Option<i64>,
    /// Expansion (cluster) jewel socket.
    pub expansion_jewel: bool,
    /// Mastery effects.
    #[serde(deserialize_with = "list")]
    pub mastery_effects: Vec<MasteryEffect>,
}

/// A list. The export's JSON writer writes an empty Lua table as `{}`, so
/// an empty object is an empty list; any other object is an error.
fn list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ListOrEmpty<T> {
        List(Vec<T>),
        Object(BTreeMap<String, IgnoredAny>),
    }
    match ListOrEmpty::deserialize(deserializer)? {
        ListOrEmpty::List(items) => Ok(items),
        ListOrEmpty::Object(map) if map.is_empty() => Ok(Vec::new()),
        ListOrEmpty::Object(_) => Err(de::Error::custom("expected a list")),
    }
}

/// Reads an export.
///
/// # Errors
/// A message when the text is not an export of [`FORMAT`].
pub fn parse(bytes: &[u8]) -> Result<Export, String> {
    let export: Export =
        serde_json::from_slice(bytes).map_err(|e| format!("the PoB export: {e}"))?;
    if export.format != FORMAT {
        return Err(format!(
            "the PoB export: format {} (this tool reads {FORMAT})",
            export.format
        ));
    }
    Ok(export)
}

//! The skill tables of bundle format 2 (Phase 3 plan, step 1a): what the
//! calc reads about skills and gems, in our own types and names (Phase 2
//! plan decision 8), shared by both games. A format-2 bundle has every file
//! below; a format-1 bundle has none (the loader refuses one).
//!
//! | Path | Content |
//! |---|---|
//! | `data/skill_types.json` | [`SkillType`]s |
//! | `data/cost_types.json` | cost type IDs (`Mana`, `Life`, `ManaPercent`, ...) that [`Cost`] names |
//! | `data/active_skills.json` | [`ActiveSkill`]s |
//! | `data/granted_effects.json` | [`GrantedEffect`]s, with their per-level rows and quality stats |
//! | `data/stat_sets.json` | [`StatSet`]s, with their per-level stat values |
//! | `data/gems.json` | [`SkillGem`]s |
//! | `data/gem_effects.json` | [`GemEffect`]s |
//! | `data/gem_tags.json` | [`GemTag`]s |
//!
//! Values are the game's, unscaled: times in milliseconds, chances in
//! hundredths of a percent, multipliers and effectiveness as the tables
//! store them. How the calc reads them (PoB divides some, adds 1 to others)
//! is the calc's rule, written and tested there, not a conversion here.
//! References are by ID (stats, skill types, item classes, base items) and
//! the validator checks them.
//!
//! Not carried (no calc in Phase 3 reads them; adding one is a new field):
//! PoE1 Ruthless (`Hardmode`) gem effects, level requirements, Vaal soul
//! costs, PoE2 quality `AltStats` and `ApplyToStatSets`, PoE2 weapon
//! requirements of active skills.

use serde::{Deserialize, Serialize, Serializer};

use super::{BundleError, BundleSource, json};

/// Writes an `f32` as its shortest text (`2.921`, not the widened
/// `2.9210000038146973`), so the bundle shows the extract's value; it reads
/// back to the same `f32`.
fn short_f32<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    let wide: f64 = v.to_string().parse().map_err(serde::ser::Error::custom)?;
    s.serialize_f64(wide)
}

/// [`short_f32`] for an optional value.
#[expect(
    clippy::ref_option,
    reason = "serde's `serialize_with` passes the field by reference"
)]
fn short_f32_opt<S: Serializer>(v: &Option<f32>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => short_f32(v, s),
        None => s.serialize_none(),
    }
}

/// Skill types.
pub const SKILL_TYPES: &str = "data/skill_types.json";
/// Cost types.
pub const COST_TYPES: &str = "data/cost_types.json";
/// Active skills.
pub const ACTIVE_SKILLS: &str = "data/active_skills.json";
/// Granted effects.
pub const GRANTED_EFFECTS: &str = "data/granted_effects.json";
/// Stat sets.
pub const STAT_SETS: &str = "data/stat_sets.json";
/// Skill gems.
pub const GEMS: &str = "data/gems.json";
/// Gem effects.
pub const GEM_EFFECTS: &str = "data/gem_effects.json";
/// Gem tags.
pub const GEM_TAGS: &str = "data/gem_tags.json";

/// The section's files, each required in format 2.
pub const FILES: [&str; 8] = [
    SKILL_TYPES,
    COST_TYPES,
    ACTIVE_SKILLS,
    GRANTED_EFFECTS,
    STAT_SETS,
    GEMS,
    GEM_EFFECTS,
    GEM_TAGS,
];

/// A skill type (`ActiveSkillType`): the tags that decide which supports
/// apply and what a skill is (spell, attack, totem, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillType {
    /// ID.
    pub id: String,
    /// The stat that marks a skill of this type, if any.
    #[serde(default)]
    pub flag_stat: Option<String>,
}

/// An active skill (`ActiveSkills`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveSkill {
    /// ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Skill types, by ID.
    #[serde(default)]
    pub types: Vec<String>,
    /// Skill types of the minions it makes, by ID.
    #[serde(default)]
    pub minion_types: Vec<String>,
    /// Item classes it needs as a weapon (PoE1; empty: none).
    #[serde(default)]
    pub weapon_classes: Vec<String>,
    /// The character uses it directly (the game's `IsManuallyCasted`).
    #[serde(default)]
    pub manually_cast: bool,
}

/// A resource cost of one level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    /// Cost type ID (`Mana`, `Life`, `ManaPercent`, ...; one of the
    /// bundle's cost types). The calc handles a closed set of them and
    /// reports any other as unsupported.
    pub resource: String,
    /// Amount, in the cost type's unit.
    pub amount: i64,
}

/// What a reservation reserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    /// Mana.
    Mana,
    /// Life.
    Life,
    /// Spirit (PoE2).
    Spirit,
}

/// A reservation of one level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reservation {
    /// The resource.
    pub resource: Resource,
    /// A share of the resource (hundredths of a percent) instead of an
    /// amount.
    #[serde(default)]
    pub percent: bool,
    /// The amount or share.
    pub amount: i64,
}

/// One level of a granted effect (`GrantedEffectsPerLevel`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectLevel {
    /// Level.
    pub level: i64,
    /// Costs (empty: none).
    #[serde(default)]
    pub costs: Vec<Cost>,
    /// Cost multiplier, percent (100: none).
    pub cost_multiplier: i64,
    /// Cooldown, milliseconds (0: none).
    #[serde(default)]
    pub cooldown: i64,
    /// Stored uses (0: none).
    #[serde(default)]
    pub stored_uses: i64,
    /// Attack speed multiplier as stored (0: none).
    #[serde(default)]
    pub attack_speed_multiplier: i64,
    /// Attack time, milliseconds (0: none).
    #[serde(default)]
    pub attack_time: i64,
    /// Reservations (empty: none).
    #[serde(default)]
    pub reservations: Vec<Reservation>,
}

/// A stat a gem's quality gives (`GrantedEffectQualityStats`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityStat {
    /// Stat ID.
    pub stat: String,
    /// Value per 1% quality, in thousandths.
    pub per_mille: i64,
}

/// A granted effect (`GrantedEffects`): an active skill or a support, with
/// its levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantedEffect {
    /// ID.
    pub id: String,
    /// A support.
    #[serde(default)]
    pub support: bool,
    /// The active skill, by ID (none for a support).
    #[serde(default)]
    pub active_skill: Option<String>,
    /// Cast time, milliseconds.
    #[serde(default)]
    pub cast_time: i64,
    /// Supports: skill types it can support, by ID.
    #[serde(default)]
    pub allowed_types: Vec<String>,
    /// Supports: skill types it adds.
    #[serde(default)]
    pub added_types: Vec<String>,
    /// Supports: skill types it cannot support.
    #[serde(default)]
    pub excluded_types: Vec<String>,
    /// Supports: skill types it adds to minions.
    #[serde(default)]
    pub added_minion_types: Vec<String>,
    /// Supports: supports only gems.
    #[serde(default)]
    pub supports_gems_only: bool,
    /// Cannot be supported.
    #[serde(default)]
    pub cannot_be_supported: bool,
    /// Stat sets by ID, the main one first.
    #[serde(default)]
    pub stat_sets: Vec<String>,
    /// Levels, in ascending level order.
    #[serde(default)]
    pub levels: Vec<EffectLevel>,
    /// Quality stats.
    #[serde(default)]
    pub quality: Vec<QualityStat>,
}

/// A stat of one level with a float value and its interpolation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatStat {
    /// Stat ID.
    pub stat: String,
    /// The value as stored (`FloatStatsValues`).
    #[serde(serialize_with = "short_f32")]
    pub value: f32,
    /// The resolved base value (`BaseResolvedValues`).
    pub resolved: i64,
    /// The game's interpolation number (`StatInterpolationTypes`: 1, 2, 3).
    /// Its meaning is the calc's rule (P3-9a), so it is kept as stored.
    pub interpolation: i64,
    /// The value of the interpolation base (`EffectivenessCostConstants`).
    #[serde(serialize_with = "short_f32")]
    pub interpolation_base: f32,
}

/// A stat of one level with an integer value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelStat {
    /// Stat ID.
    pub stat: String,
    /// Value.
    pub value: i64,
    /// The game's interpolation number, when the row gives one.
    #[serde(default)]
    pub interpolation: Option<i64>,
}

/// One level of a stat set (`GrantedEffectStatSetsPerLevel`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatSetLevel {
    /// Gem level.
    pub level: i64,
    /// Spell critical strike chance, hundredths of a percent.
    #[serde(default)]
    pub spell_crit_chance: i64,
    /// Attack critical strike chance, hundredths of a percent.
    #[serde(default)]
    pub attack_crit_chance: i64,
    /// Base damage multiplier as stored (0: none).
    #[serde(default)]
    pub base_multiplier: i64,
    /// Damage effectiveness as stored (PoE1; absent in PoE2).
    #[serde(default)]
    pub damage_effectiveness: Option<i64>,
    /// Stats with float values.
    #[serde(default)]
    pub float_stats: Vec<FloatStat>,
    /// Stats with integer values.
    #[serde(default)]
    pub stats: Vec<LevelStat>,
    /// Flag stats (no value).
    #[serde(default)]
    pub flags: Vec<String>,
    /// Effects this level grants, by ID.
    #[serde(default)]
    pub granted_effects: Vec<String>,
}

/// A stat with one value at every level of a stat set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstantStat {
    /// Stat ID.
    pub stat: String,
    /// Value.
    pub value: i64,
}

/// A stat set (`GrantedEffectStatSets`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatSet {
    /// ID.
    pub id: String,
    /// Stats every level has, valued elsewhere.
    #[serde(default)]
    pub implicit_stats: Vec<String>,
    /// Stats with one value at every level.
    #[serde(default)]
    pub constant_stats: Vec<ConstantStat>,
    /// Base effectiveness.
    #[serde(serialize_with = "short_f32")]
    pub base_effectiveness: f32,
    /// Effectiveness gained per level.
    #[serde(serialize_with = "short_f32")]
    pub incremental_effectiveness: f32,
    /// Damage effectiveness gained per level (PoE2; absent in PoE1).
    #[serde(default, serialize_with = "short_f32_opt")]
    pub damage_incremental_effectiveness: Option<f32>,
    /// Levels, in ascending level order.
    #[serde(default)]
    pub levels: Vec<StatSetLevel>,
}

/// A skill gem (`SkillGems`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillGem {
    /// The gem's base item, by ID (also the gem's key).
    pub base_item: String,
    /// Share of the strength requirement, percent.
    #[serde(default)]
    pub strength: i64,
    /// Share of the dexterity requirement, percent.
    #[serde(default)]
    pub dexterity: i64,
    /// Share of the intelligence requirement, percent.
    #[serde(default)]
    pub intelligence: i64,
    /// A Vaal gem.
    #[serde(default)]
    pub vaal: bool,
    /// Its effects (variants), by ID, in table order.
    #[serde(default)]
    pub effects: Vec<String>,
}

/// A gem effect (`GemEffects`): one variant of a gem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemEffect {
    /// ID.
    pub id: String,
    /// Name.
    pub name: String,
    /// The main granted effect, by ID.
    #[serde(default)]
    pub granted_effect: Option<String>,
    /// Further granted effects, by ID.
    #[serde(default)]
    pub additional_granted_effects: Vec<String>,
    /// Gem tags, by ID.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A gem tag (`GemTags`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemTag {
    /// ID.
    pub id: String,
    /// Display name (may be empty).
    #[serde(default)]
    pub name: String,
}

/// The skill tables.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillTables {
    /// Skill types.
    pub skill_types: Vec<SkillType>,
    /// Cost type IDs.
    pub cost_types: Vec<String>,
    /// Active skills.
    pub active_skills: Vec<ActiveSkill>,
    /// Granted effects.
    pub granted_effects: Vec<GrantedEffect>,
    /// Stat sets.
    pub stat_sets: Vec<StatSet>,
    /// Skill gems.
    pub gems: Vec<SkillGem>,
    /// Gem effects.
    pub gem_effects: Vec<GemEffect>,
    /// Gem tags.
    pub gem_tags: Vec<GemTag>,
}

impl SkillTables {
    /// Reads the section's files (the loader has checked they are listed).
    ///
    /// # Errors
    /// [`BundleError`] when a file does not read or parse.
    pub(super) fn load(source: &dyn BundleSource) -> Result<Self, BundleError> {
        Ok(Self {
            skill_types: json(source, SKILL_TYPES)?,
            cost_types: json(source, COST_TYPES)?,
            active_skills: json(source, ACTIVE_SKILLS)?,
            granted_effects: json(source, GRANTED_EFFECTS)?,
            stat_sets: json(source, STAT_SETS)?,
            gems: json(source, GEMS)?,
            gem_effects: json(source, GEM_EFFECTS)?,
            gem_tags: json(source, GEM_TAGS)?,
        })
    }
}

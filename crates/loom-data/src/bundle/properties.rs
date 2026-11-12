//! The item property and monster tables of bundle format 2 (Phase 3 plan,
//! step 1c), shared by both games, in our own types and names:
//!
//! | Path | Content |
//! |---|---|
//! | `data/weapon_types.json` | [`WeaponType`]s: a weapon base's damage, attack time, critical chance, range |
//! | `data/armour_types.json` | [`ArmourType`]s: an armour base's defences |
//! | `data/shield_types.json` | [`ShieldType`]s: a shield base's block chance |
//! | `data/monster_levels.json` | [`MonsterLevel`]s: the game's per-level monster stats |
//! | `data/game_constants.json` | [`GameConstant`]s |
//! | `data/monster_types.json` | [`MonsterType`]s: a monster type's armour and evasion multipliers (P3-1d) |
//!
//! The default monster stats that are not per level (resistances, caps,
//! ...) are the monster object template's `Stats` block, in
//! `data/object_templates.json` (`metadata/monsters/monster`, step 1b).
//!
//! Values are the game's, unscaled: chances in hundredths of a percent,
//! times in milliseconds, a constant as `value` over `divisor`. PoE1
//! stores a defence as a range, PoE2 as one value (`min` equals `max`).
//! `MonsterLevel::armour` is the table's column as stored; PoB v2.67.2
//! derives monster armour from the game constant
//! `MonsterDamageReductionImprovement` instead ("The table here is wrong",
//! `src/Export/Scripts/miscdata.lua`); the calc decides with evidence.
//!
//! Shape per game (validated, `BUNDLE_GAME_SHAPE`): PoE1 has no reload
//! time and has `alt_life_1` and `phys_conversion_multi`; PoE2 has a reload
//! time, one value per defence (`min` = `max`), and neither PoE1 column.
//!
//! Not carried (no Phase 3 calc reads them): PoE2 weapon classes (for P3-10
//! if PoE2 attacks come into scope),
//! `DefaultMonsterStats`' experience-only, difficulty, and PoE2-only leech
//! and damage columns.

use serde::{Deserialize, Serialize};

use super::{BundleError, BundleSource, json, short_f32};

/// Weapon types.
pub const WEAPON_TYPES: &str = "data/weapon_types.json";
/// Armour types.
pub const ARMOUR_TYPES: &str = "data/armour_types.json";
/// Shield types.
pub const SHIELD_TYPES: &str = "data/shield_types.json";
/// Monster levels.
pub const MONSTER_LEVELS: &str = "data/monster_levels.json";
/// Game constants.
pub const GAME_CONSTANTS: &str = "data/game_constants.json";
/// Monster types.
pub const MONSTER_TYPES: &str = "data/monster_types.json";

/// The section's files, each required in format 2.
pub const FILES: [&str; 6] = [
    WEAPON_TYPES,
    ARMOUR_TYPES,
    SHIELD_TYPES,
    MONSTER_LEVELS,
    GAME_CONSTANTS,
    MONSTER_TYPES,
];

/// A weapon base's properties (`WeaponTypes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponType {
    /// The base item, by ID.
    pub base_item: String,
    /// Critical strike chance, hundredths of a percent.
    pub crit_chance: i64,
    /// Attack time, milliseconds.
    pub attack_time: i64,
    /// Minimum physical damage.
    pub damage_min: i64,
    /// Maximum physical damage.
    pub damage_max: i64,
    /// Range as stored.
    pub range: i64,
    /// Reload time, milliseconds (PoE2; absent in PoE1).
    #[serde(default)]
    pub reload_time: Option<i64>,
}

/// A range of a base value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueRange {
    /// Lowest.
    pub min: i64,
    /// Highest.
    pub max: i64,
}

/// An armour base's properties (`ArmourTypes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmourType {
    /// The base item, by ID.
    pub base_item: String,
    /// Armour.
    pub armour: ValueRange,
    /// Evasion.
    pub evasion: ValueRange,
    /// Energy shield.
    pub energy_shield: ValueRange,
    /// Ward.
    pub ward: ValueRange,
    /// Movement speed change, percent.
    pub movement_speed: i64,
}

/// A shield base's block chance (`ShieldTypes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShieldType {
    /// The base item, by ID.
    pub base_item: String,
    /// Block chance, percent.
    pub block: i64,
}

/// The game's monster stats at one level (`DefaultMonsterStats`). The
/// levels run from 1 without gaps (validated), so level `n` is row `n - 1`.
/// The `f32` values are the game's: the calc widens them with
/// `f64::from`, never by reparsing the text, so they equal PoB's widened
/// values. `armour` is the stored column; PoB uses the game constant
/// `MonsterDamageReductionImprovement` instead (module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonsterLevel {
    /// Monster level.
    pub level: i64,
    /// Damage.
    #[serde(serialize_with = "short_f32")]
    pub damage: f32,
    /// Evasion.
    pub evasion: i64,
    /// Accuracy.
    pub accuracy: i64,
    /// Life.
    pub life: i64,
    /// Armour as stored (module docs).
    pub armour: i64,
    /// Ailment threshold.
    pub ailment_threshold: i64,
    /// Evasion of evasive monsters.
    pub evasive_evasion: i64,
    /// Life of allies (PoE1 `MinionLife`, PoE2 `AllyLife`).
    pub ally_life: i64,
    /// Damage of minions.
    #[serde(serialize_with = "short_f32")]
    pub ally_damage: f32,
    /// The second alternative life column (`AltLife2`).
    pub alt_life_2: i64,
    /// The first alternative life column (PoE1 `AltLife1`).
    #[serde(default)]
    pub alt_life_1: Option<i64>,
    /// Physical conversion multiplier (PoE1).
    #[serde(default)]
    pub phys_conversion_multi: Option<i64>,
}

/// A game constant (`GameConstants`): `value / divisor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameConstant {
    /// ID.
    pub id: String,
    /// Value.
    pub value: i64,
    /// Divisor.
    pub divisor: i64,
}

/// A monster type's defence multipliers (`MonsterTypes`), percent: the
/// game scales the monster level's armour and evasion by `100 + value`
/// (PoB v2.67.2's `Data/Bosses.lua` carries the same two columns of the
/// bosses it lists, `src/Export/Scripts/bossData.lua`). The calc's enemy
/// model reads the bosses PoB lists (P3-11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonsterType {
    /// ID.
    pub id: String,
    /// Armour multiplier, percent.
    pub armour: i64,
    /// Evasion multiplier, percent.
    pub evasion: i64,
}

/// The item property and monster tables.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTables {
    /// Weapon types.
    pub weapon_types: Vec<WeaponType>,
    /// Armour types.
    pub armour_types: Vec<ArmourType>,
    /// Shield types.
    pub shield_types: Vec<ShieldType>,
    /// Monster levels.
    pub monster_levels: Vec<MonsterLevel>,
    /// Game constants.
    pub game_constants: Vec<GameConstant>,
    /// Monster types.
    pub monster_types: Vec<MonsterType>,
}

impl PropertyTables {
    /// Reads the section's files (the loader has checked they are listed).
    ///
    /// # Errors
    /// [`BundleError`] when a file does not read or parse.
    pub(super) fn load(source: &dyn BundleSource) -> Result<Self, BundleError> {
        Ok(Self {
            weapon_types: json(source, WEAPON_TYPES)?,
            armour_types: json(source, ARMOUR_TYPES)?,
            shield_types: json(source, SHIELD_TYPES)?,
            monster_levels: json(source, MONSTER_LEVELS)?,
            game_constants: json(source, GAME_CONSTANTS)?,
            monster_types: json(source, MONSTER_TYPES)?,
        })
    }
}

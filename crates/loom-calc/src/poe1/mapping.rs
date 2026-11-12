//! The Path of Exile stat-to-effect mapping (ADR-015; Phase 3 plan,
//! decision 2 and step 8): data rows, one per stat, each with a test.
//!
//! P3-6 maps the global stats of attributes, life, mana, and energy
//! shield; P3-7 the global defences and armour pieces' local stats
//! ([`PieceMapping`]); P3-9b the stats of a spell's hit ([`HIT_ROWS`], with
//! the skill flags they need) and the mapping of a gem's own stats
//! ([`SkillMapping`]). Each row names the stat's meaning as the game's stat
//! description writes it, and how PoB v2.67.2 reads the same line
//! (`Modules/ModParser.lua`), so a difference in reading shows in review.
//! Every other stat is `Unmapped`, which makes the computed fields
//! `Partial` (never a silent zero).

use loom_domain::StatId;

use crate::condition::{Conditions, Flags};
use crate::ingest::{EffectTemplate, StatMap, StatMapping};
use crate::poe1::{ArmourSlot, Defence, PieceValue, Poe1Quantity};
use crate::quantity::{Bound, DamageType, Op, Quantity};

/// One mapped effect: what, how, and the scale per unit of the stat.
type Row = (&'static str, &'static [(Quantity, Op, f64)]);

/// A mapped effect of a hit stat: what, how, the divisor of the stat's
/// value, and the skill flags it needs (all of the first, one of the
/// second when not empty).
type HitEffect = (Quantity, Op, f64, Flags, Flags);

/// One mapped hit stat.
type HitRow = (&'static str, &'static [HitEffect]);

const NONE: Flags = Flags::NONE;
const fn skill_base(t: DamageType, b: Bound) -> Quantity {
    Quantity::Poe1(Poe1Quantity::SkillBaseDamage(t, b))
}
const fn added(t: DamageType, b: Bound) -> Quantity {
    Quantity::AddedDamage(t, b)
}
const MIN: Bound = Bound::Min;
const MAX: Bound = Bound::Max;
const PHYS: DamageType = DamageType::Physical;
const FIRE_T: DamageType = DamageType::Fire;
const COLD_T: DamageType = DamageType::Cold;
const LIGHTNING_T: DamageType = DamageType::Lightning;
const CHAOS_T: DamageType = DamageType::Chaos;

/// The stats of hits P3-9b maps (a self-cast spell's), sorted by stat ID (a
/// test checks the order), each read as PoB v2.67.2 reads it
/// (`Data/SkillStatMap.lua`, a support's own `statMap` in `Data/Skills/`,
/// and `Modules/ModParser.lua` for the same text on items):
/// - `spell_minimum_base_<type>_damage`, `..._maximum_...`: the skill's own
///   base damage (PoB `skill("ColdMin")`, ...);
/// - `global_minimum_added_<cold|lightning>_damage`, `..._maximum_...`:
///   "Adds # to # Cold Damage" of supports (`ColdMin` BASE, no flag); PoB
///   has no row for the fire and physical ones; those and the chaos ones
///   (which PoB reads) stay unmapped until a fixture needs them;
/// - `spell_minimum_added_<cold|fire|lightning>_damage`, `..._maximum_...`:
///   "Adds # to # Cold Damage to Spells" (`ColdMin` BASE, `KeywordFlag.Spell`;
///   the physical and chaos ones, which PoB reads too, stay unmapped);
/// - `damage_+%`, `spell_damage_+%`, `elemental_damage_+%`, `cold_damage_+%`,
///   `fire_damage_+%`, `lightning_damage_+%`: increased damage (`Damage` INC;
///   with `ModFlag.Spell`; `ElementalDamage`, `ColdDamage`, ... INC, which
///   PoB reads for a hit of that type);
/// - `support_controlled_destruction_spell_damage_+%_final`: "Supported
///   Skills deal #% more Spell Damage" (`Damage` MORE, `ModFlag.Spell`);
/// - `critical_strike_chance_+%`, `spell_critical_strike_chance_+%`:
///   increased critical strike chance (`CritChance` INC; with
///   `ModFlag.Spell`); `additional_base_critical_strike_chance`: "+#% to
///   Critical Strike Chance" in hundredths (`CritChance` BASE, `div = 100`);
///   `support_controlled_destruction_critical_strike_chance_+%_final`: less
///   critical strike chance (`CritChance` MORE);
/// - `base_critical_strike_multiplier_+`: "+#% to Global Critical Strike
///   Multiplier" (`CritMultiplier` BASE);
/// - `base_cast_speed_+%`: "#% increased Cast Speed" (`Speed` INC,
///   `ModFlag.Cast`);
/// - `base_reduce_enemy_<cold|fire|lightning>_resistance_%`: "Damage
///   Penetrates #% Cold Resistance" (`ColdPenetration` BASE).
pub const HIT_ROWS: &[HitRow] = &[
    (
        "additional_base_critical_strike_chance",
        &[(Quantity::CritChance, Op::Base, 100.0, NONE, NONE)],
    ),
    (
        "base_cast_speed_+%",
        &[(Quantity::Speed, Op::Increased, 1.0, Flags::CAST, NONE)],
    ),
    (
        "base_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "base_reduce_enemy_cold_resistance_%",
        &[(Quantity::Penetration(COLD_T), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "base_reduce_enemy_fire_resistance_%",
        &[(Quantity::Penetration(FIRE_T), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "base_reduce_enemy_lightning_resistance_%",
        &[(
            Quantity::Penetration(LIGHTNING_T),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    (
        "cold_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::COLD, NONE)],
    ),
    (
        "critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, NONE, NONE)],
    ),
    (
        "damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, NONE, NONE)],
    ),
    (
        "elemental_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, NONE, Flags::ELEMENTAL)],
    ),
    (
        "fire_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::FIRE, NONE)],
    ),
    (
        "global_maximum_added_cold_damage",
        &[(added(COLD_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "global_maximum_added_lightning_damage",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "global_minimum_added_cold_damage",
        &[(added(COLD_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "global_minimum_added_lightning_damage",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "lightning_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::LIGHTNING, NONE)],
    ),
    (
        "spell_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_maximum_added_cold_damage",
        &[(added(COLD_T, MAX), Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_maximum_added_fire_damage",
        &[(added(FIRE_T, MAX), Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_maximum_added_lightning_damage",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_maximum_base_chaos_damage",
        &[(skill_base(CHAOS_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_maximum_base_cold_damage",
        &[(skill_base(COLD_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_maximum_base_fire_damage",
        &[(skill_base(FIRE_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_maximum_base_lightning_damage",
        &[(skill_base(LIGHTNING_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_maximum_base_physical_damage",
        &[(skill_base(PHYS, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_minimum_added_cold_damage",
        &[(added(COLD_T, MIN), Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_minimum_added_fire_damage",
        &[(added(FIRE_T, MIN), Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_minimum_added_lightning_damage",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "spell_minimum_base_chaos_damage",
        &[(skill_base(CHAOS_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_minimum_base_cold_damage",
        &[(skill_base(COLD_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_minimum_base_fire_damage",
        &[(skill_base(FIRE_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_minimum_base_lightning_damage",
        &[(skill_base(LIGHTNING_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "spell_minimum_base_physical_damage",
        &[(skill_base(PHYS, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "support_controlled_destruction_critical_strike_chance_+%_final",
        &[(Quantity::CritChance, Op::More, 1.0, NONE, NONE)],
    ),
    (
        "support_controlled_destruction_spell_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
];

/// Whether `quantity` is read only by the main skill's hit: an effect of
/// a gem may change it, since the gem acts only on the skill it supports
/// (PoB's `skillModList`); any other quantity it would change is the
/// character's.
#[must_use]
pub const fn is_hit_quantity(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Damage
            | Quantity::Speed
            | Quantity::CritChance
            | Quantity::CritMultiplier
            | Quantity::Accuracy
            | Quantity::AddedDamage(..)
            | Quantity::Penetration(..)
            | Quantity::Poe1(Poe1Quantity::SkillBaseDamage(..))
    )
}

/// Skill stats the hit phase reads from the gem instance itself (the
/// skill's flags), so the mapping gives them no effect: `is_area_damage`
/// makes the skill's hits area damage (PoB's hand-written `area` flag in
/// its skill data; P3-9b reads the game's stat instead).
pub const SKILL_FLAGS: &[&str] = &["is_area_damage"];

/// The mapping of a gem's stats: the game's ([`Poe1Mapping`]), where every
/// effect must be one only the skill's hit reads ([`is_hit_quantity`]);
/// a stat with another effect is `Unmapped` (PoB applies a gem's stats to
/// the skill alone, so a global reading would be wrong), and the skill's
/// flags ([`SKILL_FLAGS`]) have no effect.
#[derive(Debug, Clone, Copy, Default)]
pub struct SkillMapping;

impl StatMapping for SkillMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        if SKILL_FLAGS.contains(&stat.as_str()) {
            return StatMap::NoEffect;
        }
        match Poe1Mapping.map(stat) {
            StatMap::Effects(e) if e.iter().all(|t| is_hit_quantity(t.quantity)) => {
                StatMap::Effects(e)
            }
            StatMap::Effects(_) => StatMap::Unmapped,
            other => other,
        }
    }
}

const STR: Quantity = Quantity::Strength;
const DEX: Quantity = Quantity::Dexterity;
const INT: Quantity = Quantity::Intelligence;
const FIRE: Quantity = Quantity::Resistance(DamageType::Fire);
const COLD: Quantity = Quantity::Resistance(DamageType::Cold);
const LIGHTNING: Quantity = Quantity::Resistance(DamageType::Lightning);
const CHAOS: Quantity = Quantity::Resistance(DamageType::Chaos);
const MAX_FIRE: Quantity = Quantity::MaximumResistance(DamageType::Fire);
const MAX_COLD: Quantity = Quantity::MaximumResistance(DamageType::Cold);
const MAX_LIGHTNING: Quantity = Quantity::MaximumResistance(DamageType::Lightning);
const MAX_CHAOS: Quantity = Quantity::MaximumResistance(DamageType::Chaos);
const BLOCK: Quantity = Quantity::BlockChance;
const SPELL_BLOCK: Quantity = Quantity::Poe1(Poe1Quantity::SpellBlockChance);
const MAX_SPELL_BLOCK: Quantity = Quantity::Poe1(Poe1Quantity::MaximumSpellBlockChance);
const SUPPRESSION: Quantity = Quantity::Poe1(Poe1Quantity::SpellSuppressionChance);

/// The mapped stats, sorted by stat ID (a test checks the order).
///
/// - `base_maximum_life`: "+# to maximum Life" (PoB `Life` BASE).
/// - `maximum_life_+%`: "#% increased maximum Life" (`Life` INC).
/// - `base_maximum_mana`, `maximum_mana_+%`: the same for mana.
/// - `base_maximum_energy_shield`: "+# to maximum Energy Shield", the
///   global stat (an armour piece's own is `local_energy_shield`, P3-7)
///   (`EnergyShield` BASE).
/// - `maximum_energy_shield_+%`: "#% increased maximum Energy Shield"
///   (`EnergyShield` INC).
/// - `global_defences_+%`: "#% increased Global Defences" (PoB
///   `Defences` INC, which its energy shield, armour, and evasion sums
///   read).
/// - `additional_*`: "+# to Strength" and the pairs and "all Attributes"
///   (PoB `Str`, `Dex`, `Int` BASE; the pairs and all attributes read as
///   each attribute).
/// - `base_strength`, `base_dexterity`, `base_intelligence` and the
///   pairs: the passive tree's "+# to Strength", ... (the same text,
///   `passive_skill_stat_descriptions.txt`; PoB reads the node's line as
///   above).
/// - `*_+%`: "#% increased Strength", ... (`Str` INC, ...).
///
/// P3-7 adds the global defences (`stat_descriptions.txt`; PoB's reading
/// of the same text):
/// - `base_physical_damage_reduction_rating`, `base_evasion_rating`: "+#
///   to Armour", "+# to Evasion Rating" (`Armour`, `Evasion` BASE; the
///   character template's `base_evasion_rating` is the same stat);
/// - `physical_damage_reduction_rating_+%`, `evasion_rating_+%`,
///   `evasion_and_physical_damage_reduction_rating_+%`: "#% increased
///   Armour", "... Evasion Rating", "... Evasion Rating and Armour"
///   (`Armour` INC, `Evasion` INC, `ArmourAndEvasion` INC, which both
///   sums read);
/// - `base_<type>_damage_resistance_%`, `base_resist_all_elements_%`, the
///   pairs (`fire_and_cold_damage_resistance_%`, ...), `resist_all_%`:
///   "+#% to Fire Resistance", "+#% to all Elemental Resistances", "+#%
///   to Fire and Cold Resistances", "+#% to All Resistances" (`FireResist`
///   BASE, `ElementalResist` BASE, ...);
/// - `base_maximum_<type>_damage_resistance_%`,
///   `additional_maximum_all_resistances_%`,
///   `additional_maximum_all_elemental_resistances_%`,
///   `base_maximum_all_resistances_%`: "+#% to maximum Fire Resistance",
///   "+#% to all maximum Resistances", "+#% to all maximum Elemental
///   Resistances" (`FireResistMax` BASE, ...; the template's base 75 is the
///   last stat);
/// - `additional_block_%`: "+#% Chance to Block Attack Damage"
///   (`BlockChance` BASE); `block_chance_+%`: "#% increased Chance to
///   Block Attack and Spell Damage" (`BlockChance` INC, which spell block
///   reads too); `maximum_block_%`: "+#% to maximum Chance to Block Attack
///   Damage" (`BlockChanceMax` BASE, the template's 75 included);
/// - `base_spell_block_%`, `additional_spell_block_%`: "#% Chance to
///   Block Spell Damage" (`SpellBlockChance` BASE);
///   `base_maximum_spell_block_%`: "+#% to maximum Chance to Block Spell
///   Damage" (`SpellBlockChanceMax` BASE);
/// - `base_spell_suppression_chance_%`: "+#% chance to Suppress Spell
///   Damage" (`SpellSuppressionChance` BASE). Its sibling
///   `base_spell_suppression_chance_150%_of_value` has the same text and a
///   name saying the game applies 150% of it; until that is settled it is
///   unmapped (the fields are `Partial`).
pub const ROWS: &[Row] = &[
    (
        "additional_all_attributes",
        &[
            (STR, Op::Base, 1.0),
            (DEX, Op::Base, 1.0),
            (INT, Op::Base, 1.0),
        ],
    ),
    ("additional_block_%", &[(BLOCK, Op::Base, 1.0)]),
    ("additional_dexterity", &[(DEX, Op::Base, 1.0)]),
    (
        "additional_dexterity_and_intelligence",
        &[(DEX, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    ("additional_intelligence", &[(INT, Op::Base, 1.0)]),
    (
        "additional_maximum_all_elemental_resistances_%",
        &[
            (MAX_FIRE, Op::Base, 1.0),
            (MAX_COLD, Op::Base, 1.0),
            (MAX_LIGHTNING, Op::Base, 1.0),
        ],
    ),
    (
        "additional_maximum_all_resistances_%",
        &[
            (MAX_FIRE, Op::Base, 1.0),
            (MAX_COLD, Op::Base, 1.0),
            (MAX_LIGHTNING, Op::Base, 1.0),
            (MAX_CHAOS, Op::Base, 1.0),
        ],
    ),
    ("additional_spell_block_%", &[(SPELL_BLOCK, Op::Base, 1.0)]),
    ("additional_strength", &[(STR, Op::Base, 1.0)]),
    (
        "additional_strength_and_dexterity",
        &[(STR, Op::Base, 1.0), (DEX, Op::Base, 1.0)],
    ),
    (
        "additional_strength_and_intelligence",
        &[(STR, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    (
        "all_attributes_+%",
        &[
            (STR, Op::Increased, 1.0),
            (DEX, Op::Increased, 1.0),
            (INT, Op::Increased, 1.0),
        ],
    ),
    ("base_chaos_damage_resistance_%", &[(CHAOS, Op::Base, 1.0)]),
    ("base_cold_damage_resistance_%", &[(COLD, Op::Base, 1.0)]),
    ("base_dexterity", &[(DEX, Op::Base, 1.0)]),
    (
        "base_dexterity_and_intelligence",
        &[(DEX, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    ("base_evasion_rating", &[(Quantity::Evasion, Op::Base, 1.0)]),
    ("base_fire_damage_resistance_%", &[(FIRE, Op::Base, 1.0)]),
    ("base_intelligence", &[(INT, Op::Base, 1.0)]),
    (
        "base_lightning_damage_resistance_%",
        &[(LIGHTNING, Op::Base, 1.0)],
    ),
    (
        "base_maximum_all_resistances_%",
        &[
            (MAX_FIRE, Op::Base, 1.0),
            (MAX_COLD, Op::Base, 1.0),
            (MAX_LIGHTNING, Op::Base, 1.0),
            (MAX_CHAOS, Op::Base, 1.0),
        ],
    ),
    (
        "base_maximum_chaos_damage_resistance_%",
        &[(MAX_CHAOS, Op::Base, 1.0)],
    ),
    (
        "base_maximum_cold_damage_resistance_%",
        &[(MAX_COLD, Op::Base, 1.0)],
    ),
    (
        "base_maximum_energy_shield",
        &[(Quantity::EnergyShield, Op::Base, 1.0)],
    ),
    (
        "base_maximum_fire_damage_resistance_%",
        &[(MAX_FIRE, Op::Base, 1.0)],
    ),
    ("base_maximum_life", &[(Quantity::Life, Op::Base, 1.0)]),
    (
        "base_maximum_lightning_damage_resistance_%",
        &[(MAX_LIGHTNING, Op::Base, 1.0)],
    ),
    ("base_maximum_mana", &[(Quantity::Mana, Op::Base, 1.0)]),
    (
        "base_maximum_spell_block_%",
        &[(MAX_SPELL_BLOCK, Op::Base, 1.0)],
    ),
    (
        "base_physical_damage_reduction_rating",
        &[(Quantity::Armour, Op::Base, 1.0)],
    ),
    (
        "base_resist_all_elements_%",
        &[
            (FIRE, Op::Base, 1.0),
            (COLD, Op::Base, 1.0),
            (LIGHTNING, Op::Base, 1.0),
        ],
    ),
    ("base_spell_block_%", &[(SPELL_BLOCK, Op::Base, 1.0)]),
    (
        "base_spell_suppression_chance_%",
        &[(SUPPRESSION, Op::Base, 1.0)],
    ),
    ("base_strength", &[(STR, Op::Base, 1.0)]),
    (
        "base_strength_and_dexterity",
        &[(STR, Op::Base, 1.0), (DEX, Op::Base, 1.0)],
    ),
    (
        "base_strength_and_intelligence",
        &[(STR, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    ("block_chance_+%", &[(BLOCK, Op::Increased, 1.0)]),
    (
        "cold_and_chaos_damage_resistance_%",
        &[(COLD, Op::Base, 1.0), (CHAOS, Op::Base, 1.0)],
    ),
    (
        "cold_and_lightning_damage_resistance_%",
        &[(COLD, Op::Base, 1.0), (LIGHTNING, Op::Base, 1.0)],
    ),
    ("dexterity_+%", &[(DEX, Op::Increased, 1.0)]),
    (
        "evasion_and_physical_damage_reduction_rating_+%",
        &[
            (Quantity::Armour, Op::Increased, 1.0),
            (Quantity::Evasion, Op::Increased, 1.0),
        ],
    ),
    (
        "evasion_rating_+%",
        &[(Quantity::Evasion, Op::Increased, 1.0)],
    ),
    (
        "fire_and_chaos_damage_resistance_%",
        &[(FIRE, Op::Base, 1.0), (CHAOS, Op::Base, 1.0)],
    ),
    (
        "fire_and_cold_damage_resistance_%",
        &[(FIRE, Op::Base, 1.0), (COLD, Op::Base, 1.0)],
    ),
    (
        "fire_and_lightning_damage_resistance_%",
        &[(FIRE, Op::Base, 1.0), (LIGHTNING, Op::Base, 1.0)],
    ),
    (
        "global_defences_+%",
        &[
            (Quantity::EnergyShield, Op::Increased, 1.0),
            (Quantity::Armour, Op::Increased, 1.0),
            (Quantity::Evasion, Op::Increased, 1.0),
        ],
    ),
    ("intelligence_+%", &[(INT, Op::Increased, 1.0)]),
    (
        "lightning_and_chaos_damage_resistance_%",
        &[(LIGHTNING, Op::Base, 1.0), (CHAOS, Op::Base, 1.0)],
    ),
    (
        "maximum_block_%",
        &[(Quantity::MaximumBlockChance, Op::Base, 1.0)],
    ),
    (
        "maximum_energy_shield_+%",
        &[(Quantity::EnergyShield, Op::Increased, 1.0)],
    ),
    ("maximum_life_+%", &[(Quantity::Life, Op::Increased, 1.0)]),
    ("maximum_mana_+%", &[(Quantity::Mana, Op::Increased, 1.0)]),
    (
        "physical_damage_reduction_rating_+%",
        &[(Quantity::Armour, Op::Increased, 1.0)],
    ),
    (
        "resist_all_%",
        &[
            (FIRE, Op::Base, 1.0),
            (COLD, Op::Base, 1.0),
            (LIGHTNING, Op::Base, 1.0),
            (CHAOS, Op::Base, 1.0),
        ],
    ),
    ("strength_+%", &[(STR, Op::Increased, 1.0)]),
];

/// The Path of Exile stat-to-effect mapping: [`ROWS`]; every other stat
/// is `Unmapped`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Poe1Mapping;

/// Stats that reach no slice field, whatever the build's config (41): `(stat,
/// why)`, sorted by stat ID (P3-8a). Each is read as PoB v2.67.2 reads its
/// text, and none feeds a slice field (attributes, life, mana, energy
/// shield, armour, evasion, resistances, block, spell block, spell
/// suppression, unreserved life and mana, and the main skill's hit,
/// speed, hit chance, crit, and hit DPS) on its own.
///
/// What is left out on purpose, so it stays `Unmapped` until a step maps
/// it: anything that can change a hit or its rate through the enemy
/// (shock, chill, exposure, intimidate, consecrated ground's effect,
/// curses), charges' counts, rage, area, gem levels, minion damage and
/// speed (keystones redirect them to the player), and staff or shield
/// conditioned block and damage, fortification's maximum (PoB scales
/// suppression, armour, evasion, and attack speed per fortification), and
/// `base_self_critical_strike_multiplier_-%` (PoB's skill stat map also
/// reads it as the enemy's `SelfCritMultiplier`, which reaches the main
/// skill's crit multiplier). A stat that makes one of these rows matter is
/// itself unmapped, so the fields stay `Partial` while it is in the
/// build: rage regenerated per life or mana regenerated (PoB
/// `ModParser.lua` `RageRegen` per `LifeRegen`, then rage's damage),
/// and any `PerStat` on a row's PoB mod. Mapping such a converter later
/// must revisit the rows it reaches; the guarantee holds only while it is
/// unmapped.
pub const NO_EFFECT: &[(&str, &str)] = &[
    // A skill's own base radius (PoB `skill("radius")`): it sets the area
    // PoB shows, which no slice field reads (P3-9b). Increased area stays
    // unmapped, as above.
    ("active_skill_base_area_of_effect_radius", "skill-radius"),
    ("add_power_charge_on_kill_%_chance", "charge-gain-chance"),
    ("avoid_interruption_while_casting_%", "interruption"),
    ("base_chance_to_freeze_%", "freeze"),
    (
        "base_energy_shield_leech_from_spell_damage_permyriad",
        "leech",
    ),
    ("base_movement_velocity_+%", "movement-speed"),
    ("base_stun_duration_+%", "stun"),
    (
        "cannot_be_shocked_while_at_maximum_power_charges",
        "ailment-on-self",
    ),
    ("cannot_have_life_leeched_from", "leech"),
    (
        "chance_to_create_consecrated_ground_on_melee_kill_%",
        "ground-creation-chance",
    ),
    ("chance_to_fortify_on_melee_hit_+%", "fortification"),
    (
        "chance_to_gain_power_charge_when_block_%",
        "charge-gain-chance",
    ),
    ("cold_ailment_duration_+%", "ailment-duration"),
    (
        "consecrated_ground_allies_recover_es_as_well_as_life_from_life_regeneration",
        "regeneration",
    ),
    (
        "elemental_ailment_duration_on_self_+%_while_holding_shield",
        "ailment-on-self",
    ),
    (
        "energy_shield_regeneration_rate_per_minute_if_rare_or_unique_enemy_nearby",
        "regeneration",
    ),
    ("fortify_duration_+%", "fortification"),
    ("freeze_duration_+%", "ailment-duration"),
    (
        "gain_power_charge_for_each_second_channeling_spell",
        "charge-gain-chance",
    ),
    ("gloves_freeze_proliferation_radius", "freeze"),
    ("golem_immunity_to_elemental_damage", "minion-defence"),
    ("golem_maximum_life_+%", "minion-defence"),
    ("item_generation_can_have_multiple_crafted_mods", "crafting"),
    (
        "life_regeneration_per_minute_%_per_fortification",
        "regeneration",
    ),
    ("life_regeneration_rate_per_minute_%", "regeneration"),
    (
        "life_regeneration_rate_per_minute_%_per_endurance_charge",
        "regeneration",
    ),
    (
        "life_regeneration_rate_per_minute_per_level",
        "regeneration",
    ),
    ("lightning_ailment_duration_+%", "ailment-duration"),
    ("local_has_X_abyss_sockets", "sockets"),
    ("mana_regeneration_rate_+%", "regeneration"),
    (
        "mana_regeneration_rate_+%_per_1%_spell_block_chance",
        "regeneration",
    ),
    ("mana_regeneration_rate_+%_per_power_charge", "regeneration"),
    (
        "mana_regeneration_rate_per_minute_%_if_inflicted_exposure_recently",
        "regeneration",
    ),
    (
        "minion_life_regeneration_rate_per_minute_%",
        "minion-defence",
    ),
    ("minion_maximum_life_+%", "minion-defence"),
    ("physical_damage_taken_%_as_fire", "damage-taken"),
    ("physical_damage_taken_%_as_lightning", "damage-taken"),
    ("rage_loss_delay_ms", "rage-duration"),
    ("regenerate_energy_shield_instead_of_life", "regeneration"),
    ("revive_golems_if_killed_by_enemies_ms", "minion-defence"),
    (
        "your_consecrated_ground_effect_lingers_for_ms_after_leaving_the_area",
        "ground-duration",
    ),
];

impl StatMapping for Poe1Mapping {
    fn map(&self, stat: &StatId) -> StatMap {
        if NO_EFFECT
            .binary_search_by(|(id, _)| (*id).cmp(stat.as_str()))
            .is_ok()
        {
            return StatMap::NoEffect;
        }
        if let Ok(i) = HIT_ROWS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str())) {
            return StatMap::Effects(
                HIT_ROWS[i]
                    .1
                    .iter()
                    .map(|&(quantity, op, divisor, all, any)| EffectTemplate {
                        quantity,
                        op,
                        scale: 1.0,
                        divisor,
                        conditions: Conditions::none().with_all(all).with_any(any),
                    })
                    .collect(),
            );
        }
        ROWS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str()))
            .map_or(StatMap::Unmapped, |i| {
                StatMap::Effects(
                    ROWS[i]
                        .1
                        .iter()
                        .map(|&(quantity, op, scale)| EffectTemplate {
                            quantity,
                            op,
                            scale,
                            divisor: 1.0,
                            conditions: Conditions::none(),
                        })
                        .collect(),
                )
            })
    }
}

/// What an armour piece's local stat changes on the piece: `(stat,
/// [(value, op)])`, sorted by stat ID. PoB reads the same lines as local
/// `Armour`, `Evasion`, `EnergyShield`, `ArmourAndEvasion`,
/// `ArmourAndEnergyShield`, `EvasionAndEnergyShield`, `Defences`, and
/// `BlockChance` mods, which `calcLocal` takes for the item's own values
/// (`Classes/Item.lua`, the armour branch): a hybrid acts on each defence
/// it names. Ward is not in the slice: its local stats have no effect on
/// the computed fields.
type LocalRow = (&'static str, &'static [(PieceValue, Op)]);

const ARMOUR: PieceValue = PieceValue::Defence(Defence::Armour);
const EVASION: PieceValue = PieceValue::Defence(Defence::Evasion);
const ES: PieceValue = PieceValue::Defence(Defence::EnergyShield);

/// The local stats of armour pieces (`stat_descriptions.txt`: "+# to
/// Armour", "+# to Evasion Rating", "+# to maximum Energy Shield", "+# to
/// Evasion Rating and Energy Shield", "#% increased Armour", ..., "#%
/// increased Armour, Evasion and Energy Shield", "+#% Chance to Block",
/// "#% increased Chance to Block"; each with `local` set in the stats
/// table).
pub const LOCAL_ROWS: &[LocalRow] = &[
    (
        "local_additional_block_chance_%",
        &[(PieceValue::Block, Op::Base)],
    ),
    (
        "local_armour_and_energy_shield_+%",
        &[(ARMOUR, Op::Increased), (ES, Op::Increased)],
    ),
    (
        "local_armour_and_evasion_+%",
        &[(ARMOUR, Op::Increased), (EVASION, Op::Increased)],
    ),
    (
        "local_armour_and_evasion_and_energy_shield_+%",
        &[
            (ARMOUR, Op::Increased),
            (EVASION, Op::Increased),
            (ES, Op::Increased),
        ],
    ),
    ("local_base_evasion_rating", &[(EVASION, Op::Base)]),
    (
        "local_base_physical_damage_reduction_rating",
        &[(ARMOUR, Op::Base)],
    ),
    (
        "local_block_chance_+%",
        &[(PieceValue::Block, Op::Increased)],
    ),
    ("local_energy_shield", &[(ES, Op::Base)]),
    ("local_energy_shield_+%", &[(ES, Op::Increased)]),
    (
        "local_evasion_and_energy_shield_+%",
        &[(EVASION, Op::Increased), (ES, Op::Increased)],
    ),
    ("local_evasion_rating_+%", &[(EVASION, Op::Increased)]),
    (
        "local_evasion_rating_and_energy_shield",
        &[(EVASION, Op::Base), (ES, Op::Base)],
    ),
    (
        "local_physical_damage_reduction_rating_+%",
        &[(ARMOUR, Op::Increased)],
    ),
    ("local_ward", &[]),
    ("local_ward_+%", &[]),
];

/// The mapping of an armour piece's lines: its local stats act on the
/// piece in `slot` ([`LOCAL_ROWS`]); every other stat is the game's
/// ([`Poe1Mapping`]).
#[derive(Debug, Clone, Copy)]
pub struct PieceMapping {
    /// The slot the piece is in.
    pub slot: ArmourSlot,
}

impl StatMapping for PieceMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        let Ok(i) = LOCAL_ROWS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str())) else {
            return Poe1Mapping.map(stat);
        };
        let effects = LOCAL_ROWS[i].1;
        if effects.is_empty() {
            return StatMap::NoEffect;
        }
        StatMap::Effects(
            effects
                .iter()
                .map(|&(value, op)| EffectTemplate {
                    quantity: Quantity::Poe1(Poe1Quantity::Piece(self.slot, value)),
                    op,
                    scale: 1.0,
                    divisor: 1.0,
                    conditions: Conditions::none(),
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(stat: &str) -> Vec<(Quantity, Op, f64)> {
        match Poe1Mapping.map(&StatId::new(stat)) {
            StatMap::Effects(e) => e.into_iter().map(|t| (t.quantity, t.op, t.scale)).collect(),
            other => panic!("{stat}: {other:?}"),
        }
    }

    #[test]
    fn rows_should_be_sorted_and_unique() {
        assert!(ROWS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(HIT_ROWS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(LOCAL_ROWS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(NO_EFFECT.windows(2).all(|w| w[0].0 < w[1].0));
        for (stat, _) in HIT_ROWS {
            assert!(
                ROWS.binary_search_by(|(id, _)| (*id).cmp(stat)).is_err()
                    && NO_EFFECT.binary_search_by(|(id, _)| id.cmp(stat)).is_err(),
                "{stat} is in two tables"
            );
        }
    }

    fn hit(stat: &str) -> Vec<(Quantity, Op, f64, Conditions)> {
        match Poe1Mapping.map(&StatId::new(stat)) {
            StatMap::Effects(e) => e
                .into_iter()
                .map(|t| (t.quantity, t.op, t.divisor, t.conditions))
                .collect(),
            other => panic!("{stat}: {other:?}"),
        }
    }

    #[test]
    fn hit_stats_should_map_as_pob_reads_them() {
        let none = Conditions::none;
        let spell = || Conditions::none().with_all(Flags::SPELL);
        assert_eq!(
            hit("spell_minimum_base_cold_damage"),
            [(skill_base(COLD_T, MIN), Op::Base, 1.0, none())]
        );
        assert_eq!(
            hit("spell_maximum_added_cold_damage"),
            [(added(COLD_T, MAX), Op::Base, 1.0, spell())]
        );
        assert_eq!(
            hit("global_minimum_added_cold_damage"),
            [(added(COLD_T, MIN), Op::Base, 1.0, none())]
        );
        assert_eq!(
            hit("spell_damage_+%"),
            [(Quantity::Damage, Op::Increased, 1.0, spell())]
        );
        assert_eq!(
            hit("cold_damage_+%"),
            [(
                Quantity::Damage,
                Op::Increased,
                1.0,
                Conditions::none().with_all(Flags::COLD)
            )]
        );
        assert_eq!(
            hit("elemental_damage_+%"),
            [(
                Quantity::Damage,
                Op::Increased,
                1.0,
                Conditions::none().with_any(Flags::ELEMENTAL)
            )]
        );
        assert_eq!(
            hit("support_controlled_destruction_spell_damage_+%_final"),
            [(Quantity::Damage, Op::More, 1.0, spell())]
        );
        assert_eq!(
            hit("additional_base_critical_strike_chance"),
            [(Quantity::CritChance, Op::Base, 100.0, none())]
        );
        assert_eq!(
            hit("base_cast_speed_+%"),
            [(
                Quantity::Speed,
                Op::Increased,
                1.0,
                Conditions::none().with_all(Flags::CAST)
            )]
        );
        assert_eq!(
            hit("base_reduce_enemy_cold_resistance_%"),
            [(Quantity::Penetration(COLD_T), Op::Base, 1.0, none())]
        );
        // PoB has no row for these: they stay unmapped.
        for stat in [
            "global_minimum_added_fire_damage",
            "global_minimum_added_physical_damage",
        ] {
            assert_eq!(
                Poe1Mapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
        }
        // No mapped stat reads a named condition, so the default enemy's
        // conditions (rare or unique, Pinnacle boss) change no mapped value.
        for stat in HIT_ROWS.iter().map(|r| r.0).chain(ROWS.iter().map(|r| r.0)) {
            if let StatMap::Effects(e) = Poe1Mapping.map(&StatId::new(stat)) {
                assert!(e.iter().all(|t| t.conditions.states.is_empty()), "{stat}");
            }
        }
    }

    #[test]
    fn a_gems_stats_should_act_only_on_its_skill() {
        // A hit stat maps as the game's.
        assert_eq!(
            SkillMapping.map(&StatId::new("spell_damage_+%")),
            Poe1Mapping.map(&StatId::new("spell_damage_+%"))
        );
        // A character stat on a gem would act on the character in PoB only
        // through a buff, which is not modelled: unmapped, never global.
        assert_eq!(
            SkillMapping.map(&StatId::new("maximum_life_+%")),
            StatMap::Unmapped
        );
        assert_eq!(
            SkillMapping.map(&StatId::new("is_area_damage")),
            StatMap::NoEffect
        );
        assert_eq!(
            SkillMapping.map(&StatId::new("active_skill_base_area_of_effect_radius")),
            StatMap::NoEffect
        );
        assert_eq!(
            SkillMapping.map(&StatId::new("unknown_skill_stat")),
            StatMap::Unmapped
        );
    }

    #[test]
    fn no_effect_stats_should_be_known_and_never_mapped() {
        for &(stat, why) in NO_EFFECT {
            assert_eq!(
                Poe1Mapping.map(&StatId::new(stat)),
                StatMap::NoEffect,
                "{stat}"
            );
            assert!(!why.is_empty());
            assert!(
                ROWS.binary_search_by(|(id, _)| (*id).cmp(stat)).is_err(),
                "{stat} is both mapped and without effect"
            );
            assert!(
                LOCAL_ROWS
                    .binary_search_by(|(id, _)| (*id).cmp(stat))
                    .is_err(),
                "{stat} is both a local piece stat and without effect"
            );
            // An armour piece's lines see the same.
            let piece = PieceMapping {
                slot: ArmourSlot::Helmet,
            };
            assert_eq!(piece.map(&StatId::new(stat)), StatMap::NoEffect, "{stat}");
        }
        // Left out on purpose: they can change a slice field.
        for stat in [
            "lightning_ailment_effect_+%",
            "chill_effect_+%",
            "minion_damage_+%",
            "maximum_life_+%_if_no_life_tags_on_body_armour",
            "base_mana_reservation_efficiency_+%",
            "action_speed_-%",
            "max_power_charges",
            "base_max_fortification",
            "base_self_critical_strike_multiplier_-%",
        ] {
            assert_eq!(
                Poe1Mapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
        }
    }

    #[test]
    fn global_armour_and_evasion_should_map_to_base_and_increased() {
        let armour = Quantity::Armour;
        let evasion = Quantity::Evasion;
        assert_eq!(
            effects("base_physical_damage_reduction_rating"),
            [(armour, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("physical_damage_reduction_rating_+%"),
            [(armour, Op::Increased, 1.0)]
        );
        assert_eq!(effects("base_evasion_rating"), [(evasion, Op::Base, 1.0)]);
        assert_eq!(
            effects("evasion_rating_+%"),
            [(evasion, Op::Increased, 1.0)]
        );
        assert_eq!(
            effects("evasion_and_physical_damage_reduction_rating_+%"),
            [(armour, Op::Increased, 1.0), (evasion, Op::Increased, 1.0)]
        );
    }

    #[test]
    fn resistance_stats_should_reach_each_type_they_name() {
        let base = |qs: &[Quantity]| qs.iter().map(|q| (*q, Op::Base, 1.0)).collect::<Vec<_>>();
        assert_eq!(effects("base_fire_damage_resistance_%"), base(&[FIRE]));
        assert_eq!(effects("base_cold_damage_resistance_%"), base(&[COLD]));
        assert_eq!(
            effects("base_lightning_damage_resistance_%"),
            base(&[LIGHTNING])
        );
        assert_eq!(effects("base_chaos_damage_resistance_%"), base(&[CHAOS]));
        assert_eq!(
            effects("base_resist_all_elements_%"),
            base(&[FIRE, COLD, LIGHTNING])
        );
        assert_eq!(
            effects("resist_all_%"),
            base(&[FIRE, COLD, LIGHTNING, CHAOS])
        );
        for (stat, pair) in [
            ("fire_and_cold_damage_resistance_%", [FIRE, COLD]),
            ("fire_and_lightning_damage_resistance_%", [FIRE, LIGHTNING]),
            ("cold_and_lightning_damage_resistance_%", [COLD, LIGHTNING]),
            ("fire_and_chaos_damage_resistance_%", [FIRE, CHAOS]),
            ("cold_and_chaos_damage_resistance_%", [COLD, CHAOS]),
            (
                "lightning_and_chaos_damage_resistance_%",
                [LIGHTNING, CHAOS],
            ),
        ] {
            assert_eq!(effects(stat), base(&pair), "{stat}");
        }
        for (stat, one) in [
            ("base_maximum_fire_damage_resistance_%", MAX_FIRE),
            ("base_maximum_cold_damage_resistance_%", MAX_COLD),
            ("base_maximum_lightning_damage_resistance_%", MAX_LIGHTNING),
            ("base_maximum_chaos_damage_resistance_%", MAX_CHAOS),
        ] {
            assert_eq!(effects(stat), base(&[one]), "{stat}");
        }
        let all_max = base(&[MAX_FIRE, MAX_COLD, MAX_LIGHTNING, MAX_CHAOS]);
        assert_eq!(effects("additional_maximum_all_resistances_%"), all_max);
        assert_eq!(effects("base_maximum_all_resistances_%"), all_max);
        assert_eq!(
            effects("additional_maximum_all_elemental_resistances_%"),
            base(&[MAX_FIRE, MAX_COLD, MAX_LIGHTNING])
        );
    }

    #[test]
    fn block_and_suppression_stats_should_map() {
        assert_eq!(effects("additional_block_%"), [(BLOCK, Op::Base, 1.0)]);
        assert_eq!(effects("block_chance_+%"), [(BLOCK, Op::Increased, 1.0)]);
        assert_eq!(
            effects("maximum_block_%"),
            [(Quantity::MaximumBlockChance, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("base_spell_block_%"),
            [(SPELL_BLOCK, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_spell_block_%"),
            [(SPELL_BLOCK, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("base_maximum_spell_block_%"),
            [(MAX_SPELL_BLOCK, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("base_spell_suppression_chance_%"),
            [(SUPPRESSION, Op::Base, 1.0)]
        );
        // Same text, a name saying 150% of the value: not settled.
        assert_eq!(
            Poe1Mapping.map(&StatId::new("base_spell_suppression_chance_150%_of_value")),
            StatMap::Unmapped
        );
    }

    #[test]
    fn a_piece_should_map_its_local_stats_to_itself_and_the_rest_globally() {
        let m = PieceMapping {
            slot: ArmourSlot::Gloves,
        };
        let piece = |stat: &str| match m.map(&StatId::new(stat)) {
            StatMap::Effects(e) => e
                .into_iter()
                .map(|t| (t.quantity, t.op))
                .collect::<Vec<_>>(),
            other => panic!("{stat}: {other:?}"),
        };
        let q = |v| Quantity::Poe1(Poe1Quantity::Piece(ArmourSlot::Gloves, v));
        assert_eq!(piece("local_energy_shield"), [(q(ES), Op::Base)]);
        assert_eq!(
            piece("local_base_physical_damage_reduction_rating"),
            [(q(ARMOUR), Op::Base)]
        );
        assert_eq!(piece("local_base_evasion_rating"), [(q(EVASION), Op::Base)]);
        assert_eq!(
            piece("local_evasion_rating_and_energy_shield"),
            [(q(EVASION), Op::Base), (q(ES), Op::Base)]
        );
        assert_eq!(
            piece("local_armour_and_evasion_and_energy_shield_+%"),
            [
                (q(ARMOUR), Op::Increased),
                (q(EVASION), Op::Increased),
                (q(ES), Op::Increased)
            ]
        );
        for (stat, values) in [
            ("local_physical_damage_reduction_rating_+%", vec![ARMOUR]),
            ("local_evasion_rating_+%", vec![EVASION]),
            ("local_energy_shield_+%", vec![ES]),
            ("local_armour_and_evasion_+%", vec![ARMOUR, EVASION]),
            ("local_armour_and_energy_shield_+%", vec![ARMOUR, ES]),
            ("local_evasion_and_energy_shield_+%", vec![EVASION, ES]),
        ] {
            let expect: Vec<_> = values.into_iter().map(|v| (q(v), Op::Increased)).collect();
            assert_eq!(piece(stat), expect, "{stat}");
        }
        assert_eq!(
            piece("local_additional_block_chance_%"),
            [(q(PieceValue::Block), Op::Base)]
        );
        assert_eq!(
            piece("local_block_chance_+%"),
            [(q(PieceValue::Block), Op::Increased)]
        );
        assert_eq!(m.map(&StatId::new("local_ward")), StatMap::NoEffect);
        assert_eq!(m.map(&StatId::new("local_ward_+%")), StatMap::NoEffect);
        // Global stats on the piece are the game's.
        assert_eq!(piece("base_maximum_life"), [(Quantity::Life, Op::Base)]);
        assert_eq!(
            m.map(&StatId::new("local_no_energy_shield")),
            StatMap::Unmapped
        );
    }

    #[test]
    fn an_unknown_stat_should_be_unmapped() {
        assert_eq!(
            Poe1Mapping.map(&StatId::new("local_energy_shield")),
            StatMap::Unmapped
        );
        assert_eq!(Poe1Mapping.map(&StatId::new("")), StatMap::Unmapped);
    }

    #[test]
    fn life_mana_and_energy_shield_should_map_to_base_and_increased() {
        assert_eq!(
            effects("base_maximum_life"),
            [(Quantity::Life, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("maximum_life_+%"),
            [(Quantity::Life, Op::Increased, 1.0)]
        );
        assert_eq!(
            effects("base_maximum_mana"),
            [(Quantity::Mana, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("maximum_mana_+%"),
            [(Quantity::Mana, Op::Increased, 1.0)]
        );
        assert_eq!(
            effects("base_maximum_energy_shield"),
            [(Quantity::EnergyShield, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("maximum_energy_shield_+%"),
            [(Quantity::EnergyShield, Op::Increased, 1.0)]
        );
    }

    #[test]
    fn global_defences_should_increase_each_defence() {
        assert_eq!(
            effects("global_defences_+%"),
            [
                (Quantity::EnergyShield, Op::Increased, 1.0),
                (Quantity::Armour, Op::Increased, 1.0),
                (Quantity::Evasion, Op::Increased, 1.0)
            ]
        );
    }

    #[test]
    fn attribute_stats_should_reach_each_attribute_they_name() {
        assert_eq!(effects("additional_strength"), [(STR, Op::Base, 1.0)]);
        assert_eq!(effects("additional_dexterity"), [(DEX, Op::Base, 1.0)]);
        assert_eq!(effects("additional_intelligence"), [(INT, Op::Base, 1.0)]);
        assert_eq!(
            effects("additional_strength_and_dexterity"),
            [(STR, Op::Base, 1.0), (DEX, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_strength_and_intelligence"),
            [(STR, Op::Base, 1.0), (INT, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_dexterity_and_intelligence"),
            [(DEX, Op::Base, 1.0), (INT, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_all_attributes"),
            [
                (STR, Op::Base, 1.0),
                (DEX, Op::Base, 1.0),
                (INT, Op::Base, 1.0)
            ]
        );
        // The passive tree's stats read as the items'.
        for (tree, item) in [
            ("base_strength", "additional_strength"),
            ("base_dexterity", "additional_dexterity"),
            ("base_intelligence", "additional_intelligence"),
            (
                "base_strength_and_dexterity",
                "additional_strength_and_dexterity",
            ),
            (
                "base_strength_and_intelligence",
                "additional_strength_and_intelligence",
            ),
            (
                "base_dexterity_and_intelligence",
                "additional_dexterity_and_intelligence",
            ),
        ] {
            assert_eq!(effects(tree), effects(item), "{tree}");
        }
        assert_eq!(effects("strength_+%"), [(STR, Op::Increased, 1.0)]);
        assert_eq!(effects("dexterity_+%"), [(DEX, Op::Increased, 1.0)]);
        assert_eq!(effects("intelligence_+%"), [(INT, Op::Increased, 1.0)]);
        assert_eq!(
            effects("all_attributes_+%"),
            [
                (STR, Op::Increased, 1.0),
                (DEX, Op::Increased, 1.0),
                (INT, Op::Increased, 1.0)
            ]
        );
    }
}

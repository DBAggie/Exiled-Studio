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
use crate::poe1::{ArmourSlot, Defence, PieceValue, Poe1Quantity, WeaponValue};
use crate::quantity::{Bound, DamageType, Op, Quantity};

/// One mapped effect: what, how, and the scale per unit of the stat.
type Row = (&'static str, &'static [(Quantity, Op, f64)]);

/// A mapped effect of a hit stat: what, how, the divisor of the stat's
/// value, and the skill flags it needs (all of the first, one of the
/// second when not empty).
pub type HitEffect = (Quantity, Op, f64, Flags, Flags);

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
///
/// P3-10 adds the stats of an attack's hit (`Data/SkillStatMap.lua` unless
/// named):
/// - `accuracy_rating`, `accuracy_rating_+%`: "+# to Accuracy Rating",
///   "#% increased Global Accuracy Rating" (`Accuracy` BASE, INC);
/// - `attack_minimum_added_<type>_damage`, `..._maximum_...`: "Adds # to #
///   Physical Damage to Attacks" (`PhysicalMin` BASE, `KeywordFlag.Attack`);
/// - `attack_damage_+%`, `melee_damage_+%`, `physical_damage_+%`:
///   increased attack, melee, and physical damage (`Damage` INC with
///   `ModFlag.Attack`, `ModFlag.Melee`; `PhysicalDamage` INC);
/// - `attack_speed_+%`: "#% increased Attack Speed" (`Speed` INC,
///   `ModFlag.Attack`);
/// - `active_skill_area_damage_+%_final`: a skill's more or less area
///   damage (`Damage` MORE, `ModFlag.Area`: Smite's Area Hit only);
/// - `minimum_added_lightning_damage_from_skill`, `maximum_...`: Smite's
///   aura (`Data/Skills/act_str.lua`: `LightningMin` BASE as a
///   `GlobalEffect` aura buff on the character, scaled by aura and buff
///   effect, whose stats are unmapped, so the buff is its value);
/// - `skill_physical_damage_%_to_convert_to_lightning`: the skill's
///   conversion (`SkillPhysicalDamageConvertToLightning` BASE);
/// - supports' own `statMap` rows (`Data/Skills/sup_*.lua`):
///   `support_attack_skills_elemental_damage_+%_final`
///   (`ElementalDamage` MORE, `KeywordFlag.Attack`),
///   `support_melee_physical_damage_+%_final` (`PhysicalDamage` MORE,
///   `ModFlag.Melee`), `support_melee_physical_damage_attack_speed_+%_final`
///   (`Speed` MORE, `ModFlag.Attack`).
///
/// P3-8c adds three global stats of the in-scope Smite builds
/// (`Data/SkillStatMap.lua` unless named):
/// - `attack_and_cast_speed_+%`: "#% increased Attack and Cast Speed"
///   (`Speed` INC, no flag: every skill PoB computes a rate for is an
///   attack or a cast);
/// - `elemental_damage_with_attack_skills_+%`: "#% increased Elemental
///   Damage with Attack Skills" (`ElementalDamage` INC,
///   `KeywordFlag.Attack`);
/// - `melee_critical_strike_chance_+%`: "#% increased Melee Critical Strike
///   Chance" (no stat map row; `ModParser.lua` reads the text as
///   `CritChance` INC with `ModFlag.Melee`, its "melee" flag word).
///
/// PoB tells `ModFlag.Attack` from `KeywordFlag.Attack`; both are the
/// `ATTACK` flag here, which every attack hit has and nothing else does.
pub const HIT_ROWS: &[HitRow] = &[
    (
        "accuracy_rating",
        &[(Quantity::Accuracy, Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "accuracy_rating_+%",
        &[(Quantity::Accuracy, Op::Increased, 1.0, NONE, NONE)],
    ),
    (
        "active_skill_area_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::AREA, NONE)],
    ),
    (
        "additional_base_critical_strike_chance",
        &[(Quantity::CritChance, Op::Base, 100.0, NONE, NONE)],
    ),
    (
        "attack_and_cast_speed_+%",
        &[(Quantity::Speed, Op::Increased, 1.0, NONE, NONE)],
    ),
    (
        "attack_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_maximum_added_chaos_damage",
        &[(added(CHAOS_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_maximum_added_cold_damage",
        &[(added(COLD_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_maximum_added_fire_damage",
        &[(added(FIRE_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_maximum_added_lightning_damage",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_maximum_added_physical_damage",
        &[(added(PHYS, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_minimum_added_chaos_damage",
        &[(added(CHAOS_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_minimum_added_cold_damage",
        &[(added(COLD_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_minimum_added_fire_damage",
        &[(added(FIRE_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_minimum_added_lightning_damage",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_minimum_added_physical_damage",
        &[(added(PHYS, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_speed_+%",
        &[(Quantity::Speed, Op::Increased, 1.0, Flags::ATTACK, NONE)],
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
        "elemental_damage_with_attack_skills_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::ATTACK,
            Flags::ELEMENTAL,
        )],
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
        "maximum_added_lightning_damage_from_skill",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "melee_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, Flags::MELEE, NONE)],
    ),
    (
        "melee_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::MELEE, NONE)],
    ),
    (
        "minimum_added_lightning_damage_from_skill",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "physical_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::PHYSICAL, NONE)],
    ),
    (
        "skill_physical_damage_%_to_convert_to_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::SkillConversion(PHYS, LIGHTNING_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
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
        "support_attack_skills_elemental_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::ATTACK,
            Flags::ELEMENTAL,
        )],
    ),
    (
        "support_controlled_destruction_critical_strike_chance_+%_final",
        &[(Quantity::CritChance, Op::More, 1.0, NONE, NONE)],
    ),
    (
        "support_controlled_destruction_spell_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "support_melee_physical_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::MELEE.union(Flags::PHYSICAL),
            NONE,
        )],
    ),
    (
        "support_melee_physical_damage_attack_speed_+%_final",
        &[(Quantity::Speed, Op::More, 1.0, Flags::ATTACK, NONE)],
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
            | Quantity::Poe1(Poe1Quantity::SkillBaseDamage(..) | Poe1Quantity::SkillConversion(..))
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

/// Stats that reach no slice field (96): `(stat, why)`, sorted by stat ID
/// (P3-8a, with later steps' rows). Each is read as PoB v2.67.2 reads its
/// text, and none feeds a slice field (attributes, life, mana, energy
/// shield, armour, evasion, resistances, block, spell block, spell
/// suppression, unreserved life and mana, and the main skill's hit,
/// speed, hit chance, crit, and hit DPS) on its own: a few only through a
/// config input that stays a reason ([`NO_EFFECT_DEPENDS_ON_INPUTS`]).
///
/// P3-12g adds the pantheon's soul stats (41; `poe1::pantheon`), each as
/// PoB reads its line (`Modules/ModParser.lua`): avoidance, stun,
/// ailments, curses and Blind on you, damage taken by you and its
/// reduction, regeneration and flasks, movement speed, the enemy's
/// regeneration and attack speed. Three of them read a config input:
/// - "Cannot be Blinded" (`Condition:CannotBeBlinded`): only Blind on you
///   (`conditionBlinded`) reads it, as 20% less accuracy and evasion
///   (`CalcPerform.lua` 824 to 835);
/// - "#% reduced Effect of Curses on you" (`CurseEffectOnSelf`): only
///   curses on you read it, from the `playerCursedWith*` inputs
///   (`ConfigOptions.lua` 846 to 885, `CalcPerform.lua` 3114);
/// - "#% reduced Effect of Chill on you": only chill on you
///   (`conditionChilled`, `conditionChilledEffect`), which slows action
///   speed.
///
/// "+#% Chaos Resistance against Damage Over Time" is `ChaosResist` BASE
/// with `ModFlag.Dot` (`ModParser.lua` 1001): PoB's shown resistance
/// sums without a flag, so it never counts there.
///
/// What is left out on purpose, so it stays `Unmapped` until a step maps
/// it: anything that can change a hit or its rate through the enemy
/// (shock, chill, exposure, intimidate, consecrated ground's effect,
/// curses), charges' counts, rage, area, gem levels, minion speed (stats
/// redirect it to the player; minion damage is in, with its redirects in
/// [`NO_EFFECT_DEPENDS_ON`]), and staff or shield
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
    (
        "%_of_your_and_your_minions_damage_cannot_be_reflected",
        "reflect",
    ),
    // A skill's own base radius (PoB `skill("radius")`): it sets the area
    // PoB shows, which no slice field reads (P3-9b). Increased area stays
    // unmapped, as above.
    ("active_skill_base_area_of_effect_radius", "skill-radius"),
    (
        "active_skill_base_secondary_area_of_effect_radius",
        "skill-radius",
    ),
    // How the game describes a skill's areas, its visuals, its targeting:
    // nothing PoB computes from (P3-10, Smite).
    (
        "active_skill_secondary_area_of_effect_description_mode",
        "display",
    ),
    ("add_power_charge_on_kill_%_chance", "charge-gain-chance"),
    (
        "additional_chaos_resistance_against_damage_over_time_%",
        "dot-resistance",
    ),
    ("avoid_ailments_%_from_crit", "ailment-on-self"),
    ("avoid_chained_projectile_%_chance", "avoidance"),
    ("avoid_interruption_while_casting_%", "interruption"),
    ("avoid_maim_%_chance", "ailment-on-self"),
    ("base_avoid_freeze_%", "ailment-on-self"),
    ("base_avoid_projectiles_%_chance", "avoidance"),
    ("base_chance_to_freeze_%", "freeze"),
    // A skill's chance to shock: PoB shocks the enemy only from its config
    // or a guaranteed source (`ShockBase`, `ShockOverride`,
    // `ShockMinimum`; `Modules/CalcPerform.lua`, the non-damaging
    // ailments), never from a chance (P3-10, Smite).
    ("base_chance_to_shock_%_from_skill", "ailment-chance"),
    (
        "base_energy_shield_leech_from_spell_damage_permyriad",
        "leech",
    ),
    ("base_movement_velocity_+%", "movement-speed"),
    // A skill's effect duration (Smite's aura lasts this long after a hit;
    // PoB keeps the aura up).
    ("base_skill_effect_duration", "skill-duration"),
    ("base_smite_number_of_targets", "targeting"),
    ("base_stun_duration_+%", "stun"),
    ("base_stun_recovery_+%", "stun"),
    ("cannot_be_blinded", "blind-on-self"),
    ("cannot_be_poisoned_if_x_poisons_on_you", "ailment-on-self"),
    (
        "cannot_be_shocked_while_at_maximum_power_charges",
        "ailment-on-self",
    ),
    (
        "cannot_be_stunned_if_have_been_stunned_or_blocked_stunning_hit_in_past_2_seconds",
        "stun",
    ),
    (
        "cannot_gain_corrupted_blood_while_you_have_at_least_5_stacks",
        "ailment-on-self",
    ),
    ("cannot_have_life_leeched_from", "leech"),
    (
        "chance_to_create_consecrated_ground_on_melee_kill_%",
        "ground-creation-chance",
    ),
    ("chance_to_fortify_on_melee_hit_+%", "fortification"),
    // "#% chance to Freeze, Shock and Ignite" (PoB `EnemyFreezeChance`,
    // `EnemyShockChance`, `EnemyIgniteChance` BASE): a chance, as
    // `base_chance_to_shock_%_from_skill` above; PoB's ailment chances
    // (`CalcOffence.lua` 4262) set the chance on hit and the ailment's
    // flags, never the enemy's shock or a hit (P3-8c).
    ("chance_to_freeze_shock_ignite_%", "ailment-chance"),
    (
        "chance_to_gain_power_charge_when_block_%",
        "charge-gain-chance",
    ),
    ("chaos_damage_taken_+%", "damage-taken"),
    (
        "chaos_damage_taken_over_time_+%_while_in_caustic_cloud",
        "damage-taken",
    ),
    ("chill_effectiveness_on_self_+%", "ailment-on-self"),
    ("cold_ailment_duration_+%", "ailment-duration"),
    (
        "consecrated_ground_allies_recover_es_as_well_as_life_from_life_regeneration",
        "regeneration",
    ),
    ("console_skill_dont_chase", "display"),
    ("curse_effect_on_self_+%", "curse-on-self"),
    ("debuff_time_passed_+%", "ailment-on-self"),
    ("degen_effect_+%", "damage-taken"),
    (
        "elemental_ailment_duration_on_self_+%_while_holding_shield",
        "ailment-on-self",
    ),
    (
        "elemental_damage_taken_+%_if_been_hit_recently",
        "damage-taken",
    ),
    (
        "elemental_damage_taken_+%_if_not_hit_recently",
        "damage-taken",
    ),
    (
        "enemies_that_hit_you_with_attack_recently_attack_speed_+%",
        "enemy-speed",
    ),
    (
        "enemy_life_regeneration_rate_+%_for_4_seconds_on_hit",
        "enemy-regeneration",
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
    // Culling Strike (PoB `CullPercent` MAX 10, `ModParser.lua` 5066): it
    // reaches only `CullMultiplier`, which scales `CullingDPS` and
    // `CombinedDPS` (`CalcOffence.lua` 3332 to 3339, 6289 to 6292), not
    // the hit, its rate, or `TotalDPS` (P3-12f, the Smite builds' custom
    // line).
    ("kill_enemy_on_hit_if_under_10%_life", "culling"),
    (
        "life_and_energy_shield_recovery_rate_+%_if_stopped_taking_damage_over_time_recently",
        "regeneration",
    ),
    (
        "life_flasks_gain_X_charges_every_3_seconds_if_you_have_not_used_a_life_flask_recently",
        "flask",
    ),
    ("life_recovery_+%_from_flasks_while_on_low_life", "flask"),
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
        "life_regeneration_rate_per_minute_%_while_stationary",
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
    // "Minions deal #% increased Damage" (PoB `MinionModifier` LIST of
    // `Damage` INC): only a minion's own calculation reads it, and no
    // minion is in the slice; it reaches the character only through the
    // stats in `NO_EFFECT_DEPENDS_ON`, which stay reasons (P3-8c).
    ("minion_damage_+%", "minion-damage"),
    (
        "minion_life_regeneration_rate_per_minute_%",
        "minion-defence",
    ),
    ("minion_maximum_life_+%", "minion-defence"),
    ("movement_speed_+%_per_nearby_enemy", "movement-speed"),
    (
        "movement_speed_+%_while_on_burning_ground",
        "movement-speed",
    ),
    ("no_extra_bleed_damage_while_moving", "damage-taken"),
    (
        "pantheon_abberath_ignite_duration_on_self_+%_final",
        "ailment-on-self",
    ),
    (
        "pantheon_shakari_self_poison_duration_+%_final",
        "ailment-on-self",
    ),
    (
        "physical_damage_over_time_taken_+%_while_moving",
        "damage-taken",
    ),
    (
        "physical_damage_reduction_%_if_only_one_enemy_nearby",
        "damage-reduction",
    ),
    (
        "physical_damage_reduction_%_per_hit_you_have_taken_recently",
        "damage-reduction",
    ),
    (
        "physical_damage_reduction_%_per_nearby_enemy",
        "damage-reduction",
    ),
    ("physical_damage_taken_%_as_fire", "damage-taken"),
    ("physical_damage_taken_%_as_lightning", "damage-taken"),
    ("quality_display_active_skill_area_damage_is_gem", "display"),
    (
        "quality_display_active_skill_area_damage_quality_negated_from_gem",
        "display",
    ),
    ("rage_loss_delay_ms", "rage-duration"),
    ("reflect_hexes_chance_%", "curse-on-self"),
    ("regenerate_energy_shield_instead_of_life", "regeneration"),
    ("revive_golems_if_killed_by_enemies_ms", "minion-defence"),
    (
        "self_take_no_extra_damage_from_critical_strikes_if_have_been_crit_recently",
        "damage-taken",
    ),
    ("shocked_effect_on_self_+%", "ailment-on-self"),
    ("smite_lightning_target_range", "targeting"),
    // Melee Physical Damage's more damage with poison and bleeding: no
    // ailment is in the slice.
    (
        "support_melee_physical_damage_poison_and_bleeding_damage_+%_final_from_melee_hits",
        "ailment-damage",
    ),
    ("take_half_area_damage_from_hit_%_chance", "damage-taken"),
    ("unaffected_by_burning_ground", "ground-on-self"),
    ("visual_hit_effect_elemental_is_holy", "display"),
    (
        "while_stationary_gain_additional_physical_damage_reduction_%",
        "damage-reduction",
    ),
    (
        "your_consecrated_ground_effect_lingers_for_ms_after_leaving_the_area",
        "ground-duration",
    ),
];

/// Stats that would make a [`NO_EFFECT`] row reach a slice field: `(stat,
/// row)`, sorted by stat. A row's `NoEffect` holds only while these stay
/// reasons (`unmapped-stat`), so the fields are `Partial` whenever one is
/// in the build; mapping one (even as `NoEffect`) must revisit the row (a
/// test checks each is `Unmapped` through every mapping).
///
/// The minion damage redirects are every stat whose description says
/// increases to minion damage also affect you (`stat_descriptions.txt`,
/// "Minion Damage also affect"; the skill stat
/// `active_skill_additive_minion_damage_modifiers_apply_to_all_damage_at_%_value`):
/// PoB reads them as `MinionDamageAppliesToPlayer` (`ModParser.lua` 3455
/// to 3459, `Data/SkillStatMap.lua` 905), which `CalcOffence.lua` 613
/// turns into the player's increased damage. Four more stats in the stats
/// table name the same redirect (`minion_damage_also_affect_you`,
/// `minion_damage_modifiers_apply_to_you_at_%_value`, its `150%` twin,
/// `minion_damage_modifiers_apply_to_all_damage_at_%_value`); none has a
/// description or a PoB row, so no line resolves to them, and they are
/// `Unmapped` anyway.
pub const NO_EFFECT_DEPENDS_ON: &[(&str, &str)] = &[
    (
        "active_skill_additive_minion_damage_modifiers_apply_to_all_damage_at_%_value",
        "minion_damage_+%",
    ),
    (
        "additive_minion_damage_modifiers_apply_to_you_at_%_value",
        "minion_damage_+%",
    ),
    (
        "additive_minion_damage_modifiers_apply_to_you_at_150%_value",
        "minion_damage_+%",
    ),
    (
        "dominating_blow_and_absolution_additive_minion_damage_modifiers_apply_to_you_at_150%_value",
        "minion_damage_+%",
    ),
    (
        "minion_damage_increases_and_reductions_also_affects_you",
        "minion_damage_+%",
    ),
    (
        "modifiers_to_minion_damage_also_affect_you",
        "minion_damage_+%",
    ),
];

/// Config inputs that would make a [`NO_EFFECT`] row reach a slice field:
/// `(input, row)`, sorted by input (P3-12g). A row's `NoEffect` holds only
/// while these stay config reasons, so a build that sets one is `Partial`;
/// modelling one must revisit the row (a test checks no step handles them).
pub const NO_EFFECT_DEPENDS_ON_INPUTS: &[(&str, &str)] = &[
    ("conditionBlinded", "cannot_be_blinded"),
    ("conditionChilled", "chill_effectiveness_on_self_+%"),
    ("conditionChilledEffect", "chill_effectiveness_on_self_+%"),
    ("playerCursedWithAssassinsMark", "curse_effect_on_self_+%"),
    ("playerCursedWithConductivity", "curse_effect_on_self_+%"),
    ("playerCursedWithDespair", "curse_effect_on_self_+%"),
    (
        "playerCursedWithElementalWeakness",
        "curse_effect_on_self_+%",
    ),
    ("playerCursedWithEnfeeble", "curse_effect_on_self_+%"),
    ("playerCursedWithFlammability", "curse_effect_on_self_+%"),
    ("playerCursedWithFrostbite", "curse_effect_on_self_+%"),
    ("playerCursedWithPoachersMark", "curse_effect_on_self_+%"),
    (
        "playerCursedWithProjectileWeakness",
        "curse_effect_on_self_+%",
    ),
    ("playerCursedWithPunishment", "curse_effect_on_self_+%"),
    ("playerCursedWithTemporalChains", "curse_effect_on_self_+%"),
    ("playerCursedWithVulnerability", "curse_effect_on_self_+%"),
    ("playerCursedWithWarlordsMark", "curse_effect_on_self_+%"),
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

/// What a weapon's local stat changes: `(stat, [(quantity, op)])`, sorted
/// by stat ID. PoB reads the same lines as local mods that `calcLocal`
/// takes for the weapon's own values (`Classes/Item.lua`, the weapon
/// branch): `PhysicalMin` ... BASE ("Adds # to # Physical Damage"),
/// `PhysicalDamage` INC ("#% increased Physical Damage"), `Speed` INC with
/// `ModFlag.Attack` ("#% increased Attack Speed"), `CritChance` INC ("#%
/// increased Critical Strike Chance"); "No Physical Damage" (the game's
/// `local_weapon_no_physical_damage`, 0 on the canonical increased physical
/// damage line; not 0 is a reason, `hit::weapon_data`); and "+# to
/// Accuracy Rating" as the
/// character's `Accuracy` BASE for main-hand attacks (its
/// `MainHandAttack` condition holds for every attack the engine computes,
/// which use the main hand alone).
type WeaponRow = (&'static str, &'static [(Quantity, Op)]);

const fn weapon(value: WeaponValue) -> Quantity {
    Quantity::Poe1(Poe1Quantity::Weapon(value))
}

/// The local stats of weapons (`stat_descriptions.txt`, each with `local`
/// set in the stats table).
pub const WEAPON_ROWS: &[WeaponRow] = &[
    ("local_accuracy_rating", &[(Quantity::Accuracy, Op::Base)]),
    (
        "local_attack_speed_+%",
        &[(weapon(WeaponValue::AttackSpeed), Op::Increased)],
    ),
    (
        "local_critical_strike_chance_+%",
        &[(weapon(WeaponValue::CritChance), Op::Increased)],
    ),
    (
        "local_maximum_added_chaos_damage",
        &[(weapon(WeaponValue::Damage(CHAOS_T, MAX)), Op::Base)],
    ),
    (
        "local_maximum_added_cold_damage",
        &[(weapon(WeaponValue::Damage(COLD_T, MAX)), Op::Base)],
    ),
    (
        "local_maximum_added_fire_damage",
        &[(weapon(WeaponValue::Damage(FIRE_T, MAX)), Op::Base)],
    ),
    (
        "local_maximum_added_lightning_damage",
        &[(weapon(WeaponValue::Damage(LIGHTNING_T, MAX)), Op::Base)],
    ),
    (
        "local_maximum_added_physical_damage",
        &[(weapon(WeaponValue::Damage(PHYS, MAX)), Op::Base)],
    ),
    (
        "local_minimum_added_chaos_damage",
        &[(weapon(WeaponValue::Damage(CHAOS_T, MIN)), Op::Base)],
    ),
    (
        "local_minimum_added_cold_damage",
        &[(weapon(WeaponValue::Damage(COLD_T, MIN)), Op::Base)],
    ),
    (
        "local_minimum_added_fire_damage",
        &[(weapon(WeaponValue::Damage(FIRE_T, MIN)), Op::Base)],
    ),
    (
        "local_minimum_added_lightning_damage",
        &[(weapon(WeaponValue::Damage(LIGHTNING_T, MIN)), Op::Base)],
    ),
    (
        "local_minimum_added_physical_damage",
        &[(weapon(WeaponValue::Damage(PHYS, MIN)), Op::Base)],
    ),
    (
        "local_physical_damage_+%",
        &[
            (weapon(WeaponValue::Damage(PHYS, MIN)), Op::Increased),
            (weapon(WeaponValue::Damage(PHYS, MAX)), Op::Increased),
        ],
    ),
    (
        "local_weapon_no_physical_damage",
        &[(weapon(WeaponValue::NoPhysicalDamage), Op::Base)],
    ),
];

/// The mapping of the main-hand weapon's lines: its local stats act on
/// the weapon ([`WEAPON_ROWS`]); every other stat is the game's
/// ([`Poe1Mapping`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct WeaponMapping;

impl StatMapping for WeaponMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        let Ok(i) = WEAPON_ROWS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str())) else {
            return Poe1Mapping.map(stat);
        };
        StatMap::Effects(
            WEAPON_ROWS[i]
                .1
                .iter()
                .map(|&(quantity, op)| EffectTemplate {
                    quantity,
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
            "minion_attack_speed_+%",
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
    fn a_no_effect_rows_dependencies_should_stay_unmapped() {
        assert!(NO_EFFECT_DEPENDS_ON.windows(2).all(|w| w[0].0 < w[1].0));
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for &(stat, row) in NO_EFFECT_DEPENDS_ON {
            assert!(
                NO_EFFECT.binary_search_by(|(id, _)| (*id).cmp(row)).is_ok(),
                "{stat}: {row} is not a NO_EFFECT row"
            );
            for m in mappings {
                assert_eq!(
                    m.map(&StatId::new(stat)),
                    StatMap::Unmapped,
                    "{stat}: mapping it must revisit {row}"
                );
            }
        }
    }

    #[test]
    fn the_smite_builds_global_stats_should_map_as_pob_reads_them() {
        assert_eq!(
            hit("attack_and_cast_speed_+%"),
            [(Quantity::Speed, Op::Increased, 1.0, Conditions::none())]
        );
        assert_eq!(
            hit("elemental_damage_with_attack_skills_+%"),
            [(
                Quantity::Damage,
                Op::Increased,
                1.0,
                Conditions::none()
                    .with_all(Flags::ATTACK)
                    .with_any(Flags::ELEMENTAL)
            )]
        );
        assert_eq!(
            hit("melee_critical_strike_chance_+%"),
            [(
                Quantity::CritChance,
                Op::Increased,
                1.0,
                Conditions::none().with_all(Flags::MELEE)
            )]
        );
        // A gem carrying them acts on its skill alone, as the game's.
        for stat in [
            "attack_and_cast_speed_+%",
            "elemental_damage_with_attack_skills_+%",
            "melee_critical_strike_chance_+%",
        ] {
            assert_eq!(
                SkillMapping.map(&StatId::new(stat)),
                Poe1Mapping.map(&StatId::new(stat)),
                "{stat}"
            );
        }
        for stat in ["minion_damage_+%", "chance_to_freeze_shock_ignite_%"] {
            assert_eq!(
                Poe1Mapping.map(&StatId::new(stat)),
                StatMap::NoEffect,
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

    #[test]
    fn attack_stats_should_map_as_pob_reads_them() {
        let none = Conditions::none;
        let all = |f| Conditions::none().with_all(f);
        assert_eq!(
            hit("accuracy_rating"),
            [(Quantity::Accuracy, Op::Base, 1.0, none())]
        );
        assert_eq!(
            hit("accuracy_rating_+%"),
            [(Quantity::Accuracy, Op::Increased, 1.0, none())]
        );
        assert_eq!(
            hit("attack_minimum_added_physical_damage"),
            [(added(PHYS, MIN), Op::Base, 1.0, all(Flags::ATTACK))]
        );
        assert_eq!(
            hit("attack_speed_+%"),
            [(Quantity::Speed, Op::Increased, 1.0, all(Flags::ATTACK))]
        );
        assert_eq!(
            hit("melee_damage_+%"),
            [(Quantity::Damage, Op::Increased, 1.0, all(Flags::MELEE))]
        );
        assert_eq!(
            hit("physical_damage_+%"),
            [(Quantity::Damage, Op::Increased, 1.0, all(Flags::PHYSICAL))]
        );
        assert_eq!(
            hit("active_skill_area_damage_+%_final"),
            [(Quantity::Damage, Op::More, 1.0, all(Flags::AREA))]
        );
        assert_eq!(
            hit("support_attack_skills_elemental_damage_+%_final"),
            [(
                Quantity::Damage,
                Op::More,
                1.0,
                all(Flags::ATTACK).with_any(Flags::ELEMENTAL)
            )]
        );
        assert_eq!(
            hit("support_melee_physical_damage_+%_final"),
            [(
                Quantity::Damage,
                Op::More,
                1.0,
                all(Flags::MELEE | Flags::PHYSICAL)
            )]
        );
        assert_eq!(
            hit("support_melee_physical_damage_attack_speed_+%_final"),
            [(Quantity::Speed, Op::More, 1.0, all(Flags::ATTACK))]
        );
        // Smite's aura adds lightning damage to every hit.
        assert_eq!(
            hit("minimum_added_lightning_damage_from_skill"),
            [(added(LIGHTNING_T, MIN), Op::Base, 1.0, none())]
        );
        assert_eq!(
            hit("skill_physical_damage_%_to_convert_to_lightning"),
            [(
                Quantity::Poe1(Poe1Quantity::SkillConversion(PHYS, LIGHTNING_T)),
                Op::Base,
                1.0,
                none()
            )]
        );
        // A gem's conversion acts on its skill.
        assert!(matches!(
            SkillMapping.map(&StatId::new(
                "skill_physical_damage_%_to_convert_to_lightning"
            )),
            StatMap::Effects(_)
        ));
        // Left unmapped: area of effect (it can change a hit's count).
        assert_eq!(
            Poe1Mapping.map(&StatId::new(
                "support_concentrated_effect_skill_area_of_effect_+%_final"
            )),
            StatMap::Unmapped
        );
    }

    #[test]
    fn weapon_local_stats_should_act_on_the_weapon() {
        assert!(WEAPON_ROWS.windows(2).all(|w| w[0].0 < w[1].0));
        let local = |stat: &str| match WeaponMapping.map(&StatId::new(stat)) {
            StatMap::Effects(e) => e
                .into_iter()
                .map(|t| (t.quantity, t.op, t.scale))
                .collect::<Vec<_>>(),
            other => panic!("{stat}: {other:?}"),
        };
        let weapon = |v| Quantity::Poe1(Poe1Quantity::Weapon(v));
        assert_eq!(
            local("local_physical_damage_+%"),
            [
                (weapon(WeaponValue::Damage(PHYS, MIN)), Op::Increased, 1.0),
                (weapon(WeaponValue::Damage(PHYS, MAX)), Op::Increased, 1.0)
            ]
        );
        assert_eq!(
            local("local_maximum_added_lightning_damage"),
            [(weapon(WeaponValue::Damage(LIGHTNING_T, MAX)), Op::Base, 1.0)]
        );
        assert_eq!(
            local("local_attack_speed_+%"),
            [(weapon(WeaponValue::AttackSpeed), Op::Increased, 1.0)]
        );
        assert_eq!(
            local("local_critical_strike_chance_+%"),
            [(weapon(WeaponValue::CritChance), Op::Increased, 1.0)]
        );
        assert_eq!(
            local("local_weapon_no_physical_damage"),
            [(weapon(WeaponValue::NoPhysicalDamage), Op::Base, 1.0)]
        );
        assert_eq!(
            local("local_accuracy_rating"),
            [(Quantity::Accuracy, Op::Base, 1.0)]
        );
        // Other lines are the game's.
        assert_eq!(
            WeaponMapping.map(&StatId::new("maximum_life_+%")),
            Poe1Mapping.map(&StatId::new("maximum_life_+%"))
        );
        // The global mapping does not know the weapon's local stats.
        assert_eq!(
            Poe1Mapping.map(&StatId::new("local_attack_speed_+%")),
            StatMap::Unmapped
        );
    }
}

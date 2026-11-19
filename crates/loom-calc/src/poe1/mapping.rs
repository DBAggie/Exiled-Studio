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

use loom_domain::{StatId, TypedResolution};

use crate::condition::{ConditionKey, Conditions, Flags};
use crate::ingest::{EffectTemplate, StatMap, StatMapping, TypedMap};
use crate::poe1::ailment::Ailment;
use crate::poe1::{
    ArmourSlot, Defence, LeechSource, PieceValue, Poe1Quantity, Pool, TakenOf, TakenWhen,
    WeaponValue, conditions, states,
};
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::source::Source;
use crate::store::Effect;

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
// Skill keywords (P3-8h): PoB's `KeywordFlag.<type>`, which the skill's
// types give ([`super::skill::MainSkill::keyword_flags`]).
const SK_FIRE: Flags = Flags::SKILL_FIRE;
const SK_COLD: Flags = Flags::SKILL_COLD;
const SK_LIGHTNING: Flags = Flags::SKILL_LIGHTNING;
const SK_CHAOS: Flags = Flags::SKILL_CHAOS;
/// The damaging ailments (for an `any` condition; P6-2b).
const DAMAGING_AILMENTS: Flags = Flags::IGNITE.union(Flags::BLEED).union(Flags::POISON);
const fn cast_with(keyword: Flags) -> Flags {
    Flags::CAST.union(keyword)
}
// A damage type's damage with attack or spell skills (P3-8h).
// "with Attack Skills" and "with Spell Skills" are PoB's keyword flags
// (`KeywordFlag.Attack`, `KeywordFlag.Spell`, `ModParser.lua` 971, 978),
// which an attack's or a spell's ailments and damage over time keep (P6-2b).
const ATTACK_PHYSICAL: Flags = Flags::KEYWORD_ATTACK.union(Flags::PHYSICAL);
const ATTACK_FIRE: Flags = Flags::KEYWORD_ATTACK.union(Flags::FIRE);
const ATTACK_COLD: Flags = Flags::KEYWORD_ATTACK.union(Flags::COLD);
const ATTACK_LIGHTNING: Flags = Flags::KEYWORD_ATTACK.union(Flags::LIGHTNING);
const ATTACK_CHAOS: Flags = Flags::KEYWORD_ATTACK.union(Flags::CHAOS);
const SPELL_PHYSICAL: Flags = Flags::KEYWORD_SPELL.union(Flags::PHYSICAL);
const SPELL_FIRE: Flags = Flags::KEYWORD_SPELL.union(Flags::FIRE);
const SPELL_COLD: Flags = Flags::KEYWORD_SPELL.union(Flags::COLD);
const SPELL_LIGHTNING: Flags = Flags::KEYWORD_SPELL.union(Flags::LIGHTNING);
const SPELL_CHAOS: Flags = Flags::KEYWORD_SPELL.union(Flags::CHAOS);

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
///   effect; since P3-8l the non-curse aura effect scales the stat's
///   value before it reaches this mapping,
///   `poe1::skill_modifiers::MAIN_SKILL_AURA_STATS`, and the other effect
///   stats are unmapped);
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
/// P3-8h adds the stats that name a skill keyword, each read from its text
/// by `ModParser.lua` (no stat map row):
/// - `cast_speed_for_<chaos|cold|fire|lightning>_skills_+%`: "#% increased
///   Cast Speed with Cold Skills" (`Speed` INC, `ModFlag.Cast`,
///   `KeywordFlag.Cold`); `cast_speed_for_elemental_skills_+%` needs one of
///   the fire, cold, and lightning keywords;
/// - `<chaos|cold|fire|lightning>_critical_strike_chance_+%`,
///   `..._multiplier_+`: "#% increased Critical Strike Chance with Cold
///   Skills", "+#% to Critical Strike Multiplier with Cold Skills"
///   (`CritChance` INC, `CritMultiplier` BASE, `KeywordFlag.Cold`);
/// - `<type>_damage_with_attack_skills_+%`, `..._spell_skills_+%`: "#%
///   increased Cold Damage with Attack Skills" (`ColdDamage` INC,
///   `KeywordFlag.Attack` or `KeywordFlag.Spell`).
///
/// A keyword is the skill's (`Flags::SKILL_COLD`, from its types), not the
/// damage type of a part of its hit (`Flags::COLD`). PoB tells
/// `ModFlag.Attack` from `KeywordFlag.Attack`; both are the `ATTACK` flag
/// here, which every attack hit has and nothing else does; `SPELL` stands
/// for both spell flags likewise (PoB leaves `KeywordFlag.Spell` off only
/// for the skills its own data marks `cast`: Bear Trap, Vaal Detonate
/// Dead, the Heralds of Agony, Ice, and Thunder, one minion skill; none is
/// a spell PoB's data vets, `hit::pob_vetted`, so any of them is `Partial`).
pub const HIT_ROWS: &[HitRow] = &[
    (
        "accuracy_rating",
        &[(Quantity::Accuracy, Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "accuracy_rating_+%",
        &[(Quantity::Accuracy, Op::Increased, 1.0, NONE, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1020: `Damage` MORE with `KeywordFlag.Ailment`
    // (P6-2b).
    (
        "active_skill_ailment_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::AILMENT, NONE)],
    ),
    (
        "active_skill_area_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::AREA, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1729: `Speed` MORE with `ModFlag.Cast`
    // (P6-7c3, the Flame Golem's Immolate).
    (
        "active_skill_cast_speed_+%_final",
        &[(Quantity::Speed, Op::More, 1.0, Flags::CAST, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1184: `Damage` MORE with `KeywordFlag.Ignite`
    // (P6-2b).
    (
        "active_skill_ignite_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::IGNITE, NONE)],
    ),
    (
        "additional_base_critical_strike_chance",
        &[(Quantity::CritChance, Op::Base, 100.0, NONE, NONE)],
    ),
    // P6-10b part 10: "+#% Chaos Resistance against Damage Over Time"
    // (`ChaosResist` BASE with `ModFlag.Dot`): the over-time resistance alone.
    (
        "additional_chaos_resistance_against_damage_over_time_%",
        &[(
            Quantity::Resistance(DamageType::Chaos),
            Op::Base,
            1.0,
            Flags::DOT,
            NONE,
        )],
    ),
    // P6-10b part 10: "+#% Cold Resistance against Damage Over Time"
    // (`ColdResist` BASE with `ModFlag.Dot`): the over-time resistance alone.
    (
        "additional_cold_resistance_against_damage_over_time_%",
        &[(
            Quantity::Resistance(DamageType::Cold),
            Op::Base,
            1.0,
            Flags::DOT,
            NONE,
        )],
    ),
    // P6-10b part 10: "+#% Fire Resistance against Damage Over Time"
    // (`FireResist` BASE with `ModFlag.Dot`): the over-time resistance alone.
    (
        "additional_fire_resistance_against_damage_over_time_%",
        &[(
            Quantity::Resistance(DamageType::Fire),
            Op::Base,
            1.0,
            Flags::DOT,
            NONE,
        )],
    ),
    // P6-10b part 10: "+#% Lightning Resistance against Damage Over Time"
    // (`LightningResist` BASE with `ModFlag.Dot`): the over-time resistance alone.
    (
        "additional_lightning_resistance_against_damage_over_time_%",
        &[(
            Quantity::Resistance(DamageType::Lightning),
            Op::Base,
            1.0,
            Flags::DOT,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1033: `EnemyIgniteChance` BASE 100 (P6-2b).
    (
        "always_ignite",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Ignite)),
            Op::Base,
            0.01,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 779: `Damage` INC with `ModFlag.Area` (P3-8e).
    (
        "area_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::AREA, NONE)],
    ),
    // P6-11n: the Assassin's "#% more Critical Strike Chance with
    // Spells" (`CritChance` MORE, `ModFlag.Spell`).
    (
        "assassin_ascendancy_spell_critical_strike_chance_+%_final",
        &[(Quantity::CritChance, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "attack_and_cast_speed_+%",
        &[(Quantity::Speed, Op::Increased, 1.0, NONE, NONE)],
    ),
    // P6-11n: "#% increased Critical Strike Chance with Attacks" (`CritChance`
    // INC, `ModFlag.Attack`).
    (
        "attack_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    // P6-11n: "+#% to Critical Strike Multiplier with Attacks" (`CritMultiplier`
    // BASE, `ModFlag.Attack`).
    (
        "attack_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-10b: "#% increased Mana Cost of Attacks" (`ManaCost` INC,
    // `SkillType` Attack).
    (
        "attack_mana_cost_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaCost),
            Op::Increased,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
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
    // P6-11b: "# to # Added ... Damage with Sword Attacks": BASE, `ModFlag.Sword` and `Hit`.
    (
        "attack_maximum_added_fire_damage_with_swords",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "# to # Added ... Damage with Wand Attacks": BASE, `ModFlag.Wand` and `Hit`.
    (
        "attack_maximum_added_fire_damage_with_wand",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_maximum_added_lightning_damage",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11b: "# to # Added ... Damage with Wand Attacks": BASE, `ModFlag.Wand` and `Hit`.
    (
        "attack_maximum_added_lightning_damage_with_wand",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
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
    // P6-11b: "# to # Added ... Damage with Sword Attacks": BASE, `ModFlag.Sword` and `Hit`.
    (
        "attack_minimum_added_fire_damage_with_swords",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "# to # Added ... Damage with Wand Attacks": BASE, `ModFlag.Wand` and `Hit`.
    (
        "attack_minimum_added_fire_damage_with_wand",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_minimum_added_lightning_damage",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11b: "# to # Added ... Damage with Wand Attacks": BASE, `ModFlag.Wand` and `Hit`.
    (
        "attack_minimum_added_lightning_damage_with_wand",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_minimum_added_physical_damage",
        &[(added(PHYS, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "attack_speed_+%",
        &[(Quantity::Speed, Op::Increased, 1.0, Flags::ATTACK, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1708: `ImpaleChance` BASE with
    // `KeywordFlag.Attack` (P6-2d).
    (
        "attacks_impale_on_hit_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleChance),
            Op::Base,
            1.0,
            Flags::KEYWORD_ATTACK,
            NONE,
        )],
    ),
    // P6-11b: "Axe Attacks deal #% increased Damage with Ailments": `Damage` INC, `ModFlag.Axe` and `Ailment`.
    (
        "axe_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::AXE.union(Flags::AILMENT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Attack Speed with Axes": `Speed` INC, `ModFlag.Axe`, `Hit`, `Attack`.
    (
        "axe_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::AXE.union(Flags::HIT).union(Flags::ATTACK),
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1017: `Damage` INC with `KeywordFlag.Ailment`
    // (P6-2b).
    (
        "base_ailment_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::AILMENT, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1151: `EnemyAilmentDuration` INC (P6-2b).
    (
        "base_all_ailment_duration_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentDuration),
            Op::Increased,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P6-10b part 8: "Attack Skills Cost Life instead of #% of Mana
    // Cost" (`HybridManaAndLifeCost_Life` BASE with attacks).
    (
        "base_attack_skill_cost_life_instead_of_mana_%",
        &[(
            Quantity::Poe1(Poe1Quantity::HybridLifeCost),
            Op::Base,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 709: `EnemyBleedDuration` INC (P6-2c).
    (
        "base_bleed_duration_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentDuration),
            Op::Increased,
            1.0,
            Flags::BLEED,
            NONE,
        )],
    ),
    (
        "base_cast_speed_+%",
        &[(Quantity::Speed, Op::Increased, 1.0, Flags::CAST, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1030: `EnemyIgniteChance` BASE (P6-2b).
    (
        "base_chance_to_ignite_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Ignite)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1023: `PoisonChance` BASE (P6-2c).
    (
        "base_chance_to_poison_on_hit_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Poison)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    (
        "base_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, NONE, NONE)],
    ),
    // P6-7c2: `SkillStatMap.lua` 1232 to 1239: `DealNoFire`, `DealNoCold`,
    // `DealNoLightning` FLAG; `DealNoChaos` FLAG (Brutality's).
    (
        "base_deal_no_chaos_damage",
        &[(
            Quantity::Poe1(Poe1Quantity::DealNo(DamageType::Chaos)),
            Op::Flag,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1148: `EnemyElementalAilmentDuration` INC
    // (P6-2b): the elemental ailments, of which ignite is the damaging one.
    (
        "base_elemental_status_ailment_duration_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentDuration),
            Op::Increased,
            1.0,
            NONE,
            Flags::IGNITE,
        )],
    ),
    // P6-10b part 9: "#% of Spell Damage Leeched as Energy Shield" (`DamageEnergyShieldLeech` BASE).
    (
        "base_energy_shield_leech_from_spell_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::EnergyShield,
                LeechSource::Any,
            )),
            Op::Base,
            100.0,
            Flags::SPELL,
            NONE,
        )],
    ),
    // P6-10b part 9: "#% of Attack Damage Leeched as Life" (`DamageLifeLeech` BASE).
    (
        "base_life_leech_from_attack_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(Pool::Life, LeechSource::Any)),
            Op::Base,
            100.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    // P6-10b part 9: "#% of Spell Damage Leeched as Life" (`DamageLifeLeech` BASE).
    (
        "base_life_leech_from_spell_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(Pool::Life, LeechSource::Any)),
            Op::Base,
            100.0,
            Flags::SPELL,
            NONE,
        )],
    ),
    // P6-10b part 9: "#% of Attack Damage Leeched as Mana" (`DamageManaLeech` BASE).
    (
        "base_mana_leech_from_attack_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(Pool::Mana, LeechSource::Any)),
            Op::Base,
            100.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    // P6-10b part 9: "#% of Spell Damage Leeched as Mana" (`DamageManaLeech` BASE).
    (
        "base_mana_leech_from_spell_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(Pool::Mana, LeechSource::Any)),
            Op::Base,
            100.0,
            Flags::SPELL,
            NONE,
        )],
    ),
    // P6-11n: "#% of Physical Damage Converted to Chaos Damage"
    // (`PhysicalDamageConvertToChaos` BASE).
    (
        "base_physical_damage_%_to_convert_to_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::Conversion(PHYS, CHAOS_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P3-8n: "#% of Physical Damage Converted to Fire Damage"
    // (`PhysicalDamageConvertToFire` BASE), conversion that is not the
    // skill's own.
    (
        "base_physical_damage_%_to_convert_to_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::Conversion(PHYS, FIRE_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P6-11n: "#% of Physical Damage Converted to Lightning Damage"
    // (`PhysicalDamageConvertToLightning` BASE), as the fire one.
    (
        "base_physical_damage_%_to_convert_to_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::Conversion(PHYS, LIGHTNING_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1163: `Damage` INC with `KeywordFlag.Poison`
    // (P6-2c).
    (
        "base_poison_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::POISON, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1123: `EnemyPoisonDuration` INC (P6-2c).
    (
        "base_poison_duration_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentDuration),
            Op::Increased,
            1.0,
            Flags::POISON,
            NONE,
        )],
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
    // "+#% to Critical Strike Multiplier for Spell Damage": PoB's
    // `CritMultiplier` BASE with "for spell damage" (`ModFlag.Spell`,
    // `ModParser.lua` 748, 968; P3-8e).
    (
        "base_spell_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, Flags::SPELL, NONE)],
    ),
    // P6-11n: the Berserker's "#% more Damage" (`Damage` MORE).
    (
        "berserker_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, NONE, NONE)],
    ),
    // PoB `SkillStatMap.lua` 995: `BleedChance` BASE with `ModFlag.Attack`
    // (P6-2c).
    (
        "bleed_on_hit_with_attacks_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Bleed)),
            Op::Base,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1154: `Damage` INC with `KeywordFlag.Bleed`
    // (P6-2c).
    (
        "bleeding_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::BLEED, NONE)],
    ),
    // P6-11b: "#% increased Accuracy Rating with Bows": `Accuracy` INC, `ModFlag.Bow` and `Hit`.
    (
        "bow_accuracy_rating_+%",
        &[(
            Quantity::Accuracy,
            Op::Increased,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Attack Speed with Bows": `Speed` INC, `ModFlag.Bow`, `Hit`, `Attack`.
    (
        "bow_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::BOW.union(Flags::HIT).union(Flags::ATTACK),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Critical Strike Chance with Bows": `CritChance` INC, `ModFlag.Bow` and `Hit`.
    (
        "bow_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "+#% to Critical Strike Multiplier with Bows": `CritMultiplier` BASE, `ModFlag.Bow` and `Hit`.
    (
        "bow_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Damage with Bows": `Damage` INC, `ModFlag.Bow` and `Hit`.
    (
        "bow_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 755: `FireDamage` INC with
    // `KeywordFlag.FireDot` (P6-1).
    (
        "burn_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::FIRE.union(Flags::FIRE_DOT),
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1223: the `CannotBleed` flag (P6-2c).
    (
        "cannot_cause_bleeding",
        &[(
            Quantity::Poe1(Poe1Quantity::CannotInflict(Ailment::Bleed)),
            Op::Flag,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `Data/Skills/sup_str.lua` 1158 (Cast when Damage Taken's own
    // stat map): `Damage` MORE (P6-8a).
    (
        "cast_on_damage_taken_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, NONE, NONE)],
    ),
    // PoB `Data/Skills/sup_dex.lua` 841 (Cast on Ward Break): `Damage`
    // MORE with `ModFlag.Spell` (P6-8a part 2).
    (
        "cast_on_ward_break_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    // P3-8h: "#% increased Cast Speed with Chaos Skills": `Speed` INC,
    // `ModFlag.Cast`, `KeywordFlag.Chaos` ("cast speed", "with chaos
    // skills"; `ModParser.lua` 754, 1052); likewise below.
    (
        "cast_speed_for_chaos_skills_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            cast_with(SK_CHAOS),
            NONE,
        )],
    ),
    (
        "cast_speed_for_cold_skills_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            cast_with(SK_COLD),
            NONE,
        )],
    ),
    // "with elemental skills": any of the three keywords (1051).
    (
        "cast_speed_for_elemental_skills_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::CAST,
            Flags::SKILL_ELEMENTAL,
        )],
    ),
    (
        "cast_speed_for_fire_skills_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            cast_with(SK_FIRE),
            NONE,
        )],
    ),
    (
        "cast_speed_for_lightning_skills_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            cast_with(SK_LIGHTNING),
            NONE,
        )],
    ),
    // P3-8n: "#% chance to deal Double Damage" (`DoubleDamageChance` BASE,
    // `ModParser.lua` 677).
    (
        "chance_to_deal_double_damage_%",
        &[(
            Quantity::Poe1(Poe1Quantity::DoubleDamageChance),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // "#% chance to Freeze, Shock and Ignite" (PoB `EnemyFreezeChance`,
    // `EnemyShockChance`, `EnemyIgniteChance` BASE): the ignite's chance on
    // hit (P6-2b). The freeze and shock chances set only PoB's
    // non-damaging ailment outputs, which no field reads (never the
    // enemy's shock in a hit: the config and guaranteed shocks set it,
    // P3-8k); they join when those are computed.
    (
        "chance_to_freeze_shock_ignite_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Ignite)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P3-8h: "#% increased Critical Strike Chance with Chaos Skills",
    // "+#% to Critical Strike Multiplier with Chaos Skills": `CritChance`
    // INC, `CritMultiplier` BASE with `KeywordFlag.Chaos` (745, 748); the
    // other keywords likewise.
    (
        "chaos_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, SK_CHAOS, NONE)],
    ),
    (
        "chaos_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, SK_CHAOS, NONE)],
    ),
    // "#% increased Chaos Damage": `ChaosDamage` INC, as the other types'
    // (PoB `SkillStatMap.lua`; P6-2c review: poison's physical part reads
    // it too).
    (
        "chaos_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::CHAOS, NONE)],
    ),
    // P3-8h: "#% increased Chaos Damage with Attack Skills": `ChaosDamage`
    // INC with `KeywordFlag.Attack` (701, 978); "with Spell Skills" with
    // `KeywordFlag.Spell` (971); the other damage types likewise.
    (
        "chaos_damage_with_attack_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, ATTACK_CHAOS, NONE)],
    ),
    (
        "chaos_damage_with_spell_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, SPELL_CHAOS, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1178: `ChaosDotMultiplier` BASE (P6-1).
    (
        "chaos_dot_multiplier_+",
        &[(Quantity::DotMultiplier, Op::Base, 1.0, Flags::CHAOS, NONE)],
    ),
    (
        "cold_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, SK_COLD, NONE)],
    ),
    (
        "cold_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, SK_COLD, NONE)],
    ),
    (
        "cold_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::COLD, NONE)],
    ),
    (
        "cold_damage_with_attack_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, ATTACK_COLD, NONE)],
    ),
    (
        "cold_damage_with_spell_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, SPELL_COLD, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1181: `ColdDotMultiplier` BASE (P6-1).
    (
        "cold_dot_multiplier_+",
        &[(Quantity::DotMultiplier, Op::Base, 1.0, Flags::COLD, NONE)],
    ),
    (
        "critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, NONE, NONE)],
    ),
    (
        "damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, NONE, NONE)],
    ),
    // P6-11b: "#% increased Damage with Bow Skills": `Damage` INC, `KeywordFlag.Bow`.
    (
        "damage_+%_with_bow_skills",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::KEYWORD_BOW,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 752: `Damage` INC with `ModFlag.Dot` (P6-1).
    (
        "damage_over_time_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::DOT, NONE)],
    ),
    // P6-11b: "#% increased Damage Over Time with Bow Skills": `Damage` INC, `ModFlag.Dot`, `KeywordFlag.Bow`.
    (
        "damage_over_time_+%_with_bow_skills",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::DOT.union(Flags::KEYWORD_BOW),
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1187: `BleedFaster`, `PoisonFaster`, and
    // `IgniteBurnFaster` INC (P6-2b).
    (
        "damaging_ailments_deal_damage_+%_faster",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentFaster),
            Op::Increased,
            1.0,
            NONE,
            DAMAGING_AILMENTS,
        )],
    ),
    (
        "deal_no_elemental_damage",
        &[
            (
                Quantity::Poe1(Poe1Quantity::DealNo(DamageType::Fire)),
                Op::Flag,
                1.0,
                NONE,
                NONE,
            ),
            (
                Quantity::Poe1(Poe1Quantity::DealNo(DamageType::Cold)),
                Op::Flag,
                1.0,
                NONE,
                NONE,
            ),
            (
                Quantity::Poe1(Poe1Quantity::DealNo(DamageType::Lightning)),
                Op::Flag,
                1.0,
                NONE,
                NONE,
            ),
        ],
    ),
    // PoB `SkillStatMap.lua` 1172: `DotMultiplier` BASE (P6-1).
    (
        "dot_multiplier_+",
        &[(Quantity::DotMultiplier, Op::Base, 1.0, NONE, NONE)],
    ),
    // P6-11n: "+#% to Critical Strike Multiplier with Elemental Skills"
    // (`CritMultiplier` BASE, the fire, cold, and lightning keywords).
    (
        "elemental_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            NONE,
            Flags::SKILL_ELEMENTAL,
        )],
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
            Flags::KEYWORD_ATTACK,
            Flags::ELEMENTAL,
        )],
    ),
    // PoB `SkillStatMap.lua` 1011: `BleedFaster` INC (P6-2c).
    (
        "faster_bleed_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentFaster),
            Op::Increased,
            1.0,
            Flags::BLEED,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 758: `IgniteBurnFaster` INC (P6-2b).
    (
        "faster_burn_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentFaster),
            Op::Increased,
            1.0,
            Flags::IGNITE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 761: `PoisonFaster` INC (P6-2c).
    (
        "faster_poison_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentFaster),
            Op::Increased,
            1.0,
            Flags::POISON,
            NONE,
        )],
    ),
    (
        "fire_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, SK_FIRE, NONE)],
    ),
    (
        "fire_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, SK_FIRE, NONE)],
    ),
    (
        "fire_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::FIRE, NONE)],
    ),
    (
        "fire_damage_with_attack_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, ATTACK_FIRE, NONE)],
    ),
    (
        "fire_damage_with_spell_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, SPELL_FIRE, NONE)],
    ),
    // PoB `SkillStatMap.lua` 1175: `FireDotMultiplier` BASE (P6-1).
    (
        "fire_dot_multiplier_+",
        &[(Quantity::DotMultiplier, Op::Base, 1.0, Flags::FIRE, NONE)],
    ),
    // PoB `SkillStatMap.lua` 998: `BleedChance` BASE 100 (P6-2c).
    (
        "global_bleed_on_hit",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Bleed)),
            Op::Base,
            0.01,
            NONE,
            NONE,
        )],
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
    // PoB `SkillStatMap.lua` 1026: `PoisonChance` BASE 100 (P6-2c).
    (
        "global_poison_on_hit",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Poison)),
            Op::Base,
            0.01,
            NONE,
            NONE,
        )],
    ),
    // P6-11j: "Hits have #% chance to ignore Enemy Physical Damage
    // Reduction" (`ChanceToIgnoreEnemyPhysicalDamageReduction` BASE).
    (
        "hits_ignore_enemy_monster_physical_damage_reduction_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::IgnorePdrChance),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1129: `EnemyIgniteDuration` INC (P6-2b).
    (
        "ignite_duration_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentDuration),
            Op::Increased,
            1.0,
            Flags::IGNITE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1717: `ImpaleEffect` INC (P6-2d).
    (
        "impale_debuff_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleEffect),
            Op::Increased,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1711: `ImpaleChance` BASE (P6-2d).
    (
        "impale_on_hit_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleChance),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB: `ImpaleStacksMax` BASE (the character template's stat, also on
    // items; P6-2d).
    (
        "impaled_debuff_number_of_reflected_hits",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleStacksMax),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P6-10b part 9: "#% of Physical Attack Damage Leeched as Life" (`PhysicalDamageLifeLeech` BASE).
    (
        "life_leech_from_physical_attack_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Physical),
            )),
            Op::Base,
            100.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    (
        "lightning_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, SK_LIGHTNING, NONE)],
    ),
    (
        "lightning_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, SK_LIGHTNING, NONE)],
    ),
    (
        "lightning_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::LIGHTNING, NONE)],
    ),
    (
        "lightning_damage_with_attack_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, ATTACK_LIGHTNING, NONE)],
    ),
    (
        "lightning_damage_with_spell_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, SPELL_LIGHTNING, NONE)],
    ),
    // P6-11b: "#% increased Attack Speed with Maces or Sceptres": `Speed` INC, `ModFlag.Mace`, `Hit`, `Attack`.
    (
        "mace_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::MACE.union(Flags::HIT).union(Flags::ATTACK),
            NONE,
        )],
    ),
    // P6-10b part 9: "#% of Physical Attack Damage Leeched as Mana" (`PhysicalDamageManaLeech` BASE).
    (
        "mana_leech_from_physical_attack_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Mana,
                LeechSource::Type(DamageType::Physical),
            )),
            Op::Base,
            100.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    (
        "maximum_added_lightning_damage_from_skill",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    // P6-11n: "#% increased Melee Attack Speed" (`Speed` INC, `ModFlag.Attack`
    // and `ModFlag.Melee`).
    (
        "melee_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::ATTACK.union(Flags::MELEE),
            NONE,
        )],
    ),
    (
        "melee_critical_strike_chance_+%",
        &[(Quantity::CritChance, Op::Increased, 1.0, Flags::MELEE, NONE)],
    ),
    (
        "melee_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::MELEE, NONE)],
    ),
    // PoB `SkillStatMap.lua` 776: `PhysicalDamage` INC with
    // `ModFlag.Melee` (Melee Physical Damage's quality, P3-12i).
    (
        "melee_physical_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::MELEE.union(Flags::PHYSICAL),
            NONE,
        )],
    ),
    // P6-11b: "Attacks with Melee Weapons deal #% increased Damage with Ailments": `Damage` INC, `ModFlag.WeaponMelee` and `Ailment`.
    (
        "melee_weapon_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WEAPON_MELEE.union(Flags::AILMENT),
            NONE,
        )],
    ),
    // P6-11b: "+#% to Melee Critical Strike Multiplier": `CritMultiplier` BASE, `ModFlag.Melee`.
    (
        "melee_weapon_critical_strike_multiplier_+",
        &[(Quantity::CritMultiplier, Op::Base, 1.0, Flags::MELEE, NONE)],
    ),
    (
        "minimum_added_lightning_damage_from_skill",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    // P6-11n: the Occultist's "#% more Chaos Damage" (`ChaosDamage` MORE).
    (
        "occultist_chaos_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::CHAOS, NONE)],
    ),
    // P6-11n: the Occultist's "#% more Cold Damage" (`ColdDamage` MORE).
    (
        "occultist_cold_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::COLD, NONE)],
    ),
    // P6-11b: "+#% to Critical Strike Multiplier with One Handed Melee Weapons": `CritMultiplier` BASE, `ModFlag.Weapon1H`, `WeaponMelee`, `Hit`.
    (
        "one_handed_melee_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::WEAPON_1H
                .union(Flags::WEAPON_MELEE)
                .union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11n: "#% increased Physical Attack Damage" (`PhysicalDamage` INC,
    // `ModFlag.Attack`).
    (
        "physical_attack_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::ATTACK.union(Flags::PHYSICAL),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Physical Damage with Axes": `PhysicalDamage` INC, `ModFlag.Axe` and `Hit`.
    (
        "physical_axe_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::PHYSICAL.union(Flags::AXE).union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "physical_damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, Flags::PHYSICAL, NONE)],
    ),
    (
        "physical_damage_with_attack_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, ATTACK_PHYSICAL, NONE)],
    ),
    (
        "physical_damage_with_spell_skills_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, SPELL_PHYSICAL, NONE)],
    ),
    // PoB `ModParser.lua` 739 ("physical damage over time multiplier"):
    // `PhysicalDotMultiplier` BASE (P6-1).
    (
        "physical_dot_multiplier_+",
        &[(
            Quantity::DotMultiplier,
            Op::Base,
            1.0,
            Flags::PHYSICAL,
            NONE,
        )],
    ),
    // P3-8n, P6-11b review: "#% increased Physical Damage with Staves":
    // `PhysicalDamage` INC with `ModFlag.Staff` and `ModFlag.Hit`
    // (`ModParser.lua` 939; the hand's weapon flag, P3-8n read it from the
    // `UsingStaff` state).
    (
        "physical_staff_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::PHYSICAL.union(Flags::STAFF).union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Physical Damage with Swords": `PhysicalDamage` INC, `ModFlag.Sword` and `Hit`.
    (
        "physical_sword_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::PHYSICAL.union(Flags::SWORD).union(Flags::HIT),
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 1169: `DotMultiplier` BASE with
    // `KeywordFlag.Poison` (P6-2c).
    (
        "poison_dot_multiplier_+",
        &[(Quantity::DotMultiplier, Op::Base, 1.0, Flags::POISON, NONE)],
    ),
    // PoB `ModParser.lua` 719 ("projectile damage"): `Damage` INC with
    // `ModFlag.Projectile` (P6-1).
    (
        "projectile_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::PROJECTILE,
            NONE,
        )],
    ),
    // P6-11b review: "Damage with Weapons Penetrates #% Elemental
    // Resistances": `ElementalPenetration` BASE with `ModFlag.Weapon`
    // (each hand's weapon flag).
    (
        "reduce_enemy_elemental_resistance_with_weapons_%",
        &[
            (
                Quantity::Penetration(FIRE_T),
                Op::Base,
                1.0,
                Flags::WEAPON,
                NONE,
            ),
            (
                Quantity::Penetration(COLD_T),
                Op::Base,
                1.0,
                Flags::WEAPON,
                NONE,
            ),
            (
                Quantity::Penetration(LIGHTNING_T),
                Op::Base,
                1.0,
                Flags::WEAPON,
                NONE,
            ),
        ],
    ),
    // P3-8n, P6-11b review: "Damage with Weapons Penetrates #% Fire
    // Resistance": `FirePenetration` BASE with `ModFlag.Weapon` (each hand's
    // weapon flag; P3-8n read it from a main-hand state).
    (
        "reduce_enemy_fire_resistance_with_weapons_%",
        &[(
            Quantity::Penetration(FIRE_T),
            Op::Base,
            1.0,
            Flags::WEAPON,
            NONE,
        )],
    ),
    // P6-11n: the skill's own physical conversion to chaos
    // (`SkillPhysicalDamageConvertToChaos` BASE), as the cold one.
    (
        "skill_physical_damage_%_to_convert_to_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::SkillConversion(PHYS, CHAOS_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 962: `SkillPhysicalDamageConvertToCold` BASE
    // (P6-6a, the skill sweep).
    (
        "skill_physical_damage_%_to_convert_to_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::SkillConversion(PHYS, COLD_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // PoB `SkillStatMap.lua` 965: `SkillPhysicalDamageConvertToFire` BASE
    // (P6-6a, the skill sweep).
    (
        "skill_physical_damage_%_to_convert_to_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::SkillConversion(PHYS, FIRE_T)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
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
    // P3-8n: "Spells have a #% chance to deal Double Damage" (the same, with
    // `ModFlag.Spell`).
    (
        "spell_chance_to_deal_double_damage_%",
        &[(
            Quantity::Poe1(Poe1Quantity::DoubleDamageChance),
            Op::Base,
            1.0,
            Flags::SPELL,
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
    // PoB `SkillStatMap.lua` 1714: `ImpaleChance` BASE with
    // `KeywordFlag.Spell` (P6-2d).
    (
        "spells_impale_on_hit_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleChance),
            Op::Base,
            1.0,
            Flags::KEYWORD_SPELL,
            NONE,
        )],
    ),
    // P6-2b, P6-11b review: "Staff Attacks deal #% increased Damage with
    // Ailments": `Damage` INC with `ModFlag.Ailment` and `ModFlag.Staff`
    // (`ModParser.lua` 1111): the hand's weapon flag, which an ailment keeps
    // (`WeaponMask`); P6-2b read it from `UsingStaff` with the attack keyword.
    (
        "staff_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::STAFF.union(Flags::AILMENT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Critical Strike Chance with Staves": `CritChance` INC, `ModFlag.Staff` and `Hit`.
    (
        "staff_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "Staff Attacks deal #% increased Damage with Hits and Ailments": `Damage` INC, `ModFlag.Staff`, `KeywordFlag.Hit` or `Ailment`.
    (
        "staff_hit_and_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::STAFF,
            Flags::HIT.union(Flags::AILMENT),
        )],
    ),
    (
        "support_attack_skills_elemental_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::KEYWORD_ATTACK,
            Flags::ELEMENTAL,
        )],
    ),
    // P6-7c2: Brutality's own `statMap` (`Data/Skills/sup_str.lua` 802):
    // `PhysicalDamage` MORE.
    (
        "support_brutality_physical_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::PHYSICAL, NONE)],
    ),
    // PoB `Data/Skills/sup_dex.lua` 597 (Cast On Critical Strike's
    // triggered part): `Damage` MORE with `ModFlag.Spell` (P6-8a).
    (
        "support_cast_on_crit_spell_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    // PoB `Data/Skills/sup_str.lua` 1091 (Cast on Melee Kill's triggered
    // part): `Damage` MORE with `ModFlag.Spell` (P6-8a part 2).
    (
        "support_cast_on_melee_kill_spell_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    (
        "support_controlled_destruction_critical_strike_chance_+%_final",
        &[(Quantity::CritChance, Op::More, 1.0, NONE, NONE)],
    ),
    (
        "support_controlled_destruction_spell_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    // P6-11n: "#% more Elemental Damage" (`ElementalDamage` MORE).
    (
        "support_gem_elemental_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, NONE, Flags::ELEMENTAL)],
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
    // PoB `Data/Skills/sup_str.lua` 3660 (Melee Physical Damage): `Damage`
    // MORE with `ModFlag.MeleeHit` and `KeywordFlag.Bleed` or `Poison`
    // (P6-2c).
    (
        "support_melee_physical_damage_poison_and_bleeding_damage_+%_final_from_melee_hits",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::MELEE_HIT,
            Flags::BLEED.union(Flags::POISON),
        )],
    ),
    // P6-11b: "+# to Accuracy Rating with Swords": `Accuracy` BASE, `ModFlag.Sword` and `Hit`.
    (
        "sword_accuracy_rating",
        &[(
            Quantity::Accuracy,
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Accuracy Rating with Swords": `Accuracy` INC, `ModFlag.Sword` and `Hit`.
    (
        "sword_accuracy_rating_+%",
        &[(
            Quantity::Accuracy,
            Op::Increased,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "Sword Attacks deal #% increased Damage with Ailments": `Damage` INC, `ModFlag.Sword` and `Ailment`.
    (
        "sword_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::SWORD.union(Flags::AILMENT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Attack Speed with Swords": `Speed` INC, `ModFlag.Sword`, `Hit`, `Attack`.
    (
        "sword_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::SWORD.union(Flags::HIT).union(Flags::ATTACK),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Critical Strike Chance with Swords": `CritChance` INC, `ModFlag.Sword` and `Hit`.
    (
        "sword_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "+#% to Critical Strike Multiplier with Swords": `CritMultiplier` BASE, `ModFlag.Sword` and `Hit`.
    (
        "sword_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Damage with Swords": `Damage` INC, `ModFlag.Sword` and `Hit` (ModParser "with swords").
    (
        "sword_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Accuracy Rating with Two Handed Melee Weapons": `Accuracy` INC, `ModFlag.Weapon2H`, `WeaponMelee`, `Hit`.
    (
        "two_handed_melee_accuracy_rating_+%",
        &[(
            Quantity::Accuracy,
            Op::Increased,
            1.0,
            Flags::WEAPON_2H
                .union(Flags::WEAPON_MELEE)
                .union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Attack Speed with Two Handed Melee Weapons": `Speed` INC, `ModFlag.Weapon2H`, `WeaponMelee`, `Hit`, `Attack`.
    (
        "two_handed_melee_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::WEAPON_2H
                .union(Flags::WEAPON_MELEE)
                .union(Flags::HIT)
                .union(Flags::ATTACK),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Critical Strike Chance with Two Handed Melee Weapons": `CritChance` INC, `ModFlag.Weapon2H`, `WeaponMelee`, `Hit`.
    (
        "two_handed_melee_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::WEAPON_2H
                .union(Flags::WEAPON_MELEE)
                .union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "+#% to Critical Strike Multiplier with Two Handed Melee Weapons": `CritMultiplier` BASE, `ModFlag.Weapon2H`, `WeaponMelee`, `Hit`.
    (
        "two_handed_melee_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::WEAPON_2H
                .union(Flags::WEAPON_MELEE)
                .union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Physical Damage with Two Handed Melee Weapons": `PhysicalDamage` INC, `ModFlag.Weapon2H`, `WeaponMelee`, `Hit`.
    (
        "two_handed_melee_physical_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::PHYSICAL
                .union(Flags::WEAPON_2H)
                .union(Flags::WEAPON_MELEE)
                .union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "Attacks with Two Handed Melee Weapons deal #% increased Damage with Ailments": `Damage` INC, `ModFlag.Weapon2H`, `WeaponMelee`, `Ailment` (in PoB's `WeaponMask`, which an ailment keeps).
    (
        "two_handed_melee_weapon_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WEAPON_2H
                .union(Flags::WEAPON_MELEE)
                .union(Flags::AILMENT),
            NONE,
        )],
    ),
    // P6-11b: "Attacks with Two Handed Melee Weapons deal #% increased Damage with Hits and Ailments": `Damage` INC, `ModFlag.Weapon2H` and `WeaponMelee`, `KeywordFlag.Hit` or `Ailment`.
    (
        "two_handed_melee_weapon_hit_and_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WEAPON_2H.union(Flags::WEAPON_MELEE),
            Flags::HIT.union(Flags::AILMENT),
        )],
    ),
    // P6-11b: "#% increased Accuracy Rating with Wands": `Accuracy` INC, `ModFlag.Wand` and `Hit`.
    (
        "wand_accuracy_rating_+%",
        &[(
            Quantity::Accuracy,
            Op::Increased,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Attack Speed with Wands": `Speed` INC, `ModFlag.Wand`, `Hit`, `Attack`.
    (
        "wand_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::WAND.union(Flags::HIT).union(Flags::ATTACK),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Critical Strike Chance with Wands": `CritChance` INC, `ModFlag.Wand` and `Hit`.
    (
        "wand_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "+#% to Critical Strike Multiplier with Wands": `CritMultiplier` BASE, `ModFlag.Wand` and `Hit`.
    (
        "wand_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11c: "#% increased Damage with Wands": `Damage` INC, `ModFlag.Wand`
    // and `Hit` (`ModParser.lua` 945, "with wands").
    (
        "wand_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11b: "#% increased Elemental Damage with Wands": `ElementalDamage` INC, `ModFlag.Wand` and `Hit`.
    (
        "wand_elemental_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WAND.union(Flags::HIT),
            Flags::ELEMENTAL,
        )],
    ),
    // P6-11b: "Wand Attacks deal #% increased Damage with Hits and Ailments": `Damage` INC, `ModFlag.Wand`, `KeywordFlag.Hit` or `Ailment`.
    (
        "wand_hit_and_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WAND,
            Flags::HIT.union(Flags::AILMENT),
        )],
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
            | Quantity::DotMultiplier
            | Quantity::Poe1(
                Poe1Quantity::AilmentChance(..)
                    | Poe1Quantity::CannotInflict(..)
                    | Poe1Quantity::ImpaleChance
                    | Poe1Quantity::ImpaleEffect
                    | Poe1Quantity::ImpaleStacksMax
                    | Poe1Quantity::AilmentDuration
                    | Poe1Quantity::AilmentFaster
                    | Poe1Quantity::AilmentSlower
                    | Poe1Quantity::AilmentEffect
            )
            | Quantity::Poe1(
                Poe1Quantity::SkillBaseDamage(..)
                    | Poe1Quantity::SkillConversion(..)
                    | Poe1Quantity::Conversion(..)
                    | Poe1Quantity::DoubleDamageChance
                    | Poe1Quantity::Stack(..)
                    | Poe1Quantity::ArcaneSurgeCastSpeed
                    // Its mana regeneration (P6-10b).
                    | Poe1Quantity::ArcaneSurgeManaRegen
                    // Culling Strike (P6-12).
                    | Poe1Quantity::CullingStrike
                    // The skill's cost (P6-10b).
                    | Poe1Quantity::ManaCost
                    | Poe1Quantity::Cost
                    | Poe1Quantity::ManaCostNoMult
                    | Poe1Quantity::ManaCostEfficiency
                    | Poe1Quantity::CostEfficiency
                    // The life and energy shield costs and their sources
                    // (P6-10b part 8: Lifetap's).
                    | Poe1Quantity::LifeCost
                    | Poe1Quantity::EnergyShieldCost
                    | Poe1Quantity::LifeCostEfficiency
                    | Poe1Quantity::EnergyShieldCostEfficiency
                    | Poe1Quantity::CostLifeInsteadOfMana
                    | Poe1Quantity::CostEnergyShieldInsteadOfManaOrLife
                    | Poe1Quantity::HybridLifeCost
                    | Poe1Quantity::ManaCostAsLifeCost
                    | Poe1Quantity::ManaCostAsEnergyShieldCost
                    // The hit's leech (P6-10b part 9).
                    | Poe1Quantity::DamageLeech(..)
                    | Poe1Quantity::LeechRate(_)
                    | Poe1Quantity::InstantLeech(_)
                    | Poe1Quantity::CannotLeech(_)
                    // "Deal no ... Damage" (P6-7c2, Brutality's).
                    | Poe1Quantity::DealNo(..)
                    // P6-11j.
                    | Poe1Quantity::IgnorePdrChance
            )
    )
}

/// Skill stats the hit phase reads from the gem instance itself (the
/// skill's flags), so the mapping gives them no effect: `is_area_damage`
/// makes the skill's hits area damage (PoB's hand-written `area` flag in
/// its skill data; P3-9b reads the game's stat instead).
pub const SKILL_FLAGS: &[&str] = &["is_area_damage"];

/// A gem's "always hits" (P6-6a review): PoB's `skill("cannotBeEvaded")`
/// (`SkillStatMap.lua` 145), which `CalcOffence.lua` 2162 reads as the
/// `CannotBeEvaded` flag. Only through a gem: an item's "Hits can't be
/// Evaded" is tagged with its hand (`ModCache.lua` 9458), not modelled, so
/// on an item or a passive the stat stays unmapped (a reason).
pub const ALWAYS_HIT_STAT: &str = "global_always_hit";

/// The mapping of a gem's stats: the game's ([`Poe1Mapping`]), where every
/// effect must be one only the skill's hit reads ([`is_hit_quantity`]);
/// a stat with another effect is `Unmapped` (PoB applies a gem's stats to
/// the skill alone, so a global reading would be wrong), and the skill's
/// flags ([`SKILL_FLAGS`]) have no effect.
#[derive(Debug, Clone, Copy, Default)]
pub struct SkillMapping;

impl StatMapping for SkillMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        if SKILL_FLAGS.contains(&stat.as_str())
            || super::dot::DOT_SKILL_STATS.contains(&stat.as_str())
            || stat.as_str() == super::hit::DOUBLE_HITS_STAT
            || stat.as_str() == super::hit::ARROW_STAT
        {
            return StatMap::NoEffect;
        }
        if stat.as_str() == ALWAYS_HIT_STAT {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::CannotBeEvaded),
                op: Op::Flag,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
        }
        match Poe1Mapping.map(stat) {
            StatMap::Effects(e) if e.iter().all(|t| is_hit_quantity(t.quantity)) => {
                StatMap::Effects(e)
            }
            // A gem's minion modifiers (P3-8j): only the main skill's gems
            // are read through this mapping, so they are PoB's
            // `MinionModifier` entries of the skill's own `skillModList`,
            // which a redirect turns into the skill's (`poe1::minions`).
            StatMap::Effects(e)
                if e.iter()
                    .all(|t| super::minions::is_minion_quantity(t.quantity)) =>
            {
                StatMap::Effects(e)
            }
            // A field's reason marker (P6-10b: Momentum's Swiftness): the
            // stat's reason stays on that field.
            StatMap::Effects(e)
                if e.iter().all(|t| {
                    matches!(
                        t.quantity,
                        Quantity::Poe1(
                            Poe1Quantity::MovementReason(_)
                                | Poe1Quantity::RegenReason(..)
                                | Poe1Quantity::CostReason(_)
                                | Poe1Quantity::HitReason(_)
                                | Poe1Quantity::ReductionReason(_)
                                | Poe1Quantity::EnemyHitReason(_)
                                | Poe1Quantity::PoolReason(_)
                                | Poe1Quantity::EhpReason(_)
                                | Poe1Quantity::DotTakenReason(_)
                        )
                    )
                }) =>
            {
                StatMap::Effects(e)
            }
            StatMap::Effects(_) => StatMap::Unmapped,
            other => other,
        }
    }

    fn elsewhere(&self, quantity: Quantity) -> bool {
        Poe1Mapping.elsewhere(quantity)
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

/// A stat that applies under a named condition the build settles
/// ([`super::states`], P3-8f): `(stat, [(quantity, op, flags, condition)])`.
type StateRow = (&'static str, &'static [(Quantity, Op, Flags, &'static str)]);

/// The weapon-condition stats (P3-8f), sorted by stat ID, each read as PoB
/// v2.67.2 reads its game text: "while wielding a Staff" is
/// `{ type = "Condition", var = "UsingStaff" }` (`Modules/ModParser.lua`
/// 1619), "while holding a Shield" `UsingShield` (1598, `while holding a
/// (%w+)`); "to block attack damage" `BlockChance`, "to block spell
/// damage" `SpellBlockChance`, "defences" `Defences` (energy shield,
/// armour, evasion INC), "critical strike multiplier" `CritMultiplier`
/// BASE (252, 365, 375, 748).
pub const STATE_ROWS: &[StateRow] = &[
    // P6-11g: "+# Armour if you've Blocked Recently": `Armour` BASE,
    // `BlockedRecently` (the config's three blocked checks).
    (
        "X_armour_if_you_have_blocked_recently",
        &[(
            Quantity::Armour,
            Op::Base,
            NONE,
            conditions::BLOCKED_RECENTLY,
        )],
    ),
    // P6-11b: "+#% Chance to Block Attack Damage while wielding a Staff":
    // `BlockChance` BASE, `UsingStaff` (as `staff_block_%`).
    (
        "additional_staff_block_%",
        &[(BLOCK, Op::Base, NONE, states::USING_STAFF)],
    ),
    // P6-11o: "+# to Armour while affected by Determination" (`Armour` BASE,
    // `AffectedByDetermination`).
    (
        "armour_+_while_affected_by_determination",
        &[(Quantity::Armour, Op::Base, NONE, "AffectedByDetermination")],
    ),
    // P6-11a: "Attack Skills deal #% increased Damage with Ailments while
    // holding a Shield": `Damage` INC with `ModFlag.Ailment` and the attack
    // keyword, `UsingShield` (as the staff row).
    (
        "attack_ailment_damage_+%_while_holding_shield",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::AILMENT.union(Flags::KEYWORD_ATTACK),
            states::USING_SHIELD,
        )],
    ),
    // P6-11o: "#% increased Attack Damage while affected by Precision".
    (
        "attack_damage_+%_while_affected_by_precision",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::ATTACK,
            "AffectedByPrecision",
        )],
    ),
    // P6-11a review: "#% increased Attack Damage while holding a Shield":
    // `Damage` INC with `ModFlag.Attack`, `UsingShield`.
    (
        "attack_damage_+%_while_holding_a_shield",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::ATTACK,
            states::USING_SHIELD,
        )],
    ),
    // P6-11a review: "Attack Skills deal #% increased Damage while holding a
    // Shield": `Damage` INC with `KeywordFlag.Attack` (an attack's ailments
    // too), `UsingShield`.
    (
        "attack_skills_damage_+%_while_holding_shield",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::KEYWORD_ATTACK,
            states::USING_SHIELD,
        )],
    ),
    // P6-11o: "#% increased Attack Speed while affected by Precision".
    (
        "attack_speed_+%_while_affected_by_precision",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::ATTACK,
            "AffectedByPrecision",
        )],
    ),
    // P6-11a review: "#% increased Attack Speed while holding a Shield":
    // `Speed` INC with `ModFlag.Attack`, `UsingShield`.
    (
        "attack_speed_+%_while_holding_shield",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::ATTACK,
            states::USING_SHIELD,
        )],
    ),
    // P6-11b: "#% increased Attack Speed while Dual Wielding": `Speed` INC,
    // `ModFlag.Attack`, `DualWielding`.
    (
        "attack_speed_while_dual_wielding_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::ATTACK,
            states::DUAL_WIELDING,
        )],
    ),
    // Close-out: "#% increased Cast Speed if you've dealt a Critical Strike
    // Recently" (`Speed` INC, `ModFlag.Cast`, `Condition` `CritRecently`,
    // ModParser.lua 1785), from the config (`poe1::conditions`).
    (
        "cast_speed_+%_if_have_crit_recently",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::CAST,
            conditions::CRIT_RECENTLY,
        )],
    ),
    // P6-11p: "#% chance to gain Onslaught for 4 seconds on Kill" (`Condition:Onslaught`
    // FLAG, `Condition` `KilledRecently`): Onslaught while the config says
    // you have killed recently (`poe1::buffs`).
    (
        "chance_to_gain_onslaught_on_kill_%",
        &[(
            Quantity::Poe1(Poe1Quantity::Onslaught),
            Op::Flag,
            NONE,
            conditions::KILLED_RECENTLY,
        )],
    ),
    // P6-11o: "#% increased Critical Strike Chance while affected by Wrath".
    (
        "critical_strike_chance_+%_while_affected_by_wrath",
        &[(Quantity::CritChance, Op::Increased, NONE, "AffectedByWrath")],
    ),
    // P3-8n: "against Enemies that are on Full Life" (`ActorCondition` enemy
    // `FullLife`, ModParser.lua 1945), from the config (`poe1::conditions`).
    (
        "critical_strike_chance_against_enemies_on_full_life_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            NONE,
            conditions::ENEMY_FULL_LIFE,
        )],
    ),
    // P3-8n: "against Unique Enemies" (enemy `RareOrUnique`, 1943).
    (
        "critical_strike_multiplier_+_vs_unique_enemies",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            NONE,
            conditions::ENEMY_RARE_OR_UNIQUE,
        )],
    ),
    // P6-11o: "+#% to Critical Strike Multiplier while affected by Anger".
    (
        "critical_strike_multiplier_+_while_affected_by_anger",
        &[(Quantity::CritMultiplier, Op::Base, NONE, "AffectedByAnger")],
    ),
    // P6-11o: "+#% to Critical Strike Multiplier while affected by
    // Precision".
    (
        "critical_strike_multiplier_+_while_affected_by_precision",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            NONE,
            "AffectedByPrecision",
        )],
    ),
    // P3-8n: "if you've Shattered an Enemy Recently" (1935).
    (
        "damage_+%_if_enemy_shattered_recently",
        &[(
            Quantity::Damage,
            Op::Increased,
            NONE,
            conditions::SHATTERED_ENEMY_RECENTLY,
        )],
    ),
    // P3-8n: "if you have Shocked an Enemy Recently" (1815).
    (
        "damage_+%_if_you_have_shocked_recently",
        &[(
            Quantity::Damage,
            Op::Increased,
            NONE,
            conditions::SHOCKED_ENEMY_RECENTLY,
        )],
    ),
    // P6-11b: "#% increased Damage while wielding a Wand": `Damage` INC,
    // `UsingWand`.
    (
        "damage_+%_while_wielding_wand",
        &[(Quantity::Damage, Op::Increased, NONE, states::USING_WAND)],
    ),
    (
        "defences_+%_while_wielding_staff",
        &[
            (
                Quantity::EnergyShield,
                Op::Increased,
                NONE,
                states::USING_STAFF,
            ),
            (Quantity::Armour, Op::Increased, NONE, states::USING_STAFF),
            (Quantity::Evasion, Op::Increased, NONE, states::USING_STAFF),
        ],
    ),
    // P6-11a: "+#% Chance to Block Attack Damage while Dual Wielding or
    // holding a Shield": `BlockChance` BASE, `DualWielding` or `UsingShield`
    // (PoB's `varList`); two rows, never both (a shield is no weapon).
    (
        "dual_wield_or_shield_block_%",
        &[
            (BLOCK, Op::Base, NONE, states::DUAL_WIELDING),
            (BLOCK, Op::Base, NONE, states::USING_SHIELD),
        ],
    ),
    // P6-10b part 13: "#% increased Elemental Damage taken if you've been
    // Hit Recently" (`ElementalDamageTaken` INC, `Condition`
    // `BeenHitRecently`), the config's (`poe1::conditions`).
    (
        "elemental_damage_taken_+%_if_been_hit_recently",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Elemental,
            )),
            Op::Increased,
            NONE,
            conditions::BEEN_HIT_RECENTLY,
        )],
    ),
    // P6-10b part 13: "#% increased Elemental Damage taken if you haven't
    // been Hit Recently" (`ElementalDamageTaken` INC, `Condition`
    // `BeenHitRecently` negated).
    (
        "elemental_damage_taken_+%_if_not_hit_recently",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Elemental,
            )),
            Op::Increased,
            NONE,
            conditions::NOT_HIT_RECENTLY,
        )],
    ),
    // P6-10b: "Regenerate # Energy Shield per second while a Rare or Unique
    // Enemy is Nearby" (`EnergyShieldRegen` BASE, the stat per minute; PoB's
    // enemy `ActorCondition` on `NearbyRareOrUniqueEnemy` or `RareOrUnique`).
    (
        "energy_shield_regeneration_rate_per_minute_if_rare_or_unique_enemy_nearby",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPerMinute(Pool::EnergyShield)),
            Op::Base,
            NONE,
            conditions::ENEMY_RARE_OR_UNIQUE_NEARBY,
        )],
    ),
    (
        "global_critical_strike_chance_+%_while_holding_staff",
        &[(
            Quantity::CritChance,
            Op::Increased,
            NONE,
            states::USING_STAFF,
        )],
    ),
    (
        "global_critical_strike_multiplier_+_while_holding_staff",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            NONE,
            states::USING_STAFF,
        )],
    ),
    // P6-10b: "#% increased Recovery rate of Life and Energy Shield if
    // you've stopped taking Damage Over Time Recently" (`LifeRecoveryRate`
    // and `EnergyShieldRecoveryRate` INC).
    (
        "life_and_energy_shield_recovery_rate_+%_if_stopped_taking_damage_over_time_recently",
        &[
            (
                Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::Life)),
                Op::Increased,
                NONE,
                conditions::STOPPED_TAKING_DOT_RECENTLY,
            ),
            (
                Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::EnergyShield)),
                Op::Increased,
                NONE,
                conditions::STOPPED_TAKING_DOT_RECENTLY,
            ),
        ],
    ),
    // P6-11l: "#% more maximum Life if you have at least 6 Life Masteries
    // allocated" (`Life` MORE, `MultiplierThreshold` `AllocatedLifeMastery`
    // 6).
    (
        "life_mastery_count_maximum_life_+%_final",
        &[(Quantity::Life, Op::More, NONE, states::SIX_LIFE_MASTERIES)],
    ),
    // P6-10b: "Regenerate #% of Life per second while stationary" (the
    // Soul of Shakari's; `LifeRegenPercent` BASE, `Stationary`), per minute.
    (
        "life_regeneration_rate_per_minute_%_while_stationary",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(Pool::Life)),
            Op::Base,
            NONE,
            conditions::STATIONARY,
        )],
    ),
    // P6-11o: the maximum of "Adds # to # Cold Damage while affected by Hatred".
    (
        "maximum_added_cold_damage_while_affected_by_hatred",
        &[(
            Quantity::AddedDamage(DamageType::Cold, Bound::Max),
            Op::Base,
            NONE,
            "AffectedByHatred",
        )],
    ),
    // P3-8n: "#% increased maximum Life if there are no Life Modifiers on
    // Equipped Body Armour" (`Data/ModCache.lua` 3483: an `ItemCondition`
    // on the body armour's lines; the engine reads the mods' tags,
    // `poe1::states`).
    (
        "maximum_life_+%_if_no_life_tags_on_body_armour",
        &[(
            Quantity::Life,
            Op::Increased,
            NONE,
            states::NO_LIFE_MODIFIERS_ON_BODY_ARMOUR,
        )],
    ),
    // P3-8n: "Melee Damage with Hits at Close Range": `Damage` INC with
    // `ModFlag.Melee` (716) and `KeywordFlag.Hit` (990), `AtCloseRange` (1939).
    (
        "melee_damage_+%_at_close_range",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::MELEE.union(Flags::HIT),
            conditions::AT_CLOSE_RANGE,
        )],
    ),
    // P6-11o: the minimum of "Adds # to # Cold Damage while affected by Hatred".
    (
        "minimum_added_cold_damage_while_affected_by_hatred",
        &[(
            Quantity::AddedDamage(DamageType::Cold, Bound::Min),
            Op::Base,
            NONE,
            "AffectedByHatred",
        )],
    ),
    // P6-11p: "Gain Onslaught for # seconds on Kill" (`Condition:Onslaught`
    // FLAG, `Condition` `KilledRecently`): Onslaught while the config says
    // you have killed recently (`poe1::buffs`).
    (
        "onslaught_time_granted_on_kill_ms",
        &[(
            Quantity::Poe1(Poe1Quantity::Onslaught),
            Op::Flag,
            NONE,
            conditions::KILLED_RECENTLY,
        )],
    ),
    // P6-11a: "#% increased Physical Attack Damage while holding a Shield":
    // `PhysicalDamage` INC with `ModFlag.Attack`, `UsingShield`.
    (
        "physical_attack_damage_+%_while_holding_a_shield",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::ATTACK.union(Flags::PHYSICAL),
            states::USING_SHIELD,
        )],
    ),
    // P6-10b part 13: "#% increased Physical Damage over Time taken while
    // moving" (`PhysicalDamageTakenOverTime` INC, `Condition` `Moving`).
    (
        "physical_damage_over_time_taken_+%_while_moving",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::OverTime,
                TakenOf::Type(DamageType::Physical),
            )),
            Op::Increased,
            NONE,
            conditions::MOVING,
        )],
    ),
    // P6-11o: "#% increased Physical Damage while affected by Pride".
    (
        "pride_physical_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::PHYSICAL,
            "AffectedByPride",
        )],
    ),
    // P6-11o: "Damage Penetrates #% Cold Resistance while affected by Hatred"
    // (`ColdPenetration` BASE).
    (
        "reduce_enemy_cold_resistance_%_while_affected_by_hatred",
        &[(
            Quantity::Penetration(DamageType::Cold),
            Op::Base,
            NONE,
            "AffectedByHatred",
        )],
    ),
    // P6-11o: "Damage Penetrates #% Lightning Resistance while affected by
    // Wrath" (`LightningPenetration` BASE).
    (
        "reduce_enemy_lightning_resistance_%_while_affected_by_wrath",
        &[(
            Quantity::Penetration(DamageType::Lightning),
            Op::Base,
            NONE,
            "AffectedByWrath",
        )],
    ),
    // P6-11a: "+#% Elemental Resistances while holding a Shield":
    // `ElementalResist` BASE, `UsingShield`.
    (
        "resist_all_elements_+%_while_holding_shield",
        &[
            (FIRE, Op::Base, NONE, states::USING_SHIELD),
            (COLD, Op::Base, NONE, states::USING_SHIELD),
            (LIGHTNING, Op::Base, NONE, states::USING_SHIELD),
        ],
    ),
    // P6-11a: "+#% Chance to Block Attack Damage while holding a Shield":
    // `BlockChance` BASE, `UsingShield`.
    (
        "shield_block_%",
        &[(BLOCK, Op::Base, NONE, states::USING_SHIELD)],
    ),
    (
        "shield_spell_block_%",
        &[(SPELL_BLOCK, Op::Base, NONE, states::USING_SHIELD)],
    ),
    // P6-11a: "+#% chance to Block Spell Damage while holding a Staff or
    // Shield": `SpellBlockChance` BASE, `UsingStaff` or `UsingShield`; two
    // rows, never both (a staff is two-handed).
    (
        "spell_block_chance_%_while_holding_staff_or_shield",
        &[
            (SPELL_BLOCK, Op::Base, NONE, states::USING_STAFF),
            (SPELL_BLOCK, Op::Base, NONE, states::USING_SHIELD),
        ],
    ),
    (
        "spell_block_with_staff_%",
        &[(SPELL_BLOCK, Op::Base, NONE, states::USING_STAFF)],
    ),
    (
        "spell_damage_+%_while_holding_shield",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::SPELL,
            states::USING_SHIELD,
        )],
    ),
    (
        "spell_staff_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::SPELL,
            states::USING_STAFF,
        )],
    ),
    // P6-11o: "+#% chance to Suppress Spell Damage while affected by Grace".
    (
        "spell_suppression_chance_%_while_affected_by_grace",
        &[(SUPPRESSION, Op::Base, NONE, "AffectedByGrace")],
    ),
    (
        "staff_block_%",
        &[(BLOCK, Op::Base, NONE, states::USING_STAFF)],
    ),
    // P3-8n: Arcane Surge Support's "more Spell Damage while you have Arcane
    // Surge" (`Damage` MORE, `ModFlag.Spell`, `AffectedByArcaneSurge`,
    // `sup_int.lua` 235), the config's buff (`poe1::conditions`).
    (
        "support_arcane_surge_spell_damage_+%_final_while_you_have_arcane_surge",
        &[(
            Quantity::Damage,
            Op::More,
            Flags::SPELL,
            conditions::ARCANE_SURGE,
        )],
    ),
    // P6-8a part 2: Cast while Channelling's triggered part,
    // `TriggeredDamage` MORE with `SkillType.Triggered`
    // (`Data/Skills/sup_int.lua` 759), which PoB turns into `Damage` MORE
    // when the trigger's source is the skill itself (`poe1::triggers`).
    (
        "support_cast_while_channelling_triggered_skill_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            NONE,
            super::triggers::TRIGGERED_DAMAGE,
        )],
    ),
    // P6-10b part 8: Lifetap's "more Damage while you have the Lifetap
    // buff" (`Damage` MORE, `Condition` `Lifetap`, `sup_str.lua` 3514), the
    // config's buff (`poe1::conditions`).
    (
        "support_lifetap_damage_+%_final_while_buffed",
        &[(Quantity::Damage, Op::More, NONE, conditions::LIFETAP)],
    ),
    // P6-10b part 5: "#% additional Physical Damage Reduction while
    // stationary" (`PhysicalDamageReduction` BASE, `Stationary`).
    (
        "while_stationary_gain_additional_physical_damage_reduction_%",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageReduction(DamageType::Physical)),
            Op::Base,
            NONE,
            conditions::STATIONARY,
        )],
    ),
];

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
    // P3-8n: "#% increased Action Speed" (the canonical stat is negated:
    // "#% reduced Action Speed" for a positive value), PoB's `ActionSpeed`
    // INC; the hit scales a self-cast skill's rate by it.
    (
        "action_speed_-%",
        &[(
            Quantity::Poe1(Poe1Quantity::ActionSpeed),
            Op::Increased,
            -1.0,
        )],
    ),
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
    // Basalt Flask's buff, "20% more Armour" in PoB's base data: `Armour`
    // MORE (P6-4b).
    (
        "basalt_flask_armour_+%_final",
        &[(Quantity::Armour, Op::More, 1.0)],
    ),
    // P6-10b part 5: "#% additional Physical Damage Reduction"
    // (`PhysicalDamageReduction` BASE), read by `poe1::reduction`.
    (
        "base_additional_physical_damage_reduction_%",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageReduction(DamageType::Physical)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "Armour also applies to Chaos Damage taken from Hits" (`ArmourAppliesToChaosDamageTaken` BASE).
    (
        "base_armour_applies_to_chaos_damage",
        &[(
            Quantity::Poe1(Poe1Quantity::ArmourAppliesTo(DamageType::Chaos)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "Armour also applies to Lightning Damage taken from Hits" (`ArmourAppliesToLightningDamageTaken` BASE).
    (
        "base_armour_applies_to_lightning_damage",
        &[(
            Quantity::Poe1(Poe1Quantity::ArmourAppliesTo(DamageType::Lightning)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 14: "#% chance to avoid Projectiles" (`AvoidProjectilesChance`
    // BASE), the effective health pool's.
    (
        "base_avoid_projectiles_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::AvoidProjectiles),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 9: "Cannot Leech Life" (`CannotLeechLife` FLAG).
    (
        "base_cannot_leech_life",
        &[(
            Quantity::Poe1(Poe1Quantity::CannotLeech(Pool::Life)),
            Op::Flag,
            1.0,
        )],
    ),
    // P6-10b part 9: "Cannot Leech Mana" (`CannotLeechMana` FLAG).
    (
        "base_cannot_leech_mana",
        &[(
            Quantity::Poe1(Poe1Quantity::CannotLeech(Pool::Mana)),
            Op::Flag,
            1.0,
        )],
    ),
    ("base_chaos_damage_resistance_%", &[(CHAOS, Op::Base, 1.0)]),
    ("base_cold_damage_resistance_%", &[(COLD, Op::Base, 1.0)]),
    // P6-10b: "#% increased Cost of Skills" (`Cost` INC).
    (
        "base_cost_+%",
        &[(Quantity::Poe1(Poe1Quantity::Cost), Op::Increased, 1.0)],
    ),
    // P6-10b part 15: "#% of Damage is taken from Mana before Life"
    // (`DamageTakenFromManaBeforeLife` BASE, or the type's).
    (
        "base_damage_removed_from_mana_before_life_%",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaBeforeLife(TakenOf::All)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Damage taken" (`DamageTaken` INC).
    (
        "base_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::All,
            )),
            Op::Increased,
            1.0,
        )],
    ),
    ("base_dexterity", &[(DEX, Op::Base, 1.0)]),
    (
        "base_dexterity_and_intelligence",
        &[(DEX, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    // P6-10b part 9: "#% of Cold Damage Leeched as Energy Shield" (`ColdDamageEnergyShieldLeech` BASE).
    (
        "base_energy_shield_leech_from_cold_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::EnergyShield,
                LeechSource::Type(DamageType::Cold),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Fire Damage Leeched as Energy Shield" (`FireDamageEnergyShieldLeech` BASE).
    (
        "base_energy_shield_leech_from_fire_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::EnergyShield,
                LeechSource::Type(DamageType::Fire),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Lightning Damage Leeched as Energy Shield" (`LightningDamageEnergyShieldLeech` BASE).
    (
        "base_energy_shield_leech_from_lightning_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::EnergyShield,
                LeechSource::Type(DamageType::Lightning),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b: "Regenerate # Energy Shield per second" (`EnergyShieldRegen`
    // BASE, the stat per minute).
    (
        "base_energy_shield_regeneration_rate_per_minute",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::EnergyShield)),
            Op::Base,
            1.0 / 60.0,
        )],
    ),
    // P6-10b: "Regenerate #% of Energy Shield per second"
    // (`EnergyShieldRegenPercent` BASE, the stat per minute).
    (
        "base_energy_shield_regeneration_rate_per_minute_%",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPercent(Pool::EnergyShield)),
            Op::Base,
            1.0 / 60.0,
        )],
    ),
    ("base_evasion_rating", &[(Quantity::Evasion, Op::Base, 1.0)]),
    ("base_fire_damage_resistance_%", &[(FIRE, Op::Base, 1.0)]),
    // P6-10b part 12: "#% of Fire Damage from Hits taken as Physical Damage" (`FireDamageFromHitsTakenAsPhysical` BASE).
    (
        "base_fire_hit_damage_taken_%_as_physical",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Fire),
                DamageType::Physical,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    ("base_intelligence", &[(INT, Op::Base, 1.0)]),
    // P6-10b part 8: "#% increased Life Cost of Skills" (`LifeCost` INC).
    (
        "base_life_cost_+%",
        &[(Quantity::Poe1(Poe1Quantity::LifeCost), Op::Increased, 1.0)],
    ),
    // P6-10b part 9: "#% of Chaos Damage Leeched as Life" (`ChaosDamageLifeLeech` BASE).
    (
        "base_life_leech_from_chaos_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Chaos),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Cold Damage Leeched as Life" (`ColdDamageLifeLeech` BASE).
    (
        "base_life_leech_from_cold_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Cold),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Elemental Damage Leeched as Life" (`ElementalDamageLifeLeech` BASE).
    (
        "base_life_leech_from_elemental_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Elemental,
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Fire Damage Leeched as Life" (`FireDamageLifeLeech` BASE).
    (
        "base_life_leech_from_fire_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Fire),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Lightning Damage Leeched as Life" (`LightningDamageLifeLeech` BASE).
    (
        "base_life_leech_from_lightning_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Lightning),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% of Physical Damage Leeched as Life" (`PhysicalDamageLifeLeech` BASE).
    (
        "base_life_leech_from_physical_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Physical),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b: "Regenerate # Life per second" (`LifeRegen` BASE, the stat per minute).
    (
        "base_life_regeneration_rate_per_minute",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::Life)),
            Op::Base,
            1.0 / 60.0,
        )],
    ),
    (
        "base_lightning_damage_resistance_%",
        &[(LIGHTNING, Op::Base, 1.0)],
    ),
    // P6-10b: "Skills Cost +# Mana" (`ManaCostNoMult` BASE).
    (
        "base_mana_cost_+",
        &[(Quantity::Poe1(Poe1Quantity::ManaCostNoMult), Op::Base, 1.0)],
    ),
    // P6-10b: "#% reduced Mana Cost of Skills" (`ManaCost` INC, negated).
    (
        "base_mana_cost_-%",
        &[(Quantity::Poe1(Poe1Quantity::ManaCost), Op::Increased, -1.0)],
    ),
    // P6-10b: "#% increased Mana Cost Efficiency" (`ManaCostEfficiency` INC).
    (
        "base_mana_cost_efficiency_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaCostEfficiency),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 9: "#% of Lightning Damage Leeched as Mana" (`LightningDamageManaLeech` BASE).
    (
        "base_mana_leech_from_lightning_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Mana,
                LeechSource::Type(DamageType::Lightning),
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b: "Regenerate # Mana per second" (`ManaRegen` BASE).
    (
        "base_mana_regeneration_rate_per_minute",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::Mana)),
            Op::Base,
            1.0 / 60.0,
        )],
    ),
    // P6-10b part 12: "+# to maximum Fortification" (`MaximumFortification`
    // BASE): while Fortified, the stacks the enemy's hit reads
    // (`poe1::enemy_hit`); modifiers per fortification stay unmapped.
    (
        "base_max_fortification",
        &[(
            Quantity::Poe1(Poe1Quantity::MaximumFortification),
            Op::Base,
            1.0,
        )],
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
    // P6-10b: "#% increased Movement Speed" (`MovementSpeed` INC).
    (
        "base_movement_velocity_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Physical Damage taken over time"
    // (`PhysicalDamageTakenOverTime` INC).
    (
        "base_physical_damage_over_time_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::OverTime,
                TakenOf::Type(DamageType::Physical),
            )),
            Op::Increased,
            1.0,
        )],
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
    // P6-10b part 12: "You take #% reduced Extra Damage from Critical Strikes" (`ReduceCritExtraDamage` BASE).
    (
        "base_self_critical_strike_multiplier_-%",
        &[(
            Quantity::Poe1(Poe1Quantity::CritExtraDamageReduction),
            Op::Base,
            1.0,
        )],
    ),
    // "Onslaught" (PoB `ModParser.lua` 3973: `Condition:Onslaught`): a flag
    // `poe1::buffs` turns into Onslaught's speed once (P3-12j).
    (
        "base_should_have_onslaught_from_stat",
        &[(Quantity::Poe1(Poe1Quantity::Onslaught), Op::Flag, 1.0)],
    ),
    // P6-10b: "#% increased Cost Efficiency" (`CostEfficiency` INC).
    (
        "base_skill_cost_efficiency_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::CostEfficiency),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 8: "Skills Cost Life instead of Mana" (Lifetap;
    // `CostLifeInsteadOfMana`).
    (
        "base_skill_cost_life_instead_of_mana",
        &[(
            Quantity::Poe1(Poe1Quantity::CostLifeInsteadOfMana),
            Op::Flag,
            1.0,
        )],
    ),
    // P6-10b part 8: "Skills Cost Life instead of #% of Mana Cost"
    // (`HybridManaAndLifeCost_Life` BASE).
    (
        "base_skill_cost_life_instead_of_mana_%",
        &[(Quantity::Poe1(Poe1Quantity::HybridLifeCost), Op::Base, 1.0)],
    ),
    ("base_spell_block_%", &[(SPELL_BLOCK, Op::Base, 1.0)]),
    // P6-10b part 12: "Prevent +#% of Suppressed Spell Damage" (`SpellSuppressionEffect` BASE).
    (
        "base_spell_damage_%_suppressed",
        &[(
            Quantity::Poe1(Poe1Quantity::SpellSuppressionEffect),
            Op::Base,
            1.0,
        )],
    ),
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
    // P6-11f: "+#% Chance to Block Attack Damage per 5% Chance to Block on
    // Equipped Shield" (`BlockChance` BASE, `PerStat` `ShieldBlockChance`
    // `div = 5`); `phases` reads it.
    (
        "chance_to_block_attack_damage_per_5%_chance_to_block_on_equipped_shield_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockPer5(false)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "+# Chaos Damage taken" (`ChaosDamageTaken` BASE).
    (
        "chaos_damage_taken_+",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Type(DamageType::Chaos),
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Chaos Damage taken" (`ChaosDamageTaken` INC).
    (
        "chaos_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Type(DamageType::Chaos),
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Chaos Damage taken over time" (`ChaosDamageTakenOverTime` INC).
    (
        "chaos_damage_taken_over_time_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::OverTime,
                TakenOf::Type(DamageType::Chaos),
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Chaos Damage taken as Fire Damage" (`ChaosDamageTakenAsFire` BASE).
    (
        "chaos_hit_and_dot_damage_%_taken_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Chaos),
                DamageType::Fire,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Chaos Damage taken as Lightning Damage" (`ChaosDamageTakenAsLightning` BASE).
    (
        "chaos_hit_and_dot_damage_%_taken_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Chaos),
                DamageType::Lightning,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    (
        "cold_and_chaos_damage_resistance_%",
        &[(COLD, Op::Base, 1.0), (CHAOS, Op::Base, 1.0)],
    ),
    (
        "cold_and_lightning_damage_resistance_%",
        &[(COLD, Op::Base, 1.0), (LIGHTNING, Op::Base, 1.0)],
    ),
    // P6-10b part 12: "#% of Cold Damage from Hits taken as Fire Damage" (`ColdDamageFromHitsTakenAsFire` BASE).
    (
        "cold_damage_taken_%_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Cold),
                DamageType::Fire,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Cold Damage from Hits taken as Lightning Damage" (`ColdDamageFromHitsTakenAsLightning` BASE).
    (
        "cold_damage_taken_%_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Cold),
                DamageType::Lightning,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "+# Cold Damage taken from Hits" (`ColdDamageTakenWhenHit` BASE).
    (
        "cold_damage_taken_+",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Hit,
                TakenOf::Type(DamageType::Cold),
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Cold Damage taken" (`ColdDamageTaken` INC).
    (
        "cold_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Type(DamageType::Cold),
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Cold Damage taken as Fire Damage" (`ColdDamageTakenAsFire` BASE).
    (
        "cold_hit_and_dot_damage_%_taken_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Cold),
                DamageType::Fire,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Cold Damage taken as Lightning Damage" (`ColdDamageTakenAsLightning` BASE).
    (
        "cold_hit_and_dot_damage_%_taken_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Cold),
                DamageType::Lightning,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P3-8n: "#% increased Effect of Consecrated Ground you create"
    // (`ConsecratedGroundEffect` INC, `ModParser.lua` 465): the enemy's
    // damage taken on it (`poe1::damage_taken`).
    (
        "consecrated_ground_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ConsecratedGroundEffect),
            Op::Increased,
            1.0,
        )],
    ),
    // P3-8n: "Consecrated Ground you create applies #% increased Damage taken
    // to Enemies" (the enemy's `DamageTakenConsecratedGround` INC, 2901).
    (
        "consecrated_ground_enemy_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ConsecratedGroundDamageTaken),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "You have Consecrated Ground around you while stationary"
    // (`Condition:OnConsecratedGround` while `Stationary`, `ModParser.lua`
    // 2902): a flag `poe1::buffs` reads with the config's stationary count.
    (
        "consecrated_ground_while_stationary_radius",
        &[(
            Quantity::Poe1(Poe1Quantity::ConsecratedWhileStationary),
            Op::Flag,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Damage taken from Damage Over Time"
    // (`DamageTakenOverTime` INC).
    (
        "degen_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::OverTime,
                TakenOf::All,
            )),
            Op::Increased,
            1.0,
        )],
    ),
    ("dexterity_+%", &[(DEX, Op::Increased, 1.0)]),
    // P6-10b part 12: "#% of Elemental Damage from Hits taken as Chaos Damage" (`ElementalDamageFromHitsTakenAsChaos` BASE).
    (
        "elemental_damage_taken_%_as_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Elemental,
                DamageType::Chaos,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Elemental Damage taken" (`ElementalDamageTaken` INC).
    (
        "elemental_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Elemental,
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Elemental Damage from Hits taken as Physical Damage" (`ElementalDamageFromHitsTakenAsPhysical` BASE).
    (
        "elemental_hit_damage_taken_%_as_physical",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Elemental,
                DamageType::Physical,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b: "#% faster start of Energy Shield Recharge"
    // (`EnergyShieldRechargeFaster` INC).
    (
        "energy_shield_delay_-%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnergyShieldRechargeFaster),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 9: "#% of Damage Leeched as Energy Shield" (`DamageEnergyShieldLeech` BASE).
    (
        "energy_shield_leech_from_any_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::EnergyShield,
                LeechSource::Any,
            )),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% increased total Recovery per second from Energy Shield Leech" (`EnergyShieldLeechRate` INC).
    (
        "energy_shield_leech_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechRate(Pool::EnergyShield)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "#% increased Energy Shield Recharge Rate"
    // (`EnergyShieldRecharge` INC).
    (
        "energy_shield_recharge_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnergyShieldRecharge),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "#% increased Energy Shield Recovery rate"
    // (`EnergyShieldRecoveryRate` INC).
    (
        "energy_shield_recovery_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::EnergyShield)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "#% increased Energy Shield Regeneration rate"
    // (`EnergyShieldRegen` INC).
    (
        "energy_shield_regeneration_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::EnergyShield)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "Regenerate # Energy Shield per second" (`EnergyShieldRegen`
    // BASE).
    (
        "energy_shield_regeneration_rate_per_second",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::EnergyShield)),
            Op::Base,
            1.0,
        )],
    ),
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
    // P6-10b part 12: "#% of Fire Damage from Hits taken as Cold Damage" (`FireDamageFromHitsTakenAsCold` BASE).
    (
        "fire_damage_taken_%_as_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Fire),
                DamageType::Cold,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Fire Damage from Hits taken as Lightning Damage" (`FireDamageFromHitsTakenAsLightning` BASE).
    (
        "fire_damage_taken_%_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Fire),
                DamageType::Lightning,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "+# Fire Damage taken from Hits" (`FireDamageTakenWhenHit` BASE).
    (
        "fire_damage_taken_+",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Hit,
                TakenOf::Type(DamageType::Fire),
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Fire Damage taken" (`FireDamageTaken` INC).
    (
        "fire_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Type(DamageType::Fire),
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Fire Damage taken as Lightning Damage" (`FireDamageTakenAsLightning` BASE).
    (
        "fire_hit_and_dot_damage_%_taken_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Fire),
                DamageType::Lightning,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // "Flasks applied to you have #% increased Effect": PoB's `FlaskEffect`
    // INC (`SkillStatMap.lua` 422); `poe1::flasks` reads it (P6-4b).
    (
        "flask_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::FlaskEffect),
            Op::Increased,
            1.0,
        )],
    ),
    // P3-8n: "Gain Convergence when you Hit a Unique Enemy"
    // (`Condition:CanGainConvergence`, 2661; `poe1::buffs`).
    (
        "gain_convergence_on_hitting_unique_enemy",
        &[(
            Quantity::Poe1(Poe1Quantity::CanGainConvergence),
            Op::Flag,
            1.0,
        )],
    ),
    (
        "global_defences_+%",
        &[
            (Quantity::EnergyShield, Op::Increased, 1.0),
            (Quantity::Armour, Op::Increased, 1.0),
            (Quantity::Evasion, Op::Increased, 1.0),
        ],
    ),
    // P6-10b: "#% more Mana Cost of Skills" (`ManaCost` MORE).
    (
        "hierophant_mana_cost_+%_final",
        &[(Quantity::Poe1(Poe1Quantity::ManaCost), Op::More, 1.0)],
    ),
    // P6-10b part 9: "#% of Chaos Damage Leeched as Life" (`ChaosDamageLifeLeech` BASE).
    (
        "horrible_ugly_life_leech_from_chaos_damage_perhundredthousand",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Chaos),
            )),
            Op::Base,
            0.001,
        )],
    ),
    // P6-10b part 9: "#% of Cold Damage Leeched as Life" (`ColdDamageLifeLeech` BASE).
    (
        "horrible_ugly_life_leech_from_cold_damage_perhundredthousand",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Cold),
            )),
            Op::Base,
            0.001,
        )],
    ),
    // P6-10b part 9: "#% of Fire Damage Leeched as Life" (`FireDamageLifeLeech` BASE).
    (
        "horrible_ugly_life_leech_from_fire_damage_perhundredthousand",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Fire),
            )),
            Op::Base,
            0.001,
        )],
    ),
    // P6-10b part 9: "#% of Lightning Damage Leeched as Life" (`LightningDamageLifeLeech` BASE).
    (
        "horrible_ugly_life_leech_from_lightning_damage_perhundredthousand",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Lightning),
            )),
            Op::Base,
            0.001,
        )],
    ),
    // P6-10b part 9: "#% of Physical Damage Leeched as Life" (`PhysicalDamageLifeLeech` BASE).
    (
        "horrible_ugly_life_leech_from_physical_damage_perhundredthousand",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(
                Pool::Life,
                LeechSource::Type(DamageType::Physical),
            )),
            Op::Base,
            0.001,
        )],
    ),
    // P3-8n: "Enemies Ignited or Chilled by you have #% to Elemental
    // Resistances" (the enemy's `ElementalResist` BASE while ignited or
    // chilled, 4054; `poe1::damage_taken`).
    (
        "ignites_and_chill_apply_elemental_resistance_+",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentElementalResistance),
            Op::Base,
            1.0,
        )],
    ),
    ("intelligence_+%", &[(INT, Op::Increased, 1.0)]),
    // P6-10b part 11: Unwavering Stance (the keystone: "Cannot Evade enemy
    // Attacks", `CannotEvade`; its "Cannot be Stunned" reaches no field).
    (
        "keystone_unwavering_stance",
        &[(Quantity::Poe1(Poe1Quantity::CannotEvade), Op::Flag, 1.0)],
    ),
    // P6-12: Culling Strike (PoB `CullPercent` MAX 10, `ModParser.lua`
    // 5066): `CullMultiplier`, which scales `CombinedDPS`
    // (`CalcOffence.lua` 3221, 6138), not the hit or `TotalDPS`.
    (
        "kill_enemy_on_hit_if_under_10%_life",
        &[(Quantity::Poe1(Poe1Quantity::CullingStrike), Op::Flag, 1.0)],
    ),
    // P6-10b part 9: "#% of Leech is Instant" (`InstantEnergyShieldLeech` BASE,
    // `InstantManaLeech` BASE, `InstantLifeLeech` BASE).
    (
        "leech_%_is_instant",
        &[
            (
                Quantity::Poe1(Poe1Quantity::InstantLeech(Pool::EnergyShield)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::InstantLeech(Pool::Mana)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::InstantLeech(Pool::Life)),
                Op::Base,
                1.0,
            ),
        ],
    ),
    // P6-10b part 9: "#% of Damage Leeched as Life" (`DamageLifeLeech` BASE).
    (
        "life_leech_from_any_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(Pool::Life, LeechSource::Any)),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% increased total Recovery per second from Life Leech" (`LifeLeechRate` INC).
    (
        "life_leech_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechRate(Pool::Life)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 9: "#% increased total Recovery per second from Life, Mana, or Energy Shield Leech" (`LifeLeechRate` INC, `ManaLeechRate` INC, `EnergyShieldLeechRate` INC).
    (
        "life_mana_es_leech_speed_+%",
        &[
            (
                Quantity::Poe1(Poe1Quantity::LeechRate(Pool::Life)),
                Op::Increased,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::LeechRate(Pool::Mana)),
                Op::Increased,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::LeechRate(Pool::EnergyShield)),
                Op::Increased,
                1.0,
            ),
        ],
    ),
    // P6-10b: "#% increased Life Recovery rate" (`LifeRecoveryRate` INC).
    (
        "life_recovery_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::Life)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "#% increased Life Regeneration rate" (`LifeRegen` INC).
    (
        "life_regeneration_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::Life)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "Regenerate #% of Life per second" (`LifeRegenPercent` BASE).
    (
        "life_regeneration_rate_per_minute_%",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPercent(Pool::Life)),
            Op::Base,
            1.0 / 60.0,
        )],
    ),
    (
        "lightning_and_chaos_damage_resistance_%",
        &[(LIGHTNING, Op::Base, 1.0), (CHAOS, Op::Base, 1.0)],
    ),
    // P6-10b part 15: "#% of Lightning Damage is taken from Mana before Life"
    // (`DamageTakenFromManaBeforeLife` BASE, or the type's).
    (
        "lightning_damage_%_taken_from_mana_before_life",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaBeforeLife(TakenOf::Type(
                DamageType::Lightning,
            ))),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Lightning Damage from Hits taken as Cold Damage" (`LightningDamageFromHitsTakenAsCold` BASE).
    (
        "lightning_damage_taken_%_as_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Lightning),
                DamageType::Cold,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Lightning Damage from Hits taken as Fire Damage" (`LightningDamageFromHitsTakenAsFire` BASE).
    (
        "lightning_damage_taken_%_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Lightning),
                DamageType::Fire,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Lightning Damage taken" (`LightningDamageTaken` INC).
    (
        "lightning_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Type(DamageType::Lightning),
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Lightning Damage taken as Fire Damage" (`LightningDamageTakenAsFire` BASE).
    (
        "lightning_hit_and_dot_damage_%_taken_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Lightning),
                DamageType::Fire,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // "Magic Utility Flasks applied to you have #% increased Effect": PoB's
    // `MagicUtilityFlaskEffect` INC (`ModParser.lua` 5180); `poe1::flasks`
    // reads it (P6-4b).
    (
        "magic_utility_flask_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::MagicUtilityFlaskEffect),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 9: "#% of Damage Leeched as Mana" (`DamageManaLeech` BASE).
    (
        "mana_leech_from_any_damage_permyriad",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageLeech(Pool::Mana, LeechSource::Any)),
            Op::Base,
            0.01,
        )],
    ),
    // P6-10b part 9: "#% increased total Recovery per second from Mana Leech" (`ManaLeechRate` INC).
    (
        "mana_leech_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechRate(Pool::Mana)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "#% increased Mana Recovery rate" (`ManaRecoveryRate` INC).
    (
        "mana_recovery_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::Mana)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "#% increased Mana Regeneration Rate" (`ManaRegen` INC).
    (
        "mana_regeneration_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::Regen(Pool::Mana)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b: "Regenerate #% of Mana per second" (`ManaRegenPercent` BASE).
    (
        "mana_regeneration_rate_per_minute_%",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPercent(Pool::Mana)),
            Op::Base,
            1.0 / 60.0,
        )],
    ),
    // P6-10b part 10: "Take #% more Damage" (`DamageTaken` MORE).
    (
        "marauder_hidden_ascendancy_damage_taken_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::All,
            )),
            Op::More,
            1.0,
        )],
    ),
    (
        "maximum_block_%",
        &[(Quantity::MaximumBlockChance, Op::Base, 1.0)],
    ),
    (
        "maximum_energy_shield_+%",
        &[(Quantity::EnergyShield, Op::Increased, 1.0)],
    ),
    // P6-10b part 7: "#% increased Maximum Recovery per Energy Shield Leech"
    // (`MaxEnergyShieldLeechInstance` INC; PoB's skill map reads the life and
    // energy shield stats as the rate, items' lines as the instance).
    (
        "maximum_energy_shield_leech_amount_per_leech_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechInstanceMax(Pool::EnergyShield)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 7: "#% increased Maximum total Energy Shield Recovery per second from Leech"
    // (`MaxEnergyShieldLeechRate` INC).
    (
        "maximum_energy_shield_leech_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::EnergyShield)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 7: "#% increased Maximum total Life, Mana and Energy Shield
    // Recovery per second from Leech" (the three `Max<Pool>LeechRate` INC).
    (
        "maximum_leech_rate_+%",
        &[
            (
                Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::Life)),
                Op::Increased,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::Mana)),
                Op::Increased,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::EnergyShield)),
                Op::Increased,
                1.0,
            ),
        ],
    ),
    ("maximum_life_+%", &[(Quantity::Life, Op::Increased, 1.0)]),
    // P6-10b part 7: "#% increased Maximum Recovery per Life Leech"
    // (`MaxLifeLeechInstance` INC; PoB's skill map reads the life and
    // energy shield stats as the rate, items' lines as the instance).
    (
        "maximum_life_leech_amount_per_leech_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechInstanceMax(Pool::Life)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 7: "#% increased Maximum total Life Recovery per second from Leech"
    // (`MaxLifeLeechRate` INC).
    (
        "maximum_life_leech_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::Life)),
            Op::Increased,
            1.0,
        )],
    ),
    ("maximum_mana_+%", &[(Quantity::Mana, Op::Increased, 1.0)]),
    // P6-10b part 7: "#% increased Maximum Recovery per Mana Leech"
    // (`MaxManaLeechInstance` INC; PoB's skill map reads the life and
    // energy shield stats as the rate, items' lines as the instance).
    (
        "maximum_mana_leech_amount_per_leech_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechInstanceMax(Pool::Mana)),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 7: "#% increased Maximum total Mana Recovery per second from Leech"
    // (`MaxManaLeechRate` INC).
    (
        "maximum_mana_leech_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::Mana)),
            Op::Increased,
            1.0,
        )],
    ),
    // P3-8n: "Allocates <notable>" (`GrantedPassive`, 5485): the node's
    // hash; `collect` adds the node.
    (
        "mod_granted_passive_hash",
        &[(Quantity::Poe1(Poe1Quantity::GrantedPassive), Op::Base, 1.0)],
    ),
    // P6-9b: "You can apply an additional Curse" (`SkillStatMap.lua` 1317:
    // `EnemyCurseLimit` BASE); `poe1::gems` reads it for the curse slots.
    (
        "number_of_additional_curses_allowed",
        &[(Quantity::Poe1(Poe1Quantity::CurseLimit), Op::Base, 1.0)],
    ),
    // P6-11n: "#% less Damage taken from Damage Over Time" (`DamageTakenOverTime`
    // MORE).
    (
        "passive_mastery_damage_taken_over_time_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::OverTime,
                TakenOf::All,
            )),
            Op::More,
            1.0,
        )],
    ),
    // P3-8n: "Enemies permanently take #% increased Damage for each second
    // they've ever been Chilled by you, up to a maximum of 10%" (3812;
    // `poe1::damage_taken`).
    (
        "permanent_damage_+%_per_second_of_chill",
        &[(
            Quantity::Poe1(Poe1Quantity::ChillSecondsDamageTaken),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "+# Physical Damage taken from Attack Hits" (`PhysicalDamageTakenFromAttacks` BASE).
    (
        "physical_attack_damage_taken_+",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Attack,
                TakenOf::Type(DamageType::Physical),
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 15: "#% of Physical Damage is taken from Mana before Life"
    // (`DamageTakenFromManaBeforeLife` BASE, or the type's).
    (
        "physical_damage_%_taken_from_mana_before_life",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaBeforeLife(TakenOf::Type(
                DamageType::Physical,
            ))),
            Op::Base,
            1.0,
        )],
    ),
    (
        "physical_damage_reduction_rating_+%",
        &[(Quantity::Armour, Op::Increased, 1.0)],
    ),
    // P6-10b part 12: "#% of Physical Damage from Hits taken as Chaos Damage" (`PhysicalDamageFromHitsTakenAsChaos` BASE).
    (
        "physical_damage_taken_%_as_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Physical),
                DamageType::Chaos,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Physical Damage from Hits taken as Cold Damage" (`PhysicalDamageFromHitsTakenAsCold` BASE).
    (
        "physical_damage_taken_%_as_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Physical),
                DamageType::Cold,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Physical Damage from Hits taken as Fire Damage" (`PhysicalDamageFromHitsTakenAsFire` BASE).
    (
        "physical_damage_taken_%_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Physical),
                DamageType::Fire,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Physical Damage from Hits taken as Lightning Damage" (`PhysicalDamageFromHitsTakenAsLightning` BASE).
    (
        "physical_damage_taken_%_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Physical),
                DamageType::Lightning,
                true,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "+# Physical Damage taken from Hits" (`PhysicalDamageTakenWhenHit` BASE).
    (
        "physical_damage_taken_+",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Hit,
                TakenOf::Type(DamageType::Physical),
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 10: "#% increased Physical Damage taken" (`PhysicalDamageTaken` INC).
    (
        "physical_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::Always,
                TakenOf::Type(DamageType::Physical),
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10b part 12: "#% of Physical Damage taken as Fire Damage" (`PhysicalDamageTakenAsFire` BASE).
    (
        "physical_hit_and_dot_damage_%_taken_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageTakenAs(
                TakenOf::Type(DamageType::Physical),
                DamageType::Fire,
                false,
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b part 12: "+# Physical Damage taken from Projectile Attacks" (`PhysicalDamageTakenFromProjectileAttacks` BASE).
    (
        "physical_ranged_attack_damage_taken_+",
        &[(
            Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                TakenWhen::ProjectileAttack,
                TakenOf::Type(DamageType::Physical),
            )),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b: Zealot's Oath (PoB's `ZealotsOath` flag, `CalcDefence.lua`
    // 1250): life's regeneration is energy shield's.
    (
        "regenerate_energy_shield_instead_of_life",
        &[(Quantity::Poe1(Poe1Quantity::ZealotsOath), Op::Flag, 1.0)],
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
    // P6-11k: "#% increased Defences from Equipped Shield" (`Defences` INC,
    // `SlotName` Weapon 2): each defence of the shield.
    (
        "shield_armour_+%",
        &[
            (
                Quantity::Poe1(Poe1Quantity::SlotIncreased(
                    ArmourSlot::Weapon2,
                    Defence::Armour,
                )),
                Op::Increased,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::SlotIncreased(
                    ArmourSlot::Weapon2,
                    Defence::Evasion,
                )),
                Op::Increased,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::SlotIncreased(
                    ArmourSlot::Weapon2,
                    Defence::EnergyShield,
                )),
                Op::Increased,
                1.0,
            ),
        ],
    ),
    // P6-11k: "#% increased Energy Shield from Equipped Shield"
    // (`EnergyShield` INC, `SlotName` Weapon 2).
    (
        "shield_maximum_energy_shield_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SlotIncreased(
                ArmourSlot::Weapon2,
                Defence::EnergyShield,
            )),
            Op::Increased,
            1.0,
        )],
    ),
    // Silver Flask's buff flag, "Onslaught" in PoB's base data (the line
    // PoB parses as the `Onslaught` condition, as the custom line); its
    // effect with flask effect is `poe1::flasks`' (P6-4b).
    (
        "silver_flask_display_onslaught",
        &[(Quantity::Poe1(Poe1Quantity::Onslaught), Op::Flag, 1.0)],
    ),
    // P6-11f: "+#% Chance to Block Spell Damage per 5% Chance to Block
    // Attack Damage" (`SpellBlockChance` BASE, `PerStat` `BlockChance`
    // `div = 5`); `phases` reads it.
    (
        "spell_block_%_per_5_attack_block",
        &[(Quantity::Poe1(Poe1Quantity::BlockPer5(true)), Op::Base, 1.0)],
    ),
    // Stibnite Flask's buff, "20% more Evasion Rating" in PoB's base data:
    // `Evasion` MORE (P6-4b).
    (
        "stibnite_flask_evasion_rating_+%_final",
        &[(Quantity::Evasion, Op::More, 1.0)],
    ),
    ("strength_+%", &[(STR, Op::Increased, 1.0)]),
    // P3-8n: Arcane Surge Support's cast speed (`ArcaneSurgeCastSpeed` MAX,
    // `Data/Skills/sup_int.lua` 232; `poe1::buffs`).
    (
        "support_arcane_surge_cast_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ArcaneSurgeCastSpeed),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b: Arcane Surge Support's mana regeneration (`ArcaneSurgeManaRegen`,
    // read as its largest by `poe1::buffs`).
    (
        "support_arcane_surge_mana_regeneration_rate_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ArcaneSurgeManaRegen),
            Op::Base,
            1.0,
        )],
    ),
    // P6-10b: Inspiration Support's "#% more Mana Cost" (`ManaCost` MORE,
    // its `statMap`).
    (
        "support_inspiration_mana_cost_+%_final",
        &[(Quantity::Poe1(Poe1Quantity::ManaCost), Op::More, 1.0)],
    ),
];

/// The Path of Exile stat-to-effect mapping: [`ROWS`]; every other stat
/// is `Unmapped`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Poe1Mapping;

/// Stats that reach no slice field (102): `(stat, why)`, sorted by stat ID
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
/// (shock and exposure but the rows of [`super::damage_taken`] and
/// [`super::exposure`] (P3-8k), chill, intimidate, consecrated ground's
/// effect, curses), charges' counts, rage, area, gem levels but the global ones of
/// [`super::gem_levels`] (P3-8i), minion modifiers but those of
/// [`super::minions`] (P3-8j, with the redirects to the player), and staff
/// or shield
/// conditioned block and damage. (P3-8n's fortification maximum and
/// `base_self_critical_strike_multiplier_-%` left this table in P6-10b
/// part 12: the enemy's hit reads both.) A stat that makes one of these rows matter is
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
    // PoB `SkillStatMap.lua` 588: `AreaOfEffect` MORE, which feeds only
    // radius outputs, as `base_skill_area_of_effect_+%` (P6-6a).
    ("active_skill_area_of_effect_+%_final", "area"),
    // A skill's own base radius (PoB `skill("radius")`): it sets the area
    // PoB shows, which no slice field reads (P3-9b). Increased area stays
    // unmapped, as above.
    ("active_skill_base_area_of_effect_radius", "skill-radius"),
    // PoB `SkillStatMap.lua` 570: `skill("radiusExtra")`, the area PoB
    // shows, as the skill's own base radius (P6-6a).
    ("active_skill_base_radius_+", "skill-radius"),
    (
        "active_skill_base_secondary_area_of_effect_radius",
        "skill-radius",
    ),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    (
        "active_skill_projectile_speed_+%_variation_final",
        "display",
    ),
    // How the game describes a skill's areas, its visuals, its targeting:
    // nothing PoB computes from (P3-10, Smite).
    (
        "active_skill_secondary_area_of_effect_description_mode",
        "display",
    ),
    // P3-8n: a chance to gain a frenzy charge on kill (Disciple of the
    // Slaughter): as the power charge's, PoB never reads it.
    ("add_frenzy_charge_on_kill_%_chance", "charge-gain-chance"),
    ("add_power_charge_on_kill_%_chance", "charge-gain-chance"),
    // "All Damage can Shock" (PoB `<Type>CanShock` flags), "Hits always
    // Shock" (`EnemyShockChance` BASE 100): PoB reads them only for the
    // shock chance and the shock it shows (`CalcOffence.lua` 4108, 5208
    // on), never for the enemy's shock in the hit, which only the config
    // and guaranteed shocks set (P3-8k, `poe1::damage_taken`).
    ("all_damage_can_shock", "ailment-chance"),
    // P6-7c3: PoB `SkillStatMap.lua` 1362, `PierceAllTargets`, read only
    // into `PierceCount` (`CalcOffence.lua` 1106): no slice field, as
    // `base_projectile_speed_+%`.
    ("always_pierce", "pierce"),
    ("always_shock", "ailment-chance"),
    // P6-11b: "#% increased Area of Effect while wielding a Bow" (PoB:
    // `AreaOfEffect` INC, `UsingBow`): area, as `base_skill_area_of_effect_+%`.
    ("area_of_effect_+%_while_wielding_bow", "area"),
    // P6-11b: "#% increased Area of Effect while wielding a Staff" (PoB:
    // `AreaOfEffect` INC, `UsingStaff`): area.
    ("area_of_effect_+%_while_wielding_staff", "area"),
    // P3-8n: "#% increased Area of Effect while you don't have
    // Convergence" (`AreaOfEffect` INC, not `Convergence`): area, as above.
    (
        "area_of_effect_+%_while_you_do_not_have_convergence",
        "area",
    ),
    ("avoid_ailments_%_from_crit", "ailment-on-self"),
    // P6-11f: "#% chance to Avoid Elemental Ailments" (`AvoidElementalAilments` BASE): no slice field.
    ("avoid_all_elemental_status_%", "ailment-on-self"),
    ("avoid_chained_projectile_%_chance", "avoidance"),
    ("avoid_interruption_while_casting_%", "interruption"),
    ("avoid_maim_%_chance", "ailment-on-self"),
    ("base_aura_area_of_effect_+%", "area"),
    ("base_avoid_freeze_%", "ailment-on-self"),
    // P6-11g: "#% chance to Avoid being Stunned" (`AvoidStun` BASE).
    ("base_avoid_stun_%", "stun"),
    ("base_chance_to_freeze_%", "freeze"),
    // P3-8n: a passive's "#% chance to Shock" (`EnemyShockChance` BASE): as
    // the skill's chance below, never the enemy's shock in the hit.
    ("base_chance_to_shock_%", "ailment-chance"),
    // A skill's chance to shock: PoB shocks the enemy only from its config
    // or a guaranteed source (`ShockBase`, `ShockOverride`,
    // `ShockMinimum`; `Modules/CalcPerform.lua`, the non-damaging
    // ailments), never from a chance (P3-10, Smite).
    ("base_chance_to_shock_%_from_skill", "ailment-chance"),
    // A skill marked a projectile skill (`base_is_projectile`): the skill's
    // own type (`Projectile`) says it; PoB maps no row for it (P6-1).
    ("base_is_projectile", "skill type marker"),
    // P6-11g: "#% increased Rarity of Items found" (`LootRarity` INC).
    ("base_item_found_rarity_+%", "item-rarity"),
    // P6-11g: "Gain # Life per Enemy Hit with Attacks" (`LifeOnHit` BASE): no recovery field.
    ("base_life_gain_per_target", "recovery"),
    // P6-11g: "Gain # Life per Enemy Killed" (`LifeOnKill` BASE).
    ("base_life_gained_on_enemy_death", "recovery"),
    // P6-11m: minion duration (`Duration` INC for minion skills): no slice
    // field reads a minion's duration.
    ("base_minion_duration_+%", "skill-duration"),
    // P3-8n: "Maximum # Summoned Golems" (`ActiveGolemLimit`): only golem
    // skills (no golem gem is modelled, `poe1::gems`) and modifiers per
    // summoned golem read it ([`NO_EFFECT_DEPENDS_ON`]).
    ("base_number_of_golems_allowed", "golems"),
    // P6-11m: "+# to maximum number of Skeletons" (`ActiveSkeletonLimit`).
    ("base_number_of_skeletons_allowed", "minion-count"),
    // P6-11m: "+# to maximum number of Raised Zombies" (`ActiveZombieLimit`): a count no slice field reads.
    ("base_number_of_zombies_allowed", "minion-count"),
    // P6-11f: "#% increased Projectile Speed" (`ProjectileSpeed` INC): no slice field.
    ("base_projectile_speed_+%", "projectile-speed"),
    // P3-8n: "You take #% reduced Extra Damage from Critical Strikes" on the
    // tree (`ReduceCritExtraDamage` BASE): the crits you take. A curse gem's
    // own copy is read by PoB's skill stat map as the enemy's
    // `SelfCritMultiplier`; no curse gem is modelled (`poe1::gems`), so it is
    // never ingested.
    // Area of effect (P3-8e): PoB's `AreaOfEffect` feeds only radius
    // outputs (`CalcOffence.lua` 345 to 455, 1370), none a slice field.
    ("base_skill_area_of_effect_+%", "area"),
    // A skill's effect duration (Smite's aura lasts this long after a hit;
    // PoB keeps the aura up).
    ("base_skill_effect_duration", "skill-duration"),
    ("base_smite_number_of_targets", "targeting"),
    ("base_stun_duration_+%", "stun"),
    ("base_stun_recovery_+%", "stun"),
    // P6-11f: "#% increased Block Recovery" (`BlockRecovery` INC): no slice field.
    ("block_recovery_+%", "block-recovery"),
    ("cannot_be_blinded", "blind-on-self"),
    // P6-11f: "Corrupted Blood cannot be inflicted on you" (`CorruptedBloodImmune`): no slice field.
    ("cannot_be_inflicted_by_corrupted_blood", "ailment-on-self"),
    ("cannot_be_poisoned_if_x_poisons_on_you", "ailment-on-self"),
    (
        "cannot_be_shocked_while_at_maximum_power_charges",
        "ailment-on-self",
    ),
    (
        "cannot_be_stunned_if_have_been_stunned_or_blocked_stunning_hit_in_past_2_seconds",
        "stun",
    ),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("cannot_cancel_skill_before_contact_point", "display"),
    (
        "cannot_gain_corrupted_blood_while_you_have_at_least_5_stacks",
        "ailment-on-self",
    ),
    ("cannot_have_life_leeched_from", "leech"),
    // P6-8a: read by `poe1::triggers` (Cast On Critical Strike's chance).
    ("cast_linked_spells_on_attack_crit_%", "trigger"),
    // P6-8a part 2: PoB's `chanceToTriggerOnMeleeKill`, which no module
    // reads (`Modules/`).
    ("cast_linked_spells_on_melee_kill_%", "trigger"),
    // P6-8a: Cast when Damage Taken's chance; PoB's stat map has no row.
    ("cast_on_damage_taken_%", "trigger"),
    // P6-8a: read by `poe1::triggers` (`triggeredByDamageTaken`).
    ("cast_on_damage_taken_threshold", "trigger"),
    // P6-8a: PoB's `triggeredByCoc` and `triggerOnCrit`, which
    // `poe1::triggers` reads from the support.
    ("cast_spell_on_linked_attack_crit", "trigger"),
    // P6-8a part 2: PoB's `triggeredByMeleeKill`, which `poe1::triggers`
    // decides.
    ("cast_spell_on_linked_melee_kill", "trigger"),
    // P6-8a part 2: PoB's `triggeredWhileChannelling`, which
    // `poe1::triggers` decides.
    ("cast_spell_while_linked_skill_channelling", "trigger"),
    // P6-8a part 2: read by `poe1::triggers` (Cast while Channelling's
    // interval, PoB's `triggerTime`).
    ("cast_while_channelling_time_ms", "trigger"),
    (
        "chance_to_create_consecrated_ground_on_melee_kill_%",
        "ground-creation-chance",
    ),
    ("chance_to_fortify_on_melee_hit_+%", "fortification"),
    // P6-11m: a chance to gain an endurance charge when hit (unparsed).
    ("chance_to_gain_endurance_charge_when_hit_%", "charge-gain"),
    (
        "chance_to_gain_power_charge_when_block_%",
        "charge-gain-chance",
    ),
    ("chill_effectiveness_on_self_+%", "ailment-on-self"),
    ("cold_ailment_duration_+%", "ailment-duration"),
    (
        "consecrated_ground_allies_recover_es_as_well_as_life_from_life_regeneration",
        "regeneration",
    ),
    ("console_skill_dont_chase", "display"),
    ("curse_effect_on_self_+%", "curse-on-self"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    (
        "damage_cannot_be_reflected_or_leech_if_used_by_other_object",
        "display",
    ),
    ("debuff_time_passed_+%", "ailment-on-self"),
    // P3-8n: Intensify's text; PoB maps no modifier.
    ("display_base_intensity_loss", "display"),
    (
        "elemental_ailment_duration_on_self_+%_while_holding_shield",
        "ailment-on-self",
    ),
    // P3-8n: "Your Elemental Resistances cannot be lowered by Curses": only
    // curses on you lower them, from the `playerCursedWith*` inputs, which
    // stay reasons for `curse_effect_on_self_+%`
    // ([`NO_EFFECT_DEPENDS_ON_INPUTS`]).
    (
        "elemental_resistance_cannot_be_lowered_by_curses",
        "curse-on-self",
    ),
    // P6-11f: "#% increased Endurance Charge Duration": the charges come from the config; no slice field.
    ("endurance_charge_duration_+%", "charge-duration"),
    // P6-11m: a chance to gain an endurance charge on kill (PoB: unparsed).
    ("endurance_charge_on_kill_%", "charge-gain"),
    // P6-11g: "#% chance to gain an Endurance Charge on Kill while holding a Shield": PoB does not parse it; the charges come from the config.
    (
        "endurance_charge_on_kill_percent_chance_while_holding_shield",
        "charge-gain-chance",
    ),
    // P6-11m: the enemy's own damage converted (`EnemyModifier`): what
    // you take; no slice field.
    (
        "enemies_ignited_by_you_have_physical_damage_%_converted_to_fire",
        "enemy-damage",
    ),
    (
        "enemies_that_hit_you_with_attack_recently_attack_speed_+%",
        "enemy-speed",
    ),
    (
        "enemy_life_regeneration_rate_+%_for_4_seconds_on_hit",
        "enemy-regeneration",
    ),
    // P6-11n: "#% increased Life Recovered" by flasks (`FlaskLifeRecovery`
    // INC): no field.
    ("flask_life_to_recover_+%", "flask"),
    // P6-11m: "#% increased Flask Recovery rate" (`FlaskRecoveryRate`).
    ("flask_recovery_speed_+%", "flask"),
    ("fortify_duration_+%", "fortification"),
    ("freeze_duration_+%", "ailment-duration"),
    // P6-11m: endurance charges per second if hit recently (unparsed).
    (
        "gain_endurance_charge_per_second_if_have_been_hit_recently",
        "charge-gain",
    ),
    // P6-11m: a chance to gain maximum endurance charges (PoB reads only a
    // partial `EnduranceChargesMax` with leftover text; charges come from
    // the config).
    (
        "gain_maximum_endurance_charges_on_endurance_charge_gained_%_chance",
        "charge-gain",
    ),
    (
        "gain_power_charge_for_each_second_channeling_spell",
        "charge-gain-chance",
    ),
    // P3-8n: Inspiration's charge gain; PoB maps no modifier (its charges are
    // the maximum unless the config overrides them, `poe1::stacks`).
    ("gain_righteous_charge_on_mana_spent_%", "charge-gain"),
    ("gloves_freeze_proliferation_radius", "freeze"),
    // P6-11g: "Ignites you inflict spread to other Enemies within # metre": PoB does not parse it.
    ("gloves_ignite_proliferation_radius", "area"),
    // P6-7b3b: "Golems are Immune to Elemental Damage": PoB's parser does
    // not read the line (no mod), so it reaches none of PoB's outputs; no
    // minion field reads damage taken.
    ("golem_immunity_to_elemental_damage", "minion-defence"),
    // P3-close: "#% increased Effect of Buffs granted by your Golems" left this
    // list for `poe1::skill_modifiers` (the golem gems are modelled).
    // P3-8n: the stats that add damage from a mana cost stay unmapped
    // ([`NO_EFFECT_DEPENDS_ON`]); the costs are rows since P6-10b.
    ("item_generation_can_have_multiple_crafted_mods", "crafting"),
    // P6-11g: Divine Shield (`Keystone` LIST): energy shield recovery and recoup, no slice field.
    // P6-11g: Mind Over Matter (`Keystone` LIST): damage taken from mana, no slice field.
    (
        "life_flasks_gain_X_charges_every_3_seconds_if_you_have_not_used_a_life_flask_recently",
        "flask",
    ),
    ("life_recovery_+%_from_flasks_while_on_low_life", "flask"),
    ("light_radius_+%", "light-radius"),
    ("lightning_ailment_duration_+%", "ailment-duration"),
    // "Gems Socketed always have the Quality bonus from Socket Colour":
    // its effect is the gems' quality, `poe1::sockets` (P3-12i).
    ("local_can_socket_gems_ignoring_colour", "sockets"),
    // P6-11m: a flask's amount recovered (`FlaskRecovery`): no slice field.
    ("local_flask_amount_to_recover_+%", "flask"),
    // P6-11m: flask charges on consuming a corpse (unparsed).
    (
        "local_flask_gain_X_charges_on_consuming_ignited_corpse",
        "flask",
    ),
    // P6-11m: Malediction on enemies ignited during a flask's effect
    // (`HasMalediction` on the enemy: its damage and regeneration).
    (
        "local_flask_ignited_enemies_during_effect_have_malediction",
        "enemy-debuff",
    ),
    // P6-11m: a flask's recovery rate (`FlaskRecoveryRate`).
    ("local_flask_recovery_speed_+%", "flask"),
    // P6-11m: a flask's taunt on use (unparsed).
    ("local_flask_taunt_enemies_on_use_radius", "flask"),
    // Its effect on the sockets' order is `poe1::sockets`' (a reason).
    ("local_has_X_abyss_sockets", "sockets"),
    // P6-11g: "Has # Sockets" (`SocketCount` BASE): the item's sockets come from its text.
    ("local_has_X_sockets", "sockets"),
    // P6-11m: a flask charge on crit (`FlaskChargeOnCritChance`).
    ("local_recharge_on_crit_%", "flask"),
    // P6-8a: the gems a trigger support supports; PoB has no row.
    (
        "local_support_gem_max_skill_level_requirement_to_support",
        "support-limit",
    ),
    (
        "lose_all_righteous_charges_on_mana_use_threshold",
        "charge-gain",
    ),
    // P6-11g: "Gain # Mana per Enemy Hit with Attacks" (`ManaOnHit` BASE).
    ("mana_gain_per_target", "recovery"),
    // P6-11g: "Gain # Mana per Enemy Hit with Attacks if you've used a Mana Flask in the past 10 seconds" (`ManaOnHit` BASE).
    (
        "mana_gained_on_attack_hit_if_used_mana_flask_in_past_10_seconds",
        "recovery",
    ),
    // P3-8n: "Melee Strikes target # additional nearby Enemies" (PoB's strike
    // targets, `CalcOffence.lua` 1147): not the hit.
    ("melee_attack_number_of_spirit_strikes", "targeting"),
    // P6-11f: "+# metres to Melee Strike Range" (`MeleeWeaponRangeMetre`, `UnarmedRangeMetre`): no slice field.
    ("melee_range_+", "range"),
    // P6-7c3: a monster skill's penalty against the player's minions (the
    // Flame Golem's Immolate): PoB's stat map has no row for it.
    (
        "monster_penalty_against_minions_damage_+%_final_vs_player_minions",
        "monster",
    ),
    // Bleeding on you while moving (`NoExtraBleedDamageWhileMoving`): only
    // PoB's damage over time estimate reads it, no field here.
    ("no_extra_bleed_damage_while_moving", "damage-taken"),
    (
        "pantheon_abberath_ignite_duration_on_self_+%_final",
        "ailment-on-self",
    ),
    (
        "pantheon_shakari_self_poison_duration_+%_final",
        "ailment-on-self",
    ),
    // Quartz Flask's "Phasing" (PoB's `Condition:Phasing`, `ModParser.lua`
    // 3972): only modifiers read it (unmapped, or an unsettled condition)
    // and The Hidden Blade's trigger (`CalcTriggers.lua` 927; triggers are
    // not modelled, a reason of their own) (P6-4b).
    ("phase_through_objects", "condition"),
    // P6-11m: reflected physical damage (unparsed).
    ("physical_damage_to_return_to_melee_attacker", "reflect"),
    // P6-11n: "#% increased Power Charge Duration"
    // (`PowerChargesDuration` INC): the charges come from the config.
    ("power_charge_duration_+%", "charge-duration"),
    // P6-11m: pierce (`PierceCount`), as `always_pierce`.
    ("projectile_base_number_of_targets_to_pierce", "pierce"),
    ("quality_display_active_skill_area_damage_is_gem", "display"),
    (
        "quality_display_active_skill_area_damage_quality_negated_from_gem",
        "display",
    ),
    ("rage_loss_delay_ms", "rage-duration"),
    // P6-11g: "Recover #% of Life on Kill" (`LifeOnKill` BASE per life).
    ("recover_%_maximum_life_on_kill", "recovery"),
    ("reflect_hexes_chance_%", "curse-on-self"),
    ("revive_golems_if_killed_by_enemies_ms", "minion-defence"),
    ("shocked_effect_on_self_+%", "ailment-on-self"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("show_number_of_projectiles", "display"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("skill_can_add_multiple_charges_per_action", "display"),
    // P6-8b: read by `poe1::combined` (PoB's `triggeredByMirageArcher`).
    ("skill_can_own_mirage_archers", "mirage"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("skill_cannot_be_stunned_before_contact_point", "display"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("skill_override_pvp_scaling_time_ms", "display"),
    ("smite_lightning_target_range", "targeting"),
    ("spell_area_of_effect_+%", "area"),
    // P6-7c3: a monster spell's range (the Flame Golem's Immolate): PoB's
    // stat map has no row for it.
    ("spell_maximum_action_distance_+%", "targeting"),
    // P6-8a: PoB's `skill("triggered")`, which `poe1::triggers` decides.
    ("spell_uncastable_if_triggerable", "trigger"),
    // P6-11f: "#% increased Stun Threshold" (`StunThreshold` INC): no slice field.
    ("stun_threshold_+%", "stun"),
    // P6-8b: read by `poe1::combined` (the mirage count).
    ("summon_mirage_archer_on_hit", "mirage"),
    // P3-8n: Arcane Surge Support's duration and threshold (PoB maps neither)
    // and its mana regeneration (`ArcaneSurgeManaRegen` MAX: mana
    // regeneration); its cast speed and spell damage are `poe1::buffs`.
    ("support_arcane_surge_base_duration_ms", "skill-duration"),
    (
        "support_arcane_surge_gain_buff_on_mana_use_threshold",
        "buff-gain",
    ),
    // P6-8b: the mirage's less attack speed, read by `poe1::combined` for
    // the mirage's copy alone.
    ("support_mirage_archer_attack_speed_+%_final", "mirage"),
    // P6-8b: the mirage's duration; no PoB module reads
    // `MirageArcherDuration`.
    ("support_mirage_archer_base_duration", "mirage"),
    // P6-8b: the mirage's less damage, read by `poe1::combined` for the
    // mirage's copy alone.
    ("support_mirage_archer_damage_+%_final", "mirage"),
    // P3-8n: Momentum's Swiftness and channelling (PoB maps the duration and
    // the channelling time to no modifier); its attack speed is
    // `poe1::stacks`, its speed per momentum lost a `MOVEMENT_REASONS` entry.
    ("support_momentum_base_buff_duration_ms", "skill-duration"),
    (
        "support_momentum_stack_while_channelling_base_ms",
        "channelling",
    ),
    // P3-8n: Intensify's area per intensity (`AreaOfEffect` MORE): area.
    (
        "support_spell_boost_area_of_effect_+%_final_per_charge",
        "area",
    ),
    // P6-8a part 2: Cast on Ward Break's chance; a global trigger's rate is
    // its cap (`CalcTriggers.lua` 793 to 804).
    ("trigger_on_ward_break_%_chance", "trigger"),
    // P6-8a: PvP scaling; PoB has no row.
    (
        "triggered_skill_uses_main_hand_or_averaged_attack_time_for_pvp_scaling",
        "pvp",
    ),
    ("unaffected_by_burning_ground", "ground-on-self"),
    // P6-11n: "Unaffected by Ignite" (`SelfIgniteEffect` MORE -100): ignite on
    // you, no field.
    ("unaffected_by_ignite", "ailment-on-self"),
    ("visual_hit_effect_elemental_is_holy", "display"),
    (
        "your_consecrated_ground_effect_lingers_for_ms_after_leaving_the_area",
        "ground-duration",
    ),
    // P6-11m: a mark's transfer on death (unparsed).
    (
        "your_marks_transfer_to_nearby_enemies_on_death_%_chance",
        "mark-transfer",
    ),
];

/// Stats that would make a [`NO_EFFECT`] row reach a slice field: `(stat,
/// row)`, sorted by stat. A row's `NoEffect` holds only while these stay
/// reasons (`unmapped-stat`), so the fields are `Partial` whenever one is
/// in the build; mapping one (even as `NoEffect`) must revisit the row (a
/// test checks each is `Unmapped` through every mapping).
///
/// The minion damage redirects were guards here until P3-8j, which maps
/// minion damage and the redirects PoB reads (`poe1::minions`; the ones it
/// does not map are `minions::DEPENDS_ON`).
///
/// Light radius (P3-8e) reaches a slice field only through
/// `LightRadiusAppliesToDamage` (`Damage` INC, `CalcOffence.lua` 746),
/// `LightRadiusAppliesToAccuracy` (`Accuracy` INC, 732), and Eclipse
/// Solaris's fire damage per 1% light radius (`ModParser.lua` 2436); their
/// stats are the guards. The game's other light radius stats (applying to
/// area of effect, scaling with energy shield, while phased) reach no slice
/// field and stay `Unmapped`.
pub const NO_EFFECT_DEPENDS_ON: &[(&str, &str)] = &[
    // P3-8n: damage from a mana cost (PoB `PercentStat` of `ManaCost`).
    // P3-8n: damage per summoned golem (`PerStat` of `ActiveGolemLimit`).
    (
        "damage_+%_per_active_golem",
        "base_number_of_golems_allowed",
    ),
    (
        "light_radius_additive_modifiers_apply_to_damage",
        "light_radius_+%",
    ),
    (
        "light_radius_increases_apply_to_accuracy",
        "light_radius_+%",
    ),
    (
        "maximum_added_fire_damage_to_attacks_per_1%_light_radius",
        "light_radius_+%",
    ),
    (
        "minimum_added_fire_damage_to_attacks_per_1%_light_radius",
        "light_radius_+%",
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

/// Stat ID prefixes with no effect of their own, with why: a cluster
/// jewel's lines (passive tree plan, T-i). They say what nodes the jewel
/// adds and what those nodes grant; they act only through the nodes
/// (`loom_tree::poe1::cluster`, `ResolvedPassives::cluster`), whose stats
/// the engine ingests when allocated and connected. A node the engine
/// cannot build stays a reason on the node.
pub const NO_EFFECT_PREFIXES: &[(&str, &str)] = &[
    (
        "local_affliction_jewel_",
        "cluster jewel: acts through its nodes",
    ),
    ("local_affliction_notable_", "cluster jewel: adds a notable"),
    ("local_jewel_expansion_", "cluster jewel: adds nodes"),
    // A gem's quality display stats (`quality_display_<skill>_is_gem`): text
    // on the gem only; PoB maps none of them (P6-1).
    ("quality_display_", "gem display text"),
    (
        "local_unique_jewel_grants_x_empty_passives",
        "cluster jewel: adds nodes that grant nothing",
    ),
];

/// Whether `stat` has no effect by a rule rather than a [`NO_EFFECT`] row:
/// a prefix of [`NO_EFFECT_PREFIXES`], or a skill's own stat with no
/// output ([`super::skill_no_output`]).
#[must_use]
pub fn no_effect_by_rule(stat: &str) -> bool {
    NO_EFFECT_PREFIXES.iter().any(|(p, _)| stat.starts_with(p))
        || super::skill_no_output::contains(stat)
}

/// Stats that change the enemy's hit in a way the engine does not model
/// (P6-10b parts 12 and 13), sorted: each maps to
/// [`Poe1Quantity::EnemyHitReason`], the reason of the hit's fields and
/// maximum hit alone. PoB's text: no extra damage from crits if crit
/// recently (a condition).
pub const ENEMY_HIT_REASONS: &[&str] =
    &["self_take_no_extra_damage_from_critical_strikes_if_have_been_crit_recently"];

/// Stats that change the pools maximum hit reads (P6-10b part 13), sorted:
/// each maps to [`Poe1Quantity::PoolReason`]. PoB's text: the keystone
/// Divine Shield, half of area damage taken. (Mind over Matter is a
/// keystone row since P6-10b part 15.)
pub const POOL_REASONS: &[&str] = &[
    "keystone_divine_shield",
    "take_half_area_damage_from_hit_%_chance",
];

/// Stats that change the effective health pool alone in a way the engine
/// does not model (P6-10b part 14), sorted: each maps to
/// [`Poe1Quantity::EhpReason`]. PoB's text: life and energy shield gained
/// on block and spell block (`LifeOnBlock`, `EnergyShieldOnBlock`,
/// `EnergyShieldOnSpellBlock`), lucky spell suppression; P6-11n: damage
/// taken recouped as mana (`ManaRecoup`).
pub const EHP_REASONS: &[&str] = &[
    "damage_taken_goes_to_mana_%",
    "energy_shield_%_of_armour_rating_gained_on_block",
    "life_gained_on_block",
    "recover_%_of_maximum_life_on_block",
    "recover_X_life_on_block",
    "recover_x_energy_shield_on_spell_block",
    "spell_suppression_chance_is_lucky",
];

/// Stats that change damage taken over time in a way the engine does not
/// model (P6-10b part 13), sorted: each maps to
/// [`Poe1Quantity::DotTakenReason`]. PoB's text: chaos damage taken over
/// time while in a caustic cloud (a condition).
pub const DOT_TAKEN_REASONS: &[&str] = &["chaos_damage_taken_over_time_+%_while_in_caustic_cloud"];

/// Stats that change the main skill's mana cost in a way the engine does
/// not model (P6-10b), sorted: each maps to [`Poe1Quantity::CostReason`],
/// the mana cost field's reason alone. PoB's text for each
/// (`ModParser.lua`): costs paid in life or energy shield instead
/// (`CostLifeInsteadOfMana`, `HybridManaAndLifeCost_Life`), socketed
/// attacks' total mana cost (`SocketedIn`), Manaweave's base cost from
/// unreserved mana, a flask's or full energy shield's condition, and
/// channelling or non-channelling skills' total (`SkillType` 48, no flag
/// here).
pub const COST_REASONS: &[&str] = &[
    "local_display_socketed_attacks_mana_cost_+",
    "local_display_spend_energy_shield_for_costs_before_mana_for_socketed_skills",
    "local_flask_skill_mana_cost_+%_during_flask_effect",
    "mana_cost_+%_while_on_full_energy_shield",
    "manaweave_cost_equals_%_unreserved_mana",
    "skill_mana_cost_+_with_channelling_skills",
    "skill_mana_cost_+_with_non_channelling_skills",
    "support_focus_channel_cost_+%_final_per_second_channelling_up_to_100%",
];

/// Stats that change movement speed in a way the engine does not model
/// (P6-10b), sorted: each maps to [`Poe1Quantity::MovementReason`], the
/// movement fields' reason alone. PoB's text for each (`ModParser.lua`):
/// on kill (`KilledRecently`), during a flask's effect (`UsingFlask`), on
/// burning ground, on low life, while ignited, and with no gems in the
/// boots, each a config condition the engine does not settle; per
/// summoned phantasm (`PerStat`) and per nearby enemy (the config's
/// count, limited); the armour movement penalties ignored;
/// movement speed that cannot go below its base; and Momentum's
/// Swiftness per stack removed.
pub const MOVEMENT_REASONS: &[&str] = &[
    "gain_movement_speed_+%_for_20_seconds_on_kill",
    "ignore_armour_movement_penalties",
    "movement_speed_+%_during_flask_effect",
    // Lunaris's soul: `Multiplier:NearbyEnemies` with a limit, a config
    // count the engine does not read.
    "movement_speed_+%_per_nearby_enemy",
    "movement_speed_+%_per_summoned_phantasm",
    "movement_speed_+%_while_on_burning_ground",
    "movement_speed_cannot_be_reduced_below_base",
    "movement_velocity_+%_when_on_low_life",
    "movement_velocity_+%_while_ignited",
    "movement_velocity_+%_while_no_gems_in_boots",
    // Momentum's Swiftness: `MovementSpeed` INC per the config's
    // `MomentumSwiftnessStacks` (`Multiplier:MomentumStacksRemoved`), a
    // count the engine does not read.
    "support_momentum_movement_speed_+%_per_stack_removed",
];

/// Stats that change a pool's regeneration in a way the engine does not
/// model (P6-10b), sorted: each maps to [`Poe1Quantity::RegenReason`], the
/// regeneration field's reason alone. PoB's text for each
/// (`ModParser.lua`): life per fortification stack, per uncapped fire
/// resistance (`PerStat`), per endurance charge, per level, per power
/// charge, per spell block chance, with exposure inflicted recently;
/// energy shield per nearby corpse.
pub const REGEN_REASONS: &[(&str, Pool)] = &[
    // Per fortification stack: fortification is named only
    // (`config::NAMED_ONLY`), so this stays a reason, now on the field.
    (
        "life_regeneration_per_minute_%_per_fortification",
        Pool::Life,
    ),
    (
        "life_regeneration_per_minute_per_1%_uncapped_fire_damage_resistance",
        Pool::Life,
    ),
    (
        "life_regeneration_rate_per_minute_%_per_endurance_charge",
        Pool::Life,
    ),
    ("life_regeneration_rate_per_minute_per_level", Pool::Life),
    (
        "mana_regeneration_rate_+%_per_1%_spell_block_chance",
        Pool::Mana,
    ),
    ("mana_regeneration_rate_+%_per_power_charge", Pool::Mana),
    (
        "mana_regeneration_rate_per_minute_%_if_inflicted_exposure_recently",
        Pool::Mana,
    ),
    // "For each nearby corpse, you and nearby Allies Regenerate #% of Energy
    // Shield per second" (PoB's `ExtraAura` per `NearbyCorpse`).
    (
        "necromancer_energy_shield_regeneration_rate_per_minute_%_for_you_and_allies_per_nearby_corpse",
        Pool::EnergyShield,
    ),
];

/// Stats that change the base damage reduction in a way the engine does not
/// model, sorted (P6-10b part 5): PoB's `PhysicalDamageReduction` BASE per
/// nearby enemy, per hit taken recently, and with only one enemy nearby
/// (`ModParser.lua`; multipliers and a condition the engine does not
/// settle). A field marker ([`field_marker`]) read by
/// [`super::reduction`] alone.
pub const REDUCTION_REASONS: &[&str] = &[
    "physical_damage_reduction_%_if_only_one_enemy_nearby",
    "physical_damage_reduction_%_per_hit_you_have_taken_recently",
    "physical_damage_reduction_%_per_nearby_enemy",
];

/// Stats that change a hit's rate or DPS in a way the engine does not model,
/// sorted (P6-8a): PoB's `showAverage` (`SkillStatMap.lua` 136), which
/// reports the average hit instead of DPS for a skill that is not
/// triggered; a trigger's rate turns it off (`CalcOffence.lua` 2227). A
/// field marker ([`field_marker`]) the hit reads onto its speed unless a
/// ported trigger sets the rate, not a reason on every field.
pub const HIT_REASONS: &[&str] = &["base_skill_show_average_damage_instead_of_dps"];

/// The field-scoped reason marker `stat` maps to, if any (P6-10b):
/// [`COST_REASONS`], [`MOVEMENT_REASONS`], or [`REGEN_REASONS`], each a
/// `Flag` only that field's phase reads.
#[must_use]
pub fn field_marker(stat: &str) -> Option<Poe1Quantity> {
    if let Ok(i) = ENEMY_HIT_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::EnemyHitReason(ENEMY_HIT_REASONS[i]));
    }
    if let Ok(i) = EHP_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::EhpReason(EHP_REASONS[i]));
    }
    if let Ok(i) = POOL_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::PoolReason(POOL_REASONS[i]));
    }
    if let Ok(i) = DOT_TAKEN_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::DotTakenReason(DOT_TAKEN_REASONS[i]));
    }
    if let Ok(i) = REDUCTION_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::ReductionReason(REDUCTION_REASONS[i]));
    }
    if let Ok(i) = HIT_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::HitReason(HIT_REASONS[i]));
    }
    if let Ok(i) = COST_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::CostReason(COST_REASONS[i]));
    }
    if let Ok(i) = MOVEMENT_REASONS.binary_search(&stat) {
        return Some(Poe1Quantity::MovementReason(MOVEMENT_REASONS[i]));
    }
    REGEN_REASONS
        .binary_search_by(|(id, _)| (*id).cmp(stat))
        .ok()
        .map(|i| Poe1Quantity::RegenReason(REGEN_REASONS[i].1, REGEN_REASONS[i].0))
}

impl StatMapping for Poe1Mapping {
    fn map(&self, stat: &StatId) -> StatMap {
        if let Some(q) = field_marker(stat.as_str()) {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(q),
                op: Op::Flag,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
        }
        if NO_EFFECT
            .binary_search_by(|(id, _)| (*id).cmp(stat.as_str()))
            .is_ok()
            || no_effect_by_rule(stat.as_str())
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
        if let Ok(i) = STATE_ROWS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str())) {
            return StatMap::Effects(
                STATE_ROWS[i]
                    .1
                    .iter()
                    .map(|&(quantity, op, all, state)| EffectTemplate {
                        quantity,
                        op,
                        scale: 1.0,
                        divisor: 1.0,
                        conditions: Conditions::none()
                            .with_all(all)
                            .with_state(ConditionKey::new(state), true),
                    })
                    .collect(),
            );
        }
        // Stacks and config multipliers (P3-8n): flags and `Base` values
        // only `poe1::stacks` reads.
        if let Some((kind, value)) = super::stacks::modifier(stat.as_str()) {
            let op = match value {
                super::stacks::StackValue::CanGain | super::stacks::StackValue::Intimidate => {
                    Op::Flag
                }
                _ => Op::Base,
            };
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::Stack(kind, value)),
                op,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
        }
        if let Some((kind, value)) = super::charges::modifier(stat.as_str()) {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::Charge(kind, value)),
                op: Op::Base,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
        }
        // Module tables of `(quantity, op, scale)` rows, each
        // unconditional: minion modifiers (P3-8j), exposure and the shock
        // effect and minimum (P3-8k), the other gems' reservation and aura
        // effect modifiers (P3-8l).
        if let Some(effects) = super::minions::effects(stat.as_str())
            .or_else(|| super::exposure::effects(stat.as_str()))
            .or_else(|| super::damage_taken::ailment_effects(stat.as_str()))
            .or_else(|| super::skill_modifiers::effects(stat.as_str()))
        {
            return StatMap::Effects(
                effects
                    .into_iter()
                    .map(|(quantity, op, scale)| EffectTemplate {
                        quantity,
                        op,
                        scale,
                        divisor: 1.0,
                        conditions: Conditions::none(),
                    })
                    .collect(),
            );
        }
        // Keystones that change the hit (P3-8m), some conditional.
        if let Some(effects) = super::keystones::effects(stat.as_str()) {
            return StatMap::Effects(effects);
        }
        if let Some(quantity) = super::gem_levels::quantity(stat.as_str()) {
            return StatMap::Effects(vec![EffectTemplate {
                quantity,
                op: Op::Base,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
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

    /// A minion modifier of the minion's own life or defences acts on the
    /// minion only (P6-7b3b): its unsettled value is the minion's reason
    /// ([`super::minion_actor`]), not the character's. Minion damage and
    /// speeds can reach the character through a redirect
    /// ([`super::minions`]), so they stay the character's.
    fn elsewhere(&self, quantity: Quantity) -> bool {
        match quantity {
            Quantity::Poe1(Poe1Quantity::Minion(v) | Poe1Quantity::GolemMinion(v)) => {
                !super::minions::MinionValue::REDIRECTED.contains(&v)
            }
            // A regeneration stat's unsettled value: the regeneration
            // fields' reason only (P6-10b, [`super::regen`]).
            Quantity::Poe1(
                Poe1Quantity::Regen(_)
                | Poe1Quantity::RegenPercent(_)
                | Poe1Quantity::RegenPercentPerMinute(_)
                | Poe1Quantity::RegenPerMinute(_)
                | Poe1Quantity::RecoveryRate(_)
                | Poe1Quantity::RegenReason(..)
                | Poe1Quantity::EnergyShieldRecharge
                | Poe1Quantity::EnergyShieldRechargeFaster
                | Poe1Quantity::MovementSpeed
                | Poe1Quantity::MovementReason(_)
                | Poe1Quantity::CostReason(_),
            ) => true,
            _ => false,
        }
    }

    /// The curated custom-line patterns (ADR-015 Amendment 2; their names
    /// in `loom_parser::import::poe1::custom::PATTERNS`), as PoB's parser reads
    /// them: each a character-wide effect from the line's source. A
    /// pattern this mapping does not know, or a value that is not a
    /// number, has no handler.
    fn typed(&self, resolution: &TypedResolution, source: &Source) -> TypedMap {
        let TypedResolution::CustomPattern { pattern, value } = resolution else {
            return TypedMap::Unhandled;
        };
        let Some(&(_, quantity, op)) = CUSTOM_PATTERNS
            .iter()
            .find(|(name, ..)| *name == pattern.as_str())
        else {
            return TypedMap::Unhandled;
        };
        let Ok(value) = value.parse::<f64>() else {
            return TypedMap::Unhandled;
        };
        TypedMap::Effects(vec![Effect {
            quantity,
            op,
            value,
            conditions: Conditions::none(),
            source: source.clone(),
        }])
    }
}

/// The curated custom-line patterns' effects, by name (PoB's mod for each:
/// `CritChance` BASE, `BlockChance` MORE, `Damage` MORE, `SpellBlockChance`
/// MORE, from its parse of the line; `docs/phase-3/custom-lines.md`).
pub const CUSTOM_PATTERNS: &[(&str, Quantity, Op)] = &[
    (
        "base-critical-strike-chance",
        Quantity::CritChance,
        Op::Base,
    ),
    ("more-block-chance", Quantity::BlockChance, Op::More),
    ("more-damage", Quantity::Damage, Op::More),
    (
        "more-spell-block-chance",
        Quantity::Poe1(Poe1Quantity::SpellBlockChance),
        Op::More,
    ),
];

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

    // An item line's unsettled value of a stat another actor or field
    // alone reads (P6-10b: regeneration) is that one's reason.
    fn elsewhere(&self, quantity: Quantity) -> bool {
        Poe1Mapping.elsewhere(quantity)
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
/// Accuracy Rating" as the weapon's own accuracy (PoB: `Accuracy` BASE with
/// the hand's `MainHandAttack` or `OffHandAttack` condition), which only that
/// hand's attack pass reads (Phase 6, dual wielding).
type WeaponRow = (&'static str, &'static [(Quantity, Op)]);

const fn weapon(value: WeaponValue) -> Quantity {
    Quantity::Poe1(Poe1Quantity::Weapon(value))
}

/// The local stats of weapons (`stat_descriptions.txt`, each with `local`
/// set in the stats table).
pub const WEAPON_ROWS: &[WeaponRow] = &[
    (
        "local_accuracy_rating",
        &[(weapon(WeaponValue::Accuracy), Op::Base)],
    ),
    (
        "local_attack_speed_+%",
        &[(weapon(WeaponValue::AttackSpeed), Op::Increased)],
    ),
    (
        "local_critical_strike_chance_+%",
        &[(weapon(WeaponValue::CritChance), Op::Increased)],
    ),
    // P3-8n: "Grants #% increased Elemental Damage per 2% Quality"
    // (`ElementalDamage` INC per `QualityOn{SlotName}`, `div` 2,
    // `ModParser.lua` 5759): `collect` scales it by the weapon's quality.
    (
        "local_elemental_damage_+%_per_2%_quality",
        &[(weapon(WeaponValue::ElementalDamagePerQuality), Op::Base)],
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
    // P3-8n: "Quality does not increase Physical Damage"
    // (`AlternateQualityWeapon`, 5753; `Classes/Item.lua` 2257).
    (
        "local_quality_does_not_increase_physical_damage",
        &[(weapon(WeaponValue::AlternateQuality), Op::Flag)],
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

    // An item line's unsettled value of a stat another actor or field
    // alone reads (P6-10b: regeneration) is that one's reason.
    fn elsewhere(&self, quantity: Quantity) -> bool {
        Poe1Mapping.elsewhere(quantity)
    }
}

/// The mapping of a weapon in the off hand's lines (Phase 6, dual
/// wielding): [`WeaponMapping`]'s, each weapon value on the off hand's
/// weapon ([`super::Hand::Off`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct OffHandWeaponMapping;

impl StatMapping for OffHandWeaponMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        match WeaponMapping.map(stat) {
            StatMap::Effects(effects) => StatMap::Effects(
                effects
                    .into_iter()
                    .map(|mut e| {
                        if let Quantity::Poe1(Poe1Quantity::Weapon(v)) = e.quantity {
                            e.quantity = super::Hand::Off.weapon(v);
                        }
                        e
                    })
                    .collect(),
            ),
            other => other,
        }
    }

    // An item line's unsettled value of a stat another actor or field
    // alone reads (P6-10b: regeneration) is that one's reason.
    fn elsewhere(&self, quantity: Quantity) -> bool {
        Poe1Mapping.elsewhere(quantity)
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
        // A binary search reads it (P6-10b).
        assert!(REGEN_REASONS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(MOVEMENT_REASONS.windows(2).all(|w| w[0] < w[1]));
        assert!(COST_REASONS.windows(2).all(|w| w[0] < w[1]));
        // Each marker stat maps to its own field's marker, and to nothing
        // else: the three tables are disjoint (P6-10b review).
        for stat in COST_REASONS {
            assert_eq!(
                field_marker(stat),
                Some(Poe1Quantity::CostReason(stat)),
                "{stat}"
            );
        }
        for stat in MOVEMENT_REASONS {
            assert_eq!(
                field_marker(stat),
                Some(Poe1Quantity::MovementReason(stat)),
                "{stat}"
            );
            assert!(!COST_REASONS.contains(stat), "{stat}");
        }
        for (stat, pool) in REGEN_REASONS {
            assert_eq!(
                field_marker(stat),
                Some(Poe1Quantity::RegenReason(*pool, stat)),
                "{stat}"
            );
            assert!(
                !COST_REASONS.contains(stat) && !MOVEMENT_REASONS.contains(stat),
                "{stat}"
            );
        }
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

    /// The weapon-type rows (P6-11b, P6-11c): each with PoB's weapon
    /// `ModFlag`s and keyword flags as `Data/ModCache.lua` gives them.
    #[test]
    fn weapon_type_stats_should_map_as_pob_reads_them() {
        let all = |f: Flags| Conditions::none().with_all(f);
        let inc = |q: Quantity, f: Flags| vec![(q, Op::Increased, 1.0, all(f))];
        let hit_or_ailment = |f: Flags| {
            vec![(
                Quantity::Damage,
                Op::Increased,
                1.0,
                Conditions::none()
                    .with_all(f)
                    .with_any(Flags::HIT.union(Flags::AILMENT)),
            )]
        };
        // `flags=131076`: Bow, Hit; `131077`: with Attack.
        assert_eq!(
            hit("bow_damage_+%"),
            inc(Quantity::Damage, Flags::BOW | Flags::HIT)
        );
        assert_eq!(
            hit("bow_attack_speed_+%"),
            inc(Quantity::Speed, Flags::BOW | Flags::HIT | Flags::ATTACK)
        );
        // `keywordFlags=1024` (Bow), with `flags=8` (Dot).
        assert_eq!(
            hit("damage_over_time_+%_with_bow_skills"),
            inc(Quantity::Damage, Flags::DOT | Flags::KEYWORD_BOW)
        );
        // `flags=603979781`: Weapon2H, WeaponMelee, Hit, Attack.
        assert_eq!(
            hit("two_handed_melee_attack_speed_+%"),
            inc(
                Quantity::Speed,
                Flags::WEAPON_2H | Flags::WEAPON_MELEE | Flags::HIT | Flags::ATTACK
            )
        );
        // `flags=67110912`: WeaponMelee, Ailment.
        assert_eq!(
            hit("melee_weapon_ailment_damage_+%"),
            inc(Quantity::Damage, Flags::WEAPON_MELEE | Flags::AILMENT)
        );
        // `flags=8388608, keywordFlags=786432`: Wand; Hit or Ailment.
        assert_eq!(
            hit("wand_hit_and_ailment_damage_+%"),
            hit_or_ailment(Flags::WAND)
        );
        assert_eq!(
            hit("wand_damage_+%"),
            inc(Quantity::Damage, Flags::WAND | Flags::HIT)
        );
        // The staff lines on the weapon flag (P6-11c review).
        assert_eq!(
            hit("physical_staff_damage_+%"),
            inc(
                Quantity::Damage,
                Flags::PHYSICAL | Flags::STAFF | Flags::HIT
            )
        );
        assert_eq!(
            hit("staff_ailment_damage_+%"),
            inc(Quantity::Damage, Flags::STAFF | Flags::AILMENT)
        );
        // `flags=8192`: Weapon.
        let pen = |t| (Quantity::Penetration(t), Op::Base, 1.0, all(Flags::WEAPON));
        assert_eq!(
            hit("reduce_enemy_fire_resistance_with_weapons_%"),
            [pen(FIRE_T)]
        );
        assert_eq!(
            hit("reduce_enemy_elemental_resistance_with_weapons_%"),
            [pen(FIRE_T), pen(COLD_T), pen(LIGHTNING_T)]
        );
        // Added damage "with Wand Attacks": Wand, Hit.
        assert_eq!(
            hit("attack_minimum_added_lightning_damage_with_wand"),
            [(
                added(LIGHTNING_T, MIN),
                Op::Base,
                1.0,
                all(Flags::WAND | Flags::HIT)
            )]
        );
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
                    .with_all(Flags::KEYWORD_ATTACK)
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
        // P6-2b: its ignite chance is read by the ignite.
        assert!(matches!(
            Poe1Mapping.map(&StatId::new("chance_to_freeze_shock_ignite_%")),
            StatMap::Effects(_)
        ));
        // P3-8j: minion damage is a minion modifier (`poe1::minions`).
        assert!(matches!(
            Poe1Mapping.map(&StatId::new("minion_damage_+%")),
            StatMap::Effects(_)
        ));
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

    /// P3-8h: every "with <type> Skills" stat needs the skill's keyword;
    /// "<type> Damage with Attack (Spell) Skills" the damage type and the
    /// attack (spell) flag.
    #[test]
    fn skill_keyword_stats_should_map_as_pob_reads_them() {
        let all = |f| Conditions::none().with_all(f);
        let types = [
            ("chaos", CHAOS_T),
            ("cold", COLD_T),
            ("fire", FIRE_T),
            ("lightning", LIGHTNING_T),
        ];
        for (name, t) in types {
            let k = Flags::skill_keyword(t);
            assert_eq!(
                hit(&format!("cast_speed_for_{name}_skills_+%")),
                [(Quantity::Speed, Op::Increased, 1.0, all(Flags::CAST | k))]
            );
            assert_eq!(
                hit(&format!("{name}_critical_strike_chance_+%")),
                [(Quantity::CritChance, Op::Increased, 1.0, all(k))]
            );
            assert_eq!(
                hit(&format!("{name}_critical_strike_multiplier_+")),
                [(Quantity::CritMultiplier, Op::Base, 1.0, all(k))]
            );
        }
        assert_eq!(
            hit("cast_speed_for_elemental_skills_+%"),
            [(
                Quantity::Speed,
                Op::Increased,
                1.0,
                all(Flags::CAST).with_any(Flags::SKILL_ELEMENTAL)
            )]
        );
        for (name, t) in types.into_iter().chain([("physical", PHYS)]) {
            for (kind, flag) in [
                ("attack", Flags::KEYWORD_ATTACK),
                ("spell", Flags::KEYWORD_SPELL),
            ] {
                assert_eq!(
                    hit(&format!("{name}_damage_with_{kind}_skills_+%")),
                    [(
                        Quantity::Damage,
                        Op::Increased,
                        1.0,
                        all(flag | Flags::damage(t))
                    )],
                    "{name} {kind}"
                );
            }
        }
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
                all(Flags::KEYWORD_ATTACK).with_any(Flags::ELEMENTAL)
            )]
        );
        assert_eq!(
            hit("area_damage_+%"),
            [(Quantity::Damage, Op::Increased, 1.0, all(Flags::AREA))]
        );
        assert_eq!(
            hit("base_spell_critical_strike_multiplier_+"),
            [(Quantity::CritMultiplier, Op::Base, 1.0, all(Flags::SPELL))]
        );
        assert_eq!(
            hit("melee_physical_damage_+%"),
            [(
                Quantity::Damage,
                Op::Increased,
                1.0,
                all(Flags::MELEE | Flags::PHYSICAL)
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
            [(weapon(WeaponValue::Accuracy), Op::Base, 1.0)]
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

    /// The condition rows (P3-8f) are sorted, disjoint from the other
    /// tables, and read only conditions `states` settles.
    #[test]
    fn state_rows_should_read_settled_conditions_only() {
        assert!(STATE_ROWS.windows(2).all(|w| w[0].0 < w[1].0));
        for (id, rows) in STATE_ROWS {
            assert!(!HIT_ROWS.iter().any(|r| r.0 == *id), "{id}");
            assert!(!ROWS.iter().any(|r| r.0 == *id), "{id}");
            assert!(!NO_EFFECT.iter().any(|r| r.0 == *id), "{id}");
            for (_, _, _, state) in *rows {
                assert!(
                    [
                        states::USING_STAFF,
                        states::USING_SHIELD,
                        states::USING_WAND,
                        states::NO_LIFE_MODIFIERS_ON_BODY_ARMOUR,
                        states::DUAL_WIELDING,
                        conditions::STATIONARY,
                        conditions::ENEMY_RARE_OR_UNIQUE_NEARBY,
                        states::SIX_LIFE_MASTERIES,
                        // P6-8a part 2: settled by the spell path from the
                        // trigger before the damage.
                        super::super::triggers::TRIGGERED_DAMAGE,
                        // P6-10b part 13: the negation of a check's.
                        conditions::NOT_HIT_RECENTLY,
                    ]
                    .contains(state)
                        || conditions::CHECKS.iter().any(|(_, c)| c == state)
                        // P6-11o: an aura affecting the character.
                        || conditions::AFFECTED_BY.iter().any(|(_, c)| c == state),
                    "{id}: {state}"
                );
            }
        }
        // Each row as PoB reads its text (the table's docs).
        let staff = |q: Quantity, op: Op, all: Flags| (q, op, all, states::USING_STAFF);
        let shield = |q: Quantity, op: Op, all: Flags| (q, op, all, states::USING_SHIELD);
        let config = |q: Quantity, op: Op, all: Flags, c: &'static str| (q, op, all, c);
        let expected = [
            // P6-11l: more life with at least 6 Life Masteries.
            (
                "life_mastery_count_maximum_life_+%_final",
                vec![(Quantity::Life, Op::More, NONE, states::SIX_LIFE_MASTERIES)],
            ),
            // P6-10b: energy shield regeneration near a rare or unique enemy.
            (
                "energy_shield_regeneration_rate_per_minute_if_rare_or_unique_enemy_nearby",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::RegenPerMinute(Pool::EnergyShield)),
                    Op::Base,
                    NONE,
                    conditions::ENEMY_RARE_OR_UNIQUE_NEARBY,
                )],
            ),
            // P6-10b: the Soul of Arakaali's recovery rate.
            (
                "life_and_energy_shield_recovery_rate_+%_if_stopped_taking_damage_over_time_recently",
                vec![
                    config(
                        Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::Life)),
                        Op::Increased,
                        NONE,
                        conditions::STOPPED_TAKING_DOT_RECENTLY,
                    ),
                    config(
                        Quantity::Poe1(Poe1Quantity::RecoveryRate(Pool::EnergyShield)),
                        Op::Increased,
                        NONE,
                        conditions::STOPPED_TAKING_DOT_RECENTLY,
                    ),
                ],
            ),
            // P6-10b: the Soul of Shakari's regeneration while stationary.
            (
                "life_regeneration_rate_per_minute_%_while_stationary",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(Pool::Life)),
                    Op::Base,
                    NONE,
                    conditions::STATIONARY,
                )],
            ),
            // P6-11g: "+# Armour if you've Blocked Recently" (PoB's
            // `Armour` BASE, `Condition` `BlockedRecently`).
            (
                "X_armour_if_you_have_blocked_recently",
                vec![config(
                    Quantity::Armour,
                    Op::Base,
                    NONE,
                    conditions::BLOCKED_RECENTLY,
                )],
            ),
            (
                "critical_strike_chance_against_enemies_on_full_life_+%",
                vec![config(
                    Quantity::CritChance,
                    Op::Increased,
                    NONE,
                    conditions::ENEMY_FULL_LIFE,
                )],
            ),
            (
                "critical_strike_multiplier_+_vs_unique_enemies",
                vec![config(
                    Quantity::CritMultiplier,
                    Op::Base,
                    NONE,
                    conditions::ENEMY_RARE_OR_UNIQUE,
                )],
            ),
            (
                "damage_+%_if_enemy_shattered_recently",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    NONE,
                    conditions::SHATTERED_ENEMY_RECENTLY,
                )],
            ),
            (
                "damage_+%_if_you_have_shocked_recently",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    NONE,
                    conditions::SHOCKED_ENEMY_RECENTLY,
                )],
            ),
            (
                "defences_+%_while_wielding_staff",
                vec![
                    staff(Quantity::EnergyShield, Op::Increased, NONE),
                    staff(Quantity::Armour, Op::Increased, NONE),
                    staff(Quantity::Evasion, Op::Increased, NONE),
                ],
            ),
            (
                "global_critical_strike_chance_+%_while_holding_staff",
                vec![staff(Quantity::CritChance, Op::Increased, NONE)],
            ),
            (
                "global_critical_strike_multiplier_+_while_holding_staff",
                vec![staff(Quantity::CritMultiplier, Op::Base, NONE)],
            ),
            (
                "maximum_life_+%_if_no_life_tags_on_body_armour",
                vec![(
                    Quantity::Life,
                    Op::Increased,
                    NONE,
                    states::NO_LIFE_MODIFIERS_ON_BODY_ARMOUR,
                )],
            ),
            (
                "melee_damage_+%_at_close_range",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::MELEE.union(Flags::HIT),
                    conditions::AT_CLOSE_RANGE,
                )],
            ),
            (
                "shield_spell_block_%",
                vec![shield(SPELL_BLOCK, Op::Base, NONE)],
            ),
            (
                "spell_block_with_staff_%",
                vec![staff(SPELL_BLOCK, Op::Base, NONE)],
            ),
            (
                "spell_damage_+%_while_holding_shield",
                vec![shield(Quantity::Damage, Op::Increased, Flags::SPELL)],
            ),
            (
                "spell_staff_damage_+%",
                vec![staff(Quantity::Damage, Op::Increased, Flags::SPELL)],
            ),
            ("staff_block_%", vec![staff(BLOCK, Op::Base, NONE)]),
            (
                "support_arcane_surge_spell_damage_+%_final_while_you_have_arcane_surge",
                vec![config(
                    Quantity::Damage,
                    Op::More,
                    Flags::SPELL,
                    conditions::ARCANE_SURGE,
                )],
            ),
        ];
        // Close-out: the crit-recently cast speed, first by stat ID.
        let mut expected = expected.to_vec();
        expected.insert(
            0,
            (
                "cast_speed_+%_if_have_crit_recently",
                vec![config(
                    Quantity::Speed,
                    Op::Increased,
                    Flags::CAST,
                    conditions::CRIT_RECENTLY,
                )],
            ),
        );
        // P6-11a: the shield and dual wielding rows.
        let dual = |q: Quantity, op: Op, all: Flags| (q, op, all, states::DUAL_WIELDING);
        expected.extend([
            // P6-11b: the weapon-condition rows.
            (
                "additional_staff_block_%",
                vec![staff(BLOCK, Op::Base, NONE)],
            ),
            (
                "attack_speed_while_dual_wielding_+%",
                vec![dual(Quantity::Speed, Op::Increased, Flags::ATTACK)],
            ),
            (
                "damage_+%_while_wielding_wand",
                vec![(Quantity::Damage, Op::Increased, NONE, states::USING_WAND)],
            ),
            (
                "attack_damage_+%_while_holding_a_shield",
                vec![shield(Quantity::Damage, Op::Increased, Flags::ATTACK)],
            ),
            (
                "attack_skills_damage_+%_while_holding_shield",
                vec![shield(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::KEYWORD_ATTACK,
                )],
            ),
            (
                "attack_speed_+%_while_holding_shield",
                vec![shield(Quantity::Speed, Op::Increased, Flags::ATTACK)],
            ),
            (
                "attack_ailment_damage_+%_while_holding_shield",
                vec![shield(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::AILMENT.union(Flags::KEYWORD_ATTACK),
                )],
            ),
            (
                "dual_wield_or_shield_block_%",
                vec![dual(BLOCK, Op::Base, NONE), shield(BLOCK, Op::Base, NONE)],
            ),
            (
                "physical_attack_damage_+%_while_holding_a_shield",
                vec![shield(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::ATTACK.union(Flags::PHYSICAL),
                )],
            ),
            (
                "resist_all_elements_+%_while_holding_shield",
                vec![
                    shield(FIRE, Op::Base, NONE),
                    shield(COLD, Op::Base, NONE),
                    shield(LIGHTNING, Op::Base, NONE),
                ],
            ),
            ("shield_block_%", vec![shield(BLOCK, Op::Base, NONE)]),
            (
                "spell_block_chance_%_while_holding_staff_or_shield",
                vec![
                    staff(SPELL_BLOCK, Op::Base, NONE),
                    shield(SPELL_BLOCK, Op::Base, NONE),
                ],
            ),
            // P6-10b part 5: the stationary physical damage reduction.
            (
                "while_stationary_gain_additional_physical_damage_reduction_%",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::DamageReduction(DamageType::Physical)),
                    Op::Base,
                    NONE,
                    conditions::STATIONARY,
                )],
            ),
            // P6-10b part 13: damage taken if hit recently or not, and
            // over time while moving.
            (
                "elemental_damage_taken_+%_if_been_hit_recently",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                        TakenWhen::Always,
                        TakenOf::Elemental,
                    )),
                    Op::Increased,
                    NONE,
                    conditions::BEEN_HIT_RECENTLY,
                )],
            ),
            (
                "elemental_damage_taken_+%_if_not_hit_recently",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                        TakenWhen::Always,
                        TakenOf::Elemental,
                    )),
                    Op::Increased,
                    NONE,
                    conditions::NOT_HIT_RECENTLY,
                )],
            ),
            (
                "physical_damage_over_time_taken_+%_while_moving",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                        TakenWhen::OverTime,
                        TakenOf::Type(DamageType::Physical),
                    )),
                    Op::Increased,
                    NONE,
                    conditions::MOVING,
                )],
            ),
            // P6-11o: lines while an aura affects the character.
            (
                "armour_+_while_affected_by_determination",
                vec![config(
                    Quantity::Armour,
                    Op::Base,
                    NONE,
                    "AffectedByDetermination",
                )],
            ),
            (
                "attack_damage_+%_while_affected_by_precision",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::ATTACK,
                    "AffectedByPrecision",
                )],
            ),
            (
                "attack_speed_+%_while_affected_by_precision",
                vec![config(
                    Quantity::Speed,
                    Op::Increased,
                    Flags::ATTACK,
                    "AffectedByPrecision",
                )],
            ),
            (
                "critical_strike_chance_+%_while_affected_by_wrath",
                vec![config(
                    Quantity::CritChance,
                    Op::Increased,
                    NONE,
                    "AffectedByWrath",
                )],
            ),
            (
                "critical_strike_multiplier_+_while_affected_by_anger",
                vec![config(
                    Quantity::CritMultiplier,
                    Op::Base,
                    NONE,
                    "AffectedByAnger",
                )],
            ),
            (
                "critical_strike_multiplier_+_while_affected_by_precision",
                vec![config(
                    Quantity::CritMultiplier,
                    Op::Base,
                    NONE,
                    "AffectedByPrecision",
                )],
            ),
            (
                "maximum_added_cold_damage_while_affected_by_hatred",
                vec![config(
                    Quantity::AddedDamage(DamageType::Cold, Bound::Max),
                    Op::Base,
                    NONE,
                    "AffectedByHatred",
                )],
            ),
            (
                "minimum_added_cold_damage_while_affected_by_hatred",
                vec![config(
                    Quantity::AddedDamage(DamageType::Cold, Bound::Min),
                    Op::Base,
                    NONE,
                    "AffectedByHatred",
                )],
            ),
            (
                "pride_physical_damage_+%",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::PHYSICAL,
                    "AffectedByPride",
                )],
            ),
            (
                "reduce_enemy_cold_resistance_%_while_affected_by_hatred",
                vec![config(
                    Quantity::Penetration(DamageType::Cold),
                    Op::Base,
                    NONE,
                    "AffectedByHatred",
                )],
            ),
            (
                "reduce_enemy_lightning_resistance_%_while_affected_by_wrath",
                vec![config(
                    Quantity::Penetration(DamageType::Lightning),
                    Op::Base,
                    NONE,
                    "AffectedByWrath",
                )],
            ),
            (
                "spell_suppression_chance_%_while_affected_by_grace",
                vec![config(SUPPRESSION, Op::Base, NONE, "AffectedByGrace")],
            ),
            // P6-11p: Onslaught on kill.
            (
                "chance_to_gain_onslaught_on_kill_%",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::Onslaught),
                    Op::Flag,
                    NONE,
                    conditions::KILLED_RECENTLY,
                )],
            ),
            (
                "onslaught_time_granted_on_kill_ms",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::Onslaught),
                    Op::Flag,
                    NONE,
                    conditions::KILLED_RECENTLY,
                )],
            ),
            // P6-10b part 8: Lifetap's more damage while buffed.
            (
                "support_lifetap_damage_+%_final_while_buffed",
                vec![config(
                    Quantity::Damage,
                    Op::More,
                    NONE,
                    conditions::LIFETAP,
                )],
            ),
            // P6-8a part 2: Cast while Channelling's triggered damage.
            (
                "support_cast_while_channelling_triggered_skill_damage_+%_final",
                vec![config(
                    Quantity::Damage,
                    Op::More,
                    NONE,
                    super::super::triggers::TRIGGERED_DAMAGE,
                )],
            ),
        ]);
        expected.sort_by(|a, b| a.0.cmp(b.0));
        assert_eq!(STATE_ROWS.len(), expected.len());
        for ((id, rows), (want_id, want)) in STATE_ROWS.iter().zip(&expected) {
            assert_eq!(id, want_id);
            assert_eq!(rows.to_vec(), *want, "{id}");
        }
        let StatMap::Effects(t) = Poe1Mapping.map(&StatId::new("spell_staff_damage_+%")) else {
            panic!("spell_staff_damage_+%");
        };
        assert_eq!(
            t[0].conditions,
            Conditions::none()
                .with_all(Flags::SPELL)
                .with_state(ConditionKey::new(states::USING_STAFF), true)
        );
    }

    /// P6-6a review: "always hits" acts through a gem only; every stat of
    /// `skill_no_output` is `NoEffect` through the game's and the gem's
    /// mappings; fire and cold skill conversion reach the skill.
    #[test]
    fn skill_stats_batch_one_should_map_as_pob_reads_them() {
        let id = StatId::new;
        match SkillMapping.map(&id(ALWAYS_HIT_STAT)) {
            StatMap::Effects(e) => {
                assert_eq!(e.len(), 1);
                assert_eq!(e[0].quantity, Quantity::Poe1(Poe1Quantity::CannotBeEvaded));
                assert_eq!(e[0].op, Op::Flag);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(Poe1Mapping.map(&id(ALWAYS_HIT_STAT)), StatMap::Unmapped);
        assert_eq!(WeaponMapping.map(&id(ALWAYS_HIT_STAT)), StatMap::Unmapped);
        for s in crate::poe1::skill_no_output::STATS {
            assert_eq!(Poe1Mapping.map(&id(s)), StatMap::NoEffect, "{s}");
            assert_eq!(SkillMapping.map(&id(s)), StatMap::NoEffect, "{s}");
        }
        for (s, t) in [
            (
                "skill_physical_damage_%_to_convert_to_fire",
                DamageType::Fire,
            ),
            (
                "skill_physical_damage_%_to_convert_to_cold",
                DamageType::Cold,
            ),
        ] {
            let StatMap::Effects(e) = SkillMapping.map(&id(s)) else {
                panic!("{s}");
            };
            assert_eq!(
                e[0].quantity,
                Quantity::Poe1(Poe1Quantity::SkillConversion(DamageType::Physical, t))
            );
        }
    }
}

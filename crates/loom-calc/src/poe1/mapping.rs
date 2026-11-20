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
    // P6-11ca: "Attacks have +#% to Critical Strike Chance" (`CritChance`
    // BASE, `ModFlag.Attack`; the stat in hundredths, `divide_by_one_hundred`).
    (
        "attack_additional_critical_strike_chance_permyriad",
        &[(Quantity::CritChance, Op::Base, 100.0, Flags::ATTACK, NONE)],
    ),
    // P6-11dc: "#% increased Damage with Ailments from Attack Skills"
    // (`Damage` INC with `ModFlag.Ailment` and `KeywordFlag.Attack`).
    (
        "attack_ailment_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::AILMENT.union(Flags::KEYWORD_ATTACK),
            NONE,
        )],
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
    // P6-11cw: "#% increased Attack Damage per 450 Armour" (`Damage` INC,
    // `ModFlag.Attack`, `PerStat` `Armour` `div` 450): the hit's computed
    // term (`hit::DAMAGE_PER_STAT`).
    (
        "attack_damage_+%_per_450_physical_damage_reduction_rating",
        &[(
            Quantity::Poe1(Poe1Quantity::AttackDamagePer450Armour),
            Op::Base,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
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
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_chaos_damage_with_bows",
        &[(
            added(CHAOS_T, MAX),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_chaos_damage_with_claws",
        &[(
            added(CHAOS_T, MAX),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_chaos_damage_with_daggers",
        &[(
            added(CHAOS_T, MAX),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_maximum_added_cold_damage",
        &[(added(COLD_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_axes",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_bows",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_claws",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_daggers",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_maces",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_staves",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Sword Attacks": BASE,
    // `ModFlag.Sword` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_swords",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Wand Attacks": BASE,
    // `ModFlag.Wand` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_cold_damage_with_wand",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_maximum_added_fire_damage",
        &[(added(FIRE_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_fire_damage_with_axes",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_fire_damage_with_bow",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_fire_damage_with_claws",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_fire_damage_with_daggers",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_fire_damage_with_maces",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_fire_damage_with_staves",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
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
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_axes",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_bows",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_claws",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_daggers",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_maces",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_staves",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Sword Attacks": BASE,
    // `ModFlag.Sword` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_lightning_damage_with_swords",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
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
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_axes",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_bow",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_claws",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_daggers",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_maces",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_staves",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Sword Attacks": BASE,
    // `ModFlag.Sword` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_swords",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Wand Attacks": BASE,
    // `ModFlag.Wand` and `Hit` (PoB's "with ... attacks").
    (
        "attack_maximum_added_physical_damage_with_wands",
        &[(
            added(PHYS, MAX),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_minimum_added_chaos_damage",
        &[(added(CHAOS_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_chaos_damage_with_bows",
        &[(
            added(CHAOS_T, MIN),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_chaos_damage_with_claws",
        &[(
            added(CHAOS_T, MIN),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_chaos_damage_with_daggers",
        &[(
            added(CHAOS_T, MIN),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_minimum_added_cold_damage",
        &[(added(COLD_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_axes",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_bows",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_claws",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_daggers",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_maces",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_staves",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Sword Attacks": BASE,
    // `ModFlag.Sword` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_swords",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Wand Attacks": BASE,
    // `ModFlag.Wand` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_cold_damage_with_wand",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
    ),
    (
        "attack_minimum_added_fire_damage",
        &[(added(FIRE_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE)],
    ),
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_fire_damage_with_axes",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_fire_damage_with_bow",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_fire_damage_with_claws",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_fire_damage_with_daggers",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_fire_damage_with_maces",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_fire_damage_with_staves",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
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
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_axes",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_bows",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_claws",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_daggers",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_maces",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_staves",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Sword Attacks": BASE,
    // `ModFlag.Sword` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_lightning_damage_with_swords",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
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
    // P6-11ay: "# to # Added ... Damage with Axe Attacks": BASE,
    // `ModFlag.Axe` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_axes",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::AXE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Bow Attacks": BASE,
    // `ModFlag.Bow` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_bow",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::BOW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Claw Attacks": BASE,
    // `ModFlag.Claw` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_claws",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::CLAW.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Dagger Attacks": BASE,
    // `ModFlag.Dagger` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_daggers",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::DAGGER.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Mace or Sceptre Attacks": BASE,
    // `ModFlag.Mace` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_maces",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::MACE.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Staff Attacks": BASE,
    // `ModFlag.Staff` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_staves",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::STAFF.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Sword Attacks": BASE,
    // `ModFlag.Sword` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_swords",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::SWORD.union(Flags::HIT),
            NONE,
        )],
    ),
    // P6-11ay: "# to # Added ... Damage with Wand Attacks": BASE,
    // `ModFlag.Wand` and `Hit` (PoB's "with ... attacks").
    (
        "attack_minimum_added_physical_damage_with_wands",
        &[(
            added(PHYS, MIN),
            Op::Base,
            1.0,
            Flags::WAND.union(Flags::HIT),
            NONE,
        )],
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
    // P6-11en: "#% chance to Impale Enemies on Hit with Two Handed Weapons"
    // (`ImpaleChance` BASE with `ModFlag.Weapon2H` and `Hit`).
    (
        "attacks_with_two_handed_weapons_impale_on_hit_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleChance),
            Op::Base,
            1.0,
            Flags::WEAPON_2H.union(Flags::HIT),
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
    // P6-11ef: "Attack Damage Penetrates #% of Enemy Elemental Resistances"
    // (`ElementalPenetration` BASE with `ModFlag.Attack`).
    (
        "base_attack_damage_penetrates_elemental_resist_%",
        &[
            (
                Quantity::Penetration(FIRE_T),
                Op::Base,
                1.0,
                Flags::ATTACK,
                NONE,
            ),
            (
                Quantity::Penetration(COLD_T),
                Op::Base,
                1.0,
                Flags::ATTACK,
                NONE,
            ),
            (
                Quantity::Penetration(LIGHTNING_T),
                Op::Base,
                1.0,
                Flags::ATTACK,
                NONE,
            ),
        ],
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
    // P6-11aj: Multistrike's repeats (`RepeatCount` BASE with a melee
    // weapon or unarmed, `SkillStatMap.lua` 297; its `RequiresShield`
    // alternative is not modelled: no such skill is a modelled attack).
    (
        "base_melee_attack_repeat_count",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatCount),
            Op::Base,
            1.0,
            NONE,
            Flags::WEAPON_MELEE.union(Flags::UNARMED),
        )],
    ),
    // P6-11ef: "Damage Penetrates #% of Enemy Elemental Resistances"
    // (`ElementalPenetration` BASE).
    (
        "base_penetrate_elemental_resistances_%",
        &[
            (Quantity::Penetration(FIRE_T), Op::Base, 1.0, NONE, NONE),
            (Quantity::Penetration(COLD_T), Op::Base, 1.0, NONE, NONE),
            (
                Quantity::Penetration(LIGHTNING_T),
                Op::Base,
                1.0,
                NONE,
                NONE,
            ),
        ],
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
    // P6-11ef: "#% of Physical Damage Converted to Cold Damage"
    // (`PhysicalDamageConvertToCold` BASE), as the fire one.
    (
        "base_physical_damage_%_to_convert_to_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::Conversion(PHYS, COLD_T)),
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
    // P6-11aj: Spell Echo's repeats (`RepeatCount` BASE, `ModFlag.Cast`,
    // `SkillStatMap.lua` 294).
    (
        "base_spell_repeat_count",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatCount),
            Op::Base,
            1.0,
            Flags::CAST,
            NONE,
        )],
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
    // P6-11bd: "+#% to Damage over Time Multiplier for Bleeding":
    // `DotMultiplier` BASE with `KeywordFlag.Bleed`, as poison's.
    (
        "bleeding_dot_multiplier_+",
        &[(Quantity::DotMultiplier, Op::Base, 1.0, Flags::BLEED, NONE)],
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
    // P6-11cx: "#% increased Cast Speed with Brand Skills" (`Speed` INC,
    // `ModFlag.Cast`, `SkillType.Brand`).
    (
        "cast_speed_for_brand_skills_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::CAST.union(Flags::SKILL_BRAND),
            NONE,
        )],
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
    // P6-11ec: "#% increased Chaos Damage over Time" (`ChaosDamage` INC with
    // `KeywordFlag.ChaosDot`), as the burning damage row.
    (
        "chaos_damage_over_time_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::CHAOS.union(Flags::CHAOS_DOT),
            NONE,
        )],
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
    // P6-11eo: "Curse Skills have #% increased Cast Speed" (`Speed` INC with
    // `ModFlag.Cast` and `KeywordFlag.Curse`).
    (
        "curse_cast_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::CAST.union(Flags::KEYWORD_CURSE),
            NONE,
        )],
    ),
    (
        "damage_+%",
        &[(Quantity::Damage, Op::Increased, 1.0, NONE, NONE)],
    ),
    // P6-11eq: "You and nearby Allies deal #% increased Damage" (PoB's
    // `ExtraAura` of `Damage` INC: on you, scaled by the buff and aura
    // effect on you, both unmapped, so reasons when present).
    (
        "damage_+%_for_you_and_allies_affected_by_your_auras",
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
    // P6-11ed: "#% increased Damage with Two Handed Weapons" (`Damage` INC
    // with `ModFlag.Weapon2H` and `Hit`).
    (
        "damage_+%_with_two_handed_weapons",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::WEAPON_2H.union(Flags::HIT),
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
    // P6-11dz: "+#% to Damage Over Time Multiplier with Spell Skills"
    // (`DotMultiplier` BASE with `KeywordFlag.Spell`).
    (
        "damage_over_time_multiplier_+_with_spells",
        &[(
            Quantity::DotMultiplier,
            Op::Base,
            1.0,
            Flags::KEYWORD_SPELL,
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
    // P6-11en: "#% increased Effect of Impales you inflict with Two Handed
    // Weapons" (`ImpaleEffect` INC with `ModFlag.Weapon2H` and `Hit`).
    (
        "impale_inflicted_by_two_handed_weapons_debuff_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleEffect),
            Op::Increased,
            1.0,
            Flags::WEAPON_2H.union(Flags::HIT),
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
    // P6-11cx: "Mark Skills have #% increased Cast Speed" (`Speed` INC,
    // `ModFlag.Cast`, `SkillType.Mark`).
    (
        "mark_skill_cast_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::CAST.union(Flags::SKILL_MARK),
            NONE,
        )],
    ),
    (
        "maximum_added_lightning_damage_from_skill",
        &[(added(LIGHTNING_T, MAX), Op::Base, 1.0, NONE, NONE)],
    ),
    // P6-11bo: Volatility's "#% more Maximum Attack Damage" and "#% less
    // Minimum Attack Damage" (`MaxDamage` and `MinDamage` MORE, `SkillType`
    // Attack).
    (
        "maximum_attack_damage_+%_final_from_volatility_support",
        &[(
            Quantity::Poe1(Poe1Quantity::BoundDamage(None, Bound::Max)),
            Op::More,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ), // P6-11bo: "#% more Maximum Physical Attack Damage" and Ryslatha's
    // Clutches' maximum and minimum (`MaxPhysicalDamage` and
    // `MinPhysicalDamage` MORE, `SkillType` Attack).
    (
        "maximum_physical_attack_damage_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::BoundDamage(
                Some(DamageType::Physical),
                Bound::Max,
            )),
            Op::More,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ), // P6-11n: "#% increased Melee Attack Speed" (`Speed` INC, `ModFlag.Attack`
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
    // P6-11ee: "#% increased Critical Strike Chance with Mines" (`CritChance`
    // INC with `KeywordFlag.Mine`).
    (
        "mine_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::KEYWORD_MINE,
            NONE,
        )],
    ),
    // P6-11ee: "+#% to Critical Strike Multiplier with Mines"
    // (`CritMultiplier` BASE with `KeywordFlag.Mine`).
    (
        "mine_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::KEYWORD_MINE,
            NONE,
        )],
    ),
    // P6-11bk: "#% increased Mine Damage" (`Damage` INC, `KeywordFlag.Mine`).
    (
        "mine_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::KEYWORD_MINE,
            NONE,
        )],
    ),
    // P6-11ee: "Mine Damage Penetrates #% Elemental Resistances"
    // (`ElementalPenetration` BASE with `KeywordFlag.Mine`).
    (
        "mine_damage_penetrates_%_elemental_resistance",
        &[
            (
                Quantity::Penetration(FIRE_T),
                Op::Base,
                1.0,
                Flags::KEYWORD_MINE,
                NONE,
            ),
            (
                Quantity::Penetration(COLD_T),
                Op::Base,
                1.0,
                Flags::KEYWORD_MINE,
                NONE,
            ),
            (
                Quantity::Penetration(LIGHTNING_T),
                Op::Base,
                1.0,
                Flags::KEYWORD_MINE,
                NONE,
            ),
        ],
    ),
    // P6-11cy: "#% increased Mine Throwing Speed" (`MineLayingSpeed` INC).
    (
        "mine_laying_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::MineLayingSpeed),
            Op::Increased,
            1.0,
            NONE,
            NONE,
        )],
    ),
    (
        "minimum_added_lightning_damage_from_skill",
        &[(added(LIGHTNING_T, MIN), Op::Base, 1.0, NONE, NONE)],
    ),
    (
        "minimum_attack_damage_+%_final_from_volatility_support",
        &[(
            Quantity::Poe1(Poe1Quantity::BoundDamage(None, Bound::Min)),
            Op::More,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ), // P6-11aj: Multistrike's damage of each repeat (`RepeatOneDamage` to
    // `RepeatThreeDamage` MORE with a melee weapon or unarmed).
    (
        "multistrike_damage_+%_final_on_first_repeat",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatDamage(1)),
            Op::More,
            1.0,
            NONE,
            Flags::WEAPON_MELEE.union(Flags::UNARMED),
        )],
    ),
    (
        "multistrike_damage_+%_final_on_second_repeat",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatDamage(2)),
            Op::More,
            1.0,
            NONE,
            Flags::WEAPON_MELEE.union(Flags::UNARMED),
        )],
    ),
    (
        "multistrike_damage_+%_final_on_third_repeat",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatDamage(3)),
            Op::More,
            1.0,
            NONE,
            Flags::WEAPON_MELEE.union(Flags::UNARMED),
        )],
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
    // P6-11co: "Projectile Attack Skills have #% increased Critical Strike
    // Chance" (`CritChance` INC) and "+#% to Critical Strike Multiplier"
    // (`CritMultiplier` BASE), each with the `SkillType.RangedAttack` tag.
    (
        "projectile_attack_skill_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::SKILL_RANGED_ATTACK,
            NONE,
        )],
    ),
    (
        "projectile_attack_skill_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::SKILL_RANGED_ATTACK,
            NONE,
        )],
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
    // P6-11cf: "Projectiles deal #% increased Damage with Hits against nearby
    // Enemies": PoB's parser keeps no condition for "nearby" (`Damage` INC,
    // `ModFlag.Projectile`, `KeywordFlag.Hit`).
    (
        "projectile_damage_+%_vs_nearby_enemies",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::PROJECTILE.union(Flags::HIT),
            NONE,
        )],
    ), // P6-11b review: "Damage with Weapons Penetrates #% Elemental
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
    // P6-11di: "#% increased Brand Damage" (`Damage` INC with
    // `KeywordFlag.Brand`, which PoB sets from the `Brand` skill type, as
    // `Flags::SKILL_BRAND`; P6-11cx). A brand is not a main skill whose hit
    // the engine computes, so the row keeps the line from being a reason on
    // other skills, as in PoB.
    (
        "sigil_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::SKILL_BRAND,
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
    (
        "spell_and_attack_maximum_added_cold_damage",
        &[
            (added(COLD_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE),
            (added(COLD_T, MAX), Op::Base, 1.0, Flags::SPELL, NONE),
        ],
    ),
    (
        "spell_and_attack_maximum_added_fire_damage",
        &[
            (added(FIRE_T, MAX), Op::Base, 1.0, Flags::ATTACK, NONE),
            (added(FIRE_T, MAX), Op::Base, 1.0, Flags::SPELL, NONE),
        ],
    ),
    // P6-11ef: "Adds # to # Cold Damage to Spells and Attacks" (`ColdMin`
    // and `ColdMax` BASE with `KeywordFlag.Attack` or `Spell`): a
    // row each, as no skill is both.
    (
        "spell_and_attack_minimum_added_cold_damage",
        &[
            (added(COLD_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE),
            (added(COLD_T, MIN), Op::Base, 1.0, Flags::SPELL, NONE),
        ],
    ),
    // P6-11ef: "Adds # to # Fire Damage to Spells and Attacks" (`FireMin`
    // and `FireMax` BASE with `KeywordFlag.Attack` or `Spell`): a
    // row each, as no skill is both.
    (
        "spell_and_attack_minimum_added_fire_damage",
        &[
            (added(FIRE_T, MIN), Op::Base, 1.0, Flags::ATTACK, NONE),
            (added(FIRE_T, MIN), Op::Base, 1.0, Flags::SPELL, NONE),
        ],
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
    // P6-11bw: "#% increased Spell Damage per 100 Maximum Mana, up to 40%"
    // (P6-11cw: `hit::DAMAGE_PER_STAT`, with the attack damage per 450
    // armour, `attack_damage_+%_per_450_physical_damage_reduction_rating`)
    // (the game's text; `Damage` INC, `ModFlag.Spell`, `PerStat` `Mana`
    // `div` 100 `limit` 40 `limitTotal`): the hit's computed term (`hit::spell_damage_per_mana`).
    (
        "spell_damage_+%_per_100_maximum_mana_up_to_60%",
        &[(
            Quantity::Poe1(Poe1Quantity::SpellDamagePer100Mana),
            Op::Base,
            1.0,
            Flags::SPELL,
            NONE,
        )],
    ), // P6-11aj: Awakened Spell Echo's final repeat's double damage
    // (`RepeatFinalDoubleDamageChance` BASE, `ModFlag.Spell`).
    (
        "spell_echo_plus_chance_double_damage_%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatFinalDoubleDamageChance),
            Op::Base,
            1.0,
            Flags::SPELL,
            NONE,
        )],
    ),
    (
        "spell_maximum_added_chaos_damage",
        &[(added(CHAOS_T, MAX), Op::Base, 1.0, Flags::SPELL, NONE)],
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
        "spell_minimum_added_chaos_damage",
        &[(added(CHAOS_T, MIN), Op::Base, 1.0, Flags::SPELL, NONE)],
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
    // P6-11dc: Burning Damage Support's more burning damage (`sup_str.lua`
    // 915: `FireDamage` MORE with `KeywordFlag.FireDot`).
    (
        "support_burning_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::FIRE.union(Flags::FIRE_DOT),
            NONE,
        )],
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
    // P6-11aj: Spell Echo's "#% more Damage" (`Damage` MORE) and the
    // echoes' cast speed (`Speed` MORE, `ModFlag.Cast`).
    (
        "support_echo_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, NONE, NONE)],
    ),
    // P6-11af: Efficacy's "#% more Damage over Time" (`Damage` MORE,
    // `ModFlag.Dot`) and its spell damage (`ModFlag.Spell`; `sup_int.lua`
    // 1984 to 1989).
    (
        "support_efficacy_damage_over_time_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::DOT, NONE)],
    ),
    (
        "support_efficacy_spell_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::SPELL, NONE)],
    ),
    // P6-11n: "#% more Elemental Damage" (`ElementalDamage` MORE).
    (
        "support_gem_elemental_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, NONE, Flags::ELEMENTAL)],
    ),
    // P6-11aj: Greater Spell Echo's damage per repeat
    // (`RepeatPerRepeatDamage` MORE).
    (
        "support_greater_spell_echo_spell_damage_+%_final_per_repeat",
        &[(
            Quantity::Poe1(Poe1Quantity::RepeatPerRepeatDamage),
            Op::More,
            1.0,
            NONE,
            NONE,
        )],
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
    (
        "support_multicast_cast_speed_+%_final",
        &[(Quantity::Speed, Op::More, 1.0, Flags::CAST, NONE)],
    ),
    // P6-11aj: Multistrike's "#% less Attack Damage" (`Damage` MORE,
    // `ModFlag.Attack`) and its attack speed with a melee weapon or
    // unarmed (`Speed` MORE).
    (
        "support_multiple_attack_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "support_multiple_attacks_melee_attack_speed_+%_final",
        &[(
            Quantity::Speed,
            Op::More,
            1.0,
            Flags::ATTACK,
            Flags::WEAPON_MELEE.union(Flags::UNARMED),
        )],
    ),
    // P6-11dn: Vicious Projectiles (PoB `sup_dex.lua` 4360 to 4372): chaos
    // damage over time MORE (`ChaosDamage` with `KeywordFlag.ChaosDot`).
    (
        "support_phys_chaos_projectile_chaos_damage_over_time_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::CHAOS.union(Flags::CHAOS_DOT),
            NONE,
        )],
    ),
    // P6-11dn: Vicious Projectiles' physical damage over time MORE
    // (`PhysicalDamage` with `KeywordFlag.PhysicalDot`).
    (
        "support_phys_chaos_projectile_physical_damage_over_time_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::PHYSICAL.union(Flags::PHYSICAL_DOT),
            NONE,
        )],
    ),
    // P6-11dn: Vicious Projectiles' projectile attack physical damage MORE
    // (`PhysicalDamage` with `ModFlag.Attack` and `Projectile`).
    (
        "support_projectile_attack_physical_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::ATTACK
                .union(Flags::PROJECTILE)
                .union(Flags::PHYSICAL),
            NONE,
        )],
    ),
    // P6-11dn: Vicious Projectiles' projectile attack speed MORE (`Speed`
    // with `ModFlag.Attack` and `Projectile`).
    (
        "support_projectile_attack_speed_+%_final",
        &[(
            Quantity::Speed,
            Op::More,
            1.0,
            Flags::ATTACK.union(Flags::PROJECTILE),
            NONE,
        )],
    ),
    // P6-11ca: Pulverise (`Data/Skills/sup_str.lua` 4289 to 4294): `Damage`
    // MORE with `ModFlag.Melee` and `ModFlag.Area`, `Speed` MORE with
    // `ModFlag.Attack`.
    (
        "support_pulverise_attack_speed_+%_final",
        &[(Quantity::Speed, Op::More, 1.0, Flags::ATTACK, NONE)],
    ),
    (
        "support_pulverise_melee_area_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            Flags::MELEE.union(Flags::AREA),
            NONE,
        )],
    ), // P6-11ab: Rapid Decay's "#% more Damage over Time" (`Damage` MORE,
    // `ModFlag.Dot`, `sup_dex.lua` 3854).
    (
        "support_rapid_decay_damage_over_time_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::DOT, NONE)],
    ),
    // P6-11bt: Ruthless (`SupportRuthless`, `Data/Skills/sup_str.lua` 4694 to
    // 4708): `RuthlessBlowHitMultiplier`, `RuthlessBlowMaxCount`, and
    // `RuthlessBlowAilmentMultiplier` BASE, the multipliers with
    // `ModFlag.Melee`.
    (
        "support_ruthless_big_hit_damage_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::Ruthless(super::RuthlessValue::Hit)),
            Op::Base,
            1.0,
            Flags::MELEE,
            NONE,
        )],
    ),
    (
        "support_ruthless_big_hit_max_count",
        &[(
            Quantity::Poe1(Poe1Quantity::Ruthless(super::RuthlessValue::MaxCount)),
            Op::Base,
            1.0,
            NONE,
            NONE,
        )],
    ),
    (
        "support_ruthless_blow_ailment_damage_from_melee_hits_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::Ruthless(super::RuthlessValue::Ailment)),
            Op::Base,
            1.0,
            Flags::MELEE,
            NONE,
        )],
    ), // P6-11aa: Infused Channelling's "#% more Damage" (`Damage` MORE,
    // `sup_int.lua` 3895).
    (
        "support_storm_barrier_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, NONE, NONE)],
    ),
    // P6-11ab: Swift Affliction's "#% more Skill Effect Duration and
    // Duration of Damaging Ailments" (`SkillStatMap.lua` 705): `Duration`
    // MORE, which nothing modelled reads (the `skill-duration` rows of
    // `NO_EFFECT`; an ailment that is a skill effect is a reason), and
    // `DamagingAilmentDuration` MORE.
    (
        "support_swift_affliction_skill_effect_and_damaging_ailment_duration_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentDuration),
            Op::More,
            1.0,
            NONE,
            DAMAGING_AILMENTS,
        )],
    ),
    // P6-11cy: Trap and Mine Damage's more damage (`Damage` MORE with `bor(KeywordFlag.Trap, KeywordFlag.Mine)`, either keyword).
    (
        "support_trap_and_mine_damage_+%_final",
        &[(
            Quantity::Damage,
            Op::More,
            1.0,
            NONE,
            Flags::KEYWORD_TRAP.union(Flags::KEYWORD_MINE),
        )],
    ),
    // P6-11cy: Trap and Mine Damage's less laying speed (`MineLayingSpeed` MORE).
    (
        "support_trap_and_mine_damage_mine_throwing_speed_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::MineLayingSpeed),
            Op::More,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P6-11cy: Trap and Mine Damage's less throwing speed (`TrapThrowingSpeed` MORE).
    (
        "support_trap_and_mine_damage_trap_throwing_speed_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::TrapThrowingSpeed),
            Op::More,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P6-11cy: Trap Support's more trap damage (`Damage` MORE,
    // `KeywordFlag.Trap`, `SkillStatMap.lua` 1777).
    (
        "support_trap_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::KEYWORD_TRAP, NONE)],
    ),
    // P6-11do: Void Manipulation's more chaos damage (`sup_dex.lua`:
    // `ChaosDamage` MORE).
    (
        "support_void_manipulation_chaos_damage_+%_final",
        &[(Quantity::Damage, Op::More, 1.0, Flags::CHAOS, NONE)],
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
    // P6-11eh: "#% increased Critical Strike Chance with Totem Skills"
    // (`CritChance` INC with `KeywordFlag.Totem`).
    (
        "totem_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::KEYWORD_TOTEM,
            NONE,
        )],
    ),
    // P6-11cf: totem skills' crit multiplier, attack speed, and trap damage
    // (PoB: `CritMultiplier` BASE, `Speed` INC with `ModFlag.Attack`, `Damage`
    // INC, each with its `KeywordFlag`; P6-11bk's keywords).
    (
        "totem_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::KEYWORD_TOTEM,
            NONE,
        )],
    ), // P6-11bk: "#% increased Totem Damage" (`Damage` INC, `KeywordFlag.Totem`).
    (
        "totem_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::KEYWORD_TOTEM,
            NONE,
        )],
    ), // P6-11b: "#% increased Accuracy Rating with Two Handed Melee Weapons": `Accuracy` INC, `ModFlag.Weapon2H`, `WeaponMelee`, `Hit`.
    (
        "totem_skill_attack_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::ATTACK.union(Flags::KEYWORD_TOTEM),
            NONE,
        )],
    ),
    // P6-11ef: "Spells Cast by Totems have #% increased Cast Speed" (`Speed`
    // INC with `ModFlag.Spell`, `Cast`, and `KeywordFlag.Totem`).
    (
        "totem_skill_cast_speed_+%",
        &[(
            Quantity::Speed,
            Op::Increased,
            1.0,
            Flags::SPELL.union(Flags::CAST).union(Flags::KEYWORD_TOTEM),
            NONE,
        )],
    ),
    // P6-11ef: "#% increased Trap and Mine Throwing Speed"
    // (`TrapThrowingSpeed` and `MineLayingSpeed` INC).
    (
        "trap_and_mine_throwing_speed_+%",
        &[
            (
                Quantity::Poe1(Poe1Quantity::TrapThrowingSpeed),
                Op::Increased,
                1.0,
                NONE,
                NONE,
            ),
            (
                Quantity::Poe1(Poe1Quantity::MineLayingSpeed),
                Op::Increased,
                1.0,
                NONE,
                NONE,
            ),
        ],
    ),
    // P6-11eh: "#% increased Critical Strike Chance with Traps" (`CritChance`
    // INC with `KeywordFlag.Trap`).
    (
        "trap_critical_strike_chance_+%",
        &[(
            Quantity::CritChance,
            Op::Increased,
            1.0,
            Flags::KEYWORD_TRAP,
            NONE,
        )],
    ),
    // P6-11eh: "+#% to Critical Strike Multiplier with Traps"
    // (`CritMultiplier` BASE with `KeywordFlag.Trap`).
    (
        "trap_critical_strike_multiplier_+",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            1.0,
            Flags::KEYWORD_TRAP,
            NONE,
        )],
    ),
    (
        "trap_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            1.0,
            Flags::KEYWORD_TRAP,
            NONE,
        )],
    ), // P6-11bx: "#% chance to treat Enemy Monster Elemental Resistance values
    // P6-11cy: "#% increased Trap Throwing Speed" (`TrapThrowingSpeed` INC).
    (
        "trap_throwing_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::TrapThrowingSpeed),
            Op::Increased,
            1.0,
            NONE,
            NONE,
        )],
    ),
    // P6-11ex: "#% increased Cooldown Recovery Rate of Travel Skills"
    // (`CooldownRecovery` INC, `SkillType.Travel`): the main skill's
    // cooldown reads it.
    (
        "travel_skill_cooldown_speed_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::TypedCooldownRecovery),
            Op::Increased,
            1.0,
            Flags::SKILL_TRAVEL,
            NONE,
        )],
    ),
    // as inverted" (`HitsInvertEleResChance` CHANCE, `div = 100`,
    // `SkillStatMap.lua` 821).
    (
        "treat_enemy_resistances_as_negated_on_elemental_damage_hit_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::InvertElementalResistChance),
            Op::Base,
            100.0,
            NONE,
            NONE,
        )],
    ),
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
    (
        "unique_ryuslathas_clutches_maximum_physical_attack_damage_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::BoundDamage(
                Some(DamageType::Physical),
                Bound::Max,
            )),
            Op::More,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ),
    (
        "unique_ryuslathas_clutches_minimum_physical_attack_damage_+%_final",
        &[(
            Quantity::Poe1(Poe1Quantity::BoundDamage(
                Some(DamageType::Physical),
                Bound::Min,
            )),
            Op::More,
            1.0,
            Flags::ATTACK,
            NONE,
        )],
    ), // P6-11b: "#% increased Accuracy Rating with Wands": `Accuracy` INC, `ModFlag.Wand` and `Hit`.
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
                    // An end of the damage range (P6-11bo: Volatility's).
                    | Poe1Quantity::BoundDamage(..)
                    // Ruthless (P6-11bt).
                    | Poe1Quantity::Ruthless(_)
                    // Trap and mine speeds (P6-11cy).
                    | Poe1Quantity::TrapThrowingSpeed
                    | Poe1Quantity::MineLayingSpeed
                    // P6-11bx.
                    | Poe1Quantity::InvertElementalResistChance
                    | Poe1Quantity::SkillConversion(..)
                    | Poe1Quantity::Conversion(..)
                    // Gain as extra (P6-11bp; PoB's `SkillStatMap.lua` 925
                    // to 945 for a skill's own stats).
                    | Poe1Quantity::GainAs(..)
                    | Poe1Quantity::DoubleDamageChance
                    // The repeats (P6-11aj).
                    | Poe1Quantity::RepeatCount
                    | Poe1Quantity::RepeatDamage(_)
                    | Poe1Quantity::RepeatPerRepeatDamage
                    | Poe1Quantity::RepeatFinalDoubleDamageChance
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
                    | Poe1Quantity::ManaCostChannelling(_)
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

/// Cyclone's movement speed stat (P6-11cr; [`SkillMapping`]).
pub const CYCLONE_MOVEMENT_STAT: &str = "cyclone_movement_speed_+%_final";

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
            // A support's global effect: `poe1::gems` applies it (P6-11y).
            || super::gems::is_support_global_stat(stat.as_str())
        {
            return StatMap::NoEffect;
        }
        // P6-11cr: Cyclone's movement speed as the main skill (PoB's
        // `MovementSpeed` MORE with `Condition:ChannellingCyclone`, a
        // `GlobalEffect` buff of any active Cyclone, `act_dex.lua` 4719;
        // another group's Cyclone is `poe1::gems`').
        if stat.as_str() == CYCLONE_MOVEMENT_STAT {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::MovementSpeed),
                op: Op::More,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none()
                    .with_state(ConditionKey::new(conditions::CHANNELLING_CYCLONE), true),
            }]);
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
            // A per-charge row whose effect is a hit's (P6-11do, Power
            // Charge On Critical's more damage per power charge): the
            // charge phase turns it into that effect times the count, so it
            // stays the main skill's, as PoB's `skillModList` entry.
            StatMap::Effects(e)
                if e.iter().all(|t| match t.quantity {
                    Quantity::Poe1(Poe1Quantity::Charge(
                        _,
                        super::charges::ChargeValue::Per(p),
                    )) => is_hit_quantity(p.target().0),
                    _ => false,
                }) =>
            {
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
            // A field's reason marker (P6-10b): the stat's reason stays on
            // that field.
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
            // A skill's own duration (P6-11bs): no slice field reads it
            // through the hit, as before the warcries read the character's.
            StatMap::Effects(e)
                if e.iter().all(|t| {
                    t.quantity
                        == Quantity::Poe1(Poe1Quantity::SkillTiming(
                            super::skill_modifiers::SkillTiming::Duration,
                        ))
                }) =>
            {
                StatMap::NoEffect
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

/// The minion's penetration of each element (P6-11cu).
const MINION_FIRE_PEN: Quantity = Quantity::Poe1(Poe1Quantity::Minion(
    super::minions::MinionValue::Penetration(DamageType::Fire),
));
const MINION_COLD_PEN: Quantity = Quantity::Poe1(Poe1Quantity::Minion(
    super::minions::MinionValue::Penetration(DamageType::Cold),
));
const MINION_LIGHTNING_PEN: Quantity = Quantity::Poe1(Poe1Quantity::Minion(
    super::minions::MinionValue::Penetration(DamageType::Lightning),
));

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
    // P6-11ec: "+#% Chance to Block Attack Damage if you've dealt a Critical
    // Strike Recently" (`BlockChance` BASE, `CritRecently`).
    (
        "additional_block_%_if_you_have_crit_recently",
        &[(
            Quantity::BlockChance,
            Op::Base,
            NONE,
            conditions::CRIT_RECENTLY,
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
    // P6-11bv: "while Leeching" (PoB's `Condition:Leeching`, "while
    // Leeching Energy Shield" `LeechingEnergyShield`), each as PoB reads it:
    // attack speed and attack damage with `ModFlag.Attack`, "attack and cast
    // speed" `Speed`, "armour and evasion rating" both.
    (
        "armour_evasion_+%_while_leeching",
        &[
            (Quantity::Armour, Op::Increased, NONE, conditions::LEECHING),
            (Quantity::Evasion, Op::Increased, NONE, conditions::LEECHING),
            (
                Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                Op::Increased,
                NONE,
                conditions::LEECHING,
            ),
        ],
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
    (
        "attack_and_cast_speed_+%_while_leeching",
        &[(Quantity::Speed, Op::Increased, NONE, conditions::LEECHING)],
    ),
    (
        "attack_and_cast_speed_+%_while_leeching_energy_shield",
        &[(
            Quantity::Speed,
            Op::Increased,
            NONE,
            conditions::LEECHING_ENERGY_SHIELD,
        )],
    ),
    // P6-11ec: "#% increased Attack Damage against Maimed Enemies" (`Damage`
    // INC with `ModFlag.Attack`, enemy `Maimed`).
    (
        "attack_damage_+%_vs_maimed_enemies",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::ATTACK,
            conditions::ENEMY_MAIMED,
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
    (
        "attack_damage_+%_while_leeching",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::ATTACK,
            conditions::LEECHING,
        )],
    ),
    // P6-11ec: "#% increased Attack Damage against Bleeding Enemies"
    // (`Damage` INC with `ModFlag.Attack`, enemy `Bleeding`).
    (
        "attack_damage_vs_bleeding_enemies_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::ATTACK,
            conditions::ENEMY_BLEEDING,
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
    // P6-11ec: "#% increased Attack Speed if you've dealt a Critical Strike
    // Recently" (`Speed` INC with `ModFlag.Attack`, `CritRecently`).
    (
        "attack_speed_+%_if_have_crit_recently",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::ATTACK,
            conditions::CRIT_RECENTLY,
        )],
    ),
    // P6-11ai: "+#% chance to Suppress Spell Damage while on Full Life"
    // (`SpellSuppressionChance` BASE, `FullLife`) and "#% increased Attack
    // Speed when on Full Life" (`Speed` INC with attacks, `FullLife`).
    (
        "attack_speed_+%_when_on_full_life",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::ATTACK,
            conditions::FULL_LIFE,
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
    (
        "attack_speed_+%_while_leeching",
        &[(
            Quantity::Speed,
            Op::Increased,
            Flags::ATTACK,
            conditions::LEECHING,
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
    // P6-11cc: "+#% Chance to Block Attack Damage if you were Damaged by a
    // Hit Recently" (`BlockChance` BASE, `BeenHitRecently`).
    (
        "block_chance_on_damage_taken_%",
        &[(BLOCK, Op::Base, NONE, conditions::BEEN_HIT_RECENTLY)],
    ), // Close-out: "#% increased Cast Speed if you've dealt a Critical Strike
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
    // P6-11em: "+#% Chance to Block Spell Damage if you've Cast a Spell
    // Recently" (`SpellBlockChance` BASE, `CastSpellRecently`).
    (
        "chance_to_block_spells_%_if_cast_a_spell_recently",
        &[(SPELL_BLOCK, Op::Base, NONE, conditions::CAST_SPELL_RECENTLY)],
    ),
    // P6-11ba: "#% Chance to Block Spell Damage if you were Damaged by a Hit
    // Recently": `SpellBlockChance` BASE, `BeenHitRecently` (the config's).
    (
        "chance_to_block_spells_%_if_damaged_by_a_hit_recently",
        &[(SPELL_BLOCK, Op::Base, NONE, conditions::BEEN_HIT_RECENTLY)],
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
    // P6-11eg: "#% increased Critical Strike Chance if you haven't Blocked
    // Recently" (`CritChance` INC, `BlockedRecently` negated).
    (
        "critical_strike_chance_+%_if_havent_blocked_recently",
        &[(
            Quantity::CritChance,
            Op::Increased,
            NONE,
            conditions::NOT_BLOCKED_RECENTLY,
        )],
    ),
    // P6-11dz: "#% increased Critical Strike Chance against Bleeding Enemies"
    // (`CritChance` INC, enemy `Bleeding`).
    (
        "critical_strike_chance_+%_vs_bleeding_enemies",
        &[(
            Quantity::CritChance,
            Op::Increased,
            NONE,
            conditions::ENEMY_BLEEDING,
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
    // P6-11ed: "+#% Critical Strike Multiplier while a Rare or Unique Enemy
    // is Nearby" (`CritMultiplier` BASE, enemy `NearbyRareOrUniqueEnemy`
    // or `RareOrUnique`: P6-10b's condition).
    (
        "critical_strike_multiplier_+_if_rare_or_unique_enemy_nearby",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            NONE,
            conditions::ENEMY_RARE_OR_UNIQUE_NEARBY,
        )],
    ),
    // P6-11eg: "+#% to Critical Strike Multiplier if you have Blocked
    // Recently" (`CritMultiplier` BASE, `BlockedRecently`).
    (
        "critical_strike_multiplier_+_if_you_have_blocked_recently",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            NONE,
            conditions::BLOCKED_RECENTLY,
        )],
    ),
    // P6-11dp: "+#% to Critical Strike Multiplier against Bleeding Enemies"
    // (`CritMultiplier` BASE, enemy `Bleeding`).
    (
        "critical_strike_multiplier_+_vs_bleeding_enemies",
        &[(
            Quantity::CritMultiplier,
            Op::Base,
            NONE,
            conditions::ENEMY_BLEEDING,
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
    // P6-11de: "#% increased Damage with Hits and Ailments against Marked
    // Enemy" (`Damage` INC with `KeywordFlag.Hit` or `Ailment`, enemy
    // `Marked`). A hit's query and an ailment's never carry both flags, so
    // one row each reads as PoB's either-flag match.
    (
        "damage_+%_against_enemies_marked_by_you",
        &[
            (
                Quantity::Damage,
                Op::Increased,
                Flags::HIT,
                conditions::ENEMY_MARKED,
            ),
            (
                Quantity::Damage,
                Op::Increased,
                Flags::AILMENT,
                conditions::ENEMY_MARKED,
            ),
        ],
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
    (
        "damage_+%_while_leeching",
        &[(Quantity::Damage, Op::Increased, NONE, conditions::LEECHING)],
    ),
    // P6-11b: "#% increased Damage while wielding a Wand": `Damage` INC,
    // `UsingWand`.
    (
        "damage_+%_while_wielding_wand",
        &[(Quantity::Damage, Op::Increased, NONE, states::USING_WAND)],
    ),
    // P6-11ba: "#% increased Damage over Time while Dual Wielding": `Damage`
    // INC, `ModFlag.Dot`, `DualWielding`.
    (
        "damage_over_time_+%_while_dual_wielding",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::DOT,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "#% increased Damage over Time while holding a Shield": `Damage`
    // INC, `ModFlag.Dot`, `UsingShield` (PoB's "while holding a (%w+)").
    (
        "damage_over_time_+%_while_holding_a_shield",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::DOT,
            states::USING_SHIELD,
        )],
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
            (
                Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                Op::Increased,
                NONE,
                states::USING_STAFF,
            ),
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
    // P6-11dz: "#% increased Elemental Damage during any Flask Effect"
    // (`ElementalDamage` INC, `UsingFlask`). A query carries one damage
    // type, so one row each reads as PoB's elemental match.
    (
        "elemental_damage_+%_during_flask_effect",
        &[
            (
                Quantity::Damage,
                Op::Increased,
                Flags::FIRE,
                conditions::USING_FLASK,
            ),
            (
                Quantity::Damage,
                Op::Increased,
                Flags::COLD,
                conditions::USING_FLASK,
            ),
            (
                Quantity::Damage,
                Op::Increased,
                Flags::LIGHTNING,
                conditions::USING_FLASK,
            ),
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
        "evasion_rating_+%_while_leeching",
        &[(Quantity::Evasion, Op::Increased, NONE, conditions::LEECHING)],
    ),
    // P6-11ba: "#% increased Evasion Rating while moving": `Evasion` INC,
    // `Moving` (the config's).
    (
        "evasion_rating_+%_while_moving",
        &[(Quantity::Evasion, Op::Increased, NONE, conditions::MOVING)],
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
    // P6-11dk: "# to # added Fire Damage against Burning Enemies" (PoB's
    // `SkillStatMap.lua` 831 to 836: `FireMin` and `FireMax` BASE, enemy
    // `Burning`; Immolate Support's).
    (
        "global_maximum_added_fire_damage_vs_burning_enemies",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            NONE,
            conditions::ENEMY_BURNING,
        )],
    ),
    (
        "global_minimum_added_fire_damage_vs_burning_enemies",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            NONE,
            conditions::ENEMY_BURNING,
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
    // P6-11cq: "Regenerate #% of Life per Second if you've used a Life
    // Flask in the past 10 seconds" (`LifeRegenPercent` BASE,
    // `UsingLifeFlask`; the active flasks).
    (
        "life_regeneration_rate_per_minute_%_if_used_life_flask_in_past_10_seconds",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(Pool::Life)),
            Op::Base,
            NONE,
            conditions::USING_LIFE_FLASK,
        )],
    ),
    (
        "life_regeneration_rate_per_minute_%_while_stationary",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(Pool::Life)),
            Op::Base,
            NONE,
            conditions::STATIONARY,
        )],
    ),
    // P6-11de: "Marked Enemy has #% increased Accuracy Rating" (an
    // `EnemyModifier` of `Accuracy` INC, enemy `Marked`).
    (
        "marked_enemy_accuracy_rating_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnemyAccuracy),
            Op::Increased,
            NONE,
            conditions::ENEMY_MARKED,
        )],
    ),
    // P6-11de: "Marked Enemy takes #% increased Damage" (an `EnemyModifier`
    // of `DamageTaken` INC, enemy `Marked`).
    (
        "marked_enemy_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnemyDamageTaken),
            Op::Increased,
            NONE,
            conditions::ENEMY_MARKED,
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
    // P6-11cm: "Minions deal #% increased Damage if you've used a Minion
    // Skill Recently" (PoB's `MinionModifier` LIST of `Damage` INC with the
    // outer condition `UsedMinionSkillRecently`, `ModParser.lua` 4381):
    // the minion modifier with the character's condition
    // (`minion_actor::minion_modifiers`, `minions::minions`).
    (
        "minion_damage_+%_if_have_used_a_minion_skill_recently",
        &[(
            Quantity::Poe1(Poe1Quantity::Minion(super::minions::MinionValue::Damage)),
            Op::Increased,
            NONE,
            conditions::USED_MINION_SKILL_RECENTLY,
        )],
    ),
    // P6-11ct: "Minions deal #% increased Damage while you are affected by
    // a Herald" (PoB's `MinionModifier` LIST of `Damage` INC with the
    // inner `ActorCondition` `parent` `AffectedByHerald`): the character's
    // condition.
    (
        "minion_damage_+%_while_affected_by_a_herald",
        &[(
            Quantity::Poe1(Poe1Quantity::Minion(super::minions::MinionValue::Damage)),
            Op::Increased,
            NONE,
            conditions::AFFECTED_BY_HERALD,
        )],
    ),
    // P6-11cu: "Minions Penetrate #% of Cursed Enemies' Elemental
    // Resistances" (PoB's `MinionModifier` LIST of `ElementalPenetration`
    // BASE with the inner `ActorCondition` `enemy` `Cursed`).
    (
        "minions_penetrate_elemental_resistances_%_vs_cursed_enemies",
        &[
            (MINION_FIRE_PEN, Op::Base, NONE, conditions::ENEMY_CURSED),
            (MINION_COLD_PEN, Op::Base, NONE, conditions::ENEMY_CURSED),
            (
                MINION_LIGHTNING_PEN,
                Op::Base,
                NONE,
                conditions::ENEMY_CURSED,
            ),
        ],
    ),
    // P6-11eg: "#% increased Movement Speed during any Flask Effect"
    // (`MovementSpeed` INC, `UsingFlask`).
    (
        "movement_speed_+%_during_flask_effect",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            NONE,
            conditions::USING_FLASK,
        )],
    ),
    // P6-11ee: "#% increased Movement Speed if you've dealt a Critical Strike
    // Recently" (`MovementSpeed` INC, `CritRecently`).
    (
        "movement_speed_+%_if_crit_recently",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            NONE,
            conditions::CRIT_RECENTLY,
        )],
    ),
    // P6-11ee: "#% increased Movement Speed while holding a Shield"
    // (`MovementSpeed` INC, `UsingShield`).
    (
        "movement_speed_+%_while_holding_shield",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            NONE,
            states::USING_SHIELD,
        )],
    ),
    // P6-11w: "#% increased Movement Speed while on Burning Ground"
    // (`MovementSpeed` INC, `OnBurningGround`, the config's check).
    (
        "movement_speed_+%_while_on_burning_ground",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            NONE,
            conditions::ON_BURNING_GROUND,
        )],
    ),
    // P6-11ei: "#% increased Movement Speed when on Low Life"
    // (`MovementSpeed` INC, `LowLife`).
    (
        "movement_velocity_+%_when_on_low_life",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            NONE,
            conditions::LOW_LIFE,
        )],
    ),
    // P6-11ei: "#% increased Movement Speed while Ignited"
    // (`MovementSpeed` INC, `Ignited`).
    (
        "movement_velocity_+%_while_ignited",
        &[(
            Quantity::Poe1(Poe1Quantity::MovementSpeed),
            Op::Increased,
            NONE,
            conditions::IGNITED,
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
    // P6-11dp: "#% chance to Impale Enemies on Hit with Attacks while using
    // Pride" (`ImpaleChance` BASE with `KeywordFlag.Attack`,
    // `AffectedByPride`).
    (
        "pride_chance_to_impale_with_attacks_%",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleChance),
            Op::Base,
            Flags::KEYWORD_ATTACK,
            "AffectedByPride",
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
    // P6-11dl: "Impales you inflict last # additional Hits while using Pride"
    // (`ImpaleStacksMax` BASE while `AffectedByPride`).
    (
        "pride_your_impaled_debuff_lasts_+_additional_hits",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleStacksMax),
            Op::Base,
            NONE,
            "AffectedByPride",
        )],
    ),
    // P6-11bj: "Recover #% of Energy Shield when you Block Spell Damage while
    // wielding a Staff" (`EnergyShieldOnSpellBlock` BASE, `PercentStat`
    // `EnergyShield`, `UsingStaff`; read by `enemy_hit`).
    (
        "recover_%_energy_shield_when_you_block_spell_damage_while_wielding_a_staff",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(
                super::BlockGain::EnergyShieldSpellPercent,
            )),
            Op::Base,
            NONE,
            states::USING_STAFF,
        )],
    ),
    // P6-11bj: "Recover #% of Life when you Block Attack Damage while wielding
    // a Staff" (`LifeOnBlock` BASE, `PercentStat` `Life`, `UsingStaff`).
    (
        "recover_%_life_when_you_block_attack_damage_while_wielding_a_staff",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(super::BlockGain::LifePercent)),
            Op::Base,
            NONE,
            states::USING_STAFF,
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
    // P6-11ba: "#% increased Spell Damage while Dual Wielding": `Damage`
    // INC, `ModFlag.Spell`, `DualWielding`.
    (
        "spell_damage_+%_while_dual_wielding",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
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
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_maximum_added_chaos_damage_while_dual_wielding",
        &[(
            added(CHAOS_T, MAX),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_maximum_added_cold_damage_while_dual_wielding",
        &[(
            added(COLD_T, MAX),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_maximum_added_fire_damage_while_dual_wielding",
        &[(
            added(FIRE_T, MAX),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_maximum_added_lightning_damage_while_dual_wielding",
        &[(
            added(LIGHTNING_T, MAX),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_maximum_added_physical_damage_while_dual_wielding",
        &[(
            added(PHYS, MAX),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_minimum_added_chaos_damage_while_dual_wielding",
        &[(
            added(CHAOS_T, MIN),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_minimum_added_cold_damage_while_dual_wielding",
        &[(
            added(COLD_T, MIN),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_minimum_added_fire_damage_while_dual_wielding",
        &[(
            added(FIRE_T, MIN),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_minimum_added_lightning_damage_while_dual_wielding",
        &[(
            added(LIGHTNING_T, MIN),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
        )],
    ),
    // P6-11ba: "# to # Added Spell ... Damage while Dual Wielding": BASE,
    // `KeywordFlag.Spell`, `DualWielding`.
    (
        "spell_minimum_added_physical_damage_while_dual_wielding",
        &[(
            added(PHYS, MIN),
            Op::Base,
            Flags::SPELL,
            states::DUAL_WIELDING,
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
    // P6-11ai: see "attack_speed_+%_when_on_full_life".
    (
        "spell_suppression_chance_%_while_on_full_life",
        &[(SUPPRESSION, Op::Base, NONE, conditions::FULL_LIFE)],
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
    // P6-11en: "#% increased Area Damage while wielding a Two Handed Melee
    // Weapon" (`Damage` INC with `ModFlag.Area`, `UsingTwoHandedWeapon` and
    // `UsingMeleeWeapon`).
    (
        "two_handed_melee_area_damage_+%",
        &[(
            Quantity::Damage,
            Op::Increased,
            Flags::AREA,
            states::USING_TWO_HANDED_MELEE_WEAPON,
        )],
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
    // P6-11eq: "If you've Attacked Recently, you and nearby Allies have +#% Chance to
    // Block Attack Damage" (PoB's `ExtraAura` of `BlockChance` BASE under
    // `AttackedRecently`).
    (
        "you_and_allies_additional_block_%_if_have_attacked_recently",
        &[(BLOCK, Op::Base, NONE, conditions::ATTACKED_RECENTLY)],
    ),
    // P6-11eq: "If you've Cast a Spell Recently, you and nearby Allies have +#%
    // Chance to Block Spell Damage" (PoB's `ExtraAura` of `SpellBlockChance`
    // BASE under `CastSpellRecently`).
    (
        "you_and_allies_additional_spell_block_%_if_cast_spell_recently",
        &[(SPELL_BLOCK, Op::Base, NONE, conditions::CAST_SPELL_RECENTLY)],
    ),
    // P6-11ev: "When you Warcry, you and nearby Allies gain Onslaught for 4
    // seconds" (PoB's `ExtraAura` of `Onslaught` FLAG under
    // `UsedWarcryRecently`; `poe1::buffs` applies it).
    (
        "you_and_nearby_allys_gain_onslaught_for_4_seconds_on_warcry",
        &[(
            Quantity::Poe1(Poe1Quantity::Onslaught),
            Op::Flag,
            NONE,
            conditions::USED_WARCRY_RECENTLY,
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
    // P6-11ch: Arcane Surge's effect on you and its more spell damage
    // (`ArcaneSurgeEffect` INC, `ArcaneSurgeDamage` MAX; `poe1::buffs`).
    (
        "arcane_surge_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::ArcaneSurgeEffect),
            Op::Increased,
            1.0,
        )],
    ),
    (
        "arcane_surge_on_you_spell_damage_+%_final_from_hierophant",
        &[(
            Quantity::Poe1(Poe1Quantity::ArcaneSurgeDamage),
            Op::Base,
            1.0,
        )],
    ), // P6-11by: the body armour's armour and energy shield when the helmet,
    // gloves, and boots all have both (`phases::primary_defence`).
    (
        "armour_and_energy_shield_+%_from_body_armour_if_gloves_helmet_boots_have_armour_and_energy_shield",
        &[(
            Quantity::Poe1(Poe1Quantity::BodyArmourIfHybridPieces),
            Op::Increased,
            1.0,
        )],
    ), // Basalt Flask's buff, "20% more Armour" in PoB's base data: `Armour`
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
    // P6-11er: "Auras from your Skills which affect Allies also affect
    // Enemies" (`AurasAffectEnemies` FLAG), read by `poe1::gems`.
    (
        "base_aura_skills_affecting_allies_also_affect_enemies",
        &[(
            Quantity::Poe1(Poe1Quantity::AurasAffectEnemies),
            Op::Flag,
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
    // P6-11bp: "Gain #% of <Type> Damage as Extra <Type> Damage"
    // (`<From>DamageGainAs<To>` BASE; elemental: `ElementalDamageGainAs<To>`,
    // which PoB sums into each elemental type, `CalcOffence.lua` 1893), read
    // by the hit's conversion table as the heralds' and Hatred's.
    (
        "cold_damage_%_to_add_as_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Cold, DamageType::Chaos)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "cold_damage_%_to_add_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Cold, DamageType::Fire)),
            Op::Base,
            1.0,
        )],
    ), // P6-10b part 12: "#% of Cold Damage from Hits taken as Fire Damage" (`ColdDamageFromHitsTakenAsFire` BASE).
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
    // P6-11bf: "#% increased Damage with Hits against Chilled Enemies"
    // (`Damage` INC, `KeywordFlag.Hit`, the enemy's `Chilled`):
    // `poe1::damage_taken` applies it while the enemy is chilled.
    (
        "damage_+%_vs_chilled_enemies",
        &[(
            Quantity::Poe1(Poe1Quantity::DamageVsChilled),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-10a: "#% of Damage taken Recouped as Life" (`LifeRecoup` BASE), a
    // Glorious Vanity conquest: as mana recoup below.
    (
        "damage_taken_goes_to_life_over_4_seconds_%",
        &[(Quantity::Poe1(Poe1Quantity::LifeRecoup), Op::Base, 1.0)],
    ),
    // P6-11as: "#% of Damage taken Recouped as Mana" (`ManaRecoup` BASE):
    // read by `poe1::enemy_hit` as PoB's `anyRecoup`.
    (
        "damage_taken_goes_to_mana_%",
        &[(Quantity::Poe1(Poe1Quantity::ManaRecoup), Op::Base, 1.0)],
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
    (
        "elemental_damage_%_to_add_as_chaos",
        &[
            (
                Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Fire, DamageType::Chaos)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Cold, DamageType::Chaos)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::GainAs(
                    DamageType::Lightning,
                    DamageType::Chaos,
                )),
                Op::Base,
                1.0,
            ),
        ],
    ), // P6-10b part 12: "#% of Elemental Damage from Hits taken as Chaos Damage" (`ElementalDamageFromHitsTakenAsChaos` BASE).
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
    // P6-11eq: "You and nearby Allies have +#% to Elemental Resistances"
    // (PoB's `ExtraAura` of `ElementalResist` BASE, as the damage one).
    (
        "elemental_resistances_+%_for_you_and_allies_affected_by_your_auras",
        &[
            (FIRE, Op::Base, 1.0),
            (COLD, Op::Base, 1.0),
            (LIGHTNING, Op::Base, 1.0),
        ],
    ),
    // P6-11dh: "Elemental Resistances are capped by your highest Maximum
    // Elemental Resistance instead" (`ElementalResistMaxIsHighestResistMax`
    // FLAG).
    (
        "elemental_resistances_are_limited_by_highest_maximum_elemental_resistance",
        &[(
            Quantity::Poe1(Poe1Quantity::ElementalResistMaxIsHighest),
            Op::Flag,
            1.0,
        )],
    ),
    // P6-11ak: a recovery on block (`BlockGain`, read by `enemy_hit`).
    (
        "energy_shield_%_of_armour_rating_gained_on_block",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(
                super::BlockGain::EnergyShieldPercentOfArmour,
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
    // P6-11bl: "#% increased Energy Shield from Equipped Helmet"
    // (`EnergyShield` INC, `SlotName` Helmet), as the shield's (P6-11k).
    (
        "energy_shield_from_helmet_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::SlotIncreased(
                ArmourSlot::Helmet,
                Defence::EnergyShield,
            )),
            Op::Increased,
            1.0,
        )],
    ), // P6-10b part 9: "#% of Damage Leeched as Energy Shield" (`DamageEnergyShieldLeech` BASE).
    // P6-11cb: "Maximum Energy Shield is increased by Chaos Resistance" and
    // "... by Chance to Block Spell Damage" (PoB's flags, `phases`).
    (
        "energy_shield_increased_by_chaos_resistance",
        &[(
            Quantity::Poe1(Poe1Quantity::EnergyShieldFromChaosResistance),
            Op::Flag,
            1.0,
        )],
    ),
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
    // P6-11ap: "Energy Shield protects Mana instead of Life"
    // (`EnergyShieldProtectsMana` FLAG).
    (
        "energy_shield_protects_mana",
        &[(Quantity::Poe1(Poe1Quantity::EsProtectsMana), Op::Flag, 1.0)],
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
            // P6-11ek: Iron Reflexes counts it once.
            (
                Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                Op::Increased,
                1.0,
            ),
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
        "fire_damage_%_to_add_as_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Fire, DamageType::Chaos)),
            Op::Base,
            1.0,
        )],
    ), // P6-10b part 12: "#% of Fire Damage from Hits taken as Cold Damage" (`FireDamageFromHitsTakenAsCold` BASE).
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
            // P6-11ek: Iron Reflexes counts it once.
            (
                Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                Op::Increased,
                1.0,
            ),
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
    // P6-11cp: "Ignore all Movement Penalties from Armour"
    // (`Condition:IgnoreMovementPenalties` FLAG; `poe1::movement`).
    (
        "ignore_armour_movement_penalties",
        &[(
            Quantity::Poe1(Poe1Quantity::IgnoreMovementPenalties),
            Op::Flag,
            1.0,
        )],
    ),
    // P6-11dp: "#% chance on Hitting an Enemy for all Impales on that Enemy
    // to last for an additional Hit" (`ImpaleAdditionalDurationChance`
    // BASE), read by the impale's stacks.
    (
        "impales_do_not_consume_hit_on_hit_%_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::ImpaleAdditionalDurationChance),
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
    // P6-11dq: Versatile Combatant, as PoB reads the keystone's lines: -10%
    // to maximum attack and spell block (`BlockChanceMax`,
    // `SpellBlockChanceMax` BASE), and "+2% Chance to Block Spell Damage
    // for each 1% Overcapped Chance to Block Attack Damage".
    (
        "keystone_versatile_combatant",
        &[
            (Quantity::MaximumBlockChance, Op::Base, -10.0),
            (
                Quantity::Poe1(Poe1Quantity::MaximumSpellBlockChance),
                Op::Base,
                -10.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::SpellBlockPerOvercappedBlock),
                Op::Base,
                2.0,
            ),
        ],
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
    // P6-11ak: a recovery on block (`BlockGain`, read by `enemy_hit`).
    (
        "life_gained_on_block",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(super::BlockGain::Life)),
            Op::Base,
            1.0,
        )],
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
    // P6-11bu: "Regenerate #% of Life per second per Fortification"
    // (`LifeRegenPercent` BASE per `FortificationStacks`).
    (
        "life_regeneration_per_minute_%_per_fortification",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPerFortification(Pool::Life)),
            Op::Base,
            1.0,
        )],
    ), // P6-11t: "Regenerate # Life per second for each 1% Uncapped Fire
    // Resistance" (`LifeRegen` BASE, `PerStat` `FireResistTotal`; the stat
    // per minute); [`super::regen`] reads it.
    (
        "life_regeneration_per_minute_per_1%_uncapped_fire_damage_resistance",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenPerFireResist(Pool::Life)),
            Op::Base,
            1.0 / 60.0,
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
    (
        "lightning_damage_%_to_add_as_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(
                DamageType::Lightning,
                DamageType::Chaos,
            )),
            Op::Base,
            1.0,
        )],
    ),
    (
        "lightning_damage_%_to_add_as_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(
                DamageType::Lightning,
                DamageType::Cold,
            )),
            Op::Base,
            1.0,
        )],
    ),
    (
        "lightning_damage_%_to_add_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(
                DamageType::Lightning,
                DamageType::Fire,
            )),
            Op::Base,
            1.0,
        )],
    ), // P6-10b part 12: "#% of Lightning Damage from Hits taken as Cold Damage" (`LightningDamageFromHitsTakenAsCold` BASE).
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
    // P6-11bn: "Nearby Enemies have #% to <Type> Resistance" and "to all
    // Resistances" (`EnemyModifier` `<Type>Resist` BASE; all: `ElementalResist`
    // and `ChaosResist`): the enemy's resistance, as a curse's.
    (
        "local_display_nearby_enemies_all_resistances_%",
        &[
            (
                Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Fire)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Cold)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Lightning)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Chaos)),
                Op::Base,
                1.0,
            ),
        ],
    ),
    (
        "local_display_nearby_enemy_chaos_damage_resistance_%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Chaos)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "local_display_nearby_enemy_cold_damage_resistance_%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Cold)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "local_display_nearby_enemy_fire_damage_resistance_%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Fire)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "local_display_nearby_enemy_lightning_damage_resistance_%",
        &[(
            Quantity::Poe1(Poe1Quantity::EnemyResistance(DamageType::Lightning)),
            Op::Base,
            1.0,
        )],
    ), // P6-11ad: "You count as on Low Life while at #% of maximum Life or
    // P6-11ca: a flask's crit chance against an enemy on consecrated ground
    // (`poe1::flasks::FLASK_ROWS`; the stat in hundredths,
    // `divide_by_one_hundred`).
    (
        "local_flask_critical_strike_chance_against_enemies_on_consecrated_ground_%_during_effect",
        &[(
            Quantity::Poe1(Poe1Quantity::ConsecratedCritChance),
            Op::Base,
            0.01,
        )],
    ), // P6-11bz: a flask's "Consecrated Ground created during Effect applies #%
    // increased Damage taken to Enemies" and "Enemies Ignited by you during
    // Effect take #% increased Damage" (`poe1::flasks::FLASK_ROWS`).
    (
        "local_flask_during_effect_enemy_damage_taken_+%_on_consecrated_ground",
        &[(
            Quantity::Poe1(Poe1Quantity::ConsecratedEnemyDamageTaken),
            Op::Increased,
            1.0,
        )],
    ),
    (
        "local_flask_enemies_ignited_during_flask_effect_damage_taken_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::IgnitedEnemyDamageTaken),
            Op::Increased,
            1.0,
        )],
    ), // below" (`LowLifePercentage` BASE, a fraction: `ModCache.lua` 12888
    // P6-11ce: a flask's "When Hit during effect, #% of Life loss from Damage
    // taken occurs over 4 seconds instead" (`LifeLossPrevented` BASE,
    // `UsingFlask`; `poe1::flasks::FLASK_ROWS`).
    (
        "local_life_loss_%_to_prevent_during_flask_effect_to_lose_over_time",
        &[(
            Quantity::Poe1(Poe1Quantity::LifeLossPrevented),
            Op::Base,
            1.0,
        )],
    ), // reads 55% as 0.55). PoB also reads it for the `LowLife` condition
    // from reservation (`CalcPerform.lua` 546), which no modelled
    // modifier reads; [`super::enemy_hit`] reads it for `LifeRecoverable`.
    (
        "low_life_threshold_%_override",
        &[(
            Quantity::Poe1(Poe1Quantity::LowLifePercentage),
            Op::Base,
            0.01,
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
    // P6-11t: "#% increased Mana Regeneration Rate per 1% Chance to Block
    // Spell Damage" (`ManaRegen` INC, `PerStat` `SpellBlockChance`);
    // [`super::regen`] reads it.
    (
        "mana_regeneration_rate_+%_per_1%_spell_block_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::RegenIncPerSpellBlock(Pool::Mana)),
            Op::Base,
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
    (
        "maximum_energy_shield_increased_by_spell_block_chance",
        &[(
            Quantity::Poe1(Poe1Quantity::EnergyShieldFromSpellBlock),
            Op::Flag,
            1.0,
        )],
    ), // P6-10b part 7: "#% increased Maximum Recovery per Energy Shield Leech"
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
    // P6-11dd: "Gain #% of Maximum Life as Extra Maximum Energy Shield"
    // (`LifeGainAsEnergyShield` BASE), a row of energy shield.
    (
        "maximum_life_%_to_add_as_maximum_energy_shield",
        &[(
            Quantity::Poe1(Poe1Quantity::LifeAsEnergyShield),
            Op::Base,
            1.0,
        )],
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
    // P6-11ee: "#% increased maximum Life, Mana and Global Energy Shield"
    // (`Life`, `Mana`, `EnergyShield` INC).
    (
        "maximum_life_mana_and_energy_shield_+%",
        &[
            (Quantity::Life, Op::Increased, 1.0),
            (Quantity::Mana, Op::Increased, 1.0),
            (Quantity::EnergyShield, Op::Increased, 1.0),
        ],
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
    // P6-11ah: "Modifiers to Fire Resistance also apply to Cold and
    // Lightning Resistances at #% of their Value" (`FireResConvertToCold`
    // and `FireResConvertToLightning` BASE, `ModCache.lua` 10303) and
    // "Modifiers to Maximum Fire Resistance also apply to Maximum Cold and
    // Lightning Resistances" (no placeholder: the resolver's 1; PoB's 100,
    // 10305); [`super::resist_conversion`] applies them.
    (
        "modifiers_to_fire_resistance_also_apply_to_cold_lightning_resistance_at_%_value",
        &[
            (
                Quantity::Poe1(Poe1Quantity::ResistConvert(false, DamageType::Cold)),
                Op::Base,
                1.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::ResistConvert(false, DamageType::Lightning)),
                Op::Base,
                1.0,
            ),
        ],
    ),
    (
        "modifiers_to_maximum_fire_resistance_apply_to_maximum_cold_and_lightning_resistance",
        &[
            (
                Quantity::Poe1(Poe1Quantity::ResistConvert(true, DamageType::Cold)),
                Op::Base,
                100.0,
            ),
            (
                Quantity::Poe1(Poe1Quantity::ResistConvert(true, DamageType::Lightning)),
                Op::Base,
                100.0,
            ),
        ],
    ),
    // P6-11ac: the eldritch implicit "#% Chance to Block Attack Damage"
    // resolves to this stat; PoB reads the line and the stat as
    // `BlockChance` BASE (`ModParser.lua`, `SkillStatMap.lua` 434).
    ("monster_base_block_%", &[(BLOCK, Op::Base, 1.0)]),
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
        "physical_damage_%_to_add_as_chaos",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(
                DamageType::Physical,
                DamageType::Chaos,
            )),
            Op::Base,
            1.0,
        )],
    ),
    (
        "physical_damage_%_to_add_as_cold",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Physical, DamageType::Cold)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "physical_damage_%_to_add_as_fire",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(DamageType::Physical, DamageType::Fire)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "physical_damage_%_to_add_as_lightning",
        &[(
            Quantity::Poe1(Poe1Quantity::GainAs(
                DamageType::Physical,
                DamageType::Lightning,
            )),
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
    // P6-11ak: a recovery on block (`BlockGain`, read by `enemy_hit`).
    (
        "recover_%_of_maximum_life_on_block",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(super::BlockGain::LifePercent)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-11ak: a recovery on block (`BlockGain`, read by `enemy_hit`).
    (
        "recover_X_life_on_block",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(super::BlockGain::Life)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-11ak: a recovery on block (`BlockGain`, read by `enemy_hit`).
    (
        "recover_x_energy_shield_on_spell_block",
        &[(
            Quantity::Poe1(Poe1Quantity::BlockGain(super::BlockGain::EnergyShieldSpell)),
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
    // P6-11t: "Non-Channelling Skills have +# to Total Mana Cost"
    // (`ManaCost` BASE, `SkillType` `Channel` negated) and the channelling
    // one; [`super::costs`] reads them for a main skill of that kind.
    (
        "skill_mana_cost_+_with_channelling_skills",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaCostChannelling(true)),
            Op::Base,
            1.0,
        )],
    ),
    (
        "skill_mana_cost_+_with_non_channelling_skills",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaCostChannelling(false)),
            Op::Base,
            1.0,
        )],
    ),
    // P6-11f: "+#% Chance to Block Spell Damage per 5% Chance to Block
    // Attack Damage" (`SpellBlockChance` BASE, `PerStat` `BlockChance`
    // `div = 5`); `phases` reads it.
    (
        "spell_block_%_per_5_attack_block",
        &[(Quantity::Poe1(Poe1Quantity::BlockPer5(true)), Op::Base, 1.0)],
    ),
    // P6-11al: "Your chance to Suppress Spell Damage is Lucky" (PoB's
    // `SpellSuppressionChanceIsLucky` FLAG); `enemy_hit` reads it.
    (
        "spell_suppression_chance_is_lucky",
        &[(
            Quantity::Poe1(Poe1Quantity::SuppressionLucky),
            Op::Flag,
            1.0,
        )],
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
    // P6-11aq: "#% increased Effect of Withered" (`WitherEffect` INC; read
    // by `poe1::damage_taken` for the enemy's withered stacks).
    (
        "withered_effect_+%",
        &[(
            Quantity::Poe1(Poe1Quantity::WitherEffect),
            Op::Increased,
            1.0,
        )],
    ),
    // P6-11aq: "#% chance to inflict Withered for 2 seconds on Hit" and its
    // kin (`Condition:CanWither` FLAG, `SkillStatMap.lua` 2202 to 2210);
    // a gem's is its skill's own, read by `poe1::gems::can_wither`.
    (
        "withered_on_hit_chance_%",
        &[(Quantity::Poe1(Poe1Quantity::CanWither), Op::Flag, 1.0)],
    ),
    (
        "withered_on_hit_for_2_seconds_%_chance",
        &[(Quantity::Poe1(Poe1Quantity::CanWither), Op::Flag, 1.0)],
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
    // P6-11eb: chances to gain a charge (on a trap or mine, on a melee crit, on
    // stunning): PoB's calc never reads them; the counts are the config's.
    (
        "%_chance_to_gain_frenzy_charge_on_mine_detonated_targeting_an_enemy",
        "charge-gain-chance",
    ),
    (
        "%_chance_to_gain_frenzy_charge_on_trap_triggered_by_an_enemy",
        "charge-gain-chance",
    ),
    (
        "%_chance_to_gain_power_charge_on_mine_detonated_targeting_an_enemy",
        "charge-gain-chance",
    ),
    (
        "%_chance_to_gain_power_charge_on_trap_triggered_by_an_enemy",
        "charge-gain-chance",
    ),
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
    // P6-11do: Power Charge On Critical's chance to gain a power charge on crit
    // (PoB's calc never reads it; the count is the config's).
    (
        "add_power_charge_on_critical_strike_%",
        "charge-gain-chance",
    ),
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
    // P6-11ds: Hextouch's chance to apply the linked hex on hit (PoB: display only;
    // the hex's own support, `SupportCurseOnHitCurse`, applies it).
    ("apply_linked_curses_on_hit_%", "trigger"),
    // P6-11as: "#% increased Area of Effect per Endurance Charge"
    // (`AreaOfEffect` INC per `EnduranceCharge`): area, as
    // `base_skill_area_of_effect_+%`.
    ("area_of_effect_+%_per_endurance_charge", "area"),
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
    // P6-11at: "Herald Skills have #% increased Area of Effect"
    // (`AreaOfEffect` INC, `SkillType.Herald`): area.
    ("area_of_effect_+%_with_herald_skills", "area"),
    // P6-11ba: chances to blind, taunt, or hinder on hit (yours and your
    // minions'), the Abyss jewels' additions: PoB parses none of these lines,
    // and the enemy is Blinded only by the config's condition.
    ("attacks_chance_to_blind_on_hit_%", "on-hit-debuff"),
    ("attacks_chance_to_taunt_on_hit_%", "on-hit-debuff"),
    (
        "attacks_number_of_additional_projectiles",
        "projectile-count",
    ),
    ("avoid_ailments_%_from_crit", "ailment-on-self"),
    // P6-11f: "#% chance to Avoid Elemental Ailments" (`AvoidElementalAilments` BASE): no slice field.
    ("avoid_all_elemental_status_%", "ailment-on-self"),
    ("avoid_chained_projectile_%_chance", "avoidance"),
    ("avoid_interruption_while_casting_%", "interruption"),
    ("avoid_maim_%_chance", "ailment-on-self"),
    // P6-11eb: lines no slice field reads: knockback chance, frenzy charge
    // duration, mana on kill, chill duration on enemies (the chill's effect is
    // its own), pierce and chaining range (unparsed in PoB), the atlas's and a
    // mercenary's.
    ("azmeri_warden_wisps_found_+%", "no-slice"),
    // P6-11dz: "#% increased Duration of Ailments on You": ailments on you, no slice
    // field.
    ("base_all_ailment_duration_on_self_+%", "ailment-on-self"),
    ("base_aura_area_of_effect_+%", "area"),
    // P6-10a: "#% chance to Avoid Bleeding" (`AvoidBleed` BASE) and
    // "#% chance to Avoid being Poisoned" (`AvoidPoison` BASE), Glorious
    // Vanity conquests: PoB's ailment avoidance, no slice field.
    ("base_avoid_bleed_%", "ailment-on-self"),
    // P6-11as: "#% chance to Avoid being Chilled": chill on you.
    ("base_avoid_chill_%", "ailment-on-self"),
    ("base_avoid_freeze_%", "ailment-on-self"),
    // P6-11ay: "#% chance to Avoid being Ignited" (`AvoidIgnite` BASE), an
    // Abyss jewel's addition: no slice field.
    ("base_avoid_ignite_%", "ailment-on-self"),
    ("base_avoid_poison_%", "ailment-on-self"),
    // P6-11ay: "#% chance to Avoid being Shocked" (`AvoidShock` BASE): no
    // slice field (the enemy's shock is the config's).
    ("base_avoid_shock_%", "ailment-on-self"),
    // P6-11g: "#% chance to Avoid being Stunned" (`AvoidStun` BASE).
    ("base_avoid_stun_%", "stun"),
    // P6-11eb: immunities on you (`ChillImmune`, `FreezeImmune`, `StunImmune`):
    // no slice field.
    ("base_cannot_be_chilled", "ailment-on-self"),
    ("base_cannot_be_frozen", "ailment-on-self"),
    ("base_cannot_be_stunned", "ailment-on-self"),
    ("base_chance_to_freeze_%", "freeze"),
    // P3-8n: a passive's "#% chance to Shock" (`EnemyShockChance` BASE): as
    // the skill's chance below, never the enemy's shock in the hit.
    ("base_chance_to_shock_%", "ailment-chance"),
    // A skill's chance to shock: PoB shocks the enemy only from its config
    // or a guaranteed source (`ShockBase`, `ShockOverride`,
    // `ShockMinimum`; `Modules/CalcPerform.lua`, the non-damaging
    // ailments), never from a chance (P3-10, Smite).
    ("base_chance_to_shock_%_from_skill", "ailment-chance"),
    ("base_charge_duration_+%", "charge-duration"),
    ("base_frenzy_charge_duration_+%", "no-slice"),
    ("base_global_chance_to_knockback_%", "no-slice"),
    // A skill marked a projectile skill (`base_is_projectile`): the skill's
    // own type (`Projectile`) says it; PoB maps no row for it (P6-1).
    ("base_is_projectile", "skill type marker"),
    // P6-11g: "#% increased Rarity of Items found" (`LootRarity` INC).
    ("base_item_found_rarity_+%", "item-rarity"),
    // P6-11g: "Gain # Life per Enemy Hit with Attacks" (`LifeOnHit` BASE): no recovery field.
    ("base_life_gain_per_target", "recovery"),
    // P6-11g: "Gain # Life per Enemy Killed" (`LifeOnKill` BASE).
    ("base_life_gained_on_enemy_death", "recovery"),
    ("base_mana_gained_on_enemy_death", "no-slice"),
    // P6-11m: minion duration (`Duration` INC for minion skills): no slice
    // field reads a minion's duration.
    ("base_minion_duration_+%", "skill-duration"),
    // P3-8n: "Maximum # Summoned Golems" (`ActiveGolemLimit`): only golem
    // skills (no golem gem is modelled, `poe1::gems`) and modifiers per
    // summoned golem read it ([`NO_EFFECT_DEPENDS_ON`]).
    ("base_number_of_golems_allowed", "golems"),
    ("base_number_of_projectiles", "projectile-count"),
    // P6-11m: "+# to maximum number of Skeletons" (`ActiveSkeletonLimit`).
    ("base_number_of_skeletons_allowed", "minion-count"),
    // P6-11cd: "+# to maximum number of Spectres" (`ActiveSpectreLimit`): a count
    // no slice field reads, as the zombies' and skeletons'.
    ("base_number_of_spectres_allowed", "minion-count"),
    // P6-11m: "+# to maximum number of Raised Zombies" (`ActiveZombieLimit`): a count no slice field reads.
    ("base_number_of_zombies_allowed", "minion-count"),
    // P6-11f: "#% increased Projectile Speed" (`ProjectileSpeed` INC): no slice field.
    ("base_projectile_speed_+%", "projectile-speed"),
    ("base_secondary_skill_effect_duration", "skill-duration"),
    // P6-11em: ignite duration on you and the chance to be frozen, shocked,
    // and ignited (PoB parses neither): no slice field.
    ("base_self_ignite_duration_-%", "ailment-on-self"),
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
    // P6-11at: "Supported Skills have #% reduced Enemy Stun Threshold"
    // (`EnemyStunThreshold` INC, `mult = -1`): stun, no slice field.
    ("base_stun_threshold_reduction_+%", "stun"),
    ("base_totem_duration", "totem-duration"),
    // P6-11eh: lines no slice field reads: totems' range and placement speed
    // (`TotemPlacementSpeed`, PoB's own output only), stun avoidance on kill,
    // a rare monster's mods on kill, blight towers, strike targeting distance,
    // a projectile's contact position, reflected cold damage, totems' taunt.
    ("base_totem_range", "no-slice"),
    ("base_trap_duration", "skill-duration"),
    ("blight_scout_tower_range_+%", "no-slice"),
    // P6-11f: "#% increased Block Recovery" (`BlockRecovery` INC): no slice field.
    ("block_recovery_+%", "block-recovery"),
    ("can_have_active_mercenary_follower", "no-slice"),
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
    // P6-11br: stun immunity while using a chaos skill (PoB does not parse it).
    ("cannot_be_stunned_while_using_chaos_skill", "stun"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("cannot_cancel_skill_before_contact_point", "display"),
    (
        "cannot_gain_corrupted_blood_while_you_have_at_least_5_stacks",
        "ailment-on-self",
    ),
    ("cannot_have_life_leeched_from", "leech"),
    ("cannot_pierce", "no-slice"),
    // P6-11eg: lines no slice field reads: taunt duration, Desecrate's and
    // Unearth's corpses, totems' life and resistances, attribute requirements,
    // reflected damage taken.
    ("cannot_take_reflected_elemental_damage", "no-slice"),
    ("cannot_take_reflected_physical_damage", "no-slice"),
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
    // P6-11ea: skills' radius stats (PoB's `radius` skill data and area of
    // effect): area, no slice field.
    (
        "caustic_arrow_explode_on_hit_base_area_of_effect_radius",
        "area",
    ),
    ("chaining_range_+%", "no-slice"),
    ("chance_to_be_frozen_shocked_ignited_%", "ailment-on-self"),
    (
        "chance_to_create_consecrated_ground_on_melee_kill_%",
        "ground-creation-chance",
    ),
    // P6-11eb: "Projectiles have #% chance for an additional Projectile when
    // Forking" (PoB: `ProjectileCount` BASE): as the projectile count, read
    // only with `SequentialProjectiles` (Barrage's, a guard).
    ("chance_to_fork_extra_projectile_%", "projectile-count"),
    ("chance_to_fortify_on_melee_hit_+%", "fortification"),
    // P6-11eg: chances to gain a charge on block and on a non-critical strike
    // (PoB does not parse them; the counts are the config's).
    (
        "chance_to_gain_endurance_charge_on_block_%",
        "charge-gain-chance",
    ),
    (
        "chance_to_gain_endurance_charge_on_melee_crit_%",
        "charge-gain-chance",
    ),
    // P6-11m: a chance to gain an endurance charge when hit (unparsed).
    ("chance_to_gain_endurance_charge_when_hit_%", "charge-gain"),
    (
        "chance_to_gain_endurance_charge_when_you_stun_enemy_%",
        "charge-gain-chance",
    ),
    (
        "chance_to_gain_power_charge_when_block_%",
        "charge-gain-chance",
    ),
    // P6-11at: "#% chance to Unnerve Enemies for 4 seconds on Hit": PoB
    // reads no modifier from it (no stat map row; Unnerve is the config's
    // `conditionEnemyUnnerved`, `poe1::damage_taken`).
    ("chance_to_unnerve_on_hit_%", "ailment-chance"),
    // P6-11u: the charges' count is the config's (`poe1::charges`).
    ("charge_duration_+%", "charge-duration"),
    // P6-11eg: flask charges gained, used, and generated (`FlaskChargesGained`,
    // `FlaskChargesUsed` INC, `FlaskChargesGenerated` BASE): no slice field
    // reads a flask's charges.
    ("charges_gained_+%", "flask"),
    ("chill_duration_+%", "no-slice"),
    ("chill_effectiveness_on_self_+%", "ailment-on-self"),
    ("cold_ailment_duration_+%", "ailment-duration"),
    ("cold_damage_to_return_to_melee_attacker", "no-slice"),
    (
        "consecrated_ground_allies_recover_es_as_well_as_life_from_life_regeneration",
        "regeneration",
    ),
    ("console_skill_dont_chase", "display"),
    ("curse_effect_on_self_+%", "curse-on-self"),
    // P6-11dl: Cyclone's area per additional melee range (`AreaOfEffect` INC per
    // `AdditionalMeleeRange`): area, no slice field.
    (
        "cyclone_area_of_effect_+%_per_additional_melee_range",
        "area",
    ),
    // P6-11cf: "Damage cannot be Reflected" (PoB: reflected damage taken, which no
    // slice field reads).
    ("damage_cannot_be_reflected", "reflect"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    (
        "damage_cannot_be_reflected_or_leech_if_used_by_other_object",
        "display",
    ),
    ("debuff_time_passed_+%", "ailment-on-self"),
    ("desecrate_maximum_number_of_corpses", "no-slice"),
    // P6-11cy: a support that cannot support melee attacks (which supports apply is the skill's types').
    ("disable_skill_if_melee_attack", "skill type marker"),
    // P3-8n: Intensify's text; PoB maps no modifier.
    ("display_base_intensity_loss", "display"),
    // P6-11eh: Blight's stack limit shown on the item.
    ("display_max_blight_stacks", "display"),
    // P6-11ed: display-only lines: gem experience (unparsed in PoB), extra gore,
    // ignite's visual variant, a dummy display stat.
    ("dummy_stat_display_nothing", "display"),
    ("earthquake_radius_+%", "area"),
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
    ("enchantment_boots_stun_avoid_%_on_kill", "no-slice"),
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
    // P6-11eh: "Recover #% of Energy Shield when you Block" (PoB does not
    // parse it): no recovery field reads a block.
    ("energy_shield_%_gained_on_block", "recovery"),
    // P6-11be: "Cursed Enemies you or your Minions Kill have a #% chance
    // to Explode" (PoB's `ExplodeMod`): read only by PoB's own explosion
    // skill (`EnemyExplode`), which reaches a field only as the main skill or
    // in full DPS, and which no modelled build includes (tasks file).
    (
        "explode_cursed_enemies_for_25%_life_as_chaos_on_kill_chance_%",
        "explode",
    ),
    // P6-11dj: on-kill explosions (PoB's `ExplodeMod` LIST and `CanExplode`): read
    // only by PoB's own "On Kill Monster Explosion" skill (`EnemyExplode`, a
    // group PoB adds with source "Explode"), which no game gem grants: the
    // gem table skips a group with a source (`poe1::gems`), and the gem is
    // `unmodelled-gem` as a main skill or in full DPS.
    (
        "explode_enemies_for_500%_life_as_fire_on_kill_%_chance",
        "explosion",
    ),
    ("extra_gore", "display"),
    ("extra_target_targeting_distance_+%", "no-slice"),
    // P6-11dj: reflected fire damage to melee attackers (unparsed in PoB; damage
    // to attackers, no slice field), as the physical one.
    ("fire_damage_to_return_to_melee_attacker", "reflect"),
    ("flask_charges_recovered_per_3_seconds", "flask"),
    ("flask_charges_used_+%", "flask"),
    // P6-11n: "#% increased Life Recovered" by flasks (`FlaskLifeRecovery`
    // INC): no field.
    ("flask_life_to_recover_+%", "flask"),
    // P6-11m: "#% increased Flask Recovery rate" (`FlaskRecoveryRate`).
    ("flask_recovery_speed_+%", "flask"),
    // P6-11eh: "#% increased Projectile Fork angle": PoB parses it as
    // `ProjectileCount` INC (its text's "Projectile"), which no slice field
    // reads either.
    ("forking_angle_+%", "projectile-count"),
    ("fortify_duration_+%", "fortification"),
    ("freeze_duration_+%", "ailment-duration"),
    // P6-11ad: "You count as on Full Life while at #% of maximum Life or
    // above" (`FullLifePercentage` BASE): PoB reads it only for the config
    // option's tooltip (`CalcPerform.lua` 76, `ConfigOptions.lua` 68).
    ("full_life_threshold_%_override", "life-threshold"),
    // P6-11as: "Gain # Life when you Stun an Enemy": PoB leaves the line
    // unparsed (`ModCache.lua`), and a gain on stun reaches no slice field.
    ("gain_X_life_on_stun", "recovery"),
    // P6-11m: endurance charges per second if hit recently (unparsed).
    (
        "gain_endurance_charge_per_second_if_have_been_hit_recently",
        "charge-gain",
    ),
    // P6-11eh: chances to gain a charge on a curse, on hitting a unique, rare,
    // or marked enemy, on blocking a spell, and to lose a frenzy charge on a
    // travel skill (PoB parses none; the counts are the config's).
    (
        "gain_frenzy_charge_on_hitting_marked_enemy_%",
        "charge-gain-chance",
    ),
    (
        "gain_frenzy_charge_on_hitting_rare_or_unique_enemy_%",
        "charge-gain-chance",
    ),
    (
        "gain_frenzy_charge_on_hitting_unique_enemy_%",
        "charge-gain-chance",
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
    ("gain_power_charge_on_curse_cast_%", "charge-gain-chance"),
    (
        "gain_power_charge_on_non_critical_strike_%",
        "charge-gain-chance",
    ),
    ("gain_rare_monster_mods_on_kill_ms", "no-slice"),
    // P3-8n: Inspiration's charge gain; PoB maps no modifier (its charges are
    // the maximum unless the config overrides them, `poe1::stacks`).
    ("gain_righteous_charge_on_mana_spent_%", "charge-gain"),
    ("gem_experience_gain_+%", "display"),
    ("global_chance_to_blind_on_hit_%", "on-hit-debuff"),
    ("global_item_attribute_requirements_+%", "no-slice"),
    ("gloves_freeze_proliferation_radius", "freeze"),
    // P6-11g: "Ignites you inflict spread to other Enemies within # metre": PoB does not parse it.
    ("gloves_ignite_proliferation_radius", "area"),
    // P6-7b3b: "Golems are Immune to Elemental Damage": PoB's parser does
    // not read the line (no mod), so it reaches none of PoB's outputs; no
    // minion field reads damage taken.
    ("golem_immunity_to_elemental_damage", "minion-defence"),
    // P6-11bc: "Modifiers to Chance to Avoid being Shocked apply to all
    // Elemental Ailments" (PoB's `ShockAvoidAppliesToElementalAilments`
    // flag): ailments on you, no slice field.
    (
        "has_avoid_shock_as_avoid_all_elemental_ailments",
        "ailment-on-self",
    ),
    ("ignite_art_variation", "display"),
    // P6-11be: "Your Hexes can affect Hexproof Enemies"
    // (`CursesIgnoreHexproof`): no input or modelled modifier makes the enemy
    // hexproof, so curses already apply.
    ("ignore_hexproof", "curse"),
    // P3-close: "#% increased Effect of Buffs granted by your Golems" left this
    // list for `poe1::skill_modifiers` (the golem gems are modelled).
    // P3-8n: the stats that add damage from a mana cost stay unmapped
    // ([`NO_EFFECT_DEPENDS_ON`]); the costs are rows since P6-10b.
    ("item_generation_can_have_multiple_crafted_mods", "crafting"),
    ("kinetic_fusillade_subsequent_targeting_radius", "area"),
    // P6-11eg: item properties: the sell price, modification while corrupted,
    // the implicit limit, and a wand base's crafting rule.
    (
        "kinetic_wand_implicit_cannot_roll_caster_modifiers",
        "display",
    ),
    // P6-11g: Divine Shield (`Keystone` LIST): energy shield recovery and recoup, no slice field.
    // P6-11g: Mind Over Matter (`Keystone` LIST): damage taken from mana, no slice field.
    (
        "life_flasks_gain_X_charges_every_3_seconds_if_you_have_not_used_a_life_flask_recently",
        "flask",
    ),
    // P6-11cz: life flask charges on suppressing a spell (PoB leaves the text unparsed; flask charges reach no slice field).
    ("life_flasks_gain_X_charges_on_suppressing_spell", "flask"),
    ("life_recovery_+%_from_flasks_while_on_low_life", "flask"),
    ("light_radius_+%", "light-radius"),
    ("lightning_ailment_duration_+%", "ailment-duration"),
    ("lightning_arrow_radius_+%", "area"),
    // P6-11ee: lines no slice field reads: an item's attribute requirements,
    // flask automation and on-use effects, maim's effect, a mercenary's
    // unique slots, a corpse's life, minions' taunt and defensive stance.
    ("local_attribute_requirements_+%", "no-slice"),
    // P6-11dj: "Can be Anointed": an item property PoB reads only to offer the
    // anoint; the anoint's passive is the item's own line (P3-8n).
    ("local_can_be_anointed", "display"),
    // "Gems Socketed always have the Quality bonus from Socket Colour":
    // its effect is the gems' quality, `poe1::sockets` (P3-12i).
    ("local_can_socket_gems_ignoring_colour", "sockets"),
    // P6-11m: a flask's amount recovered (`FlaskRecovery`): no slice field.
    ("local_flask_amount_to_recover_+%", "flask"),
    // P6-11bz: the radius of a flask's consecrated ground (PoB does not parse it).
    ("local_flask_area_of_consecrated_ground_+%", "area"),
    // P6-11br: whether magic utility flask effects can be removed, and
    // flask charges during the effect (PoB does not parse them; the effects
    // apply while the flask is on, as the config sets).
    (
        "local_flask_cannot_gain_charges_during_flask_effect",
        "flask",
    ),
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
    // P6-11em: a flask's stun recovery while healing (PoB does not parse it).
    ("local_flask_stun_recovery_+%_while_healing", "stun"),
    // P6-11m: a flask's taunt on use (unparsed).
    ("local_flask_taunt_enemies_on_use_radius", "flask"),
    ("local_flask_use_causes_area_knockback", "no-slice"),
    ("local_flask_use_causes_monster_flee_chance_%", "no-slice"),
    ("local_flask_use_on_adjacent_flask_use", "no-slice"),
    ("local_flask_use_on_flask_effect_ended", "no-slice"),
    (
        "local_flask_use_on_hitting_rare_unique_enemy_while_inactive",
        "no-slice",
    ),
    // P6-11br: gem experience (PoB does not parse it).
    ("local_gem_experience_gain_+%", "gem-experience"),
    // Its effect on the sockets' order is `poe1::sockets`' (a reason).
    ("local_has_X_abyss_sockets", "sockets"),
    // P6-11g: "Has # Sockets" (`SocketCount` BASE): the item's sockets come from its text.
    ("local_has_X_sockets", "sockets"),
    ("local_intelligence_requirement_+", "no-slice"),
    ("local_item_allow_modification_while_corrupted", "display"),
    ("local_item_implicit_modifier_limit", "display"),
    ("local_item_sell_price_doubled", "display"),
    // P6-11ea: radius jewels' allocation stats (Intuitive Leap, Thread of Hope's
    // ring, Impossible Escape): the pipeline reads them when it resolves the
    // passives (`loom_pipeline`, `RING_STAT`, `LEAP_STAT`, `ESCAPE_STAT`); they
    // act through the allocated nodes, not as modifiers.
    ("local_jewel_variable_ring_radius_value", "tree-allocation"),
    // P6-11m: a flask charge on crit (`FlaskChargeOnCritChance`).
    ("local_recharge_on_crit_%", "flask"),
    // P6-8a: the gems a trigger support supports; PoB has no row.
    (
        "local_support_gem_max_skill_level_requirement_to_support",
        "support-limit",
    ),
    (
        "local_unique_jewel_disconnected_passives_can_be_allocated_around_keystone_hash",
        "tree-allocation",
    ),
    (
        "local_unique_jewel_nearby_disconnected_keystone_passives_can_be_allocated",
        "tree-allocation",
    ),
    (
        "local_unique_jewel_nearby_disconnected_passives_can_be_allocated",
        "tree-allocation",
    ),
    (
        "lose_a_frenzy_charge_on_travel_skill_use_%_chance",
        "charge-gain-chance",
    ),
    // P6-11em: "#% chance to lose a Power Charge when you gain Elusive" (PoB
    // parses it to nothing; the count is the config's).
    (
        "lose_a_power_charge_when_you_gain_elusive_%_chance",
        "charge-gain-chance",
    ),
    (
        "lose_all_righteous_charges_on_mana_use_threshold",
        "charge-gain",
    ),
    ("magic_utility_flasks_cannot_be_removed", "flask"),
    ("maim_effect_+%", "no-slice"),
    // P6-11g: "Gain # Mana per Enemy Hit with Attacks" (`ManaOnHit` BASE).
    ("mana_gain_per_target", "recovery"),
    // P6-11g: "Gain # Mana per Enemy Hit with Attacks if you've used a Mana Flask in the past 10 seconds" (`ManaOnHit` BASE).
    (
        "mana_gained_on_attack_hit_if_used_mana_flask_in_past_10_seconds",
        "recovery",
    ),
    ("maximum_life_+%_for_corpses_you_create", "no-slice"),
    // P6-11be: "Reflects # to # Lightning Damage to Melee Attackers" (PoB
    // parses none of it): damage dealt back, no slice field.
    (
        "maximum_lightning_damage_to_return_to_melee_attacker",
        "reflect",
    ),
    // P3-8n: "Melee Strikes target # additional nearby Enemies" (PoB's strike
    // targets, `CalcOffence.lua` 1147): not the hit.
    ("melee_attack_number_of_spirit_strikes", "targeting"),
    // P6-11f: "+# metres to Melee Strike Range" (`MeleeWeaponRangeMetre`, `UnarmedRangeMetre`): no slice field.
    ("melee_range_+", "range"),
    // P6-11ef: area of effect for melee skills, warcries, while wielding a two
    // handed melee weapon, and per power charge (`AreaOfEffect` INC): no slice
    // field reads area.
    ("melee_skills_area_of_effect_+%", "area"),
    ("mercenary_can_use_unique_amulet", "no-slice"),
    ("mercenary_can_use_unique_belt", "no-slice"),
    ("mercenary_can_use_unique_gloves", "no-slice"),
    ("mercenary_can_use_unique_helmet", "no-slice"),
    ("mine_duration_+%", "skill-duration"),
    // P6-11cy: the trap and mine supports' player-versus-player scaling.
    ("mine_override_pvp_scaling_time_ms", "pvp"),
    (
        "minimum_lightning_damage_to_return_to_melee_attacker",
        "reflect",
    ),
    ("minion_attacks_chance_to_blind_on_hit_%", "on-hit-debuff"),
    ("minion_attacks_chance_to_taunt_on_hit_%", "on-hit-debuff"),
    ("minion_chance_to_taunt_on_hit_%", "no-slice"),
    // P6-11bd: "Minions are Aggressive": PoB parses none of it, and no
    // slice field reads a minion's aggression.
    ("minion_larger_aggro_radius", "minion-behaviour"),
    // P6-11dr: "Minions Leech #% of Damage as Life" (a `MinionModifier` of
    // `DamageLifeLeech`): no slice field reads a minion's leech.
    (
        "minion_life_leech_from_any_damage_permyriad",
        "minion-leech",
    ),
    // P6-11be: "Minions Recover #% of Life on Minion Death" (PoB parses
    // none of it): a recovery on an event, no slice field.
    (
        "minion_recover_%_maximum_life_on_minion_death",
        "recovery-event",
    ),
    // P6-11bc: "Minions have #% increased Area of Effect" (the minion's
    // `AreaOfEffect` INC): area, as `base_skill_area_of_effect_+%` (the
    // minion's radius outputs are no slice field either).
    ("minion_skill_area_of_effect_+%", "area"),
    ("minion_spells_chance_to_hinder_on_hit_%", "on-hit-debuff"),
    ("minions_are_defensive", "no-slice"),
    // P6-7c3: a monster skill's penalty against the player's minions (the
    // Flame Golem's Immolate): PoB's stat map has no row for it.
    (
        "monster_penalty_against_minions_damage_+%_final_vs_player_minions",
        "monster",
    ),
    // P6-11dl: "Projectile Barrages have no spread" (unparsed in PoB): the
    // projectiles' geometry, no slice field.
    ("no_barrage_projectile_spread", "projectile-spread"),
    // Bleeding on you while moving (`NoExtraBleedDamageWhileMoving`): only
    // PoB's damage over time estimate reads it, no field here.
    ("no_extra_bleed_damage_while_moving", "damage-taken"),
    // P6-11dg: "Bow Attacks fire an additional Arrow" (PoB's `ProjectileCount` BASE with
    // `ModFlag.Bow`): as the projectile count, read only with
    // `SequentialProjectiles` (Barrage's `projectiles_barrage`, a guard).
    ("number_of_additional_arrows", "projectile-count"),
    // P6-11u: the projectile count (PoB `ProjectileCount`): it reaches DPS
    // only with `SequentialProjectiles` (`CalcOffence.lua` 769 to 772), a
    // skill base mod (unvetted) or Barrage's `projectiles_barrage`, a guard.
    ("number_of_additional_projectiles", "projectile-count"),
    // P6-11ef: the trap and mine limits (`ActiveTrapLimit`, `ActiveMineLimit`
    // BASE): PoB shows them and its "for each trap" and "for each mine" lines
    // read them; no slice field does.
    ("number_of_additional_remote_mines_allowed", "no-slice"),
    ("number_of_additional_traps_allowed", "no-slice"),
    // P6-11u: the chain count (PoB `ChainCountMax`): read by the per-chain
    // lines, each a guard.
    ("number_of_chains", "chain-count"),
    // P6-11bd: stun on enemies with two handed melee weapons, and Occultist's
    // "Cannot Be Stunned while you have Energy Shield": no slice field.
    ("occultist_immune_to_stun_while_has_energy_shield", "stun"),
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
    ("power_charge_on_block_%_chance", "charge-gain-chance"),
    ("power_charge_on_spell_block_%_chance", "charge-gain-chance"),
    // P6-11m: pierce (`PierceCount`), as `always_pierce`.
    ("projectile_base_number_of_targets_to_pierce", "pierce"),
    ("projectile_uses_contact_position", "no-slice"),
    ("quality_display_active_skill_area_damage_is_gem", "display"),
    (
        "quality_display_active_skill_area_damage_quality_negated_from_gem",
        "display",
    ),
    ("rage_loss_delay_ms", "rage-duration"),
    // P6-11br: projectile directions (PoB does not parse it).
    ("random_projectile_direction", "targeting"),
    // P6-11em: "Recover #% of Energy Shield over 1 second when you take
    // Physical Damage from an Enemy Hit" (PoB does not parse it): no recovery
    // field reads a hit taken.
    (
        "recover_%_energy_shield_over_1_second_when_you_take_physical_damage_from_enemy_hits",
        "recovery",
    ),
    // P6-11g: "Recover #% of Life on Kill" (`LifeOnKill` BASE per life).
    ("recover_%_maximum_life_on_kill", "recovery"),
    // P6-11eg: "Recover #% of Mana on Kill" and "... of Energy Shield on Kill"
    // (`ManaOnKill`, `EnergyShieldOnKill` BASE): no recovery field reads a kill.
    ("recover_%_maximum_mana_on_kill", "recovery"),
    ("recover_energy_shield_%_on_kill", "recovery"),
    ("reflect_hexes_chance_%", "curse-on-self"),
    // P6-11aj: Multistrike's repeats choose nearby targets.
    (
        "repeat_target_selection_distance_from_initial_target_bias",
        "targeting",
    ),
    ("revive_golems_if_killed_by_enemies_ms", "minion-defence"),
    ("shocked_effect_on_self_+%", "ailment-on-self"),
    // P6-6a (the skill sweep): a skill's constant stat no stat map reads
    // (neither `SkillStatMap.lua` nor a skill's own `statMap`) and no
    // module names: PoB reads it into no output.
    ("show_number_of_projectiles", "display"),
    // P6-11u: brands are not computed (`hit`).
    ("sigil_duration_+%", "skill-duration"),
    ("skill_area_of_effect_+%_per_power_charge", "area"),
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
    // P6-11u: a skill effect duration (PoB `Duration`): read only by
    // skill functions, base mods, stages, brands, and ailments that are
    // skill effects, each a reason (`hit::pob_vetted`, `poe1::ailment`).
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
    ("spells_chance_to_hinder_on_hit_%", "on-hit-debuff"),
    // P6-11as: "#% increased Stun Duration on Enemies per Endurance
    // Charge" (`EnemyStunDuration` INC): no slice field.
    ("stun_duration_+%_per_endurance_charge", "stun"),
    // P6-11f: "#% increased Stun Threshold" (`StunThreshold` INC): no slice field.
    ("stun_threshold_+%", "stun"),
    // P6-8b: read by `poe1::combined` (the mirage count).
    ("summon_mirage_archer_on_hit", "mirage"),
    ("summon_totem_cast_speed_+%", "no-slice"),
    // P3-8n: Arcane Surge Support's duration and threshold (PoB maps neither)
    // and its mana regeneration (`ArcaneSurgeManaRegen` MAX: mana
    // regeneration); its cast speed and spell damage are `poe1::buffs`.
    ("support_arcane_surge_base_duration_ms", "skill-duration"),
    (
        "support_arcane_surge_gain_buff_on_mana_use_threshold",
        "buff-gain",
    ),
    // P6-11aj: Greater Spell Echo's area per repeat (`RepeatPerRepeatAreaOfEffect`): the area, no slice field.
    (
        "support_greater_spell_echo_area_of_effect_+%_per_repeat",
        "area",
    ),
    ("support_makes_skill_mine_pvp_damage_+%_final", "pvp"),
    ("support_makes_skill_trap_pvp_damage_+%_final", "pvp"),
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
    // P6-11bt: Ruthless's stun duration (`baseStunDuration`).
    (
        "support_ruthless_big_hit_stun_base_duration_override_ms",
        "stun",
    ),
    // P3-8n: Intensify's area per intensity (`AreaOfEffect` MORE): area.
    (
        "support_spell_boost_area_of_effect_+%_final_per_charge",
        "area",
    ),
    // P6-11cj: Empower's "Supported Skills have +# to Level": the active gem's
    // level, read by `skill::supported_level`, not a modifier.
    ("supported_active_skill_gem_level_+", "gem-level"),
    // P6-11ck: Enhance's "Supported Skills have +#% to Quality": the active
    // gem's quality, read by `skill::supported_level`.
    ("supported_active_skill_gem_quality_%", "gem-level"),
    // P6-11cl: the awakened supports' "Supported <type> Skills have +# to Level":
    // the active gem's level for a skill of that type, read by
    // `skill::supported_level` (`SUPPORTED_TYPED_LEVEL_STATS`).
    ("supported_aura_skill_gem_level_+", "gem-level"),
    ("supported_chaos_skill_gem_level_+", "gem-level"),
    ("supported_cold_skill_gem_level_+", "gem-level"),
    ("supported_curse_skill_gem_level_+", "gem-level"),
    ("supported_elemental_skill_gem_level_+", "gem-level"),
    ("supported_fire_skill_gem_level_+", "gem-level"),
    ("supported_lightning_skill_gem_level_+", "gem-level"),
    ("supported_minion_skill_gem_level_+", "gem-level"),
    ("supported_physical_skill_gem_level_+", "gem-level"),
    ("supported_strike_skill_gem_level_+", "gem-level"),
    ("taunt_duration_+%", "no-slice"),
    // P6-11u: how long a totem lasts; no slice field.
    ("totem_duration_+%", "totem-duration"),
    ("totem_elemental_resistance_%", "no-slice"),
    ("totem_life_+%", "no-slice"),
    ("totem_placement_range_+%", "no-slice"),
    (
        "totems_taunt_enemies_around_them_for_x_seconds_when_summoned",
        "no-slice",
    ),
    ("trap_trigger_radius_+%", "area"),
    // P6-11da: a trap skill's visual variant (PoB's skill data lists it with no stat map entry).
    ("trap_variation", "display"),
    // P6-8a part 2: Cast on Ward Break's chance; a global trigger's rate is
    // its cap (`CalcTriggers.lua` 793 to 804).
    ("trigger_on_ward_break_%_chance", "trigger"),
    // P6-8a: PvP scaling; PoB has no row.
    (
        "triggered_skill_uses_main_hand_or_averaged_attack_time_for_pvp_scaling",
        "pvp",
    ),
    ("two_handed_melee_area_of_effect_+%", "area"),
    ("two_handed_melee_stun_duration_+%", "stun"),
    ("unaffected_by_burning_ground", "ground-on-self"),
    // P6-11n: "Unaffected by Ignite" (`SelfIgniteEffect` MORE -100): ignite on
    // you, no field.
    ("unaffected_by_ignite", "ailment-on-self"),
    ("visual_hit_effect_elemental_is_holy", "display"),
    (
        "wand_attacks_fire_an_additional_projectile",
        "projectile-count",
    ),
    ("warcry_skill_area_of_effect_+%", "area"),
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
/// row)`, sorted by stat, then row (a stat may guard several rows). A row's `NoEffect` holds only while these stay
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
    // P6-11u: projectiles counted as chains.
    (
        "modifiers_to_number_of_projectiles_instead_apply_to_chaining",
        "number_of_additional_projectiles",
    ),
    // P6-11u: PoB's `PerStat` `Chain` and `ChainRemaining`.
    ("projectile_damage_+%_per_chain", "number_of_chains"),
    (
        "projectile_damage_+%_per_remaining_chain",
        "number_of_chains",
    ),
    (
        "projectile_non_chaos_damage_to_add_as_chaos_damage_%_per_chain",
        "number_of_chains",
    ),
    // P6-11dg: Barrage's, for the bows' additional arrows.
    ("projectiles_barrage", "chance_to_fork_extra_projectile_%"),
    ("projectiles_barrage", "number_of_additional_arrows"),
    // P6-11u: Barrage's `SequentialProjectiles`.
    ("projectiles_barrage", "number_of_additional_projectiles"),
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
    // P6-10a: a Timeless jewel's type, seed, and conqueror: read by
    // `loom_pipeline::timeless`, it acts through the nodes it conquers
    // (each a reason without the results); "Historic" is its display.
    (
        "local_unique_jewel_alternate_tree_",
        "timeless jewel: acts through its conquered nodes",
    ),
    ("local_is_alternate_tree_jewel", "timeless jewel: Historic"),
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
/// each maps to [`Poe1Quantity::PoolReason`]. PoB's text: half of area
/// damage taken. (Mind over Matter is a keystone row since P6-10b part
/// 15, Divine Shield since P6-11am.)
pub const POOL_REASONS: &[&str] = &["take_half_area_damage_from_hit_%_chance"];

/// Stats that change the effective health pool alone in a way the engine
/// does not model (P6-10b part 14), sorted: each maps to
/// [`Poe1Quantity::EhpReason`]. PoB's text: P6-11n: damage taken recouped
/// as mana (`ManaRecoup`). Lucky spell suppression is a row since P6-11al. The gains on
/// block are [`Poe1Quantity::BlockGain`] rows since P6-11ak.
///
/// P6-11aw, ward (Heroic Tragedy's conquests): increased ward and ward
/// from armour items (`Ward` INC), which PoB's effective health pool reads
/// through `output.Ward` (`CalcDefence.lua` 223); the engine has no ward
/// pool. "#% chance to Restore Ward when you Block" PoB reads only in part,
/// so applies none of it.
pub const EHP_REASONS: &[&str] = &[
    "restore_ward_on_block_%",
    "ward_+%",
    "ward_from_armour_item_+%",
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
/// unreserved mana, and a flask's or full energy shield's condition.
/// Channelling and non-channelling skills' total is
/// [`Poe1Quantity::ManaCostChannelling`] (P6-11t).
pub const COST_REASONS: &[&str] = &[
    "local_display_socketed_attacks_mana_cost_+",
    "local_display_spend_energy_shield_for_costs_before_mana_for_socketed_skills",
    "local_flask_skill_mana_cost_+%_during_flask_effect",
    "mana_cost_+%_while_on_full_energy_shield",
    "manaweave_cost_equals_%_unreserved_mana",
    "support_focus_channel_cost_+%_final_per_second_channelling_up_to_100%",
];

/// Stats that change movement speed in a way the engine does not model
/// (P6-10b), sorted: each maps to [`Poe1Quantity::MovementReason`], the
/// movement fields' reason alone. PoB's text for each (`ModParser.lua`):
/// on kill (`KilledRecently`) and with no gems in the boots, each a
/// config condition the engine does not settle; per
/// summoned phantasm (`PerStat`); movement speed that cannot go below its
/// base. (The armour movement penalties ignored are a row since P6-11cp;
/// during a flask's effect, on low life, and while ignited are
/// [`STATE_ROWS`] since P6-11ei.) Momentum's Swiftness is
/// `poe1::gems`'s (P6-11y).
pub const MOVEMENT_REASONS: &[&str] = &[
    "gain_movement_speed_+%_for_20_seconds_on_kill",
    "movement_speed_+%_per_summoned_phantasm",
    "movement_speed_cannot_be_reduced_below_base",
    "movement_velocity_+%_while_no_gems_in_boots",
];

/// Stats that change a pool's regeneration in a way the engine does not
/// model (P6-10b), sorted: each maps to [`Poe1Quantity::RegenReason`], the
/// regeneration field's reason alone. PoB's text for each
/// (`ModParser.lua`): life per level. (Energy shield per nearby corpse is
/// [`super::stacks`]' since P6-11ev.)
/// (Life per fortification stack is a row since P6-11bu, read by
/// [`super::regen`] with the stacks.) (Mana with exposure inflicted recently is
/// `poe1::exposure`'s since P6-11au.) Per
/// endurance and power charge are [`super::charges`]'s, per uncapped fire
/// resistance and per spell block chance [`super::regen`]'s (P6-11t).
pub const REGEN_REASONS: &[(&str, Pool)] =
    &[("life_regeneration_rate_per_minute_per_level", Pool::Life)];

/// Stats that change the base damage reduction in a way the engine does not
/// model, sorted (P6-10b part 5). Empty since P6-11cg: per hit taken
/// recently, per nearby enemy, and with only one enemy nearby are
/// [`super::stacks`]'s. A field marker ([`field_marker`]) read by
/// [`super::reduction`] alone.
pub const REDUCTION_REASONS: &[&str] = &[];

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
        // Devotion and its readers (P6-11az): `Base` values only
        // `poe1::devotion` reads.
        if stat.as_str() == "base_devotion" {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::Devotion),
                op: Op::Base,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
        }
        if let Some(i) = super::devotion::reader(stat.as_str()) {
            return StatMap::Effects(vec![EffectTemplate {
                quantity: Quantity::Poe1(Poe1Quantity::DevotionReader(i)),
                op: Op::Base,
                scale: 1.0,
                divisor: 1.0,
                conditions: Conditions::none(),
            }]);
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
        if let Ok(i) = super::charges::MULTI.binary_search_by(|(id, _)| (*id).cmp(stat.as_str())) {
            return StatMap::Effects(
                super::charges::MULTI[i]
                    .1
                    .iter()
                    .map(|&(kind, value)| EffectTemplate {
                        quantity: Quantity::Poe1(Poe1Quantity::Charge(kind, value)),
                        op: Op::Base,
                        scale: 1.0,
                        divisor: 1.0,
                        conditions: Conditions::none(),
                    })
                    .collect(),
            );
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
                | Poe1Quantity::RegenIncPerSpellBlock(_)
                | Poe1Quantity::RegenPerFireResist(_)
                | Poe1Quantity::RegenPerFortification(_)
                // Regeneration per charge (P6-11t).
                | Poe1Quantity::Charge(
                    _,
                    super::charges::ChargeValue::Per(
                        super::charges::PerCharge::LifeRegenPercentPerMinute
                            | super::charges::PerCharge::LifeRegenPerMinute
                            | super::charges::PerCharge::ManaRegen,
                    ),
                )
                | Poe1Quantity::RecoveryRate(_)
                | Poe1Quantity::RegenReason(..)
                | Poe1Quantity::EnergyShieldRecharge
                | Poe1Quantity::EnergyShieldRechargeFaster
                | Poe1Quantity::MovementSpeed
                // Movement speed per nearby enemy (P6-11v).
                | Poe1Quantity::Stack(
                    super::stacks::StackKind::NearbyEnemies,
                    super::stacks::StackValue::Per(super::stacks::PerStack::MovementSpeedUpTo8),
                )
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
    // P6-11cc: a weapon's "#% chance to Poison on Hit": PoB parses it as the
    // global `PoisonChance` BASE (no hand condition), and so does the engine.
    (
        "local_poison_on_hit_%",
        &[(
            Quantity::Poe1(Poe1Quantity::AilmentChance(Ailment::Poison)),
            Op::Base,
        )],
    ), // P3-8n: "Quality does not increase Physical Damage"
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
        // A row that increases both armour and evasion also carries the
        // shared marker, which Iron Reflexes' modifier subtracts once; no
        // row has a more of both (P6-11ek, `phases::iron_reflexes_modifier`).
        let shared = Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared);
        let check = |stat: &str, effects: &[(Quantity, Op)]| {
            let has = |q: Quantity, op: Op| effects.contains(&(q, op));
            if has(Quantity::Armour, Op::Increased) && has(Quantity::Evasion, Op::Increased) {
                assert!(has(shared, Op::Increased), "{stat}");
            }
            assert!(
                !(has(Quantity::Armour, Op::More) && has(Quantity::Evasion, Op::More)),
                "{stat}"
            );
        };
        for (stat, rows) in ROWS {
            let effects: Vec<_> = rows.iter().map(|r| (r.0, r.1)).collect();
            check(stat, &effects);
        }
        for (stat, rows) in STATE_ROWS {
            let effects: Vec<_> = rows.iter().map(|r| (r.0, r.1)).collect();
            check(stat, &effects);
        }
        for (stat, rows) in HIT_ROWS {
            let effects: Vec<_> = rows.iter().map(|r| (r.0, r.1)).collect();
            check(stat, &effects);
        }
        // A field marker is read before every table (P6-11ei: a state row
        // under a marker would never apply).
        for (stat, _) in STATE_ROWS {
            assert_eq!(field_marker(stat), None, "{stat}");
        }
        for (stat, _) in HIT_ROWS {
            assert_eq!(field_marker(stat), None, "{stat}");
        }
        for (stat, _) in ROWS {
            assert_eq!(field_marker(stat), None, "{stat}");
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
        // P6-11ay: every weapon's "with ... Attacks" (`ModFlag` of the
        // weapon and `Hit`; PoB's `flags=131076` for a bow).
        for (stat, weapon, t, b) in [
            (
                "attack_maximum_added_chaos_damage_with_bows",
                Flags::BOW,
                CHAOS_T,
                MAX,
            ),
            (
                "attack_minimum_added_fire_damage_with_staves",
                Flags::STAFF,
                FIRE_T,
                MIN,
            ),
            (
                "attack_minimum_added_cold_damage_with_maces",
                Flags::MACE,
                COLD_T,
                MIN,
            ),
            (
                "attack_minimum_added_physical_damage_with_claws",
                Flags::CLAW,
                PHYS,
                MIN,
            ),
            (
                "attack_maximum_added_lightning_damage_with_axes",
                Flags::AXE,
                LIGHTNING_T,
                MAX,
            ),
            (
                "attack_minimum_added_chaos_damage_with_daggers",
                Flags::DAGGER,
                CHAOS_T,
                MIN,
            ),
        ] {
            assert_eq!(
                hit(stat),
                [(added(t, b), Op::Base, 1.0, all(weapon | Flags::HIT))],
                "{stat}"
            );
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
        // A modelled support's global effect is `poe1::gems`'s (P6-11y).
        assert_eq!(
            SkillMapping.map(&StatId::new(
                "support_momentum_movement_speed_+%_per_stack_removed"
            )),
            StatMap::NoEffect
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
        assert!(NO_EFFECT_DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
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
            [
                (armour, Op::Increased, 1.0),
                (evasion, Op::Increased, 1.0),
                (
                    Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                    Op::Increased,
                    1.0
                )
            ]
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
                (Quantity::Evasion, Op::Increased, 1.0),
                (
                    Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                    Op::Increased,
                    1.0
                )
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
                        // P6-11en: the hands' weapon types.
                        states::USING_TWO_HANDED_MELEE_WEAPON,
                        conditions::STATIONARY,
                        conditions::ENEMY_RARE_OR_UNIQUE_NEARBY,
                        states::SIX_LIFE_MASTERIES,
                        // P6-8a part 2: settled by the spell path from the
                        // trigger before the damage.
                        super::super::triggers::TRIGGERED_DAMAGE,
                        // P6-10b part 13: the negation of a check's.
                        conditions::NOT_HIT_RECENTLY,
                        // P6-11eg: the negation of the blocked checks.
                        conditions::NOT_BLOCKED_RECENTLY,
                        // P6-11cq: from the active flasks.
                        conditions::USING_LIFE_FLASK,
                        // P6-11ct: from the heralds among the skills.
                        conditions::AFFECTED_BY_HERALD,
                        // P6-11de: from the marks among the curses.
                        conditions::ENEMY_MARKED,
                        // P6-11dz: from the active flasks.
                        conditions::USING_FLASK,
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
                    staff(
                        Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                        Op::Increased,
                        NONE,
                    ),
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
            // P6-11ai: on full life.
            (
                "attack_speed_+%_when_on_full_life",
                vec![config(
                    Quantity::Speed,
                    Op::Increased,
                    Flags::ATTACK,
                    conditions::FULL_LIFE,
                )],
            ),
            (
                "spell_suppression_chance_%_while_on_full_life",
                vec![config(SUPPRESSION, Op::Base, NONE, conditions::FULL_LIFE)],
            ),
            // P6-11w: movement speed on burning ground.
            (
                "movement_speed_+%_while_on_burning_ground",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    NONE,
                    conditions::ON_BURNING_GROUND,
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
            // P6-11cc.
            (
                "block_chance_on_damage_taken_%",
                vec![(BLOCK, Op::Base, NONE, conditions::BEEN_HIT_RECENTLY)],
            ),
            // P6-11bv.
            (
                "armour_evasion_+%_while_leeching",
                vec![
                    (Quantity::Armour, Op::Increased, NONE, conditions::LEECHING),
                    (Quantity::Evasion, Op::Increased, NONE, conditions::LEECHING),
                    (
                        Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
                        Op::Increased,
                        NONE,
                        conditions::LEECHING,
                    ),
                ],
            ),
            (
                "attack_and_cast_speed_+%_while_leeching",
                vec![(Quantity::Speed, Op::Increased, NONE, conditions::LEECHING)],
            ),
            (
                "attack_and_cast_speed_+%_while_leeching_energy_shield",
                vec![(
                    Quantity::Speed,
                    Op::Increased,
                    NONE,
                    conditions::LEECHING_ENERGY_SHIELD,
                )],
            ),
            (
                "attack_damage_+%_while_leeching",
                vec![(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::ATTACK,
                    conditions::LEECHING,
                )],
            ),
            (
                "attack_speed_+%_while_leeching",
                vec![(
                    Quantity::Speed,
                    Op::Increased,
                    Flags::ATTACK,
                    conditions::LEECHING,
                )],
            ),
            (
                "damage_+%_while_leeching",
                vec![(Quantity::Damage, Op::Increased, NONE, conditions::LEECHING)],
            ),
            (
                "evasion_rating_+%_while_leeching",
                vec![(Quantity::Evasion, Op::Increased, NONE, conditions::LEECHING)],
            ),
            // P6-11bj.
            (
                "recover_%_energy_shield_when_you_block_spell_damage_while_wielding_a_staff",
                vec![(
                    Quantity::Poe1(Poe1Quantity::BlockGain(
                        super::super::BlockGain::EnergyShieldSpellPercent,
                    )),
                    Op::Base,
                    NONE,
                    states::USING_STAFF,
                )],
            ),
            (
                "recover_%_life_when_you_block_attack_damage_while_wielding_a_staff",
                vec![(
                    Quantity::Poe1(Poe1Quantity::BlockGain(
                        super::super::BlockGain::LifePercent,
                    )),
                    Op::Base,
                    NONE,
                    states::USING_STAFF,
                )],
            ),
            // P6-11ba.
            (
                "spell_minimum_added_fire_damage_while_dual_wielding",
                vec![(
                    added(FIRE_T, MIN),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_maximum_added_fire_damage_while_dual_wielding",
                vec![(
                    added(FIRE_T, MAX),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_minimum_added_cold_damage_while_dual_wielding",
                vec![(
                    added(COLD_T, MIN),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_maximum_added_cold_damage_while_dual_wielding",
                vec![(
                    added(COLD_T, MAX),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_minimum_added_lightning_damage_while_dual_wielding",
                vec![(
                    added(LIGHTNING_T, MIN),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_maximum_added_lightning_damage_while_dual_wielding",
                vec![(
                    added(LIGHTNING_T, MAX),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_minimum_added_physical_damage_while_dual_wielding",
                vec![(
                    added(PHYS, MIN),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_maximum_added_physical_damage_while_dual_wielding",
                vec![(
                    added(PHYS, MAX),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_minimum_added_chaos_damage_while_dual_wielding",
                vec![(
                    added(CHAOS_T, MIN),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_maximum_added_chaos_damage_while_dual_wielding",
                vec![(
                    added(CHAOS_T, MAX),
                    Op::Base,
                    Flags::SPELL,
                    states::DUAL_WIELDING,
                )],
            ),
            (
                "spell_damage_+%_while_dual_wielding",
                vec![dual(Quantity::Damage, Op::Increased, Flags::SPELL)],
            ),
            (
                "damage_over_time_+%_while_dual_wielding",
                vec![dual(Quantity::Damage, Op::Increased, Flags::DOT)],
            ),
            (
                "damage_over_time_+%_while_holding_a_shield",
                vec![(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::DOT,
                    states::USING_SHIELD,
                )],
            ),
            (
                "chance_to_block_spells_%_if_damaged_by_a_hit_recently",
                vec![config(
                    SPELL_BLOCK,
                    Op::Base,
                    NONE,
                    conditions::BEEN_HIT_RECENTLY,
                )],
            ),
            (
                "evasion_rating_+%_while_moving",
                vec![config(
                    Quantity::Evasion,
                    Op::Increased,
                    NONE,
                    conditions::MOVING,
                )],
            ),
            // P6-11cq: life regeneration while using a life flask.
            (
                "life_regeneration_rate_per_minute_%_if_used_life_flask_in_past_10_seconds",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(Pool::Life)),
                    Op::Base,
                    NONE,
                    conditions::USING_LIFE_FLASK,
                )],
            ),
            // P6-11cu: minion penetration against cursed enemies.
            (
                "minions_penetrate_elemental_resistances_%_vs_cursed_enemies",
                vec![
                    config(MINION_FIRE_PEN, Op::Base, NONE, conditions::ENEMY_CURSED),
                    config(MINION_COLD_PEN, Op::Base, NONE, conditions::ENEMY_CURSED),
                    config(
                        MINION_LIGHTNING_PEN,
                        Op::Base,
                        NONE,
                        conditions::ENEMY_CURSED,
                    ),
                ],
            ),
            // P6-11ct: minion damage while affected by a herald.
            (
                "minion_damage_+%_while_affected_by_a_herald",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::Minion(
                        crate::poe1::minions::MinionValue::Damage,
                    )),
                    Op::Increased,
                    NONE,
                    conditions::AFFECTED_BY_HERALD,
                )],
            ),
            // P6-11ee: movement speed after a crit and while holding a
            // shield.
            (
                "movement_speed_+%_if_crit_recently",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    NONE,
                    conditions::CRIT_RECENTLY,
                )],
            ),
            (
                "movement_speed_+%_while_holding_shield",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    NONE,
                    states::USING_SHIELD,
                )],
            ),
            // P6-11ev: Onslaught for you and allies on a warcry.
            (
                "you_and_nearby_allys_gain_onslaught_for_4_seconds_on_warcry",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::Onslaught),
                    Op::Flag,
                    NONE,
                    conditions::USED_WARCRY_RECENTLY,
                )],
            ),
            // P6-11eq: block for you and allies after an attack or a spell.
            (
                "you_and_allies_additional_block_%_if_have_attacked_recently",
                vec![config(BLOCK, Op::Base, NONE, conditions::ATTACKED_RECENTLY)],
            ),
            (
                "you_and_allies_additional_spell_block_%_if_cast_spell_recently",
                vec![config(
                    SPELL_BLOCK,
                    Op::Base,
                    NONE,
                    conditions::CAST_SPELL_RECENTLY,
                )],
            ),
            // P6-11en: area damage with a two-handed melee weapon.
            (
                "two_handed_melee_area_damage_+%",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::AREA,
                    states::USING_TWO_HANDED_MELEE_WEAPON,
                )],
            ),
            // P6-11em: spell block after casting a spell.
            (
                "chance_to_block_spells_%_if_cast_a_spell_recently",
                vec![config(
                    SPELL_BLOCK,
                    Op::Base,
                    NONE,
                    conditions::CAST_SPELL_RECENTLY,
                )],
            ),
            // P6-11ei: movement speed on low life and while ignited.
            (
                "movement_velocity_+%_when_on_low_life",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    NONE,
                    conditions::LOW_LIFE,
                )],
            ),
            (
                "movement_velocity_+%_while_ignited",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    NONE,
                    conditions::IGNITED,
                )],
            ),
            // P6-11eg: crits by whether you have blocked recently; movement
            // speed during a flask effect.
            (
                "critical_strike_chance_+%_if_havent_blocked_recently",
                vec![config(
                    Quantity::CritChance,
                    Op::Increased,
                    NONE,
                    conditions::NOT_BLOCKED_RECENTLY,
                )],
            ),
            (
                "critical_strike_multiplier_+_if_you_have_blocked_recently",
                vec![config(
                    Quantity::CritMultiplier,
                    Op::Base,
                    NONE,
                    conditions::BLOCKED_RECENTLY,
                )],
            ),
            (
                "movement_speed_+%_during_flask_effect",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::MovementSpeed),
                    Op::Increased,
                    NONE,
                    conditions::USING_FLASK,
                )],
            ),
            // P6-11ed: crit multiplier with a rare or unique enemy nearby.
            (
                "critical_strike_multiplier_+_if_rare_or_unique_enemy_nearby",
                vec![config(
                    Quantity::CritMultiplier,
                    Op::Base,
                    NONE,
                    conditions::ENEMY_RARE_OR_UNIQUE_NEARBY,
                )],
            ),
            // P6-11ec: block and attack speed after a crit, attack damage
            // against bleeding and maimed enemies.
            (
                "additional_block_%_if_you_have_crit_recently",
                vec![config(
                    Quantity::BlockChance,
                    Op::Base,
                    NONE,
                    conditions::CRIT_RECENTLY,
                )],
            ),
            (
                "attack_damage_+%_vs_maimed_enemies",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::ATTACK,
                    conditions::ENEMY_MAIMED,
                )],
            ),
            (
                "attack_damage_vs_bleeding_enemies_+%",
                vec![config(
                    Quantity::Damage,
                    Op::Increased,
                    Flags::ATTACK,
                    conditions::ENEMY_BLEEDING,
                )],
            ),
            (
                "attack_speed_+%_if_have_crit_recently",
                vec![config(
                    Quantity::Speed,
                    Op::Increased,
                    Flags::ATTACK,
                    conditions::CRIT_RECENTLY,
                )],
            ),
            // P6-11dz: crit chance against bleeding enemies, elemental
            // damage during any flask effect.
            (
                "critical_strike_chance_+%_vs_bleeding_enemies",
                vec![config(
                    Quantity::CritChance,
                    Op::Increased,
                    NONE,
                    conditions::ENEMY_BLEEDING,
                )],
            ),
            (
                "elemental_damage_+%_during_flask_effect",
                vec![
                    config(
                        Quantity::Damage,
                        Op::Increased,
                        Flags::FIRE,
                        conditions::USING_FLASK,
                    ),
                    config(
                        Quantity::Damage,
                        Op::Increased,
                        Flags::COLD,
                        conditions::USING_FLASK,
                    ),
                    config(
                        Quantity::Damage,
                        Op::Increased,
                        Flags::LIGHTNING,
                        conditions::USING_FLASK,
                    ),
                ],
            ),
            // P6-11dp: crit multiplier against bleeding enemies, Pride's
            // impale chance.
            (
                "critical_strike_multiplier_+_vs_bleeding_enemies",
                vec![config(
                    Quantity::CritMultiplier,
                    Op::Base,
                    NONE,
                    conditions::ENEMY_BLEEDING,
                )],
            ),
            (
                "pride_chance_to_impale_with_attacks_%",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::ImpaleChance),
                    Op::Base,
                    Flags::KEYWORD_ATTACK,
                    "AffectedByPride",
                )],
            ),
            // P6-11dl: Pride's impale stacks.
            (
                "pride_your_impaled_debuff_lasts_+_additional_hits",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::ImpaleStacksMax),
                    Op::Base,
                    NONE,
                    "AffectedByPride",
                )],
            ),
            // P6-11dk: added fire damage against burning enemies.
            (
                "global_maximum_added_fire_damage_vs_burning_enemies",
                vec![config(
                    added(FIRE_T, MAX),
                    Op::Base,
                    NONE,
                    conditions::ENEMY_BURNING,
                )],
            ),
            (
                "global_minimum_added_fire_damage_vs_burning_enemies",
                vec![config(
                    added(FIRE_T, MIN),
                    Op::Base,
                    NONE,
                    conditions::ENEMY_BURNING,
                )],
            ),
            // P6-11de: against the marked enemy, and the marked enemy's
            // damage taken and accuracy.
            (
                "damage_+%_against_enemies_marked_by_you",
                vec![
                    config(
                        Quantity::Damage,
                        Op::Increased,
                        Flags::HIT,
                        conditions::ENEMY_MARKED,
                    ),
                    config(
                        Quantity::Damage,
                        Op::Increased,
                        Flags::AILMENT,
                        conditions::ENEMY_MARKED,
                    ),
                ],
            ),
            (
                "marked_enemy_accuracy_rating_+%",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::EnemyAccuracy),
                    Op::Increased,
                    NONE,
                    conditions::ENEMY_MARKED,
                )],
            ),
            (
                "marked_enemy_damage_taken_+%",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::EnemyDamageTaken),
                    Op::Increased,
                    NONE,
                    conditions::ENEMY_MARKED,
                )],
            ),
            // P6-11cm: the minion modifier with the character's condition.
            (
                "minion_damage_+%_if_have_used_a_minion_skill_recently",
                vec![config(
                    Quantity::Poe1(Poe1Quantity::Minion(
                        crate::poe1::minions::MinionValue::Damage,
                    )),
                    Op::Increased,
                    NONE,
                    conditions::USED_MINION_SKILL_RECENTLY,
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

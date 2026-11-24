//! Power, frenzy, and endurance charges (Phase 3 plan, step 12; P3-12e):
//! the config's `use<Kind>Charges` and `override<Kind>Charges` inputs.
//!
//! What PoB v2.67.2 does:
//! - `usePowerCharges` and its siblings (`Modules/ConfigOptions.lua` 900
//!   to 917) set the `UsePowerCharges` flag in combat; the override inputs
//!   (count options, applied when not 0, or their placeholder) set
//!   `PowerCharges` OVERRIDE, read only inside that flag's branch;
//! - `doActorCharges` (`Modules/CalcPerform.lua` 980 to 1120): with the
//!   flag, the charges are the override or the maximum, the maximum the
//!   `PowerChargesMax` BASE sum (the character's `max_power_charges`,
//!   `Modules/CalcSetup.lua` 47 to 49, plus modifiers), and at least the
//!   minimum (0 without modifiers); then `Multiplier:PowerCharge` is the
//!   count;
//! - `calcs.initModDB` (`Modules/CalcSetup.lua` 625 to 630) gives each
//!   charge its effect per charge, from the character's constants:
//!   `CritChance` INC `critical_strike_chance_+%_per_power_charge`;
//!   attack `Speed` INC `base_attack_speed_+%_per_frenzy_charge`, cast
//!   `Speed` INC `base_cast_speed_+%_per_frenzy_charge`, `Damage` MORE
//!   `object_inherent_damage_+%_final_per_frenzy_charge`;
//!   `PhysicalDamageReduction` and `ElementalDamageReduction` BASE per
//!   endurance charge (P6-10b part 5: [`super::reduction`]).
//!
//! The engine has no multipliers: each effect is one effect valued per
//! charge times the count. The count is PoB's (P3-8d):
//! `max(use ? (override or maximum) : 0, min(maximum, minimum))`, the
//! maximum the character constant plus the modifiers' (at least 0), the
//! minimum the modifiers' (at least 0). So a minimum gives charges without
//! the `use` input, as in PoB.
//!
//! **Modifiers** ([`MODIFIERS`], P3-8d): the stats of the maximum, the
//! minimum, and effects per charge map to [`Poe1Quantity::Charge`]
//! (`Base`, unconditional); [`charges`] reads them after every other
//! source and turns each per-charge effect into its target
//! ([`PerCharge::target`]) times the count, with the modifier's source.
//! What else reads or changes the counts ("at maximum" flags, conversions
//! to absorption, affliction, or brutal charges, "maximum frenzy charges is
//! maximum power charges", per-charge effects not in the table) is a stat
//! the mapping does not know, so a reason (`unmapped-stat`), or a
//! `NoEffect` row that reaches no slice field (a test in this module).
//!
//! The endurance charges' own effects change no computed field: like
//! [`super::config::NAMED_ONLY`], their only other effect is a multiplier
//! modifiers read; a per-endurance-charge modifier in [`MODIFIERS`] is
//! counted.

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use super::config::last_input as last;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::input::CalcData;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, UnsupportedReason};
use crate::store::{Conditions, Effect};

/// A kind of charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeKind {
    /// Power charges.
    Power,
    /// Frenzy charges.
    Frenzy,
    /// Endurance charges.
    Endurance,
}

impl ChargeKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Frenzy => "frenzy",
            Self::Endurance => "endurance",
        }
    }
}

/// What a charge modifier changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeValue {
    /// The maximum (PoB's `<Kind>ChargesMax` BASE).
    Maximum,
    /// The minimum (PoB's `<Kind>ChargesMin` BASE).
    Minimum,
    /// "Your Maximum Frenzy Charges is equal to your Maximum Power Charges"
    /// (P6-11bg; PoB's `MaximumFrenzyChargesIsMaximumPowerCharges`, a flag
    /// that overrides `FrenzyChargesMax` with the power charges' maximum,
    /// `CalcPerform.lua` 955 to 960): `Base`, any value above 0 on.
    MaximumFromPower,
    /// "Count as having maximum number of <Kind> Charges" (P6-11bl; PoB's
    /// `HaveMaximum<Kind>Charges` flag, `CalcPerform.lua` 1072 to 1080: the
    /// count is the maximum, whatever the config set): `Base`, any value
    /// above 0 on.
    CountAsMaximum,
    /// "Gain Brutal Charges instead of Endurance Charges" (P6-11hi; PoB's
    /// `EnduranceChargesConvertToBrutalCharges`, `CalcPerform.lua` 1042 to
    /// 1046): `Base`, any value above 0 on, on endurance charges.
    ToBrutal,
    /// "Maximum Brutal Charges is equal to Maximum Endurance Charges"
    /// (P6-11hi; `MaximumEnduranceChargesEqualsMaximumBrutalCharges`, 975).
    BrutalMaximumIsEndurance,
    /// "Modifiers to Minimum Endurance Charges instead apply to Minimum
    /// Brutal Charges" (P6-11hi;
    /// `MinimumEnduranceChargesEqualsMinimumBrutalCharges`, 974).
    BrutalMinimumIsEndurance,
    /// An effect per charge (a mod with PoB's `Multiplier:<Kind>Charge`
    /// tag).
    Per(PerCharge),
}

impl ChargeValue {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Maximum => "maximum",
            Self::Minimum => "minimum",
            Self::MaximumFromPower => "maximum-from-power",
            Self::CountAsMaximum => "count-as-maximum",
            Self::ToBrutal => "to-brutal",
            Self::BrutalMaximumIsEndurance => "brutal-maximum-is-endurance",
            Self::BrutalMinimumIsEndurance => "brutal-minimum-is-endurance",
            Self::Per(p) => p.name(),
        }
    }
}

/// An effect per charge: its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PerCharge {
    /// Increased attack speed (`Speed` INC with `ModFlag.Attack`;
    /// P6-11db).
    AttackSpeed,
    /// Increased accuracy rating (`Accuracy` INC; P6-11ec).
    Accuracy,
    /// Added accuracy rating (`Accuracy` BASE; P6-11ij).
    FlatAccuracy,
    /// Increased damage (`Damage` INC).
    Damage,
    /// More damage (`Damage` MORE; P6-11do: Power Charge On Critical's,
    /// a support's stat the main skill's mapping keeps).
    MoreDamage,
    /// Increased spell damage (`Damage` INC with `ModFlag.Spell`).
    SpellDamage,
    /// Increased wand damage (`Damage` INC with `ModFlag.Wand` and `Hit`;
    /// P6-11ef).
    WandDamage,
    /// Increased trap throwing speed (`TrapThrowingSpeed` INC; P6-11eh).
    TrapThrowingSpeed,
    /// Increased mine throwing speed (`MineLayingSpeed` INC; P6-11eh).
    MineLayingSpeed,
    /// Critical strike multiplier with traps (`CritMultiplier` BASE with
    /// `KeywordFlag.Trap`; P6-11eh).
    TrapCritMultiplier,
    /// Increased critical strike chance with mines (`CritChance` INC with
    /// `KeywordFlag.Mine`; P6-11eh).
    MineCritChance,
    /// Increased critical strike chance (`CritChance` INC).
    CritChance,
    /// Critical strike multiplier (`CritMultiplier` BASE).
    CritMultiplier,
    /// Increased evasion rating (`Evasion` INC; P6-11f).
    Evasion,
    /// Increased energy shield (`EnergyShield` INC; P6-11fb).
    EnergyShield,
    /// Increased movement speed (`MovementSpeed` INC; P6-10b).
    MovementSpeed,
    /// Life regeneration, percent of life per minute (`LifeRegenPercent`
    /// BASE per second, read divided by 60; P6-11t).
    LifeRegenPercentPerMinute,
    /// Life regeneration per second, the stat per minute (`LifeRegen`
    /// BASE, the stat's value over 60, [`PerCharge::scale`]; P6-11bq).
    LifeRegenPerMinute,
    /// Increased mana regeneration rate (`ManaRegen` INC; P6-11t).
    ManaRegen,
    /// Increased armour (`Armour` INC; P6-11cn).
    Armour,
    /// Additional physical damage reduction (`PhysicalDamageReduction`
    /// BASE; P6-11cn).
    PhysicalDamageReduction,
    /// Additional elemental damage reduction (`ElementalDamageReduction`
    /// BASE; P6-11cn).
    ElementalDamageReduction,
    /// Damage over time multiplier for bleeding (`DotMultiplier` BASE with
    /// `KeywordFlag.Bleed`; P6-11id).
    BleedDotMultiplier,
    /// Damage over time multiplier for poison (`DotMultiplier` BASE with
    /// `KeywordFlag.Poison`; close-out M2).
    PoisonDotMultiplier,
    /// More accuracy rating (`Accuracy` MORE; close-out M2).
    MoreAccuracy,
}

impl PerCharge {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AttackSpeed => "per.attack-speed",
            Self::Accuracy => "per.accuracy",
            Self::FlatAccuracy => "per.flat-accuracy",
            Self::Damage => "per.damage",
            Self::MoreDamage => "per.more-damage",
            Self::SpellDamage => "per.spell-damage",
            Self::WandDamage => "per.wand-damage",
            Self::TrapThrowingSpeed => "per.trap-throwing-speed",
            Self::MineLayingSpeed => "per.mine-laying-speed",
            Self::TrapCritMultiplier => "per.trap-crit-multiplier",
            Self::MineCritChance => "per.mine-crit-chance",
            Self::CritChance => "per.crit-chance",
            Self::CritMultiplier => "per.crit-multiplier",
            Self::Evasion => "per.evasion",
            Self::EnergyShield => "per.energy-shield",
            Self::MovementSpeed => "per.movement-speed",
            Self::LifeRegenPercentPerMinute => "per.life-regen-percent-per-minute",
            Self::LifeRegenPerMinute => "per.life-regen-per-minute",
            Self::ManaRegen => "per.mana-regen",
            Self::Armour => "per.armour",
            Self::PhysicalDamageReduction => "per.physical-damage-reduction",
            Self::ElementalDamageReduction => "per.elemental-damage-reduction",
            Self::BleedDotMultiplier => "per.bleed-dot-multiplier",
            Self::PoisonDotMultiplier => "per.poison-dot-multiplier",
            Self::MoreAccuracy => "per.more-accuracy",
        }
    }

    /// The effect one charge has: quantity, operation, and the flags a
    /// query needs.
    #[must_use]
    pub const fn target(self) -> (Quantity, Op, Flags) {
        match self {
            Self::AttackSpeed => (Quantity::Speed, Op::Increased, Flags::ATTACK),
            Self::Accuracy => (Quantity::Accuracy, Op::Increased, Flags::NONE),
            Self::FlatAccuracy => (Quantity::Accuracy, Op::Base, Flags::NONE),
            Self::Damage => (Quantity::Damage, Op::Increased, Flags::NONE),
            Self::MoreDamage => (Quantity::Damage, Op::More, Flags::NONE),
            Self::SpellDamage => (Quantity::Damage, Op::Increased, Flags::SPELL),
            Self::WandDamage => (
                Quantity::Damage,
                Op::Increased,
                Flags::WAND.union(Flags::HIT),
            ),
            Self::TrapThrowingSpeed => (
                Quantity::Poe1(super::Poe1Quantity::TrapThrowingSpeed),
                Op::Increased,
                Flags::NONE,
            ),
            Self::MineLayingSpeed => (
                Quantity::Poe1(super::Poe1Quantity::MineLayingSpeed),
                Op::Increased,
                Flags::NONE,
            ),
            Self::TrapCritMultiplier => (Quantity::CritMultiplier, Op::Base, Flags::KEYWORD_TRAP),
            Self::MineCritChance => (Quantity::CritChance, Op::Increased, Flags::KEYWORD_MINE),
            Self::CritChance => (Quantity::CritChance, Op::Increased, Flags::NONE),
            Self::CritMultiplier => (Quantity::CritMultiplier, Op::Base, Flags::NONE),
            Self::Evasion => (Quantity::Evasion, Op::Increased, Flags::NONE),
            Self::EnergyShield => (Quantity::EnergyShield, Op::Increased, Flags::NONE),
            Self::MovementSpeed => (
                Quantity::Poe1(super::Poe1Quantity::MovementSpeed),
                Op::Increased,
                Flags::NONE,
            ),
            Self::LifeRegenPercentPerMinute => (
                Quantity::Poe1(super::Poe1Quantity::RegenPercentPerMinute(
                    super::Pool::Life,
                )),
                Op::Base,
                Flags::NONE,
            ),
            Self::LifeRegenPerMinute => (
                Quantity::Poe1(super::Poe1Quantity::Regen(super::Pool::Life)),
                Op::Base,
                Flags::NONE,
            ),
            Self::ManaRegen => (
                Quantity::Poe1(super::Poe1Quantity::Regen(super::Pool::Mana)),
                Op::Increased,
                Flags::NONE,
            ),
            Self::Armour => (Quantity::Armour, Op::Increased, Flags::NONE),
            Self::BleedDotMultiplier => (Quantity::DotMultiplier, Op::Base, Flags::BLEED),
            Self::PoisonDotMultiplier => (Quantity::DotMultiplier, Op::Base, Flags::POISON),
            Self::MoreAccuracy => (Quantity::Accuracy, Op::More, Flags::NONE),
            Self::PhysicalDamageReduction => (
                Quantity::Poe1(super::Poe1Quantity::DamageReduction(
                    crate::quantity::DamageType::Physical,
                )),
                Op::Base,
                Flags::NONE,
            ),
            Self::ElementalDamageReduction => (
                Quantity::Poe1(super::Poe1Quantity::ElementalDamageReduction),
                Op::Base,
                Flags::NONE,
            ),
        }
    }

    /// The factor from the stat's value to the effect's: 1/60 for a stat
    /// per minute read per second (P6-11bq), else 1.
    #[must_use]
    pub const fn scale(self) -> f64 {
        match self {
            Self::LifeRegenPerMinute => 1.0 / 60.0,
            _ => 1.0,
        }
    }
}

/// Charge stats that are several of [`MODIFIERS`]' effects at once
/// (P6-11bb), sorted by stat ID, each as PoB reads its text:
/// - "+# to Maximum Power Charges and Maximum Endurance Charges":
///   `PowerChargesMax` and `EnduranceChargesMax` BASE;
/// - "#% increased Damage per Endurance, Frenzy or Power Charge":
///   `Damage` INC per `TotalCharges` (`PerStat`), which PoB sums from
///   the three kinds (`Modules/CalcPerform.lua` 1081), so a per-charge
///   damage effect of each.
pub const MULTI: &[(&str, &[(ChargeKind, ChargeValue)])] = &[
    (
        "damage_+%_per_frenzy_power_or_endurance_charge",
        &[
            (ChargeKind::Endurance, ChargeValue::Per(PerCharge::Damage)),
            (ChargeKind::Frenzy, ChargeValue::Per(PerCharge::Damage)),
            (ChargeKind::Power, ChargeValue::Per(PerCharge::Damage)),
        ],
    ),
    (
        "maximum_power_and_endurance_charges_+",
        &[
            (ChargeKind::Power, ChargeValue::Maximum),
            (ChargeKind::Endurance, ChargeValue::Maximum),
        ],
    ),
];

/// The charge modifiers the engine models (module docs), sorted by stat
/// ID, each checked against PoB v2.67.2's reading of its game text
/// (`Modules/ModParser.lua`: "maximum power charges" `PowerChargesMax`
/// BASE, "minimum power charges" `PowerChargesMin` BASE, and the
/// `per power charge` tag `{ type = "Multiplier", var = "PowerCharge" }`
/// on `Damage` INC, spell `Damage` INC, `CritChance` INC, and
/// `CritMultiplier` BASE; the frenzy and endurance stats alike).
pub const MODIFIERS: &[(&str, ChargeKind, ChargeValue)] = &[
    // P6-11ec: "#% increased Accuracy Rating per Frenzy Charge" (`Accuracy`
    // INC per `Multiplier:FrenzyCharge`).
    (
        "accuracy_rating_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::Accuracy),
    ),
    // P6-11ij: "+# to Accuracy Rating per Frenzy Charge" (`Accuracy` BASE
    // per `Multiplier:FrenzyCharge`).
    (
        "accuracy_rating_+_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::FlatAccuracy),
    ),
    // P6-11db: "#% increased Attack Speed per Frenzy Charge" (`Speed` INC
    // with `ModFlag.Attack` per `Multiplier:FrenzyCharge`).
    (
        "base_attack_speed_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::AttackSpeed),
    ),
    (
        "base_minimum_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::Minimum,
    ),
    (
        "base_minimum_frenzy_charges",
        ChargeKind::Frenzy,
        ChargeValue::Minimum,
    ),
    (
        "base_minimum_power_charges",
        ChargeKind::Power,
        ChargeValue::Minimum,
    ),
    // P6-11id: "+#% to Damage over Time Multiplier for Bleeding per
    // Endurance Charge" (`DotMultiplier` BASE, `KeywordFlag.Bleed`, per
    // `EnduranceCharge`).
    (
        "bleeding_dot_multiplier_+_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::BleedDotMultiplier),
    ),
    (
        "count_as_having_max_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::CountAsMaximum,
    ),
    (
        "count_as_having_max_frenzy_charges",
        ChargeKind::Frenzy,
        ChargeValue::CountAsMaximum,
    ),
    (
        "count_as_having_max_power_charges",
        ChargeKind::Power,
        ChargeValue::CountAsMaximum,
    ),
    (
        "critical_strike_chance_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::CritChance),
    ),
    (
        "critical_strike_multiplier_+_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::CritMultiplier),
    ),
    (
        "damage_+%_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::Damage),
    ),
    (
        "damage_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::Damage),
    ),
    (
        "damage_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::Damage),
    ),
    // Close-out M2: "#% more Accuracy Rating per Frenzy Charge" (`Accuracy`
    // MORE per `FrenzyCharge`).
    (
        "deadeye_accuracy_rating_+%_final_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::MoreAccuracy),
    ),
    // P6-11cn: "#% additional Elemental Damage Reduction per Endurance
    // Charge" (`ElementalDamageReduction` BASE per
    // `Multiplier:EnduranceCharge`).
    (
        "elemental_damage_reduction_%_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::ElementalDamageReduction),
    ),
    // P6-11dp: Endurance Charge on Melee Stun's more damage per endurance
    // charge (`sup_str.lua` 2005: `Damage` MORE per
    // `Multiplier:EnduranceCharge`).
    (
        "endurance_charge_on_melee_stun_damage_+%_final_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::MoreDamage),
    ),
    // P6-11fb: "#% increased Energy Shield per Power Charge" (`EnergyShield`
    // INC per `Multiplier:PowerCharge`).
    (
        "energy_shield_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::EnergyShield),
    ),
    (
        "evasion_rating_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::Evasion),
    ),
    // P6-11hi: "Gain Brutal Charges instead of Endurance Charges".
    (
        "gain_brutal_charges_instead_of_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::ToBrutal,
    ),
    // P6-11bq: "Regenerate # Life per second per Endurance Charge"
    // (`LifeRegen` BASE per `Multiplier:EnduranceCharge`).
    (
        "life_regen_per_minute_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::LifeRegenPerMinute),
    ),
    (
        "life_regeneration_rate_per_minute_%_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::LifeRegenPercentPerMinute),
    ),
    (
        "mana_regeneration_rate_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::ManaRegen),
    ),
    (
        "max_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::Maximum,
    ),
    (
        "max_frenzy_charges",
        ChargeKind::Frenzy,
        ChargeValue::Maximum,
    ),
    ("max_power_charges", ChargeKind::Power, ChargeValue::Maximum),
    // P6-11hi: "Maximum Brutal Charges is equal to Maximum Endurance Charges".
    (
        "maximum_brutal_charges_is_equal_to_maximum_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::BrutalMaximumIsEndurance,
    ),
    (
        "maximum_frenzy_charges_is_equal_to_maximum_power_charges",
        ChargeKind::Frenzy,
        ChargeValue::MaximumFromPower,
    ),
    // P6-11eh: "#% increased Critical Strike Chance per Power Charge when used
    // by Mines" (`SkillStatMap.lua` 1842: `CritChance` INC with
    // `KeywordFlag.Mine` per `Multiplier:PowerCharge`).
    (
        "mine_critical_strike_chance_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::MineCritChance),
    ),
    // P6-11eh: "#% increased Mine Throwing Speed per Frenzy Charge"
    // (`SkillStatMap.lua` 1826: `MineLayingSpeed` INC per
    // `Multiplier:FrenzyCharge`).
    (
        "mine_throwing_speed_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::MineLayingSpeed),
    ),
    // P6-11hi: "Modifiers to Minimum Endurance Charges instead apply to Minimum Brutal Charges".
    (
        "modifiers_to_minimum_endurance_charges_instead_apply_to_brutal_charges",
        ChargeKind::Endurance,
        ChargeValue::BrutalMinimumIsEndurance,
    ),
    (
        "movement_velocity_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::MovementSpeed),
    ),
    // P6-11cn: "#% additional Physical Damage Reduction per Endurance
    // Charge" (`PhysicalDamageReduction` BASE) and "#% increased Armour per
    // Endurance Charge" (`Armour` INC), each per
    // `Multiplier:EnduranceCharge`.
    (
        "physical_damage_reduction_%_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::PhysicalDamageReduction),
    ),
    (
        "physical_damage_reduction_rating_+%_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::Armour),
    ),
    // Close-out M2: "+#% to Damage over Time Multiplier for Poison per
    // Frenzy Charge" (`DotMultiplier` BASE, `KeywordFlag.Poison`, per
    // `FrenzyCharge`).
    (
        "poison_dot_multiplier_+_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::PoisonDotMultiplier),
    ),
    (
        "spell_damage_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::SpellDamage),
    ),
    // P6-11do: Power Charge On Critical's more damage per power charge
    // (`sup_int.lua` 5604: `Damage` MORE per `Multiplier:PowerCharge`).
    (
        "support_power_charge_on_crit_damage_+%_final_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::MoreDamage),
    ),
    // P6-11eh: "+#% to Critical Strike Multiplier per Power Charge when used
    // by Traps" (`SkillStatMap.lua` 1801: `CritMultiplier` BASE with
    // `KeywordFlag.Trap` per `Multiplier:PowerCharge`).
    (
        "trap_critical_strike_multiplier_+_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::TrapCritMultiplier),
    ),
    // P6-11eh: "#% increased Trap Throwing Speed per Frenzy Charge"
    // (`SkillStatMap.lua` 1792: `TrapThrowingSpeed` INC per
    // `Multiplier:FrenzyCharge`).
    (
        "trap_throwing_speed_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::TrapThrowingSpeed),
    ),
    // P6-11ef: "#% increased Wand Damage per Power Charge" (`Damage` INC
    // with `ModFlag.Wand` and `Hit` per `Multiplier:PowerCharge`).
    (
        "wand_damage_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::WandDamage),
    ),
];

/// The charge modifier `stat` is, if any ([`MODIFIERS`]).
#[must_use]
pub fn modifier(stat: &str) -> Option<(ChargeKind, ChargeValue)> {
    MODIFIERS
        .binary_search_by(|(id, _, _)| (*id).cmp(stat))
        .ok()
        .map(|i| (MODIFIERS[i].1, MODIFIERS[i].2))
}

/// The character constant of the triple damage chance per brutal charge
/// (`CalcSetup.lua` 540).
const TRIPLE_DAMAGE_PER_BRUTAL_STAT: &str = "chance_to_deal_triple_damage_%_per_brutal_charge";

/// One kind of charge.
#[derive(Debug, Clone, Copy)]
pub struct Charge {
    /// The kind.
    pub kind: ChargeKind,
    /// The config input that uses the charges.
    pub use_input: &'static str,
    /// The config input that sets their count.
    pub override_input: &'static str,
    /// The character constant of their maximum.
    pub max_stat: &'static str,
    /// Each effect per charge: the character constant of its value, the
    /// quantity, the operation, and the flags a query needs.
    pub effects: &'static [(&'static str, Quantity, Op, Flags)],
}

/// The three charges, in PoB's order.
pub const CHARGES: &[Charge] = &[
    Charge {
        kind: ChargeKind::Power,
        use_input: "usePowerCharges",
        override_input: "overridePowerCharges",
        max_stat: "max_power_charges",
        effects: &[(
            "critical_strike_chance_+%_per_power_charge",
            Quantity::CritChance,
            Op::Increased,
            Flags::NONE,
        )],
    },
    Charge {
        kind: ChargeKind::Frenzy,
        use_input: "useFrenzyCharges",
        override_input: "overrideFrenzyCharges",
        max_stat: "max_frenzy_charges",
        effects: &[
            (
                "base_attack_speed_+%_per_frenzy_charge",
                Quantity::Speed,
                Op::Increased,
                Flags::ATTACK,
            ),
            (
                "base_cast_speed_+%_per_frenzy_charge",
                Quantity::Speed,
                Op::Increased,
                Flags::CAST,
            ),
            (
                "object_inherent_damage_+%_final_per_frenzy_charge",
                Quantity::Damage,
                Op::More,
                Flags::NONE,
            ),
        ],
    },
    // Physical and elemental damage reduction (`CalcSetup.lua` 515 and
    // 516; P6-10b part 5, read by `poe1::reduction`).
    Charge {
        kind: ChargeKind::Endurance,
        use_input: "useEnduranceCharges",
        override_input: "overrideEnduranceCharges",
        max_stat: "max_endurance_charges",
        effects: &[
            (
                "physical_damage_reduction_%_per_endurance_charge",
                Quantity::Poe1(Poe1Quantity::DamageReduction(
                    crate::quantity::DamageType::Physical,
                )),
                Op::Base,
                Flags::NONE,
            ),
            (
                "elemental_damage_reduction_%_per_endurance_charge",
                Quantity::Poe1(Poe1Quantity::ElementalDamageReduction),
                Op::Base,
                Flags::NONE,
            ),
        ],
    },
];

/// Whether `input` is one of the charges' inputs, which [`charges`]
/// handles.
#[must_use]
pub fn is_charge_input(input: &str) -> bool {
    CHARGES
        .iter()
        .any(|c| c.use_input == input || c.override_input == input)
}

/// What an override input gives: `Ok(None)` when unset (absent or 0, as
/// PoB's count options), `Err(())` for a value PoB would read otherwise
/// than a number, or a number that is not finite.
fn count(value: Option<&ConfigValue>) -> Result<Option<f64>, ()> {
    match value {
        None => Ok(None),
        // A non-finite number (a build document can carry one) is not a
        // count PoB could hold: a reason, never a silent 0.
        Some(ConfigValue::Number(n)) if !n.value().0.is_finite() => Err(()),
        Some(ConfigValue::Number(n)) => Ok(Some(n.value().0).filter(|n| *n != 0.0)),
        Some(ConfigValue::Boolean(_) | ConfigValue::String(_)) => Err(()),
    }
}

/// The `Base` sum of the unconditional effects on `Charge(kind, value)`
/// so far, and the first one's source.
fn modifiers(ingest: &Ingest<'_>, kind: ChargeKind, value: ChargeValue) -> (f64, Option<Source>) {
    let mut sum = 0.0;
    let mut first = None;
    for e in ingest.effects() {
        if e.quantity == Quantity::Poe1(Poe1Quantity::Charge(kind, value))
            && e.op == Op::Base
            && e.conditions == Conditions::none()
        {
            sum += e.value;
            first.get_or_insert_with(|| e.source.clone());
        }
    }
    (sum, first)
}

/// The maximum power charges (P6-11gr; PoB's `PowerChargesMax`): the
/// character's `max_power_charges` and the unconditional maximum
/// modifiers, at least 0. `None` without the constant.
#[must_use]
pub fn power_maximum(ingest: &Ingest<'_>, data: &dyn CalcData) -> Option<f64> {
    let power = CHARGES.iter().find(|c| c.kind == ChargeKind::Power)?;
    let (more, _) = modifiers(ingest, ChargeKind::Power, ChargeValue::Maximum);
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let base = data.character_constant(power.max_stat)? as f64;
    Some((base + more).max(0.0))
}

/// Adds the effects of the charges (module docs): from the active config
/// set's `use` and override inputs and the charge modifiers ingested so
/// far, so it runs after every other source. The constant effects' source
/// is the `use` input when it is on (whichever input set the count),
/// else the first minimum modifier's; each per-charge modifier keeps its
/// own. A `use` input that is not a flag, or an override that is not a
/// number, is a reason under the `use` input; a missing constant is
/// indeterminate.
pub fn charges(set: Option<&ConfigSet>, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    for charge in CHARGES {
        let reason = || UnsupportedReason::UnmodelledConfig {
            input: charge.use_input.to_owned(),
        };
        let use_on = match set.and_then(|s| last(s, charge.use_input, ConfigInputKind::Input)) {
            None | Some(ConfigValue::Boolean(false)) => false,
            Some(ConfigValue::Boolean(true)) => true,
            Some(ConfigValue::Number(_) | ConfigValue::String(_)) => {
                ingest.reason(reason());
                continue;
            }
        };
        let per: Vec<Effect> = ingest
            .effects()
            .iter()
            .filter(|e| {
                matches!(
                    e.quantity,
                    Quantity::Poe1(Poe1Quantity::Charge(k, ChargeValue::Per(_))) if k == charge.kind
                ) && e.op == Op::Base
                    && e.conditions == Conditions::none()
            })
            .cloned()
            .collect();
        // No effect on a slice field: nothing to count.
        if charge.effects.is_empty() && per.is_empty() {
            continue;
        }
        let (min, min_source) = modifiers(ingest, charge.kind, ChargeValue::Minimum);
        let min = min.max(0.0);
        let (at_max, at_max_source) = modifiers(ingest, charge.kind, ChargeValue::CountAsMaximum);
        let at_max = at_max > 0.0;
        if !use_on && min == 0.0 && !at_max {
            continue;
        }
        // The frenzy charges' maximum is the power charges' with the flag
        // (`CalcPerform.lua` 955 to 960).
        let of = if charge.kind == ChargeKind::Frenzy
            && modifiers(ingest, ChargeKind::Frenzy, ChargeValue::MaximumFromPower).0 > 0.0
        {
            CHARGES
                .iter()
                .find(|c| c.kind == ChargeKind::Power)
                .unwrap_or(charge)
        } else {
            charge
        };
        let (more, _) = modifiers(ingest, of.kind, ChargeValue::Maximum);
        // The input, else its placeholder (PoB's `BuildModList`); read only
        // when the charges are used, as PoB does.
        let overridden = if use_on {
            match count(set.and_then(|s| last(s, charge.override_input, ConfigInputKind::Input))) {
                Ok(None) => count(
                    set.and_then(|s| last(s, charge.override_input, ConfigInputKind::Placeholder)),
                ),
                other => other,
            }
        } else {
            Ok(None)
        };
        let overridden = match overridden {
            Ok(n) => n,
            Err(()) => {
                ingest.reason(reason());
                continue;
            }
        };
        let max = match data.character_constant(of.max_stat) {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            Some(max) => (max as f64 + more).max(0.0),
            None => {
                ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: of.max_stat.to_owned(),
                });
                continue;
            }
        };
        let used = if use_on {
            overridden.unwrap_or(max)
        } else {
            0.0
        };
        // Brutal charges (P6-11hi; `CalcPerform.lua` 974, 975, 1042 to
        // 1046, 3069 to 3073): the endurance charges become brutal charges,
        // `max(endurance, min(brutal maximum, brutal minimum))`, and are 0
        // unless the character counts as having the maximum;
        // with the charges used, each brutal charge gives the character's
        // triple damage chance.
        if charge.kind == ChargeKind::Endurance
            && modifiers(ingest, ChargeKind::Endurance, ChargeValue::ToBrutal).0 > 0.0
        {
            if use_on {
                let flag = |v| modifiers(ingest, ChargeKind::Endurance, v).0 > 0.0;
                let brutal_max = if flag(ChargeValue::BrutalMaximumIsEndurance) {
                    max
                } else {
                    0.0
                };
                let brutal_min = if flag(ChargeValue::BrutalMinimumIsEndurance) {
                    min
                } else {
                    0.0
                };
                let brutal = used.max(brutal_min.min(brutal_max));
                match data.character_constant(TRIPLE_DAMAGE_PER_BRUTAL_STAT) {
                    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
                    Some(per) if brutal > 0.0 => ingest.effect(Effect {
                        quantity: Quantity::Poe1(Poe1Quantity::TripleDamageChance),
                        op: Op::Base,
                        value: per as f64 * brutal,
                        conditions: Conditions::none(),
                        source: Source::Config {
                            input: charge.use_input.to_owned(),
                        },
                    }),
                    Some(_) => {}
                    None => ingest.indeterminate(IndeterminateReason::MissingData {
                        table: "character_constants",
                        key: TRIPLE_DAMAGE_PER_BRUTAL_STAT.to_owned(),
                    }),
                }
            }
            // `HaveMaximumEnduranceCharges` sets the endurance charges to the
            // maximum after the conversion (1078 to 1080).
            if !at_max {
                continue;
            }
        }
        let count = if at_max { max } else { used.max(min.min(max)) };
        if count == 0.0 {
            continue;
        }
        let source = if at_max {
            match at_max_source {
                Some(s) => s,
                None => continue,
            }
        } else if use_on {
            Source::Config {
                input: charge.use_input.to_owned(),
            }
        } else if let Some(s) = min_source {
            s
        } else {
            continue;
        };
        for &(stat, quantity, op, flags) in charge.effects {
            match data.character_constant(stat) {
                #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
                Some(per) => ingest.effect(Effect {
                    quantity,
                    op,
                    value: per as f64 * count,
                    conditions: Conditions::none().with_all(flags),
                    source: source.clone(),
                }),
                None => ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                }),
            }
        }
        for e in per {
            let Quantity::Poe1(Poe1Quantity::Charge(_, ChargeValue::Per(p))) = e.quantity else {
                continue;
            };
            let (quantity, op, flags) = p.target();
            ingest.effect(Effect {
                quantity,
                op,
                value: e.value * p.scale() * count,
                conditions: Conditions::none().with_all(flags),
                source: e.source,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poe1::mapping::{HIT_ROWS, LOCAL_ROWS, ROWS, WEAPON_ROWS};

    #[test]
    fn multi_stats_should_be_sorted_and_apart_from_the_single_ones() {
        assert!(MULTI.windows(2).all(|w| w[0].0 < w[1].0));
        for (id, effects) in MULTI {
            assert!(modifier(id).is_none(), "{id}");
            assert!(effects.len() > 1, "{id}");
        }
    }

    #[test]
    fn charge_inputs_should_be_known_and_not_named_only() {
        for c in CHARGES {
            assert!(is_charge_input(c.use_input));
            assert!(is_charge_input(c.override_input));
            assert!(!super::super::config::named_only(c.use_input));
            assert!(!super::super::config::named_only(c.override_input));
        }
        assert!(!is_charge_input("useSiphoningCharges"));
    }

    /// A word of the stat ID is `charge` or `charges` (not `recharge`).
    fn names_a_charge(id: &str) -> bool {
        id.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == "charge" || w == "charges")
    }

    /// `NoEffect` rows that name a charge, each checked: none scales a
    /// slice field per charge or changes a count PoB's calc reads.
    const NO_EFFECT_CHARGE_ROWS: &[&str] = &[
        // Close-out M7: travel skills' cooldown per frenzy charge.
        "travel_skills_cooldown_speed_+%_per_frenzy_charge",
        // Close-out M3: a life flask's charges.
        "life_flask_charges_recovered_per_3_seconds",
        // Close-out M2: frenzy charge duration per frenzy charge.
        "frenzy_charge_duration_+%_per_frenzy_charge",
        // Chances to gain a charge: PoB's calc never reads them.
        "add_frenzy_charge_on_kill_%_chance",
        // P6-11as: area and stun duration per endurance charge (no slice
        // field).
        "area_of_effect_+%_per_endurance_charge",
        "stun_duration_+%_per_endurance_charge",
        // P6-11ef: area per power charge (no slice field).
        "skill_area_of_effect_+%_per_power_charge",
        // P6-11eg: chances to gain a charge on block and on a non-critical
        // strike (the counts are the config's), and flask charges.
        "chance_to_gain_endurance_charge_on_block_%",
        "power_charge_on_block_%_chance",
        // P6-11eh: more chances to gain a charge, and to lose one on a
        // travel skill (PoB parses none; the counts are the config's).
        "gain_power_charge_on_curse_cast_%",
        "gain_frenzy_charge_on_hitting_unique_enemy_%",
        "gain_frenzy_charge_on_hitting_rare_or_unique_enemy_%",
        "gain_frenzy_charge_on_hitting_marked_enemy_%",
        "power_charge_on_spell_block_%_chance",
        "lose_a_frenzy_charge_on_travel_skill_use_%_chance",
        // P6-11em: losing a power charge on gaining Elusive.
        "lose_a_power_charge_when_you_gain_elusive_%_chance",
        "gain_power_charge_on_non_critical_strike_%",
        "charges_gained_+%",
        "flask_charges_used_+%",
        "flask_charges_recovered_per_3_seconds",
        "add_power_charge_on_kill_%_chance",
        // P6-11do: Power Charge On Critical's chance (the count is the
        // config's).
        "add_power_charge_on_critical_strike_%",
        // P6-11eb: chances on a trap or mine, a melee crit, a stun; and the
        // frenzy charges' duration. The counts are the config's.
        "%_chance_to_gain_frenzy_charge_on_mine_detonated_targeting_an_enemy",
        "%_chance_to_gain_frenzy_charge_on_trap_triggered_by_an_enemy",
        "%_chance_to_gain_power_charge_on_mine_detonated_targeting_an_enemy",
        "%_chance_to_gain_power_charge_on_trap_triggered_by_an_enemy",
        "chance_to_gain_endurance_charge_on_melee_crit_%",
        "chance_to_gain_endurance_charge_when_you_stun_enemy_%",
        "base_frenzy_charge_duration_+%",
        // Shock on you: not a slice field.
        "cannot_be_shocked_while_at_maximum_power_charges",
        // P6-11cz: flask charges, not power, frenzy, or endurance charges.
        "life_flasks_gain_X_charges_on_suppressing_spell",
        "chance_to_gain_power_charge_when_block_%",
        // A charge's duration (P6-11f, P6-11n, P6-11u): the count comes
        // from the config.
        "base_charge_duration_+%",
        "charge_duration_+%",
        "endurance_charge_duration_+%",
        "power_charge_duration_+%",
        // A chance to gain a charge (P6-11g): PoB does not parse it.
        "endurance_charge_on_kill_percent_chance_while_holding_shield",
        // P6-11m: chances and rates to gain endurance charges; PoB does not
        // parse them, or reads only a partial maximum with leftover text.
        // The count comes from the config.
        "chance_to_gain_endurance_charge_when_hit_%",
        "endurance_charge_on_kill_%",
        "gain_endurance_charge_per_second_if_have_been_hit_recently",
        "gain_maximum_endurance_charges_on_endurance_charge_gained_%_chance",
        // P6-11m: a flask charge (`recharge` is not a charge word; this
        // names flask "charges").
        "local_flask_gain_X_charges_on_consuming_ignited_corpse",
        // P6-11br: flask charges during the flask's effect.
        "local_flask_cannot_gain_charges_during_flask_effect",
        "gain_power_charge_for_each_second_channeling_spell",
        // A skill's constant stat no stat map reads (P6-6a): not a charge
        // count.
        "skill_can_add_multiple_charges_per_action",
        // Flask charges (Ryslatha's soul, P3-12g): not the power, frenzy,
        // or endurance charges this module counts.
        "life_flasks_gain_X_charges_every_3_seconds_if_you_have_not_used_a_life_flask_recently",
        // Inspiration's charge gain and loss (P3-8n): PoB maps neither;
        // Inspiration charges are `poe1::stacks`'s, not this module's.
        "gain_righteous_charge_on_mana_spent_%",
        "lose_all_righteous_charges_on_mana_use_threshold",
        // Intensify's area per intensity (its "charge" is intensity,
        // `poe1::stacks`): area, not a slice field.
        "support_spell_boost_area_of_effect_+%_final_per_charge",
        // P6-11hz: warcries' chance to grant a charge per power, which PoB
        // leaves unparsed (the charges are the config's).
        "warcry_chance_to_gain_frenzy_power_endurance_charge_%_per_power",
    ];

    /// The module's argument: no row of the mapping's tables reads or
    /// changes a charge, so every stat that does is a reason, reaches no
    /// slice field, or is one of [`MODIFIERS`], which map only to
    /// [`Poe1Quantity::Charge`] (P3-8d).
    #[test]
    fn no_mapped_stat_should_read_or_change_a_charge() {
        use crate::ingest::{StatMap, StatMapping};
        use crate::poe1::ArmourSlot;
        use crate::poe1::mapping::{
            NO_EFFECT, PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping,
        };
        use loom_domain::StatId;

        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        // P6-11gr: the curse limit from the maximum power charges, which
        // `power_maximum` gives `poe1::gems`; it changes no charge count.
        let reads_maximum = ["has_curse_limit_equal_to_maximum_power_charges"];
        for id in ids {
            assert!(
                !names_a_charge(id) || reads_maximum.contains(&id),
                "{id}: revisit poe1::charges"
            );
        }
        // The charge modifiers map only to their own quantity, read here,
        // and no other table holds one (the mapping asks them first).
        assert!(MODIFIERS.windows(2).all(|w| w[0].0 < w[1].0));
        for (id, _, _) in MODIFIERS {
            let elsewhere = HIT_ROWS
                .iter()
                .map(|r| r.0)
                .chain(ROWS.iter().map(|r| r.0))
                .chain(NO_EFFECT.iter().map(|r| r.0))
                .any(|other| other == *id);
            assert!(!elsewhere, "{id}");
        }
        for (id, _) in NO_EFFECT {
            assert!(
                !names_a_charge(id) || NO_EFFECT_CHARGE_ROWS.contains(id),
                "{id}: a NoEffect row naming a charge; check it and list it"
            );
        }
        assert!(!names_a_charge("energy_shield_recharge_rate_+%"));
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        // The maxima map only to their charge quantity.
        for c in CHARGES {
            let StatMap::Effects(t) = Poe1Mapping.map(&StatId::new(c.max_stat)) else {
                panic!("{}", c.max_stat);
            };
            assert_eq!(
                t.iter().map(|t| t.quantity).collect::<Vec<_>>(),
                [Quantity::Poe1(Poe1Quantity::Charge(
                    c.kind,
                    ChargeValue::Maximum
                ))]
            );
            // A gem's stats never set a maximum; an item's lines hand it
            // to the game mapping.
            assert_eq!(
                SkillMapping.map(&StatId::new(c.max_stat)),
                StatMap::Unmapped
            );
            for m in [&piece as &dyn StatMapping, &WeaponMapping] {
                assert_eq!(
                    m.map(&StatId::new(c.max_stat)),
                    Poe1Mapping.map(&StatId::new(c.max_stat)),
                    "{}",
                    c.max_stat
                );
            }
        }
    }
}

//! Path of Exile rules (ADR-001; Phase 3 plan, decision 13). The shared
//! engine (store, graph, states, math) is game-neutral; everything only
//! Path of Exile has lives here: its quantities and fields, its
//! stat-to-effect mapping (P3-8), and its phases (P3-6 on).
//!
//! P3-6 computes the character fields (attributes, life, mana, energy
//! shield from global sources, unreserved life and mana): [`collect`]
//! gathers the build, [`mapping`] maps the stats those fields read,
//! [`phases`] computes them. P3-7 adds the core defences: armour,
//! evasion, energy shield with armour pieces, resistances, block, spell
//! block, spell suppression. P3-9b computes the main skill's hit when it is
//! a self-cast spell ([`hit`]): average damage, cast rate, crit chance and
//! multiplier, hit DPS. Its other hit fields, and every hit field of
//! another kind of skill, are `Unsupported` with `NotComputed`.

pub mod collect;
pub mod config;
pub mod enemy;
pub mod hit;
pub mod mapping;
pub mod phases;
pub mod skill;

pub use mapping::Poe1Mapping;

use loom_domain::Game;

use crate::engine::{Phase, Ruleset};
use crate::error::CalcError;
use crate::ingest::{Ingest, StatMapping};
use crate::input::CalcInput;
use crate::quantity::{Bound, DamageType};
use crate::result::Field;

/// A quantity only Path of Exile has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Poe1Quantity {
    /// Chance to suppress spell damage.
    SpellSuppressionChance,
    /// Chance to block spell damage.
    SpellBlockChance,
    /// Maximum chance to block spell damage.
    MaximumSpellBlockChance,
    /// A value of the armour piece in a slot (its local properties).
    Piece(ArmourSlot, PieceValue),
    /// The main skill's own base damage of a type (its level's
    /// "spell_minimum_base_cold_damage", ...).
    SkillBaseDamage(DamageType, Bound),
    /// The main skill's base critical strike chance, percent (its level's).
    SkillCritChance,
    /// The main skill's damage effectiveness (its level's), which scales
    /// added damage.
    DamageEffectiveness,
    /// The main skill's base damage multiplier (its level's).
    BaseDamageMultiplier,
    /// The main skill's cast time, seconds.
    CastTime,
    /// A value of the weapon in the main hand (its local properties).
    Weapon(WeaponValue),
    /// The skill's own conversion of one damage type to another, percent
    /// (PoB's `Skill<From>DamageConvertTo<To>`, which takes precedence over
    /// other conversion).
    SkillConversion(DamageType, DamageType),
    /// The main skill's attack speed multiplier, percent (its level's: PoB's
    /// `attackSpeedMultiplier`).
    AttackSpeedMultiplier,
    /// The enemy's largest physical damage reduction, percent.
    MaximumPhysicalDamageReduction,
}

/// A value of the weapon in the main hand (`Classes/Item.lua`, the weapon
/// branch of `BuildModListForSlotNum`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeaponValue {
    /// Damage of a type: `Base` is the base's (physical) and the local
    /// flat lines ("Adds # to # Physical Damage"), `Increased` the local
    /// increases.
    Damage(DamageType, Bound),
    /// Attack time, milliseconds (the base's).
    AttackTime,
    /// Local attack speed (`Increased`).
    AttackSpeed,
    /// Critical strike chance, percent: `Base` is the base's and the local
    /// flat lines, `Increased` the local increases.
    CritChance,
    /// The weapon's quality, percent.
    Quality,
    /// The weapon deals no physical damage when not 0 (the game's
    /// `local_weapon_no_physical_damage`, which the canonical "#% increased
    /// Physical Damage" line carries as 0).
    NoPhysicalDamage,
}

impl core::fmt::Display for WeaponValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Damage(t, b) => write!(f, "damage.{}.{}", t.name(), b.name()),
            Self::AttackTime => f.write_str("attack-time"),
            Self::AttackSpeed => f.write_str("attack-speed"),
            Self::CritChance => f.write_str("crit-chance"),
            Self::Quality => f.write_str("quality"),
            Self::NoPhysicalDamage => f.write_str("no-physical-damage"),
        }
    }
}

impl core::fmt::Display for Poe1Quantity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SpellSuppressionChance => f.write_str("spell-suppression-chance"),
            Self::SpellBlockChance => f.write_str("spell-block-chance"),
            Self::MaximumSpellBlockChance => f.write_str("maximum-spell-block-chance"),
            Self::Piece(slot, value) => write!(f, "piece.{}.{value}", slot.name()),
            Self::SkillBaseDamage(t, b) => {
                write!(f, "skill-base-damage.{}.{}", t.name(), b.name())
            }
            Self::SkillCritChance => f.write_str("skill-crit-chance"),
            Self::DamageEffectiveness => f.write_str("damage-effectiveness"),
            Self::BaseDamageMultiplier => f.write_str("base-damage-multiplier"),
            Self::CastTime => f.write_str("cast-time"),
            Self::Weapon(value) => write!(f, "weapon.{value}"),
            Self::SkillConversion(from, to) => {
                write!(f, "skill-conversion.{}.{}", from.name(), to.name())
            }
            Self::AttackSpeedMultiplier => f.write_str("attack-speed-multiplier"),
            Self::MaximumPhysicalDamageReduction => {
                f.write_str("maximum-physical-damage-reduction")
            }
        }
    }
}

/// An item slot whose armour piece has local defences, in the order PoB
/// sums them (`Modules/CalcDefence.lua`: Helmet, Gloves, Boots, Body
/// Armour, Weapon 2; its Weapon 3 is a minion's Aegis item, not the
/// player's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArmourSlot {
    /// Helmet.
    Helmet,
    /// Gloves.
    Gloves,
    /// Boots.
    Boots,
    /// Body armour.
    BodyArmour,
    /// The off hand (a shield).
    Weapon2,
}

impl ArmourSlot {
    /// Every slot, in PoB's order.
    pub const ALL: [Self; 5] = [
        Self::Helmet,
        Self::Gloves,
        Self::Boots,
        Self::BodyArmour,
        Self::Weapon2,
    ];

    /// Stable name: PoB's slot name lowercased, spaces as `-`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Helmet => "helmet",
            Self::Gloves => "gloves",
            Self::Boots => "boots",
            Self::BodyArmour => "body-armour",
            Self::Weapon2 => "weapon-2",
        }
    }

    /// The breakdown row's node name (`slice-fields.json` revision 2:
    /// `step.slot.<source>`); a static table, so `Term::Step` holds it.
    #[must_use]
    pub const fn row(self) -> &'static str {
        match self {
            Self::Helmet => "slot.helmet",
            Self::Gloves => "slot.gloves",
            Self::Boots => "slot.boots",
            Self::BodyArmour => "slot.body-armour",
            Self::Weapon2 => "slot.weapon-2",
        }
    }

    /// The slot of a build's slot name; a swap slot in use counts as its
    /// main slot (PoB removes ` Swap` from the names of the weapon set in
    /// use).
    #[must_use]
    pub fn of(slot: &str) -> Option<Self> {
        match slot.strip_suffix(" Swap").unwrap_or(slot) {
            "Helmet" => Some(Self::Helmet),
            "Gloves" => Some(Self::Gloves),
            "Boots" => Some(Self::Boots),
            "Body Armour" => Some(Self::BodyArmour),
            "Weapon 2" => Some(Self::Weapon2),
            _ => None,
        }
    }
}

/// A primary defence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Defence {
    /// Armour.
    Armour,
    /// Evasion rating.
    Evasion,
    /// Energy shield.
    EnergyShield,
}

impl Defence {
    /// Every defence.
    pub const ALL: [Self; 3] = [Self::Armour, Self::Evasion, Self::EnergyShield];

    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Armour => "armour",
            Self::Evasion => "evasion",
            Self::EnergyShield => "energy-shield",
        }
    }
}

/// A value of an armour piece (`Classes/Item.lua`, the armour branch of
/// `BuildModListForSlotNum`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PieceValue {
    /// The piece is equipped (a `Flag`).
    Equipped,
    /// A defence: `Base` is the base's minimum and the local flat lines,
    /// `Increased` the local increases.
    Defence(Defence),
    /// The base's range of a defence (maximum less minimum).
    Variance(Defence),
    /// Where in the range the piece rolled, 0 to 1 (the item's
    /// `<Defence>BasePercentile`; 1 when it has none).
    Percentile(Defence),
    /// The piece's quality, percent.
    Quality,
    /// A shield's chance to block: `Base` is the base's and the local
    /// flat lines, `Increased` the local increases.
    Block,
}

impl core::fmt::Display for PieceValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Equipped => f.write_str("equipped"),
            Self::Defence(d) => f.write_str(d.name()),
            Self::Variance(d) => write!(f, "variance.{}", d.name()),
            Self::Percentile(d) => write!(f, "percentile.{}", d.name()),
            Self::Quality => f.write_str("quality"),
            Self::Block => f.write_str("block"),
        }
    }
}

/// A field only Path of Exile has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Poe1Field {
    /// Chance to suppress spell damage.
    SpellSuppressionChance,
    /// Chance to block spell damage.
    SpellBlockChance,
}

impl Poe1Field {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SpellSuppressionChance => "spell-suppression-chance",
            Self::SpellBlockChance => "spell-block-chance",
        }
    }
}

/// The Path of Exile slice: the 24 fields of
/// `docs/phase-3/slice-fields.json` (revisions 1 and 2), in its order, in our
/// names (`TotalDPS` is [`Field::HitDps`]).
pub const SLICE: [Field; 24] = [
    Field::Strength,
    Field::Dexterity,
    Field::Intelligence,
    Field::Life,
    Field::Mana,
    Field::EnergyShield,
    Field::Armour,
    Field::Evasion,
    Field::Resistance(DamageType::Fire),
    Field::Resistance(DamageType::Cold),
    Field::Resistance(DamageType::Lightning),
    Field::Resistance(DamageType::Chaos),
    Field::BlockChance,
    Field::Poe1(Poe1Field::SpellBlockChance),
    Field::Poe1(Poe1Field::SpellSuppressionChance),
    Field::LifeUnreserved,
    Field::ManaUnreserved,
    Field::AverageDamage,
    Field::Speed,
    Field::HitSpeed,
    Field::HitChance,
    Field::CritChance,
    Field::CritMultiplier,
    Field::HitDps,
];

/// The Path of Exile rules.
#[derive(Debug, Clone, Copy, Default)]
pub struct Poe1Rules;

impl Ruleset for Poe1Rules {
    fn game(&self) -> Game {
        Game::Poe1
    }

    fn slice(&self) -> &[Field] {
        &SLICE
    }

    fn mapping(&self) -> &dyn StatMapping {
        &Poe1Mapping
    }

    fn collect(&self, input: &CalcInput<'_>, ingest: &mut Ingest<'_>) -> Result<(), CalcError> {
        collect::collect(input, ingest);
        Ok(())
    }

    fn phases(&self) -> &[Phase] {
        &phases::PHASES
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::quantity::Quantity;

    #[test]
    fn poe1_names_should_be_stable() {
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::SpellSuppressionChance).to_string(),
            "poe1.spell-suppression-chance"
        );
        assert_eq!(
            Field::Poe1(Poe1Field::SpellBlockChance).to_string(),
            "poe1.spell-block-chance"
        );
        assert_eq!(
            Poe1Quantity::SpellBlockChance.to_string(),
            "spell-block-chance"
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::Piece(
                ArmourSlot::BodyArmour,
                PieceValue::Percentile(Defence::EnergyShield)
            ))
            .to_string(),
            "poe1.piece.body-armour.percentile.energy-shield"
        );
        assert_eq!(ArmourSlot::of("Weapon 2 Swap"), Some(ArmourSlot::Weapon2));
        assert_eq!(ArmourSlot::of("Weapon 1"), None);
        for slot in ArmourSlot::ALL {
            assert_eq!(slot.row(), format!("slot.{}", slot.name()));
        }
        assert_eq!(
            Poe1Field::SpellSuppressionChance.name(),
            "spell-suppression-chance"
        );
    }

    #[test]
    fn the_slice_should_have_each_field_once() {
        let unique: BTreeSet<_> = SLICE.iter().collect();
        assert_eq!(unique.len(), SLICE.len());
    }

    #[test]
    fn the_rules_should_run_the_phases_in_order() {
        assert_eq!(Poe1Rules.game(), Game::Poe1);
        let names: Vec<_> = Poe1Rules.phases().iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "attributes",
                "life-mana",
                "resistances",
                "block",
                "primary-defences",
                "unreserved",
                "hit"
            ]
        );
        assert_eq!(
            Quantity::Poe1(Poe1Quantity::SkillBaseDamage(DamageType::Cold, Bound::Min)).to_string(),
            "poe1.skill-base-damage.cold.min"
        );
    }
}

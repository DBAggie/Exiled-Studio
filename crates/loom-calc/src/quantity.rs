//! Our own names for what modifiers change (Phase 3 plan, decision 2).
//!
//! Stat IDs come in; the stat-to-effect mapping turns each into effects on
//! these quantities. PoB's output names appear only in the parity tool's
//! mapping. A quantity one game has alone sits in that game's variant
//! ([`Quantity::Poe1`]), as `loom_domain::GameSpecific` does for build
//! choices (ADR-001).

use core::fmt;

use crate::poe1::Poe1Quantity;

/// A damage type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DamageType {
    /// Physical.
    Physical,
    /// Fire.
    Fire,
    /// Cold.
    Cold,
    /// Lightning.
    Lightning,
    /// Chaos.
    Chaos,
}

impl DamageType {
    /// Every damage type, in order.
    pub const ALL: [Self; 5] = [
        Self::Physical,
        Self::Fire,
        Self::Cold,
        Self::Lightning,
        Self::Chaos,
    ];

    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Physical => "physical",
            Self::Fire => "fire",
            Self::Cold => "cold",
            Self::Lightning => "lightning",
            Self::Chaos => "chaos",
        }
    }
}

/// Which end of a damage range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bound {
    /// The lowest roll.
    Min,
    /// The highest roll.
    Max,
}

impl Bound {
    /// Both ends, lowest first.
    pub const ALL: [Self; 2] = [Self::Min, Self::Max];

    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Min => "min",
            Self::Max => "max",
        }
    }
}

/// What an effect changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quantity {
    /// Strength.
    Strength,
    /// Dexterity.
    Dexterity,
    /// Intelligence.
    Intelligence,
    /// Maximum life.
    Life,
    /// Maximum mana.
    Mana,
    /// Maximum energy shield.
    EnergyShield,
    /// Armour.
    Armour,
    /// Evasion rating.
    Evasion,
    /// Resistance to a damage type.
    Resistance(DamageType),
    /// Maximum resistance to a damage type.
    MaximumResistance(DamageType),
    /// Chance to block attack damage. Its `Increased` and `More` effects
    /// scale every block chance (the game's "Chance to Block Attack and
    /// Spell Damage").
    BlockChance,
    /// Maximum chance to block attack damage.
    MaximumBlockChance,
    /// Damage; which damage (type, skill kind) is set by the effect's
    /// conditions and the query's context.
    Damage,
    /// Attack or cast speed, by the query's context.
    Speed,
    /// Critical strike chance.
    CritChance,
    /// Critical strike multiplier.
    CritMultiplier,
    /// Accuracy rating.
    Accuracy,
    /// Damage of a type added to hits ("Adds # to # Cold Damage"); which
    /// hits, by the effect's conditions. Scaled by the skill's damage
    /// effectiveness.
    AddedDamage(DamageType, Bound),
    /// Penetration of the enemy's resistance to a damage type, percent.
    Penetration(DamageType),
    /// A Path of Exile only quantity.
    Poe1(Poe1Quantity),
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strength => f.write_str("strength"),
            Self::Dexterity => f.write_str("dexterity"),
            Self::Intelligence => f.write_str("intelligence"),
            Self::Life => f.write_str("life"),
            Self::Mana => f.write_str("mana"),
            Self::EnergyShield => f.write_str("energy-shield"),
            Self::Armour => f.write_str("armour"),
            Self::Evasion => f.write_str("evasion"),
            Self::Resistance(t) => write!(f, "resistance.{}", t.name()),
            Self::MaximumResistance(t) => write!(f, "maximum-resistance.{}", t.name()),
            Self::BlockChance => f.write_str("block-chance"),
            Self::MaximumBlockChance => f.write_str("maximum-block-chance"),
            Self::Damage => f.write_str("damage"),
            Self::Speed => f.write_str("speed"),
            Self::CritChance => f.write_str("crit-chance"),
            Self::CritMultiplier => f.write_str("crit-multiplier"),
            Self::Accuracy => f.write_str("accuracy"),
            Self::AddedDamage(t, b) => write!(f, "added-damage.{}.{}", t.name(), b.name()),
            Self::Penetration(t) => write!(f, "penetration.{}", t.name()),
            Self::Poe1(q) => write!(f, "poe1.{q}"),
        }
    }
}

/// How an effect combines with the others on its quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Op {
    /// Added to the base (flat).
    Base,
    /// Summed with the other increases; the total scales the base by
    /// `1 + sum / 100`. A reduction is a negative value.
    Increased,
    /// Multiplies by `1 + value / 100`, each one separately. "Less" is a
    /// negative value.
    More,
    /// Sets a flag; the value is not read.
    Flag,
    /// Replaces the quantity's value.
    Override,
}

impl Op {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Increased => "increased",
            Self::More => "more",
            Self::Flag => "flag",
            Self::Override => "override",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantity_names_should_be_stable() {
        assert_eq!(Quantity::Life.to_string(), "life");
        assert_eq!(
            Quantity::MaximumResistance(DamageType::Fire).to_string(),
            "maximum-resistance.fire"
        );
        assert_eq!(Op::More.name(), "more");
        assert_eq!(
            Quantity::AddedDamage(DamageType::Cold, Bound::Max).to_string(),
            "added-damage.cold.max"
        );
        assert_eq!(
            Quantity::Penetration(DamageType::Lightning).to_string(),
            "penetration.lightning"
        );
        assert_eq!(DamageType::ALL.len(), 5);
    }
}

//! Path of Exile rules (ADR-001; Phase 3 plan, decision 13). The shared
//! engine (store, graph, states, math) is game-neutral; everything only
//! Path of Exile has lives here: its quantities and fields, its
//! stat-to-effect mapping (P3-8), and its phases (P3-6 on).
//!
//! P3-6 computes the character fields (attributes, life, mana, energy
//! shield from global sources, unreserved life and mana): [`collect`]
//! gathers the build, [`mapping`] maps the stats those fields read,
//! [`phases`] computes them. Every other slice field is `Unsupported`
//! with `NotComputed`; each later engine PR adds what it models.

pub mod collect;
pub mod mapping;
pub mod phases;

pub use mapping::Poe1Mapping;

use loom_domain::Game;

use crate::engine::{Phase, Ruleset};
use crate::error::CalcError;
use crate::ingest::{Ingest, StatMapping};
use crate::input::CalcInput;
use crate::quantity::DamageType;
use crate::result::Field;

/// A quantity only Path of Exile has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Poe1Quantity {
    /// Chance to suppress spell damage.
    SpellSuppressionChance,
    /// Chance to block spell damage.
    SpellBlockChance,
}

impl Poe1Quantity {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SpellSuppressionChance => "spell-suppression-chance",
            Self::SpellBlockChance => "spell-block-chance",
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
        assert_eq!(Poe1Quantity::SpellBlockChance.name(), "spell-block-chance");
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
            ["attributes", "life-mana", "energy-shield", "unreserved"]
        );
    }
}

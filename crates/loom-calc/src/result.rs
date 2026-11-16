//! A calculation's result (Phase 3 plan, step 4): the slice fields with
//! value, state, reasons, and their provenance graph. The versioned
//! document is P3-13's `loom_serialization::dto::calc::v1`.

use std::collections::BTreeMap;

use loom_domain::Game;

use crate::graph::{Graph, NodeIndex};
use crate::poe1::Poe1Field;
use crate::quantity::DamageType;
use crate::state::{FieldState, Reasons, StateKind};

/// A computed output (our own names; the parity tool maps them to PoB's,
/// `docs/phase-3/slice-fields.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
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
    /// Resistance to a damage type, after its cap.
    Resistance(DamageType),
    /// Chance to block attack damage.
    BlockChance,
    /// Life not reserved.
    LifeUnreserved,
    /// Mana not reserved.
    ManaUnreserved,
    /// The main skill's average damage per hit (for attacks, times the
    /// chance to hit).
    AverageDamage,
    /// The main skill's uses per second.
    Speed,
    /// The main skill's hits per second, when it differs from its uses.
    HitSpeed,
    /// The main skill's chance to hit (attacks).
    HitChance,
    /// The main skill's critical strike chance.
    CritChance,
    /// The main skill's critical strike multiplier.
    CritMultiplier,
    /// The main skill's hit damage per second: not the whole build's
    /// damage (Phase 3 plan, N6).
    HitDps,
    /// A Path of Exile only field.
    Poe1(Poe1Field),
}

impl core::fmt::Display for Field {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Self::Strength => "strength",
            Self::Dexterity => "dexterity",
            Self::Intelligence => "intelligence",
            Self::Life => "life",
            Self::Mana => "mana",
            Self::EnergyShield => "energy-shield",
            Self::Armour => "armour",
            Self::Evasion => "evasion",
            Self::Resistance(t) => return write!(f, "resistance.{}", t.name()),
            Self::BlockChance => "block-chance",
            Self::LifeUnreserved => "life-unreserved",
            Self::ManaUnreserved => "mana-unreserved",
            Self::AverageDamage => "average-damage",
            Self::Speed => "speed",
            Self::HitSpeed => "hit-speed",
            Self::HitChance => "hit-chance",
            Self::CritChance => "crit-chance",
            Self::CritMultiplier => "crit-multiplier",
            Self::HitDps => "hit-dps",
            Self::Poe1(p) => return write!(f, "poe1.{}", p.name()),
        };
        f.write_str(name)
    }
}

/// One field's result.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldResult {
    /// The value; `None` when the field is not computed.
    pub value: Option<f64>,
    /// The value's node in the result's graph; `None` when not computed.
    pub node: Option<NodeIndex>,
    /// The state with its reasons.
    pub state: FieldState,
}

impl FieldResult {
    /// A field the engine does not compute.
    #[must_use]
    pub const fn unsupported(reason: crate::state::UnsupportedReason) -> Self {
        Self {
            value: None,
            node: None,
            state: FieldState::Unsupported(reason),
        }
    }
}

/// A calculation's result.
#[derive(Debug, Clone, PartialEq)]
pub struct CalcResult {
    /// The build's game.
    pub game: Game,
    /// Every field of the game's slice, in order; a field the engine does
    /// not compute is `Unsupported`. Empty when the game has no
    /// calculation.
    pub fields: BTreeMap<Field, FieldResult>,
    /// The provenance graph every field's node points into.
    pub graph: Graph,
    /// The worst state of the fields; `Unsupported` when the game has no
    /// calculation (ADR-001).
    pub state: StateKind,
    /// The reasons that apply to every computed field (from collecting
    /// the build: unmapped stats, quarantined lines, unhandled kinds),
    /// kept here as well so they show even when no field is computed.
    pub reasons: Reasons,
}

impl CalcResult {
    /// The result for a game without a calculation.
    #[must_use]
    pub fn game_unsupported(game: Game) -> Self {
        Self {
            game,
            fields: BTreeMap::new(),
            graph: Graph::new(),
            state: StateKind::Unsupported,
            reasons: Reasons::new(),
        }
    }

    /// The worst state of `fields`; `Complete` when there are none.
    #[must_use]
    pub fn worst(fields: &BTreeMap<Field, FieldResult>) -> StateKind {
        fields
            .values()
            .map(|f| f.state.kind())
            .max()
            .unwrap_or(StateKind::Complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::UnsupportedReason;

    #[test]
    fn field_names_should_be_stable() {
        assert_eq!(Field::Life.to_string(), "life");
        assert_eq!(
            Field::Resistance(DamageType::Chaos).to_string(),
            "resistance.chaos"
        );
        assert_eq!(Field::HitDps.to_string(), "hit-dps");
    }

    #[test]
    fn worst_should_take_the_worst_field_state() {
        let mut fields = BTreeMap::new();
        assert_eq!(CalcResult::worst(&fields), StateKind::Complete);
        fields.insert(
            Field::Life,
            FieldResult {
                value: Some(1.0),
                node: None,
                state: FieldState::Complete,
            },
        );
        fields.insert(
            Field::Mana,
            FieldResult::unsupported(UnsupportedReason::NotComputed),
        );
        assert_eq!(CalcResult::worst(&fields), StateKind::Unsupported);
        let r = CalcResult::game_unsupported(Game::Poe2);
        assert!(r.fields.is_empty());
        assert_eq!(r.state, StateKind::Unsupported);
    }
}

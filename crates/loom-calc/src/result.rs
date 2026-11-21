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
    /// The main skill's own damage over time per second (PoB's
    /// `TotalDot`; Phase 6). Present only when the skill has some.
    DotDps,
    /// The main skill's ignite damage per second (PoB's `IgniteDPS`;
    /// Phase 6). Present only when the skill ignites.
    IgniteDps,
    /// The main skill's bleeding damage per second (PoB's `BleedDPS`;
    /// Phase 6). Present only when the skill's attack bleeds.
    BleedDps,
    /// The main skill's poisons' damage per second, every poison together
    /// (PoB's `TotalPoisonDPS`; Phase 6). Present only when the skill
    /// poisons.
    PoisonDps,
    /// A Path of Exile only field.
    Poe1(Poe1Field),
    /// A field of the main skill's minion (Phase 6 P6-7b2, plan decision
    /// 2: a shared name, PoE2 has minions too). Present only when the main
    /// skill has a minion the engine computes.
    Minion(MinionField),
}

/// A minion's field ([`Field::Minion`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MinionField {
    /// Maximum life.
    Life,
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
    /// Strength (P6-7c: the minion's hit reads it, as the character's).
    Strength,
    /// Dexterity (P6-7c).
    Dexterity,
    /// Intelligence (P6-7c).
    Intelligence,
    /// The minion's main skill's average damage per hit (P6-7c; PoB's
    /// `output.Minion.AverageDamage`).
    AverageDamage,
    /// Its uses per second.
    Speed,
    /// Its hits per second, when they differ from its uses.
    HitSpeed,
    /// Its chance to hit.
    HitChance,
    /// Its critical strike chance.
    CritChance,
    /// Its critical strike multiplier.
    CritMultiplier,
    /// Its hit damage per second (PoB's `TotalDPS`).
    HitDps,
    /// Its own damage over time per second.
    DotDps,
    /// Its ignite damage per second.
    IgniteDps,
    /// Its bleeding damage per second.
    BleedDps,
    /// Its poisons' damage per second.
    PoisonDps,
    /// Its impale damage per second.
    ImpaleDps,
    /// Its maximum mana (review of P6-7c: every character field has a
    /// minion copy, so a phase under the minion actor never drops a write).
    Mana,
    /// Its life not reserved.
    LifeUnreserved,
    /// Its mana not reserved.
    ManaUnreserved,
    /// A game-specific field of the minion's.
    Poe1(Poe1Field),
}

impl MinionField {
    /// The name after `minion.`.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Life => "life".to_owned(),
            Self::EnergyShield => "energy-shield".to_owned(),
            Self::Armour => "armour".to_owned(),
            Self::Evasion => "evasion".to_owned(),
            Self::Resistance(t) => format!("resistance.{}", t.name()),
            Self::BlockChance => "block-chance".to_owned(),
            Self::Strength => "strength".to_owned(),
            Self::Dexterity => "dexterity".to_owned(),
            Self::Intelligence => "intelligence".to_owned(),
            Self::AverageDamage => "average-damage".to_owned(),
            Self::Speed => "speed".to_owned(),
            Self::HitSpeed => "hit-speed".to_owned(),
            Self::HitChance => "hit-chance".to_owned(),
            Self::CritChance => "crit-chance".to_owned(),
            Self::CritMultiplier => "crit-multiplier".to_owned(),
            Self::HitDps => "hit-dps".to_owned(),
            Self::DotDps => "dot-dps".to_owned(),
            Self::IgniteDps => "ignite-dps".to_owned(),
            Self::BleedDps => "bleed-dps".to_owned(),
            Self::PoisonDps => "poison-dps".to_owned(),
            Self::ImpaleDps => "impale-dps".to_owned(),
            Self::Mana => "mana".to_owned(),
            Self::LifeUnreserved => "life-unreserved".to_owned(),
            Self::ManaUnreserved => "mana-unreserved".to_owned(),
            Self::Poe1(p) => format!("poe1.{}", p.name()),
        }
    }
}

impl Field {
    /// The minion's field a phase writing `self` writes while the current
    /// actor is the minion (P6-7c: the character's phases compute the
    /// minion's hit unchanged, [`crate::engine::Calc::with_actor`]). Every
    /// field has one (P6-7c review), so no write is dropped and no read
    /// falls back to the character's. A minion field is itself.
    #[must_use]
    pub const fn for_minion(self) -> Self {
        let m = match self {
            Self::Minion(m) => m,
            Self::Strength => MinionField::Strength,
            Self::Dexterity => MinionField::Dexterity,
            Self::Intelligence => MinionField::Intelligence,
            Self::Life => MinionField::Life,
            Self::EnergyShield => MinionField::EnergyShield,
            Self::Armour => MinionField::Armour,
            Self::Evasion => MinionField::Evasion,
            Self::Resistance(t) => MinionField::Resistance(t),
            Self::BlockChance => MinionField::BlockChance,
            Self::AverageDamage => MinionField::AverageDamage,
            Self::Speed => MinionField::Speed,
            Self::HitSpeed => MinionField::HitSpeed,
            Self::HitChance => MinionField::HitChance,
            Self::CritChance => MinionField::CritChance,
            Self::CritMultiplier => MinionField::CritMultiplier,
            Self::HitDps => MinionField::HitDps,
            Self::DotDps => MinionField::DotDps,
            Self::IgniteDps => MinionField::IgniteDps,
            Self::BleedDps => MinionField::BleedDps,
            Self::PoisonDps => MinionField::PoisonDps,
            Self::Poe1(crate::poe1::Poe1Field::ImpaleDps) => MinionField::ImpaleDps,
            Self::Mana => MinionField::Mana,
            Self::LifeUnreserved => MinionField::LifeUnreserved,
            Self::ManaUnreserved => MinionField::ManaUnreserved,
            Self::Poe1(p) => MinionField::Poe1(p),
        };
        Self::Minion(m)
    }
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
            Self::DotDps => "dot-dps",
            Self::IgniteDps => "ignite-dps",
            Self::BleedDps => "bleed-dps",
            Self::PoisonDps => "poison-dps",
            Self::Poe1(p) => return write!(f, "poe1.{}", p.name()),
            Self::Minion(m) => return write!(f, "minion.{}", m.name()),
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
        assert_eq!(
            Field::Minion(MinionField::HitDps).to_string(),
            "minion.hit-dps"
        );
    }

    /// P6-7c: under the minion actor a phase writes the minion's copy of
    /// an offence field; the character's own pools have none.
    #[test]
    fn for_minion_should_name_the_minions_copy() {
        assert_eq!(
            Field::HitDps.for_minion(),
            Field::Minion(MinionField::HitDps)
        );
        assert_eq!(
            Field::Poe1(Poe1Field::ImpaleDps).for_minion(),
            Field::Minion(MinionField::ImpaleDps)
        );
        assert_eq!(
            Field::Minion(MinionField::Life).for_minion(),
            Field::Minion(MinionField::Life)
        );
        assert_eq!(
            Field::ManaUnreserved.for_minion(),
            Field::Minion(MinionField::ManaUnreserved)
        );
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

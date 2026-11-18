//! Movement speed (Phase 6 P6-10b, part 3): the character's
//! `MovementSpeedMod` and `EffectiveMovementSpeedMod` as PoB v2.67.2
//! computes them (`Modules/CalcDefence.lua` 1504 to 1508).
//!
//! What PoB does:
//! - `MovementSpeedMod = calcLib.mod(MovementSpeed)`: `(1 + inc / 100) x
//!   more` of `MovementSpeed`;
//! - `EffectiveMovementSpeedMod = MovementSpeedMod x ActionSpeedMod`, the
//!   action speed modifier `calcs.actionSpeedMod` (`CalcPerform.lua`
//!   1107), the hit's ([`super::hit::action_speed_modifier`]).
//!
//! Not modelled, so reasons on these fields only
//! ([`super::mapping::MOVEMENT_REASONS`]): movement speed under conditions
//! the engine does not settle, per summoned phantasm or nearby enemy, the
//! armour movement penalties ignored, movement speed that cannot go below
//! its base, and Momentum's Swiftness. PoB's `MovementSpeed` override and
//! `MovementSpeedEqualHighestLinkedPlayers` (party members) have no mapped
//! stat: their stats stay `unmapped-stat` reasons (every field's).

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::Quantity;
use crate::result::Field;

/// The movement phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let mut reasons = phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::MovementReason(stat)) => Some(stat),
            _ => None,
        },
        reads,
    );
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let quantity = Quantity::Poe1(Poe1Quantity::MovementSpeed);
    let inc = store.increased(graph, quantity, &cx)?;
    let more = store.more_rounded(graph, quantity, &cx)?;
    reasons.extend(&inc.reasons);
    reasons.extend(&more.reasons);
    let modifier = phases::product(
        calc,
        &[
            Valued {
                node: inc.node,
                value: inc.value,
            },
            Valued {
                node: more.node,
                value: more.value,
            },
        ],
        Term::Result,
    )?;
    // The action speed's reasons are the effective field's alone.
    let mut effective_reasons = reasons.clone();
    let action = super::hit::action_speed_modifier(calc, &mut effective_reasons)?;
    calc.set_derived(
        Field::Poe1(Poe1Field::MovementSpeedMod),
        phases::query(modifier, reasons),
        &[],
    );
    let effective = phases::product(calc, &[modifier, action], Term::Result)?;
    calc.set_derived(
        Field::Poe1(Poe1Field::EffectiveMovementSpeedMod),
        phases::query(effective, effective_reasons),
        &[],
    );
    Ok(())
}

/// Whether the movement fields read `quantity`.
const fn reads(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(Poe1Quantity::MovementSpeed | Poe1Quantity::MovementReason(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_movement_fields_should_read_their_quantities_only() {
        assert!(reads(Quantity::Poe1(Poe1Quantity::MovementSpeed)));
        assert!(reads(Quantity::Poe1(Poe1Quantity::MovementReason("x"))));
        assert!(!reads(Quantity::Poe1(Poe1Quantity::ActionSpeed)));
        assert!(!reads(Quantity::Speed));
    }
}

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
//! - "Movement Speed cannot be modified to below Base Value" (P6-11ih,
//!   PoB's `MovementSpeedCannotBeBelowBase`, 1505 to 1507): the modifier is
//!   at least 1.
//!
//! Not modelled, so reasons on these fields only
//! ([`super::mapping::MOVEMENT_REASONS`]): movement speed while no gems are
//! in the boots. (On kill is a state row since P6-11ih.)
//!
//! Per nearby enemy and per summoned phantasm are [`super::stacks`]' (P6-11v,
//! P6-11ie).
//!
//! The armour bases' movement penalty (P6-11cp): PoB's `MovementSpeed` INC
//! with the negated condition `IgnoreMovementPenalties` (`Item.lua` 2362),
//! which "Ignore all Movement Penalties from Armour" sets as a flag. The
//! phase reads the flag and queries with the condition settled. PoB's `MovementSpeed` override and
//! `MovementSpeedEqualHighestLinkedPlayers` (party members) have no mapped
//! stat: their stats stay `unmapped-stat` reasons (every field's).

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::ConditionKey;
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::Quantity;
use crate::result::Field;

/// PoB's condition that drops the armour movement penalty (module docs).
pub const IGNORE_MOVEMENT_PENALTIES: &str = "IgnoreMovementPenalties";

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
    let plain = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let ignore = store.flag(
        graph,
        Quantity::Poe1(Poe1Quantity::IgnoreMovementPenalties),
        &plain,
    )?;
    reasons.extend(&ignore.reasons);
    let cx = plain.clone().with_state(
        ConditionKey::new(IGNORE_MOVEMENT_PENALTIES),
        ignore.value > 0.0,
    );
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
    // Queried only with an effect, so other builds keep their graphs.
    let floor_quantity = Quantity::Poe1(Poe1Quantity::MovementSpeedNotBelowBase);
    let floor = if phases::has(calc, floor_quantity) {
        let (store, graph) = calc.parts();
        let floor = store.flag(graph, floor_quantity, &plain)?;
        reasons.extend(&floor.reasons);
        floor.value > 0.0
    } else {
        false
    };
    let modifier = if floor {
        let one = phases::constant(calc, "movement-speed-base", 1.0)?;
        phases::pair(calc, modifier, one, Term::Result, false)?
    } else {
        modifier
    };
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
        Quantity::Poe1(
            Poe1Quantity::MovementSpeed
                | Poe1Quantity::MovementReason(_)
                | Poe1Quantity::IgnoreMovementPenalties
                | Poe1Quantity::MovementSpeedNotBelowBase
        )
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

//! Movement speed (Phase 6 P6-10b, part 3): the character's
//! `MovementSpeedMod` and `EffectiveMovementSpeedMod` as PoB v2.67.2
//! computes them (`Modules/CalcDefence.lua` 1504 to 1508).
//!
//! What PoB does:
//! - `MovementSpeedMod = calcLib.mod(MovementSpeed)`: `(1 + inc / 100) x
//!   more` of `MovementSpeed`;
//! - `EffectiveMovementSpeedMod = MovementSpeedMod x ActionSpeedMod`, the
//!   action speed modifier `calcs.actionSpeedMod` (`CalcPerform.lua`
//!   1107): `max(0, 1 + ActionSpeed INC / 100)`, with no minimum, maximum
//!   reduction, or Temporal Chains mapped (as [`super::hit`]'s).
//!
//! Not modelled, so reasons on these fields only
//! ([`super::mapping::MOVEMENT_REASONS`]): movement speed under conditions
//! the engine does not settle, per summoned phantasm, the armour movement
//! penalties ignored, and movement speed that cannot go below its base.

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::Quantity;
use crate::result::Field;
use crate::state::{Reasons, UnsupportedReason};
use loom_domain::StatId;

/// The movement phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let mut reasons = Reasons::new();
    for e in calc.store().effects() {
        if let Quantity::Poe1(Poe1Quantity::MovementReason(stat)) = e.quantity {
            reasons.partial.insert(UnsupportedReason::UnmappedStat {
                stat: StatId::new(stat),
                source: e.source.clone(),
            });
        }
    }
    for (quantities, reason) in calc.store().elsewhere() {
        if quantities.iter().any(|q| {
            matches!(
                q,
                Quantity::Poe1(Poe1Quantity::MovementSpeed | Poe1Quantity::MovementReason(_))
            )
        }) {
            reasons.indeterminate.insert(reason.clone());
        }
    }
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let quantity = Quantity::Poe1(Poe1Quantity::MovementSpeed);
    let inc = store.increased(graph, quantity, &cx)?;
    let more = store.more_rounded(graph, quantity, &cx)?;
    let action = store.increased(graph, Quantity::Poe1(Poe1Quantity::ActionSpeed), &cx)?;
    reasons.extend(&inc.reasons);
    reasons.extend(&more.reasons);
    let mut effective_reasons = reasons.clone();
    effective_reasons.extend(&action.reasons);
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
    calc.set_derived(
        Field::Poe1(Poe1Field::MovementSpeedMod),
        phases::query(modifier, reasons),
        &[],
    );
    let zero = phases::constant(calc, "no-minimum-action-speed", 0.0)?;
    let action = phases::pair(
        calc,
        Valued {
            node: action.node,
            value: action.value,
        },
        zero,
        Term::Step("action-speed"),
        false,
    )?;
    let effective = phases::product(calc, &[modifier, action], Term::Result)?;
    calc.set_derived(
        Field::Poe1(Poe1Field::EffectiveMovementSpeedMod),
        phases::query(effective, effective_reasons),
        &[],
    );
    Ok(())
}

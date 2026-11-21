//! The base damage reduction of each damage type (Phase 6 P6-10b, part 5):
//! PoB v2.67.2's `Base<Type>DamageReduction` (`Modules/CalcDefence.lua`
//! 1496 to 1501).
//!
//! What PoB does:
//! - `DamageReductionMax` is the character constant
//!   `maximum_physical_damage_reduction_%` (90; `Data.lua` 236), unless a
//!   modifier overrides it;
//! - `Base<Type>DamageReduction = min(max(0, <Type>DamageReduction BASE
//!   (+ ElementalDamageReduction BASE for fire, cold, and lightning)),
//!   DamageReductionMax)`, unless a modifier overrides it.
//!
//! Its sources here: the additional physical damage reduction line, the
//! stationary one (a state row), and each endurance charge's physical and
//! elemental damage reduction ([`super::charges`], `CalcSetup.lua` 515 and
//! 516). A stat that changes the reduction a way the engine does not model
//! (per nearby enemy, per hit taken recently, with one enemy nearby; the
//! maximum or an override) is a reason on these fields only
//! ([`super::mapping::REDUCTION_REASONS`]). The reduction against a hit
//! (PoB's `<Type>DamageReduction` with armour, the enemy's hit, and
//! overwhelm) is not computed.

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::{IndeterminateReason, Reasons};

/// The character constant of the largest damage reduction (PoB's
/// `data.misc.DamageReductionCap`).
const MAX_STAT: &str = "maximum_physical_damage_reduction_%";

/// The damage types in PoB's order (`dmgTypeList`).
const TYPES: [DamageType; 5] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Chaos,
];

/// Whether `quantity` is one this phase reads (for unsettled values).
const fn is_reduction(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(Poe1Quantity::DamageReduction(_) | Poe1Quantity::ElementalDamageReduction)
    )
}

/// The phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let mut marker = phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::ReductionReason(stat)) => Some(stat),
            _ => None,
        },
        is_reduction,
    );
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let max = match input.data.character_constant(MAX_STAT) {
        Some(v) => v as f64,
        None => {
            marker
                .indeterminate
                .insert(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: MAX_STAT.to_owned(),
                });
            90.0
        }
    };
    let cx = calc.context(Flags::NONE);
    for ty in TYPES {
        let mut reasons = marker.clone();
        let own = sum(
            calc,
            Quantity::Poe1(Poe1Quantity::DamageReduction(ty)),
            &cx,
            &mut reasons,
        )?;
        let total = if matches!(
            ty,
            DamageType::Fire | DamageType::Cold | DamageType::Lightning
        ) {
            let elemental = sum(
                calc,
                Quantity::Poe1(Poe1Quantity::ElementalDamageReduction),
                &cx,
                &mut reasons,
            )?;
            phases::total(calc, &[own, elemental], Term::Step("reduction-sum"))?
        } else {
            own
        };
        let zero = phases::constant(calc, "no-negative-reduction", 0.0)?;
        let floored = phases::pair(calc, total, zero, Term::Step("reduction-at-least-0"), false)?;
        let cap = phases::constant(calc, "damage-reduction-max", max)?;
        let value = phases::pair(calc, floored, cap, Term::Result, true)?;
        calc.set(
            Field::Poe1(Poe1Field::BaseDamageReduction(ty)),
            phases::query(value, reasons),
        );
    }
    Ok(())
}

/// The `Base` sum of `quantity`, its reasons into `reasons`.
fn sum(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &crate::condition::QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (store, graph) = calc.parts();
    let q = store.sum(graph, quantity, Op::Base, cx, Term::Step("reduction-base"))?;
    reasons.extend(&q.reasons);
    Ok(Valued {
        node: q.node,
        value: q.value,
    })
}

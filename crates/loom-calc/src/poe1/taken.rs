//! The character's damage taken over time (Phase 6 P6-10b, part 10):
//! PoB v2.67.2's `<Type>TakenDotMult` (`Modules/CalcDefence.lua` 1880 to
//! 1940). The hit's multiplier, PoB's first `<Type>TakenHitMult`, is the
//! input of its incoming hit (1942 on), which overwrites that output: it
//! waits for the enemy's hit.
//!
//! What PoB does, for each damage type:
//! - the base: the `INC` sum and the more product of `DamageTaken` and
//!   `<Type>DamageTaken`, with `ElementalDamageTaken` for fire, cold, and
//!   lightning;
//! - damage over time: the base with `DamageTakenOverTime` and
//!   `<Type>DamageTakenOverTime` (and the elemental one);
//!   `<Type>TakenDotMult = max((1 - resist / 100) x (1 - reduction / 100)
//!   x (1 + inc / 100) x more, 0)`, `resist` the type's resistance over
//!   time (`<Type>ResistOverTime`: its base summed with `ModFlag.Dot`,
//!   [`phases::resistance_in`]; physical's 0) and `reduction` its base
//!   damage reduction ([`super::reduction`]).
//!
//! PoB's more products are rounded per call (`ModDB:More`); the engine
//! rounds each quantity's, the same while each call has at most one
//! source. Not modelled, each an unmapped stat and so a reason: damage
//! taken from a hit's source (attacks, spells, projectiles), reflected
//! damage, conditional damage taken, and ignoring a resistance or a
//! reduction (`SelfIgnore<Type>Resistance`).

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity, TakenOf, TakenWhen};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::Reasons;

/// The damage types in PoB's order (`dmgTypeList`).
const TYPES: [DamageType; 5] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Chaos,
];

/// The phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for ty in TYPES {
        // Damage taken over time's markers (P6-10b part 13).
        let mut reasons = phases::marker_reasons(
            calc,
            |q| match q {
                Quantity::Poe1(Poe1Quantity::DotTakenReason(stat)) => Some(stat),
                _ => None,
            },
            |q| matches!(q, Quantity::Poe1(Poe1Quantity::SelfDamageTaken(..))),
        );
        let of = scopes(ty);
        let dot = multiplier(
            calc,
            &of,
            &[TakenWhen::Always, TakenWhen::OverTime],
            &mut reasons,
        )?;
        let read = |calc: &Calc<'_>, f: Field| {
            calc.field(f).and_then(|r| {
                Some(Valued {
                    node: r.node?,
                    value: r.value?,
                })
            })
        };
        let Some(reduction) = read(calc, Field::Poe1(Poe1Field::BaseDamageReduction(ty))) else {
            continue;
        };
        // PoB's `<Type>ResistOverTime`: the resistance with its base in a
        // damage over time context; physical's is 0 (`CalcDefence.lua` 511).
        let resist = if ty == DamageType::Physical {
            phases::constant(calc, "physical-resistance", 0.0)?
        } else {
            let (resist, resist_reasons) = phases::resistance_in(calc, ty, Flags::DOT)?;
            reasons.extend(&resist_reasons);
            resist
        };
        let kept_resist = kept(calc, resist, "resistance-kept")?;
        let kept_reduction = kept(calc, reduction, "reduction-kept")?;
        let product = phases::product(
            calc,
            &[kept_resist, kept_reduction, dot],
            Term::Step("dot-taken"),
        )?;
        let zero = phases::constant(calc, "no-negative-taken", 0.0)?;
        let value = phases::pair(calc, product, zero, Term::Result, false)?;
        calc.set_derived(
            Field::Poe1(Poe1Field::TakenDotMult(ty)),
            phases::query(value, reasons),
            &[Field::Poe1(Poe1Field::BaseDamageReduction(ty))],
        );
    }
    Ok(())
}

/// The names a type's damage taken reads: all damage, the type's, and the
/// elemental types' for fire, cold, and lightning.
fn scopes(ty: DamageType) -> Vec<TakenOf> {
    let mut of = vec![TakenOf::All, TakenOf::Type(ty)];
    if matches!(
        ty,
        DamageType::Fire | DamageType::Cold | DamageType::Lightning
    ) {
        of.push(TakenOf::Elemental);
    }
    of
}

/// `(1 + inc / 100) x more` over the `when` and `of` names.
fn multiplier(
    calc: &mut Calc<'_>,
    of: &[TakenOf],
    when: &[TakenWhen],
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cx = calc.context(Flags::NONE);
    let mut incs = Vec::new();
    let mut mores = Vec::new();
    for w in when {
        for o in of {
            let q = Quantity::Poe1(Poe1Quantity::SelfDamageTaken(*w, *o));
            let (store, graph) = calc.parts();
            let inc = store.sum(graph, q, Op::Increased, &cx, Term::Step("taken-increased"))?;
            let more = store.more_rounded(graph, q, &cx)?;
            reasons.extend(&inc.reasons);
            reasons.extend(&more.reasons);
            incs.push(Valued {
                node: inc.node,
                value: inc.value,
            });
            mores.push(Valued {
                node: more.node,
                value: more.value,
            });
        }
    }
    let inc = phases::total(calc, &incs, Term::Step("taken-increased-sum"))?;
    let hundred = phases::constant(calc, "percent", 100.0)?;
    let part = super::hit::ratio(calc, inc, hundred, Term::Step("taken-increased-part"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let factor = phases::total(calc, &[one, part], Term::Step("taken-increased-factor"))?;
    let more = phases::product(calc, &mores, Term::Step("taken-more"))?;
    phases::product(calc, &[factor, more], Term::Step("taken"))
}

/// `1 - x / 100`, in PoB's order.
fn kept(calc: &mut Calc<'_>, x: Valued, label: &'static str) -> Result<Valued, CalcError> {
    let hundred = phases::constant(calc, "percent", 100.0)?;
    let part = super::hit::ratio(calc, x, hundred, Term::Step(label))?;
    let one = phases::constant(calc, "one", 1.0)?;
    super::hit::minus(calc, one, part, Term::Step(label))
}

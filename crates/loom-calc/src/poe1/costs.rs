//! The main skill's mana cost (Phase 6 P6-10b, part 4): PoB v2.67.2's
//! `ManaCost` (`Modules/CalcOffence.lua` 1677 to 1800).
//!
//! What PoB does for an upfront mana cost:
//! - the base: the skill level's `Mana` cost (the granted effect level's
//!   costs, divisor 1), rounded to two decimals;
//! - `finalBaseCost = floor(base x mult) + ManaCostNoMult`, `mult` the
//!   supports' cost multipliers, `floor(More(SupportManaMultiplier), 4)`
//!   ([`super::gems::support_multiplier`], as reservation's);
//! - then `(1 + inc / 100)`, `inc` the `INC` sum of `ManaCost` and `Cost`,
//!   rounded up when `inc` is below 0 and down otherwise; the `ManaCost`
//!   more, then the `Cost` more, each rounded up below 1 and down
//!   otherwise; divided by the cost efficiency, `calcLib.mod` of
//!   `ManaCostEfficiency` and `CostEfficiency`; plus the total, the `BASE`
//!   sum of `ManaCost` and `Cost`; never below 0.
//!
//! Not modelled, so reasons on this field only
//! ([`super::mapping::COST_REASONS`] and the skill's own data): a cost in
//! another resource (life, energy shield, rage, percentages, per second),
//! skills whose reservation becomes their cost, costs paid in life or
//! energy shield instead, and conditional or socketed cost lines.

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::source::Source;
use crate::state::{Reasons, UnsupportedReason};
use loom_domain::StatId;

/// The cost phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let field = Field::Poe1(Poe1Field::ManaCost);
    let Ok(skill) = super::skill::main_skill(input.build, input.data, &levels) else {
        calc.set_unsupported(field, UnsupportedReason::NotComputed);
        return Ok(());
    };
    let source = Source::Gem {
        group: skill.group,
        gem: skill.active_gem.gem,
    };
    let Some(level) = super::skill::effect_level(skill.effect(), skill.active_gem.level) else {
        calc.set_unsupported(
            field,
            UnsupportedReason::UnmodelledSkill {
                source,
                what: "no-level",
            },
        );
        return Ok(());
    };
    let mut reasons = Reasons::new();
    for e in calc.store().effects() {
        if let Quantity::Poe1(Poe1Quantity::CostReason(stat)) = e.quantity {
            reasons.partial.insert(UnsupportedReason::UnmappedStat {
                stat: StatId::new(stat),
                source: e.source.clone(),
            });
        }
    }
    for (quantities, reason) in calc.store().elsewhere() {
        if quantities.iter().any(|q| is_cost(*q)) {
            reasons.indeterminate.insert(reason.clone());
        }
    }
    let mut amount = 0;
    for c in &level.costs {
        if c.resource == "Mana" {
            amount += c.amount;
        } else {
            reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: "cost-resource",
            });
        }
    }
    if skill.types.contains("ReservationBecomesCost") {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "reservation-becomes-cost",
        });
    }
    let mut flags = skill.keyword_flags();
    if skill.types.contains("Attack") {
        flags = flags | Flags::ATTACK;
    }
    if skill.types.contains("Spell") {
        flags = flags | Flags::SPELL;
    }
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let base = phases::constant(calc, "base-mana-cost", amount as f64)?;
    let mult = super::gems::support_multiplier(&skill.supports);
    let mult = phases::constant(calc, "support-cost-multiplier", mult)?;
    let scaled = phases::product(calc, &[base, mult], Term::Step("multiplied"))?;
    let floored = round(calc, scaled, false, Term::Step("multiplied-floor"))?;
    let mana = |q| Quantity::Poe1(q);
    let no_mult = sum(
        calc,
        mana(Poe1Quantity::ManaCostNoMult),
        Op::Base,
        flags,
        &mut reasons,
    )?;
    let mut cost = phases::total(calc, &[floored, no_mult], Term::Base)?;
    // Increased: one sum of both names.
    let inc_mana = sum(
        calc,
        mana(Poe1Quantity::ManaCost),
        Op::Increased,
        flags,
        &mut reasons,
    )?;
    let inc_any = sum(
        calc,
        mana(Poe1Quantity::Cost),
        Op::Increased,
        flags,
        &mut reasons,
    )?;
    let inc = phases::total(calc, &[inc_mana, inc_any], Term::Step("increased"))?;
    let hundredth = phases::constant(calc, "percent", 0.01)?;
    let one = phases::constant(calc, "one", 1.0)?;
    let inc_part = phases::product(calc, &[inc, hundredth], Term::Step("increased-part"))?;
    let inc_factor = phases::total(calc, &[one, inc_part], Term::Step("increased-factor"))?;
    let up = inc.value < 0.0;
    cost = phases::product(calc, &[inc_factor, cost], Term::Step("after-increased"))?;
    cost = round(calc, cost, up, Term::Step("after-increased-rounded"))?;
    for q in [Poe1Quantity::ManaCost, Poe1Quantity::Cost] {
        let more = more(calc, mana(q), flags, &mut reasons)?;
        let up = more.value < 1.0;
        cost = phases::product(calc, &[more, cost], Term::Step("after-more"))?;
        cost = round(calc, cost, up, Term::Step("after-more-rounded"))?;
    }
    // The cost efficiency, `calcLib.mod` of both names.
    let eff_mana = sum(
        calc,
        mana(Poe1Quantity::ManaCostEfficiency),
        Op::Increased,
        flags,
        &mut reasons,
    )?;
    let eff_any = sum(
        calc,
        mana(Poe1Quantity::CostEfficiency),
        Op::Increased,
        flags,
        &mut reasons,
    )?;
    let eff = phases::total(calc, &[eff_mana, eff_any], Term::Step("efficiency"))?;
    let eff_part = phases::product(calc, &[eff, hundredth], Term::Step("efficiency-part"))?;
    let eff_factor = phases::total(calc, &[one, eff_part], Term::Step("efficiency-factor"))?;
    let value = cost.value / eff_factor.value;
    let node = calc.graph_mut().push(
        Node::Quotient {
            term: Term::Step("after-efficiency"),
            a: cost.node,
            b: eff_factor.node,
        },
        value,
    )?;
    cost = Valued { node, value };
    let total_mana = sum(
        calc,
        mana(Poe1Quantity::ManaCost),
        Op::Base,
        flags,
        &mut reasons,
    )?;
    let total_any = sum(
        calc,
        mana(Poe1Quantity::Cost),
        Op::Base,
        flags,
        &mut reasons,
    )?;
    cost = phases::total(
        calc,
        &[cost, total_mana, total_any],
        Term::Step("with-total"),
    )?;
    let zero = phases::constant(calc, "no-negative-cost", 0.0)?;
    let cost = phases::pair(calc, cost, zero, Term::Result, false)?;
    calc.set_derived(field, phases::query(cost, reasons), &[]);
    Ok(())
}

/// Whether `quantity` is one the cost reads.
const fn is_cost(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(
            Poe1Quantity::ManaCost
                | Poe1Quantity::Cost
                | Poe1Quantity::ManaCostNoMult
                | Poe1Quantity::ManaCostEfficiency
                | Poe1Quantity::CostEfficiency
                | Poe1Quantity::CostReason(_)
        )
    )
}

/// The `op` sum of `quantity` in the skill's context.
fn sum(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    op: Op,
    flags: Flags,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cx = calc.context(flags);
    let (store, graph) = calc.parts();
    let q = store.sum(graph, quantity, op, &cx, Term::Step("cost-sum"))?;
    reasons.extend(&q.reasons);
    Ok(Valued {
        node: q.node,
        value: q.value,
    })
}

/// The more multiplier of `quantity` in the skill's context.
fn more(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    flags: Flags,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cx = calc.context(flags);
    let (store, graph) = calc.parts();
    let q = store.more_rounded(graph, quantity, &cx)?;
    reasons.extend(&q.reasons);
    Ok(Valued {
        node: q.node,
        value: q.value,
    })
}

/// `ceil(x)` when `up`, else `floor(x)`, never below 0 (PoB's `m_max(0,
/// m_ceil(..))` and `m_max(0, m_floor(..))`).
fn round(calc: &mut Calc<'_>, x: Valued, up: bool, term: Term) -> Result<Valued, CalcError> {
    let (value, node) = if up {
        (crate::math::ceil(x.value), Node::Ceil { term, of: x.node })
    } else {
        (
            crate::math::floor(x.value),
            Node::Floor { term, of: x.node },
        )
    };
    let node = calc.graph_mut().push(node, value)?;
    let zero = phases::constant(calc, "no-negative-cost", 0.0)?;
    phases::pair(
        calc,
        Valued { node, value },
        zero,
        Term::Step("not-negative"),
        false,
    )
}

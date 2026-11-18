//! The leech caps (Phase 6 P6-10b, part 7): PoB v2.67.2's
//! `Max<Pool>LeechRate`, `Max<Pool>LeechInstance`, and
//! `MaxLifeLeechRatePercent` (`Modules/CalcDefence.lua` 1201 to 1233).
//!
//! What PoB does, for life, energy shield, and mana:
//! - the rate cap's percent is `calcLib.val(Max<Pool>LeechRate)`: its
//!   `Base` sum times `(1 + inc / 100) x more`, 0 when the sum is 0; the
//!   sum holds the character's base (`CalcSetup.lua` 46, 47, and 51): the
//!   character constants `maximum_life_leech_rate_%_per_minute` and
//!   `maximum_mana_leech_rate_%_per_minute` / 60, and 10 for energy
//!   shield, which equals its constant
//!   `maximum_energy_shield_leech_rate_%_per_minute` (600) / 60, the one
//!   read here;
//! - `Max<Pool>LeechRate = pool x percent / 100`; life's percent is its own
//!   output, `MaxLifeLeechRatePercent`;
//! - the instance cap the same from `Max<Pool>LeechInstance`, its base the
//!   constant `maximum_<pool>_leech_amount_per_leech_%_max_<pool>`
//!   (`CalcSetup.lua` 52 to 54).
//!
//! Not modelled, each source an unmapped stat and so a reason: a cap
//! conditioned on what the engine does not settle (a crit or a savage hit
//! recently, Defiance, Zealotry, maximum rage), a doubled cap, and life's
//! percent taken from a parent or party member (minions and the Mirror of
//! Kalandra's flags).

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity, Pool};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::state::{IndeterminateReason, Reasons};

/// The pools in PoB's order, each with its field and the character
/// constants of its rate cap (percent per minute) and its instance cap.
const POOLS: [(Pool, Field, &str, &str); 3] = [
    (
        Pool::Life,
        Field::Life,
        "maximum_life_leech_rate_%_per_minute",
        "maximum_life_leech_amount_per_leech_%_max_life",
    ),
    (
        Pool::EnergyShield,
        Field::EnergyShield,
        "maximum_energy_shield_leech_rate_%_per_minute",
        "maximum_energy_shield_leech_amount_per_leech_%_max_energy_shield",
    ),
    (
        Pool::Mana,
        Field::Mana,
        "maximum_mana_leech_rate_%_per_minute",
        "maximum_mana_leech_amount_per_leech_%_max_mana",
    ),
];

/// The phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for (pool, pool_field, rate_stat, instance_stat) in POOLS {
        for (quantity, stat, per_minute) in [
            (Poe1Quantity::LeechRateMax(pool), rate_stat, true),
            (Poe1Quantity::LeechInstanceMax(pool), instance_stat, false),
        ] {
            let quantity = Quantity::Poe1(quantity);
            let mut reasons = phases::marker_reasons(calc, |_| None, |q| q == quantity);
            let percent = percent(calc, quantity, stat, per_minute, &mut reasons)?;
            let rate = matches!(quantity, Quantity::Poe1(Poe1Quantity::LeechRateMax(_)));
            if rate && pool == Pool::Life {
                calc.set(
                    Field::Poe1(Poe1Field::MaxLifeLeechRatePercent),
                    phases::query(percent, reasons.clone()),
                );
            }
            let Some((node, value)) = calc
                .field(pool_field)
                .and_then(|f| Some((f.node?, f.value?)))
            else {
                continue;
            };
            let pool_value = Valued { node, value };
            // PoB's order, `pool x percent / 100`, for its floating point.
            let product = phases::product(calc, &[pool_value, percent], Term::Step("of-pool"))?;
            let hundred = phases::constant(calc, "percent", 100.0)?;
            let quotient = product.value / hundred.value;
            let node = calc.graph_mut().push(
                Node::Quotient {
                    term: Term::Result,
                    a: product.node,
                    b: hundred.node,
                },
                quotient,
            )?;
            let value = Valued {
                node,
                value: quotient,
            };
            let field = if rate {
                Poe1Field::MaxLeechRate(pool)
            } else {
                Poe1Field::MaxLeechInstance(pool)
            };
            calc.set_derived(
                Field::Poe1(field),
                phases::query(value, reasons),
                &[pool_field],
            );
        }
    }
    Ok(())
}

/// PoB's `calcLib.val` of `quantity` with the character's base from the
/// constant `stat` (per minute when `per_minute`, so divided by 60).
fn percent(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    stat: &'static str,
    per_minute: bool,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let mut terms = vec![phases::sum(
        calc,
        quantity,
        Op::Base,
        Term::Step("modifiers"),
        reasons,
    )?];
    match calc.input().data.character_constant(stat) {
        Some(v) => {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            let v = v as f64;
            let base = if per_minute { v / 60.0 } else { v };
            terms.push(phases::constant(calc, stat, base)?);
        }
        None => {
            reasons
                .indeterminate
                .insert(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                });
        }
    }
    let base = phases::total(calc, &terms, Term::Base)?;
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let inc = store.increased(graph, quantity, &cx)?;
    let more = store.more_rounded(graph, quantity, &cx)?;
    reasons.extend(&inc.reasons);
    reasons.extend(&more.reasons);
    let inc = Valued {
        node: inc.node,
        value: inc.value,
    };
    let more = Valued {
        node: more.node,
        value: more.value,
    };
    phases::product(calc, &[base, inc, more], Term::Step("cap-percent"))
}

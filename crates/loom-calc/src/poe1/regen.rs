//! Life and mana regeneration (Phase 6 P6-10b, part 1): the character's
//! `LifeRegen` and `ManaRegen` as PoB v2.67.2 computes them
//! (`Modules/CalcDefence.lua` 1237 to 1285).
//!
//! What PoB does, per pool:
//! - `baseRegen = Sum(BASE <Pool>Regen) + pool x Sum(BASE
//!   <Pool>RegenPercent) / 100`, the pool its maximum; mana's sum holds
//!   the character's base, `ManaRegen` BASE `ManaRegenBase` per point of
//!   mana (`CalcSetup.lua` 497; `ManaRegenBase` is the character constant
//!   `mana_regeneration_rate_per_minute_%` / 60 / 100, `Modules/Data.lua`
//!   247);
//! - `regen = baseRegen x (1 + inc / 100) x more` of `<Pool>Regen`;
//! - `<Pool>Regen = round(regen x <Pool>RecoveryRateMod, 1)`, the mod
//!   `calcLib.mod(<Pool>RecoveryRate)` (1194 to 1198; for life only
//!   without `CannotRecoverLifeOutsideLeech`, an unmapped stat).
//!
//! Not modelled, so reasons where they appear (each source an unmapped
//! stat or a gem's reason): no regeneration (`No<Pool>Regen`,
//! `CannotGain<Pool>`), Zealot's Oath and life regeneration applying to
//! energy shield, a regeneration rate moved to another pool, and
//! regeneration conditioned on what the engine does not settle.

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity, Pool};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::state::{IndeterminateReason, Reasons, UnsupportedReason};
use loom_domain::StatId;

/// The character constant of mana's base regeneration, percent per minute.
pub const MANA_BASE_STAT: &str = "mana_regeneration_rate_per_minute_%";

/// The regeneration phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for (pool, field, pool_field) in [
        (Pool::Mana, Poe1Field::ManaRegen, Field::Mana),
        (Pool::Life, Poe1Field::LifeRegen, Field::Life),
    ] {
        let Some((node, value)) = calc
            .field(pool_field)
            .and_then(|f| Some((f.node?, f.value?)))
        else {
            continue;
        };
        let pool_value = Valued { node, value };
        let mut reasons = Reasons::new();
        // The pool's regeneration reasons (module docs): stats it does not
        // model, and its stats' unsettled values.
        for e in calc.store().effects() {
            if let Quantity::Poe1(Poe1Quantity::RegenReason(p, stat)) = e.quantity
                && p == pool
            {
                reasons.partial.insert(UnsupportedReason::UnmappedStat {
                    stat: StatId::new(stat),
                    source: e.source.clone(),
                });
            }
        }
        for (quantities, reason) in calc.store().elsewhere() {
            if quantities.iter().any(|q| reads(*q, pool)) {
                reasons.indeterminate.insert(reason.clone());
            }
        }
        let mut flat_terms = vec![phases::sum(
            calc,
            Quantity::Poe1(Poe1Quantity::Regen(pool)),
            Op::Base,
            Term::Step("flat"),
            &mut reasons,
        )?];
        if pool == Pool::Mana {
            match calc.input().data.character_constant(MANA_BASE_STAT) {
                Some(per_minute) => {
                    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
                    let per_mana = per_minute as f64 / 60.0 / 100.0;
                    let per_mana = phases::constant(calc, "mana-regeneration-base", per_mana)?;
                    flat_terms.push(phases::product(
                        calc,
                        &[per_mana, pool_value],
                        Term::Step("base-per-mana"),
                    )?);
                }
                None => {
                    reasons
                        .indeterminate
                        .insert(IndeterminateReason::MissingData {
                            table: "character_constants",
                            key: MANA_BASE_STAT.to_owned(),
                        });
                }
            }
        }
        let flat = phases::total(calc, &flat_terms, Term::Step("flat-sum"))?;
        let mut percent = phases::sum(
            calc,
            Quantity::Poe1(Poe1Quantity::RegenPercent(pool)),
            Op::Base,
            Term::Step("percent"),
            &mut reasons,
        )?;
        let per_minute = Quantity::Poe1(Poe1Quantity::RegenPercentPerMinute(pool));
        if phases::has(calc, per_minute) {
            let sum = phases::sum(
                calc,
                per_minute,
                Op::Base,
                Term::Step("percent-per-minute"),
                &mut reasons,
            )?;
            let sixtieth = phases::constant(calc, "per-second", 1.0 / 60.0)?;
            let per_second =
                phases::product(calc, &[sum, sixtieth], Term::Step("percent-per-second"))?;
            percent = phases::total(calc, &[percent, per_second], Term::Step("percent-sum"))?;
        }
        let hundredth = phases::constant(calc, "percent", 0.01)?;
        let of_pool = phases::product(
            calc,
            &[pool_value, percent, hundredth],
            Term::Step("percent-of-pool"),
        )?;
        let base = phases::total(calc, &[flat, of_pool], Term::Base)?;
        let (inc, more) = modifiers(
            calc,
            Quantity::Poe1(Poe1Quantity::Regen(pool)),
            &mut reasons,
        )?;
        let regen = phases::product(calc, &[base, inc, more], Term::Step("regeneration"))?;
        let (rinc, rmore) = modifiers(
            calc,
            Quantity::Poe1(Poe1Quantity::RecoveryRate(pool)),
            &mut reasons,
        )?;
        let rate = phases::product(calc, &[rinc, rmore], Term::Step("recovery-rate"))?;
        let raw = phases::product(calc, &[regen, rate], Term::Step("unrounded"))?;
        let value = round_1(calc, raw)?;
        calc.set_derived(
            Field::Poe1(field),
            phases::query(value, reasons),
            &[pool_field],
        );
    }
    Ok(())
}

/// Whether the regeneration of `pool` reads `quantity`.
const fn reads(quantity: Quantity, pool: Pool) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(
            Poe1Quantity::Regen(p)
            | Poe1Quantity::RegenPercent(p)
            | Poe1Quantity::RegenPercentPerMinute(p)
            | Poe1Quantity::RecoveryRate(p)
            | Poe1Quantity::RegenReason(p, _)
        ) if matches!((p, pool), (Pool::Life, Pool::Life) | (Pool::Mana, Pool::Mana))
    )
}

/// `(1 + inc / 100)` and the more of `quantity`, with their reasons.
fn modifiers(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued), CalcError> {
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let inc = store.increased(graph, quantity, &cx)?;
    let more = store.more_rounded(graph, quantity, &cx)?;
    reasons.extend(&inc.reasons);
    reasons.extend(&more.reasons);
    Ok((
        Valued {
            node: inc.node,
            value: inc.value,
        },
        Valued {
            node: more.node,
            value: more.value,
        },
    ))
}

/// PoB's `round(x, 1)`: `floor(x x 10 + 0.5) / 10`.
fn round_1(calc: &mut Calc<'_>, x: Valued) -> Result<Valued, CalcError> {
    let ten = phases::constant(calc, "ten", 10.0)?;
    let scaled = phases::product(calc, &[x, ten], Term::Step("round-scaled"))?;
    let rounded = phases::round(calc, scaled, Term::Step("round-whole"))?;
    let value = rounded.value / 10.0;
    let node = calc.graph_mut().push(
        Node::Quotient {
            term: Term::Result,
            a: rounded.node,
            b: ten.node,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

//! Regeneration and energy shield recharge (Phase 6 P6-10b, parts 1 and
//! 2): the character's `LifeRegen`, `ManaRegen`, `EnergyShieldRegen`,
//! `EnergyShieldRecharge`, and `EnergyShieldRechargeDelay` as PoB v2.67.2
//! computes them (`Modules/CalcDefence.lua` 1237 to 1383).
//!
//! What PoB does, per pool, in the order mana, life, energy shield:
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
//! - Zealot's Oath (1250 to 1259): life regenerates nothing; life's flat
//!   and percent sums, each when above 0, are energy shield's too.
//! - Per stat (P6-11t; PoB's `PerStat` tag, `floor(stat + 0.0001)` per
//!   1%, `ModStore.lua` 458): "Regenerate # Life per second for each 1%
//!   Uncapped Fire Resistance" adds to the flat sum per point of the fire
//!   resistance's uncapped total (`FireResistTotal`, computed earlier in
//!   `CalcDefence.lua`), and "#% increased Mana Regeneration Rate per 1%
//!   Chance to Block Spell Damage" to the increase per point of the spell
//!   block chance (the field, capped, as PoB's `output.SpellBlockChance`).
//!
//! Energy shield recharge (1325 to 1383), when nothing stops it (those
//! stats are unmapped, reasons): `round(energy shield x base x (1 + inc /
//! 100) x more x EnergyShieldRecoveryRateMod)` of `EnergyShieldRecharge`,
//! the base the character constant `energy_shield_recharge_rate_per_minute_%`
//! / 60 / 100, 2000 / 6000 (`Data.lua` 248). Line 249 sets the same key
//! to 0.33, but under LuaJIT a table constructor's constant entries fill a
//! template first and its computed ones are stored after, so 248's value
//! is the one PoB uses: its outputs are the constant's (980 energy shield
//! recharges 327, where 0.33 would give 323; `evasion-es-shadow.xml`).
//! The delay is `2 / (1 + inc / 100)` of `EnergyShieldRechargeFaster`
//! (`data.misc.EnergyShieldRechargeDelay`). PoB's `EnergyShieldRecharge`
//! override has no mapped stat: its stat stays an `unmapped-stat` reason.
//!
//! Not modelled, so reasons where they appear (each source an unmapped
//! stat or a gem's reason): no regeneration (`No<Pool>Regen`,
//! `CannotGain<Pool>`), life regeneration applying to energy shield, a
//! regeneration rate moved to another pool, regeneration conditioned on
//! what the engine does not settle, and recharge applying to life.

use super::charges::{ChargeValue, PerCharge};
use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity, Pool};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::state::{IndeterminateReason, Reasons};

/// The character constant of mana's base regeneration, percent per minute.
pub const MANA_BASE_STAT: &str = "mana_regeneration_rate_per_minute_%";

/// The character constant of energy shield's recharge, percent per minute.
pub const RECHARGE_BASE_STAT: &str = "energy_shield_recharge_rate_per_minute_%";

/// Energy shield's recharge delay in seconds (PoB's
/// `data.misc.EnergyShieldRechargeDelay`).
pub const RECHARGE_DELAY: f64 = 2.0;

/// The regeneration phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let zealots_oath = {
        let cx = calc.context(Flags::NONE);
        let (store, graph) = calc.parts();
        store
            .flag(graph, Quantity::Poe1(Poe1Quantity::ZealotsOath), &cx)?
            .value
            > 0.0
    };
    // Life's flat and percent sums under Zealot's Oath, with life's reasons,
    // for energy shield. The pools run in PoB's order (mana, life, energy
    // shield), so life's sums are in before energy shield reads them.
    let mut moved: Vec<(Valued, Valued, Reasons)> = Vec::new();
    for (pool, field, pool_field) in [
        (Pool::Mana, Poe1Field::ManaRegen, Field::Mana),
        (Pool::Life, Poe1Field::LifeRegen, Field::Life),
        (
            Pool::EnergyShield,
            Poe1Field::EnergyShieldRegen,
            Field::EnergyShield,
        ),
    ] {
        let Some((node, value)) = calc
            .field(pool_field)
            .and_then(|f| Some((f.node?, f.value?)))
        else {
            continue;
        };
        let pool_value = Valued { node, value };
        let mut reasons = pool_reasons(calc, pool);
        let mut flat_terms = vec![phases::sum(
            calc,
            Quantity::Poe1(Poe1Quantity::Regen(pool)),
            Op::Base,
            Term::Step("flat"),
            &mut reasons,
        )?];
        let flat_per_minute = Quantity::Poe1(Poe1Quantity::RegenPerMinute(pool));
        if phases::has(calc, flat_per_minute) {
            let sum = phases::sum(
                calc,
                flat_per_minute,
                Op::Base,
                Term::Step("flat-per-minute"),
                &mut reasons,
            )?;
            let sixtieth = phases::constant(calc, "per-second", 1.0 / 60.0)?;
            flat_terms.push(phases::product(
                calc,
                &[sum, sixtieth],
                Term::Step("flat-per-second"),
            )?);
        }
        let per_fire = Quantity::Poe1(Poe1Quantity::RegenPerFireResist(pool));
        if phases::has(calc, per_fire) {
            let per = phases::sum(
                calc,
                per_fire,
                Op::Base,
                Term::Step("per-fire-resistance"),
                &mut reasons,
            )?;
            let (_, total, resist_reasons) =
                phases::resistance_parts(calc, crate::quantity::DamageType::Fire, Flags::NONE)?;
            reasons.extend(&resist_reasons);
            let points = per_point(calc, total)?;
            flat_terms.push(phases::product(
                calc,
                &[per, points],
                Term::Step("flat-per-fire-resistance"),
            )?);
        }
        if pool == Pool::Mana
            && let Some(per_mana) = per_minute_constant(calc, MANA_BASE_STAT, &mut reasons)?
        {
            flat_terms.push(phases::product(
                calc,
                &[per_mana, pool_value],
                Term::Step("base-per-mana"),
            )?);
        }
        let mut percent_terms = vec![phases::sum(
            calc,
            Quantity::Poe1(Poe1Quantity::RegenPercent(pool)),
            Op::Base,
            Term::Step("percent"),
            &mut reasons,
        )?];
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
            percent_terms.push(phases::product(
                calc,
                &[sum, sixtieth],
                Term::Step("percent-per-second"),
            )?);
        }
        if pool == Pool::EnergyShield {
            // Zealot's Oath: life's sums, each when above 0.
            for (flat, percent, life) in moved.drain(..) {
                reasons.extend(&life);
                if flat.value > 0.0 {
                    flat_terms.push(flat);
                }
                if percent.value > 0.0 {
                    percent_terms.push(percent);
                }
            }
        }
        let flat = phases::total(calc, &flat_terms, Term::Step("flat-sum"))?;
        let percent = phases::total(calc, &percent_terms, Term::Step("percent-sum"))?;
        if pool == Pool::Life && zealots_oath {
            moved.push((flat, percent, reasons.clone()));
            let zero = phases::constant(calc, "zealots-oath", 0.0)?;
            calc.set_derived(
                Field::Poe1(field),
                phases::query(zero, reasons),
                &[pool_field],
            );
            continue;
        }
        let hundredth = phases::constant(calc, "percent", 0.01)?;
        let of_pool = phases::product(
            calc,
            &[pool_value, percent, hundredth],
            Term::Step("percent-of-pool"),
        )?;
        let base = phases::total(calc, &[flat, of_pool], Term::Base)?;
        let (mut inc, more) = modifiers(
            calc,
            Quantity::Poe1(Poe1Quantity::Regen(pool)),
            &mut reasons,
        )?;
        let per_block = Quantity::Poe1(Poe1Quantity::RegenIncPerSpellBlock(pool));
        let mut deps = vec![pool_field];
        if phases::has(calc, per_block)
            && let Some((node, value)) = calc
                .field(Field::Poe1(Poe1Field::SpellBlockChance))
                .and_then(|f| Some((f.node?, f.value?)))
        {
            deps.push(Field::Poe1(Poe1Field::SpellBlockChance));
            let per = phases::sum(
                calc,
                per_block,
                Op::Base,
                Term::Step("increased-per-spell-block"),
                &mut reasons,
            )?;
            let points = per_point(calc, Valued { node, value })?;
            let hundredth = phases::constant(calc, "percent", 0.01)?;
            let extra = phases::product(
                calc,
                &[per, points, hundredth],
                Term::Step("increased-from-spell-block"),
            )?;
            inc = phases::total(
                calc,
                &[inc, extra],
                Term::Step("increased-with-spell-block"),
            )?;
        }
        let regen = phases::product(calc, &[base, inc, more], Term::Step("regeneration"))?;
        let rate = recovery_rate(calc, pool, &mut reasons)?;
        let raw = phases::product(calc, &[regen, rate], Term::Step("unrounded"))?;
        let value = round_1(calc, raw)?;
        calc.set_derived(Field::Poe1(field), phases::query(value, reasons), &deps);
    }
    recharge(calc)
}

/// Energy shield's recharge and its delay (module docs).
fn recharge(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let Some((node, value)) = calc
        .field(Field::EnergyShield)
        .and_then(|f| Some((f.node?, f.value?)))
    else {
        return Ok(());
    };
    let es = Valued { node, value };
    let mut reasons = phases::marker_reasons(
        calc,
        |_| None,
        |q| {
            matches!(
                q,
                Quantity::Poe1(
                    Poe1Quantity::EnergyShieldRecharge
                        | Poe1Quantity::RecoveryRate(Pool::EnergyShield)
                )
            )
        },
    );
    // The delay reads only the faster start.
    let mut delay_reasons = phases::marker_reasons(
        calc,
        |_| None,
        |q| q == Quantity::Poe1(Poe1Quantity::EnergyShieldRechargeFaster),
    );
    let mut factors = vec![es];
    if let Some(base) = per_minute_constant(calc, RECHARGE_BASE_STAT, &mut reasons)? {
        factors.push(base);
    }
    let (inc, more) = modifiers(
        calc,
        Quantity::Poe1(Poe1Quantity::EnergyShieldRecharge),
        &mut reasons,
    )?;
    let rate = recovery_rate(calc, Pool::EnergyShield, &mut reasons)?;
    factors.extend([inc, more, rate]);
    // Eternal Youth's recharge applies to life (P6-11ao; `CalcDefence.lua`
    // 1325 to 1351): PoB sets `LifeRecharge` (no slice field) and no energy
    // shield recharge; the delay stays.
    let to_life = phases::has(calc, Quantity::Poe1(Poe1Quantity::EsRechargeAppliesToLife)) && {
        let cx = calc.context(crate::condition::Flags::NONE);
        let (store, graph) = calc.parts();
        let q = store.flag(
            graph,
            Quantity::Poe1(Poe1Quantity::EsRechargeAppliesToLife),
            &cx,
        )?;
        reasons.extend(&q.reasons);
        q.value != 0.0
    };
    if !to_life {
        let raw = phases::product(calc, &factors, Term::Step("unrounded"))?;
        let value = phases::round(calc, raw, Term::Result)?;
        calc.set_derived(
            Field::Poe1(Poe1Field::EnergyShieldRecharge),
            phases::query(value, reasons),
            &[Field::EnergyShield],
        );
    }
    let base = phases::constant(calc, "recharge-delay", RECHARGE_DELAY)?;
    let (faster, _) = modifiers(
        calc,
        Quantity::Poe1(Poe1Quantity::EnergyShieldRechargeFaster),
        &mut delay_reasons,
    )?;
    let value = base.value / faster.value;
    let node = calc.graph_mut().push(
        Node::Quotient {
            term: Term::Result,
            a: base.node,
            b: faster.node,
        },
        value,
    )?;
    calc.set_derived(
        Field::Poe1(Poe1Field::EnergyShieldRechargeDelay),
        phases::query(Valued { node, value }, delay_reasons),
        &[Field::EnergyShield],
    );
    Ok(())
}

/// The pool's regeneration reasons (module docs): stats it does not model,
/// and its stats' unsettled values.
fn pool_reasons(calc: &Calc<'_>, pool: Pool) -> Reasons {
    phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::RegenReason(p, stat)) if p == pool => Some(stat),
            _ => None,
        },
        |q| reads(q, pool),
    )
}

/// A character constant in percent per minute as a fraction per second,
/// or a missing data reason.
fn per_minute_constant(
    calc: &mut Calc<'_>,
    stat: &'static str,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    match calc.input().data.character_constant(stat) {
        Some(per_minute) => {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            let per_second = per_minute as f64 / 60.0 / 100.0;
            Ok(Some(phases::constant(calc, stat, per_second)?))
        }
        None => {
            reasons
                .indeterminate
                .insert(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                });
            Ok(None)
        }
    }
}

/// The pool's recovery rate modifier, `(1 + inc / 100) x more`.
pub(super) fn recovery_rate(
    calc: &mut Calc<'_>,
    pool: Pool,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (inc, more) = modifiers(
        calc,
        Quantity::Poe1(Poe1Quantity::RecoveryRate(pool)),
        reasons,
    )?;
    phases::product(calc, &[inc, more], Term::Step("recovery-rate"))
}

/// PoB's `PerStat` count with `div = 1`: `floor(stat + 0.0001)`.
fn per_point(calc: &mut Calc<'_>, stat: Valued) -> Result<Valued, CalcError> {
    let epsilon = phases::constant(calc, "per-stat-epsilon", 0.0001)?;
    let raw = phases::total(calc, &[stat, epsilon], Term::Step("per-stat-raw"))?;
    let value = (stat.value + 0.0001).floor();
    let node = calc.graph_mut().push(
        Node::Floor {
            term: Term::Step("per-stat"),
            of: raw.node,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// Whether the regeneration of `pool` reads `quantity`.
fn reads(quantity: Quantity, pool: Pool) -> bool {
    match quantity {
        Quantity::Poe1(
            Poe1Quantity::Regen(p)
            | Poe1Quantity::RegenPercent(p)
            | Poe1Quantity::RegenPercentPerMinute(p)
            | Poe1Quantity::RegenPerMinute(p)
            | Poe1Quantity::RegenIncPerSpellBlock(p)
            | Poe1Quantity::RegenPerFireResist(p)
            | Poe1Quantity::RecoveryRate(p)
            | Poe1Quantity::RegenReason(p, _),
        ) => p == pool,
        // Regeneration per charge (P6-11t and P6-11bq, [`super::charges`]).
        Quantity::Poe1(Poe1Quantity::Charge(_, ChargeValue::Per(per))) => matches!(
            (per, pool),
            (
                PerCharge::LifeRegenPercentPerMinute | PerCharge::LifeRegenPerMinute,
                Pool::Life
            ) | (PerCharge::ManaRegen, Pool::Mana)
        ),
        _ => false,
    }
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

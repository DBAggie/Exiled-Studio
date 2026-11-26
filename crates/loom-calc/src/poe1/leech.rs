//! The leech caps (Phase 6 P6-10b, part 7): PoB v2.67.2's
//! `Max<Pool>LeechRate`, `Max<Pool>LeechInstance`, and
//! `MaxLifeLeechRatePercent` (`Modules/CalcDefence.lua` 1201 to 1233);
//! and the main skill's leech rates (part 9, [`rates`]).
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
use super::{LeechSource, Poe1Field, Poe1Quantity, Pool};
use crate::condition::{Flags, QueryContext};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::{IndeterminateReason, Reasons};

/// The pools in PoB's order, each with its field and the character
/// constants of its rate cap (percent per minute) and its instance cap.
const CAPS: [(Pool, Field, &str, &str); 3] = [
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
    for (pool, pool_field, rate_stat, instance_stat) in CAPS {
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

/// The pools a hit leeches, in PoB's order (`CalcOffence.lua` 3982 to
/// 3990).
pub(super) const POOLS: [Pool; 3] = [Pool::Life, Pool::EnergyShield, Pool::Mana];

/// PoB's `data.misc.LeechRateBase`: one leech instance recovers 2% of
/// the pool per second (`Modules/Data.lua` 254).
pub const LEECH_RATE_BASE: f64 = 0.02;

/// The pool's field (its maximum).
const fn pool_field(pool: Pool) -> Field {
    match pool {
        Pool::Life => Field::Life,
        Pool::Mana => Field::Mana,
        Pool::EnergyShield => Field::EnergyShield,
    }
}

/// Whether `ty` is an elemental type.
const fn elemental(ty: DamageType) -> bool {
    matches!(
        ty,
        DamageType::Fire | DamageType::Cold | DamageType::Lightning
    )
}

/// The percent of `ty`'s damage the hit leeches as `pool`: PoB's
/// `Sum("BASE", cfg, "DamageLeech", "Damage<Pool>Leech",
/// "<Type>Damage<Pool>Leech", "ElementalDamage<Pool>Leech")`
/// (`CalcOffence.lua` 3529 to 3539).
fn percent_of(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    pool: Pool,
    ty: DamageType,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let mut sources = vec![LeechSource::Any, LeechSource::Type(ty)];
    if elemental(ty) {
        sources.push(LeechSource::Elemental);
    }
    let mut terms = Vec::with_capacity(sources.len());
    for source in sources {
        terms.push(super::hit::sum_of(
            calc,
            Quantity::Poe1(Poe1Quantity::DamageLeech(pool, source)),
            Op::Base,
            cx,
            Term::Step("leech-percent"),
            reasons,
        )?);
    }
    phases::total(calc, &terms, Term::Step("leech-percent-sum"))
}

/// The main skill's leech rates (P6-10b part 9): PoB's `<Pool>LeechRate`
/// (`CalcOffence.lua` 3296 to 3667 and 3981 to 4005), for a spell's or a
/// one-handed attack's hit. For each pool the hit leeches:
/// - each pass (crits, non-crits): `sum over types of the type's average
///   against the enemy x its leech percent / 100`, nothing when the skill
///   cannot leech the pool (`CannotLeech<Pool>`), at most the pool's
///   instance cap; then weighted by the crit chance;
/// - the instant share (`Instant<Pool>Leech`, 0 to 100) taken out;
/// - `duration = leech / pool / LeechRateBase`, `instances = duration x
///   hitRate`, `hitRate = HitChance / 100 x Speed`;
/// - `instanceRate = pool x LeechRateBase x (1 + inc / 100) x more` of
///   `<Pool>LeechRate`; `rate = instances x instanceRate`;
/// - `<Pool>LeechRate = instant x hitRate + min(rate, Max<Pool>LeechRate)
///   x <Pool>RecoveryRateMod`.
///
/// The field exists only when the hit leeches the pool. Not modelled, each
/// an unmapped stat and so a reason: Ghost Reaver, Immortal Ambition, leech
/// conditioned on a crit, the enemy's own leech lines, totems', traps', and
/// mines' leech to the player.
///
/// # Errors
/// A graph error.
pub(super) fn rates(
    calc: &mut Calc<'_>,
    hit: &super::ailment::AilmentHit<'_>,
    unvetted: &Reasons,
) -> Result<(), CalcError> {
    for pool in POOLS {
        let cx = calc.context(hit.flags);
        let mut reasons = pool_reasons(calc, pool, unvetted);
        let Some(caps) = caps(calc, pool) else {
            continue;
        };
        let cannot = cannot_leech(calc, pool, &cx, &mut reasons)?;
        let Some(hand) = hand_leech(calc, hit, hit.rate, pool, &cx, cannot, &caps, &mut reasons)?
        else {
            continue;
        };
        finish(calc, pool, &cx, &caps, &hand, reasons)?;
    }
    Ok(())
}

/// A dual wielder's leech rates (open items S7): each hand's pass as
/// [`rates`] up to its instances and instant rate, at PoB's `hitRate` of
/// that hand's chance to hit and the hands' combined `speed`
/// (`globalOutput.Speed`, `CalcOffence.lua` 3211); then PoB's `combineStat`
/// `DPS` (3877 to 3891, 2099 to 2103): `(main + off) / 2`, the sum when the
/// skill hits with both (`double`); then the rate as [`rates`], in the main
/// hand's context (PoB's `skillCfg`, which no modelled leech rate modifier
/// tells from a hand's). A hand that does not leech the pool adds 0.
///
/// # Errors
/// A graph error.
pub(super) fn rates_dual(
    calc: &mut Calc<'_>,
    hands: [&super::ailment::AilmentHit<'_>; 2],
    speed: Valued,
    double: bool,
    unvetted: &Reasons,
) -> Result<(), CalcError> {
    for pool in POOLS {
        let cx = calc.context(hands[0].flags);
        let mut reasons = pool_reasons(calc, pool, unvetted);
        let Some(caps) = caps(calc, pool) else {
            continue;
        };
        let mut each = Vec::with_capacity(2);
        for hand in hands {
            let hand_cx = calc.context(hand.flags);
            let cannot = cannot_leech(calc, pool, &hand_cx, &mut reasons)?;
            each.push(hand_leech(
                calc,
                hand,
                speed,
                pool,
                &hand_cx,
                cannot,
                &caps,
                &mut reasons,
            )?);
        }
        if each.iter().all(Option::is_none) {
            continue;
        }
        let zero = phases::constant(calc, "zero", 0.0)?;
        let two = phases::constant(calc, "two", 2.0)?;
        let combine = |calc: &mut Calc<'_>, pick: fn(&HandLeech) -> Valued| {
            let terms: Vec<Valued> = each.iter().map(|h| h.as_ref().map_or(zero, pick)).collect();
            let sum = phases::total(calc, &terms, Term::Step("both-hands"))?;
            if double {
                Ok::<_, CalcError>(sum)
            } else {
                quotient(calc, sum, two, Term::Step("hands-average"))
            }
        };
        let instances = combine(calc, |h| h.instances)?;
        let instant_rate = combine(calc, |h| h.instant_rate)?;
        let base = phases::constant(calc, "leech-rate-base", LEECH_RATE_BASE)?;
        let combined = HandLeech {
            instances,
            instant_rate,
            base,
        };
        finish(calc, pool, &cx, &caps, &combined, reasons)?;
    }
    Ok(())
}

/// The pool's leech reasons: the markers of its quantities, then the
/// hit's.
fn pool_reasons(calc: &Calc<'_>, pool: Pool, unvetted: &Reasons) -> Reasons {
    let mut reasons = phases::marker_reasons(
        calc,
        |_| None,
        |q| {
            matches!(
                q,
                Quantity::Poe1(
                    Poe1Quantity::DamageLeech(p, _)
                        | Poe1Quantity::LeechRate(p)
                        | Poe1Quantity::InstantLeech(p)
                        | Poe1Quantity::CannotLeech(p)
                ) if p == pool
            )
        },
    );
    reasons.extend(unvetted);
    reasons
}

/// The pool and its two caps, the fields the rate reads.
struct Caps {
    pool_value: Valued,
    instance_cap: Valued,
    rate_cap: Valued,
}

fn caps(calc: &Calc<'_>, pool: Pool) -> Option<Caps> {
    let read = |f: Field| {
        calc.field(f).and_then(|r| {
            Some(Valued {
                node: r.node?,
                value: r.value?,
            })
        })
    };
    Some(Caps {
        pool_value: read(pool_field(pool))?,
        instance_cap: read(Field::Poe1(Poe1Field::MaxLeechInstance(pool)))?,
        rate_cap: read(Field::Poe1(Poe1Field::MaxLeechRate(pool)))?,
    })
}

/// `CannotLeech<Pool>` in `cx`.
fn cannot_leech(
    calc: &mut Calc<'_>,
    pool: Pool,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<bool, CalcError> {
    Ok(super::hit::flag_of(
        calc,
        Quantity::Poe1(Poe1Quantity::CannotLeech(pool)),
        cx,
        reasons,
    )?
    .is_some())
}

/// One pass's leech instances and instant rate, and the leech rate base's
/// node.
struct HandLeech {
    instances: Valued,
    instant_rate: Valued,
    base: Valued,
}

/// One hand's (or a spell's) leech up to its instances and instant rate
/// ([`rates`]); `None` when it leeches nothing of the pool. `rate` is the
/// speed of PoB's `hitRate`.
#[expect(
    clippy::too_many_arguments,
    reason = "one pass's inputs, shared by the one-hand and dual-wield paths"
)]
fn hand_leech(
    calc: &mut Calc<'_>,
    hit: &super::ailment::AilmentHit<'_>,
    rate: Valued,
    pool: Pool,
    cx: &QueryContext,
    cannot: bool,
    caps: &Caps,
    reasons: &mut Reasons,
) -> Result<Option<HandLeech>, CalcError> {
    // Each pass's total, capped per instance.
    let mut any = false;
    let mut passes = Vec::with_capacity(2);
    for (index, pass) in hit.sources.against.iter().enumerate() {
        let mut terms = Vec::new();
        for &(ty, against) in pass {
            let percent = percent_of(calc, cx, pool, ty, reasons)?;
            if percent.value > 0.0 && !cannot {
                any = true;
                let product = phases::product(calc, &[against, percent], Term::Step("leeched"))?;
                let hundred = phases::constant(calc, "percent", 100.0)?;
                terms.push(quotient(
                    calc,
                    product,
                    hundred,
                    Term::Step("leech-of-type"),
                )?);
            }
        }
        passes.push((
            index == 1,
            phases::total(calc, &terms, Term::Step("leech-total"))?,
        ));
    }
    if !any {
        return Ok(None);
    }
    let one = phases::constant(calc, "one", 1.0)?;
    let mut weighted = Vec::with_capacity(2);
    for (crit, total) in passes {
        let capped = phases::pair(
            calc,
            total,
            caps.instance_cap,
            Term::Step("leech-instance-capped"),
            true,
        )?;
        let crit_share = super::hit::percent_of(calc, hit.crit, Term::Step("crit-share"))?;
        let portion = if crit {
            crit_share
        } else {
            super::hit::minus(calc, one, crit_share, Term::Step("non-crit-share"))?
        };
        weighted.push(phases::product(
            calc,
            &[capped, portion],
            Term::Step("leech-pass"),
        )?);
    }
    let leech = phases::total(calc, &weighted, Term::Step("leech-per-hit"))?;
    // The instant share, `max(min(sum, 100), 0) / 100`.
    let instant_sum = super::hit::sum_of(
        calc,
        Quantity::Poe1(Poe1Quantity::InstantLeech(pool)),
        Op::Base,
        cx,
        Term::Step("instant-leech"),
        reasons,
    )?;
    let hundred = phases::constant(calc, "percent", 100.0)?;
    let zero = phases::constant(calc, "zero", 0.0)?;
    let at_most = phases::pair(
        calc,
        instant_sum,
        hundred,
        Term::Step("instant-at-most"),
        true,
    )?;
    let at_least = phases::pair(calc, at_most, zero, Term::Step("instant-at-least"), false)?;
    let proportion = quotient(calc, at_least, hundred, Term::Step("instant-proportion"))?;
    let (instant, leech) = if proportion.value > 0.0 {
        let instant = phases::product(calc, &[leech, proportion], Term::Step("instant"))?;
        let kept = super::hit::minus(calc, one, proportion, Term::Step("not-instant"))?;
        (
            instant,
            phases::product(calc, &[leech, kept], Term::Step("leech-over-time"))?,
        )
    } else {
        (zero, leech)
    };
    // `hitRate = HitChance / 100 x Speed x dpsMultiplier` (1).
    let chance = super::hit::percent_of(calc, hit.hit_chance, Term::Step("hit-share"))?;
    let hit_rate = phases::product(calc, &[chance, rate], Term::Step("hit-rate"))?;
    let base = phases::constant(calc, "leech-rate-base", LEECH_RATE_BASE)?;
    // `getLeechInstances`: 0 when the pool is 0.
    let instances = if caps.pool_value.value == 0.0 {
        zero
    } else {
        let per_pool = quotient(calc, leech, caps.pool_value, Term::Step("leech-of-pool"))?;
        let duration = quotient(calc, per_pool, base, Term::Step("leech-duration"))?;
        phases::product(calc, &[duration, hit_rate], Term::Step("leech-instances"))?
    };
    let instant_rate = phases::product(calc, &[instant, hit_rate], Term::Step("instant-rate"))?;
    Ok(Some(HandLeech {
        instances,
        instant_rate,
        base,
    }))
}

/// The rate from the instances and instant rate ([`rates`]), set as the
/// pool's field.
fn finish(
    calc: &mut Calc<'_>,
    pool: Pool,
    cx: &QueryContext,
    caps: &Caps,
    hand: &HandLeech,
    mut reasons: Reasons,
) -> Result<(), CalcError> {
    let (inc, more) = super::hit::modifiers_of(
        calc,
        Quantity::Poe1(Poe1Quantity::LeechRate(pool)),
        cx,
        &mut reasons,
    )?;
    let instance_rate = phases::product(
        calc,
        &[caps.pool_value, hand.base, inc, more],
        Term::Step("leech-instance-rate"),
    )?;
    let rate = phases::product(
        calc,
        &[hand.instances, instance_rate],
        Term::Step("leech-rate"),
    )?;
    let capped = phases::pair(
        calc,
        rate,
        caps.rate_cap,
        Term::Step("leech-rate-capped"),
        true,
    )?;
    let recovery = super::regen::recovery_rate(calc, pool, &mut reasons)?;
    let recovered = phases::product(calc, &[capped, recovery], Term::Step("leech-recovered"))?;
    let value = phases::total(calc, &[hand.instant_rate, recovered], Term::Result)?;
    calc.set_derived(
        Field::Poe1(Poe1Field::LeechRate(pool)),
        phases::query(value, reasons),
        &[
            pool_field(pool),
            Field::Poe1(Poe1Field::MaxLeechInstance(pool)),
            Field::Poe1(Poe1Field::MaxLeechRate(pool)),
            Field::CritChance,
            Field::Speed,
        ],
    );
    Ok(())
}

/// `a / b` as one graph node.
fn quotient(calc: &mut Calc<'_>, a: Valued, b: Valued, term: Term) -> Result<Valued, CalcError> {
    super::hit::ratio(calc, a, b, term)
}

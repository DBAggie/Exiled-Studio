//! The main skill's costs (Phase 6 P6-10b, parts 4 and 8): PoB v2.67.2's
//! `ManaCost`, `LifeCost`, and `ESCost` (`Modules/CalcOffence.lua` 1677
//! to 1800) and their costs per second (5794 to 5825).
//!
//! What PoB does, for the upfront costs in its order (mana, life, energy
//! shield):
//! - the base: the skill level's cost of the resource (the granted effect
//!   level's costs, divisor 1), rounded to two decimals;
//! - `finalBaseCost = floor(base x mult) + <Resource>CostNoMult`, `mult`
//!   the supports' cost multipliers, `floor(More(SupportManaMultiplier),
//!   4)` ([`super::gems::support_multiplier`], as reservation's);
//! - the conversions, in the first pass: with `CostLifeInsteadOfMana`
//!   (Lifetap, "Skills Cost Life instead of Mana") life's final base
//!   gains mana's and mana's becomes 0; otherwise, with
//!   `ManaCostAsLifeCost` or `HybridManaAndLifeCost_Life` above 0 and no
//!   `CostESInsteadOfManaOrLife`, life's gains `round(mana's x (hybrid +
//!   additional) / 100)`. Energy shield's the same with
//!   `CostESInsteadOfManaOrLife` (gaining mana's and life's) or
//!   `ManaCostAsEnergyShieldCost`;
//! - then `(1 + inc / 100)`, `inc` the `INC` sum of `<Resource>Cost` and
//!   `Cost`, rounded up when `inc` is below 0 and down otherwise; the
//!   resource's more, then the `Cost` more, each rounded up below 1 and
//!   down otherwise ([`rounded`]); divided by the cost efficiency,
//!   `calcLib.mod` of `<Resource>CostEfficiency` and `CostEfficiency`
//!   (`(1 + inc / 100) x more`), held at 0 or above; plus the total, the
//!   `BASE` sum of `<Resource>Cost` and `Cost`; never below 0. An
//!   efficiency at or below 0 leaves the field unsupported. Mana's total
//!   also holds "Channelling Skills have +# to Total Mana Cost" or the
//!   non-channelling one, by the main skill's `Channel` type (PoB's
//!   `ManaCost` BASE with a `SkillType` tag; P6-11t);
//! - mana's last, with `HybridManaAndLifeCost_Life` above 0: `floor((1 -
//!   hybrid / 100) x cost)`.
//!
//! Not modelled, so reasons on the cost fields only
//! ([`super::mapping::COST_REASONS`] and the skill's own data): a cost in
//! another resource (rage, souls, percentages, per second), skills whose
//! reservation becomes their cost, and conditional or socketed cost lines.
//! PoB's `<Resource>CostBase` (Archmage's and Frostmage's base cost from
//! mana), `Base<Resource>CostOverride`, and `HasNoCost` have no mapped
//! stat: their stats stay `unmapped-stat` reasons (every field's). The
//! Blood Magic keystone (which also removes mana and reserves life) is an
//! unmapped stat.

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

/// An upfront cost's resource, in PoB's order (`costs.order`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resource {
    Mana,
    Life,
    EnergyShield,
}

impl Resource {
    const ALL: [Self; 3] = [Self::Mana, Self::Life, Self::EnergyShield];

    /// The resource's name in the skill level's costs (PoB's `Costs.lua`).
    const fn data_name(self) -> &'static str {
        match self {
            Self::Mana => "Mana",
            Self::Life => "Life",
            Self::EnergyShield => "ES",
        }
    }

    /// PoB's `<Resource>Cost` quantity.
    const fn cost(self) -> Poe1Quantity {
        match self {
            Self::Mana => Poe1Quantity::ManaCost,
            Self::Life => Poe1Quantity::LifeCost,
            Self::EnergyShield => Poe1Quantity::EnergyShieldCost,
        }
    }

    /// PoB's `<Resource>CostEfficiency` quantity.
    const fn efficiency(self) -> Poe1Quantity {
        match self {
            Self::Mana => Poe1Quantity::ManaCostEfficiency,
            Self::Life => Poe1Quantity::LifeCostEfficiency,
            Self::EnergyShield => Poe1Quantity::EnergyShieldCostEfficiency,
        }
    }

    const fn field(self) -> Field {
        Field::Poe1(match self {
            Self::Mana => Poe1Field::ManaCost,
            Self::Life => Poe1Field::LifeCost,
            Self::EnergyShield => Poe1Field::EnergyShieldCost,
        })
    }

    const fn per_second(self) -> Field {
        Field::Poe1(match self {
            Self::Mana => Poe1Field::ManaPerSecondCost,
            Self::Life => Poe1Field::LifePerSecondCost,
            Self::EnergyShield => Poe1Field::EnergyShieldPerSecondCost,
        })
    }
}

/// The cost phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let unsupported = |calc: &mut Calc<'_>, reason: UnsupportedReason| {
        for r in Resource::ALL {
            calc.set_unsupported(r.field(), reason.clone());
        }
    };
    let Ok(skill) = super::skill::main_skill(input.build, input.data, &levels) else {
        unsupported(calc, UnsupportedReason::NotComputed);
        return Ok(());
    };
    let source = Source::Gem {
        group: skill.group,
        gem: skill.active_gem.gem,
    };
    // A skill PoB disables for the weapons held has no offence output, its
    // cost included (`CalcOffence.lua` 339).
    match skill.disabled_by_hands(&super::collect::hands(&input)) {
        Ok(false) => {}
        Ok(true) => {
            unsupported(calc, UnsupportedReason::NotComputed);
            return Ok(());
        }
        Err(what) => {
            unsupported(calc, UnsupportedReason::UnmodelledSkill { source, what });
            return Ok(());
        }
    }
    let Some(level) = super::skill::effect_level(skill.effect(), skill.active_gem.level) else {
        unsupported(
            calc,
            UnsupportedReason::UnmodelledSkill {
                source,
                what: "no-level",
            },
        );
        return Ok(());
    };
    let mut reasons = phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::CostReason(stat)) => Some(stat),
            _ => None,
        },
        is_cost,
    );
    let mut amounts = [0; 3];
    for c in &level.costs {
        match Resource::ALL
            .iter()
            .position(|r| r.data_name() == c.resource)
        {
            Some(i) => amounts[i] += c.amount,
            None => {
                reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
                    source: source.clone(),
                    what: "cost-resource",
                });
            }
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
    let channelling = skill.types.contains("Channel");
    let mult = super::gems::support_multiplier(&skill.supports, &skill.cost_mores);
    let mult = phases::constant(calc, "support-cost-multiplier", mult)?;
    // The first pass: each resource's final base cost.
    let mut finals = Vec::with_capacity(3);
    for (r, amount) in Resource::ALL.into_iter().zip(amounts) {
        #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
        let base = phases::constant(calc, "base-cost", amount as f64)?;
        let scaled = phases::product(calc, &[base, mult], Term::Step("multiplied"))?;
        let floored = round(calc, scaled, false, Term::Step("multiplied-floor"))?;
        let final_base = if r == Resource::Mana {
            let no_mult = sum(
                calc,
                Quantity::Poe1(Poe1Quantity::ManaCostNoMult),
                Op::Base,
                flags,
                &mut reasons,
            )?;
            phases::total(calc, &[floored, no_mult], Term::Step("final-base"))?
        } else {
            floored
        };
        finals.push(final_base);
    }
    let [mut mana, mut life, mut es] = [finals[0], finals[1], finals[2]];
    let life_instead = flag(
        calc,
        Poe1Quantity::CostLifeInsteadOfMana,
        flags,
        &mut reasons,
    )?;
    let es_instead = flag(
        calc,
        Poe1Quantity::CostEnergyShieldInsteadOfManaOrLife,
        flags,
        &mut reasons,
    )?;
    let hybrid = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::HybridLifeCost),
        Op::Base,
        flags,
        &mut reasons,
    )?;
    let as_life = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::ManaCostAsLifeCost),
        Op::Base,
        flags,
        &mut reasons,
    )?;
    let as_es = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::ManaCostAsEnergyShieldCost),
        Op::Base,
        flags,
        &mut reasons,
    )?;
    let zero = phases::constant(calc, "moved-cost", 0.0)?;
    if life_instead {
        life = phases::total(calc, &[life, mana], Term::Step("life-instead-of-mana"))?;
        mana = zero;
    } else if (hybrid.value > 0.0 || as_life.value > 0.0) && !es_instead {
        let share = phases::total(calc, &[hybrid, as_life], Term::Step("life-share"))?;
        let added = share_of(calc, mana, share)?;
        life = phases::total(calc, &[life, added], Term::Step("life-from-mana"))?;
    }
    if es_instead {
        es = phases::total(calc, &[es, mana, life], Term::Step("energy-shield-instead"))?;
        mana = zero;
        life = zero;
    } else if as_es.value > 0.0 {
        let added = share_of(calc, mana, as_es)?;
        es = phases::total(calc, &[es, added], Term::Step("energy-shield-from-mana"))?;
    }
    // Eldritch Battery (P6-11ap; `CalcOffence.lua` 5793 to 5825): the mana
    // cost per second is paid with energy shield (`ESPerSecondCost`), and
    // the energy shield cost's own is then skipped unless mana's cost is
    // also moved to energy shield (a sum the engine leaves as a reason).
    let eb = phases::has(calc, Quantity::Poe1(Poe1Quantity::EsProtectsMana))
        && flag(calc, Poe1Quantity::EsProtectsMana, flags, &mut reasons)?;
    let mut es_from_mana = false;
    // The second pass: each resource's modifiers.
    for (r, final_base) in [
        (Resource::Mana, mana),
        (Resource::Life, life),
        (Resource::EnergyShield, es),
    ] {
        let mut reasons = reasons.clone();
        let Some(mut cost) = modified(calc, r, final_base, flags, channelling, &mut reasons)?
        else {
            calc.set_unsupported(
                r.field(),
                UnsupportedReason::UnmodelledSkill {
                    source: source.clone(),
                    what: "cost-efficiency",
                },
            );
            continue;
        };
        if r == Resource::Mana && hybrid.value > 0.0 {
            // `max(0, floor((1 - hybrid / 100) x cost))` (1786 to 1788).
            let hundredth = phases::constant(calc, "percent", -0.01)?;
            let part = phases::product(calc, &[hybrid, hundredth], Term::Step("hybrid-part"))?;
            let one = phases::constant(calc, "one", 1.0)?;
            let kept = phases::total(calc, &[one, part], Term::Step("hybrid-kept"))?;
            let scaled = phases::product(calc, &[kept, cost], Term::Step("hybrid-scaled"))?;
            cost = round(calc, scaled, false, Term::Result)?;
        }
        calc.set_derived(r.field(), phases::query(cost, reasons), &[]);
        match r {
            Resource::Mana if eb => {
                es_from_mana = per_second(calc, &skill, r, Resource::EnergyShield, cost, &source)?;
            }
            Resource::EnergyShield if es_from_mana => {
                if as_es.value != 0.0 && cost.value > 0.0 {
                    calc.set_unsupported(
                        r.per_second(),
                        UnsupportedReason::UnmodelledSkill {
                            source: source.clone(),
                            what: "eldritch-battery-cost-per-second",
                        },
                    );
                }
            }
            _ => {
                per_second(calc, &skill, r, r, cost, &source)?;
            }
        }
    }
    Ok(())
}

/// PoB's `round(mana x share / 100)`, the cost moved from mana.
fn share_of(calc: &mut Calc<'_>, mana: Valued, share: Valued) -> Result<Valued, CalcError> {
    let hundredth = phases::constant(calc, "percent", 0.01)?;
    let raw = phases::product(calc, &[mana, share, hundredth], Term::Step("moved-raw"))?;
    phases::round(calc, raw, Term::Step("moved"))
}

/// A resource's cost from its final base cost (module docs, the second
/// pass), or `None` when its efficiency is at or below 0. `channelling`:
/// whether the main skill is a channelling skill (mana's total).
fn modified(
    calc: &mut Calc<'_>,
    r: Resource,
    final_base: Valued,
    flags: Flags,
    channelling: bool,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let own = Quantity::Poe1(r.cost());
    let any = Quantity::Poe1(Poe1Quantity::Cost);
    // Increased: one sum of both names.
    let inc_own = sum(calc, own, Op::Increased, flags, reasons)?;
    let inc_any = sum(calc, any, Op::Increased, flags, reasons)?;
    let inc = phases::total(calc, &[inc_own, inc_any], Term::Step("increased"))?;
    let hundredth = phases::constant(calc, "percent", 0.01)?;
    let one = phases::constant(calc, "one", 1.0)?;
    let inc_part = phases::product(calc, &[inc, hundredth], Term::Step("increased-part"))?;
    let inc_factor = phases::total(calc, &[one, inc_part], Term::Step("increased-factor"))?;
    let up = inc.value < 0.0;
    let mut cost = phases::product(
        calc,
        &[inc_factor, final_base],
        Term::Step("after-increased"),
    )?;
    cost = round(calc, cost, up, Term::Step("after-increased-rounded"))?;
    for q in [own, any] {
        let more = more(calc, q, flags, reasons)?;
        let up = more.value < 1.0;
        cost = phases::product(calc, &[more, cost], Term::Step("after-more"))?;
        cost = round(calc, cost, up, Term::Step("after-more-rounded"))?;
    }
    // The cost efficiency, `calcLib.mod` of both names.
    let eff_own = Quantity::Poe1(r.efficiency());
    let eff_any = Quantity::Poe1(Poe1Quantity::CostEfficiency);
    let eff_a = sum(calc, eff_own, Op::Increased, flags, reasons)?;
    let eff_b = sum(calc, eff_any, Op::Increased, flags, reasons)?;
    let eff = phases::total(calc, &[eff_a, eff_b], Term::Step("efficiency"))?;
    let eff_part = phases::product(calc, &[eff, hundredth], Term::Step("efficiency-part"))?;
    let eff_inc = phases::total(calc, &[one, eff_part], Term::Step("efficiency-increased"))?;
    let eff_more_own = more(calc, eff_own, flags, reasons)?;
    let eff_more_any = more(calc, eff_any, flags, reasons)?;
    let eff_factor = phases::product(
        calc,
        &[eff_inc, eff_more_own, eff_more_any],
        Term::Step("efficiency-factor"),
    )?;
    // An efficiency at or below 0: PoB divides by it (a cost no build
    // reaches); the field says so rather than giving infinity.
    if eff_factor.value <= 0.0 {
        return Ok(None);
    }
    let value = cost.value / eff_factor.value;
    let node = calc.graph_mut().push(
        Node::Quotient {
            term: Term::Step("after-efficiency"),
            a: cost.node,
            b: eff_factor.node,
        },
        value,
    )?;
    // `max(0, cost / efficiency)` before the total (`CalcOffence.lua` 1779).
    let zero = phases::constant(calc, "no-negative-cost", 0.0)?;
    cost = phases::pair(
        calc,
        Valued { node, value },
        zero,
        Term::Step("after-efficiency-not-negative"),
        false,
    )?;
    let total_own = sum(calc, own, Op::Base, flags, reasons)?;
    let total_any = sum(calc, any, Op::Base, flags, reasons)?;
    let mut terms = vec![cost, total_own, total_any];
    let by_kind = Quantity::Poe1(Poe1Quantity::ManaCostChannelling(channelling));
    if r == Resource::Mana && phases::has(calc, by_kind) {
        terms.push(sum(calc, by_kind, Op::Base, flags, reasons)?);
    }
    cost = phases::total(calc, &terms, Term::Step("with-total"))?;
    let zero = phases::constant(calc, "no-negative-cost", 0.0)?;
    Ok(Some(phases::pair(calc, cost, zero, Term::Result, false)?))
}

/// A resource's cost per second (P6-10b parts 6 and 8; PoB's
/// `<Resource>PerSecondCost`, `CalcOffence.lua` 5794 to 5825): for a cost
/// above 0, the cost times the skill's `Speed` over its repeat count
/// (each repeat is a use that costs nothing; P6-11aj). A trap's, a mine's, or a totem's (their throwing,
/// laying, or placement speed) is not modelled and leaves the field
/// unsupported. Eldritch Battery pays the mana cost with energy shield
/// (`into`, P6-11ap). No field without a cost or a computed `Speed`.
/// Whether PoB sets the field (its `<into>PerSecondHasCost`).
fn per_second(
    calc: &mut Calc<'_>,
    skill: &super::skill::MainSkill<'_>,
    r: Resource,
    into: Resource,
    cost: Valued,
    source: &Source,
) -> Result<bool, CalcError> {
    let field = into.per_second();
    if cost.value <= 0.0 {
        return Ok(false);
    }
    if ["Trapped", "RemoteMined", "SummonsTotem"]
        .iter()
        .any(|t| skill.types.contains(*t))
    {
        calc.set_unsupported(
            field,
            UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: "cost-per-second",
            },
        );
        return Ok(true);
    }
    let Some(speed) = calc.field(Field::Speed).and_then(|r| {
        Some(Valued {
            node: r.node?,
            value: r.value?,
        })
    }) else {
        return Ok(false);
    };
    // PoB's `useSpeed = Speed / repeats`, then the cost times it.
    let repeats = phases::constant(calc, "repeats", super::repeats::count(calc))?;
    let use_speed = speed.value / repeats.value;
    let node = calc.graph_mut().push(
        Node::Quotient {
            term: Term::Step("use-speed"),
            a: speed.node,
            b: repeats.node,
        },
        use_speed,
    )?;
    let use_speed = Valued {
        node,
        value: use_speed,
    };
    let value = phases::product(calc, &[cost, use_speed], Term::Result)?;
    calc.set_derived(
        field,
        phases::query(value, Reasons::new()),
        &[r.field(), Field::Speed],
    );
    Ok(true)
}

/// Whether `quantity` is one the costs read.
const fn is_cost(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(
            Poe1Quantity::ManaCost
                | Poe1Quantity::LifeCost
                | Poe1Quantity::EnergyShieldCost
                | Poe1Quantity::Cost
                | Poe1Quantity::ManaCostNoMult
                | Poe1Quantity::ManaCostEfficiency
                | Poe1Quantity::LifeCostEfficiency
                | Poe1Quantity::EnergyShieldCostEfficiency
                | Poe1Quantity::CostEfficiency
                | Poe1Quantity::CostLifeInsteadOfMana
                | Poe1Quantity::CostEnergyShieldInsteadOfManaOrLife
                | Poe1Quantity::HybridLifeCost
                | Poe1Quantity::ManaCostAsLifeCost
                | Poe1Quantity::ManaCostAsEnergyShieldCost
                | Poe1Quantity::EsProtectsMana
                | Poe1Quantity::CostReason(_)
        )
    )
}

/// Whether the flag `quantity` is set in the skill's context.
fn flag(
    calc: &mut Calc<'_>,
    quantity: Poe1Quantity,
    flags: Flags,
    reasons: &mut Reasons,
) -> Result<bool, CalcError> {
    let cx = calc.context(flags);
    let (store, graph) = calc.parts();
    let q = store.flag(graph, Quantity::Poe1(quantity), &cx)?;
    reasons.extend(&q.reasons);
    Ok(q.value > 0.0)
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
    let value = rounded(x.value, up);
    let node = if up {
        Node::Ceil { term, of: x.node }
    } else {
        Node::Floor { term, of: x.node }
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

/// PoB's rounding of a cost step: `ceil` when `up` (a net reduction:
/// increased below 0, a more below 1), else `floor`. The step is then
/// held at 0 or above ([`round`]).
#[must_use]
pub fn rounded(x: f64, up: bool) -> f64 {
    if up {
        crate::math::ceil(x)
    } else {
        crate::math::floor(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A net reduction rounds up and an increase down, as PoB's
    /// `m_ceil`/`m_floor` (`CalcOffence.lua` 1764 to 1777).
    #[test]
    fn a_cost_step_should_round_toward_the_base() {
        // 23 x 0.88 = 20.24: reduced, so up to 21.
        assert_eq!(rounded(23.0 * 0.88, true), 21.0);
        // 23 x 1.1 = 25.3: increased, so down to 25.
        assert_eq!(rounded(23.0 * 1.1, false), 25.0);
        // A whole value stays.
        assert_eq!(rounded(20.0, true), 20.0);
        assert_eq!(rounded(20.0, false), 20.0);
    }

    /// Every cost quantity, and only those, is the fields' to read
    /// (unsettled values reach them alone).
    #[test]
    fn the_cost_should_read_its_quantities_only() {
        for q in [
            Poe1Quantity::ManaCost,
            Poe1Quantity::LifeCost,
            Poe1Quantity::EnergyShieldCost,
            Poe1Quantity::Cost,
            Poe1Quantity::ManaCostNoMult,
            Poe1Quantity::ManaCostEfficiency,
            Poe1Quantity::LifeCostEfficiency,
            Poe1Quantity::EnergyShieldCostEfficiency,
            Poe1Quantity::CostEfficiency,
            Poe1Quantity::CostLifeInsteadOfMana,
            Poe1Quantity::CostEnergyShieldInsteadOfManaOrLife,
            Poe1Quantity::HybridLifeCost,
            Poe1Quantity::ManaCostAsLifeCost,
            Poe1Quantity::ManaCostAsEnergyShieldCost,
            Poe1Quantity::CostReason("x"),
        ] {
            assert!(is_cost(Quantity::Poe1(q)), "{q}");
        }
        assert!(!is_cost(Quantity::Poe1(Poe1Quantity::MovementSpeed)));
        assert!(!is_cost(Quantity::Damage));
    }

    /// Each resource names its level cost as PoB's `Costs.lua` does.
    #[test]
    fn each_resource_should_have_its_data_name() {
        let names: Vec<_> = Resource::ALL.iter().map(|r| r.data_name()).collect();
        assert_eq!(names, ["Mana", "Life", "ES"]);
    }
}

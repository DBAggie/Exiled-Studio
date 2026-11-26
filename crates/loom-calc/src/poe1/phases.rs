//! The Path of Exile phases of P3-6 and P3-7 (Phase 3 plan, steps 6 and
//! 7): attributes, life, mana, resistances, block, spell block, spell
//! suppression, energy shield, armour, evasion, and unreserved life and
//! mana.
//!
//! Each formula follows the game as PoB v2.67.2 computes it, cited per
//! phase, with the same order of operations (floating-point products
//! depend on it) and the same rounding points. Every value is a graph
//! node, built to `docs/phase-3/slice-fields.json` revision 2's `graph`
//! references: a field's node has [`Term::Result`]; below it the nearest
//! [`Term::Base`], [`Term::Increased`], and [`Term::More`] nodes are the
//! breakdown's terms; a defence row is `step.slot.<source>` (a static
//! name per slot, `ArmourSlot::row`), and an armour piece's own nodes are
//! `step.piece-*` so the search for a row's terms finds the row's.
//!
//! Attribute bonuses read the attribute's field node, so a derived field
//! carries its attribute's reasons (`Calc::set_derived`).

use super::{ArmourSlot, Defence, PieceValue, Poe1Field, Poe1Quantity, Pool, ReservationKind};
use crate::condition::Flags;
use crate::engine::{Calc, Phase};
use crate::error::CalcError;
use crate::graph::{Node, NodeIndex, Term};
use crate::math;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::{Reasons, StateKind};
use crate::store::Query;

/// The phases, in order.
pub const PHASES: [Phase; 21] = [
    Phase {
        name: "attributes",
        run: attributes,
    },
    Phase {
        name: "life-mana",
        run: life_mana,
    },
    Phase {
        name: "resistances",
        run: resistances,
    },
    Phase {
        name: "block",
        run: block,
    },
    Phase {
        name: "primary-defences",
        run: primary_defences,
    },
    Phase {
        name: "evade",
        run: super::evade::phase,
    },
    Phase {
        name: "unreserved",
        run: unreserved,
    },
    Phase {
        name: "damage-reduction",
        run: super::reduction::phase,
    },
    Phase {
        name: "damage-taken",
        run: super::taken::phase,
    },
    Phase {
        name: "enemy-hit",
        run: super::enemy_hit::phase,
    },
    Phase {
        name: "regeneration",
        run: super::regen::phase,
    },
    Phase {
        name: "leech",
        run: super::leech::phase,
    },
    Phase {
        name: "movement",
        run: super::movement::phase,
    },
    Phase {
        name: "minion",
        run: super::minion_actor::phase,
    },
    // The costs before the hit, as PoB computes them before the passes'
    // damage (open items S3: Manaforged Arrows' damage per mana spent);
    // their costs per second after the hit's and the damage over time's
    // speed.
    Phase {
        name: "costs",
        run: super::costs::phase,
    },
    Phase {
        name: "hit",
        run: super::hit::hit,
    },
    Phase {
        name: "dot",
        run: super::dot::dot,
    },
    Phase {
        name: "cost-per-second",
        run: super::costs::per_second_phase,
    },
    Phase {
        name: "combined",
        run: super::combined::phase,
    },
    Phase {
        name: "minion-hit",
        run: super::minion_hit::phase,
    },
    Phase {
        name: "full-dps",
        run: super::full_dps::phase,
    },
];

/// A node with its value.
#[derive(Debug, Clone, Copy)]
pub(super) struct Valued {
    pub(super) node: NodeIndex,
    pub(super) value: f64,
}

/// How a chain multiplies: PoB's life is `base * inc * more` (left to
/// right); `calcLib.val` and `calcLib.mod` give `base * (inc * more)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    /// `(base * inc) * more`.
    Left,
    /// `base * (inc * more)`.
    Right,
}

/// A chain's product (`base x increase factor x more factor`), with the
/// reasons the store's queries gave.
struct ChainParts {
    product: Valued,
    reasons: Reasons,
}

/// `base * (1 + inc / 100) * more` on `quantity`, with extra base and
/// increase terms the rules add (attribute bonuses). The product's term
/// is `term` (a field's chain is wrapped by its rounding).
fn chain(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    extra_base: &[Valued],
    extra_inc: &[Valued],
    order: Order,
    term: Term,
) -> Result<ChainParts, CalcError> {
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let effects = store.sum(graph, quantity, Op::Base, &cx, Term::Step("base-effects"))?;
    let mut base = 0.0 + effects.value;
    let mut terms = vec![effects.node];
    for e in extra_base {
        base += e.value;
        terms.push(e.node);
    }
    let base_node = graph.push(
        Node::Sum {
            term: Term::Base,
            terms,
        },
        base,
    )?;

    let inc_effects = store.sum(
        graph,
        quantity,
        Op::Increased,
        &cx,
        Term::Step("increased-effects"),
    )?;
    let mut inc = 0.0 + inc_effects.value;
    let mut terms = vec![inc_effects.node];
    for e in extra_inc {
        inc += e.value;
        terms.push(e.node);
    }
    let inc_sum = graph.push(
        Node::Sum {
            term: Term::Step("increased-sum"),
            terms,
        },
        inc,
    )?;
    let inc_factor = math::percent_factor(inc);
    let inc_node = graph.push(
        Node::PercentFactor {
            term: Term::Increased,
            of: inc_sum,
        },
        inc_factor,
    )?;

    // PoB rounds a list's more multiplier to two decimals
    // (`ModStore::more_rounded`; P6-7b3b and its review).
    let more = store.more_rounded(graph, quantity, &cx)?;
    let (value, factors) = match order {
        Order::Left => (
            base * inc_factor * more.value,
            vec![base_node, inc_node, more.node],
        ),
        Order::Right => {
            let modifier = inc_factor * more.value;
            let m = graph.push(
                Node::Product {
                    term: Term::Step("modifier"),
                    factors: vec![inc_node, more.node],
                },
                modifier,
            )?;
            (base * modifier, vec![base_node, m])
        }
    };
    let product = graph.push(Node::Product { term, factors }, value)?;
    let mut reasons = effects.reasons;
    reasons.extend(&inc_effects.reasons);
    reasons.extend(&more.reasons);
    Ok(ChainParts {
        product: Valued {
            node: product,
            value,
        },
        reasons,
    })
}

/// `round(x)` (PoB's `m_floor(x + 0.5)`) with `term`.
pub(super) fn round(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    let value = math::round_half_up(x.value);
    let node = calc
        .graph_mut()
        .push(Node::RoundHalfUp { term, of: x.node }, value)?;
    Ok(Valued { node, value })
}

/// `max(x, minimum)` as a field's result.
fn at_least(
    calc: &mut Calc<'_>,
    x: Valued,
    label: &'static str,
    minimum: f64,
) -> Result<Valued, CalcError> {
    let graph = calc.graph_mut();
    let b = graph.push(
        Node::Constant {
            label,
            value: minimum,
        },
        minimum,
    )?;
    let value = math::max(x.value, minimum);
    let node = graph.push(
        Node::Max {
            term: Term::Result,
            a: x.node,
            b,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// `floor(attribute * factor)`: an attribute's bonus. `factor` is `0.5`
/// for "per 2" (exact), `0.2` for "per 5", and `0.1` for "per 10": `0.2`
/// and `0.1` as doubles are a little above a fifth and a tenth, so for a
/// whole number `n` the product is at least `n / 5` (`n / 10`) and below
/// the next whole number; the floor equals PoB's `m_floor(n / 5)` and
/// `m_floor(n / 10)` (a test checks every `n` up to 100,000).
pub(super) fn bonus(
    calc: &mut Calc<'_>,
    attribute: Field,
    factor: f64,
    label: &'static str,
    term: &'static str,
) -> Result<Option<Valued>, CalcError> {
    // Supreme Prodigy's "Gain no inherent bonuses from Attributes"
    // (P6-11hx): only the keystone maps to the flag, unconditionally.
    if no_attribute_bonuses(calc) {
        return Ok(None);
    }
    let Some((node, value)) = calc
        .field(attribute)
        .and_then(|f| Some((f.node?, f.value?)))
    else {
        return Ok(None);
    };
    let graph = calc.graph_mut();
    let c = graph.push(
        Node::Constant {
            label,
            value: factor,
        },
        factor,
    )?;
    let scaled = value * factor;
    let p = graph.push(
        Node::Product {
            term: Term::Step("attribute-scaled"),
            factors: vec![node, c],
        },
        scaled,
    )?;
    let value = math::floor(scaled);
    let node = graph.push(
        Node::Floor {
            term: Term::Step(term),
            of: p,
        },
        value,
    )?;
    Ok(Some(Valued { node, value }))
}

pub(super) fn query(v: Valued, reasons: Reasons) -> Query {
    Query {
        value: v.value,
        node: v.node,
        reasons,
    }
}

/// Strength, dexterity, intelligence: `max(round(base * inc * more), 0)`
/// (`Modules/CalcPerform.lua`, `calculateAttributes`: `m_max(round(
/// calcLib.val(modDB, stat)), 0)`). The second pass PoB makes settles
/// attribute conditions, which no mapped stat has yet.
fn attributes(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for (field, quantity) in [
        (Field::Strength, Quantity::Strength),
        (Field::Dexterity, Quantity::Dexterity),
        (Field::Intelligence, Quantity::Intelligence),
    ] {
        let parts = chain(
            calc,
            quantity,
            &[],
            &[],
            Order::Right,
            Term::Step("unrounded"),
        )?;
        let rounded = round(calc, parts.product, Term::Step("rounded"))?;
        let value = at_least(calc, rounded, "minimum", 0.0)?;
        calc.set(field, query(value, parts.reasons));
    }
    Ok(())
}

/// Life: `max(round(base * inc * more), 1)` with `floor(Str / 2)` added to
/// the base (`Modules/CalcPerform.lua`: `Life` BASE `m_floor(output.Str /
/// 2)` from "Strength", then `doActorLifeMana`). Mana: `round(base * inc
/// * more)` with `floor(Int / 2)` added (`Mana` BASE from
/// "Intelligence"; `output.Mana = round(calcLib.val(modDB, "Mana"))`).
///
/// Chaos Inoculation (P6-11ej, `CalcPerform.lua` 81 to 83): life is 1,
/// its flag's node times 1. Life and mana conversion and overrides come
/// from stats the mapping does not know yet, which make the fields
/// `Partial`.
fn life_mana(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let mut flag_reasons = Reasons::new();
    let str_bonus = strength_life(calc, &mut flag_reasons)?;
    let parts = chain(
        calc,
        Quantity::Life,
        str_bonus.as_slice(),
        &[],
        Order::Left,
        Term::Step("unrounded"),
    )?;
    let rounded = round(calc, parts.product, Term::Step("rounded"))?;
    let life = at_least(calc, rounded, "minimum", 1.0)?;
    let mut reasons = parts.reasons;
    reasons.extend(&flag_reasons);
    let life = match chaos_inoculation(calc, &mut reasons)? {
        Some(flag) => {
            let one = constant(calc, "chaos-inoculation-life", 1.0)?;
            product(calc, &[flag, one], Term::Result)?
        }
        None => life,
    };
    calc.set_derived(Field::Life, query(life, reasons), &[Field::Strength]);

    let int_bonus = bonus(
        calc,
        Field::Intelligence,
        0.5,
        "half",
        "mana-from-intelligence",
    )?;
    let parts = chain(
        calc,
        Quantity::Mana,
        int_bonus.as_slice(),
        &[],
        Order::Right,
        Term::Step("unrounded"),
    )?;
    let mana = round(calc, parts.product, Term::Result)?;
    calc.set_derived(
        Field::Mana,
        query(mana, parts.reasons),
        &[Field::Intelligence],
    );
    Ok(())
}

/// Life from strength, `floor(Str / 2)`, unless "Strength provides no
/// bonus to Maximum Life" (P6-11gj; PoB's `NoStrBonusToLife`,
/// `CalcPerform.lua` 496).
fn strength_life(calc: &mut Calc<'_>, reasons: &mut Reasons) -> Result<Option<Valued>, CalcError> {
    if flag_of(calc, Poe1Quantity::NoStrBonusToLife, reasons)?.is_some() {
        return Ok(None);
    }
    bonus(calc, Field::Strength, 0.5, "half", "life-from-strength")
}

/// Chaos Inoculation's flag when it is on (P6-11ej; PoB's
/// `modDB:Flag(nil, "ChaosInoculation")`): its node, value 1.
fn chaos_inoculation(
    calc: &mut Calc<'_>,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    flag_of(calc, Poe1Quantity::ChaosInoculation, reasons)
}

/// A character flag when it is on (P6-11ek): its node, value 1.
fn flag_of(
    calc: &mut Calc<'_>,
    flag: Poe1Quantity,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(flag);
    if !has(calc, q) {
        return Ok(None);
    }
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let f = store.flag(graph, q, &cx)?;
    reasons.extend(&f.reasons);
    Ok((f.value != 0.0).then_some(Valued {
        node: f.node,
        value: f.value,
    }))
}

/// A minion's life (P6-7b3a, `doActorLifeMana` for `env.minion`):
/// `override or max(round(base x inc x more), 1)`, without the strength
/// bonus (a minion's Strength is 0 until `StrengthAddedToMinions`,
/// P6-7b3b).
pub(super) fn minion_life(calc: &mut Calc<'_>) -> Result<(Valued, Reasons), CalcError> {
    let parts = chain(
        calc,
        Quantity::Life,
        &[],
        &[],
        Order::Left,
        Term::Step("unrounded"),
    )?;
    let rounded = round(calc, parts.product, Term::Step("rounded"))?;
    let life = at_least(calc, rounded, "minimum", 1.0)?;
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let mut reasons = parts.reasons;
    match store.override_value(graph, Quantity::Life, &cx)? {
        Some(q) => {
            reasons.extend(&q.reasons);
            Ok((
                Valued {
                    node: q.node,
                    value: q.value,
                },
                reasons,
            ))
        }
        None => Ok((life, reasons)),
    }
}

/// A constant of a rule.
pub(super) fn constant(
    calc: &mut Calc<'_>,
    label: &'static str,
    value: f64,
) -> Result<Valued, CalcError> {
    let node = calc
        .graph_mut()
        .push(Node::Constant { label, value }, value)?;
    Ok(Valued { node, value })
}

/// The sum of `op` effects on `quantity`, with `term`.
pub(super) fn sum(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    op: Op,
    term: Term,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let q = store.sum(graph, quantity, op, &cx, term)?;
    reasons.extend(&q.reasons);
    Ok(Valued {
        node: q.node,
        value: q.value,
    })
}

/// A field's own reasons (P6-10b, review): each effect `marker` names as a
/// field-scoped reason marker (its stat) is an `unmapped-stat` reason, and
/// each unsettled value on a quantity `reads` accepts
/// ([`crate::store::ModStore::elsewhere`]) is an indeterminate one. The
/// regeneration, movement, and cost fields read their markers through it.
pub(super) fn marker_reasons(
    calc: &Calc<'_>,
    marker: impl Fn(Quantity) -> Option<&'static str>,
    reads: impl Fn(Quantity) -> bool,
) -> Reasons {
    let mut reasons = Reasons::new();
    for e in calc.store().effects() {
        if let Some(stat) = marker(e.quantity) {
            reasons
                .partial
                .insert(crate::state::UnsupportedReason::UnmappedStat {
                    stat: loom_domain::StatId::new(stat),
                    source: e.source.clone(),
                });
        }
    }
    for (quantities, reason) in calc.store().elsewhere() {
        if quantities.iter().any(|q| reads(*q)) {
            reasons.indeterminate.insert(reason.clone());
        }
    }
    reasons
}

/// Whether the store holds any effect on `quantity`.
/// Whether the attributes grant no inherent bonus (P6-11hx; PoB's
/// `NoAttributeBonuses`, `CalcPerform.lua` 494): the keystone's
/// unconditional flag is present.
pub(super) fn no_attribute_bonuses(calc: &Calc<'_>) -> bool {
    has(calc, Quantity::Poe1(Poe1Quantity::NoAttributeBonuses))
}

pub(super) fn has(calc: &Calc<'_>, quantity: Quantity) -> bool {
    calc.store()
        .effects()
        .iter()
        .any(|e| e.quantity == quantity)
}

/// `a op b` as a two-input node.
pub(super) fn pair(
    calc: &mut Calc<'_>,
    a: Valued,
    b: Valued,
    term: Term,
    min: bool,
) -> Result<Valued, CalcError> {
    let (node, value) = if min {
        (
            Node::Min {
                term,
                a: a.node,
                b: b.node,
            },
            math::min(a.value, b.value),
        )
    } else {
        (
            Node::Max {
                term,
                a: a.node,
                b: b.node,
            },
            math::max(a.value, b.value),
        )
    };
    let node = calc.graph_mut().push(node, value)?;
    Ok(Valued { node, value })
}

/// `trunc(x)` (PoB's `m_modf`) with `term`.
fn trunc(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    let value = math::trunc(x.value);
    let node = calc
        .graph_mut()
        .push(Node::Trunc { term, of: x.node }, value)?;
    Ok(Valued { node, value })
}

/// The product of `factors` from 1, left to right, with `term`.
pub(super) fn product(
    calc: &mut Calc<'_>,
    factors: &[Valued],
    term: Term,
) -> Result<Valued, CalcError> {
    let mut value = 1.0;
    for f in factors {
        value *= f.value;
    }
    let node = calc.graph_mut().push(
        Node::Product {
            term,
            factors: factors.iter().map(|f| f.node).collect(),
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// The sum of `terms` from 0, in order, with `term`.
pub(super) fn total(
    calc: &mut Calc<'_>,
    terms: &[Valued],
    term: Term,
) -> Result<Valued, CalcError> {
    let mut value = 0.0;
    for t in terms {
        value += t.value;
    }
    let node = calc.graph_mut().push(
        Node::Sum {
            term,
            terms: terms.iter().map(|t| t.node).collect(),
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// `1 + x / 100` with `term`.
pub(super) fn factor(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    let value = math::percent_factor(x.value);
    let node = calc
        .graph_mut()
        .push(Node::PercentFactor { term, of: x.node }, value)?;
    Ok(Valued { node, value })
}

/// Resistances (`Modules/CalcDefence.lua`, `calcs.resistances`), for
/// fire, cold, lightning, and chaos: `max = trunc(min(base max, 90))`,
/// `total = trunc(base x max(calcLib.mod, 0))`, `min = -200`, the value
/// `max(min(total, max), min)`. The base includes the resistance penalty
/// (`collect`); elemental stats are mapped to each element. The
/// breakdown's terms are `step.min`, `step.max`, `step.total`.
fn resistances(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for t in RESISTANCES {
        let (value, reasons) = resistance(calc, t)?;
        calc.set(Field::Resistance(t), query(value, reasons));
    }
    Ok(())
}

/// The resistances, in PoB's order.
pub(super) const RESISTANCES: [DamageType; 4] = [
    DamageType::Fire,
    DamageType::Cold,
    DamageType::Lightning,
    DamageType::Chaos,
];

/// One resistance of the current actor ([`resistances`]; P6-7b3a: the
/// minion's too, `calcs.resistances(actor)`).
pub(super) fn resistance(
    calc: &mut Calc<'_>,
    t: DamageType,
) -> Result<(Valued, Reasons), CalcError> {
    resistance_in(calc, t, Flags::NONE)
}

/// [`resistance`] with its base summed in `base_flags`: PoB's
/// `<Type>ResistOverTime` sums the base with `ModFlag.Dot` (`dotBase`,
/// `CalcDefence.lua` 589 and 590; P6-10b part 10), the rest as the
/// resistance's.
pub(super) fn resistance_in(
    calc: &mut Calc<'_>,
    t: DamageType,
    base_flags: Flags,
) -> Result<(Valued, Reasons), CalcError> {
    resistance_parts(calc, t, base_flags).map(|(value, _, reasons)| (value, reasons))
}

/// [`resistance_in`]'s value, its uncapped total (PoB's
/// `<Type>ResistTotal`, truncated; P6-11t), and its reasons.
pub(super) fn resistance_parts(
    calc: &mut Calc<'_>,
    t: DamageType,
    base_flags: Flags,
) -> Result<(Valued, Valued, Reasons), CalcError> {
    let mut reasons = Reasons::new();
    let quantity = Quantity::Resistance(t);
    let base = {
        let cx = calc.context(base_flags);
        let (store, graph) = calc.parts();
        let q = store.sum(graph, quantity, Op::Base, &cx, Term::Base)?;
        reasons.extend(&q.reasons);
        Valued {
            node: q.node,
            value: q.value,
        }
    };
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let inc = store.increased(graph, quantity, &cx)?;
    let more = store.more_rounded(graph, quantity, &cx)?;
    reasons.extend(&inc.reasons);
    reasons.extend(&more.reasons);
    let modifier = product(
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
        Term::Step("modifier"),
    )?;
    let zero = constant(calc, "no-negative-modifier", 0.0)?;
    let modifier = pair(
        calc,
        modifier,
        zero,
        Term::Step("modifier-at-least-zero"),
        false,
    )?;
    let raw = product(calc, &[base, modifier], Term::Step("unrounded"))?;
    let total = trunc(calc, raw, Term::Step("total"))?;

    let max = if is_elemental(t)
        && has(
            calc,
            Quantity::Poe1(Poe1Quantity::ElementalResistMaxIsHighest),
        ) {
        highest_maximum_resistance(calc, &mut reasons)?
    } else {
        let max_base = sum(
            calc,
            Quantity::MaximumResistance(t),
            Op::Base,
            Term::Step("maximum-sum"),
            &mut reasons,
        )?;
        let cap = constant(calc, "maximum-resistance-cap", MAX_RESIST_CAP)?;
        pair(calc, max_base, cap, Term::Step("maximum-capped"), true)?
    };
    let max = trunc(calc, max, Term::Step("max"))?;
    let floor = constant(calc, "resistance-floor", RESIST_FLOOR)?;
    let min = trunc(calc, floor, Term::Step("min"))?;

    let capped = pair(calc, total, max, Term::Step("capped"), true)?;
    let value = pair(calc, capped, min, Term::Result, false)?;
    Ok((value, total, reasons))
}

/// Whether `t` is an elemental damage type (PoB's `isElemental`).
const fn is_elemental(t: DamageType) -> bool {
    matches!(
        t,
        DamageType::Fire | DamageType::Cold | DamageType::Lightning
    )
}

/// The highest maximum elemental resistance (P6-11dh; `CalcDefence.lua`
/// 561 to 576, Melding of the Flesh): each elemental maximum's
/// `min(cap, sum)`, the highest of them and 0 (PoB starts from 0), which
/// overrides each elemental maximum.
fn highest_maximum_resistance(
    calc: &mut Calc<'_>,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cap = constant(calc, "maximum-resistance-cap", MAX_RESIST_CAP)?;
    let mut highest = constant(calc, "no-highest-maximum", 0.0)?;
    for t in RESISTANCES.into_iter().filter(|t| is_elemental(*t)) {
        let max_base = sum(
            calc,
            Quantity::MaximumResistance(t),
            Op::Base,
            Term::Step("maximum-sum"),
            reasons,
        )?;
        let capped = pair(calc, max_base, cap, Term::Step("maximum-capped"), true)?;
        highest = pair(calc, highest, capped, Term::Step("highest-maximum"), false)?;
    }
    Ok(highest)
}

/// PoB's `data.misc.MaxResistCap`: no maximum resistance above 90%.
const MAX_RESIST_CAP: f64 = 90.0;
/// PoB's `data.misc.ResistFloor`.
const RESIST_FLOOR: f64 = -200.0;
/// PoB's `data.misc.BlockChanceCap`.
const BLOCK_CHANCE_CAP: f64 = 90.0;
/// PoB's `data.misc.SuppressionChanceCap`.
const SUPPRESSION_CHANCE_CAP: f64 = 100.0;

/// Block, spell block, and spell suppression (`Modules/CalcDefence.lua`,
/// `calcs.defence`):
/// - attack block: `min(round((shield + base) x (1 + inc / 100) x more),
///   min(max, 90))`, the shield's own `floor((base block + local flat) x
///   (1 + local inc / 100))` (`Classes/Item.lua`);
/// - spell block: `min(round(base x (1 + inc / 100) x more), min(max,
///   90))`, with attack block's increases and more (the game's "Chance to
///   Block Attack and Spell Damage");
/// - spell suppression: `min(base, 100)`.
///
/// Keystones and flags that replace these (Necromantic Aegis, "cannot
/// block", block chance equal to another) come from stats the mapping
/// does not know, which make the fields `Partial`.
fn block(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let mut reasons = Reasons::new();
    let shield = shield_block(calc, &mut reasons)?;
    let AttackBlock {
        value,
        reasons,
        inc,
        more,
        modifier_reasons,
        over_cap,
    } = attack_block(calc, shield, reasons)?;
    calc.set(Field::BlockChance, query(value, reasons));
    spell_block(calc, value, over_cap, inc, more, &modifier_reasons)
}

/// PoB's `PerStat` tag with `div = 5` (P6-11f): `floor(stat / 5 +
/// 0.0001) x sum(BlockPer5(spell))`, as an extra base term, or `None` when
/// no such effect applies.
fn per_5(
    calc: &mut Calc<'_>,
    stat: Valued,
    spell: bool,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::BlockPer5(spell));
    if !has(calc, q) {
        return Ok(None);
    }
    let per = sum(calc, q, Op::Base, Term::Step("per-5-effects"), reasons)?;
    let five = constant(calc, "per-5", 5.0)?;
    // The block chances are whole numbers (rounded, floored), so PoB's
    // `+ 0.0001` never changes the floor.
    let value = crate::math::floor(stat.value / 5.0);
    let node = calc.graph_mut().push(
        Node::Quotient {
            term: Term::Step("per-5-ratio"),
            a: stat.node,
            b: five.node,
        },
        stat.value / 5.0,
    )?;
    let count = calc.graph_mut().push(
        Node::Floor {
            term: Term::Step("per-5-count"),
            of: node,
        },
        value,
    )?;
    Ok(Some(product(
        calc,
        &[Valued { node: count, value }, per],
        Term::Step("per-5"),
    )?))
}

/// Attack block's value and its shared modifier ([`attack_block`]).
pub(super) struct AttackBlock {
    /// The chance to block attack damage.
    pub(super) value: Valued,
    /// Its reasons.
    pub(super) reasons: Reasons,
    /// The increase factor, which spell block shares.
    inc: Valued,
    /// The more factor, which spell block shares.
    more: Valued,
    /// The modifier's reasons.
    modifier_reasons: Reasons,
    /// PoB's `BlockChanceOverCap`, `max(0, total - maximum)` (P6-11dq),
    /// built only when a spell block line reads it.
    over_cap: Option<Valued>,
}

/// The current actor's chance to block attack damage ([`block`]; P6-7b3a:
/// the minion's too, `calcs.defence(env, actor)`), with the shield's own
/// chance as an extra base term when there is one.
pub(super) fn attack_block(
    calc: &mut Calc<'_>,
    shield: Option<Valued>,
    mut reasons: Reasons,
) -> Result<AttackBlock, CalcError> {
    let cx = calc.context(Flags::NONE);
    let mut base_terms = Vec::new();
    // "Gain #% Chance to Block from Equipped Shield instead of the Shield's
    // value" (close-out M1; `CalcDefence.lua` 678): the override replaces
    // the shield's chance in the base; the "per 5%" lines still read the
    // shield's own (`output.ShieldBlockChance`).
    let replace = Quantity::Poe1(Poe1Quantity::ReplaceShieldBlock);
    let replaced = if shield.is_some() && has(calc, replace) {
        let (store, graph) = calc.parts();
        store.override_value(graph, replace, &cx)?.map(|q| {
            reasons.extend(&q.reasons);
            Valued {
                node: q.node,
                value: q.value,
            }
        })
    } else {
        None
    };
    base_terms.extend(replaced.or(shield));
    // "per 5% Chance to Block on Equipped Shield" (P6-11f).
    if let Some(shield) = shield
        && let Some(term) = per_5(calc, shield, false, &mut reasons)?
    {
        base_terms.push(term);
    }
    base_terms.push(sum(
        calc,
        Quantity::BlockChance,
        Op::Base,
        Term::Step("block-effects"),
        &mut reasons,
    )?);
    let base = total(calc, &base_terms, Term::Base)?;
    // Attack and spell block share these nodes (and their reasons).
    let (store, graph) = calc.parts();
    let inc = store.increased(graph, Quantity::BlockChance, &cx)?;
    let more = store.more_rounded(graph, Quantity::BlockChance, &cx)?;
    let mut modifier_reasons = inc.reasons.clone();
    modifier_reasons.extend(&more.reasons);
    reasons.extend(&modifier_reasons);
    let inc = Valued {
        node: inc.node,
        value: inc.value,
    };
    let more = Valued {
        node: more.node,
        value: more.value,
    };
    let raw = product(calc, &[base, inc, more], Term::Step("unrounded"))?;
    let rounded = round(calc, raw, Term::Step("total"))?;
    let max_base = sum(
        calc,
        Quantity::MaximumBlockChance,
        Op::Base,
        Term::Step("maximum-sum"),
        &mut reasons,
    )?;
    let cap = constant(calc, "block-chance-cap", BLOCK_CHANCE_CAP)?;
    let max = pair(calc, max_base, cap, Term::Step("max"), true)?;
    let value = pair(calc, rounded, max, Term::Result, true)?;
    // PoB's `BlockChanceOverCap` (`CalcDefence.lua` 694), for "+#% Chance to
    // Block Spell Damage for each 1% Overcapped Chance to Block Attack
    // Damage" (P6-11dq).
    let over_cap = if has(
        calc,
        Quantity::Poe1(Poe1Quantity::SpellBlockPerOvercappedBlock),
    ) {
        let over = binary(calc, Binary::Difference, rounded, max, "over-cap-raw")?;
        let zero = constant(calc, "no-over-cap", 0.0)?;
        Some(pair(calc, over, zero, Term::Step("over-cap"), false)?)
    } else {
        None
    };
    Ok(AttackBlock {
        value,
        reasons,
        inc,
        more,
        modifier_reasons,
        over_cap,
    })
}

/// Spell block and spell suppression ([`block`]): spell block shares
/// attack block's increases and more.
fn spell_block(
    calc: &mut Calc<'_>,
    attack_block: Valued,
    over_cap: Option<Valued>,
    inc: Valued,
    more: Valued,
    modifier_reasons: &Reasons,
) -> Result<(), CalcError> {
    let mut reasons = Reasons::new();
    let mut base = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::SpellBlockChance),
        Op::Base,
        Term::Base,
        &mut reasons,
    )?;
    // "per 5% Chance to Block Attack Damage" (P6-11f): PoB's `PerStat` of
    // the final attack block.
    if let Some(term) = per_5(calc, attack_block, true, &mut reasons)? {
        base = total(calc, &[base, term], Term::Base)?;
    }
    // Versatile Combatant (P6-11dq): `SpellBlockChance` BASE per 1% of
    // attack block over its maximum (`PerStat`, `div = 1`; the block
    // chances are whole numbers, so PoB's floor changes nothing).
    if let Some(over_cap) = over_cap {
        let per = sum(
            calc,
            Quantity::Poe1(Poe1Quantity::SpellBlockPerOvercappedBlock),
            Op::Base,
            Term::Step("per-overcapped-effects"),
            &mut reasons,
        )?;
        let term = product(calc, &[over_cap, per], Term::Step("per-overcapped"))?;
        base = total(calc, &[base, term], Term::Base)?;
    }
    reasons.extend(modifier_reasons);
    let raw = product(calc, &[base, inc, more], Term::Step("unrounded"))?;
    let rounded = round(calc, raw, Term::Step("total"))?;
    let max_base = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::MaximumSpellBlockChance),
        Op::Base,
        Term::Step("maximum-sum"),
        &mut reasons,
    )?;
    let cap = constant(calc, "block-chance-cap", BLOCK_CHANCE_CAP)?;
    let max = pair(calc, max_base, cap, Term::Step("max"), true)?;
    let value = pair(calc, rounded, max, Term::Result, true)?;
    calc.set(
        Field::Poe1(Poe1Field::SpellBlockChance),
        query(value, reasons),
    );

    let mut reasons = Reasons::new();
    let base = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::SpellSuppressionChance),
        Op::Base,
        Term::Base,
        &mut reasons,
    )?;
    let cap = constant(calc, "suppression-chance-cap", SUPPRESSION_CHANCE_CAP)?;
    let value = pair(calc, base, cap, Term::Result, true)?;
    // Acrobatics (P6-11el): the chance is overridden to 0 (PoB's
    // `SpellSuppressionChance` OVERRIDE 0 from the keystone).
    let value = match flag_of(calc, Poe1Quantity::SuppressionToSpellDodge, &mut reasons)? {
        Some(flag) => {
            let zero = constant(calc, "acrobatics-suppression", 0.0)?;
            product(calc, &[flag, zero, value], Term::Result)?
        }
        None => value,
    };
    calc.set(
        Field::Poe1(Poe1Field::SpellSuppressionChance),
        query(value, reasons),
    );
    Ok(())
}

/// A defence's global modifier, `calcLib.mod`: `(1 + inc / 100) x more`,
/// with the attribute bonus as an extra increase. Every row of the
/// defence multiplies by it.
fn defence_modifier(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    extra_inc: Option<Valued>,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    defence_modifier_with(calc, quantity, extra_inc, None, reasons)
}

/// [`defence_modifier`] with one more factor in its more product (P6-11gq:
/// the body armour's doubled armour, PoB's `Armour` MORE with the
/// `SlotName`, which its slot row shows as the row's more).
fn defence_modifier_with(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    extra_inc: Option<Valued>,
    extra_more: Option<Valued>,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let effects = sum(
        calc,
        quantity,
        Op::Increased,
        Term::Step("increased-effects"),
        reasons,
    )?;
    let mut terms = vec![effects];
    terms.extend(extra_inc);
    let inc = total(calc, &terms, Term::Step("increased-sum"))?;
    let inc = factor(calc, inc, Term::Increased)?;
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let more = store.more_rounded(graph, quantity, &cx)?;
    reasons.extend(&more.reasons);
    let more = Valued {
        node: more.node,
        value: more.value,
    };
    let more = match extra_more {
        Some(extra) => product(calc, &[more, extra], Term::More)?,
        None => more,
    };
    product(calc, &[inc, more], Term::Step("modifier"))
}

/// The shield's own chance to block, when the off hand holds one:
/// `floor(base x (1 + local inc / 100))` (`Classes/Item.lua`).
pub(super) fn shield_block(
    calc: &mut Calc<'_>,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let shield_q = Quantity::Poe1(Poe1Quantity::Piece(ArmourSlot::Weapon2, PieceValue::Block));
    if !has(calc, shield_q) {
        return Ok(None);
    }
    let b = sum(
        calc,
        shield_q,
        Op::Base,
        Term::Step("shield-block-base"),
        reasons,
    )?;
    let inc = sum(
        calc,
        shield_q,
        Op::Increased,
        Term::Step("shield-block-increased-sum"),
        reasons,
    )?;
    let inc = factor(calc, inc, Term::Step("shield-block-increased"))?;
    let raw = product(calc, &[b, inc], Term::Step("shield-block-unrounded"))?;
    let value = math::floor(raw.value);
    let node = calc.graph_mut().push(
        Node::Floor {
            term: Term::Step("shield-block"),
            of: raw.node,
        },
        value,
    )?;
    Ok(Some(Valued { node, value }))
}

/// The value of `defence` on the armour piece in `slot` (`Classes/
/// Item.lua`): `round((min + local flat + range x percentile) x (1 + local
/// inc / 100) x (1 + quality / 100))`. The percentile is the item's
/// `<Defence>BasePercentile`; else, when the item shows the defence and
/// the range is not 0, the one that gives the shown value back (PoB's
/// `Item.lua:2323-2337`: `round(clamp((shown / (inc x quality) - flat) /
/// range, 0, 1), 4)`; PoB's flat leaves out hybrid local flats there and
/// adds them after, ours keeps them, so a hybrid piece shows its own
/// value; `docs/ui/b2-data/item-evidence.md`); else 1. Its nodes are
/// `step.piece-*`, and the value's term is [`Term::Base`]: the row's base.
pub(super) fn piece(
    calc: &mut Calc<'_>,
    slot: ArmourSlot,
    defence: Defence,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let q = |v| Quantity::Poe1(Poe1Quantity::Piece(slot, v));
    let flat = sum(
        calc,
        q(PieceValue::Defence(defence)),
        Op::Base,
        Term::Step("piece-flat"),
        reasons,
    )?;
    let variance = sum(
        calc,
        q(PieceValue::Variance(defence)),
        Op::Base,
        Term::Step("piece-range"),
        reasons,
    )?;
    let inc = sum(
        calc,
        q(PieceValue::Defence(defence)),
        Op::Increased,
        Term::Step("piece-increased-sum"),
        reasons,
    )?;
    let inc = factor(calc, inc, Term::Step("piece-increased"))?;
    let quality = sum(
        calc,
        q(PieceValue::Quality),
        Op::Base,
        Term::Step("piece-quality-sum"),
        reasons,
    )?;
    let quality = factor(calc, quality, Term::Step("piece-quality"))?;
    let percentile = if has(calc, q(PieceValue::Percentile(defence))) {
        sum(
            calc,
            q(PieceValue::Percentile(defence)),
            Op::Base,
            Term::Step("piece-percentile"),
            reasons,
        )?
    } else if variance.value != 0.0 && has(calc, q(PieceValue::Shown(defence))) {
        let shown = sum(
            calc,
            q(PieceValue::Shown(defence)),
            Op::Base,
            Term::Step("piece-shown"),
            reasons,
        )?;
        // A shown value of 0 or less gives no percentile, as in PoB
        // (`Item.lua:2323`, `armourData.Armour > 0`), so the roll is the
        // top. A copy of a base with a defence range never shows 0.
        if shown.value > 0.0 {
            derived_percentile(calc, shown, [inc, quality], flat, variance)?
        } else {
            constant(calc, "default-percentile", 1.0)?
        }
    } else {
        constant(calc, "default-percentile", 1.0)?
    };
    let rolled = product(calc, &[variance, percentile], Term::Step("piece-rolled"))?;
    let base = total(calc, &[flat, rolled], Term::Step("piece-base"))?;
    let raw = product(calc, &[base, inc, quality], Term::Step("piece-unrounded"))?;
    round(calc, raw, Term::Base)
}

/// Ward (P6-11hn; `CalcDefence.lua` 829 to 948, 1052): each equipped armour
/// piece's ward above 0 times `calcLib.mod(modDB, slotCfg, "Ward",
/// "Defences")` (the global increases, the piece's own `SlotName` ones,
/// and the more multipliers), plus the global flat `Ward` BASE times the
/// same without a slot, `max(floor(sum), 0)`. Ghostwrithe's energy shield
/// increases applied to ward and the body armour's armour and evasion
/// converted to ward have no row: their stats are reasons. A plain value,
/// as the other pools [`super::enemy_hit`] reads.
pub(super) fn ward(calc: &mut Calc<'_>, reasons: &mut Reasons) -> Result<f64, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::Ward);
    let mut rows = Vec::new();
    for slot in ArmourSlot::ALL {
        if !has(
            calc,
            Quantity::Poe1(Poe1Quantity::Piece(slot, PieceValue::Equipped)),
        ) {
            continue;
        }
        let base = piece(calc, slot, Defence::Ward, reasons)?;
        if base.value > 0.0 {
            let own = Quantity::Poe1(Poe1Quantity::SlotIncreased(slot, Defence::Ward));
            let own = if has(calc, own) {
                sum(
                    calc,
                    own,
                    Op::Increased,
                    Term::Step("ward-slot-increased"),
                    reasons,
                )?
                .value
            } else {
                0.0
            };
            rows.push((base.value, own));
        }
    }
    let flat = sum(calc, q, Op::Base, Term::Step("ward-flat"), reasons)?.value;
    if rows.is_empty() && flat <= 0.0 {
        return Ok(0.0);
    }
    let inc = sum(
        calc,
        q,
        Op::Increased,
        Term::Step("ward-increased"),
        reasons,
    )?
    .value;
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let more = store.more_rounded(graph, q, &cx)?;
    reasons.extend(&more.reasons);
    let more = more.value;
    let mut ward: f64 = rows
        .iter()
        .map(|&(base, own)| base * (1.0 + (inc + own) / 100.0) * more)
        .sum();
    if flat > 0.0 {
        ward += flat * (1.0 + inc / 100.0) * more;
    }
    Ok(math::floor(ward).max(0.0))
}

/// `round(clamp((shown / (inc x quality) - flat) / range, 0, 1), 4)`
/// ([`piece`]).
fn derived_percentile(
    calc: &mut Calc<'_>,
    shown: Valued,
    [inc, quality]: [Valued; 2],
    flat: Valued,
    variance: Valued,
) -> Result<Valued, CalcError> {
    let scale = product(calc, &[inc, quality], Term::Step("piece-shown-scale"))?;
    let unscaled = binary(calc, Binary::Quotient, shown, scale, "piece-shown-base")?;
    let over = binary(
        calc,
        Binary::Difference,
        unscaled,
        flat,
        "piece-shown-rolled",
    )?;
    let raw = binary(
        calc,
        Binary::Quotient,
        over,
        variance,
        "piece-shown-fraction",
    )?;
    let one = constant(calc, "percentile-max", 1.0)?;
    let zero = constant(calc, "percentile-min", 0.0)?;
    let capped = pair(calc, raw, one, Term::Step("piece-shown-capped"), true)?;
    let clamped = pair(calc, capped, zero, Term::Step("piece-shown-clamped"), false)?;
    let places = constant(calc, "percentile-places", 10_000.0)?;
    let scaled = product(calc, &[clamped, places], Term::Step("piece-shown-scaled"))?;
    let rounded = round(calc, scaled, Term::Step("piece-shown-rounded"))?;
    binary(calc, Binary::Quotient, rounded, places, "piece-percentile")
}

/// A quotient or difference node.
#[derive(Clone, Copy)]
enum Binary {
    Quotient,
    Difference,
}

fn binary(
    calc: &mut Calc<'_>,
    kind: Binary,
    a: Valued,
    b: Valued,
    step: &'static str,
) -> Result<Valued, CalcError> {
    let term = Term::Step(step);
    let (node, value) = match kind {
        Binary::Quotient => (
            Node::Quotient {
                term,
                a: a.node,
                b: b.node,
            },
            a.value / b.value,
        ),
        Binary::Difference => (
            Node::Difference {
                term,
                a: a.node,
                b: b.node,
            },
            a.value - b.value,
        ),
    };
    let node = calc.graph_mut().push(node, value)?;
    Ok(Valued { node, value })
}

/// Energy shield, armour, and evasion (`Modules/CalcDefence.lua`, primary
/// defences): each the sum of its rows, `max(round(sum), 0)`. A row is
/// an armour piece's value (in the order Helmet, Gloves, Boots, Body
/// Armour, Weapon 2) or the global flat value, each when above 0, times
/// the defence's `calcLib.mod` (with `Defences`, mapped into each). The
/// attribute bonuses are increases on every row: intelligence `floor(Int
/// / 10)`% energy shield, dexterity `floor(Dex / 5)`% evasion
/// (`Modules/CalcPerform.lua`). Rows are `step.slot.<slot>` and
/// `step.slot.global` (static names, `ArmourSlot::row`).
fn primary_defences(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let int_bonus = bonus(
        calc,
        Field::Intelligence,
        0.1,
        "tenth",
        "energy-shield-from-intelligence",
    )?;
    // Energy shield increased by the spell block chance or the chaos
    // resistance (P6-11cb; `CalcDefence.lua` 812 to 824), each when its flag
    // is present: one more term of energy shield's increase.
    let mut es_terms: Vec<Valued> = int_bonus.into_iter().collect();
    let mut es_deps = vec![Field::Intelligence];
    for (flag, from) in [
        (
            Poe1Quantity::EnergyShieldFromSpellBlock,
            Field::Poe1(Poe1Field::SpellBlockChance),
        ),
        (
            Poe1Quantity::EnergyShieldFromChaosResistance,
            Field::Resistance(crate::quantity::DamageType::Chaos),
        ),
    ] {
        if !has(calc, Quantity::Poe1(flag)) {
            continue;
        }
        if let Some(v) = calc.field(from).and_then(|f| {
            Some(Valued {
                node: f.node?,
                value: f.value?,
            })
        }) {
            es_terms.push(v);
            es_deps.push(from);
        }
    }
    let int_bonus = match es_terms.len() {
        0 => None,
        1 => Some(es_terms[0]),
        _ => Some(total(
            calc,
            &es_terms,
            Term::Step("energy-shield-increases"),
        )?),
    };
    // Iron Reflexes' "Dexterity provides no bonus to Evasion Rating"
    // (`NoDexBonusToEvasion`, `CalcPerform.lua` 509; P6-11ek).
    let mut no_dex_reasons = Reasons::new();
    let dex_bonus =
        if flag_of(calc, Poe1Quantity::NoDexBonusToEvasion, &mut no_dex_reasons)?.is_some() {
            None
        } else {
            bonus(
                calc,
                Field::Dexterity,
                0.2,
                "fifth",
                "evasion-from-dexterity",
            )?
        };
    // Life gained as energy shield (P6-11dd): one more row of energy
    // shield, which reads life's base and so strength.
    let mut life_reasons = Reasons::new();
    let life_row = life_as_energy_shield(calc, int_bonus, true, &mut life_reasons)?;
    if life_row.is_some() {
        es_deps.push(Field::Strength);
    }
    for (defence, field, quantity, extra, dep) in [
        (
            Defence::EnergyShield,
            Field::EnergyShield,
            Quantity::EnergyShield,
            int_bonus,
            es_deps.as_slice(),
        ),
        (Defence::Armour, Field::Armour, Quantity::Armour, None, &[]),
        (
            Defence::Evasion,
            Field::Evasion,
            Quantity::Evasion,
            dex_bonus,
            &[Field::Dexterity],
        ),
    ] {
        let row = if matches!(defence, Defence::EnergyShield) {
            life_row
        } else {
            None
        };
        let (value, mut reasons) = primary_defence(calc, defence, quantity, extra, row)?;
        if row.is_some() {
            reasons.extend(&life_reasons);
        }
        if dep.is_empty() {
            calc.set(field, query(value, reasons));
        } else {
            calc.set_derived(field, query(value, reasons), dep);
        }
    }
    Ok(())
}

/// Life gained as extra energy shield (P6-11dd; `CalcDefence.lua` 1023
/// to 1036): `Life BASE x percent / 100`, times `calcLib.mod(modDB, nil,
/// "Life", "EnergyShield", "Defences")`: the increases of life and energy
/// shield summed (`es_inc`: energy shield's extra increases, the
/// intelligence bonus among them, as in PoB's store), times life's and
/// energy shield's rounded more multipliers (PoB rounds per mod name).
/// Life's base is its BASE effects and the strength bonus, as
/// `life_mana`'s. `None` without the stat. With Chaos Inoculation the row
/// is 1 (`CalcDefence.lua` 1027; P6-11ej). `strength`: whether life's base
/// has the strength bonus (the character's; a minion's life base is its
/// monster life alone, RV-12).
///
/// # Errors
/// A graph error.
pub(super) fn life_as_energy_shield(
    calc: &mut Calc<'_>,
    es_inc: Option<Valued>,
    strength: bool,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::LifeAsEnergyShield);
    if !has(calc, q) {
        return Ok(None);
    }
    let percent = sum(
        calc,
        q,
        Op::Base,
        Term::Step("life-as-energy-shield-percent"),
        reasons,
    )?;
    if percent.value <= 0.0 {
        return Ok(None);
    }
    if let Some(flag) = chaos_inoculation(calc, reasons)? {
        let one = constant(calc, "chaos-inoculation-life-as-energy-shield", 1.0)?;
        let row = product(
            calc,
            &[flag, one],
            Term::Step("slot.conversion.life-to-energy-shield"),
        )?;
        return Ok(Some(row));
    }
    let str_bonus = if strength {
        strength_life(calc, reasons)?
    } else {
        None
    };
    let life_effects = sum(
        calc,
        Quantity::Life,
        Op::Base,
        Term::Step("life-base-effects"),
        reasons,
    )?;
    let mut base_terms = vec![life_effects];
    base_terms.extend(str_bonus);
    let life_base = total(calc, &base_terms, Term::Step("life-base"))?;
    let scaled = product(
        calc,
        &[life_base, percent],
        Term::Step("life-as-energy-shield-scaled"),
    )?;
    let hundred = constant(calc, "hundred", 100.0)?;
    let base = binary(
        calc,
        Binary::Quotient,
        scaled,
        hundred,
        "life-as-energy-shield-base",
    )?;
    let base = total(calc, &[base], Term::Base)?;
    let mut incs = vec![
        sum(
            calc,
            Quantity::Life,
            Op::Increased,
            Term::Step("life-increased-effects"),
            reasons,
        )?,
        sum(
            calc,
            Quantity::EnergyShield,
            Op::Increased,
            Term::Step("increased-effects"),
            reasons,
        )?,
    ];
    incs.extend(es_inc);
    let inc = total(calc, &incs, Term::Step("increased-sum"))?;
    let inc = factor(calc, inc, Term::Increased)?;
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let life_more = store.more_rounded(graph, Quantity::Life, &cx)?;
    let es_more = store.more_rounded(graph, Quantity::EnergyShield, &cx)?;
    reasons.extend(&life_more.reasons);
    reasons.extend(&es_more.reasons);
    let row = product(
        calc,
        &[
            base,
            inc,
            Valued {
                node: life_more.node,
                value: life_more.value,
            },
            Valued {
                node: es_more.node,
                value: es_more.value,
            },
        ],
        Term::Step("slot.conversion.life-to-energy-shield"),
    )?;
    Ok(Some(row))
}

/// One primary defence of the current actor ([`primary_defences`];
/// P6-7b3a: the minion's too, whose store has no armour piece): each
/// piece's row and the global row, times the modifier, rounded. `life_row`
/// is energy shield's row from life ([`life_as_energy_shield`]), last as in
/// PoB.
pub(super) fn primary_defence(
    calc: &mut Calc<'_>,
    defence: Defence,
    quantity: Quantity,
    extra: Option<Valued>,
    life_row: Option<Valued>,
) -> Result<(Valued, Reasons), CalcError> {
    let mut reasons = Reasons::new();
    // Iron Reflexes (P6-11ek): evasion's rows go to armour.
    let iron_reflexes = if matches!(defence, Defence::Armour | Defence::Evasion) {
        flag_of(calc, Poe1Quantity::IronReflexes, &mut reasons)?
    } else {
        None
    };
    if let (Defence::Evasion, Some(flag)) = (defence, iron_reflexes) {
        let zero = constant(calc, "iron-reflexes-evasion", 0.0)?;
        let value = product(calc, &[flag, zero], Term::Result)?;
        return Ok((value, reasons));
    }
    let modifier = defence_modifier(calc, quantity, extra, &mut reasons)?;
    // "Armour from Equipped Body Armour is doubled" (P6-11gq): the body
    // armour's armour row times 2, once (PoB's `globalLimit` 100).
    let doubled = if defence == Defence::Armour {
        body_armour_doubled(calc, &mut reasons)?
    } else {
        None
    };
    let mut rows = Vec::new();
    for slot in ArmourSlot::ALL {
        if !has(
            calc,
            Quantity::Poe1(Poe1Quantity::Piece(slot, PieceValue::Equipped)),
        ) {
            continue;
        }
        let base = piece(calc, slot, defence, &mut reasons)?;
        if base.value > 0.0 {
            // A piece's own increase (P6-11k, PoB's `SlotName` tag): the
            // piece's modifier adds it to the global increase; other pieces
            // keep the shared modifier.
            let own = Quantity::Poe1(Poe1Quantity::SlotIncreased(slot, defence));
            // The body armour's increase when the helmet, gloves, and boots
            // each have armour and energy shield (P6-11by; PoB's
            // `StatThreshold` on `ArmourOn<slot>` and `EnergyShieldOn<slot>`,
            // each piece's own value, at least 1).
            let hybrid = Quantity::Poe1(Poe1Quantity::BodyArmourIfHybridPieces);
            let hybrid_applies = slot == ArmourSlot::BodyArmour
                && matches!(defence, Defence::Armour | Defence::EnergyShield)
                && has(calc, hybrid)
                && hybrid_pieces(calc, &mut reasons)?;
            let slot_more = doubled.filter(|_| slot == ArmourSlot::BodyArmour);
            let modifier = if has(calc, own) || hybrid_applies || slot_more.is_some() {
                let mut incs = Vec::new();
                if has(calc, own) {
                    incs.push(sum(
                        calc,
                        own,
                        Op::Increased,
                        Term::Step("slot-increased"),
                        &mut reasons,
                    )?);
                }
                if hybrid_applies {
                    incs.push(sum(
                        calc,
                        hybrid,
                        Op::Increased,
                        Term::Step("slot-increased-hybrid"),
                        &mut reasons,
                    )?);
                }
                let extra_inc = if incs.is_empty() {
                    extra
                } else {
                    let slot_inc = total(calc, &incs, Term::Step("slot-increases"))?;
                    Some(match extra {
                        Some(e) => total(calc, &[e, slot_inc], Term::Step("extra-and-slot"))?,
                        None => slot_inc,
                    })
                };
                defence_modifier_with(calc, quantity, extra_inc, slot_more, &mut reasons)?
            } else {
                modifier
            };
            rows.push(product(calc, &[base, modifier], Term::Step(slot.row()))?);
        }
    }
    let flat = sum(
        calc,
        quantity,
        Op::Base,
        Term::Step("base-effects"),
        &mut reasons,
    )?;
    let base = total(calc, &[flat], Term::Base)?;
    if base.value > 0.0 {
        rows.push(product(calc, &[base, modifier], Term::Step("slot.global"))?);
    }
    if let Some(flag) = iron_reflexes {
        iron_reflexes_rows(calc, flag, doubled, &mut rows, &mut reasons)?;
    }
    rows.extend(life_row);
    let sum = total(calc, &rows, Term::Step("slots"))?;
    let rounded = round(calc, sum, Term::Step("rounded"))?;
    let value = at_least(calc, rounded, "minimum", 0.0)?;
    Ok((value, reasons))
}

/// "Armour from Equipped Body Armour is doubled" when on (P6-11gq; PoB's
/// `Armour` MORE 100 with the `SlotName` Body Armour, at most once): the
/// flag times 2.
fn body_armour_doubled(
    calc: &mut Calc<'_>,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let Some(flag) = flag_of(calc, Poe1Quantity::BodyArmourArmourDoubled, reasons)? else {
        return Ok(None);
    };
    let two = constant(calc, "body-armour-armour-doubled", 2.0)?;
    Ok(Some(product(
        calc,
        &[flag, two],
        Term::Step("body-armour-armour-doubled"),
    )?))
}

/// Iron Reflexes' armour rows (P6-11ek; `CalcDefence.lua` 919 to 923 and
/// 980 to 984): each piece's evasion, then the global evasion base, times
/// [`iron_reflexes_modifier`], after armour's own rows as in PoB.
fn iron_reflexes_rows(
    calc: &mut Calc<'_>,
    flag: Valued,
    doubled: Option<Valued>,
    rows: &mut Vec<Valued>,
    reasons: &mut Reasons,
) -> Result<(), CalcError> {
    let shared = iron_reflexes_modifier(calc, None, reasons)?;
    for slot in ArmourSlot::ALL {
        if !has(
            calc,
            Quantity::Poe1(Poe1Quantity::Piece(slot, PieceValue::Equipped)),
        ) {
            continue;
        }
        let base = piece(calc, slot, Defence::Evasion, reasons)?;
        if base.value > 0.0 {
            let own = [Defence::Armour, Defence::Evasion]
                .iter()
                .any(|d| has(calc, Quantity::Poe1(Poe1Quantity::SlotIncreased(slot, *d))));
            let modifier = if own {
                iron_reflexes_modifier(calc, Some(slot), reasons)?
            } else {
                shared
            };
            // The body armour's evasion counts as its armour, so its
            // `SlotName` doubling applies (P6-11gq).
            let mut factors = vec![base, modifier, flag];
            if slot == ArmourSlot::BodyArmour {
                factors.extend(doubled);
            }
            rows.push(product(
                calc,
                &factors,
                Term::Step(slot.evasion_to_armour_row()),
            )?);
        }
    }
    let flat = sum(
        calc,
        Quantity::Evasion,
        Op::Base,
        Term::Step("evasion-base-effects"),
        reasons,
    )?;
    if flat.value > 0.0 {
        // The row's base as PoB's breakdown shows it (P6-11gq).
        let base = total(calc, &[flat], Term::Base)?;
        rows.push(product(
            calc,
            &[base, shared, flag],
            Term::Step("slot.conversion.evasion-to-armour"),
        )?);
    }
    Ok(())
}

/// Iron Reflexes' modifier (P6-11ek): PoB's `calcLib.mod(modDB, slotCfg,
/// "Armour", "Evasion", "ArmourAndEvasion", "Defences")`. The engine maps
/// each shared increase to both `Armour` and `Evasion`, and to
/// [`Poe1Quantity::ArmourAndEvasionShared`], so the increase is armour's
/// plus evasion's less the shared, each counted once; with `slot`, that
/// piece's own increases of both. The more product is armour's times
/// evasion's (no row has a more of both; the mapping's tests check it).
fn iron_reflexes_modifier(
    calc: &mut Calc<'_>,
    slot: Option<ArmourSlot>,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let mut terms = vec![
        sum(
            calc,
            Quantity::Armour,
            Op::Increased,
            Term::Step("armour-increased-effects"),
            reasons,
        )?,
        sum(
            calc,
            Quantity::Evasion,
            Op::Increased,
            Term::Step("evasion-increased-effects"),
            reasons,
        )?,
    ];
    if let Some(slot) = slot {
        for d in [Defence::Armour, Defence::Evasion] {
            let own = Quantity::Poe1(Poe1Quantity::SlotIncreased(slot, d));
            if has(calc, own) {
                terms.push(sum(
                    calc,
                    own,
                    Op::Increased,
                    Term::Step("slot-increased"),
                    reasons,
                )?);
            }
        }
    }
    let both = total(calc, &terms, Term::Step("increased-both"))?;
    let shared = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::ArmourAndEvasionShared),
        Op::Increased,
        Term::Step("shared-increased-effects"),
        reasons,
    )?;
    let inc = binary(calc, Binary::Difference, both, shared, "increased-sum")?;
    let inc = factor(calc, inc, Term::Increased)?;
    let cx = calc.context(Flags::NONE);
    let (store, graph) = calc.parts();
    let armour_more = store.more_rounded(graph, Quantity::Armour, &cx)?;
    let evasion_more = store.more_rounded(graph, Quantity::Evasion, &cx)?;
    reasons.extend(&armour_more.reasons);
    reasons.extend(&evasion_more.reasons);
    product(
        calc,
        &[
            inc,
            Valued {
                node: armour_more.node,
                value: armour_more.value,
            },
            Valued {
                node: evasion_more.node,
                value: evasion_more.value,
            },
        ],
        Term::Step("modifier"),
    )
}

/// Whether the helmet, gloves, and boots are each equipped with armour and
/// energy shield above 0 ([`Poe1Quantity::BodyArmourIfHybridPieces`]).
fn hybrid_pieces(calc: &mut Calc<'_>, reasons: &mut Reasons) -> Result<bool, CalcError> {
    for slot in [ArmourSlot::Helmet, ArmourSlot::Gloves, ArmourSlot::Boots] {
        if !has(
            calc,
            Quantity::Poe1(Poe1Quantity::Piece(slot, PieceValue::Equipped)),
        ) {
            return Ok(false);
        }
        for defence in [Defence::Armour, Defence::EnergyShield] {
            if piece(calc, slot, defence, reasons)?.value < 1.0 {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Unreserved life and mana: the pool less its reservation
/// (`doActorLifeManaReservation`, `CalcPerform.lua` 526 to 560: `reserved
/// = flat + ceil(max x percent / 100)`, `Unreserved = max - reserved`).
/// The skills' per-skill amounts are `Reservation` effects
/// ([`super::gems`], P3-12h); a pool no skill reserves keeps its value, as
/// a one-term sum, so graphs without a reservation are unchanged.
fn unreserved(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for (field, pool, which) in [
        (Field::LifeUnreserved, Field::Life, Pool::Life),
        (Field::ManaUnreserved, Field::Mana, Pool::Mana),
    ] {
        let Some((node, value, kind)) = calc
            .field(pool)
            .and_then(|f| Some((f.node?, f.value?, f.state.kind())))
        else {
            continue;
        };
        // PoB writes `<pool>Unreserved` only when the pool is above 0
        // (`Modules/CalcPerform.lua`, reservation: `if max > 0`); for an
        // empty pool the field stays not computed, as absent as PoB's.
        // An indeterminate pool's value is not settled, so its unreserved
        // field is derived (and indeterminate) as usual.
        if value <= 0.0 && kind != StateKind::Indeterminate {
            continue;
        }
        let flat_q = Quantity::Poe1(Poe1Quantity::Reservation(which, ReservationKind::Flat));
        let percent_q = Quantity::Poe1(Poe1Quantity::Reservation(which, ReservationKind::Percent));
        if !has(calc, flat_q) && !has(calc, percent_q) {
            // Nothing reserved: the pool itself, as a one-term sum so the
            // field has its own result node.
            let node = calc.graph_mut().push(
                Node::Sum {
                    term: Term::Result,
                    terms: vec![node],
                },
                value,
            )?;
            calc.set_derived(
                field,
                Query {
                    value,
                    node,
                    reasons: Reasons::new(),
                },
                &[pool],
            );
            continue;
        }
        let mut reasons = Reasons::new();
        let max = Valued { node, value };
        let flat = sum(
            calc,
            flat_q,
            Op::Base,
            Term::Step("reserved-flat"),
            &mut reasons,
        )?;
        let percent = sum(
            calc,
            percent_q,
            Op::Base,
            Term::Step("reserved-percent"),
            &mut reasons,
        )?;
        // `m_ceil(max * percent / 100)`, in PoB's order.
        let share = product(calc, &[max, percent], Term::Step("pool-times-percent"))?;
        let hundred = constant(calc, "hundred", 100.0)?;
        let share_value = share.value / hundred.value;
        let share = Valued {
            node: calc.graph_mut().push(
                Node::Quotient {
                    term: Term::Step("pool-share"),
                    a: share.node,
                    b: hundred.node,
                },
                share_value,
            )?,
            value: share_value,
        };
        let ceiled = math::ceil(share.value);
        let ceiled = Valued {
            node: calc.graph_mut().push(
                Node::Ceil {
                    term: Term::Step("reserved-from-percent"),
                    of: share.node,
                },
                ceiled,
            )?,
            value: ceiled,
        };
        let reserved = total(calc, &[flat, ceiled], Term::Step("reserved"))?;
        let unreserved = max.value - reserved.value;
        let node = calc.graph_mut().push(
            Node::Difference {
                term: Term::Result,
                a: max.node,
                b: reserved.node,
            },
            unreserved,
        )?;
        calc.set_derived(
            field,
            Query {
                value: unreserved,
                node,
                reasons,
            },
            &[pool],
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_tenth_should_floor_as_an_integer_division() {
        for n in 0_u32..=100_000 {
            let product = f64::from(n) * 0.1;
            assert_eq!(product.floor(), f64::from(n / 10), "{n}");
            assert_eq!((f64::from(n) * 0.5).floor(), f64::from(n / 2), "{n}");
            assert_eq!((f64::from(n) * 0.2).floor(), f64::from(n / 5), "{n}");
        }
    }
}

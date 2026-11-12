//! The Path of Exile phases of P3-6 (Phase 3 plan, step 6): attributes,
//! life, mana, energy shield, and unreserved life and mana.
//!
//! Each formula follows the game as PoB v2.67.2 computes it, cited per
//! phase, with the same order of operations (floating-point products
//! depend on it) and the same rounding points. Every value is a graph
//! node, built to `docs/phase-3/slice-fields.json` revision 2's `graph`
//! references: a field's node has [`Term::Result`]; below it the nearest
//! [`Term::Base`], [`Term::Increased`], and [`Term::More`] nodes are the
//! breakdown's terms; an energy shield row is `step.slot.<source>`.
//!
//! Attribute bonuses read the attribute's field node, so a derived field
//! carries its attribute's reasons (`Calc::set_derived`).

use crate::condition::QueryContext;
use crate::engine::{Calc, Phase};
use crate::error::CalcError;
use crate::graph::{Node, NodeIndex, Term};
use crate::math;
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::state::Reasons;
use crate::store::Query;

/// The phases, in order.
pub const PHASES: [Phase; 4] = [
    Phase {
        name: "attributes",
        run: attributes,
    },
    Phase {
        name: "life-mana",
        run: life_mana,
    },
    Phase {
        name: "energy-shield",
        run: energy_shield,
    },
    Phase {
        name: "unreserved",
        run: unreserved,
    },
];

/// A node with its value.
#[derive(Debug, Clone, Copy)]
struct Valued {
    node: NodeIndex,
    value: f64,
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

/// The parts of a chain: `base`, the increase factor, the more factor,
/// and their product, with the reasons the store's queries gave.
struct ChainParts {
    base: Valued,
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
    let cx = QueryContext::default();
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

    let more = store.more(graph, quantity, &cx)?;
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
        base: Valued {
            node: base_node,
            value: base,
        },
        product: Valued {
            node: product,
            value,
        },
        reasons,
    })
}

/// `round(x)` (PoB's `m_floor(x + 0.5)`) with `term`.
fn round(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
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
/// for "per 2" (exact) and `0.1` for "per 10": `0.1` as a double is a
/// little above one tenth, so for a whole number `n` the product is at
/// least `n / 10` and below the next whole number; the floor equals
/// PoB's `m_floor(n / 10)` (a test checks every `n` up to 100,000).
fn bonus(
    calc: &mut Calc<'_>,
    attribute: Field,
    factor: f64,
    label: &'static str,
    term: &'static str,
) -> Result<Option<Valued>, CalcError> {
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

fn query(v: Valued, reasons: Reasons) -> Query {
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
/// Chaos Inoculation, life and mana conversion, and overrides come from
/// stats the mapping does not know yet, which make the fields `Partial`.
fn life_mana(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let str_bonus = bonus(calc, Field::Strength, 0.5, "half", "life-from-strength")?;
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
    calc.set_derived(Field::Life, query(life, parts.reasons), &[Field::Strength]);

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

/// Energy shield: the sum of its rows, `max(round(sum), 0)`
/// (`Modules/CalcDefence.lua`). P3-6 has the `Global` row: flat global
/// energy shield times `calcLib.mod(modDB, nil, "EnergyShield",
/// "Defences")`, present only when the base is above 0; intelligence
/// adds `floor(Int / 10)` percent increased (`EnergyShield` INC from
/// "Intelligence"). Armour pieces' rows need their base properties
/// (P3-7); an item in such a slot makes the field `Partial` until then.
fn energy_shield(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let int_bonus = bonus(
        calc,
        Field::Intelligence,
        0.1,
        "tenth",
        "energy-shield-from-intelligence",
    )?;
    let parts = chain(
        calc,
        Quantity::EnergyShield,
        &[],
        int_bonus.as_slice(),
        Order::Right,
        Term::Step("slot.global"),
    )?;
    let mut rows = Vec::new();
    let mut total = 0.0;
    if parts.base.value > 0.0 {
        total += parts.product.value;
        rows.push(parts.product.node);
    }
    let sum = calc.graph_mut().push(
        Node::Sum {
            term: Term::Step("slots"),
            terms: rows,
        },
        total,
    )?;
    let rounded = round(
        calc,
        Valued {
            node: sum,
            value: total,
        },
        Term::Step("rounded"),
    )?;
    let es = at_least(calc, rounded, "minimum", 0.0)?;
    calc.set_derived(
        Field::EnergyShield,
        query(es, parts.reasons),
        &[Field::Intelligence],
    );
    Ok(())
}

/// Unreserved life and mana: the pool less its reservation
/// (`doActorLifeManaReservation`: `max - reserved`). No reservation is
/// modelled yet: every enabled gem (the only source of reservation in the
/// slice's builds) makes the fields `Partial`, so the pool is the value.
fn unreserved(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    for (field, pool) in [
        (Field::LifeUnreserved, Field::Life),
        (Field::ManaUnreserved, Field::Mana),
    ] {
        let Some((node, value)) = calc.field(pool).and_then(|f| Some((f.node?, f.value?))) else {
            continue;
        };
        let value = 0.0 + value;
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
        }
    }
}

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

use super::{ArmourSlot, Defence, PieceValue, Poe1Field, Poe1Quantity};
use crate::condition::QueryContext;
use crate::engine::{Calc, Phase};
use crate::error::CalcError;
use crate::graph::{Node, NodeIndex, Term};
use crate::math;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::Field;
use crate::state::{Reasons, StateKind};
use crate::store::Query;

/// The phases, in order.
pub const PHASES: [Phase; 7] = [
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
        name: "unreserved",
        run: unreserved,
    },
    Phase {
        name: "spell-hit",
        run: super::hit::spell_hit,
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
    let cx = QueryContext::default();
    let (store, graph) = calc.parts();
    let q = store.sum(graph, quantity, op, &cx, term)?;
    reasons.extend(&q.reasons);
    Ok(Valued {
        node: q.node,
        value: q.value,
    })
}

/// Whether the store holds any effect on `quantity`.
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
    for t in [
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Lightning,
        DamageType::Chaos,
    ] {
        let mut reasons = Reasons::new();
        let quantity = Quantity::Resistance(t);
        let base = sum(calc, quantity, Op::Base, Term::Base, &mut reasons)?;
        let cx = QueryContext::default();
        let (store, graph) = calc.parts();
        let inc = store.increased(graph, quantity, &cx)?;
        let more = store.more(graph, quantity, &cx)?;
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

        let max_base = sum(
            calc,
            Quantity::MaximumResistance(t),
            Op::Base,
            Term::Step("maximum-sum"),
            &mut reasons,
        )?;
        let cap = constant(calc, "maximum-resistance-cap", MAX_RESIST_CAP)?;
        let max = pair(calc, max_base, cap, Term::Step("maximum-capped"), true)?;
        let max = trunc(calc, max, Term::Step("max"))?;
        let floor = constant(calc, "resistance-floor", RESIST_FLOOR)?;
        let min = trunc(calc, floor, Term::Step("min"))?;

        let capped = pair(calc, total, max, Term::Step("capped"), true)?;
        let value = pair(calc, capped, min, Term::Result, false)?;
        calc.set(Field::Resistance(t), query(value, reasons));
    }
    Ok(())
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
    let cx = QueryContext::default();
    let mut reasons = Reasons::new();
    let shield_q = Quantity::Poe1(Poe1Quantity::Piece(ArmourSlot::Weapon2, PieceValue::Block));
    let mut base_terms = Vec::new();
    if has(calc, shield_q) {
        let b = sum(
            calc,
            shield_q,
            Op::Base,
            Term::Step("shield-block-base"),
            &mut reasons,
        )?;
        let inc = sum(
            calc,
            shield_q,
            Op::Increased,
            Term::Step("shield-block-increased-sum"),
            &mut reasons,
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
        base_terms.push(Valued { node, value });
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
    let more = store.more(graph, Quantity::BlockChance, &cx)?;
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
    calc.set(Field::BlockChance, query(value, reasons));

    let mut reasons = Reasons::new();
    let base = sum(
        calc,
        Quantity::Poe1(Poe1Quantity::SpellBlockChance),
        Op::Base,
        Term::Base,
        &mut reasons,
    )?;
    reasons.extend(&modifier_reasons);
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
    let cx = QueryContext::default();
    let (store, graph) = calc.parts();
    let more = store.more(graph, quantity, &cx)?;
    reasons.extend(&more.reasons);
    product(
        calc,
        &[
            inc,
            Valued {
                node: more.node,
                value: more.value,
            },
        ],
        Term::Step("modifier"),
    )
}

/// The value of `defence` on the armour piece in `slot` (`Classes/
/// Item.lua`): `round((min + local flat + range x percentile) x (1 + local
/// inc / 100) x (1 + quality / 100))`; the percentile is 1 when the item
/// gives none. Its nodes are `step.piece-*`, and the value's term is
/// [`Term::Base`]: the row's base.
fn piece(
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
    let percentile = if has(calc, q(PieceValue::Percentile(defence))) {
        sum(
            calc,
            q(PieceValue::Percentile(defence)),
            Op::Base,
            Term::Step("piece-percentile"),
            reasons,
        )?
    } else {
        constant(calc, "default-percentile", 1.0)?
    };
    let rolled = product(calc, &[variance, percentile], Term::Step("piece-rolled"))?;
    let base = total(calc, &[flat, rolled], Term::Step("piece-base"))?;
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
    let raw = product(calc, &[base, inc, quality], Term::Step("piece-unrounded"))?;
    round(calc, raw, Term::Base)
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
    let dex_bonus = bonus(
        calc,
        Field::Dexterity,
        0.2,
        "fifth",
        "evasion-from-dexterity",
    )?;
    for (defence, field, quantity, extra, dep) in [
        (
            Defence::EnergyShield,
            Field::EnergyShield,
            Quantity::EnergyShield,
            int_bonus,
            Some(Field::Intelligence),
        ),
        (Defence::Armour, Field::Armour, Quantity::Armour, None, None),
        (
            Defence::Evasion,
            Field::Evasion,
            Quantity::Evasion,
            dex_bonus,
            Some(Field::Dexterity),
        ),
    ] {
        let mut reasons = Reasons::new();
        let modifier = defence_modifier(calc, quantity, extra, &mut reasons)?;
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
        let sum = total(calc, &rows, Term::Step("slots"))?;
        let rounded = round(calc, sum, Term::Step("rounded"))?;
        let value = at_least(calc, rounded, "minimum", 0.0)?;
        match dep {
            Some(dep) => calc.set_derived(field, query(value, reasons), &[dep]),
            None => calc.set(field, query(value, reasons)),
        }
    }
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
        // Nothing is reserved yet (P3-9a, P3-12): the pool itself, as a
        // one-term sum so the field has its own result node.
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
            assert_eq!((f64::from(n) * 0.2).floor(), f64::from(n / 5), "{n}");
        }
    }
}

//! The modifier store (Phase 3 plan, step 4): effects keyed by quantity,
//! operation, and conditions, with sum, product, flag, and override
//! queries that each record their provenance in the graph.
//!
//! Every query sorts the effects it combines by source, then value, before
//! combining them, so the order effects were added in never changes a
//! result, to the bit (floating-point sums depend on order).

use core::cmp::Ordering;

use crate::condition::{Applies, QueryContext};
use crate::graph::{Graph, GraphError, Node, NodeIndex, Term};
use crate::math;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, Reasons, UnsupportedReason};

pub use crate::condition::Conditions;

/// One effect of one source on one quantity.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// What it changes.
    pub quantity: Quantity,
    /// How it combines.
    pub op: Op,
    /// The value (ignored for [`Op::Flag`]).
    pub value: f64,
    /// When it applies.
    pub conditions: Conditions,
    /// Where it comes from.
    pub source: Source,
}

/// A query's result: its value, its node in the graph, and the reasons
/// collected on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// The value.
    pub value: f64,
    /// The node holding it.
    pub node: NodeIndex,
    /// Why the value may be incomplete.
    pub reasons: Reasons,
}

/// The canonical order of effects: by source, then by value.
fn canonical(a: &Effect, b: &Effect) -> Ordering {
    a.source
        .cmp(&b.source)
        .then_with(|| a.value.total_cmp(&b.value))
}

/// The effects of a build, in the order they were added.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModStore {
    effects: Vec<Effect>,
}

impl ModStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an effect.
    pub fn add(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// The effects, in the order they were added.
    #[must_use]
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// The number of effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Whether the store has no effect.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// The effects on `quantity` with `op` that apply in `cx`, in canonical
    /// order. An effect whose named conditions `cx` does not settle is left
    /// out and gives `UnmodelledCondition`.
    fn select(
        &self,
        quantity: Quantity,
        op: Op,
        cx: &QueryContext,
        reasons: &mut Reasons,
    ) -> Vec<&Effect> {
        let mut out: Vec<&Effect> = Vec::new();
        for e in self
            .effects
            .iter()
            .filter(|e| e.quantity == quantity && e.op == op)
        {
            match e.conditions.applies(cx) {
                Applies::Yes => out.push(e),
                Applies::No => {}
                Applies::Unknown(keys) => {
                    for condition in keys {
                        reasons
                            .partial
                            .insert(UnsupportedReason::UnmodelledCondition { condition });
                    }
                }
            }
        }
        out.sort_by(|a, b| canonical(a, b));
        out
    }

    /// Appends one contribution leaf per effect.
    fn leaves(graph: &mut Graph, effects: &[&Effect]) -> Result<Vec<NodeIndex>, GraphError> {
        effects
            .iter()
            .map(|e| {
                graph.push(
                    Node::Contribution {
                        source: e.source.clone(),
                        quantity: e.quantity,
                        op: e.op,
                        value: e.value,
                    },
                    e.value,
                )
            })
            .collect()
    }

    /// The sum of the values of `op` effects on `quantity` (from `0`).
    ///
    /// # Errors
    /// A graph error (the graph is full).
    pub fn sum(
        &self,
        graph: &mut Graph,
        quantity: Quantity,
        op: Op,
        cx: &QueryContext,
        term: Term,
    ) -> Result<Query, GraphError> {
        let mut reasons = Reasons::new();
        let effects = self.select(quantity, op, cx, &mut reasons);
        let mut value = 0.0;
        for e in &effects {
            value += e.value;
        }
        let terms = Self::leaves(graph, &effects)?;
        let node = graph.push(Node::Sum { term, terms }, value)?;
        Ok(Query {
            value,
            node,
            reasons,
        })
    }

    /// The increase factor: `1 + (the sum of Increased) / 100`.
    ///
    /// # Errors
    /// A graph error (the graph is full).
    pub fn increased(
        &self,
        graph: &mut Graph,
        quantity: Quantity,
        cx: &QueryContext,
    ) -> Result<Query, GraphError> {
        let sum = self.sum(
            graph,
            quantity,
            Op::Increased,
            cx,
            Term::Step("increased-sum"),
        )?;
        let value = math::percent_factor(sum.value);
        let node = graph.push(
            Node::PercentFactor {
                term: Term::Increased,
                of: sum.node,
            },
            value,
        )?;
        Ok(Query {
            value,
            node,
            reasons: sum.reasons,
        })
    }

    /// The more factor: the product of `1 + value / 100` over the `More`
    /// effects (from `1`).
    ///
    /// # Errors
    /// A graph error (the graph is full).
    pub fn more(
        &self,
        graph: &mut Graph,
        quantity: Quantity,
        cx: &QueryContext,
    ) -> Result<Query, GraphError> {
        let mut reasons = Reasons::new();
        let effects = self.select(quantity, Op::More, cx, &mut reasons);
        let leaves = Self::leaves(graph, &effects)?;
        let mut factors = Vec::with_capacity(leaves.len());
        let mut value = 1.0;
        for (e, leaf) in effects.iter().zip(leaves) {
            let f = math::percent_factor(e.value);
            value *= f;
            factors.push(graph.push(
                Node::PercentFactor {
                    term: Term::Step("more-factor"),
                    of: leaf,
                },
                f,
            )?);
        }
        let node = graph.push(
            Node::Product {
                term: Term::More,
                factors,
            },
            value,
        )?;
        Ok(Query {
            value,
            node,
            reasons,
        })
    }

    /// Whether a `Flag` effect on `quantity` applies: `1` if one does,
    /// else `0`.
    ///
    /// # Errors
    /// A graph error (the graph is full).
    pub fn flag(
        &self,
        graph: &mut Graph,
        quantity: Quantity,
        cx: &QueryContext,
    ) -> Result<Query, GraphError> {
        let mut reasons = Reasons::new();
        let effects = self.select(quantity, Op::Flag, cx, &mut reasons);
        let value = if effects.is_empty() { 0.0 } else { 1.0 };
        // A flag's leaves carry 1, whatever value the effect holds.
        let mut of = Vec::with_capacity(effects.len());
        for e in &effects {
            of.push(graph.push(
                Node::Contribution {
                    source: e.source.clone(),
                    quantity: e.quantity,
                    op: e.op,
                    value: 1.0,
                },
                1.0,
            )?);
        }
        let node = graph.push(
            Node::Any {
                term: Term::Step("flag"),
                of,
            },
            value,
        )?;
        Ok(Query {
            value,
            node,
            reasons,
        })
    }

    /// The `Override` on `quantity`, if one applies. Overrides with
    /// different values contradict each other: the first in canonical
    /// order is returned with `ConflictingOverride`.
    ///
    /// # Errors
    /// A graph error (the graph is full).
    pub fn override_value(
        &self,
        graph: &mut Graph,
        quantity: Quantity,
        cx: &QueryContext,
    ) -> Result<Option<Query>, GraphError> {
        let mut reasons = Reasons::new();
        let effects = self.select(quantity, Op::Override, cx, &mut reasons);
        let Some(first) = effects.first() else {
            return Ok(None);
        };
        if effects
            .iter()
            .any(|e| e.value.to_bits() != first.value.to_bits())
        {
            reasons
                .indeterminate
                .insert(IndeterminateReason::ConflictingOverride { quantity });
        }
        let value = first.value;
        let leaves = Self::leaves(graph, &effects[..1])?;
        let node = graph.push(
            Node::Sum {
                term: Term::Override,
                terms: leaves,
            },
            value,
        )?;
        Ok(Some(Query {
            value,
            node,
            reasons,
        }))
    }

    /// `base * (1 + increased / 100) * more` on `quantity`, or its override
    /// when one applies; the node is the field's [`Term::Result`].
    ///
    /// Unknown conditions of effects with another operation (or on the
    /// override) are reported as well, since each could reach the value.
    ///
    /// # Errors
    /// A graph error (the graph is full).
    pub fn chain(
        &self,
        graph: &mut Graph,
        quantity: Quantity,
        cx: &QueryContext,
    ) -> Result<Query, GraphError> {
        let base = self.sum(graph, quantity, Op::Base, cx, Term::Base)?;
        let inc = self.increased(graph, quantity, cx)?;
        let more = self.more(graph, quantity, cx)?;
        let mut reasons = base.reasons;
        reasons.extend(&inc.reasons);
        reasons.extend(&more.reasons);
        let value = base.value * inc.value * more.value;
        let product = graph.push(
            Node::Product {
                term: Term::Result,
                factors: vec![base.node, inc.node, more.node],
            },
            value,
        )?;
        match self.override_value(graph, quantity, cx)? {
            None => Ok(Query {
                value,
                node: product,
                reasons,
            }),
            Some(over) => {
                reasons.extend(&over.reasons);
                let node = graph.push(
                    Node::Override {
                        by: over.node,
                        replaced: product,
                    },
                    over.value,
                )?;
                Ok(Query {
                    value: over.value,
                    node,
                    reasons,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::{ConditionKey, Flags};
    use crate::quantity::DamageType;

    fn effect(op: Op, value: f64, source: Source) -> Effect {
        Effect {
            quantity: Quantity::Life,
            op,
            value,
            conditions: Conditions::none(),
            source,
        }
    }

    fn rule(name: &'static str) -> Source {
        Source::Rule(name)
    }

    #[test]
    fn sum_should_add_matching_effects_only() {
        let mut s = ModStore::new();
        s.add(effect(Op::Base, 10.0, rule("a")));
        s.add(effect(Op::Base, 5.0, rule("b")));
        s.add(effect(Op::Increased, 50.0, rule("c")));
        s.add(Effect {
            quantity: Quantity::Mana,
            ..effect(Op::Base, 100.0, rule("d"))
        });
        let mut g = Graph::new();
        let q = s
            .sum(
                &mut g,
                Quantity::Life,
                Op::Base,
                &QueryContext::default(),
                Term::Base,
            )
            .unwrap();
        assert_eq!(q.value, 15.0);
        assert!(q.reasons.is_empty());
        assert_eq!(g.len(), 3);
        g.check().unwrap();
    }

    #[test]
    fn increased_should_sum_then_scale() {
        let mut s = ModStore::new();
        s.add(effect(Op::Increased, 50.0, rule("a")));
        s.add(effect(Op::Increased, -20.0, rule("b")));
        let mut g = Graph::new();
        let q = s
            .increased(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap();
        assert_eq!(q.value, 1.3);
        g.check().unwrap();
    }

    #[test]
    fn more_should_multiply_each_factor() {
        let mut s = ModStore::new();
        s.add(effect(Op::More, 50.0, rule("a")));
        s.add(effect(Op::More, -20.0, rule("b")));
        let mut g = Graph::new();
        let q = s
            .more(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap();
        assert_eq!(q.value, 0.8 * 1.5);
        g.check().unwrap();
    }

    #[test]
    fn flag_should_be_one_when_an_effect_applies() {
        let mut s = ModStore::new();
        let mut g = Graph::new();
        let cx = QueryContext::with_flags(Flags::SPELL);
        assert_eq!(s.flag(&mut g, Quantity::Life, &cx).unwrap().value, 0.0);
        s.add(Effect {
            conditions: Conditions::none().with_all(Flags::ATTACK),
            ..effect(Op::Flag, 0.0, rule("a"))
        });
        assert_eq!(s.flag(&mut g, Quantity::Life, &cx).unwrap().value, 0.0);
        s.add(effect(Op::Flag, 0.0, rule("b")));
        assert_eq!(s.flag(&mut g, Quantity::Life, &cx).unwrap().value, 1.0);
        g.check().unwrap();
    }

    #[test]
    fn chain_should_combine_base_increased_and_more() {
        let mut s = ModStore::new();
        s.add(effect(Op::Base, 100.0, rule("a")));
        s.add(effect(Op::Increased, 50.0, rule("b")));
        s.add(effect(Op::More, 10.0, rule("c")));
        let mut g = Graph::new();
        let q = s
            .chain(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap();
        assert_eq!(q.value, 100.0 * 1.5 * 1.1);
        assert_eq!(g.evaluate(q.node).unwrap().to_bits(), q.value.to_bits());
        assert!(matches!(
            g.node(q.node),
            Some(Node::Product {
                term: Term::Result,
                ..
            })
        ));
        g.check().unwrap();
    }

    #[test]
    fn override_should_replace_the_chain() {
        let mut s = ModStore::new();
        s.add(effect(Op::Base, 100.0, rule("a")));
        s.add(effect(Op::Override, 1.0, rule("b")));
        let mut g = Graph::new();
        let q = s
            .chain(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap();
        assert_eq!(q.value, 1.0);
        assert!(q.reasons.is_empty());
        assert!(matches!(g.node(q.node), Some(Node::Override { .. })));
        g.check().unwrap();
    }

    #[test]
    fn conflicting_overrides_should_be_indeterminate() {
        let mut s = ModStore::new();
        s.add(effect(Op::Override, 1.0, rule("b")));
        s.add(effect(Op::Override, 2.0, rule("a")));
        let mut g = Graph::new();
        let q = s
            .override_value(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap()
            .unwrap();
        assert_eq!(q.value, 2.0, "the first by source");
        assert!(
            q.reasons
                .indeterminate
                .contains(&IndeterminateReason::ConflictingOverride {
                    quantity: Quantity::Life
                })
        );
        let mut s2 = ModStore::new();
        s2.add(effect(Op::Override, 1.0, rule("a")));
        s2.add(effect(Op::Override, 1.0, rule("b")));
        let q2 = s2
            .override_value(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap()
            .unwrap();
        assert!(q2.reasons.is_empty(), "equal overrides agree");
        g.check().unwrap();
    }

    #[test]
    fn damage_conditions_should_select_by_type_and_skill_kind() {
        let mut s = ModStore::new();
        let dmg = |value, conditions, name| Effect {
            quantity: Quantity::Damage,
            op: Op::Increased,
            value,
            conditions,
            source: rule(name),
        };
        s.add(dmg(10.0, Conditions::none(), "any"));
        s.add(dmg(
            20.0,
            Conditions::none().with_all(Flags::SPELL),
            "spell",
        ));
        s.add(dmg(
            40.0,
            Conditions::none().with_any(Flags::ELEMENTAL),
            "elemental",
        ));
        s.add(dmg(80.0, Conditions::none().with_all(Flags::FIRE), "fire"));
        s.add(dmg(
            160.0,
            Conditions::none().with_all(Flags::ATTACK),
            "attack",
        ));
        let mut g = Graph::new();
        let cold_spell =
            QueryContext::with_flags(Flags::SPELL | Flags::HIT | Flags::damage(DamageType::Cold));
        let q = s
            .sum(
                &mut g,
                Quantity::Damage,
                Op::Increased,
                &cold_spell,
                Term::Base,
            )
            .unwrap();
        assert_eq!(q.value, 70.0);
        let phys_attack = QueryContext::with_flags(Flags::ATTACK | Flags::PHYSICAL);
        let q = s
            .sum(
                &mut g,
                Quantity::Damage,
                Op::Increased,
                &phys_attack,
                Term::Base,
            )
            .unwrap();
        assert_eq!(q.value, 170.0);
    }

    #[test]
    fn unsettled_condition_should_leave_the_effect_out_and_give_a_reason() {
        let key = ConditionKey::new("on-full-life");
        let mut s = ModStore::new();
        s.add(effect(Op::Base, 10.0, rule("a")));
        s.add(Effect {
            conditions: Conditions::none().with_state(key.clone(), true),
            ..effect(Op::Base, 5.0, rule("b"))
        });
        let mut g = Graph::new();
        let q = s
            .sum(
                &mut g,
                Quantity::Life,
                Op::Base,
                &QueryContext::default(),
                Term::Base,
            )
            .unwrap();
        assert_eq!(q.value, 10.0);
        assert!(
            q.reasons
                .partial
                .contains(&UnsupportedReason::UnmodelledCondition {
                    condition: key.clone()
                })
        );
        let settled = QueryContext::default().with_state(key, true);
        let q = s
            .sum(&mut g, Quantity::Life, Op::Base, &settled, Term::Base)
            .unwrap();
        assert_eq!(q.value, 15.0);
        assert!(q.reasons.is_empty());
    }

    #[test]
    fn chain_should_report_unsettled_conditions_of_every_operation() {
        let key = ConditionKey::new("x");
        let mut s = ModStore::new();
        s.add(Effect {
            conditions: Conditions::none().with_state(key, true),
            ..effect(Op::More, 50.0, rule("a"))
        });
        let mut g = Graph::new();
        let q = s
            .chain(&mut g, Quantity::Life, &QueryContext::default())
            .unwrap();
        assert_eq!(q.reasons.partial.len(), 1);
        assert_eq!(q.value, 0.0);
    }

    #[test]
    fn queries_should_not_depend_on_insertion_order() {
        let values = [0.1, 0.2, 0.3, 1e16, -1e16, 7.25];
        let mut a = ModStore::new();
        let mut b = ModStore::new();
        for (i, v) in values.iter().enumerate() {
            a.add(effect(Op::Base, *v, Source::Gem { group: 0, gem: i }));
        }
        for (i, v) in values.iter().enumerate().rev() {
            b.add(effect(Op::Base, *v, Source::Gem { group: 0, gem: i }));
        }
        let (mut ga, mut gb) = (Graph::new(), Graph::new());
        let cx = QueryContext::default();
        let qa = a
            .sum(&mut ga, Quantity::Life, Op::Base, &cx, Term::Base)
            .unwrap();
        let qb = b
            .sum(&mut gb, Quantity::Life, Op::Base, &cx, Term::Base)
            .unwrap();
        assert_eq!(qa.value.to_bits(), qb.value.to_bits());
        assert_eq!(ga, gb);
        assert_eq!(a.len(), 6);
        assert!(!a.is_empty());
        assert_eq!(a.effects().len(), 6);
    }
}

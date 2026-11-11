//! The provenance graph (ADR-003; Phase 3 plan, decision 8).
//!
//! Each output is a node; each contribution is a leaf naming its source,
//! quantity, operation, and value. Composite nodes say how their children
//! combine and which breakdown term they are ([`Term`]). The graph is the
//! breakdown: the UI and the parity tool read it, never separate strings.
//!
//! Nodes are only appended, and a node's children are always earlier
//! nodes, so the graph is acyclic by construction. Each node records the
//! value the engine computed; [`Graph::evaluate_all`] recomputes every
//! value from the leaves alone, and [`Graph::check`] proves the two agree
//! to the bit.

use crate::math;
use crate::quantity::{Op, Quantity};
use crate::source::Source;

/// A node's position in its graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIndex(u32);

impl NodeIndex {
    /// The position.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0 as usize
    }
}

/// Which breakdown term a node is (`docs/phase-3/slice-fields.json`,
/// `contributions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
    /// The flat base: the sum of `Base` effects and engine base values.
    Base,
    /// `1 + the sum of Increased / 100`.
    Increased,
    /// The product of each `More` factor.
    More,
    /// The value an override sets.
    Override,
    /// A field's value.
    Result,
    /// Another named step of a phase.
    Step(&'static str),
}

/// A node.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// One effect from one source.
    Contribution {
        /// Where it comes from.
        source: Source,
        /// What it changes.
        quantity: Quantity,
        /// How it combines.
        op: Op,
        /// Its value.
        value: f64,
    },
    /// A constant of a rule or of the game data.
    Constant {
        /// What it is.
        label: &'static str,
        /// The value.
        value: f64,
    },
    /// The sum of its terms, in order, from `0`.
    Sum {
        /// The breakdown term.
        term: Term,
        /// The terms.
        terms: Vec<NodeIndex>,
    },
    /// The product of its factors, in order, from `1`.
    Product {
        /// The breakdown term.
        term: Term,
        /// The factors.
        factors: Vec<NodeIndex>,
    },
    /// `1 + of / 100`.
    PercentFactor {
        /// The breakdown term.
        term: Term,
        /// The percentage.
        of: NodeIndex,
    },
    /// `1` if a child is not `0`, else `0` (a flag).
    Any {
        /// The breakdown term.
        term: Term,
        /// The children.
        of: Vec<NodeIndex>,
    },
    /// The value of `by`, replacing `replaced`.
    Override {
        /// The overriding value.
        by: NodeIndex,
        /// What it replaces (kept for the breakdown).
        replaced: NodeIndex,
    },
    /// [`math::floor`].
    Floor {
        /// The breakdown term.
        term: Term,
        /// The input.
        of: NodeIndex,
    },
    /// [`math::round_half_up`].
    RoundHalfUp {
        /// The breakdown term.
        term: Term,
        /// The input.
        of: NodeIndex,
    },
    /// [`math::min`].
    Min {
        /// The breakdown term.
        term: Term,
        /// The first input.
        a: NodeIndex,
        /// The second input.
        b: NodeIndex,
    },
    /// [`math::max`].
    Max {
        /// The breakdown term.
        term: Term,
        /// The first input.
        a: NodeIndex,
        /// The second input.
        b: NodeIndex,
    },
}

impl Node {
    /// The node's children.
    #[must_use]
    pub fn children(&self) -> Vec<NodeIndex> {
        match self {
            Self::Contribution { .. } | Self::Constant { .. } => Vec::new(),
            Self::Sum { terms: c, .. }
            | Self::Product { factors: c, .. }
            | Self::Any { of: c, .. } => c.clone(),
            Self::PercentFactor { of, .. }
            | Self::Floor { of, .. }
            | Self::RoundHalfUp { of, .. } => {
                vec![*of]
            }
            Self::Override { by, replaced } => vec![*by, *replaced],
            Self::Min { a, b, .. } | Self::Max { a, b, .. } => vec![*a, *b],
        }
    }

    /// The node's value from its children's values (`get` reads a child).
    fn evaluate(&self, get: impl Fn(NodeIndex) -> f64) -> f64 {
        match self {
            Self::Contribution { value, .. } | Self::Constant { value, .. } => *value,
            Self::Sum { terms, .. } => terms.iter().fold(0.0, |acc, t| acc + get(*t)),
            Self::Product { factors, .. } => factors.iter().fold(1.0, |acc, f| acc * get(*f)),
            Self::PercentFactor { of, .. } => math::percent_factor(get(*of)),
            Self::Any { of, .. } => {
                if of.iter().any(|c| get(*c) != 0.0) {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Override { by, .. } => get(*by),
            Self::Floor { of, .. } => math::floor(get(*of)),
            Self::RoundHalfUp { of, .. } => math::round_half_up(get(*of)),
            Self::Min { a, b, .. } => math::min(get(*a), get(*b)),
            Self::Max { a, b, .. } => math::max(get(*a), get(*b)),
        }
    }
}

/// Why a graph cannot be evaluated.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum GraphError {
    /// A node names a child that is not an earlier node of the graph.
    #[error("node {node} names child {child}, which is not an earlier node")]
    BadChild {
        /// The node.
        node: usize,
        /// The child.
        child: usize,
    },
    /// A node's recorded value differs from its evaluation.
    #[error("node {node} records {recorded}, evaluates to {evaluated}")]
    ValueMismatch {
        /// The node.
        node: usize,
        /// The value the engine recorded.
        recorded: f64,
        /// The value the graph gives.
        evaluated: f64,
    },
    /// The graph has more nodes than an index can name.
    #[error("the graph is full")]
    Full,
}

/// A provenance graph.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Graph {
    nodes: Vec<Node>,
    values: Vec<f64>,
}

impl Graph {
    /// An empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph has no node.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// A node.
    #[must_use]
    pub fn node(&self, i: NodeIndex) -> Option<&Node> {
        self.nodes.get(i.get())
    }

    /// The value the engine recorded for a node.
    #[must_use]
    pub fn value(&self, i: NodeIndex) -> Option<f64> {
        self.values.get(i.get()).copied()
    }

    /// Every node with its recorded value, in order.
    pub fn iter(&self) -> impl Iterator<Item = (NodeIndex, &Node, f64)> {
        self.nodes
            .iter()
            .zip(self.values.iter().copied())
            .enumerate()
            .filter_map(|(i, (n, v))| u32::try_from(i).ok().map(|i| (NodeIndex(i), n, v)))
    }

    /// Appends a node with the value the engine computed for it. Its
    /// children must be earlier nodes of this graph.
    ///
    /// # Errors
    /// [`GraphError::BadChild`] for a child that is not an earlier node,
    /// [`GraphError::Full`] past `u32::MAX` nodes.
    pub fn push(&mut self, node: Node, value: f64) -> Result<NodeIndex, GraphError> {
        let at = self.nodes.len();
        if let Some(child) = node.children().into_iter().find(|c| c.get() >= at) {
            return Err(GraphError::BadChild {
                node: at,
                child: child.get(),
            });
        }
        let index = u32::try_from(at).map_err(|_| GraphError::Full)?;
        self.nodes.push(node);
        self.values.push(value);
        Ok(NodeIndex(index))
    }

    /// Appends a node whose value is computed from its children's recorded
    /// values (for phases that combine query results).
    ///
    /// # Errors
    /// As [`Graph::push`].
    pub fn derive(&mut self, node: Node) -> Result<NodeIndex, GraphError> {
        let at = self.nodes.len();
        if let Some(child) = node.children().into_iter().find(|c| c.get() >= at) {
            return Err(GraphError::BadChild {
                node: at,
                child: child.get(),
            });
        }
        let value = node.evaluate(|c| self.values[c.get()]);
        self.push(node, value)
    }

    /// Every node's value recomputed from the leaves, ignoring the values
    /// the engine recorded for composite nodes.
    ///
    /// # Errors
    /// [`GraphError::BadChild`] when a node names a later child (not
    /// possible for a graph built with [`Graph::push`]).
    pub fn evaluate_all(&self) -> Result<Vec<f64>, GraphError> {
        let mut out: Vec<f64> = Vec::with_capacity(self.nodes.len());
        for (at, node) in self.nodes.iter().enumerate() {
            if let Some(child) = node.children().into_iter().find(|c| c.get() >= at) {
                return Err(GraphError::BadChild {
                    node: at,
                    child: child.get(),
                });
            }
            let value = node.evaluate(|c| out[c.get()]);
            out.push(value);
        }
        Ok(out)
    }

    /// One node's value recomputed from the leaves.
    ///
    /// # Errors
    /// As [`Graph::evaluate_all`]; [`GraphError::BadChild`] also for an
    /// index past the end.
    pub fn evaluate(&self, i: NodeIndex) -> Result<f64, GraphError> {
        let all = self.evaluate_all()?;
        all.get(i.get()).copied().ok_or(GraphError::BadChild {
            node: all.len(),
            child: i.get(),
        })
    }

    /// Checks that every recorded value equals its evaluation, bit for bit.
    ///
    /// # Errors
    /// The first [`GraphError::ValueMismatch`], or an evaluation error.
    pub fn check(&self) -> Result<(), GraphError> {
        let all = self.evaluate_all()?;
        for (node, (recorded, evaluated)) in self.values.iter().zip(all).enumerate() {
            if recorded.to_bits() != evaluated.to_bits() {
                return Err(GraphError::ValueMismatch {
                    node,
                    recorded: *recorded,
                    evaluated,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(g: &mut Graph, value: f64) -> NodeIndex {
        g.push(
            Node::Contribution {
                source: Source::Character,
                quantity: Quantity::Life,
                op: Op::Base,
                value,
            },
            value,
        )
        .unwrap()
    }

    #[test]
    fn derive_should_compute_each_node_kind() {
        let mut g = Graph::new();
        let a = leaf(&mut g, 2.5);
        let b = leaf(&mut g, 4.0);
        let sum = g
            .derive(Node::Sum {
                term: Term::Base,
                terms: vec![a, b],
            })
            .unwrap();
        let pct = g
            .derive(Node::PercentFactor {
                term: Term::Increased,
                of: b,
            })
            .unwrap();
        let prod = g
            .derive(Node::Product {
                term: Term::Result,
                factors: vec![sum, pct],
            })
            .unwrap();
        let floor = g
            .derive(Node::Floor {
                term: Term::Result,
                of: a,
            })
            .unwrap();
        let round = g
            .derive(Node::RoundHalfUp {
                term: Term::Result,
                of: a,
            })
            .unwrap();
        let min = g
            .derive(Node::Min {
                term: Term::Result,
                a,
                b,
            })
            .unwrap();
        let max = g
            .derive(Node::Max {
                term: Term::Result,
                a,
                b,
            })
            .unwrap();
        let any = g
            .derive(Node::Any {
                term: Term::Result,
                of: vec![a],
            })
            .unwrap();
        let over = g.derive(Node::Override { by: b, replaced: a }).unwrap();
        let c = g
            .derive(Node::Constant {
                label: "c",
                value: 7.0,
            })
            .unwrap();
        assert_eq!(g.value(sum), Some(6.5));
        assert_eq!(g.value(pct), Some(1.04));
        assert_eq!(g.value(prod), Some(6.5 * 1.04));
        assert_eq!(g.value(floor), Some(2.0));
        assert_eq!(g.value(round), Some(3.0));
        assert_eq!(g.value(min), Some(2.5));
        assert_eq!(g.value(max), Some(4.0));
        assert_eq!(g.value(any), Some(1.0));
        assert_eq!(g.value(over), Some(4.0));
        assert_eq!(g.value(c), Some(7.0));
        assert_eq!(g.len(), 12);
        g.check().unwrap();
    }

    #[test]
    fn push_should_refuse_a_child_that_is_not_earlier() {
        let mut g = Graph::new();
        let err = g
            .push(
                Node::Floor {
                    term: Term::Result,
                    of: NodeIndex(0),
                },
                0.0,
            )
            .unwrap_err();
        assert_eq!(err, GraphError::BadChild { node: 0, child: 0 });
        assert!(g.is_empty());
    }

    #[test]
    fn check_should_find_a_recorded_value_the_graph_does_not_give() {
        let mut g = Graph::new();
        let a = leaf(&mut g, 1.0);
        g.push(
            Node::Sum {
                term: Term::Base,
                terms: vec![a, a],
            },
            3.0,
        )
        .unwrap();
        assert!(matches!(
            g.check(),
            Err(GraphError::ValueMismatch { node: 1, .. })
        ));
        assert_eq!(g.evaluate(NodeIndex(1)), Ok(2.0));
        assert!(g.evaluate(NodeIndex(5)).is_err());
    }

    #[test]
    fn empty_sum_and_product_should_be_their_identities() {
        let mut g = Graph::new();
        let s = g
            .derive(Node::Sum {
                term: Term::Base,
                terms: vec![],
            })
            .unwrap();
        let p = g
            .derive(Node::Product {
                term: Term::More,
                factors: vec![],
            })
            .unwrap();
        let any = g
            .derive(Node::Any {
                term: Term::Result,
                of: vec![],
            })
            .unwrap();
        assert_eq!(g.value(s), Some(0.0));
        assert_eq!(g.value(p), Some(1.0));
        assert_eq!(g.value(any), Some(0.0));
        assert_eq!(g.iter().count(), 3);
    }
}

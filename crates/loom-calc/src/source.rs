//! Where a contribution comes from: the leaves of the provenance graph
//! (Phase 3 plan, decision 8).

use loom_domain::{Adapted, ItemId, NodeId};

/// The origin of an effect.
///
/// Sources have a total order; the modifier store sorts the effects it
/// combines by it, so the order effects were added in never changes a
/// result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// The character itself (class base values, per-level growth).
    Character,
    /// A line of an item.
    Item {
        /// The item.
        item: ItemId,
        /// The line's index among the item's lines.
        line: usize,
        /// The import tolerance the line needed, carried so the UI can
        /// mark it (plan decision 11).
        adapted: Option<Adapted>,
    },
    /// An allocated passive node.
    Passive {
        /// The node.
        node: NodeId,
    },
    /// A gem.
    Gem {
        /// The socket group's index.
        group: usize,
        /// The gem's index in the group.
        gem: usize,
    },
    /// A config input.
    Config {
        /// The input's name.
        input: String,
    },
    /// The enemy.
    Enemy,
    /// A rule of the engine (for example an attribute's implicit effect),
    /// by name.
    Rule(&'static str),
}

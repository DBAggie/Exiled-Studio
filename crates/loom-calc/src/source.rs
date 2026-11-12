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
    /// An item's base: the properties the game data gives the base
    /// (an armour piece's defence ranges, a shield's block chance).
    ItemBase {
        /// The item.
        item: ItemId,
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
    /// A line of a custom modifier block of the active config set.
    CustomModifier {
        /// The block's index in the set.
        block: usize,
        /// The line's index among the block's lines.
        line: usize,
    },
    /// A custom modifier line of a config input of the active config set
    /// (PoB v2.67.2's legacy `customMods`).
    ConfigInputLine {
        /// The input's name.
        input: String,
        /// The line's index among the input's lines.
        line: usize,
    },
}

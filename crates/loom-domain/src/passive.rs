//! Passive tree specs: the allocated nodes, mastery effects, and jewel
//! sockets the calculations need (Phase 2 plan, decision 1).

use crate::id::{ItemId, NodeId, SpecId, TreeVersion};
use crate::opaque::{Attributes, Ordered};
use crate::value::Sourced;

/// The build's passive specs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Passives {
    /// The active spec, if set.
    pub active: Option<SpecId>,
    /// The specs, with untyped content in place.
    pub specs: Ordered<PassiveSpec>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// A mastery node and the effect chosen on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasteryEffect {
    /// The mastery node.
    pub node: NodeId,
    /// The chosen effect's ID.
    pub effect: u32,
}

/// A jewel socket node and the item in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JewelSocket {
    /// The socket node.
    pub node: NodeId,
    /// The socketed item; `None` when the socket is empty.
    pub item: Option<ItemId>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// Where a spec's typed child sits among its untyped ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecPart {
    /// The jewel socket list ([`PassiveSpec::jewel_sockets`]).
    JewelSockets,
}

/// One passive tree spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassiveSpec {
    /// The spec's ID within the build.
    pub id: SpecId,
    /// The title, if any.
    pub title: Option<String>,
    /// The tree version.
    pub tree_version: Option<TreeVersion>,
    /// Allocated nodes, in source order, with the list's source text.
    pub nodes: Sourced<Vec<NodeId>>,
    /// Chosen mastery effects, in source order, with the list's source
    /// text.
    pub mastery_effects: Sourced<Vec<MasteryEffect>>,
    /// Jewel sockets, with untyped content in place.
    pub jewel_sockets: Ordered<JewelSocket>,
    /// The spec's children: where the socket list sits among untyped
    /// content.
    pub layout: Ordered<SpecPart>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

impl PassiveSpec {
    /// An empty spec.
    #[must_use]
    pub fn new(id: SpecId) -> Self {
        Self {
            id,
            title: None,
            tree_version: None,
            nodes: Sourced::new(Vec::new()),
            mastery_effects: Sourced::new(Vec::new()),
            jewel_sockets: Ordered::new(),
            layout: Ordered::new(),
            attributes: Attributes::new(),
        }
    }
}

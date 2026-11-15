//! Passive tree specs: the allocated nodes, mastery effects, and jewel
//! sockets the calculations need (Phase 2 plan, decision 1).

use crate::id::{ItemId, NodeId, SpecId, TreeVersion};
use crate::opaque::{Attribute, Attributes, Ordered};
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

    /// PoB's attribute for the spec's class number.
    pub const CLASS: &'static str = "classId";
    /// PoB's attribute for the spec's ascendancy number within its class
    /// (0: none).
    pub const ASCENDANCY: &'static str = "ascendClassId";
    /// PoB's attribute for the spec's secondary (bloodline) ascendancy.
    pub const SECONDARY_ASCENDANCY: &'static str = "secondaryAscendClassId";

    /// The spec's class number (PoB's `classId`), when it has one that
    /// reads as a number (passive tree plan, T-h). Typed through the
    /// attribute it is kept in, so PoB's attribute order round-trips byte
    /// for byte.
    #[must_use]
    pub fn class(&self) -> Option<u32> {
        self.number(Self::CLASS)
    }

    /// The spec's ascendancy number (PoB's `ascendClassId`; 0 for none).
    #[must_use]
    pub fn ascendancy(&self) -> Option<u32> {
        self.number(Self::ASCENDANCY)
    }

    /// The spec's secondary ascendancy (PoB's `secondaryAscendClassId`):
    /// preserved, never edited (passive tree plan, T-g).
    #[must_use]
    pub fn secondary_ascendancy(&self) -> Option<u32> {
        self.number(Self::SECONDARY_ASCENDANCY)
    }

    /// Sets the class number, in place when the spec has the attribute.
    pub fn set_class(&mut self, class: u32) {
        self.set(Self::CLASS, class);
    }

    /// Sets the ascendancy number, in place when the spec has the
    /// attribute.
    pub fn set_ascendancy(&mut self, ascendancy: u32) {
        self.set(Self::ASCENDANCY, ascendancy);
    }

    /// Sets the secondary ascendancy, in place when the spec has the
    /// attribute.
    pub fn set_secondary_ascendancy(&mut self, ascendancy: u32) {
        self.set(Self::SECONDARY_ASCENDANCY, ascendancy);
    }

    fn number(&self, name: &str) -> Option<u32> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .and_then(|a| a.value.trim().parse().ok())
    }

    fn set(&mut self, name: &str, value: u32) {
        let value = value.to_string();
        match self.attributes.iter_mut().find(|a| a.name == name) {
            Some(a) => a.value = value,
            None => self.attributes.push(Attribute::new(name, value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_and_ascendancy_should_be_typed_and_set_in_place() {
        let mut s = PassiveSpec::new(SpecId(1));
        s.attributes.push(Attribute::new("ascendClassId", " 2"));
        s.attributes.push(Attribute::new("classId", "x"));
        assert_eq!(s.ascendancy(), Some(2));
        assert_eq!(s.class(), None);
        assert_eq!(s.secondary_ascendancy(), None);
        s.set_class(5);
        s.set_secondary_ascendancy(1);
        let names: Vec<(&str, &str)> = s
            .attributes
            .iter()
            .map(|a| (a.name.as_str(), a.value.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("ascendClassId", " 2"),
                ("classId", "5"),
                ("secondaryAscendClassId", "1")
            ]
        );
    }
}

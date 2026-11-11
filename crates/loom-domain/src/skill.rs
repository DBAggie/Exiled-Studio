//! Skill sets, socket groups, and gems.

use crate::id::{GemId, SkillId, SkillSetId, SlotId};
use crate::opaque::{Attributes, Ordered};
use crate::value::Sourced;

/// The build's skill sets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Skills {
    /// The active skill set, if set.
    pub active: Option<SkillSetId>,
    /// The sets, with untyped content in place.
    pub sets: Ordered<SkillSet>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// One skill set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSet {
    /// The set's ID within the build.
    pub id: SkillSetId,
    /// The title, if any.
    pub title: Option<String>,
    /// Socket groups, with untyped content in place.
    pub groups: Ordered<SocketGroup>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// A group of linked gems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketGroup {
    /// The label, if any.
    pub label: Option<String>,
    /// Whether the group is enabled.
    pub enabled: bool,
    /// The slot the group is socketed in, if any.
    pub slot: Option<SlotId>,
    /// The main active skill (1-based, as builds write it), if set.
    pub main_active_skill: Option<Sourced<u32>>,
    /// Gems, with untyped content in place.
    pub gems: Ordered<Gem>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// One gem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gem {
    /// The gem item, if known.
    pub gem: Option<GemId>,
    /// The skill, if known.
    pub skill: Option<SkillId>,
    /// The name as written, if any.
    pub name: Option<String>,
    /// Level.
    pub level: Sourced<u32>,
    /// Quality.
    pub quality: Sourced<i32>,
    /// Whether the gem is enabled.
    pub enabled: bool,
    /// Attributes not typed.
    pub attributes: Attributes,
}

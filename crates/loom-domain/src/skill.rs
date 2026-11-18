//! Skill sets, socket groups, and gems.

use crate::error::DomainError;
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
    /// How the build's format held the sets.
    pub layout: SkillsLayout,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// How a build's format held its skill sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SkillsLayout {
    /// Each set in its own element.
    #[default]
    Sets,
    /// One set (ID 1, no title, no attributes) whose socket groups the
    /// format held directly, with no set element (PoB's layout before skill
    /// sets, which PoB loads into set 1).
    Flat,
}

impl Skills {
    /// Checks the layout: flat skills hold exactly one set, with ID 1, no
    /// title, and no attributes, and no untyped content beside it.
    ///
    /// # Errors
    /// [`DomainError::FlatSkillsLayout`] when they do not.
    pub fn check_layout(&self) -> Result<(), DomainError> {
        if self.layout == SkillsLayout::Sets {
            return Ok(());
        }
        match self.sets.entries() {
            [crate::opaque::Entry::Typed(set)]
                if set.id == SkillSetId(1) && set.title.is_none() && set.attributes.is_empty() =>
            {
                Ok(())
            }
            _ => Err(DomainError::FlatSkillsLayout),
        }
    }
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

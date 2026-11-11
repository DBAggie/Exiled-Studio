//! What a modifier line resolved to (ADR-015 and its Amendment 1).
//!
//! A resolved line is stat IDs with values from a stat description, or a
//! typed result per `kind` read from game data. A line resolved only
//! through an import tolerance carries [`Adapted`]. A line that does not
//! resolve is quarantined with a reason, never dropped.

use crate::id::{GemId, StatId};

/// A stat's value as read from a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatValue {
    /// Shown as one number: the raw values that show as it.
    Exact {
        /// Lowest raw value.
        min: i64,
        /// Highest raw value.
        max: i64,
    },
    /// Shown as a range `(a-b)`: from the lowest raw value of `a` to the
    /// highest of `b`.
    Ranged {
        /// Lowest raw value.
        min: i64,
        /// Highest raw value.
        max: i64,
    },
    /// Not shown in the text; `assumed` is a value the description's
    /// condition allows, if one is clear.
    Hidden {
        /// The assumed value.
        assumed: Option<i64>,
    },
    /// Shown as a name: every key with that name, sorted.
    Lookup(Vec<i64>),
}

/// A stat ID and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatLine {
    /// The stat.
    pub stat: StatId,
    /// Its value.
    pub value: StatValue,
}

/// A typed result read from game data (ADR-015 Amendment 1). Each variant
/// is one `kind`; its names come from game data tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedResolution {
    /// `granted-skill`: the line names a skill the item grants.
    GrantedSkill {
        /// The data row that grants the skill (for example a graft's ID).
        source: String,
        /// The granted skill's gem.
        gem: GemId,
        /// The skill level read from the line.
        level: u32,
    },
    /// `item-property`: a display property, not a modifier.
    ItemProperty {
        /// The property's data ID.
        property: String,
        /// Values read from the line, by placeholder, in text order.
        values: Vec<PropertyValue>,
    },
    /// `catalyst-quality`: a quality type and the stats it carries.
    CatalystQuality {
        /// The quality type's data ID.
        quality_type: String,
        /// The type's stats.
        stats: Vec<StatLine>,
        /// The quality read from the line.
        quality: i64,
    },
}

impl TypedResolution {
    /// The `kind`, as reports name it.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::GrantedSkill { .. } => "granted-skill",
            Self::ItemProperty { .. } => "item-property",
            Self::CatalystQuality { .. } => "catalyst-quality",
        }
    }
}

/// One placeholder value of a property line, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyValue {
    /// The placeholder's number.
    pub slot: u8,
    /// The value's text (a plain decimal number).
    pub text: String,
}

/// What a line resolved to. A line the import pipeline's ambiguity filter
/// narrowed to one description is resolved like any other; the filter's
/// evidence (dropped candidates, notes) is report-only, as are the
/// mod-backing and composed-wrapper details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// Stat IDs with values, from a stat description.
    Stats(Vec<StatLine>),
    /// A typed result.
    Typed(TypedResolution),
}

/// The import tolerance a line needed (ADR-015 Amendment 1). Persisted
/// with the line; an exporter keeps the line's source text (Phase 2 plan,
/// decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Adapted {
    /// The text differs from the game's in letter case only.
    CaseFolded,
    /// The line resolves together with the next line of another section;
    /// that line points back with [`LineResolution::Continues`].
    SplitSection,
}

impl Adapted {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CaseFolded => "case-folded",
            Self::SplitSection => "split-section",
        }
    }
}

/// Why a line did not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedReason {
    /// No stat description has the line's text.
    NoTemplate,
    /// Descriptions have the text, and none fits its values.
    NoVariantFits,
    /// Only descriptions another one shadows fit.
    OnlyShadowed,
    /// More than one description fits, and the item cannot choose; the
    /// candidates' stats, in description order.
    Ambiguous(Vec<Vec<StatId>>),
}

impl UnresolvedReason {
    /// Stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::NoTemplate => "no-template",
            Self::NoVariantFits => "no-variant-fits",
            Self::OnlyShadowed => "only-shadowed",
            Self::Ambiguous(_) => "ambiguous",
        }
    }
}

/// Why a modifier line is not resolved by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exclusion {
    /// The item's variant selection does not include the line.
    NotSelected,
    /// The line is marked custom (user text, not game text).
    Custom,
    /// The line's section is not resolved (the import adapter names it).
    Section,
}

impl Exclusion {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotSelected => "not-selected",
            Self::Custom => "custom",
            Self::Section => "section",
        }
    }
}

/// The state of a modifier line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineResolution {
    /// Not resolved yet (read, before resolution runs).
    Pending,
    /// Not resolved by design.
    Excluded(Exclusion),
    /// Resolved, with the tolerance it needed, if any.
    Resolved {
        /// The result.
        result: Resolved,
        /// The tolerance, if any.
        adapted: Option<Adapted>,
    },
    /// A later line of a multi-line resolution (a description over
    /// several lines, or the second line of a split-section pair):
    /// resolved with the line at this index of the item's lines, which
    /// carries the result. [`crate::ItemBody::check_links`] checks the
    /// link.
    Continues {
        /// Index of the first line of the pair.
        line: usize,
    },
    /// Not resolved; quarantined with a reason.
    Unresolved(UnresolvedReason),
}

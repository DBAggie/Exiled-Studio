//! Stat-description model (Spike B, PR B1): the game's tables that turn stat
//! IDs and values into the lines shown in game, loaded from the JSON that
//! `loom-extractor extract` writes (`stat_descriptions/**/*.json`, manifest
//! format 3), plus a forward renderer `(stats, values) -> lines`.
//!
//! **Spike-level model.** Phase 2 designs the canonical schema and bundle
//! format and may replace these types. The loader refuses manifest formats
//! it does not know ([`SUPPORTED_MANIFEST_FORMATS`]).
//!
//! Pieces:
//! - [`load`]: manifest and file loaders from bytes (no filesystem).
//! - [`template`]: markup stripping and placeholder parsing of format text.
//! - [`handler`]: handler kinds; numeric semantics with their basis.
//! - [`scope`]: a set of files and the include chain of one file.
//! - [`render`]: the forward renderer and the [`render::Lookup`] trait for
//!   handlers that name rows of other tables.
//!
//! Nothing is dropped silently (ADR-015): an unknown handler is kept as
//! [`Handler::Unknown`], a format text that does not parse keeps its
//! [`template::TemplateError`], and the renderer reports every input stat
//! it did not render with a reason.

pub mod handler;
pub mod load;
pub mod render;
pub mod scope;
pub mod template;

pub use handler::{Basis, Handler, HandlerKind, LookupHandler, NumericHandler, ValueFormat};
pub use load::{LoadError, ManifestFile, SUPPORTED_MANIFEST_FORMATS, StatDescManifest};
pub use render::{
    Lookup, NoLookup, NotRendered, NotRenderedReason, Rendered, RenderedLine, StatValue,
    VariantFilter, render_description,
};
pub use scope::{DescRef, Scope, ScopeError, StatDescriptions};
pub use template::{PlaceholderFormat, Segment, Template, TemplateError};

/// One parsed stat-description file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatDescFile {
    /// Path in the game index, as the extractor recorded it.
    pub source: String,
    /// Included paths as written (case as in the file).
    pub includes: Vec<String>,
    /// Description blocks in file order.
    pub descriptions: Vec<Description>,
    /// Stats the file declares as having no description.
    pub no_description: Vec<String>,
}

/// One `description` or `handed_description` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    /// Name after the keyword, if any.
    pub name: Option<String>,
    /// 1-based line of the keyword in the game file.
    pub line: usize,
    /// Stat IDs (main hand for a handed description).
    pub stats: Vec<String>,
    /// Off-hand stat IDs of a handed description, same length as `stats`.
    pub off_hand_stats: Option<Vec<String>>,
    /// English variants in file order (first match wins).
    pub variants: Vec<Variant>,
}

impl Description {
    /// The stat IDs of one hand.
    #[must_use]
    pub fn stats_of(&self, hand: Hand) -> Option<&[String]> {
        match hand {
            Hand::Main => Some(&self.stats),
            Hand::Off => self.off_hand_stats.as_deref(),
        }
    }
}

/// Which stat list of a description is used. Only handed descriptions have
/// an off hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Hand {
    /// `stats`.
    Main,
    /// `off_hand_stats`.
    Off,
}

/// One English variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// One condition per stat, on the raw stat value.
    pub conditions: Vec<Condition>,
    /// Flagged `gem_quality` (shown for gem quality only).
    pub gem_quality: bool,
    /// Flagged `table_only` (PoE2).
    pub table_only: bool,
    /// Flags without known meaning, kept as written (the renderer reports
    /// a chosen variant that has one).
    pub unknown_flags: Vec<String>,
    /// Handlers in written order.
    pub handlers: Vec<Handler>,
    /// 1-based line in the game file.
    pub line: usize,
    /// Format text as written (escapes and markup kept).
    pub text: String,
    /// The parsed format text, or why it did not parse.
    pub template: Result<Template, TemplateError>,
}

/// A variant condition on one stat's raw value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// `min <= value <= max`; `None` is open (`#`).
    Range {
        /// Lower bound.
        min: Option<i64>,
        /// Upper bound.
        max: Option<i64>,
    },
    /// `!v`: the value is not `v`.
    Not(i64),
}

impl Condition {
    /// Whether `value` satisfies the condition.
    #[must_use]
    pub fn holds(self, value: i64) -> bool {
        match self {
            Self::Range { min, max } => {
                min.is_none_or(|min| value >= min) && max.is_none_or(|max| value <= max)
            }
            Self::Not(v) => value != v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_should_treat_open_bounds_as_unbounded() {
        let c = Condition::Range {
            min: Some(1),
            max: None,
        };
        assert!(c.holds(1) && c.holds(i64::MAX) && !c.holds(0));
    }

    #[test]
    fn condition_should_reject_only_the_excluded_value() {
        assert!(Condition::Not(0).holds(-1) && !Condition::Not(0).holds(0));
    }
}

//! Quarantine and completeness (ADR-014, ADR-015): content a build holds
//! but could not interpret. It is kept for export and shown, never
//! dropped. Opaque content ([`crate::OpaqueNode`]) is not quarantined: the
//! model does not type it on purpose.

use crate::id::ItemId;
use crate::item::UnresolvedReason;

/// Why content is quarantined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuarantineReason {
    /// A modifier line that did not resolve.
    Unresolved(UnresolvedReason),
    /// A modifier line not resolved yet.
    Pending,
    /// An item line the import adapter did not recognize.
    UnrecognizedLine,
    /// A spec line with a key the import adapter does not know.
    UnknownSpec,
}

impl QuarantineReason {
    /// Stable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Unresolved(UnresolvedReason::NoTemplate) => "LINE_NO_TEMPLATE",
            Self::Unresolved(UnresolvedReason::NoVariantFits) => "LINE_NO_VARIANT_FITS",
            Self::Unresolved(UnresolvedReason::OnlyShadowed) => "LINE_ONLY_SHADOWED",
            Self::Unresolved(UnresolvedReason::Ambiguous(_)) => "LINE_AMBIGUOUS",
            Self::Unresolved(UnresolvedReason::NoBase(_)) => "LINE_NO_BASE",
            Self::Pending => "LINE_PENDING",
            Self::UnrecognizedLine => "LINE_UNRECOGNIZED",
            Self::UnknownSpec => "LINE_UNKNOWN_SPEC",
        }
    }
}

/// One quarantined item line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineEntry {
    /// The item.
    pub item: ItemId,
    /// Index of the line among the item's lines.
    pub line: usize,
    /// Why.
    pub reason: QuarantineReason,
}

/// Whether a build holds only content it interpreted (ADR-014). A
/// calculation adds `Indeterminate`; a failed import is an error, not a
/// build, so neither is a build state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completeness {
    /// Nothing is quarantined.
    Complete,
    /// Some content is quarantined.
    Partial,
}

impl Completeness {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
        }
    }
}

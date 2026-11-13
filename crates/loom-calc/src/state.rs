//! Field states and their reasons (ADR-014; Phase 3 plan, decisions 1
//! and 14).
//!
//! A field is `Complete` only when every contribution that can reach it
//! was modelled. Anything else gives a reason from a closed list, so
//! nothing is ignored silently.

use std::collections::BTreeSet;

use loom_domain::{ItemId, StatId};

use crate::condition::ConditionKey;
use crate::quantity::Quantity;
use crate::result::Field;
use crate::source::Source;

/// Why a field is not `Complete`, or not computed (a closed list).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedReason {
    /// A stat the stat-to-effect mapping does not know. It may reach any
    /// field, so it marks every computed field.
    UnmappedStat {
        /// The stat.
        stat: StatId,
        /// Where it comes from.
        source: Source,
    },
    /// An item line that did not resolve (quarantined).
    UnresolvedLine {
        /// The line.
        source: Source,
        /// The resolver's reason (`UnresolvedReason::name`).
        reason: &'static str,
    },
    /// An item line excluded by design whose effect the game may still
    /// apply (custom text, a section the importer does not resolve). Lines
    /// of a variant the item does not select have no effect in game and
    /// give no reason.
    ExcludedLine {
        /// The line.
        source: Source,
        /// The exclusion (`Exclusion::name`).
        exclusion: &'static str,
    },
    /// An item line that was never resolved (the build skipped
    /// resolution).
    PendingLine {
        /// The line.
        source: Source,
    },
    /// A typed resolution (ADR-015 Amendment 1) without a handler.
    UnhandledKind {
        /// The `kind`.
        kind: &'static str,
        /// Where it comes from.
        source: Source,
    },
    /// An effect needs a named condition the build and config do not
    /// settle.
    UnmodelledCondition {
        /// The condition.
        condition: ConditionKey,
        /// The quantity of the effect left out.
        quantity: Quantity,
        /// The source of the effect left out.
        source: Source,
    },
    /// A field derived from another field that is not computed.
    DependsOnNotComputed {
        /// The field it needs.
        field: Field,
    },
    /// The engine does not compute this field yet.
    NotComputed,
    /// An enabled gem whose effects (buffs, auras, reservation) the engine
    /// does not model yet.
    UnmodelledGem {
        /// The gem.
        source: Source,
    },
    /// A config input the engine does not model yet.
    UnmodelledConfig {
        /// The input's name.
        input: String,
    },
    /// A game-specific build choice the engine does not model.
    UnmodelledChoice {
        /// Which choice.
        choice: &'static str,
        /// Its value, as the build writes it.
        value: String,
    },
    /// An item in use whose effect goes beyond its lines (base
    /// properties, an active flask, an abyssal socket, ...), not modelled
    /// yet.
    UnmodelledItem {
        /// The item.
        item: ItemId,
        /// Its slot, as the build names it.
        slot: String,
        /// What is not modelled.
        what: &'static str,
    },
    /// An allocated passive whose stats the data does not hold, by a rule
    /// of the game (a node generated from a jewel), not modelled yet.
    UnmodelledPassive {
        /// The passive.
        source: Source,
        /// What is not modelled.
        what: &'static str,
    },
    /// An item line the import did not read (an unrecognized line or an
    /// unknown spec line).
    UnreadLine {
        /// The line.
        source: Source,
    },
    /// An item line whose values the import format scales in a way the
    /// engine does not model (a catalyst's quality, a corrupted range, an
    /// item's modifier magnitudes): the line gives no effect.
    ScaledLine {
        /// The line.
        source: Source,
        /// The scaling (`catalyst`, `corrupted-range`,
        /// `modifier-magnitude`).
        scaling: &'static str,
    },
    /// The build allocates passives, but the tree version of the data was
    /// not given, so the build's tree version was not checked.
    UncheckedTreeVersion,
    /// Part of the main skill the engine does not model yet (a level stat
    /// with another interpolation, several stat sets, physical hit damage
    /// against the enemy's armour, ...).
    UnmodelledSkill {
        /// The gem (or the rule) it comes from.
        source: Source,
        /// What is not modelled.
        what: &'static str,
    },
}

impl UnsupportedReason {
    /// Stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::UnmappedStat { .. } => "unmapped-stat",
            Self::UnresolvedLine { .. } => "unresolved-line",
            Self::ExcludedLine { .. } => "excluded-line",
            Self::PendingLine { .. } => "pending-line",
            Self::UnhandledKind { .. } => "unhandled-kind",
            Self::UnmodelledCondition { .. } => "unmodelled-condition",
            Self::DependsOnNotComputed { .. } => "depends-on-not-computed",
            Self::NotComputed => "not-computed",
            Self::UnmodelledGem { .. } => "unmodelled-gem",
            Self::UnmodelledConfig { .. } => "unmodelled-config",
            Self::UnmodelledChoice { .. } => "unmodelled-choice",
            Self::UnmodelledItem { .. } => "unmodelled-item",
            Self::UnmodelledPassive { .. } => "unmodelled-passive",
            Self::UnreadLine { .. } => "unread-line",
            Self::ScaledLine { .. } => "scaled-line",
            Self::UncheckedTreeVersion => "unchecked-tree-version",
            Self::UnmodelledSkill { .. } => "unmodelled-skill",
        }
    }
}

/// Why a field's value cannot be settled (the inputs contradict each
/// other, or the data cannot settle a value).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndeterminateReason {
    /// Two overrides of one quantity give different values.
    ConflictingOverride {
        /// The quantity.
        quantity: Quantity,
    },
    /// A stat's value read from its line is not one number (a range, or a
    /// hidden value without an assumption).
    UnsettledValue {
        /// The stat.
        stat: StatId,
        /// Where it comes from.
        source: Source,
    },
    /// The build names something the game data does not have (a class, a
    /// passive, a mastery effect).
    MissingData {
        /// The table.
        table: &'static str,
        /// The key the build gives.
        key: String,
    },
    /// The build's passive tree is another version than the data's.
    TreeVersion {
        /// The build's.
        build: String,
        /// The data's.
        data: String,
    },
}

impl IndeterminateReason {
    /// Stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ConflictingOverride { .. } => "conflicting-override",
            Self::UnsettledValue { .. } => "unsettled-value",
            Self::MissingData { .. } => "missing-data",
            Self::TreeVersion { .. } => "tree-version",
        }
    }
}

/// Content the build holds that has no effect by design, shown with the
/// result (ADR-015 Amendment 2); it changes no state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Note {
    /// A custom modifier line that is a label or a comment
    /// (`TypedResolution::CustomNote`).
    CustomNote {
        /// The line.
        source: Source,
    },
}

impl Note {
    /// The note's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::CustomNote { .. } => "custom-note",
        }
    }
}

/// The reasons a computation collected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reasons {
    /// Content that was not modelled.
    pub partial: BTreeSet<UnsupportedReason>,
    /// Values that could not be settled.
    pub indeterminate: BTreeSet<IndeterminateReason>,
    /// Content without effect by design; never a reason for a state.
    pub notes: BTreeSet<Note>,
}

impl Reasons {
    /// No reason.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether there is no reason (notes are none).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.partial.is_empty() && self.indeterminate.is_empty()
    }

    /// Adds every reason and note of `other`.
    pub fn extend(&mut self, other: &Self) {
        self.partial.extend(other.partial.iter().cloned());
        self.indeterminate
            .extend(other.indeterminate.iter().cloned());
        self.notes.extend(other.notes.iter().cloned());
    }

    /// The state these reasons give: `Indeterminate` if any value is
    /// unsettled, else `Partial` if anything was not modelled, else
    /// `Complete`.
    #[must_use]
    pub fn into_state(self) -> FieldState {
        if !self.indeterminate.is_empty() {
            FieldState::Indeterminate {
                reasons: self.indeterminate,
                partial: self.partial,
            }
        } else if !self.partial.is_empty() {
            FieldState::Partial(self.partial)
        } else {
            FieldState::Complete
        }
    }
}

/// A field's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldState {
    /// Every contribution was modelled.
    Complete,
    /// Computed, but some content that can reach the field was not
    /// modelled.
    Partial(BTreeSet<UnsupportedReason>),
    /// Not computed: outside the engine.
    Unsupported(UnsupportedReason),
    /// The value cannot be settled. `partial` keeps what was not
    /// modelled as well.
    Indeterminate {
        /// Why.
        reasons: BTreeSet<IndeterminateReason>,
        /// Content that was not modelled, if any.
        partial: BTreeSet<UnsupportedReason>,
    },
}

impl FieldState {
    /// The state's kind.
    #[must_use]
    pub const fn kind(&self) -> StateKind {
        match self {
            Self::Complete => StateKind::Complete,
            Self::Partial(_) => StateKind::Partial,
            Self::Unsupported(_) => StateKind::Unsupported,
            Self::Indeterminate { .. } => StateKind::Indeterminate,
        }
    }
}

/// A state's kind, ordered from best to worst: the build-level state is
/// the worst of its fields. `Indeterminate` is worst because its inputs
/// contradict each other; `Unsupported` has no value but no contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateKind {
    /// Every contribution modelled.
    Complete,
    /// Some content not modelled.
    Partial,
    /// Not computed.
    Unsupported,
    /// Not settled.
    Indeterminate,
}

impl StateKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unsupported => "unsupported",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reason_should_be_complete() {
        assert_eq!(Reasons::new().into_state(), FieldState::Complete);
    }

    #[test]
    fn an_unmodelled_reason_should_be_partial() {
        let mut r = Reasons::new();
        r.partial.insert(UnsupportedReason::NotComputed);
        assert_eq!(r.into_state().kind(), StateKind::Partial);
    }

    #[test]
    fn an_unsettled_value_should_win_and_keep_the_partial_reasons() {
        let mut r = Reasons::new();
        r.partial.insert(UnsupportedReason::NotComputed);
        r.indeterminate
            .insert(IndeterminateReason::ConflictingOverride {
                quantity: Quantity::Life,
            });
        let FieldState::Indeterminate { reasons, partial } = r.into_state() else {
            panic!("not indeterminate");
        };
        assert_eq!(reasons.len(), 1);
        assert_eq!(partial.len(), 1);
    }

    #[test]
    fn state_kinds_should_order_from_best_to_worst() {
        assert!(StateKind::Complete < StateKind::Partial);
        assert!(StateKind::Partial < StateKind::Unsupported);
        assert!(StateKind::Unsupported < StateKind::Indeterminate);
        assert_eq!(StateKind::Indeterminate.name(), "indeterminate");
    }

    #[test]
    fn extend_should_merge_without_repeats() {
        let mut a = Reasons::new();
        a.partial.insert(UnsupportedReason::NotComputed);
        let b = a.clone();
        a.extend(&b);
        assert_eq!(a.partial.len(), 1);
        assert!(!a.is_empty());
    }
}

//! Domain model for the build aggregate: identifiers, value objects, and
//! the substructures a build owns (ADR-003).
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access. Modifiers are
//! `(stat_id, values)` pairs or typed results from game data (ADR-015 and
//! its Amendment 1); unknown content is quarantined or kept opaque, never
//! dropped.
//!
//! What is typed is what the calculations need (Phase 2 plan, decision 1):
//! the character, passive specs, skills, items with every line, item sets,
//! and config inputs. Everything else is [`OpaqueNode`] content kept in
//! place among typed siblings ([`Ordered`]). Game-data names (classes,
//! gems, stats, bases, sections) are IDs or text, never Rust enums, and
//! choices only one game has live in [`GameSpecific`] (ADR-001).

mod build;
mod config;
mod error;
mod game;
mod id;
mod item;
mod opaque;
mod passive;
pub mod poe1;
mod quarantine;
mod skill;
mod value;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

pub use build::{Build, BuildPart, Character};
pub use config::{Config, ConfigInput, ConfigSet, ConfigValue};
pub use error::DomainError;
pub use game::{DataVersion, Game, GameSpecific, Poe2Choices};
pub use id::{
    AscendancyId, ClassId, ConfigSetId, GemId, InfluenceId, ItemId, ItemSetId, NodeId, SkillId,
    SkillSetId, SlotId, SpecId, StatId, TreeVersion,
};
pub use item::{
    Adapted, AltVariant, Exclusion, HeaderLine, Item, ItemBody, ItemLine, ItemPart, ItemSet, Items,
    ItemsEntry, LineKind, LineResolution, ModSection, ModifierLine, Prefix, PropertyValue, Rarity,
    Resolved, SlotEntry, StatLine, StatValue, TypedResolution, UnresolvedReason, VariantSelection,
};
pub use opaque::{Attribute, Attributes, Entry, OpaqueNode, Ordered};
pub use passive::{JewelSocket, MasteryEffect, PassiveSpec, Passives, SpecPart};
pub use quarantine::{Completeness, QuarantineEntry, QuarantineReason};
pub use skill::{Gem, SkillSet, Skills, SocketGroup};
pub use value::{Number, Sourced};

//! The item import pipeline (Phase 2 plan, step 2): from a domain item
//! (`loom_domain::ItemBody`) and the game data it needs, to resolved lines.
//! Lifted from Spike B's corpus harness (`tools/parity-cli`) one part per
//! PR; each lift keeps Spike B's results (`docs/phase-2/lift-evidence.md`).
//!
//! Game data is read through [`ItemData`]. Game-specific rules (which mod
//! domain uses which stat-description scope, ...) are per-game data in
//! [`ItemRules`], never constants here (plan decision 5, ADR-001). Rules of
//! a legacy text format (PoB's base-name prefixes, suffixed base names)
//! stay in that format's adapter, which hands this module a [`BaseQuery`].
//!
//! - [`base`]: an item's base rows and stat-description scope (P2-2b).

pub mod base;

pub use base::{BaseError, BaseQuery, BaseRow, FoundBase, ItemData, ItemRules, find_base};

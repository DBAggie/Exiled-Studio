//! Import parsing: resolves modifier text to stat IDs using the game's
//! stat-description tables (ADR-015). Lines that cannot be resolved are
//! quarantined and reported, never silently dropped.
//!
//! - [`resolve`]: the reverse matcher, one item line to stat IDs and raw
//!   values through one stat-description scope (Spike B, PR B2a).
//!
//! PoB's item-text reader (`pob_item`, Spike B PR B3) moved to
//! `loom_serialization::legacy::pob_item` in Phase 2 (plan decision 4): it
//! reads a legacy format, and the import pipeline takes the
//! `loom_domain` item.
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access. Item text is
//! untrusted input: indexing that can panic is linted.

#![warn(clippy::indexing_slicing)]

pub mod resolve;

//! Domain model for the build aggregate: identifiers, value objects, and
//! the substructures a build owns (ADR-003).
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access. Modifiers are
//! `(stat_id, values)` pairs (ADR-015); unknown content is quarantined, never
//! dropped.

//! Import parsing: resolves modifier text to stat IDs using the game's
//! stat-description tables (ADR-015). Lines that cannot be resolved are
//! quarantined and reported, never silently dropped.
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access.

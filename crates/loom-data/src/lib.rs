//! Canonical game-data model for PoE1 and PoE2 and bundle loading through an
//! abstract source rather than file paths (ADR-005, ADR-016).
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access. Shared types
//! carry no PoE1-only assumptions (ADR-001).

//! The calc result document (Phase 3 plan, step 13): a calculation's
//! fields with value, state, and reasons, and its provenance graph, as
//! versioned DTOs (ADR-003, ADR-014). Every parity report reads this
//! document; a later shape change is a new version with a migration
//! (Phase 2 plan, decision 14).
//!
//! Each format version is its own module.

pub mod v1;

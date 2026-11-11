//! The successor JSON build document (ADR-005): versioned DTOs, kept
//! separate from the domain types (ADR-003), and the deterministic
//! conversion between them (Phase 2 plan, step 7).
//!
//! Each format version is its own module. A document is read by the
//! module of its version; migrations (step 9) bring an older document to
//! the current version before it is parsed.

pub mod v1;

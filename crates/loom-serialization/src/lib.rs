//! Versioned persistence DTOs, sequential migrations, and legacy XML/share
//! code adapters (ADR-005, ADR-009). DTOs stay separate from domain types.
//!
//! Held to the core-crate rules (ADR-016) so any host, including a future web
//! host, can read and write builds: no filesystem, thread, clock,
//! environment, or network access.
//!
//! - [`dto`]: the successor JSON build document, by version (ADR-005;
//!   Phase 2 plan, step 7).
//! - [`legacy`]: Path of Building's formats (ADR-005). Today the item-text
//!   reader [`legacy::pob_item`] with its mapping onto the domain item, and
//!   the build XML adapter [`legacy::pob_xml`]; share codes follow (Phase 2
//!   plan, step 10).
//! - [`migrate`]: sequential migrations of the build document, run by
//!   [`dto::read`] before it is parsed (Phase 2 plan, step 9).
//!
//! Legacy input is untrusted: indexing that can panic is linted.

#![warn(clippy::indexing_slicing)]

pub mod dto;
pub mod legacy;
pub mod migrate;

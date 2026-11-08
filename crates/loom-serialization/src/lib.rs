//! Versioned persistence DTOs, sequential migrations, and legacy XML/share
//! code adapters (ADR-005, ADR-009). DTOs stay separate from domain types.
//!
//! Held to the core-crate rules (ADR-016) so any host, including a future web
//! host, can read and write builds: no filesystem, thread, clock,
//! environment, or network access.

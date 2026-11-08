//! Calculation engine. Consumes immutable build snapshots, computes in `f64`
//! with typed canonicalization at the boundaries, and emits a structured
//! provenance graph (ADR-003).
//!
//! Core crate (ADR-016): compiles for `wasm32-unknown-unknown` and does no
//! filesystem, thread, clock, environment, or network access. Scheduling,
//! cancellation, and parallelism live in the host layer.

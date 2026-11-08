//! Application layer: the command/query/event API and the single task that
//! owns the mutable build aggregate (ADR-004). This is the public API that
//! the UI and addons use (ADR-010). Background jobs, cancellation, and
//! parallelism live here, not in the core crates (ADR-016).

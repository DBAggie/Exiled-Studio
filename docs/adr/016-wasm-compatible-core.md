# ADR-016: WASM-Compatible Core

- Status: Accepted
- Date: 2026-11-08

## Decision
The domain, calculation, data-model, and parsing crates must compile for `wasm32-unknown-unknown`. They do not use the filesystem, threads, clocks, or network directly; those are injected by host adapters (Tauri, CLI, a future web host). CI builds these crates for `wasm32` on every PR.

Whether to ship a browser version is **not decided**. This ADR keeps the option cheap.

## Consequences
- Background jobs, cancellation, and parallelism live in the application/host layer, not the calc crates.
- Data bundles are loaded through an abstract source, not file paths.
- Retrofitting this later would be expensive, so it is enforced from Phase 0.

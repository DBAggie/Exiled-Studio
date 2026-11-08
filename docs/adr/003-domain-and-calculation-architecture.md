# ADR-003: Domain and Calculation Architecture

- Status: Accepted
- Date: 2026-02-01

## Decision
Use a behavior-first hybrid rewrite. Capture observable legacy behavior with differential tests, but redesign Rust boundaries rather than mechanically porting Lua modules.

The model is a rich-domain hybrid: a Build aggregate owns well-separated substructures; known concepts enforce invariants; unknown content is explicitly quarantined. Commands mutate the aggregate, calculations consume immutable snapshots, and persistence DTOs remain separate from domain types.

The calculation engine uses f64 internally with typed canonicalization at domain/output boundaries. It exposes a structured provenance graph. Calculations run as deterministic background snapshot jobs with cancellation and stale-result handling; measured incremental caching may be added later.

## Consequences
- Build is the transaction, undo/redo, and persistence boundary.
- Calculation explanations are first-class API data, not UI strings.
- Numeric comparison uses per-field tolerance metadata.
- Full event sourcing and a general-purpose rules DSL are deferred.

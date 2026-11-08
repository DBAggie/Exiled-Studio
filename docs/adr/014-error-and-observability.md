# ADR-014: Errors, Observability, and Diagnostics

- Status: Accepted
- Date: 2026-02-01

## Decision
Use typed Rust errors plus structured warnings and completeness results. Stable error codes support localization, tests, recovery, diagnostics, and triage. The UI formats messages only at the presentation boundary.

Use structured local logs with controlled retention, correlation IDs, default redaction, and user-triggered diagnostic export. Results distinguish complete, partial, indeterminate, and fatal states. Unsupported content is never silently ignored.

## Consequences
- Async commands, calculations, imports, updates, and persistence can be correlated.
- Diagnostic packages remain user-controlled and previewable.
- Error and warning schemas are versioned public contracts.

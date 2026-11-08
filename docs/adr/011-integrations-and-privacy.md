# ADR-011: Integrations, Sharing, and Privacy

- Status: Accepted
- Date: 2026-02-01

## Decision
Use local-first sharing with optional integrations. No account is required. Files and compatible share codes are primary; hosted sharing or synchronization may be added later by explicit consent.

Trade, character import, update, and other network functionality live in dedicated Tauri/application adapters. The domain core is deterministic and offline. Use user-controlled diagnostic packages, structured local logs with controlled retention, explicit redaction, and preview before export. No background telemetry.

## Consequences
- Integrations return typed domain inputs, not UI-specific data.
- Credentials use OS-protected storage when required.
- Privacy boundaries are visible and testable.

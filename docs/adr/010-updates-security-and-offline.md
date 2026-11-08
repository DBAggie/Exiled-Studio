# ADR-010: Updates, Trust, Extensions, and Offline Operation

- Status: Accepted (revised 2026-11-08)
- Date: 2026-02-01

## Decision
Application updates are discovered automatically but require approval. Signed compatible data bundles may update automatically, with rollback and strict-offline/manual modes. Builds use pinned data by default; users explicitly upgrade data versions.

The app is fully useful offline. Network calls are confined to Tauri/application integration adapters and do not enter the domain core.

## Signing (revised)
Until a core reviewer team exists, signing uses two keys: one for the application and one for data bundles, each with a documented rotation and revocation procedure. The offline-root plus delegated-key ceremony is deferred to the first release where more than one maintainer holds release authority.

## Extensibility (revised, replaces "no arbitrary plugins or scripts")
Addons are **API-first and sandboxed**:
- Addons interact with the application only through the public command/query/event API (ADR-004). No addon gets direct access to core memory, the filesystem, or the network through the application.
- First addon transport: a **local MCP server** (ADR-017). Later transports may include local IPC clients and WASM modules with explicit capability grants.
- Addon permissions are scoped: read projections, propose commands, and write commands. Writes require user approval in the app unless the user grants a standing permission.
- Signing sets a **trust level** shown to the user; it is not a hard gate. Unsigned addons run only with explicit user consent and never with more capability than they are granted.
- Declarative data/content packs remain supported. Unsigned content packs are quarantined until the user accepts them.

## Consequences
- Trust, rotation, revocation, downgrade, rollback, quarantine, and addon permission enforcement require tests.
- Existing builds are not silently reinterpreted by automatic data updates.
- The addon API is versioned as a public contract.
- No account or telemetry is required.

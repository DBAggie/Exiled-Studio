# Architecture Decision Records

ADRs are authoritative. To change a decision, add an amendment (or a new ADR that supersedes one) and get maintainer approval; never silently edit an accepted decision. "Revised" means amended during the 2026-11-08 review.

| ADR | Title | Status |
|---|---|---|
| [000](000-context-and-governance.md) | Context, governance, naming (Exiled Studio), license | Revised |
| [001](001-migration-and-platform.md) | Migration posture, calculation authority, platforms | Revised |
| [002](002-application-shell-and-core.md) | Application shell and domain core (Tauri 2, React, Rust) | Accepted |
| [003](003-domain-and-calculation-architecture.md) | Domain and calculation architecture | Accepted |
| [004](004-api-state-and-concurrency.md) | Application API, state, and concurrency | Accepted |
| [005](005-build-and-data-compatibility.md) | Build formats and game-data source | Revised; Amendment 1 (Oodle via external tool); Amendment 2 (GGG's passive tree export as a pinned input, sprites in bundles; proposed 2026-11-14, accepted on merge) |
| [006](006-parity-and-oracle.md) | Parity verification, oracle, divergence register | Revised |
| [007](007-corpus-and-coverage.md) | Parity corpus and coverage | Revised; Amendment 1 (credited community builds, removal on request) |
| [008](008-ui-and-accessibility.md) | UI, UX, and accessibility | Accepted; Amendment 1 (passive tree editor before skills, proposed 2026-11-14, accepted on merge) |
| [009](009-persistence-and-recovery.md) | Persistence, undo, and recovery | Accepted |
| [010](010-updates-security-and-offline.md) | Updates, signing, addons, offline | Revised |
| [011](011-integrations-and-privacy.md) | Integrations, sharing, and privacy | Accepted |
| [012](012-toolchain-and-repository.md) | Repository, toolchain, governance | Revised |
| [013](013-roadmap-and-release-quality.md) | Roadmap, release quality, first milestone | Revised |
| [014](014-error-and-observability.md) | Errors, observability, diagnostics | Accepted |
| [015](015-stat-id-modifier-model.md) | Stat-ID-first modifier model | Accepted (new); Amendment 1 (typed resolutions, adapted lines); Amendment 2 (custom modifier notes and patterns, proposed 2026-11-13, accepted on merge) |
| [016](016-wasm-compatible-core.md) | WASM-compatible core | Accepted (new) |
| [017](017-mcp-reference-addon.md) | Local MCP server as reference addon | Accepted (new) |

New ADRs take the next number and use the same header format (Status, Date, Decision, Consequences).

Amendments are appended to the ADR as `## Amendment N (date): title` sections with Context, Decision, and Consequences. The original text stays unchanged, the Status line names the amendment, and the table above lists it. The maintainer approves an amendment by merging the PR that adds it.

# ADR-017: Local MCP Server as Reference Addon

- Status: Accepted
- Date: 2026-11-08

## Decision
The first Claude integration, and the reference implementation of the addon API (ADR-010), is a **local MCP server** exposed by the application over stdio or localhost.
- Tools and resources map onto the public command/query/event API: read build projections and calculation breakdowns; propose or apply commands (item edits, config changes); compare alternatives.
- **Read-only by default.** Write commands require approval in the app, or a standing grant the user can revoke.
- Write commands go through the normal command path, so they are undoable and recorded in history like any user action.
- The app contains no LLM API key, no model calls, and no outbound network for this feature. MCP clients (Claude Desktop, Claude Code, others) connect to it.
- The MCP surface is versioned with the addon API contract.

## Deferred
An in-app assistant panel that calls the Claude API (key handling, consent for sending build data, cost display).

## Consequences
- MCP tool definitions are a public contract and are tested like the API.
- Localhost transport needs authentication (per-session token) and binds to loopback only.

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
- Localhost transport needs authentication (a bearer token: per session, or kept between starts in the OS's protected store per Amendment 1) and binds to loopback only.

## Amendment 1 (2026-11-14): a token kept between starts (accepted)
Proposed in the UX sprint 2 session (UX-26, `docs/ui/ux-walkthrough.md`); **accepted by the maintainer 2026-11-14** and implemented in the same sprint.

### Context
UX-26: the server takes a new port and a new token at every start, so the user re-runs `claude mcp add` after each restart. The port half needs no decision (this ADR does not fix the port): the shell now tries the last port first and falls back to an ephemeral one (`loom_integrations::mcp::Server::start_on`). The token half does: this ADR says "per-session token", and ADR-011 says credentials use OS-protected storage when required.

### Proposal
- The token is made once and kept between starts in OS-protected storage (Windows Credential Manager, the Secret Service on Linux, the Keychain on macOS), never in `settings.json` or any plain file.
- **Rotate** in the AI connection screen makes a new one at once (the running server takes it; old clients are refused). Stopping the server does not rotate it.
- Where no protected store is available, the token stays per session (today's behaviour), and the screen says why.
- Unchanged: loopback only, the `Host` and `Origin` checks, read-only by default, the standing grant never stored.

### Consequences
- Pro: `claude mcp add` is run once. Con: a long-lived secret on disk (protected by the OS user account) instead of one that dies with the session; a new dependency for the OS store (a reviewed crate, `cargo-deny`).

### Implementation
- `loom_integrations::mcp::token`: `TokenStore`, `OsStore` (the `keyring` crate 3.6.3, MIT or Apache-2.0, with only its native back ends: Windows Credential Manager, the macOS Keychain, the Linux kernel keyring; no D-Bus or C library), `NoStore`, `kept_token`, `rotate`. The credential is named "Exiled Studio MCP server"; it holds only the token.
- The shell (`loom-tauri`) starts the server with the kept token (`Server::start_with`) and has `mcp_rotate`: a new token, kept, and a running server restarted on its port with it. `McpStatus.token_kept` (app API v1, additive) tells the screen which case it is in; the screen has **Rotate token**.
- Linux: the kernel keyring keeps the token for the login session, not across a reboot; after one, a new token is made and kept. Tests and the `e2e` build use `NoStore`, so they never touch the user's credentials.

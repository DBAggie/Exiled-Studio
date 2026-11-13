//! Network-facing adapters: trade, character import, update checks, and the
//! local MCP addon transport (ADR-011, ADR-017). This is the only layer that
//! may use the network. No telemetry.
//!
//! - [`mcp`]: the local MCP server (Phase 4), loopback only.

pub mod mcp;

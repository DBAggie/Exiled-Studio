//! The knowledge MCP server (ADR-018): read-only game data and economy
//! snapshots for Claude sessions, over stdio.
//!
//! - [`knowledge`]: the bundle it serves (the newest installed one signed by a
//!   trusted key, every file's hash checked at startup; or a development
//!   folder, labelled unsigned) and its provenance.
//! - [`snapshots`]: the knowledge pack's economy snapshot files (unsigned,
//!   untrusted input: size caps, a header check).
//! - [`tools`]: the tool list (contract `schemas/knowledge-mcp-tools.v1.json`)
//!   and the calls.
//! - [`rpc`]: one JSON-RPC message in, at most one out.
//!
//! No network, no writes, no telemetry; logs go to stderr only.

pub mod knowledge;
pub mod patches;
pub mod rpc;
pub mod snapshots;
pub mod tools;

mod lookup;

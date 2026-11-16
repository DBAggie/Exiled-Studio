//! The data keys the application trusts: [`loom_bundles::trust_list`]
//! (the release list compiled in, the public test key's only when the host
//! asks: debug and `e2e` builds; `loom-tauri` decides). Moved to
//! `loom-bundles` so the knowledge MCP shares the rule (ADR-018).

pub use loom_bundles::{trust_list, verify_manifest};

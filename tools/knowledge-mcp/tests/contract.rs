//! The tool contract (ADR-018): `tools/list` equals the committed
//! `schemas/knowledge-mcp-tools.v1.json`. A change to the tool list is a
//! reviewed diff of that file:
//! `LOOM_WRITE_KNOWLEDGE_CONTRACT=1 cargo test -p loom-knowledge-mcp --test contract`.

use std::path::PathBuf;

use loom_knowledge_mcp::tools::{self, CONTRACT_VERSION, PROTOCOL_VERSION};
use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn document() -> Value {
    json!({
        "contract_version": CONTRACT_VERSION,
        "protocol_version": PROTOCOL_VERSION,
        "server": "exiled-studio-knowledge",
        "tools": tools::list(),
    })
}

#[test]
fn the_tool_list_should_equal_the_committed_contract() {
    let path = root().join("schemas/knowledge-mcp-tools.v1.json");
    let text = serde_json::to_string_pretty(&document()).unwrap() + "\n";
    if std::env::var_os("LOOM_WRITE_KNOWLEDGE_CONTRACT").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        committed, text,
        "the tool list changed: review it, then rewrite the contract with LOOM_WRITE_KNOWLEDGE_CONTRACT=1"
    );
}

#[test]
fn every_tool_should_be_read_only_and_closed_world() {
    let list = tools::list();
    let tools = list.as_array().unwrap();
    assert_eq!(tools.len(), 8);
    for t in tools {
        assert_eq!(t["annotations"]["readOnlyHint"], true, "{}", t["name"]);
        assert_eq!(t["annotations"]["openWorldHint"], false, "{}", t["name"]);
        assert_eq!(
            t["inputSchema"]["additionalProperties"], false,
            "{}",
            t["name"]
        );
    }
}

#[test]
fn the_protocol_version_should_be_the_app_mcps() {
    let app =
        std::fs::read_to_string(root().join("crates/loom-integrations/src/mcp/tools.rs")).unwrap();
    assert!(app.contains(&format!(
        "pub const PROTOCOL_VERSION: &str = \"{PROTOCOL_VERSION}\";"
    )));
}

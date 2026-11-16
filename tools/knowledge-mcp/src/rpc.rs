//! One JSON-RPC 2.0 message in, at most one out (MCP's stdio transport:
//! one message per line). Mirrors the app MCP's handling
//! (`loom_integrations::mcp`): no batches, notifications get no answer.

use std::io::{BufRead, Read};

use serde_json::{Value, json};

use crate::knowledge::Knowledge;
use crate::tools::{self, CallError, PROTOCOL_VERSION};

/// The longest line read; a longer one is answered with a parse error.
pub const MAX_LINE: usize = 1024 * 1024;

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Reads one line of at most [`MAX_LINE`] bytes into `buf` (without
/// the newline): `Ok(None)` at the end of input, `Ok(Some(false))` when
/// the line was longer (the rest of it is read and dropped).
///
/// # Errors
/// Reading `input`.
pub fn read_line(input: &mut impl BufRead, buf: &mut Vec<u8>) -> std::io::Result<Option<bool>> {
    buf.clear();
    let cap = MAX_LINE as u64 + 1;
    let n = input.by_ref().take(cap).read_until(b'\n', buf)?;
    if n == 0 {
        return Ok(None);
    }
    if buf.last() == Some(&b'\n') {
        buf.pop();
        return Ok(Some(true));
    }
    if buf.len() as u64 <= MAX_LINE as u64 {
        return Ok(Some(true)); // the last line, with no newline
    }
    let mut rest = Vec::new();
    loop {
        rest.clear();
        let n = input.by_ref().take(cap).read_until(b'\n', &mut rest)?;
        if n == 0 || rest.last() == Some(&b'\n') {
            return Ok(Some(false));
        }
    }
}

/// The answer to a line longer than [`MAX_LINE`].
#[must_use]
pub fn too_large() -> String {
    error(&Value::Null, -32700, "message too large").to_string()
}

/// Handles one line; `None` when nothing is to be written (a notification,
/// an empty line).
#[must_use]
pub fn handle(k: &Knowledge, line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if line.len() > MAX_LINE {
        return Some(too_large());
    }
    let Ok(msg) = serde_json::from_str::<Value>(line) else {
        return Some(error(&Value::Null, -32700, "parse error").to_string());
    };
    respond(k, &msg).map(|v| v.to_string())
}

fn respond(k: &Knowledge, msg: &Value) -> Option<Value> {
    let Some(obj) = msg.as_object() else {
        return Some(error(&Value::Null, -32600, "invalid request"));
    };
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        // A response from the client, or junk: nothing to answer.
        return obj
            .get("id")
            .is_none()
            .then(|| error(&Value::Null, -32600, "invalid request"));
    };
    let id = obj.get("id")?.clone();
    let params = obj.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "exiled-studio-knowledge", "title": "Exiled Studio knowledge", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Path of Exile game data from Exiled Studio's signed bundles, and dated economy snapshots. Call get_snapshot_info first. Every answer names its game and patch: never use it for the other game. Quote prices with their as_of date and league."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::list()})),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            match tools::call(k, name, &args) {
                Ok(value) => Ok(json!({
                    "content": [{"type": "text", "text": value.to_string()}],
                    "structuredContent": value,
                    "isError": false
                })),
                Err(CallError::Tool(e)) => Ok(json!({
                    "content": [{"type": "text", "text": format!("{}: {}", e.code, e.message)}],
                    "structuredContent": {"code": e.code, "message": e.message},
                    "isError": true
                })),
                Err(CallError::Params(m)) => Err((-32602, m)),
            }
        }
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => error(&id, code, &message),
    })
}

//! MCP contract tests (Phase 4 plan, gate 3): the committed tool list,
//! the transport's refusals, the read tools, and change requests under the
//! user's permissions, over real loopback HTTP.
//!
//! `LOOM_WRITE_MCP_CONTRACT=1 cargo test -p loom-integrations --test
//! mcp_contract` rewrites `schemas/mcp-tools.v1.json` (a contract change is
//! a reviewed diff of that file).

#![expect(clippy::unwrap_used, reason = "tests; a failure should panic")]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use loom_app::{App, Config, HostCommand, api};
use loom_integrations::mcp::{self, Server};
use serde_json::{Value, json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn synthetic() -> String {
    std::fs::read_to_string(
        root().join("tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml"),
    )
    .unwrap()
}

struct Fixture {
    app: Arc<App>,
    server: Server,
    dir: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.stop();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture(test: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("loom-mcp-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut config = Config::new(
        dir.clone(),
        Some(root().join("crates/loom-data/tests/fixtures/bundle-poe1")),
    );
    config.instance = test.to_owned();
    let app = Arc::new(App::start(config, Arc::new(|_| {})).unwrap());
    app.execute(HostCommand::Api(api::Command::ImportText {
        text: synthetic(),
    }))
    .unwrap();
    // Wait for the calculation.
    for _ in 0..3000 {
        if app.snapshot().calc.state == api::CalcState::Done {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let server = Server::start(Arc::clone(&app)).unwrap();
    Fixture { app, server, dir }
}

/// Sends a raw request; the status and body.
fn send(addr: SocketAddr, head: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let mut s = TcpStream::connect(addr).unwrap();
    s.write_all(head.as_bytes()).unwrap();
    s.write_all(body).unwrap();
    let mut out = Vec::new();
    s.read_to_end(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out).into_owned();
    let status: u16 = text.split(' ').nth(1).unwrap().parse().unwrap();
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.as_bytes().to_vec())
        .unwrap_or_default();
    (status, body)
}

fn post_with(f: &Fixture, headers: &str, body: &Value) -> (u16, Value) {
    let addr = f.server.addr();
    let body = body.to_string();
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\n{headers}\r\n",
        addr.port(),
        body.len()
    );
    let (status, out) = send(addr, &head, body.as_bytes());
    let value = serde_json::from_slice(&out).unwrap_or(Value::Null);
    (status, value)
}

fn auth(f: &Fixture) -> String {
    format!("Authorization: Bearer {}\r\n", f.server.token())
}

fn rpc(f: &Fixture, method: &str, params: Value) -> Value {
    let (status, v) = post_with(
        f,
        &auth(f),
        &json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}),
    );
    assert_eq!(status, 200, "{v}");
    v
}

fn tool(f: &Fixture, name: &str, args: Value) -> Value {
    let v = rpc(f, "tools/call", json!({"name": name, "arguments": args}));
    v["result"].clone()
}

#[test]
fn tool_list_should_equal_the_committed_contract() {
    let f = fixture("contract");
    let v = rpc(&f, "tools/list", json!({}));
    let tools = &v["result"]["tools"];
    let file = root().join("schemas/mcp-tools.v1.json");
    let mut text = serde_json::to_string_pretty(&json!({
        "title": "Exiled Studio MCP tools, contract v1",
        "contract_version": mcp::CONTRACT_VERSION,
        "protocol_version": mcp::PROTOCOL_VERSION,
        "tools": tools,
    }))
    .unwrap();
    text.push('\n');
    if std::env::var_os("LOOM_WRITE_MCP_CONTRACT").is_some() {
        std::fs::write(&file, &text).unwrap();
    }
    let committed = std::fs::read_to_string(&file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        committed, text,
        "the tool list changed: a contract change (rewrite with LOOM_WRITE_MCP_CONTRACT=1 and review)"
    );
    // No tool takes a path (ADR-010).
    assert!(!text.to_lowercase().contains("\"path\""));
}

#[test]
fn transport_should_refuse_requests_without_the_right_token_host_or_origin() {
    let f = fixture("refusals");
    let ping = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});
    assert_eq!(post_with(&f, "", &ping).0, 401, "no token");
    assert_eq!(
        post_with(&f, "Authorization: Bearer wrong\r\n", &ping).0,
        401,
        "a wrong token"
    );
    let mut wrong_length = f.server.token().to_owned();
    wrong_length.push('0');
    assert_eq!(
        post_with(
            &f,
            &format!("Authorization: Bearer {wrong_length}\r\n"),
            &ping
        )
        .0,
        401
    );
    assert_eq!(
        post_with(
            &f,
            &format!("{}Origin: http://evil.example\r\n", auth(&f)),
            &ping
        )
        .0,
        403,
        "a foreign origin"
    );
    assert_eq!(
        post_with(
            &f,
            &format!("{}Origin: http://localhost:5173\r\n", auth(&f)),
            &ping
        )
        .0,
        200,
        "a loopback origin"
    );
    assert_eq!(
        post_with(
            &f,
            &format!("{}MCP-Protocol-Version: 1999-01-01\r\n", auth(&f)),
            &ping
        )
        .0,
        400
    );
    // A rebinding page names its own host.
    let addr = f.server.addr();
    let body = ping.to_string();
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: evil.example:{}\r\n{}Content-Length: {}\r\n\r\n",
        addr.port(),
        auth(&f),
        body.len()
    );
    assert_eq!(send(addr, &head, body.as_bytes()).0, 403, "a foreign host");
    // GET (no server-sent events here) and other paths.
    let head = format!(
        "GET /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{}\r\n",
        addr.port(),
        auth(&f)
    );
    assert_eq!(send(addr, &head, b"").0, 405);
    let head = format!(
        "POST /other HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{}Content-Length: 2\r\n\r\n",
        addr.port(),
        auth(&f)
    );
    assert_eq!(send(addr, &head, b"{}").0, 404);
    // Too large, chunked, malformed.
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{}Content-Length: {}\r\n\r\n",
        addr.port(),
        auth(&f),
        mcp::MAX_BODY + 1
    );
    assert_eq!(send(addr, &head, b"").0, 413);
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nTransfer-Encoding: chunked\r\n\r\n",
        addr.port()
    );
    assert_eq!(send(addr, &head, b"0\r\n\r\n").0, 411);
    assert_eq!(send(addr, "nonsense\r\n\r\n", b"").0, 400);
    // Not JSON; a batch; a notification.
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{}Content-Length: 3\r\n\r\n",
        addr.port(),
        auth(&f)
    );
    let (status, body) = send(addr, &head, b"{x}");
    assert_eq!(status, 400);
    assert!(String::from_utf8_lossy(&body).contains("-32700"));
    let (_, v) = post_with(&f, &auth(&f), &json!([ping]));
    assert_eq!(v["error"]["code"], -32600);
    let (status, _) = post_with(
        &f,
        &auth(&f),
        &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    assert_eq!(status, 202);
}

#[test]
fn a_stopped_servers_token_should_be_dead() {
    let mut f = fixture("restart");
    let old = f.server.token().to_owned();
    let old_addr = f.server.addr();
    assert!(f.app.mcp_status().running);
    f.server.stop();
    std::thread::sleep(Duration::from_millis(50));
    assert!(!f.app.mcp_status().running);
    assert!(TcpStream::connect_timeout(&old_addr, Duration::from_millis(500)).is_err());
    f.server = Server::start(Arc::clone(&f.app)).unwrap();
    assert_ne!(f.server.token(), old);
    let ping = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});
    assert_eq!(
        post_with(&f, &format!("Authorization: Bearer {old}\r\n"), &ping).0,
        401
    );
}

#[test]
fn read_tools_should_answer_with_the_apps_projections() {
    let f = fixture("read");
    let init = rpc(
        &f,
        "initialize",
        json!({"protocolVersion": mcp::PROTOCOL_VERSION, "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1"}}),
    );
    assert_eq!(init["result"]["protocolVersion"], mcp::PROTOCOL_VERSION);
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let build = tool(&f, "get_build", json!({}));
    assert_eq!(build["isError"], false);
    assert_eq!(build["structuredContent"]["build"]["game"], "poe1");
    assert!(!build.to_string().contains("\"path\""), "no file paths");

    let fields = tool(&f, "list_fields", json!({}));
    let list = fields["structuredContent"]["fields"].as_array().unwrap();
    assert!(list.iter().any(|x| x["field"] == "life"));

    let breakdown = tool(&f, "get_breakdown", json!({"field": "life"}));
    assert_eq!(breakdown["structuredContent"]["field"]["field"], "life");
    assert!(
        !breakdown["structuredContent"]["nodes"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    let missing = rpc(
        &f,
        "tools/call",
        json!({"name": "get_breakdown", "arguments": {"field": "nope"}}),
    );
    assert_eq!(missing["error"]["code"], -32602);

    let items = tool(&f, "list_items", json!({}));
    assert!(
        !items["structuredContent"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let item = tool(&f, "get_item", json!({"item": 1}));
    let lines = item["structuredContent"]["lines"].as_array().unwrap();
    let life = lines
        .iter()
        .find(|l| l["text"] == "+15 to maximum Life")
        .unwrap();
    assert_eq!(life["status"], "matched");
    let warnings = tool(&f, "list_warnings", json!({}));
    assert!(
        !warnings["structuredContent"]["warnings"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let edit = json!({"item": 1, "ops": [{"replace": {"line": life["index"], "text": "+30 to maximum Life"}}], "expect": [{"line": life["index"], "text": "+15 to maximum Life"}]});
    let preview = tool(&f, "preview_item_edit", json!({"edit": edit}));
    assert_eq!(preview["isError"], false);
    assert!(
        !preview["structuredContent"]["fields"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !f.app.snapshot().history.can_undo,
        "a preview applies nothing"
    );

    let unknown = rpc(
        &f,
        "tools/call",
        json!({"name": "save_build", "arguments": {}}),
    );
    assert_eq!(unknown["error"]["code"], -32602);
    let unknown = rpc(&f, "resources/list", json!({}));
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn change_requests_should_need_the_users_permission_and_approval() {
    let f = fixture("requests");
    rpc(
        &f,
        "initialize",
        json!({"protocolVersion": mcp::PROTOCOL_VERSION, "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1"}}),
    );
    let item = tool(&f, "get_item", json!({"item": 1}));
    let lines = item["structuredContent"]["lines"]
        .as_array()
        .unwrap()
        .clone();
    let life = lines
        .iter()
        .find(|l| l["text"] == "+15 to maximum Life")
        .unwrap();
    let edit = json!({"item": 1, "ops": [{"replace": {"line": life["index"], "text": "+30 to maximum Life"}}], "expect": [{"line": life["index"], "text": "+15 to maximum Life"}]});

    // Read-only by default.
    let refused = tool(&f, "request_item_edit", json!({"edit": edit}));
    assert_eq!(refused["isError"], true);
    assert_eq!(refused["structuredContent"]["code"], "APP_REQUESTS_OFF");

    // Allowed: it waits for the user.
    f.app.set_permissions(api::McpPermissions {
        requests: true,
        standing_grant: false,
    });
    std::thread::sleep(Duration::from_millis(50));
    let pending = tool(
        &f,
        "request_item_edit",
        json!({"edit": edit, "note": "more life"}),
    );
    assert_eq!(pending["structuredContent"]["status"], "pending");
    assert_eq!(pending["structuredContent"]["client"], "test-client");
    let id = pending["structuredContent"]["id"].as_u64().unwrap();
    assert!(!f.app.snapshot().history.can_undo);

    // The user approves in the app: one undoable step.
    f.app
        .execute(HostCommand::Api(api::Command::Approve { id }))
        .unwrap();
    let status = tool(&f, "get_request", json!({"id": id}));
    assert_eq!(status["structuredContent"]["status"], "applied");
    assert!(f.app.snapshot().history.can_undo);

    // The same request again is stale.
    let stale = tool(&f, "request_item_edit", json!({"edit": edit}));
    assert_eq!(stale["structuredContent"]["code"], "APP_STALE_EDIT");
}

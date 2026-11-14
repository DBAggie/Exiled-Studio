//! The local MCP server, the reference addon (ADR-017, ADR-010; Phase 4
//! plan, decision 11).
//!
//! - **Transport.** MCP's Streamable HTTP, request-response subset: a
//!   POST of one JSON-RPC message gets one JSON answer (202 for a
//!   notification); no server-sent events, so GET and DELETE get 405.
//! - **Exposure.** `127.0.0.1` only, an ephemeral port, off until the user
//!   starts it. Each connection is served on its own thread, at most
//!   [`MAX_CONNECTIONS`] at once, and must send its whole request within
//!   [`http::DEADLINE`], so a slow client cannot hold the server; the head
//!   is checked before the body is read. Tool calls run one at a time (a
//!   preview is a full calculation).
//! - **Authentication.** A random bearer token made at each start (dead at
//!   stop), compared in constant time. The `Host` header must name the
//!   server's own loopback address and an `Origin` header, if sent, must
//!   be a loopback origin (DNS rebinding).
//! - **Permissions.** Reading is allowed while the server runs; change
//!   requests follow the user's settings in the application
//!   ([`loom_app::App::request_change`]).
//!
//! The application makes no outbound connection and holds no key.

mod http;
pub mod tools;

use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use loom_app::App;
use serde_json::{Value, json};

pub use http::{DEADLINE, MAX_BODY};

/// The most connections served at once; more get 503.
pub const MAX_CONNECTIONS: usize = 4;
pub use tools::{CONTRACT_VERSION, PROTOCOL_VERSION};

/// Protocol versions a client may name in `MCP-Protocol-Version`.
const ACCEPTED_VERSIONS: [&str; 2] = ["2025-06-18", "2025-03-26"];

/// A running server.
pub struct Server {
    addr: SocketAddr,
    token: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    app: Arc<App>,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

/// 32 random bytes as hex.
fn token() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Equal without an early exit on the first different byte.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0_u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Server {
    /// Starts the server for `app` and records its address and token on
    /// it.
    ///
    /// # Errors
    /// The I/O error of binding, or of the random source.
    pub fn start(app: Arc<App>) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let addr = listener.local_addr()?;
        let token = token()?;
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Conn {
            app: Arc::clone(&app),
            port: addr.port(),
            token: token.clone(),
            client: Mutex::new("unknown client".to_owned()),
            calls: Mutex::new(()),
            open: AtomicUsize::new(0),
        });
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("loom-mcp".to_owned())
                .spawn(move || {
                    for stream in listener.incoming() {
                        if stop.load(Ordering::SeqCst) {
                            break;
                        }
                        let Ok(stream) = stream else { continue };
                        if state.open.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                            state.open.fetch_sub(1, Ordering::SeqCst);
                            let _ = http::respond(&stream, 503, None, b"");
                            continue;
                        }
                        let conn = Arc::clone(&state);
                        let spawned = std::thread::Builder::new()
                            .name("loom-mcp-conn".to_owned())
                            .spawn(move || {
                                conn.serve(&stream);
                                conn.open.fetch_sub(1, Ordering::SeqCst);
                            });
                        if spawned.is_err() {
                            state.open.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                })?
        };
        app.set_endpoint(Some((url(addr), token.clone())));
        Ok(Self {
            addr,
            token,
            stop,
            thread: Some(thread),
            app,
        })
    }

    /// The address it listens on.
    #[must_use]
    pub const fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The endpoint URL.
    #[must_use]
    pub fn url(&self) -> String {
        url(self.addr)
    }

    /// The bearer token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Stops the server; its token dies with it and the standing grant
    /// ends.
    pub fn stop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.stop.store(true, Ordering::SeqCst);
            // Wake the accept loop.
            let _ = TcpStream::connect(self.addr);
            let _ = thread.join();
            self.app.set_endpoint(None);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn url(addr: SocketAddr) -> String {
    format!("http://{addr}/mcp")
}

struct Conn {
    app: Arc<App>,
    port: u16,
    token: String,
    client: Mutex<String>,
    /// Held while a message is handled: tool calls run one at a time.
    calls: Mutex<()>,
    /// Connections being served.
    open: AtomicUsize,
}

/// Whether an `Origin` header is a loopback origin.
fn loopback_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split(':').next().unwrap_or_default();
    matches!(host, "127.0.0.1" | "localhost")
        && rest
            .split_once(':')
            .is_none_or(|(_, port)| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}

impl Conn {
    fn serve(&self, stream: &TcpStream) {
        let fail = |e: http::ReadError| {
            let status = match e {
                http::ReadError::Malformed => 400,
                http::ReadError::TooLarge => 413,
                http::ReadError::NoLength => 411,
                http::ReadError::Io => return,
            };
            let _ = http::respond(stream, status, None, b"");
        };
        let head = match http::read_head(stream) {
            Ok(h) => h,
            Err(e) => return fail(e),
        };
        // Host, Origin, token, path, and method before reading a body.
        if let Some(status) = self.refuse(&head.request) {
            let _ = http::respond(stream, status, None, b"");
            return;
        }
        let request = match head.body() {
            Ok(r) => r,
            Err(e) => return fail(e),
        };
        let (status, body) = self.answer(&request);
        let content_type = (!body.is_empty()).then_some("application/json");
        let _ = http::respond(stream, status, content_type, &body);
    }

    /// Why a request's head is refused, as a status; `None` when it may
    /// proceed.
    fn refuse(&self, request: &http::Request) -> Option<u16> {
        // DNS rebinding: the Host must be this server's loopback address.
        let host_ok = request.header("host").is_some_and(|h| {
            h == format!("127.0.0.1:{}", self.port) || h == format!("localhost:{}", self.port)
        });
        if !host_ok {
            return Some(403);
        }
        if request
            .header("origin")
            .is_some_and(|o| !loopback_origin(o))
        {
            return Some(403);
        }
        let authorized = request
            .header("authorization")
            .and_then(|a| a.strip_prefix("Bearer "))
            .is_some_and(|t| constant_time_eq(t.as_bytes(), self.token.as_bytes()));
        if !authorized {
            return Some(401);
        }
        if request.target.split('?').next() != Some("/mcp") {
            return Some(404);
        }
        if request.method != "POST" {
            return Some(405);
        }
        if request
            .header("mcp-protocol-version")
            .is_some_and(|v| !ACCEPTED_VERSIONS.contains(&v))
        {
            return Some(400);
        }
        None
    }

    fn answer(&self, request: &http::Request) -> (u16, Vec<u8>) {
        let Ok(message) = serde_json::from_slice::<Value>(&request.body) else {
            return (400, rpc_error(&Value::Null, -32700, "parse error"));
        };
        let _one = self
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match self.handle(&message) {
            Some(response) => (200, serde_json::to_vec(&response).unwrap_or_default()),
            None => (202, Vec::new()),
        }
    }

    /// One JSON-RPC message; `None` for a notification or a response.
    fn handle(&self, message: &Value) -> Option<Value> {
        let Some(obj) = message.as_object() else {
            // Batches were removed in 2025-06-18.
            return Some(rpc_value(&Value::Null, -32600, "invalid request"));
        };
        if obj.get("jsonrpc") != Some(&json!("2.0")) {
            return Some(rpc_value(&Value::Null, -32600, "invalid request"));
        }
        let id = obj.get("id").cloned();
        let method = obj.get("method").and_then(Value::as_str);
        let (Some(id), Some(method)) = (id, method) else {
            // A notification (`notifications/initialized`, ...) or a
            // response to nothing we sent.
            return None;
        };
        let params = obj.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => {
                if let Some(name) = params["clientInfo"]["name"].as_str()
                    && let Ok(mut client) = self.client.lock()
                {
                    *client = name.chars().take(64).collect();
                }
                Ok(json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "exiled-studio", "title": "Exiled Studio", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": "Read-only by default: read the open build and its calculations, and preview item edits. Item edits are requests the user approves in the application (if they allow requests at all). The application calculates every number; never present your own arithmetic as the application's."
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools::list()})),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);
                let client = self.client.lock().map(|c| c.clone()).unwrap_or_default();
                match tools::call(&self.app, &client, name, &args) {
                    Ok(value) => Ok(json!({
                        "content": [{"type": "text", "text": value.to_string()}],
                        "structuredContent": value,
                        "isError": false
                    })),
                    Err(tools::CallError::Tool(e)) => Ok(json!({
                        "content": [{"type": "text", "text": format!("{}: {}", e.code, e.message)}],
                        "structuredContent": {"code": e.code, "message": e.message},
                        "isError": true
                    })),
                    Err(tools::CallError::Params(m)) => Err((-32602, m)),
                }
            }
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => rpc_value(&id, code, &m),
        })
    }
}

fn rpc_value(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Vec<u8> {
    serde_json::to_vec(&rpc_value(id, code, message)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_should_compare_whole_tokens() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }

    #[test]
    fn loopback_origin_should_accept_only_loopback_http_origins() {
        for ok in [
            "http://127.0.0.1",
            "http://localhost:5173",
            "http://127.0.0.1:1",
        ] {
            assert!(loopback_origin(ok), "{ok}");
        }
        for bad in [
            "https://127.0.0.1",
            "http://evil.example",
            "http://localhost.evil.example",
            "http://127.0.0.1.nip.io",
            "http://localhost:",
            "null",
        ] {
            assert!(!loopback_origin(bad), "{bad}");
        }
    }
}

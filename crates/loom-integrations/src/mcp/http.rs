//! The HTTP the MCP transport needs (Phase 4 plan, decision 11): one
//! request per connection, read with fixed limits, answered, then closed.
//! Hand-written on `std::net` so no web stack enters the dependency tree.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// The largest body read.
pub const MAX_BODY: usize = 1024 * 1024;
/// The largest request head (request line and headers).
const MAX_HEAD: usize = 16 * 1024;
/// How long a client may take to send its whole request.
pub const DEADLINE: Duration = Duration::from_secs(10);

/// A request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The method.
    pub method: String,
    /// The target (path and query).
    pub target: String,
    /// Headers with lowercase names, in order.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Request {
    /// The first header with this (lowercase) name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Why a request could not be read; each maps to a status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// Malformed (400).
    Malformed,
    /// Too large (413).
    TooLarge,
    /// Chunked or another transfer coding (411: a length is required).
    NoLength,
    /// The connection failed or timed out (no answer).
    Io,
}

/// A request's head, read; its body not yet.
pub struct Head<'s> {
    /// The request, with an empty body.
    pub request: Request,
    stream: &'s TcpStream,
    reader: BufReader<io::Take<&'s TcpStream>>,
    deadline: Instant,
}

/// Sets the read timeout to what is left of the deadline.
fn arm(stream: &TcpStream, deadline: Instant) -> Result<(), ReadError> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(ReadError::Io);
    }
    stream
        .set_read_timeout(Some(left))
        .map_err(|_| ReadError::Io)
}

/// Reads one request's head (request line and headers) within the whole
/// request's deadline ([`DEADLINE`]): a client that trickles bytes cannot
/// hold the connection longer.
///
/// # Errors
/// [`ReadError`].
pub fn read_head(stream: &TcpStream) -> Result<Head<'_>, ReadError> {
    let deadline = Instant::now() + DEADLINE;
    let mut reader = BufReader::new(stream.take((MAX_HEAD + MAX_BODY) as u64));
    let mut head = 0;
    let mut line = String::new();
    let mut next = |reader: &mut BufReader<_>, line: &mut String| -> Result<(), ReadError> {
        arm(stream, deadline)?;
        line.clear();
        let n = reader.read_line(line).map_err(|_| ReadError::Io)?;
        head += n;
        if n == 0 {
            return Err(ReadError::Io);
        }
        if head > MAX_HEAD {
            return Err(ReadError::TooLarge);
        }
        Ok(())
    };
    next(&mut reader, &mut line)?;
    let mut parts = line.trim_end().split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(ReadError::Malformed);
    };
    if !version.starts_with("HTTP/1.") {
        return Err(ReadError::Malformed);
    }
    let (method, target) = (method.to_owned(), target.to_owned());
    let mut headers = Vec::new();
    loop {
        next(&mut reader, &mut line)?;
        let l = line.trim_end_matches(['\r', '\n']);
        if l.is_empty() {
            break;
        }
        let (name, value) = l.split_once(':').ok_or(ReadError::Malformed)?;
        if name.is_empty() || name.contains(' ') {
            return Err(ReadError::Malformed);
        }
        headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
    }
    let request = Request {
        method,
        target,
        headers,
        body: Vec::new(),
    };
    if request.header("transfer-encoding").is_some() {
        return Err(ReadError::NoLength);
    }
    Ok(Head {
        request,
        stream,
        reader,
        deadline,
    })
}

impl Head<'_> {
    /// Reads the body (after the caller checked the head), within the
    /// same deadline.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn body(mut self) -> Result<Request, ReadError> {
        let length: usize = match self.request.header("content-length") {
            None => 0,
            Some(v) => v.parse().map_err(|_| ReadError::Malformed)?,
        };
        if length > MAX_BODY {
            return Err(ReadError::TooLarge);
        }
        let mut body = vec![0; length];
        let mut at = 0;
        while at < length {
            arm(self.stream, self.deadline)?;
            let n = self
                .reader
                .read(body.get_mut(at..).unwrap_or_default())
                .map_err(|_| ReadError::Io)?;
            if n == 0 {
                return Err(ReadError::Io);
            }
            at += n;
        }
        self.request.body = body;
        Ok(self.request)
    }
}

/// Writes a response and closes the connection.
///
/// # Errors
/// The I/O error.
pub fn respond(
    mut stream: &TcpStream,
    status: u16,
    content_type: Option<&str>,
    body: &[u8],
) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Content Too Large",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        body.len()
    );
    if let Some(t) = content_type {
        head.push_str("Content-Type: ");
        head.push_str(t);
        head.push_str("\r\n");
    }
    if status == 405 {
        head.push_str("Allow: POST\r\n");
    }
    if status == 401 {
        head.push_str("WWW-Authenticate: Bearer\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

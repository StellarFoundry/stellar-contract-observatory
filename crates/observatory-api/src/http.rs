//! A minimal, bounded HTTP/1.1 transport.
//!
//! The API is transport-agnostic; this module is a small, dependency-free
//! adapter suitable for local and test use. It bounds request sizes and reads
//! exactly `Content-Length` bytes. A production-grade server framework can be
//! added later without changing the router or services.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use super::api::Api;

/// Maximum accepted request body size (8 MiB; base64 payloads are smaller).
pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
/// Maximum number of header lines.
pub const MAX_HEADERS: usize = 100;

/// A parsed HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// HTTP method, uppercased.
    pub method: String,
    /// Request path, without query string.
    pub path: String,
    /// Query string, if any.
    pub query: Option<String>,
    /// Lowercased header names to values.
    pub headers: BTreeMap<String, String>,
    /// Raw body bytes.
    pub body: Vec<u8>,
}

impl Request {
    /// Read a header by lowercase name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

/// An HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Status code.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Response body.
    pub body: Vec<u8>,
}

impl Response {
    /// Build an empty response.
    #[must_use]
    pub fn empty(status: u16) -> Self {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Build a JSON response.
    #[must_use]
    pub fn json(status: u16, value: &serde_json::Value) -> Self {
        let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
        Response {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body,
        }
    }

    /// Add a header.
    #[must_use]
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    /// Serialize to wire bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let reason = reason_phrase(self.status);
        let mut out = format!("HTTP/1.1 {} {}\r\n", self.status, reason).into_bytes();
        for (name, value) in &self.headers {
            out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        out.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        out.extend_from_slice(b"Connection: close\r\n\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

/// Parse an HTTP request from a stream.
pub fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(None);
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_ascii_uppercase();
    let target = parts.next().unwrap_or_default().to_string();
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), Some(query.to_string())),
        None => (target, None),
    };

    let mut headers = BTreeMap::new();
    let mut count = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        count += 1;
        if count > MAX_HEADERS {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "too many headers",
            ));
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let length: usize = headers
        .get("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request body too large",
        ));
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;

    Ok(Some(Request {
        method,
        path,
        query,
        headers,
        body,
    }))
}

/// Serve requests until the listener is dropped or an accept error occurs.
pub fn serve(listener: TcpListener, api: Arc<Api>) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let api = Arc::clone(&api);
        std::thread::spawn(move || {
            let client_id = stream
                .peer_addr()
                .map(|addr| addr.ip().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            let now = now_secs();
            if let Ok(Some(request)) = read_request(&mut stream) {
                let response = api.handle(&request, &client_id, now);
                let _ = stream.write_all(&response.to_bytes());
                let _ = stream.flush();
            }
        });
    }
}

/// Current wall-clock time in seconds.
#[must_use]
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_serializes_with_content_length() {
        let response = Response::json(200, &serde_json::json!({ "ok": true }));
        let bytes = response.to_bytes();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: application/json"));
        assert!(text.contains("Content-Length: "));
        assert!(text.contains("{\"ok\":true}"));
    }

    #[test]
    fn reads_a_simple_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(addr).unwrap();
        client
            .write_all(b"POST /api/v1/health HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\n\r\n{}")
            .unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let request = read_request(&mut server).unwrap().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/health");
        assert_eq!(request.body, b"{}");
    }
}

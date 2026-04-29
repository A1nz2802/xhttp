//! HTTP request reading and parsing.
//!
//! Reading and parsing are kept as separate operations:
//!
//! - [`HttpRequest::read_from`] handles the I/O concern: pulling exactly
//!   one full request off a stream, respecting `Content-Length`.
//! - [`HttpRequest::parse`] handles the syntactic concern: turning a raw
//!   request string into a structured value.
//!
//! Splitting them keeps each function testable in isolation and avoids
//! coupling the parser to a particular reader type.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use crate::http::method::HttpMethod;

// `method`, `version`, and `headers` are part of the public surface of a
// parsed request even if no current handler reads them. Suppressing the
// dead-code lint keeps the type complete for future handlers without
// hiding genuinely unused fields elsewhere.
#[allow(dead_code)]
#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    /// Looks up a header by name, ignoring ASCII case.
    ///
    /// HTTP header field names are case-insensitive per RFC 9110, so a
    /// client may send `content-length`, `Content-Length`, or any other
    /// casing and it must compare equal. The `HashMap` stores keys with
    /// whatever case was received, so we cannot use a direct `get`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Reports whether the connection should be closed after this
    /// request, applying HTTP version defaults.
    ///
    /// HTTP/1.0 closes by default and must opt in to persistent
    /// connections via `Connection: keep-alive`. HTTP/1.1 inverted this:
    /// connections persist by default and must opt out via
    /// `Connection: close`. Either way, an explicit header overrides the
    /// version-based default.
    pub fn wants_close(&self) -> bool {
        match self.header("Connection") {
            Some(v) if v.eq_ignore_ascii_case("close") => true,
            Some(v) if v.eq_ignore_ascii_case("keep-alive") => false,
            _ => self.version == "HTTP/1.0",
        }
    }

    /// Reads exactly one HTTP request from `stream` and returns its raw
    /// bytes as a UTF-8 string.
    ///
    /// The procedure is:
    ///
    /// 1. Read line by line until an empty CRLF line marks the end of
    ///    the headers. A single `read()` is insufficient because TCP is
    ///    a byte stream — a request can be split across multiple reads,
    ///    and `read()` may return fewer bytes than were sent.
    /// 2. Parse the headers found so far for `Content-Length` to learn
    ///    how many bytes of body to read. Without this header, a body is
    ///    assumed absent.
    /// 3. Pull *exactly* `Content-Length` more bytes off the stream. The
    ///    body is opaque and may contain `\r\n`, so we cannot keep
    ///    line-reading; we must size-bound the read.
    ///
    /// `BufReader` is used to avoid one syscall per byte: `read_until`
    /// would otherwise be catastrophic on a raw `TcpStream`.
    pub fn read_from<R: Read>(stream: &mut R) -> Result<String, String> {
        let mut reader = BufReader::new(stream);
        let mut buffer: Vec<u8> = Vec::new();

        loop {
            let mut line: Vec<u8> = Vec::new();
            let bytes_read = reader
                .read_until(b'\n', &mut line)
                .map_err(|e| format!("Failed to read from stream: {e}"))?;

            // `read_until` returns 0 only on EOF. Hitting EOF before the
            // header section is terminated means the peer closed the
            // connection mid-request — there is nothing valid to parse.
            if bytes_read == 0 {
                return Err("Connection closed before headers were complete".to_string());
            }

            buffer.extend_from_slice(&line);

            // The empty line after the headers is the only line that
            // consists solely of CRLF, so it serves as the unambiguous
            // boundary between head and body.
            if line == b"\r\n" {
                break;
            }
        }

        // `from_utf8_lossy` is acceptable here because `Content-Length`
        // is ASCII; any non-UTF-8 byte in the headers would already be a
        // protocol violation.
        let head_str = String::from_utf8_lossy(&buffer);
        let content_length: usize = head_str
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                if key.trim().eq_ignore_ascii_case("Content-Length") {
                    value.trim().parse().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        if content_length > 0 {
            // `read_exact` loops internally until the buffer is full,
            // unlike `read` which may return early with a partial read.
            // This is the right primitive when the protocol tells us
            // exactly how many bytes to expect.
            let mut body = vec![0u8; content_length];
            reader
                .read_exact(&mut body)
                .map_err(|e| format!("Failed to read body: {e}"))?;
            buffer.extend_from_slice(&body);
        }

        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    /// Parses a raw HTTP request string into structured fields.
    ///
    /// Expects the canonical wire format: a request line, zero or more
    /// header lines, an empty line, and an optional body — each line
    /// separated by CRLF.
    pub fn parse(raw: &str) -> Result<HttpRequest, String> {
        // The double CRLF is the only delimiter the protocol guarantees
        // between head and body, so it is the safe split point.
        let (head, body) = raw
            .split_once("\r\n\r\n")
            .ok_or("Malformed request".to_string())?;

        let mut head_lines = head.lines();

        // Request Line — e.g. "GET /hello HTTP/1.1".
        let request_line = head_lines
            .next()
            .ok_or("Missing request line".to_string())?;

        let mut parts = request_line.split_whitespace();

        let method = HttpMethod::from_str(parts.next().ok_or("Missing method".to_string())?)?;

        let path = parts.next().ok_or("Missing path".to_string())?.to_string();

        let version = parts
            .next()
            .ok_or("Missing version".to_string())?
            .to_string();

        let mut headers = HashMap::new();

        for line in head_lines {
            if line.is_empty() {
                break;
            };

            let (key, value) = line
                .split_once(":")
                .ok_or(format!("Malformed header: {line}"))?;

            headers.insert(key.to_string(), value.trim().to_string());
        }

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body: if body.is_empty() {
                None
            } else {
                Some(body.to_string())
            },
        })
    }
}

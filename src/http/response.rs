//! HTTP response building and serialization.
//!
//! Two encoding strategies live here because HTTP/1.1 supports two ways
//! of telling the client where the body ends:
//!
//! - `Content-Length`: the entire body size is known up front, sent in
//!   one piece. Used for fixed payloads like cached files or short
//!   strings.
//! - `Transfer-Encoding: chunked`: the body is sent in self-described
//!   chunks, terminated by a zero-length chunk. Used when the body is
//!   produced incrementally and its total size cannot be known in advance.

use std::collections::HashMap;

use super::CRLF;

#[derive(Debug)]
pub struct HttpResponse {
    pub chunked: bool,
    pub version: String,
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Serializes the response to bytes ready to be written to a stream.
    pub fn serialize(&self) -> Vec<u8> {
        if self.chunked {
            self.serialize_chunked()
        } else {
            self.serialize_normal()
        }
    }

    fn serialize_normal(&self) -> Vec<u8> {
        let status_line = format!(
            "{} {} {}{CRLF}",
            self.version, self.status_code, self.reason
        );

        // `Content-Length` is mandatory for keep-alive: without it (and
        // without chunked encoding) the client has no way to know where
        // this response ends and the next one begins.
        let mut headers = format!("Content-Length: {}{CRLF}", self.body.len());

        for (key, value) in &self.headers {
            headers.push_str(&format!("{key}: {value}{CRLF}"));
        }

        // The empty CRLF separates headers from body — the same
        // delimiter the request parser relies on.
        headers.push_str(CRLF);

        let mut response = format!("{status_line}{headers}").into_bytes();

        response.extend(&self.body);

        response
    }

    fn serialize_chunked(&self) -> Vec<u8> {
        let status_line = format!(
            "{} {} {}{CRLF}",
            self.version, self.status_code, self.reason
        );

        // `Transfer-Encoding: chunked` replaces `Content-Length`; sending
        // both is illegal per RFC 9112 because they would conflict.
        let mut headers = format!("Transfer-Encoding: chunked{CRLF}");

        for (key, value) in &self.headers {
            headers.push_str(&format!("{key}: {value}{CRLF}"));
        }

        headers.push_str(CRLF);

        let mut response = format!("{status_line}{headers}").into_bytes();

        // Each chunk is prefixed by its length in hexadecimal followed
        // by CRLF, then the chunk bytes, then CRLF again. The size of 10
        // is arbitrary — production servers tune this for throughput.
        for chunk in self.body.chunks(10) {
            let size_line = format!("{:x}{CRLF}", chunk.len());
            response.extend(size_line.as_bytes());

            response.extend(chunk);
            response.extend(CRLF.as_bytes());
        }

        // The terminator is a zero-sized chunk plus an empty trailer
        // section. Without it the client would wait forever for more
        // data.
        response.extend(format!("0{CRLF}{CRLF}").as_bytes());

        response
    }

    pub fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            chunked: false,
            version: "HTTP/1.1".to_string(),
            status_code: 200,
            reason: "OK".to_string(),
            headers: HashMap::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn ok_chunked(body: &str) -> HttpResponse {
        HttpResponse {
            chunked: true,
            version: "HTTP/1.1".to_string(),
            status_code: 200,
            reason: "OK".to_string(),
            headers: HashMap::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn not_found() -> HttpResponse {
        HttpResponse {
            chunked: false,
            version: "HTTP/1.1".to_string(),
            status_code: 404,
            reason: "Not Found".to_string(),
            headers: HashMap::new(),
            body: "Not Found".as_bytes().to_vec(),
        }
    }

    /// Builds a `400 Bad Request` response carrying `message` as the
    /// body. The message is intended for client-side debugging — it
    /// describes which part of the request failed to parse.
    pub fn bad_request(message: &str) -> HttpResponse {
        HttpResponse {
            chunked: false,
            version: "HTTP/1.1".to_string(),
            status_code: 400,
            reason: "Bad Request".to_string(),
            headers: HashMap::new(),
            body: message.as_bytes().to_vec(),
        }
    }

    #[allow(dead_code)]
    pub fn internal_server_error() -> HttpResponse {
        HttpResponse {
            chunked: false,
            version: "HTTP/1.1".to_string(),
            status_code: 500,
            reason: "Internal Server Error".to_string(),
            headers: HashMap::new(),
            body: "Internal Server Error".as_bytes().to_vec(),
        }
    }
}

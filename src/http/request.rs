use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use crate::http::method::HttpMethod;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn wants_close(&self) -> bool {
        match self.header("Connection") {
            Some(v) if v.eq_ignore_ascii_case("close") => true,
            Some(v) if v.eq_ignore_ascii_case("keep-alive") => false,
            _ => self.version == "HTTP/1.0",
        }
    }

    pub fn read_from<R: Read>(stream: &mut R) -> Result<String, String> {
        let mut reader = BufReader::new(stream);
        let mut buffer: Vec<u8> = Vec::new();

        loop {
            let mut line: Vec<u8> = Vec::new();
            let bytes_read = reader
                .read_until(b'\n', &mut line)
                .map_err(|e| format!("Failed to read from stream: {e}"))?;

            if bytes_read == 0 {
                return Err("Connection closed before headers were complete".to_string());
            }

            buffer.extend_from_slice(&line);

            if line == b"\r\n" {
                break;
            }
        }

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
            let mut body = vec![0u8; content_length];
            reader
                .read_exact(&mut body)
                .map_err(|e| format!("Failed to read body: {e}"))?;
            buffer.extend_from_slice(&body);
        }

        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    pub fn parse(raw: &str) -> Result<HttpRequest, String> {
        let (head, body) = raw
            .split_once("\r\n\r\n")
            .ok_or("Malformed request".to_string())?;

        let mut head_lines = head.lines();

        // Request Line
        // e.g. "GET /hello HTTP/1.1"
        let request_line = head_lines
            .next()
            .ok_or("Missing request line".to_string())?;

        // ["GET", "/hello", "HTTP/1.1"]
        let mut parts = request_line.split_whitespace();

        // "GET"
        let method = HttpMethod::from_str(parts.next().ok_or("Missing method".to_string())?)?;

        // "/hello"
        let path = parts.next().ok_or("Missing path".to_string())?.to_string();

        // "HTTP/1.1"
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

use std::collections::HashMap;

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

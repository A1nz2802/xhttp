use std::collections::HashMap;

pub enum HttpMethod {
    GET,
    POST,
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
}

impl HttpMethod {
    pub fn from_str(method: &str) -> Result<HttpMethod, String> {
        match method {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            _ => Err(format!("Unknown method: {method}")),
        }
    }
}

impl HttpRequest {
    pub fn parse(raw: &str) -> Result<HttpRequest, String> {
        let mut lines = raw.lines();

        // Request Line
        // e.g. "GET /hello HTTP/1.1"
        let request_line = match lines.next() {
            Some(m) => m,
            None => return Err("Missing request line".to_string()),
        };

        // ["GET", "/hello", "HTTP/1.1"]
        let mut parts = request_line.split_whitespace();

        // "GET"
        let method = match parts.next() {
            Some(m) => HttpMethod::from_str(m)?,
            None => return Err("Missing method".to_string()),
        };

        // "/hello"
        let path = match parts.next() {
            Some(p) => p,
            None => return Err("Missing path".to_string()),
        };

        // "HTTP/1.1"
        let version = match parts.next() {
            Some(v) => v,
            None => return Err("Missing version".to_string()),
        };

        let mut headers = HashMap::new();

        for line in lines {
            if line.is_empty() {
                break;
            };

            let mut parts = line.splitn(2, ':');

            let key = match parts.next() {
                Some(k) => k.to_string(),
                None => return Err("Malformed header: missing key".to_string()),
            };

            let value = match parts.next() {
                Some(v) => v.trim().to_string(),
                None => return Err("Malformed header: missing value".to_string()),
            };

            headers.insert(key, value);
        }

        Ok(HttpRequest {
            method,
            path: path.to_string(),
            version: version.to_string(),
            headers,
        })
    }
}

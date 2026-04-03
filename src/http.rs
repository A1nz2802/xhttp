use std::collections::HashMap;

#[derive(Debug)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpMethod {
    pub fn from_str(method: &str) -> Result<HttpMethod, String> {
        match method {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            _ => Err(format!("Unknown method: {method}")),
        }
    }
}

impl HttpRequest {
    pub fn parse(raw: &str) -> Result<HttpRequest, String> {
        let mut lines = raw.lines();

        // Request Line
        // e.g. "GET /hello HTTP/1.1"
        let request_line = lines.next().ok_or("Missing request line".to_string())?;

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

        for line in lines {
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
        })
    }
}

impl HttpResponse {
    pub fn serialize(&self) -> String {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
            self.status_code,
            self.reason,
            self.body.len()
        );

        for (key, value) in &self.headers {
            response.push_str(&format!("{key}: {value}\r\n"));
        }

        response.push_str(&format!("\r\n{}", self.body));

        response
    }
}

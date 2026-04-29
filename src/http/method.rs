//! HTTP request methods.
//!
//! Only the methods actively used by this server are modeled. Any other
//! verb the client sends is rejected at parse time, surfaced to the
//! caller as a `400 Bad Request`.

#[derive(Debug)]
pub enum HttpMethod {
    Get,
    Post,
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

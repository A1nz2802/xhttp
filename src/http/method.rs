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

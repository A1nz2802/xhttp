use std::collections::HashMap;

#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: String,
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

    pub fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status_code: 200,
            reason: "OK".to_string(),
            headers: HashMap::new(),
            body: body.to_string(),
        }
    }

    pub fn not_found() -> HttpResponse {
        HttpResponse {
            status_code: 404,
            reason: "Not Found".to_string(),
            headers: HashMap::new(),
            body: "Not Found".to_string(),
        }
    }
}

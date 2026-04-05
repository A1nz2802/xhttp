use std::{collections::HashMap, str::from_utf8};

#[derive(Debug)]
pub struct HttpResponse {
    pub chunked: bool,
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn serialize(&self) -> String {
        if self.chunked {
            self.serialize_chunked()
        } else {
            self.serialize_normal()
        }
    }

    fn serialize_normal(&self) -> String {
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

    fn serialize_chunked(&self) -> String {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\nTransfer-Encoding: chunked\r\n",
            self.status_code, self.reason,
        );

        for (key, value) in &self.headers {
            response.push_str(&format!("{key}: {value}\r\n"));
        }

        response.push_str("\r\n");

        for chunk in self.body.as_bytes().chunks(10) {
            let chunk_str = match from_utf8(chunk) {
                Ok(s) => s,
                Err(_) => continue,
            };

            response.push_str(&format!("{:x}\r\n", chunk.len()));
            response.push_str(&format!("{chunk_str}\r\n"));
        }

        response.push_str("0\r\n\r\n");

        response
    }

    pub fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            chunked: false,
            status_code: 200,
            reason: "OK".to_string(),
            headers: HashMap::new(),
            body: body.to_string(),
        }
    }

    pub fn ok_chunked(body: &str) -> HttpResponse {
        HttpResponse {
            status_code: 200,
            reason: "OK".to_string(),
            headers: HashMap::new(),
            body: body.to_string(),
            chunked: true,
        }
    }

    pub fn not_found() -> HttpResponse {
        HttpResponse {
            chunked: false,
            status_code: 404,
            reason: "Not Found".to_string(),
            headers: HashMap::new(),
            body: "Not Found".to_string(),
        }
    }
}

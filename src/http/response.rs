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

        let mut headers = format!("Content-Length: {}{CRLF}", self.body.len());

        for (key, value) in &self.headers {
            headers.push_str(&format!("{key}: {value}{CRLF}"));
        }

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

        let mut headers = format!("Transfer-Encoding: chunked{CRLF}");

        for (key, value) in &self.headers {
            headers.push_str(&format!("{key}: {value}{CRLF}"));
        }

        headers.push_str(CRLF);

        let mut response = format!("{status_line}{headers}").into_bytes();

        for chunk in self.body.chunks(10) {
            let size_line = format!("{:x}{CRLF}", chunk.len());
            response.extend(size_line.as_bytes());

            response.extend(chunk);
            response.extend(CRLF.as_bytes());
        }

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
}

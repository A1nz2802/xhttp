mod http;

use std::io::Write;
use std::net::TcpListener;
use std::{collections::HashMap, io::Read};

use http::{HttpRequest, HttpResponse};

fn main() {
    let listener = match TcpListener::bind("127.0.0.1:7878") {
        Ok(listener) => listener,
        Err(e) => panic!("Failed to bind to address: {e}"),
    };

    println!("Server listening on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
                continue;
            }
        };

        let mut buffer = [0u8; 1024];

        let bytes_read = match stream.read(&mut buffer) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Failed to read from stream: {e}");
                continue;
            }
        };

        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        println!("--- Request received ({bytes_read} bytes) ---");
        println!("{request}");

        let http_request = match HttpRequest::parse(&request) {
            Ok(req) => req,
            Err(e) => {
                eprint!("Failed to parse request: {e}");
                continue;
            }
        };

        let response = match http_request.path.as_str() {
            "/ping" => HttpResponse {
                status_code: 200,
                reason: "OK".to_string(),
                headers: HashMap::new(),
                body: "Pong".to_string(),
            },
            _ => HttpResponse {
                status_code: 404,
                reason: "Not Found".to_string(),
                headers: HashMap::new(),
                body: "Not Found".to_string(),
            },
        };

        if let Err(e) = stream.write_all(response.serialize().as_bytes()) {
            eprint!("Failed to write to stream: {e}");
        }

        println!("{:?}", http_request);
        println!("-----------------------");
        println!("{:#?}", http_request);
    }
}

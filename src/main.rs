mod http;

use std::io::{Read, Write};
use std::net::TcpListener;

use http::HttpRequest;

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

        let _http_request = match HttpRequest::parse(&request) {
            Ok(req) => req,
            Err(e) => {
                eprint!("Failed to parse request: {e}");
                continue;
            }
        };

        // ----

        let response = format!(
            "{}\r\n{}\r\n\r\n{}",
            "HTTP/1.1 200 OK", "Content-Length: 4", "Pong"
        );

        match stream.write_all(response.as_bytes()) {
            Ok(()) => {}
            Err(e) => {
                eprint!("Failed to write to stream: {e}");
            }
        };
    }
}

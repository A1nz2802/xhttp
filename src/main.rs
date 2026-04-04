mod http;
mod router;

use std::io::Write;
use std::net::TcpListener;
use std::{collections::HashMap, io::Read};

use http::{HttpRequest, HttpResponse};
use router::Router;

fn handle_ping(_req: &HttpRequest) -> HttpResponse {
    HttpResponse {
        status_code: 200,
        reason: "OK".to_string(),
        headers: HashMap::new(),
        body: "Pong".to_string(),
    }
}

fn handle_echo(req: &HttpRequest) -> HttpResponse {
    let body = match &req.body {
        Some(b) => b.clone(),
        None => "No body received".to_string(),
    };
    HttpResponse {
        status_code: 200,
        reason: "OK".to_string(),
        headers: HashMap::new(),
        body,
    }
}

fn main() {
    let listener = match TcpListener::bind("127.0.0.1:7878") {
        Ok(listener) => listener,
        Err(e) => panic!("Failed to bind to address: {e}"),
    };

    println!("Server listening on http://127.0.0.1:7878");

    let mut router = Router::new();
    router.add_route("/ping", Box::new(handle_ping));
    router.add_route("/echo", Box::new(handle_echo));

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

        println!("{:?}", http_request);
        println!("-----------------------");
        println!("{:#?}", http_request);

        let response = router.handle(&http_request);

        if let Err(e) = stream.write_all(response.serialize().as_bytes()) {
            eprint!("Failed to write to stream: {e}");
        }
    }
}

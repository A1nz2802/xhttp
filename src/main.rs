mod handlers;
mod http;
mod router;
mod thread_pool;

use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::sync::Arc;

use router::Router;

use handlers::{handle_echo, handle_ping, handle_stream};
use http::{HttpRequest, HttpResponse};

fn main() {
    let listener = match TcpListener::bind("127.0.0.1:7878") {
        Ok(listener) => listener,
        Err(e) => panic!("Failed to bind to address: {e}"),
    };

    println!("Server listening on http://127.0.0.1:7878");

    let mut router = Router::new();

    router.add_route("/ping", Box::new(handle_ping));
    router.add_route("/echo", Box::new(handle_echo));
    router.add_route("/stream", Box::new(handle_stream));

    let pool = thread_pool::ThreadPool::new(4);
    let router = Arc::new(router);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
                continue;
            }
        };

        let router = Arc::clone(&router);

        pool.execute(move || {
            let mut buffer = [0u8; 1024];

            let bytes_read = match stream.read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Failed to read from stream: {e}");
                    return;
                }
            };

            let request = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("--- Request received ({bytes_read} bytes) ---");
            println!("{request}");

            let response = match HttpRequest::parse(&request) {
                Ok(http_request) => router.handle(&http_request),
                Err(e) => {
                    eprintln!("Failed to parse request: {e}");
                    HttpResponse::bad_request(&e)
                }
            };

            if let Err(e) = stream.write_all(&response.serialize()) {
                eprintln!("Failed to write to stream: {e}");
            }
        });
    }
}

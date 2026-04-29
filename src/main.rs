mod handlers;
mod http;
mod router;
mod thread_pool;

use std::io::Write;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

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
            if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(5))) {
                eprintln!("Failed to set read timeout: {e}");
                return;
            }

            loop {
                let raw = match HttpRequest::read_from(&mut stream) {
                    Ok(r) => r,
                    Err(_) => break,
                };

                println!("--- Request received ({} bytes) ---", raw.len());

                let (mut response, close) = match HttpRequest::parse(&raw) {
                    Ok(req) => {
                        let close = req.wants_close();
                        (router.handle(&req), close)
                    }
                    Err(e) => {
                        eprintln!("Failed to parse request: {e}");
                        (HttpResponse::bad_request(&e), true)
                    }
                };

                let connection_value = if close { "close" } else { "keep-alive" };
                response
                    .headers
                    .insert("Connection".to_string(), connection_value.to_string());

                if let Err(e) = stream.write_all(&response.serialize()) {
                    eprintln!("Failed to write to stream: {e}");
                    break;
                }

                if close {
                    break;
                }
            }
        });
    }
}

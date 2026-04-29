//! Server entry point: binds a TCP listener, configures the router, and
//! dispatches each accepted connection to a worker in the thread pool.

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

    // The router is configured once and then read by every worker. We
    // freeze it inside an `Arc` so it can be shared across threads
    // without copies — `Arc` only allows shared (read-only) access,
    // which is exactly what handlers need.
    let router = Arc::new(router);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
                continue;
            }
        };

        // Cloned outside the closure so the `move` below captures the
        // clone, not the original. Each iteration produces a fresh
        // handle to the same router; the atomic refcount tracks
        // ownership.
        let router = Arc::clone(&router);

        pool.execute(move || {
            // An idle keep-alive connection without a timeout would tie
            // up a worker indefinitely — a slowloris-style attack could
            // exhaust the pool by opening sockets and never sending
            // anything. The timeout also bounds how long a healthy
            // session may stay idle between requests.
            if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(5))) {
                eprintln!("Failed to set read timeout: {e}");
                return;
            }

            // Each iteration of this loop services one HTTP request.
            // The TCP connection is reused across iterations to honor
            // HTTP/1.1's persistent-connection default, avoiding the
            // cost of a fresh TCP handshake per request. The `while let`
            // pattern terminates the session on any read error — peer
            // closed, read timeout, I/O failure — none of which leave a
            // usable stream behind.
            while let Ok(raw) = HttpRequest::read_from(&mut stream) {
                println!("--- Request received ({} bytes) ---", raw.len());

                // Both branches produce an `HttpResponse` so a single
                // write path covers normal and error cases. The error
                // path always closes the connection: a parse failure
                // means we cannot trust where the next request would
                // begin in the byte stream.
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

                // Echoing `Connection` back is technically redundant on
                // 1.1 (keep-alive is implicit), but it disambiguates
                // intent for older clients and proxies that key off the
                // header rather than the version.
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

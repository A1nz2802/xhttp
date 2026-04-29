//! Built-in request handlers used by the router.

use std::{collections::HashMap, fs::read};

use crate::http::{HttpRequest, HttpResponse};

pub fn handle_ping(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::ok("pong")
}

pub fn handle_echo(req: &HttpRequest) -> HttpResponse {
    let body = match &req.body {
        Some(b) => b.clone(),
        None => "No body received".to_string(),
    };

    HttpResponse::ok(&body)
}

/// Serves a file from the `public/` directory based on the request
/// path. Used as the fallback when no explicit route matches.
///
/// The path is derived directly from the request — a real server would
/// need to canonicalize it and reject traversal attempts (`..`) to
/// avoid serving files outside `public/`. This implementation trusts
/// the input.
pub fn handle_static(req: &HttpRequest) -> HttpResponse {
    let ext = match req.path.rsplit_once('.') {
        Some((_, e)) => e,
        None => "",
    };

    let mime = get_mime_type(ext);

    let file_path = format!("public{}", req.path);

    let content = match read(file_path) {
        Ok(c) => c,
        Err(_) => return HttpResponse::not_found(),
    };

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), mime.to_string());

    HttpResponse {
        chunked: false,
        version: "HTTP/1.1".to_string(),
        status_code: 200,
        reason: "OK".to_string(),
        headers,
        body: content,
    }
}

/// Demonstrates chunked transfer encoding by sending a fixed string in
/// pieces. Useful as a smoke test for the chunked serializer.
pub fn handle_stream(_req: &HttpRequest) -> HttpResponse {
    let body = "Hello from chunked stream! ".repeat(5);
    HttpResponse::ok_chunked(&body)
}

fn get_mime_type(ext: &str) -> &str {
    match ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        // Catch-all for unknown types. Browsers will treat this as a
        // download rather than risking incorrect rendering.
        _ => "application/octet-stream",
    }
}

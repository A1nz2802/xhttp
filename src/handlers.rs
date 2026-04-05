use std::{collections::HashMap, fs::read_to_string};

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

pub fn handle_static(req: &HttpRequest) -> HttpResponse {
    let ext = match req.path.rsplit_once('.') {
        Some((_, e)) => e,
        None => "",
    };

    let mime = get_mime_type(ext);

    let file_path = format!("public{}", req.path);

    let content = match read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return HttpResponse::not_found(),
    };

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), mime.to_string());

    HttpResponse {
        chunked: false,
        status_code: 200,
        reason: "OK".to_string(),
        headers,
        body: content,
    }
}

pub fn handle_stream(_req: &HttpRequest) -> HttpResponse {
    let body = "Hello from chunked stream! ".repeat(5);
    HttpResponse::ok_chunked(&body)
}

fn get_mime_type(ext: &str) -> &str {
    match ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        _ => "application/octet-stream",
    }
}

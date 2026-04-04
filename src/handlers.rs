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

use crate::{
    handlers::handle_static,
    http::{HttpRequest, HttpResponse},
};
use std::collections::HashMap;

type Handler = Box<dyn Fn(&HttpRequest) -> HttpResponse>;

pub struct Router {
    routes: HashMap<String, Handler>,
}

impl Router {
    pub fn new() -> Router {
        Router {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, path: &str, handler: Handler) {
        self.routes.insert(path.to_string(), handler);
    }

    pub fn handle(&self, request: &HttpRequest) -> HttpResponse {
        match self.routes.get(&request.path) {
            Some(handler) => handler(request),
            None => handle_static(request),
        }
    }
}

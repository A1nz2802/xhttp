//! Path-based dispatch: maps a request path to a handler, with a static
//! file fallback for paths that aren't explicitly registered.

use crate::{
    handlers::handle_static,
    http::{HttpRequest, HttpResponse},
};
use std::collections::HashMap;

/// A request handler stored in the router.
///
/// `Send + Sync` are required because the router lives behind an `Arc`
/// shared across worker threads: `Send` lets the handler reference move
/// between threads, and `Sync` lets multiple workers invoke it
/// simultaneously through `&Router`. Plain functions automatically
/// satisfy both, so handlers like `handle_ping` need no extra work.
type Handler = Box<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

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

    /// Dispatches a request to the registered handler, or to the static
    /// file handler when no route matches. `&self` (shared borrow) is
    /// enough because dispatch only reads the routing table — that's
    /// what makes the router safely shareable across threads after
    /// configuration is done.
    pub fn handle(&self, request: &HttpRequest) -> HttpResponse {
        match self.routes.get(&request.path) {
            Some(handler) => handler(request),
            None => handle_static(request),
        }
    }
}

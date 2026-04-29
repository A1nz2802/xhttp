//! HTTP message types and the wire-format primitives shared between
//! request reading and response writing.

mod method;
mod request;
mod response;

/// Line terminator mandated by the HTTP/1.1 specification. Defining it
/// once prevents subtle bugs from mixing `\n` and `\r\n` in the wire
/// format.
pub const CRLF: &str = "\r\n";

pub use request::HttpRequest;
pub use response::HttpResponse;

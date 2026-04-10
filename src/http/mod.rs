mod method;
mod request;
mod response;

pub const CRLF: &str = "\r\n";

pub use method::HttpMethod;
pub use request::HttpRequest;
pub use response::HttpResponse;

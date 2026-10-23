//! A minimal HTTP/1.1 client: one `GET` per connection, over a unix socket,
//! TCP or TLS.
//!
//! - `url`: `Url`, parsed from the config
//! - `client`: `get_head` over TCP or TLS
//! - `tls`: TLS settings trusting the OS's CA list, fetching a server's
//!   certificate, readable TLS errors
//! - `request`: writes the request
//! - `response`: reads and parses the response

mod client;
mod request;
mod response;
mod tls;
mod url;

pub use client::get_head;
pub use request::send_get;
pub use response::{Head, Response, read_all};
pub use tls::{certificate, describe};
pub use url::{HostPort, Url};

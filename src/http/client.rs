use std::io;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use super::request::{send, send_get};
use super::response::{Head, Response, read_all, read_head};
use super::tls::{connector, server_name};
use super::url::{Scheme, Url};

/// Responses to a request other than `get_head` are only needed for their
/// status and error text.
const MAX_RESPONSE: u64 = 64 << 10;

/// `GET`s `url` and returns the response's status line and headers. HTTPS
/// certificates are verified against the OS's CA list. Redirects aren't
/// followed. Has no timeout of its own.
pub async fn get_head(url: &Url) -> io::Result<Head> {
    let tcp = TcpStream::connect((url.host.as_str(), url.port)).await?;
    match url.scheme {
        Scheme::Http => get_on(tcp, url).await,
        Scheme::Https => {
            let tls = connector().connect(server_name(&url.host)?, tcp).await?;
            get_on(tls, url).await
        }
    }
}

/// `POST`s a JSON `body` to `url`, like `request` otherwise.
pub async fn post_json(url: &Url, headers: &[(&str, &str)], body: &[u8]) -> io::Result<Response> {
    let mut all = vec![("Content-Type", "application/json")];
    all.extend_from_slice(headers);
    request(url, "POST", &all, Some(body)).await
}

/// Sends any request, as `send` describes, and reads the response, like
/// `get_head` otherwise.
pub async fn request(
    url: &Url,
    method: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> io::Result<Response> {
    let tcp = TcpStream::connect((url.host.as_str(), url.port)).await?;
    match url.scheme {
        Scheme::Http => request_on(tcp, url, method, headers, body).await,
        Scheme::Https => {
            let tls = connector().connect(server_name(&url.host)?, tcp).await?;
            request_on(tls, url, method, headers, body).await
        }
    }
}

async fn get_on<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, url: &Url) -> io::Result<Head> {
    send_get(&mut stream, &url.authority(), &url.target).await?;
    read_head(stream).await
}

async fn request_on<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    url: &Url,
    method: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> io::Result<Response> {
    send(
        &mut stream,
        method,
        &url.authority(),
        &url.target,
        headers,
        body,
    )
    .await?;
    read_all(stream, MAX_RESPONSE).await
}

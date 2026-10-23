use std::io;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use super::request::{send_get, send_post};
use super::response::{Head, Response, read_all, read_head};
use super::tls::{connector, server_name};
use super::url::{Scheme, Url};

/// Responses to a `POST` are only needed for their status and error text.
const MAX_POST_RESPONSE: u64 = 64 << 10;

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

/// `POST`s a JSON `body` to `url`, like `get_head` otherwise.
pub async fn post(url: &Url, headers: &[(&str, &str)], body: &[u8]) -> io::Result<Response> {
    let tcp = TcpStream::connect((url.host.as_str(), url.port)).await?;
    match url.scheme {
        Scheme::Http => post_on(tcp, url, headers, body).await,
        Scheme::Https => {
            let tls = connector().connect(server_name(&url.host)?, tcp).await?;
            post_on(tls, url, headers, body).await
        }
    }
}

async fn get_on<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, url: &Url) -> io::Result<Head> {
    send_get(&mut stream, &url.authority(), &url.target).await?;
    read_head(stream).await
}

async fn post_on<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    url: &Url,
    headers: &[(&str, &str)],
    body: &[u8],
) -> io::Result<Response> {
    send_post(&mut stream, &url.authority(), &url.target, headers, body).await?;
    read_all(stream, MAX_POST_RESPONSE).await
}

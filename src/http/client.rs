use std::io;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use super::request::send_get;
use super::response::{Head, read_head};
use super::tls::{connector, server_name};
use super::url::{Scheme, Url};

/// `GET`s `url` and returns the response's status line and headers. HTTPS
/// certificates are verified against the OS's CA list. Redirects aren't
/// followed. Has no timeout of its own.
pub async fn get_head(url: &Url) -> io::Result<Head> {
    let tcp = TcpStream::connect((url.host.as_str(), url.port)).await?;
    match url.scheme {
        Scheme::Http => exchange(tcp, url).await,
        Scheme::Https => {
            let tls = connector().connect(server_name(&url.host)?, tcp).await?;
            exchange(tls, url).await
        }
    }
}

async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, url: &Url) -> io::Result<Head> {
    send_get(&mut stream, &url.authority(), &url.target).await?;
    read_head(stream).await
}

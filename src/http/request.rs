use std::io;

use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Headers the request writer always sets itself.
pub const RESERVED_HEADERS: [&str; 4] =
    ["host", "content-length", "connection", "transfer-encoding"];

/// Sends a `GET` with `Connection: close`, so the response ends when the
/// server closes the connection.
pub async fn send_get<S: AsyncWrite + Unpin>(
    stream: &mut S,
    authority: &str,
    target: &str,
) -> io::Result<()> {
    send(stream, "GET", authority, target, &[], None).await
}

/// Sends a request with `Connection: close` like `send_get`. `headers` are
/// `(name, value)` pairs added as they are, and replace the default
/// `User-Agent` and `Accept`; they mustn't be one of `RESERVED_HEADERS`.
pub async fn send<S: AsyncWrite + Unpin>(
    stream: &mut S,
    method: &str,
    authority: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> io::Result<()> {
    let has = |name: &str| headers.iter().any(|(n, _)| n.eq_ignore_ascii_case(name));
    let mut request = format!("{method} {target} HTTP/1.1\r\nHost: {authority}\r\n");
    if !has("user-agent") {
        request.push_str(&format!(
            "User-Agent: zuschauer/{}\r\n",
            env!("CARGO_PKG_VERSION")
        ));
    }
    if !has("accept") {
        request.push_str("Accept: */*\r\n");
    }
    if let Some(body) = body {
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("Connection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;
    if let Some(body) = body {
        stream.write_all(body).await?;
    }
    stream.flush().await
}

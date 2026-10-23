use std::io;

use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Sends a `GET` with `Connection: close`, so the response ends when the
/// server closes the connection.
pub async fn send_get<S: AsyncWrite + Unpin>(
    stream: &mut S,
    authority: &str,
    target: &str,
) -> io::Result<()> {
    let request = format!(
        "GET {target} HTTP/1.1\r\n\
         Host: {authority}\r\n\
         User-Agent: zuschauer/{}\r\n\
         Accept: */*\r\n\
         Connection: close\r\n\r\n",
        env!("CARGO_PKG_VERSION"),
    );
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await
}

/// Sends a `POST` of a JSON `body`, with `Connection: close` like `send_get`.
/// `headers` are `(name, value)` pairs added as they are.
pub async fn send_post<S: AsyncWrite + Unpin>(
    stream: &mut S,
    authority: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> io::Result<()> {
    let mut request = format!(
        "POST {target} HTTP/1.1\r\n\
         Host: {authority}\r\n\
         User-Agent: zuschauer/{}\r\n\
         Accept: */*\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n",
        env!("CARGO_PKG_VERSION"),
        body.len(),
    );
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

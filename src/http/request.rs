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

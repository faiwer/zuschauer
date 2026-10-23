use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};

/// Responses with headers bigger than this are rejected.
const MAX_HEAD: usize = 64 << 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub status: u16,
    pub content_type: Option<String>,
    chunked: bool,
}

impl Head {
    #[cfg(test)]
    pub fn new(status: u16, content_type: Option<String>) -> Self {
        Self {
            status,
            content_type,
            chunked: false,
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub head: Head,
    pub body: Vec<u8>,
}

/// Reads up to `max` bytes until the server closes the connection.
pub async fn read_all<S: AsyncRead + Unpin>(stream: S, max: u64) -> io::Result<Response> {
    let mut raw = Vec::new();
    stream.take(max).read_to_end(&mut raw).await?;
    parse(&raw)
}

/// Reads only the status line and headers; the body is never downloaded.
pub async fn read_head<S: AsyncRead + Unpin>(mut stream: S) -> io::Result<Head> {
    let mut raw = Vec::with_capacity(4096);
    let mut chunk = [0; 4096];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Err(invalid("connection closed before the headers ended"));
        }
        // The terminator can straddle two reads.
        let search_from = raw.len().saturating_sub(3);
        raw.extend_from_slice(&chunk[..n]);
        if raw[search_from..].windows(4).any(|w| w == b"\r\n\r\n") {
            return parse_head(&raw).map(|(head, _)| head);
        }
        if raw.len() > MAX_HEAD {
            return Err(invalid("headers too large"));
        }
    }
}

fn parse(raw: &[u8]) -> io::Result<Response> {
    let (head, rest) = parse_head(raw)?;
    let body = if head.chunked {
        dechunk(rest)?
    } else {
        rest.to_vec()
    };
    Ok(Response { head, body })
}

/// The head and whatever follows it.
fn parse_head(raw: &[u8]) -> io::Result<(Head, &[u8])> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| invalid("malformed HTTP response"))?;
    let text = std::str::from_utf8(&raw[..split]).map_err(|_| invalid("malformed HTTP headers"))?;
    let mut lines = text.split("\r\n");
    let status = lines
        .next()
        .filter(|line| line.starts_with("HTTP/"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| invalid("malformed HTTP status line"))?;
    let mut head = Head {
        status,
        content_type: None,
        chunked: false,
    };
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-type") {
            head.content_type = Some(value.to_owned());
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            head.chunked = value.eq_ignore_ascii_case("chunked");
        }
    }
    Ok((head, &raw[split + 4..]))
}

/// Decodes `Transfer-Encoding: chunked`: `<hex size>\r\n<data>\r\n` repeated,
/// ending with a zero-size chunk.
fn dechunk(mut raw: &[u8]) -> io::Result<Vec<u8>> {
    let mut body = Vec::with_capacity(raw.len());
    loop {
        let line_end = raw
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| invalid("truncated chunked body"))?;
        let size_line =
            std::str::from_utf8(&raw[..line_end]).map_err(|_| invalid("bad chunk size"))?;
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16).map_err(|_| invalid("bad chunk size"))?;
        if size == 0 {
            return Ok(body);
        }
        let data = &raw[line_end + 2..];
        if data.len() < size {
            return Err(invalid("truncated chunked body"));
        }
        body.extend_from_slice(&data[..size]);
        raw = data[size..]
            .strip_prefix(b"\r\n")
            .ok_or_else(|| invalid("bad chunk terminator"))?;
    }
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_response() {
        let r =
            parse(b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}").unwrap();
        assert_eq!(r.head.status, 200);
        assert_eq!(r.head.content_type.as_deref(), Some("application/json"));
        assert_eq!(r.body, b"{\"a\":1}");
    }

    #[test]
    fn decodes_chunked_body() {
        let r = parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n")
            .unwrap();
        assert_eq!(r.body, b"{\"a\":1}");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"garbage").is_err());
        assert!(parse(b"SSH-2.0-OpenSSH\r\n\r\n").is_err());
        assert!(
            parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n10\r\nshort").is_err()
        );
    }

    #[tokio::test]
    async fn reads_head_split_across_reads() {
        let (mut tx, rx) = tokio::io::duplex(8);
        let writer = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            // Fails once the reader has the headers and hangs up, which is the point.
            let _ = tx
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/html\r\n\r\n<html>")
                .await;
        });
        let head = read_head(rx).await.unwrap();
        writer.await.unwrap();
        assert_eq!(head.status, 502);
        assert_eq!(head.content_type.as_deref(), Some("text/html"));
    }
}

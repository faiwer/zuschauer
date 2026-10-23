//! Minimal client for the Docker Engine API over a unix socket. Podman serves
//! the same API. Every request sends `Connection: close`, so the response
//! ends when the server closes the socket. Bodies may still arrive chunked
//! (OrbStack does that even for HTTP/1.0 requests).

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::de::DeserializeOwned;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE: u64 = 8 << 20;

#[derive(Debug)]
pub struct Api {
    socket: PathBuf,
}

impl Api {
    pub fn new(socket: PathBuf) -> Self {
        Self { socket }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> io::Result<T> {
        let response = timeout(TIMEOUT, self.request(path)).await.map_err(|_| {
            io::Error::new(io::ErrorKind::TimedOut, format!("GET {path} timed out"))
        })??;
        let body =
            body(&response).map_err(|e| io::Error::new(e.kind(), format!("GET {path}: {e}")))?;
        serde_json::from_slice(&body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("GET {path}: {e}")))
    }

    async fn request(&self, path: &str) -> io::Result<Vec<u8>> {
        let mut stream = UnixStream::connect(&self.socket).await?;
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await?;
        let mut response = Vec::new();
        stream.take(MAX_RESPONSE).read_to_end(&mut response).await?;
        Ok(response)
    }
}

/// The configured socket, then `$DOCKER_HOST`, then the usual locations of
/// Docker, Podman (rootful and rootless), OrbStack and Docker Desktop.
pub fn find_socket(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured {
        return Some(path.to_owned());
    }

    if let Ok(host) = std::env::var("DOCKER_HOST")
        && let Some(path) = host.strip_prefix("unix://")
    {
        return Some(PathBuf::from(path));
    }

    let mut candidates = vec![
        PathBuf::from("/var/run/docker.sock"),
        PathBuf::from("/run/podman/podman.sock"),
    ];
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        candidates.push(Path::new(&runtime).join("podman/podman.sock"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = Path::new(&home);
        candidates.push(home.join(".orbstack/run/docker.sock"));
        candidates.push(home.join(".docker/run/docker.sock"));
    }
    // `exists` follows symlinks, so a dangling `/var/run/docker.sock` left
    // behind by an uninstalled Docker Desktop is skipped.
    candidates.into_iter().find(|path| path.exists())
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

fn body(response: &[u8]) -> io::Result<Vec<u8>> {
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| invalid("malformed HTTP response"))?;
    let head =
        std::str::from_utf8(&response[..split]).map_err(|_| invalid("malformed HTTP headers"))?;
    let mut lines = head.split("\r\n");
    let code = lines
        .next()
        .and_then(|status| status.split_whitespace().nth(1))
        .unwrap_or("?");
    let chunked = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && value.trim().eq_ignore_ascii_case("chunked")
        })
    });
    let raw = &response[split + 4..];
    let body = if chunked { dechunk(raw)? } else { raw.to_vec() };
    if code != "200" {
        let snippet = String::from_utf8_lossy(&body[..body.len().min(200)]);
        return Err(io::Error::other(format!("HTTP {code}: {}", snippet.trim())));
    }
    Ok(body)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_body() {
        let response = b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
        assert_eq!(body(response).unwrap(), b"{\"a\":1}");
    }

    #[test]
    fn reports_http_errors() {
        let response = b"HTTP/1.0 404 Not Found\r\n\r\n{\"message\":\"No such container\"}";
        let error = body(response).unwrap_err().to_string();
        assert!(
            error.contains("404") && error.contains("No such container"),
            "{error}"
        );
    }

    #[test]
    fn decodes_chunked_body() {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n";
        assert_eq!(body(response).unwrap(), b"{\"a\":1}");
    }

    #[test]
    fn rejects_garbage() {
        assert!(body(b"garbage").is_err());
        assert!(body(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n10\r\nshort").is_err());
    }
}

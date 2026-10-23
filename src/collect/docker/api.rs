//! The Docker Engine API over a unix socket. Podman serves the same API.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::de::DeserializeOwned;
use tokio::net::UnixStream;
use tokio::time::timeout;

use crate::http::{Response, read_all, send_get};

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
        let with_path = |e: io::Error| io::Error::new(e.kind(), format!("GET {path}: {e}"));
        let response = timeout(TIMEOUT, self.request(path))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "timed out"))
            .flatten()
            .map_err(with_path)?;
        if response.head.status != 200 {
            let snippet = String::from_utf8_lossy(&response.body[..response.body.len().min(200)]);
            return Err(with_path(io::Error::other(format!(
                "HTTP {}: {}",
                response.head.status,
                snippet.trim()
            ))));
        }
        serde_json::from_slice(&response.body)
            .map_err(|e| with_path(io::Error::new(io::ErrorKind::InvalidData, e)))
    }

    async fn request(&self, path: &str) -> io::Result<Response> {
        let mut stream = UnixStream::connect(&self.socket).await?;
        send_get(&mut stream, "localhost", path).await?;
        read_all(stream, MAX_RESPONSE).await
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

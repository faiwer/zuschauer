use std::io;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::crypto::ring;
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::{CertificateError, ClientConfig, Error as TlsError, RootCertStore};

use super::url::HostPort;

/// The OS updates its CA list on its own (e.g. `dnf update` on Fedora), but
/// a long-running process only sees the copy it loaded.
const RELOAD_EVERY: Duration = Duration::from_secs(24 * 3600);

static CACHE: Mutex<Option<(Instant, TlsConnector)>> = Mutex::new(None);

/// Trusts the OS's CA list. Loaded on the first TLS connection, not at start.
pub fn connector() -> TlsConnector {
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    match &*cache {
        Some((loaded, connector)) if loaded.elapsed() < RELOAD_EVERY => connector.clone(),
        _ => {
            let connector = load();
            *cache = Some((Instant::now(), connector.clone()));
            connector
        }
    }
}

pub fn server_name(host: &str) -> io::Result<ServerName<'static>> {
    ServerName::try_from(host.to_owned())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

/// Connects, verifies the certificate like a browser would, and returns it.
/// Has no timeout of its own.
pub async fn certificate(target: &HostPort) -> io::Result<CertificateDer<'static>> {
    let tcp = TcpStream::connect((target.host.as_str(), target.port)).await?;
    let tls = connector().connect(server_name(&target.host)?, tcp).await?;
    let (_, connection) = tls.get_ref();
    connection
        .peer_certificates()
        .and_then(|chain| chain.first())
        .map(|leaf| leaf.clone().into_owned())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "server sent no certificate"))
}

/// A short message for the page. TLS errors are reworded; rustls's own text
/// includes raw Unix timestamps and debug output.
pub fn describe(error: &io::Error) -> String {
    let Some(TlsError::InvalidCertificate(cert)) = error.get_ref().and_then(|e| e.downcast_ref())
    else {
        return error.to_string();
    };
    match cert {
        CertificateError::Expired => "certificate expired".to_owned(),
        CertificateError::ExpiredContext { time, not_after } => {
            format!("certificate expired {} ago", days(*time, *not_after))
        }
        CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
            "certificate not valid yet".to_owned()
        }
        CertificateError::UnknownIssuer => "certificate not trusted (unknown issuer)".to_owned(),
        CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
            "certificate is for a different name".to_owned()
        }
        CertificateError::Revoked => "certificate revoked".to_owned(),
        other => format!("bad certificate: {other:?}"),
    }
}

/// `later - earlier`, rounded down to whole days, at least `0d`.
fn days(later: UnixTime, earlier: UnixTime) -> String {
    let seconds = later.as_secs().saturating_sub(earlier.as_secs());
    format!("{}d", seconds / 86400)
}

fn load() -> TlsConnector {
    let found = rustls_native_certs::load_native_certs();
    for e in &found.errors {
        eprintln!("tls: loading OS certificates: {e}");
    }
    let mut roots = RootCertStore::empty();
    let (added, _) = roots.add_parsable_certificates(found.certs);
    eprintln!("tls: {added} trusted root certificates");
    // `ring` was picked at build time, so the only failure here, an
    // unsupported protocol version, can't happen.
    let config = ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("ring supports the default TLS versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

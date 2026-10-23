use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::crypto::ring;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

/// The OS updates its CA list on its own (e.g. `dnf update` on Fedora), but
/// a long-running process only sees the copy it loaded.
const RELOAD_EVERY: Duration = Duration::from_secs(24 * 3600);

static CACHE: Mutex<Option<(Instant, TlsConnector)>> = Mutex::new(None);

/// Trusts the OS's CA list. Loaded on the first HTTPS request, not at start.
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

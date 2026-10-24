use maud::{Markup, html};

use crate::config::{App, AppCertificate, CertificatesConfig};
use crate::model::{EntityId, Status, Timestamp, now};
use crate::store::{Entity, Store};

use super::super::format::{ago, until};

pub struct CertificateRow {
    host: String,
    status: Option<Status>,
    expires_at: Option<Timestamp>,
    /// Expires within `certificates.warnBefore`.
    expiring: bool,
    error: Option<String>,
}

pub fn certificate_row(
    store: &Store,
    app: &App,
    watched: &AppCertificate,
    config: &CertificatesConfig,
) -> CertificateRow {
    let host = watched.target.to_string();
    let entity = store.entity(&EntityId::Certificate {
        app: app.id.clone(),
        host: host.clone(),
    });
    let expires_at = entity.and_then(Entity::expires_at);
    let warn_before = Timestamp::try_from(config.warn_before.as_secs()).unwrap_or(Timestamp::MAX);
    CertificateRow {
        host,
        status: entity.and_then(Entity::status),
        expires_at,
        expiring: expires_at.is_some_and(|at| at < now().saturating_add(warn_before)),
        error: entity.and_then(Entity::error).map(str::to_owned),
    }
}

pub fn render_certificate_row(row: &CertificateRow) -> Markup {
    html! {
        tr {
            th { (row.host) }
            @match (row.status, row.expires_at) {
                (Some(Status::Up { .. }), Some(at)) if row.expiring => {
                    td { strong { "expiring" } }
                    td { "in " (until(at)) }
                }
                (Some(Status::Up { .. }), Some(at)) => {
                    td { "valid" }
                    td { "expires in " (until(at)) }
                }
                (Some(Status::Down { since }), _) => {
                    td { "failing " (ago(since)) }
                    td { "—" }
                }
                _ => {
                    td { "pending" }
                    td { "—" }
                }
            }
            td { (row.error.as_deref().unwrap_or_default()) }
        }
    }
}

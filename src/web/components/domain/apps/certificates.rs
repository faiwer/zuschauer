use maud::{Markup, html};

use crate::config::{App, AppCertificate, CertificatesConfig};
use crate::model::{EntityId, Status, Timestamp, now};
use crate::store::{Entity, Store};

use super::super::super::ui::{Icon, Level, ago, date, tinted_icon, until};

pub struct CertificateRow {
    pub(super) host: String,
    pub(super) status: Option<Status>,
    pub(super) expires_at: Option<Timestamp>,
    /// Expires within `certificates.warnBefore`.
    pub(super) expiring: bool,
    pub(super) error: Option<String>,
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

impl CertificateRow {
    /// Like the apps table's certificate icon: a check that fails is a
    /// warning unless the certificate last seen has expired.
    fn level(&self) -> Level {
        match (self.status, self.expires_at) {
            (_, Some(at)) if at <= now() => Level::Problem,
            (Some(Status::Down { .. }), _) => Level::Warning,
            (_, Some(_)) if self.expiring => Level::Warning,
            (_, Some(_)) => Level::Ok,
            _ => Level::Unknown,
        }
    }
}

/// The status icon and host, then the expiry date, or the error while the
/// check fails.
pub fn render_certificate_item(row: &CertificateRow) -> Markup {
    let level = row.level();
    html! {
        li {
            (tinted_icon(Icon::Cert, level.class(), None))
            " "
            span.highlight.(level.text_class()) { (row.host) }
            " — "
            @match (row.status, row.expires_at, &row.error) {
                (Some(Status::Down { .. }), _, Some(error)) => (error),
                (Some(Status::Up { .. }), Some(at), _) => { "expires at " (date(at)) }
                _ => "no result yet",
            }
        }
    }
}

pub fn render_certificate_row(row: &CertificateRow) -> Markup {
    html! {
        tr.table-row {
            td.highlight { (row.host) }
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

use crate::config::Severity;
use crate::model::Timestamp;

#[derive(Debug, Clone)]
pub struct Message {
    /// The alert's ID.
    pub alert: String,
    /// The alert's label, the same while firing and resolved.
    pub label: String,
    pub title: String,
    pub body: String,
    /// The problem, without how long it lasted.
    pub detail: String,
    /// While firing: what it fires as. Resolved: the loudest announced.
    pub severity: Severity,
    /// When the problem started.
    pub since: Timestamp,
    /// When it resolved; `None` while firing.
    pub ended: Option<Timestamp>,
}

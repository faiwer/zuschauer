use crate::config::Severity;

#[derive(Debug, Clone)]
pub struct Message {
    /// The alert's ID.
    pub alert: String,
    pub title: String,
    pub body: String,
    /// The firing alert's severity; `None` for a resolved alert, which is
    /// announced quietly.
    pub severity: Option<Severity>,
}

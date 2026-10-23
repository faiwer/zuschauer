use crate::config::Severity;

#[derive(Debug, Clone)]
pub struct Message {
    pub title: String,
    pub body: String,
    /// The firing alert's severity; `None` for a resolved alert, which is
    /// announced quietly.
    pub severity: Option<Severity>,
}

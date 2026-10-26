use std::collections::BTreeMap;

use crate::model::{MetricKind, Sample, Status, Timestamp};

use super::series::Series;

#[derive(Debug, Default)]
pub struct Entity {
    series: BTreeMap<MetricKind, Series>,
    /// Only for entities that can come and go, or fail.
    status: Option<Status>,
    /// Why the last attempt failed, for entities that can fail.
    error: Option<String>,
    /// Certificates: when the last one seen expires.
    expires_at: Option<Timestamp>,
    /// Commands: when the run in progress started.
    running_since: Option<Timestamp>,
    /// Commands: what the last successful run printed.
    message: Option<String>,
}

impl Entity {
    pub fn series(&self, metric: MetricKind) -> Option<&Series> {
        self.series.get(&metric)
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn expires_at(&self) -> Option<Timestamp> {
        self.expires_at
    }

    pub fn running_since(&self) -> Option<Timestamp> {
        self.running_since
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub(super) fn push(&mut self, metric: MetricKind, sample: Sample) {
        self.series.entry(metric).or_default().push(sample);
    }

    pub(super) fn set_status(&mut self, status: Status) {
        self.status = Some(status);
    }

    pub(super) fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub(super) fn set_expires_at(&mut self, at: Timestamp) {
        self.expires_at = Some(at);
    }

    pub(super) fn set_running_since(&mut self, at: Option<Timestamp>) {
        self.running_since = at;
    }

    pub(super) fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }
}

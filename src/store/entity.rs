use std::collections::BTreeMap;

use crate::model::{MetricKind, Sample, Status};

use super::series::Series;

#[derive(Debug, Default)]
pub struct Entity {
    series: BTreeMap<MetricKind, Series>,
    /// Only for entities that can come and go, or fail.
    status: Option<Status>,
    /// Why the last attempt failed, for entities that can fail.
    error: Option<String>,
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

    pub(super) fn push(&mut self, metric: MetricKind, sample: Sample) {
        self.series.entry(metric).or_default().push(sample);
    }

    pub(super) fn set_status(&mut self, status: Status) {
        self.status = Some(status);
    }

    pub(super) fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }
}

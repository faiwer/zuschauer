use std::collections::BTreeMap;

use crate::model::{MetricKind, Sample, Status};

use super::series::Series;

#[derive(Debug, Default)]
pub struct Entity {
    series: BTreeMap<MetricKind, Series>,
    /// Only for entities that can come and go, like containers.
    status: Option<Status>,
}

impl Entity {
    pub fn series(&self, metric: MetricKind) -> Option<&Series> {
        self.series.get(&metric)
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }

    pub(super) fn push(&mut self, metric: MetricKind, sample: Sample) {
        self.series.entry(metric).or_default().push(sample);
    }

    pub(super) fn set_status(&mut self, status: Status) {
        self.status = Some(status);
    }
}

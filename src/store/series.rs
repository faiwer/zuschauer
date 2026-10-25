use crate::model::Sample;

use super::ring::RingBuffer;

/// Points kept per series. The time span covered is `HISTORY_POINTS * every`.
pub const HISTORY_POINTS: usize = 512;

#[derive(Debug)]
pub struct Series {
    points: RingBuffer<Sample>,
}

impl Default for Series {
    fn default() -> Self {
        Self {
            points: RingBuffer::new(HISTORY_POINTS),
        }
    }
}

impl Series {
    pub fn push(&mut self, sample: Sample) {
        self.points.push(sample);
    }

    pub fn latest(&self) -> Option<Sample> {
        self.points.iter().next_back().copied()
    }

    /// Oldest first.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Sample> {
        self.points.iter()
    }
}

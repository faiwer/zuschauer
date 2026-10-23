/// In bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Usage {
    pub used: u64,
    pub total: u64,
}

impl Usage {
    pub fn percent(self) -> Option<f32> {
        if self.total == 0 || self.used > self.total {
            return None;
        }
        Some(self.used as f32 / self.total as f32 * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_percent() {
        assert_eq!(Usage { used: 1, total: 4 }.percent(), Some(25.0));
        assert_eq!(Usage { used: 0, total: 0 }.percent(), None);
        assert_eq!(Usage { used: 5, total: 4 }.percent(), None);
    }
}

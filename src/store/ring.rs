use std::collections::VecDeque;

/// Fixed-capacity buffer: once full, every push drops the oldest item.
#[derive(Debug)]
pub struct RingBuffer<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring buffer capacity must be positive");
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    /// Oldest first.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_items_until_full() {
        let mut ring = RingBuffer::new(3);
        ring.push(1);
        ring.push(2);
        assert_eq!(ring.iter().copied().collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn drops_oldest_when_full() {
        let mut ring = RingBuffer::new(3);
        for i in 1..=5 {
            ring.push(i);
        }
        assert_eq!(ring.iter().copied().collect::<Vec<_>>(), [3, 4, 5]);
    }
}

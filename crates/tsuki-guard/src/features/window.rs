use std::collections::VecDeque;
use tsuki_common::telemetry::TelemetrySample;

/// Circular sliding window buffer for telemetry samples.
/// Guarantees zero heap reallocations after initial construction (PERF-02).
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    capacity: usize,
    buffer: VecDeque<TelemetrySample>,
}

impl SlidingWindow {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    /// Push a new telemetry sample into the window.
    /// If full, the oldest sample is evicted.
    pub fn push(&mut self, sample: TelemetrySample) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(sample);
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    pub fn latest(&self) -> Option<&TelemetrySample> {
        self.buffer.back()
    }

    pub fn oldest(&self) -> Option<&TelemetrySample> {
        self.buffer.front()
    }

    /// Get all samples in chronological order.
    pub fn samples(&self) -> Vec<TelemetrySample> {
        self.buffer.iter().cloned().collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &TelemetrySample> {
        self.buffer.iter()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsuki_common::telemetry::PressureRecord;

    fn make_sample(ts: u64) -> TelemetrySample {
        TelemetrySample {
            timestamp_ms: ts,
            pressure: PressureRecord::default(),
            vmstat: Default::default(),
            cgroups: vec![],
        }
    }

    #[test]
    fn test_sliding_window_capacity() {
        let mut window = SlidingWindow::new(3);
        assert_eq!(window.len(), 0);

        window.push(make_sample(100));
        window.push(make_sample(200));
        assert_eq!(window.len(), 2);
        assert!(!window.is_full());

        window.push(make_sample(300));
        assert_eq!(window.len(), 3);
        assert!(window.is_full());
        assert_eq!(window.oldest().unwrap().timestamp_ms, 100);
        assert_eq!(window.latest().unwrap().timestamp_ms, 300);

        // Eviction
        window.push(make_sample(400));
        assert_eq!(window.len(), 3);
        assert_eq!(window.oldest().unwrap().timestamp_ms, 200);
        assert_eq!(window.latest().unwrap().timestamp_ms, 400);
    }
}

//! Event Collector
//!
//! Ring buffer collector for high-throughput event collection.

#![allow(dead_code)]

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::events::RawEvent;
use crate::types::Timestamp;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for event collector
#[derive(Debug, Clone)]
pub struct EventCollectorConfig {
    /// Ring buffer size
    pub buffer_size: usize,
    /// Maximum events per tick
    pub max_events_per_tick: usize,
    /// Enable rate limiting
    pub rate_limiting: bool,
    /// Rate limit (events per second)
    pub rate_limit: u64,
    /// Batch size for processing
    pub batch_size: usize,
}

impl Default for EventCollectorConfig {
    fn default() -> Self {
        Self {
            buffer_size: 65536,
            max_events_per_tick: 10000,
            rate_limiting: true,
            rate_limit: 100000,
            batch_size: 256,
        }
    }
}

impl EventCollectorConfig {
    /// Create small config (for testing)
    #[inline]
    pub fn small() -> Self {
        Self {
            buffer_size: 1024,
            max_events_per_tick: 100,
            rate_limiting: false,
            rate_limit: 10000,
            batch_size: 32,
        }
    }

    /// Create large config (for production)
    #[inline]
    pub fn large() -> Self {
        Self {
            buffer_size: 262144,
            max_events_per_tick: 50000,
            rate_limiting: true,
            rate_limit: 500000,
            batch_size: 1024,
        }
    }
}

// ============================================================================
// EVENT COLLECTOR
// ============================================================================

/// Event collector - aggregates events from all probes
pub struct EventCollector {
    /// Configuration
    config: EventCollectorConfig,
    /// Event buffer
    buffer: Vec<RawEvent>,
    /// Next write position
    write_pos: usize,
    /// Next read position
    read_pos: usize,
    /// Events in buffer
    count: usize,
    /// Total events received
    total_received: AtomicU64,
    /// Total events dropped
    total_dropped: AtomicU64,
    /// Current rate (events per second)
    current_rate: AtomicU64,
    /// Last rate calculation timestamp
    last_rate_time: Timestamp,
    /// Events since last rate calculation
    events_since_rate: u64,
}

impl EventCollector {
    /// Create new collector
    pub fn new(config: EventCollectorConfig) -> Self {
        let capacity = config.buffer_size;
        Self {
            config,
            buffer: Vec::with_capacity(capacity),
            write_pos: 0,
            read_pos: 0,
            count: 0,
            total_received: AtomicU64::new(0),
            total_dropped: AtomicU64::new(0),
            current_rate: AtomicU64::new(0),
            last_rate_time: Timestamp::now(),
            events_since_rate: 0,
        }
    }

    /// Collect event from probe
    pub fn collect(&mut self, event: RawEvent) -> bool {
        self.total_received.fetch_add(1, Ordering::Relaxed);

        // Rate limiting check
        if self.config.rate_limiting {
            let rate = self.current_rate.load(Ordering::Relaxed);
            if rate > self.config.rate_limit {
                self.total_dropped.fetch_add(1, Ordering::Relaxed);
                return false;
            }
        }

        // Buffer full check
        if self.count >= self.config.buffer_size {
            self.total_dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Add to buffer
        if self.buffer.len() < self.config.buffer_size {
            self.buffer.push(event);
        } else {
            self.buffer[self.write_pos] = event;
        }
        self.write_pos = (self.write_pos + 1) % self.config.buffer_size;
        self.count += 1;
        self.events_since_rate += 1;

        true
    }

    /// Drain events up to max count
    pub fn drain(&mut self, max: usize) -> Vec<RawEvent> {
        let take = core::cmp::min(max, self.count);
        let mut result = Vec::with_capacity(take);

        for _ in 0..take {
            if self.count > 0 {
                result.push(self.buffer[self.read_pos].clone());
                self.read_pos = (self.read_pos + 1) % self.config.buffer_size;
                self.count -= 1;
            }
        }

        result
    }

    /// Get batch of events
    #[inline(always)]
    pub fn get_batch(&mut self) -> Vec<RawEvent> {
        self.drain(self.config.batch_size)
    }

    /// Update rate calculation
    #[inline]
    pub fn update_rate(&mut self, now: Timestamp) {
        let elapsed = now.elapsed_since(self.last_rate_time);
        if elapsed.as_millis() >= 1000 {
            let rate = self.events_since_rate;
            self.current_rate.store(rate, Ordering::Relaxed);
            self.events_since_rate = 0;
            self.last_rate_time = now;
        }
    }

    /// Get current fill level (0.0 to 1.0)
    #[inline]
    pub fn fill_level(&self) -> f32 {
        if self.config.buffer_size == 0 {
            0.0
        } else {
            self.count as f32 / self.config.buffer_size as f32
        }
    }

    /// Is buffer full?
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.count >= self.config.buffer_size
    }

    /// Is buffer empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get statistics
    #[inline]
    pub fn stats(&self) -> CollectorStats {
        CollectorStats {
            total_received: self.total_received.load(Ordering::Relaxed),
            total_dropped: self.total_dropped.load(Ordering::Relaxed),
            buffer_used: self.count,
            buffer_capacity: self.config.buffer_size,
            current_rate: self.current_rate.load(Ordering::Relaxed),
        }
    }

    /// Reset statistics
    #[inline]
    pub fn reset_stats(&self) {
        self.total_received.store(0, Ordering::Relaxed);
        self.total_dropped.store(0, Ordering::Relaxed);
        self.current_rate.store(0, Ordering::Relaxed);
    }

    /// Clear buffer
    #[inline]
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_pos = 0;
        self.read_pos = 0;
        self.count = 0;
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self::new(EventCollectorConfig::default())
    }
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Collector statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CollectorStats {
    /// Total events received
    pub total_received: u64,
    /// Total events dropped
    pub total_dropped: u64,
    /// Buffer slots used
    pub buffer_used: usize,
    /// Buffer capacity
    pub buffer_capacity: usize,
    /// Current rate (events/second)
    pub current_rate: u64,
}

impl CollectorStats {
    /// Get drop rate
    #[inline]
    pub fn drop_rate(&self) -> f64 {
        if self.total_received == 0 {
            0.0
        } else {
            self.total_dropped as f64 / self.total_received as f64
        }
    }

    /// Get buffer utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.buffer_capacity == 0 {
            0.0
        } else {
            self.buffer_used as f64 / self.buffer_capacity as f64
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::events::EventData;
    use super::super::probe::ProbeType;
    use super::*;
    use crate::types::ProbeId;

    fn make_event() -> RawEvent {
        RawEvent::new(
            ProbeId::generate(),
            ProbeType::Cpu,
            EventData::Raw(vec![1, 2, 3]),
        )
    }

    #[test]
    fn test_collector_collect() {
        let config = EventCollectorConfig {
            buffer_size: 100,
            rate_limiting: false,
            ..Default::default()
        };
        let mut collector = EventCollector::new(config);

        assert!(collector.collect(make_event()));
        assert_eq!(collector.stats().total_received, 1);
        assert_eq!(collector.count, 1);
    }

    #[test]
    fn test_collector_drain() {
        let config = EventCollectorConfig {
            buffer_size: 100,
            rate_limiting: false,
            ..Default::default()
        };
        let mut collector = EventCollector::new(config);

        for _ in 0..10 {
            collector.collect(make_event());
        }

        let events = collector.drain(5);
        assert_eq!(events.len(), 5);
        assert_eq!(collector.count, 5);
    }

    #[test]
    fn test_collector_full() {
        let config = EventCollectorConfig {
            buffer_size: 5,
            rate_limiting: false,
            ..Default::default()
        };
        let mut collector = EventCollector::new(config);

        for _ in 0..5 {
            assert!(collector.collect(make_event()));
        }

        // Should be dropped
        assert!(!collector.collect(make_event()));
        assert_eq!(collector.stats().total_dropped, 1);
    }
}

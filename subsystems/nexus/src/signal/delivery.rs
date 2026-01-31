//! Signal Delivery Optimizer
//!
//! Optimizes signal delivery to threads based on affinity and availability.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{SignalNumber, ThreadId};

/// Delivery recommendation
#[derive(Debug, Clone)]
pub struct DeliveryRecommendation {
    /// Target thread (None = any)
    pub target_thread: Option<ThreadId>,
    /// Priority
    pub priority: u8,
    /// Should batch with other signals
    pub batch: bool,
    /// Delay before delivery (nanoseconds)
    pub delay_ns: u64,
    /// Reason
    pub reason: String,
}

/// Delivery optimizer
pub struct DeliveryOptimizer {
    /// Per-thread signal affinity (which threads prefer which signals)
    thread_affinity: BTreeMap<ThreadId, Vec<SignalNumber>>,
    /// Thread availability (currently not in critical section)
    thread_available: BTreeMap<ThreadId, bool>,
    /// Delivery latency per thread
    thread_latency: BTreeMap<ThreadId, u64>,
    /// Enable batching
    batching_enabled: bool,
    /// Batch window (nanoseconds)
    batch_window_ns: u64,
}

impl DeliveryOptimizer {
    /// Create new delivery optimizer
    pub fn new() -> Self {
        Self {
            thread_affinity: BTreeMap::new(),
            thread_available: BTreeMap::new(),
            thread_latency: BTreeMap::new(),
            batching_enabled: false,
            batch_window_ns: 1_000_000, // 1ms
        }
    }

    /// Register thread
    pub fn register_thread(&mut self, tid: ThreadId) {
        self.thread_affinity.insert(tid, Vec::new());
        self.thread_available.insert(tid, true);
        self.thread_latency.insert(tid, 0);
    }

    /// Unregister thread
    pub fn unregister_thread(&mut self, tid: ThreadId) {
        self.thread_affinity.remove(&tid);
        self.thread_available.remove(&tid);
        self.thread_latency.remove(&tid);
    }

    /// Set thread affinity for signal
    pub fn set_affinity(&mut self, tid: ThreadId, signo: SignalNumber) {
        self.thread_affinity.entry(tid).or_default().push(signo);
    }

    /// Clear thread affinity
    pub fn clear_affinity(&mut self, tid: ThreadId) {
        if let Some(affinities) = self.thread_affinity.get_mut(&tid) {
            affinities.clear();
        }
    }

    /// Update thread availability
    pub fn set_available(&mut self, tid: ThreadId, available: bool) {
        self.thread_available.insert(tid, available);
    }

    /// Check if thread is available
    pub fn is_available(&self, tid: ThreadId) -> bool {
        self.thread_available.get(&tid).copied().unwrap_or(true)
    }

    /// Record delivery latency
    pub fn record_latency(&mut self, tid: ThreadId, latency_ns: u64) {
        if let Some(avg) = self.thread_latency.get_mut(&tid) {
            // Exponential moving average
            *avg = (*avg * 9 + latency_ns) / 10;
        }
    }

    /// Get thread latency
    pub fn get_latency(&self, tid: ThreadId) -> u64 {
        self.thread_latency.get(&tid).copied().unwrap_or(0)
    }

    /// Find best thread for signal delivery
    pub fn optimize_delivery(
        &self,
        signo: SignalNumber,
        threads: &[ThreadId],
    ) -> DeliveryRecommendation {
        if threads.is_empty() {
            return DeliveryRecommendation {
                target_thread: None,
                priority: 5,
                batch: false,
                delay_ns: 0,
                reason: String::from("No threads available"),
            };
        }

        // Find thread with affinity
        for tid in threads {
            if let Some(affinities) = self.thread_affinity.get(tid) {
                if affinities.contains(&signo) {
                    if self.thread_available.get(tid).copied().unwrap_or(true) {
                        return DeliveryRecommendation {
                            target_thread: Some(*tid),
                            priority: 8,
                            batch: false,
                            delay_ns: 0,
                            reason: String::from("Thread has affinity for signal"),
                        };
                    }
                }
            }
        }

        // Find available thread with lowest latency
        let mut best_thread = threads[0];
        let mut best_latency = u64::MAX;

        for tid in threads {
            let available = self.thread_available.get(tid).copied().unwrap_or(true);
            if !available {
                continue;
            }

            let latency = self.thread_latency.get(tid).copied().unwrap_or(0);
            if latency < best_latency {
                best_latency = latency;
                best_thread = *tid;
            }
        }

        DeliveryRecommendation {
            target_thread: Some(best_thread),
            priority: 5,
            batch: self.batching_enabled,
            delay_ns: if self.batching_enabled {
                self.batch_window_ns
            } else {
                0
            },
            reason: String::from("Selected thread with lowest latency"),
        }
    }

    /// Set batching enabled
    pub fn set_batching(&mut self, enabled: bool) {
        self.batching_enabled = enabled;
    }

    /// Check if batching is enabled
    pub fn batching_enabled(&self) -> bool {
        self.batching_enabled
    }

    /// Set batch window
    pub fn set_batch_window(&mut self, window_ns: u64) {
        self.batch_window_ns = window_ns;
    }

    /// Get batch window
    pub fn batch_window(&self) -> u64 {
        self.batch_window_ns
    }
}

impl Default for DeliveryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

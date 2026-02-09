//! Contention Analyzer
//!
//! Lock contention analysis and hotspot detection.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::{LockId, ThreadId};
use crate::core::NexusTimestamp;

/// Contention statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ContentionStats {
    /// Lock ID
    pub lock_id: LockId,
    /// Total wait time (ns)
    pub total_wait_ns: u64,
    /// Maximum wait time (ns)
    pub max_wait_ns: u64,
    /// Average wait time (ns)
    pub avg_wait_ns: f64,
    /// Total acquisitions
    pub acquisitions: u64,
    /// Total contentions
    pub contentions: u64,
    /// Failed attempts
    pub failed_attempts: u64,
}

impl ContentionStats {
    /// Record contention
    pub fn record(&mut self, wait_ns: u64, contended: bool) {
        self.acquisitions += 1;

        if contended {
            self.contentions += 1;
            self.total_wait_ns += wait_ns;
            if wait_ns > self.max_wait_ns {
                self.max_wait_ns = wait_ns;
            }
            let alpha = 0.1;
            self.avg_wait_ns = alpha * wait_ns as f64 + (1.0 - alpha) * self.avg_wait_ns;
        }
    }

    /// Record failed attempt
    #[inline(always)]
    pub fn record_failed(&mut self) {
        self.failed_attempts += 1;
    }

    /// Contention ratio
    #[inline]
    pub fn contention_ratio(&self) -> f64 {
        if self.acquisitions == 0 {
            0.0
        } else {
            self.contentions as f64 / self.acquisitions as f64
        }
    }
}

/// Contention event
#[derive(Debug, Clone)]
pub struct ContentionEvent {
    /// Lock ID
    pub lock_id: LockId,
    /// Waiting thread
    pub waiter: ThreadId,
    /// Holder thread
    pub holder: Option<ThreadId>,
    /// Wait time (ns)
    pub wait_ns: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Analyzes lock contention
pub struct ContentionAnalyzer {
    /// Per-lock stats
    stats: BTreeMap<LockId, ContentionStats>,
    /// Contention events
    events: VecDeque<ContentionEvent>,
    /// Max events
    max_events: usize,
    /// Hotspots
    hotspots: Vec<LockId>,
}

impl ContentionAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 10000,
            hotspots: Vec::new(),
        }
    }

    /// Record acquisition
    pub fn record(
        &mut self,
        lock_id: LockId,
        waiter: ThreadId,
        holder: Option<ThreadId>,
        wait_ns: u64,
    ) {
        let contended = wait_ns > 0 || holder.is_some();

        let stats = self
            .stats
            .entry(lock_id)
            .or_insert_with(|| ContentionStats {
                lock_id,
                ..Default::default()
            });
        stats.record(wait_ns, contended);

        if contended {
            let event = ContentionEvent {
                lock_id,
                waiter,
                holder,
                wait_ns,
                timestamp: NexusTimestamp::now(),
            };

            self.events.push_back(event);
            if self.events.len() > self.max_events {
                self.events.pop_front();
            }
        }

        self.update_hotspots();
    }

    /// Update hotspots
    fn update_hotspots(&mut self) {
        let mut sorted: Vec<_> = self
            .stats
            .iter()
            .filter(|(_, s)| s.contention_ratio() > 0.1)
            .map(|(&id, s)| (id, s.total_wait_ns))
            .collect();

        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        self.hotspots = sorted.iter().take(10).map(|(id, _)| *id).collect();
    }

    /// Get stats
    #[inline(always)]
    pub fn get_stats(&self, lock_id: LockId) -> Option<&ContentionStats> {
        self.stats.get(&lock_id)
    }

    /// Get hotspots
    #[inline(always)]
    pub fn hotspots(&self) -> &[LockId] {
        &self.hotspots
    }

    /// Get recent events
    #[inline]
    pub fn recent_events(&mut self, n: usize) -> &[ContentionEvent] {
        let start = self.events.len().saturating_sub(n);
        let slice = self.events.make_contiguous();
        &slice[start..]
    }

    /// Get highly contended locks
    #[inline]
    pub fn highly_contended(&self, threshold: f64) -> Vec<LockId> {
        self.stats
            .iter()
            .filter(|(_, s)| s.contention_ratio() > threshold)
            .map(|(&id, _)| id)
            .collect()
    }
}

impl Default for ContentionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

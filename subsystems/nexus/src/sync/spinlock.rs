//! Spinlock Analyzer
//!
//! Analyzes spinlock behavior and spin times.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::{LockId, ThreadId};
use crate::core::NexusTimestamp;

/// Spinlock statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SpinStats {
    /// Lock ID
    pub lock_id: LockId,
    /// Total spins
    pub total_spins: u64,
    /// Total spin iterations
    pub total_iterations: u64,
    /// Average iterations
    pub avg_iterations: f64,
    /// Maximum iterations
    pub max_iterations: u64,
    /// Successful spins
    pub successful: u64,
    /// Failed (timeout/abort)
    pub failed: u64,
}

impl SpinStats {
    /// Record spin
    pub fn record(&mut self, iterations: u64, success: bool) {
        self.total_spins += 1;
        self.total_iterations += iterations;

        if success {
            self.successful += 1;
        } else {
            self.failed += 1;
        }

        if iterations > self.max_iterations {
            self.max_iterations = iterations;
        }

        let alpha = 0.1;
        self.avg_iterations = alpha * iterations as f64 + (1.0 - alpha) * self.avg_iterations;
    }

    /// Success rate
    #[inline]
    pub fn success_rate(&self) -> f64 {
        if self.total_spins == 0 {
            1.0
        } else {
            self.successful as f64 / self.total_spins as f64
        }
    }
}

/// Spin event
#[derive(Debug, Clone)]
pub struct SpinEvent {
    /// Lock ID
    pub lock_id: LockId,
    /// Thread
    pub thread: ThreadId,
    /// Iterations
    pub iterations: u64,
    /// Success
    pub success: bool,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Long spin
#[derive(Debug, Clone)]
pub struct LongSpin {
    /// Lock ID
    pub lock_id: LockId,
    /// Thread
    pub thread: ThreadId,
    /// Holder
    pub holder: Option<ThreadId>,
    /// Iterations
    pub iterations: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Analyzes spinlock behavior
pub struct SpinlockAnalyzer {
    /// Per-spinlock stats
    stats: BTreeMap<LockId, SpinStats>,
    /// Spin events
    events: VecDeque<SpinEvent>,
    /// Long spins
    long_spins: Vec<LongSpin>,
}

impl SpinlockAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            events: VecDeque::new(),
            long_spins: Vec::new(),
        }
    }

    /// Record spin
    pub fn record(
        &mut self,
        lock_id: LockId,
        thread: ThreadId,
        iterations: u64,
        success: bool,
        holder: Option<ThreadId>,
    ) {
        let stats = self.stats.entry(lock_id).or_insert_with(|| SpinStats {
            lock_id,
            ..Default::default()
        });
        stats.record(iterations, success);

        let event = SpinEvent {
            lock_id,
            thread,
            iterations,
            success,
            timestamp: NexusTimestamp::now(),
        };

        self.events.push_back(event);
        if self.events.len() > 10000 {
            self.events.pop_front();
        }

        // Track long spins
        if iterations > 10000 {
            self.long_spins.push(LongSpin {
                lock_id,
                thread,
                holder,
                iterations,
                timestamp: NexusTimestamp::now(),
            });
        }
    }

    /// Get stats
    #[inline(always)]
    pub fn get_stats(&self, lock_id: LockId) -> Option<&SpinStats> {
        self.stats.get(&lock_id)
    }

    /// Get long spins
    #[inline(always)]
    pub fn long_spins(&self) -> &[LongSpin] {
        &self.long_spins
    }

    /// Get high-spin locks
    #[inline]
    pub fn high_spin_locks(&self, threshold: u64) -> Vec<LockId> {
        self.stats
            .iter()
            .filter(|(_, s)| s.avg_iterations > threshold as f64)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get recent events
    #[inline]
    pub fn recent_events(&mut self, n: usize) -> &[SpinEvent] {
        let start = self.events.len().saturating_sub(n);
        let slice = self.events.make_contiguous();
        &slice[start..]
    }
}

impl Default for SpinlockAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

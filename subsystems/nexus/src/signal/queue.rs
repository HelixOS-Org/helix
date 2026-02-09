//! Signal Queue Manager
//!
//! Manages pending signal queues with coalescing support.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use super::{DeliveryState, PendingSignal, ProcessId, SignalNumber};

/// Queue statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct QueueStats {
    /// Total signals enqueued
    pub enqueued: u64,
    /// Total signals delivered
    pub delivered: u64,
    /// Total signals dropped
    pub dropped: u64,
    /// Total signals coalesced
    pub coalesced: u64,
    /// Total signals ignored
    pub ignored: u64,
    /// Peak queue depth
    pub peak_depth: u64,
    /// Current queue depth
    pub current_depth: u64,
}

/// Signal queue manager
#[repr(align(64))]
pub struct SignalQueueManager {
    /// Pending signals per process
    pending: BTreeMap<ProcessId, Vec<PendingSignal>>,
    /// Queue capacity per process
    queue_capacity: usize,
    /// Per-process stats
    per_process_stats: BTreeMap<ProcessId, QueueStats>,
    /// Global stats
    global_stats: QueueStats,
    /// Enable signal coalescing
    coalescing_enabled: bool,
    /// Signals that can be coalesced
    coalescable_signals: Vec<SignalNumber>,
    /// Priority signals (delivered first)
    priority_signals: Vec<SignalNumber>,
}

impl SignalQueueManager {
    /// Create new queue manager
    pub fn new(capacity: usize) -> Self {
        Self {
            pending: BTreeMap::new(),
            queue_capacity: capacity,
            per_process_stats: BTreeMap::new(),
            global_stats: QueueStats::default(),
            coalescing_enabled: true,
            coalescable_signals: vec![SignalNumber::SIGCHLD, SignalNumber::SIGCONT],
            priority_signals: vec![SignalNumber::SIGKILL, SignalNumber::SIGSTOP],
        }
    }

    /// Enqueue signal
    pub fn enqueue(&mut self, signal: PendingSignal) -> bool {
        let pid = signal.target_pid;
        let signo = signal.info.signo;

        let queue = self.pending.entry(pid).or_default();
        let stats = self.per_process_stats.entry(pid).or_default();

        // Check if queue is full
        if queue.len() >= self.queue_capacity {
            stats.dropped += 1;
            self.global_stats.dropped += 1;
            return false;
        }

        // Try to coalesce
        if self.coalescing_enabled && self.coalescable_signals.contains(&signo) {
            for existing in queue.iter_mut() {
                if existing.info.signo == signo && existing.state == DeliveryState::Pending {
                    // Coalesce - just update the existing signal
                    stats.coalesced += 1;
                    self.global_stats.coalesced += 1;
                    return true;
                }
            }
        }

        // Add to queue
        queue.push(signal);
        stats.enqueued += 1;
        stats.current_depth = queue.len() as u64;
        if stats.current_depth > stats.peak_depth {
            stats.peak_depth = stats.current_depth;
        }

        self.global_stats.enqueued += 1;
        self.global_stats.current_depth += 1;
        if self.global_stats.current_depth > self.global_stats.peak_depth {
            self.global_stats.peak_depth = self.global_stats.current_depth;
        }

        true
    }

    /// Dequeue next signal for process
    pub fn dequeue(&mut self, pid: ProcessId) -> Option<PendingSignal> {
        let queue = self.pending.get_mut(&pid)?;

        if queue.is_empty() {
            return None;
        }

        // Priority signals first
        for (i, signal) in queue.iter().enumerate() {
            if signal.state == DeliveryState::Pending
                && self.priority_signals.contains(&signal.info.signo)
            {
                let signal = queue.remove(i);
                self.update_dequeue_stats(pid);
                return Some(signal);
            }
        }

        // Then any pending signal
        for (i, signal) in queue.iter().enumerate() {
            if signal.state == DeliveryState::Pending {
                let signal = queue.remove(i);
                self.update_dequeue_stats(pid);
                return Some(signal);
            }
        }

        None
    }

    /// Update stats after dequeue
    fn update_dequeue_stats(&mut self, pid: ProcessId) {
        if let Some(stats) = self.per_process_stats.get_mut(&pid) {
            stats.delivered += 1;
            stats.current_depth = stats.current_depth.saturating_sub(1);
        }
        self.global_stats.delivered += 1;
        self.global_stats.current_depth = self.global_stats.current_depth.saturating_sub(1);
    }

    /// Mark signal as delivered
    #[inline]
    pub fn mark_delivered(&mut self, pid: ProcessId, signo: SignalNumber) {
        if let Some(queue) = self.pending.get_mut(&pid) {
            for signal in queue.iter_mut() {
                if signal.info.signo == signo && signal.state == DeliveryState::Delivering {
                    signal.state = DeliveryState::Delivered;
                    break;
                }
            }
        }
    }

    /// Mark signal as ignored
    pub fn mark_ignored(&mut self, pid: ProcessId, signo: SignalNumber) {
        if let Some(queue) = self.pending.get_mut(&pid) {
            queue.retain(|s| {
                if s.info.signo == signo && s.state == DeliveryState::Pending {
                    if let Some(stats) = self.per_process_stats.get_mut(&pid) {
                        stats.ignored += 1;
                        stats.current_depth = stats.current_depth.saturating_sub(1);
                    }
                    self.global_stats.ignored += 1;
                    self.global_stats.current_depth =
                        self.global_stats.current_depth.saturating_sub(1);
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Get pending count for process
    #[inline]
    pub fn pending_count(&self, pid: ProcessId) -> usize {
        self.pending
            .get(&pid)
            .map(|q| {
                q.iter()
                    .filter(|s| s.state == DeliveryState::Pending)
                    .count()
            })
            .unwrap_or(0)
    }

    /// Get pending signals for process
    #[inline(always)]
    pub fn get_pending(&self, pid: ProcessId) -> Option<&[PendingSignal]> {
        self.pending.get(&pid).map(|v| v.as_slice())
    }

    /// Check if signal is pending
    #[inline]
    pub fn is_pending(&self, pid: ProcessId, signo: SignalNumber) -> bool {
        self.pending
            .get(&pid)
            .map(|q| {
                q.iter()
                    .any(|s| s.info.signo == signo && s.state == DeliveryState::Pending)
            })
            .unwrap_or(false)
    }

    /// Cleanup delivered signals
    #[inline]
    pub fn cleanup(&mut self, pid: ProcessId) {
        if let Some(queue) = self.pending.get_mut(&pid) {
            queue
                .retain(|s| s.state == DeliveryState::Pending || s.state == DeliveryState::Blocked);
        }
    }

    /// Get global stats
    #[inline(always)]
    pub fn global_stats(&self) -> &QueueStats {
        &self.global_stats
    }

    /// Get per-process stats
    #[inline(always)]
    pub fn process_stats(&self, pid: ProcessId) -> Option<&QueueStats> {
        self.per_process_stats.get(&pid)
    }

    /// Set coalescing enabled
    #[inline(always)]
    pub fn set_coalescing(&mut self, enabled: bool) {
        self.coalescing_enabled = enabled;
    }

    /// Set coalescable signals
    #[inline(always)]
    pub fn set_coalescable_signals(&mut self, signals: Vec<SignalNumber>) {
        self.coalescable_signals = signals;
    }

    /// Set priority signals
    #[inline(always)]
    pub fn set_priority_signals(&mut self, signals: Vec<SignalNumber>) {
        self.priority_signals = signals;
    }

    /// Get queue capacity
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.queue_capacity
    }
}

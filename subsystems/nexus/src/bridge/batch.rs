//! # Automatic Syscall Batching
//!
//! Merges multiple similar syscalls into single optimized operations.
//! For example, 4 sequential 4KB reads become a single 16KB DMA transfer.

use alloc::vec::Vec;

use super::syscall::{SyscallId, SyscallType};

// ============================================================================
// BATCH TYPES
// ============================================================================

/// A single entry in the batch queue
#[derive(Debug, Clone)]
pub struct BatchEntry {
    /// The original syscall ID
    pub id: SyscallId,
    /// The syscall type
    pub syscall_type: SyscallType,
    /// Data size (bytes)
    pub data_size: usize,
    /// Submission timestamp (ticks)
    pub submitted_at: u64,
    /// Deadline — must be dispatched by this time
    pub deadline: u64,
    /// Process ID
    pub pid: u64,
}

impl BatchEntry {
    /// Create a new batch entry
    pub fn new(id: SyscallId, syscall_type: SyscallType, data_size: usize) -> Self {
        Self {
            id,
            syscall_type,
            data_size,
            submitted_at: 0,
            deadline: u64::MAX,
            pid: 0,
        }
    }

    /// Set the submission timestamp
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.submitted_at = ts;
        self
    }

    /// Set the deadline
    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = deadline;
        self
    }

    /// Set the pid
    pub fn with_pid(mut self, pid: u64) -> Self {
        self.pid = pid;
        self
    }
}

/// A group of batched syscalls that will be executed as one
#[derive(Debug, Clone)]
pub struct BatchGroup {
    /// The merged syscall type
    pub syscall_type: SyscallType,
    /// All entries in this batch
    pub entries: Vec<BatchEntry>,
    /// Total merged data size
    pub total_data_size: usize,
    /// Earliest deadline in the group
    pub earliest_deadline: u64,
    /// Estimated latency savings (ns) vs individual execution
    pub estimated_savings_ns: u64,
}

impl BatchGroup {
    /// Create a new batch group
    pub fn new(syscall_type: SyscallType) -> Self {
        Self {
            syscall_type,
            entries: Vec::new(),
            total_data_size: 0,
            earliest_deadline: u64::MAX,
            estimated_savings_ns: 0,
        }
    }

    /// Add an entry to the batch
    pub fn add(&mut self, entry: BatchEntry) {
        self.total_data_size += entry.data_size;
        if entry.deadline < self.earliest_deadline {
            self.earliest_deadline = entry.deadline;
        }
        self.entries.push(entry);
        self.recalculate_savings();
    }

    /// Number of entries in this batch
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Whether this batch is worth merging (saves more than overhead)
    pub fn is_worthwhile(&self) -> bool {
        self.entries.len() >= 2 && self.estimated_savings_ns > 500
    }

    /// Recalculate estimated savings
    fn recalculate_savings(&mut self) {
        // Each individual syscall has ~2000ns overhead (context switch, etc.)
        // A batched operation shares the overhead
        let individual_cost = self.entries.len() as u64 * 2000;
        let batched_cost = 2000 + (self.entries.len() as u64 - 1) * 200;
        self.estimated_savings_ns = individual_cost.saturating_sub(batched_cost);
    }
}

/// The outcome of a batch decision
#[derive(Debug, Clone)]
pub enum BatchDecision {
    /// Execute immediately — not worth batching
    ExecuteNow(BatchEntry),
    /// Queued — waiting for more entries
    Queued,
    /// Batch is ready — execute as group
    BatchReady(BatchGroup),
    /// Queue is full — flush everything
    FlushAll(Vec<BatchGroup>),
}

/// Statistics about batching performance
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    /// Total entries submitted
    pub total_submitted: u64,
    /// Total entries batched
    pub total_batched: u64,
    /// Total entries executed individually
    pub total_individual: u64,
    /// Total batches formed
    pub total_batches: u64,
    /// Total estimated time saved (ns)
    pub total_savings_ns: u64,
    /// Average batch size
    pub avg_batch_size: f64,
}

impl BatchStats {
    pub fn batching_rate(&self) -> f64 {
        if self.total_submitted == 0 {
            return 0.0;
        }
        self.total_batched as f64 / self.total_submitted as f64
    }
}

// ============================================================================
// BATCH OPTIMIZER
// ============================================================================

/// The batch optimizer — queues incoming syscalls and forms optimal batches.
///
/// ## Strategy
///
/// 1. Incoming batchable syscalls are placed in per-type queues
/// 2. When a queue reaches `batch_threshold` entries, a batch is formed
/// 3. Deadlines force early flush if entries would expire
/// 4. Non-batchable syscalls pass through immediately
pub struct BatchOptimizer {
    /// Per-type queues (type_key -> pending entries)
    queues: Vec<(SyscallType, Vec<BatchEntry>)>,
    /// Minimum entries to form a batch
    batch_threshold: usize,
    /// Maximum time to wait for a batch to fill (ticks)
    max_wait_ticks: u64,
    /// Maximum entries in any single queue
    max_queue_size: usize,
    /// Statistics
    stats: BatchStats,
}

impl BatchOptimizer {
    /// Create a new optimizer
    pub fn new(batch_threshold: usize, max_wait_ticks: u64) -> Self {
        Self {
            queues: Vec::new(),
            batch_threshold: batch_threshold.max(2),
            max_wait_ticks,
            max_queue_size: 64,
            stats: BatchStats::default(),
        }
    }

    /// Submit an entry for potential batching
    pub fn submit(&mut self, entry: BatchEntry) -> BatchDecision {
        self.stats.total_submitted += 1;

        // Non-batchable → execute immediately
        if !entry.syscall_type.is_batchable() {
            self.stats.total_individual += 1;
            return BatchDecision::ExecuteNow(entry);
        }

        let syscall_type = entry.syscall_type;

        // Find or create queue for this type
        let queue_idx = self.queues.iter().position(|(t, _)| *t == syscall_type);

        let idx = match queue_idx {
            Some(i) => i,
            None => {
                self.queues.push((syscall_type, Vec::new()));
                self.queues.len() - 1
            },
        };

        self.queues[idx].1.push(entry);

        // Check if batch is ready
        if self.queues[idx].1.len() >= self.batch_threshold {
            let entries = core::mem::take(&mut self.queues[idx].1);
            let mut group = BatchGroup::new(syscall_type);
            self.stats.total_batched += entries.len() as u64;
            for e in entries {
                group.add(e);
            }
            self.stats.total_batches += 1;
            self.stats.total_savings_ns += group.estimated_savings_ns;
            let batch_size = group.size();
            let avg = &mut self.stats.avg_batch_size;
            *avg = (*avg * 0.9) + (batch_size as f64 * 0.1);
            return BatchDecision::BatchReady(group);
        }

        // Check total queue sizes
        let total_queued: usize = self.queues.iter().map(|(_, q)| q.len()).sum();
        if total_queued >= self.max_queue_size {
            return BatchDecision::FlushAll(self.flush());
        }

        BatchDecision::Queued
    }

    /// Flush all pending queues into batch groups
    pub fn flush(&mut self) -> Vec<BatchGroup> {
        let mut groups = Vec::new();
        let mut total_batched = 0u64;
        let mut total_savings = 0u64;

        for (syscall_type, queue) in &mut self.queues {
            if queue.is_empty() {
                continue;
            }

            let entries = core::mem::take(queue);
            let count = entries.len() as u64;
            let mut group = BatchGroup::new(*syscall_type);
            for e in entries {
                group.add(e);
            }
            if group.size() > 0 {
                total_batched += count;
                total_savings += group.estimated_savings_ns;
                groups.push(group);
            }
        }

        self.stats.total_batched += total_batched;
        self.stats.total_batches += groups.len() as u64;
        self.stats.total_savings_ns += total_savings;
        for g in &groups {
            self.update_avg_batch_size(g.size());
        }

        groups
    }

    /// Check for expired entries and flush them
    pub fn check_deadlines(&mut self, current_time: u64) -> Vec<BatchGroup> {
        let mut expired_groups = Vec::new();
        let mut total_batched = 0u64;
        let mut total_savings = 0u64;
        let max_wait = self.max_wait_ticks;

        for (syscall_type, queue) in &mut self.queues {
            let has_expired = queue
                .iter()
                .any(|e| current_time.saturating_sub(e.submitted_at) >= max_wait);

            if has_expired && !queue.is_empty() {
                let entries = core::mem::take(queue);
                let count = entries.len() as u64;
                let mut group = BatchGroup::new(*syscall_type);
                for e in entries {
                    group.add(e);
                }
                total_batched += count;
                total_savings += group.estimated_savings_ns;
                expired_groups.push(group);
            }
        }

        self.stats.total_batched += total_batched;
        self.stats.total_batches += expired_groups.len() as u64;
        self.stats.total_savings_ns += total_savings;
        for g in &expired_groups {
            self.update_avg_batch_size(g.size());
        }

        expired_groups
    }

    /// Get batching statistics
    pub fn stats(&self) -> &BatchStats {
        &self.stats
    }

    /// Current total pending count
    pub fn pending_count(&self) -> usize {
        self.queues.iter().map(|(_, q)| q.len()).sum()
    }

    fn update_avg_batch_size(&mut self, size: usize) {
        if self.stats.total_batches == 0 {
            self.stats.avg_batch_size = 0.0;
        } else {
            let prev = self.stats.avg_batch_size * (self.stats.total_batches - 1) as f64;
            self.stats.avg_batch_size = (prev + size as f64) / self.stats.total_batches as f64;
        }
    }
}

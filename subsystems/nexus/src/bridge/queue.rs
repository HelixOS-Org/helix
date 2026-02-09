//! # Syscall Queue Management
//!
//! Priority-based syscall queuing:
//! - Multi-priority queues
//! - Work stealing
//! - Queue drainage policies
//! - Backpressure management
//! - Queue metrics and monitoring
//! - Fair scheduling across queues

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// QUEUE PRIORITY
// ============================================================================

/// Queue priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    /// Real-time priority
    Realtime   = 0,
    /// High priority
    High       = 1,
    /// Normal priority
    Normal     = 2,
    /// Low priority
    Low        = 3,
    /// Background / idle
    Background = 4,
}

impl QueuePriority {
    /// Weight for scheduling (higher = more CPU time)
    #[inline]
    pub fn weight(&self) -> u32 {
        match self {
            Self::Realtime => 100,
            Self::High => 50,
            Self::Normal => 20,
            Self::Low => 5,
            Self::Background => 1,
        }
    }

    /// Max queue depth
    #[inline]
    pub fn max_depth(&self) -> usize {
        match self {
            Self::Realtime => 64,
            Self::High => 256,
            Self::Normal => 1024,
            Self::Low => 512,
            Self::Background => 256,
        }
    }
}

// ============================================================================
// QUEUE ENTRY
// ============================================================================

/// Queued syscall entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QueueEntry {
    /// Entry ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Arguments
    pub args: [u64; 6],
    /// Enqueue timestamp
    pub enqueued_at: u64,
    /// Deadline (0 = no deadline)
    pub deadline: u64,
    /// Priority
    pub priority: QueuePriority,
    /// Estimated cost (arbitrary units)
    pub estimated_cost: u32,
}

/// Queue entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    /// Queued, waiting
    Pending,
    /// Being processed
    Processing,
    /// Complete
    Complete,
    /// Timed out
    TimedOut,
    /// Cancelled
    Cancelled,
}

// ============================================================================
// SINGLE QUEUE
// ============================================================================

/// A single priority queue
#[repr(align(64))]
pub struct SyscallQueue {
    /// Priority level
    pub priority: QueuePriority,
    /// Entries
    entries: Vec<QueueEntry>,
    /// Maximum depth
    max_depth: usize,
    /// Total enqueued
    pub total_enqueued: u64,
    /// Total dequeued
    pub total_dequeued: u64,
    /// Total dropped (overflow)
    pub total_dropped: u64,
    /// Total timed out
    pub total_timed_out: u64,
    /// Peak depth
    pub peak_depth: usize,
}

impl SyscallQueue {
    pub fn new(priority: QueuePriority) -> Self {
        Self {
            priority,
            entries: Vec::new(),
            max_depth: priority.max_depth(),
            total_enqueued: 0,
            total_dequeued: 0,
            total_dropped: 0,
            total_timed_out: 0,
            peak_depth: 0,
        }
    }

    /// Enqueue entry
    pub fn enqueue(&mut self, entry: QueueEntry) -> bool {
        if self.entries.len() >= self.max_depth {
            self.total_dropped += 1;
            return false;
        }

        self.total_enqueued += 1;
        self.entries.push(entry);

        if self.entries.len() > self.peak_depth {
            self.peak_depth = self.entries.len();
        }

        true
    }

    /// Dequeue highest priority entry (respecting deadlines)
    pub fn dequeue(&mut self, current_time: u64) -> Option<QueueEntry> {
        if self.entries.is_empty() {
            return None;
        }

        // Find entry with earliest deadline (or first if no deadlines)
        let mut best_idx = 0;
        let mut best_deadline = u64::MAX;

        for (i, entry) in self.entries.iter().enumerate() {
            // Skip expired
            if entry.deadline > 0 && current_time > entry.deadline {
                continue;
            }

            let deadline = if entry.deadline > 0 {
                entry.deadline
            } else {
                u64::MAX
            };

            if deadline < best_deadline {
                best_deadline = deadline;
                best_idx = i;
            }
        }

        self.total_dequeued += 1;
        Some(self.entries.remove(best_idx))
    }

    /// Remove expired entries
    #[inline]
    pub fn expire(&mut self, current_time: u64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|e| e.deadline == 0 || current_time <= e.deadline);
        let expired = before - self.entries.len();
        self.total_timed_out += expired as u64;
        expired
    }

    /// Depth
    #[inline(always)]
    pub fn depth(&self) -> usize {
        self.entries.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Peek at next entry
    #[inline(always)]
    pub fn peek(&self) -> Option<&QueueEntry> {
        self.entries.first()
    }

    /// Steal entries (for work stealing)
    #[inline]
    pub fn steal(&mut self, count: usize) -> Vec<QueueEntry> {
        let steal_count = count.min(self.entries.len() / 2);
        let start = self.entries.len() - steal_count;
        self.entries.split_off(start)
    }
}

// ============================================================================
// BACKPRESSURE
// ============================================================================

/// Backpressure state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureState {
    /// No pressure
    Normal,
    /// Moderate - start slowing down
    Moderate,
    /// High - actively rejecting
    High,
    /// Critical - emergency drain
    Critical,
}

/// Backpressure configuration
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Moderate threshold (percent of max depth)
    pub moderate_threshold: u32,
    /// High threshold
    pub high_threshold: u32,
    /// Critical threshold
    pub critical_threshold: u32,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            moderate_threshold: 50,
            high_threshold: 75,
            critical_threshold: 90,
        }
    }
}

// ============================================================================
// DRAINAGE POLICY
// ============================================================================

/// How to drain queues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainagePolicy {
    /// Strict priority (always drain highest first)
    StrictPriority,
    /// Weighted fair queuing
    WeightedFair,
    /// Round robin across priorities
    RoundRobin,
    /// Deficit round robin
    DeficitRoundRobin,
}

/// Drainage state for deficit round robin
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DrainageState {
    /// Policy
    pub policy: DrainagePolicy,
    /// Per-queue deficit counters (for DRR)
    deficits: BTreeMap<u8, u32>,
    /// Quantum per queue
    quantum: u32,
    /// Current queue index (for round robin)
    current_idx: u8,
}

impl DrainageState {
    pub fn new(policy: DrainagePolicy) -> Self {
        Self {
            policy,
            deficits: BTreeMap::new(),
            quantum: 100,
            current_idx: 0,
        }
    }

    /// Select next queue to drain
    pub fn select_queue(&mut self, depths: &[(QueuePriority, usize)]) -> Option<QueuePriority> {
        match self.policy {
            DrainagePolicy::StrictPriority => depths
                .iter()
                .filter(|(_, d)| *d > 0)
                .map(|(p, _)| *p)
                .next(),
            DrainagePolicy::RoundRobin => {
                let non_empty: Vec<QueuePriority> = depths
                    .iter()
                    .filter(|(_, d)| *d > 0)
                    .map(|(p, _)| *p)
                    .collect();

                if non_empty.is_empty() {
                    return None;
                }

                let idx = self.current_idx as usize % non_empty.len();
                self.current_idx = self.current_idx.wrapping_add(1);
                Some(non_empty[idx])
            },
            DrainagePolicy::WeightedFair => {
                // Weighted: pick queue with highest weight * depth ratio
                let mut best: Option<(QueuePriority, u64)> = None;
                for &(prio, depth) in depths {
                    if depth == 0 {
                        continue;
                    }
                    let score = prio.weight() as u64 * depth as u64;
                    match best {
                        None => best = Some((prio, score)),
                        Some((_, s)) if score > s => best = Some((prio, score)),
                        _ => {},
                    }
                }
                best.map(|(p, _)| p)
            },
            DrainagePolicy::DeficitRoundRobin => {
                let non_empty: Vec<QueuePriority> = depths
                    .iter()
                    .filter(|(_, d)| *d > 0)
                    .map(|(p, _)| *p)
                    .collect();

                if non_empty.is_empty() {
                    return None;
                }

                // Add quantum to all deficits
                for &prio in &non_empty {
                    let deficit = self.deficits.entry(prio as u8).or_insert(0);
                    *deficit += self.quantum * prio.weight() / 20;
                }

                // Pick queue with highest deficit
                let selected = non_empty
                    .iter()
                    .max_by_key(|p| self.deficits.get(&(**p as u8)).copied().unwrap_or(0))
                    .copied();

                if let Some(prio) = selected {
                    if let Some(deficit) = self.deficits.get_mut(&(prio as u8)) {
                        *deficit = deficit.saturating_sub(self.quantum);
                    }
                }

                selected
            },
        }
    }
}

// ============================================================================
// QUEUE MANAGER
// ============================================================================

/// Queue statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct QueueManagerStats {
    /// Total enqueued
    pub total_enqueued: u64,
    /// Total dequeued
    pub total_dequeued: u64,
    /// Total dropped
    pub total_dropped: u64,
    /// Total expired
    pub total_expired: u64,
    /// Total stolen (work stealing)
    pub total_stolen: u64,
    /// Average wait time (microseconds)
    pub avg_wait_us: u64,
}

/// Multi-priority syscall queue manager
#[repr(align(64))]
pub struct QueueManager {
    /// Priority queues
    queues: BTreeMap<u8, SyscallQueue>,
    /// Drainage state
    drainage: DrainageState,
    /// Backpressure config
    backpressure_config: BackpressureConfig,
    /// Current backpressure state
    pub backpressure: BackpressureState,
    /// Next entry ID
    next_id: u64,
    /// Statistics
    pub stats: QueueManagerStats,
}

impl QueueManager {
    pub fn new(policy: DrainagePolicy) -> Self {
        let mut queues = BTreeMap::new();
        queues.insert(
            QueuePriority::Realtime as u8,
            SyscallQueue::new(QueuePriority::Realtime),
        );
        queues.insert(
            QueuePriority::High as u8,
            SyscallQueue::new(QueuePriority::High),
        );
        queues.insert(
            QueuePriority::Normal as u8,
            SyscallQueue::new(QueuePriority::Normal),
        );
        queues.insert(
            QueuePriority::Low as u8,
            SyscallQueue::new(QueuePriority::Low),
        );
        queues.insert(
            QueuePriority::Background as u8,
            SyscallQueue::new(QueuePriority::Background),
        );

        Self {
            queues,
            drainage: DrainageState::new(policy),
            backpressure_config: BackpressureConfig::default(),
            backpressure: BackpressureState::Normal,
            next_id: 1,
            stats: QueueManagerStats::default(),
        }
    }

    /// Enqueue syscall
    pub fn enqueue(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        args: [u64; 6],
        priority: QueuePriority,
        deadline: u64,
        cost: u32,
        timestamp: u64,
    ) -> Option<u64> {
        // Check backpressure
        if self.backpressure == BackpressureState::Critical && priority != QueuePriority::Realtime {
            self.stats.total_dropped += 1;
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;

        let entry = QueueEntry {
            id,
            pid,
            syscall_nr,
            args,
            enqueued_at: timestamp,
            deadline,
            priority,
            estimated_cost: cost,
        };

        if let Some(queue) = self.queues.get_mut(&(priority as u8)) {
            if queue.enqueue(entry) {
                self.stats.total_enqueued += 1;
                self.update_backpressure();
                return Some(id);
            }
        }

        self.stats.total_dropped += 1;
        None
    }

    /// Dequeue next entry based on drainage policy
    pub fn dequeue(&mut self, current_time: u64) -> Option<QueueEntry> {
        let depths: Vec<(QueuePriority, usize)> = [
            QueuePriority::Realtime,
            QueuePriority::High,
            QueuePriority::Normal,
            QueuePriority::Low,
            QueuePriority::Background,
        ]
        .iter()
        .map(|p| {
            let depth = self.queues.get(&(*p as u8)).map(|q| q.depth()).unwrap_or(0);
            (*p, depth)
        })
        .collect();

        let selected = self.drainage.select_queue(&depths)?;

        if let Some(queue) = self.queues.get_mut(&(selected as u8)) {
            let entry = queue.dequeue(current_time);
            if entry.is_some() {
                self.stats.total_dequeued += 1;
                self.update_backpressure();
            }
            entry
        } else {
            None
        }
    }

    /// Expire old entries
    #[inline]
    pub fn expire(&mut self, current_time: u64) -> usize {
        let mut total = 0;
        for queue in self.queues.values_mut() {
            total += queue.expire(current_time);
        }
        self.stats.total_expired += total as u64;
        total
    }

    /// Update backpressure state
    fn update_backpressure(&mut self) {
        let total_depth: usize = self.queues.values().map(|q| q.depth()).sum();
        let total_capacity: usize = self.queues.values().map(|q| q.max_depth).sum();

        if total_capacity == 0 {
            self.backpressure = BackpressureState::Normal;
            return;
        }

        let fill_pct = (total_depth * 100 / total_capacity) as u32;

        self.backpressure = if fill_pct >= self.backpressure_config.critical_threshold {
            BackpressureState::Critical
        } else if fill_pct >= self.backpressure_config.high_threshold {
            BackpressureState::High
        } else if fill_pct >= self.backpressure_config.moderate_threshold {
            BackpressureState::Moderate
        } else {
            BackpressureState::Normal
        };
    }

    /// Total depth across all queues
    #[inline(always)]
    pub fn total_depth(&self) -> usize {
        self.queues.values().map(|q| q.depth()).sum()
    }

    /// Queue depth for specific priority
    #[inline]
    pub fn queue_depth(&self, priority: QueuePriority) -> usize {
        self.queues
            .get(&(priority as u8))
            .map(|q| q.depth())
            .unwrap_or(0)
    }
}

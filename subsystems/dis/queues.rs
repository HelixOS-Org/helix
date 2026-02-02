//! # Multi-Level Priority Queues
//!
//! The Queue subsystem implements a sophisticated multi-level feedback queue
//! system that supports priority inheritance, aging, and dynamic queue management.
//!
//! ## Queue Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                         MULTI-LEVEL QUEUE SYSTEM                             │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐  │
//! │  │                          REAL-TIME QUEUES                              │  │
//! │  │                                                                        │  │
//! │  │  Priority 99 ║████████████████████████████████████████████████████████  │  │
//! │  │  Priority 98 ║██████████████████████████████████████████████████████    │  │
//! │  │  Priority 97 ║████████████████████████████████████████████████████      │  │
//! │  │  ...         ║                                                          │  │
//! │  │  Priority 50 ║████████████████████████████                              │  │
//! │  └────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐  │
//! │  │                        INTERACTIVE QUEUES                              │  │
//! │  │                                                                        │  │
//! │  │  High     ║██████████████████████████████████████                       │  │
//! │  │  Normal   ║████████████████████████████████                             │  │
//! │  │  Low      ║██████████████████████████                                   │  │
//! │  └────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐  │
//! │  │                          NORMAL QUEUES                                 │  │
//! │  │                                                                        │  │
//! │  │  Level 0  ║██████████████████████████████████████████████████████████   │  │
//! │  │  Level 1  ║████████████████████████████████████████████████████████     │  │
//! │  │  Level 2  ║██████████████████████████████████████████████████████       │  │
//! │  │  Level 3  ║████████████████████████████████████████████████████         │  │
//! │  │  Level 4  ║██████████████████████████████████████████████████           │  │
//! │  │  Level 5  ║████████████████████████████████████████████████             │  │
//! │  │  Level 6  ║██████████████████████████████████████████████               │  │
//! │  │  Level 7  ║████████████████████████████████████████████                 │  │
//! │  └────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐  │
//! │  │                           BATCH QUEUE                                  │  │
//! │  │                                                                        │  │
//! │  │  Queue    ║████████████████████████████████████████████████             │  │
//! │  └────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐  │
//! │  │                        BACKGROUND QUEUE                                │  │
//! │  │                                                                        │  │
//! │  │  Queue    ║████████████████████████████████████████████                 │  │
//! │  └────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐  │
//! │  │                           IDLE QUEUE                                   │  │
//! │  │                                                                        │  │
//! │  │  Queue    ║████████████████████████████████████████                     │  │
//! │  └────────────────────────────────────────────────────────────────────────┘  │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Priority Inheritance**: Prevents priority inversion
//! - **Aging**: Prevents starvation of low-priority tasks
//! - **Dynamic Levels**: Multi-level feedback with automatic demotion
//! - **Deadline Support**: Special handling for deadline-constrained tasks
//! - **Fair Scheduling**: Virtual runtime tracking for fairness

use alloc::collections::{BTreeMap, BinaryHeap, VecDeque};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering as AtomicOrdering};

use spin::{Mutex, RwLock};

use super::{Nanoseconds, TaskId};

// =============================================================================
// Queue Configuration
// =============================================================================

/// Queue system configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Number of real-time priority levels (1-99)
    pub realtime_levels: u8,
    /// Number of interactive levels
    pub interactive_levels: u8,
    /// Number of normal levels (MLFQ)
    pub normal_levels: u8,
    /// Enable aging
    pub aging_enabled: bool,
    /// Aging interval (ticks)
    pub aging_interval: u64,
    /// Aging boost (levels to boost)
    pub aging_boost: u8,
    /// Enable priority inheritance
    pub priority_inheritance: bool,
    /// Default time slice per level (ns)
    pub time_slices: Vec<Nanoseconds>,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            realtime_levels: 50,
            interactive_levels: 3,
            normal_levels: 8,
            aging_enabled: true,
            aging_interval: 1000,
            aging_boost: 1,
            priority_inheritance: true,
            time_slices: vec![
                Nanoseconds::from_millis(2),   // RT
                Nanoseconds::from_millis(4),   // Interactive high
                Nanoseconds::from_millis(6),   // Interactive normal
                Nanoseconds::from_millis(8),   // Interactive low
                Nanoseconds::from_millis(10),  // Normal 0
                Nanoseconds::from_millis(15),  // Normal 1
                Nanoseconds::from_millis(20),  // Normal 2
                Nanoseconds::from_millis(25),  // Normal 3
                Nanoseconds::from_millis(30),  // Normal 4
                Nanoseconds::from_millis(40),  // Normal 5
                Nanoseconds::from_millis(50),  // Normal 6
                Nanoseconds::from_millis(60),  // Normal 7
                Nanoseconds::from_millis(100), // Batch
                Nanoseconds::from_millis(200), // Background
                Nanoseconds::from_millis(500), // Idle
            ],
        }
    }
}

// =============================================================================
// Queue Entry
// =============================================================================

/// Entry in a queue
#[derive(Debug, Clone)]
pub struct QueueEntry {
    /// Task ID
    pub task_id: TaskId,
    /// Effective priority (may be boosted)
    pub effective_priority: i16,
    /// Base priority
    pub base_priority: i16,
    /// Virtual runtime (for fairness)
    pub vruntime: u64,
    /// Deadline (if applicable)
    pub deadline: Option<Nanoseconds>,
    /// Time slice remaining
    pub time_slice: Nanoseconds,
    /// Wait time (for aging)
    pub wait_ticks: u64,
    /// Level in MLFQ
    pub level: u8,
    /// Entry flags
    pub flags: QueueEntryFlags,
}

bitflags::bitflags! {
    /// Queue entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct QueueEntryFlags: u32 {
        /// Task is boosted
        const BOOSTED = 1 << 0;
        /// Task has deadline
        const HAS_DEADLINE = 1 << 1;
        /// Task is real-time
        const REALTIME = 1 << 2;
        /// Task is interactive
        const INTERACTIVE = 1 << 3;
        /// Task is batch
        const BATCH = 1 << 4;
        /// Task is background
        const BACKGROUND = 1 << 5;
        /// Task is idle
        const IDLE = 1 << 6;
        /// Priority inherited
        const PRIORITY_INHERITED = 1 << 7;
        /// Recently preempted
        const PREEMPTED = 1 << 8;
        /// Newly woken
        const NEWLY_WOKEN = 1 << 9;
    }
}

impl QueueEntry {
    /// Create new queue entry
    pub fn new(task_id: TaskId, priority: i16) -> Self {
        Self {
            task_id,
            effective_priority: priority,
            base_priority: priority,
            vruntime: 0,
            deadline: None,
            time_slice: Nanoseconds::from_millis(10),
            wait_ticks: 0,
            level: 0,
            flags: QueueEntryFlags::empty(),
        }
    }

    /// Set as real-time
    pub fn realtime(mut self, priority: i16) -> Self {
        self.effective_priority = priority;
        self.base_priority = priority;
        self.flags.insert(QueueEntryFlags::REALTIME);
        self.time_slice = Nanoseconds::from_millis(2);
        self
    }

    /// Set as interactive
    pub fn interactive(mut self) -> Self {
        self.flags.insert(QueueEntryFlags::INTERACTIVE);
        self.time_slice = Nanoseconds::from_millis(6);
        self
    }

    /// Set as batch
    pub fn batch(mut self) -> Self {
        self.flags.insert(QueueEntryFlags::BATCH);
        self.time_slice = Nanoseconds::from_millis(100);
        self
    }

    /// Set as background
    pub fn background(mut self) -> Self {
        self.flags.insert(QueueEntryFlags::BACKGROUND);
        self.time_slice = Nanoseconds::from_millis(200);
        self
    }

    /// Set deadline
    pub fn with_deadline(mut self, deadline: Nanoseconds) -> Self {
        self.deadline = Some(deadline);
        self.flags.insert(QueueEntryFlags::HAS_DEADLINE);
        self
    }

    /// Boost priority
    pub fn boost(&mut self, amount: i16) {
        self.effective_priority = self.effective_priority.saturating_add(amount);
        self.flags.insert(QueueEntryFlags::BOOSTED);
    }

    /// Reset priority to base
    pub fn reset_priority(&mut self) {
        self.effective_priority = self.base_priority;
        self.flags.remove(QueueEntryFlags::BOOSTED);
        self.flags.remove(QueueEntryFlags::PRIORITY_INHERITED);
    }

    /// Inherit priority
    pub fn inherit_priority(&mut self, priority: i16) {
        if priority > self.effective_priority {
            self.effective_priority = priority;
            self.flags.insert(QueueEntryFlags::PRIORITY_INHERITED);
        }
    }

    /// Age the entry (increase wait time)
    pub fn age(&mut self) {
        self.wait_ticks += 1;
    }

    /// Demote to next level
    pub fn demote(&mut self) {
        if self.level < 7 {
            self.level += 1;
        }
    }

    /// Promote to previous level
    pub fn promote(&mut self) {
        if self.level > 0 {
            self.level -= 1;
        }
    }
}

// =============================================================================
// Priority Queue (for RT tasks with deadlines)
// =============================================================================

/// Deadline-aware priority queue entry
#[derive(Debug, Clone)]
struct DeadlineEntry {
    task_id: TaskId,
    deadline: Nanoseconds,
    priority: i16,
}

impl PartialEq for DeadlineEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.task_id == other.task_id
    }
}

impl Eq for DeadlineEntry {}

impl PartialOrd for DeadlineEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeadlineEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Earlier deadline = higher priority (reverse for max-heap)
        other
            .deadline
            .cmp(&self.deadline)
            .then_with(|| other.priority.cmp(&self.priority))
    }
}

/// Deadline queue (earliest deadline first)
pub struct DeadlineQueue {
    heap: BinaryHeap<DeadlineEntry>,
    index: BTreeMap<TaskId, Nanoseconds>,
}

impl DeadlineQueue {
    /// Create new deadline queue
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            index: BTreeMap::new(),
        }
    }

    /// Insert task with deadline
    pub fn insert(&mut self, task_id: TaskId, deadline: Nanoseconds, priority: i16) {
        let entry = DeadlineEntry {
            task_id,
            deadline,
            priority,
        };
        self.heap.push(entry);
        self.index.insert(task_id, deadline);
    }

    /// Get next task (earliest deadline)
    pub fn peek(&self) -> Option<TaskId> {
        self.heap.peek().map(|e| e.task_id)
    }

    /// Pop next task
    pub fn pop(&mut self) -> Option<TaskId> {
        while let Some(entry) = self.heap.pop() {
            // Skip if task was removed/updated
            if let Some(&deadline) = self.index.get(&entry.task_id) {
                if deadline == entry.deadline {
                    self.index.remove(&entry.task_id);
                    return Some(entry.task_id);
                }
            }
        }
        None
    }

    /// Remove task
    pub fn remove(&mut self, task_id: TaskId) -> bool {
        self.index.remove(&task_id).is_some()
        // Note: Entry stays in heap but will be skipped in pop()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Update deadline
    pub fn update_deadline(&mut self, task_id: TaskId, new_deadline: Nanoseconds, priority: i16) {
        self.index.insert(task_id, new_deadline);
        self.heap.push(DeadlineEntry {
            task_id,
            deadline: new_deadline,
            priority,
        });
    }
}

impl Default for DeadlineQueue {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// FIFO Queue
// =============================================================================

/// Simple FIFO queue for a priority level
#[derive(Debug)]
pub struct FifoQueue {
    queue: VecDeque<QueueEntry>,
    count: AtomicU32,
}

impl FifoQueue {
    /// Create new FIFO queue
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            count: AtomicU32::new(0),
        }
    }

    /// Push to back
    pub fn push_back(&mut self, entry: QueueEntry) {
        self.queue.push_back(entry);
        self.count.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Push to front (high priority)
    pub fn push_front(&mut self, entry: QueueEntry) {
        self.queue.push_front(entry);
        self.count.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Pop from front
    pub fn pop_front(&mut self) -> Option<QueueEntry> {
        let entry = self.queue.pop_front();
        if entry.is_some() {
            self.count.fetch_sub(1, AtomicOrdering::Relaxed);
        }
        entry
    }

    /// Peek at front
    pub fn peek(&self) -> Option<&QueueEntry> {
        self.queue.front()
    }

    /// Remove specific task
    pub fn remove(&mut self, task_id: TaskId) -> Option<QueueEntry> {
        if let Some(pos) = self.queue.iter().position(|e| e.task_id == task_id) {
            let entry = self.queue.remove(pos);
            if entry.is_some() {
                self.count.fetch_sub(1, AtomicOrdering::Relaxed);
            }
            return entry;
        }
        None
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count.load(AtomicOrdering::Relaxed) == 0
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.count.load(AtomicOrdering::Relaxed) as usize
    }

    /// Age all entries
    pub fn age_all(&mut self) {
        for entry in &mut self.queue {
            entry.age();
        }
    }

    /// Get entries that need boosting
    pub fn get_aged_entries(&self, threshold: u64) -> Vec<TaskId> {
        self.queue
            .iter()
            .filter(|e| e.wait_ticks >= threshold)
            .map(|e| e.task_id)
            .collect()
    }
}

impl Default for FifoQueue {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Fair Queue (Virtual Runtime based)
// =============================================================================

/// Virtual runtime entry
#[derive(Debug, Clone)]
struct VruntimeEntry {
    task_id: TaskId,
    vruntime: u64,
}

impl PartialEq for VruntimeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.vruntime == other.vruntime && self.task_id == other.task_id
    }
}

impl Eq for VruntimeEntry {}

impl PartialOrd for VruntimeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VruntimeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower vruntime = higher priority
        other.vruntime.cmp(&self.vruntime)
    }
}

/// Fair queue using virtual runtime (CFS-like)
pub struct FairQueue {
    /// Priority queue (min-heap by vruntime)
    tree: BinaryHeap<VruntimeEntry>,
    /// Task -> entry mapping
    entries: BTreeMap<TaskId, QueueEntry>,
    /// Minimum vruntime
    min_vruntime: AtomicU64,
    /// Task count
    count: AtomicU32,
}

impl FairQueue {
    /// Create new fair queue
    pub fn new() -> Self {
        Self {
            tree: BinaryHeap::new(),
            entries: BTreeMap::new(),
            min_vruntime: AtomicU64::new(0),
            count: AtomicU32::new(0),
        }
    }

    /// Insert task
    pub fn insert(&mut self, mut entry: QueueEntry) {
        // New tasks start at minimum vruntime
        if entry.vruntime == 0 {
            entry.vruntime = self.min_vruntime.load(AtomicOrdering::Relaxed);
        }

        let ventry = VruntimeEntry {
            task_id: entry.task_id,
            vruntime: entry.vruntime,
        };

        self.tree.push(ventry);
        self.entries.insert(entry.task_id, entry);
        self.count.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Get next task (lowest vruntime)
    pub fn pick_next(&mut self) -> Option<QueueEntry> {
        while let Some(ventry) = self.tree.pop() {
            if let Some(entry) = self.entries.remove(&ventry.task_id) {
                // Check if vruntime matches (entry may have been updated)
                if entry.vruntime == ventry.vruntime {
                    self.count.fetch_sub(1, AtomicOrdering::Relaxed);
                    return Some(entry);
                } else {
                    // Put back if vruntime changed
                    self.entries.insert(entry.task_id, entry);
                }
            }
        }
        None
    }

    /// Peek at next task
    pub fn peek_next(&self) -> Option<&QueueEntry> {
        // Find first valid entry
        for ventry in self.tree.iter() {
            if let Some(entry) = self.entries.get(&ventry.task_id) {
                if entry.vruntime == ventry.vruntime {
                    return Some(entry);
                }
            }
        }
        None
    }

    /// Remove task
    pub fn remove(&mut self, task_id: TaskId) -> Option<QueueEntry> {
        if let Some(entry) = self.entries.remove(&task_id) {
            self.count.fetch_sub(1, AtomicOrdering::Relaxed);
            // Entry stays in tree but will be skipped
            return Some(entry);
        }
        None
    }

    /// Update task after running
    pub fn update_vruntime(&mut self, task_id: TaskId, delta: u64) {
        if let Some(entry) = self.entries.get_mut(&task_id) {
            entry.vruntime += delta;

            // Update minimum
            let min = self.min_vruntime.load(AtomicOrdering::Relaxed);
            if entry.vruntime > min {
                // Only update if we're behind
            } else {
                self.min_vruntime
                    .fetch_max(entry.vruntime, AtomicOrdering::Relaxed);
            }

            // Re-insert in tree
            self.tree.push(VruntimeEntry {
                task_id,
                vruntime: entry.vruntime,
            });
        }
    }

    /// Re-queue task after running
    pub fn requeue(&mut self, mut entry: QueueEntry, runtime: Nanoseconds) {
        // Update vruntime based on actual runtime
        let delta = runtime.raw() / 1_000_000; // Simplified weight
        entry.vruntime += delta;

        // Update minimum vruntime
        self.min_vruntime
            .fetch_max(entry.vruntime, AtomicOrdering::Relaxed);

        self.insert(entry);
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count.load(AtomicOrdering::Relaxed) == 0
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.count.load(AtomicOrdering::Relaxed) as usize
    }

    /// Get minimum vruntime
    pub fn min_vruntime(&self) -> u64 {
        self.min_vruntime.load(AtomicOrdering::Relaxed)
    }
}

impl Default for FairQueue {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Multi-Level Feedback Queue
// =============================================================================

/// Multi-level feedback queue
pub struct MultiLevelQueue {
    /// Levels (0 = highest priority)
    levels: Vec<Mutex<FifoQueue>>,
    /// Number of levels
    num_levels: usize,
    /// Time slices per level
    time_slices: Vec<Nanoseconds>,
    /// Task level tracking
    task_levels: RwLock<BTreeMap<TaskId, u8>>,
    /// Total task count
    count: AtomicU32,
}

impl MultiLevelQueue {
    /// Create new multi-level queue
    pub fn new(num_levels: usize) -> Self {
        let mut levels = Vec::with_capacity(num_levels);
        let mut time_slices = Vec::with_capacity(num_levels);

        for i in 0..num_levels {
            levels.push(Mutex::new(FifoQueue::new()));
            // Time slice doubles with each level
            time_slices.push(Nanoseconds::from_millis((10 << i) as u64));
        }

        Self {
            levels,
            num_levels,
            time_slices,
            task_levels: RwLock::new(BTreeMap::new()),
            count: AtomicU32::new(0),
        }
    }

    /// Insert task at level
    pub fn insert(&self, entry: QueueEntry, level: usize) {
        let level = level.min(self.num_levels - 1);
        self.levels[level].lock().push_back(entry.clone());
        self.task_levels.write().insert(entry.task_id, level as u8);
        self.count.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Get next task (from highest priority level)
    pub fn pop_next(&self) -> Option<QueueEntry> {
        for level in &self.levels {
            if let Some(entry) = level.lock().pop_front() {
                self.task_levels.write().remove(&entry.task_id);
                self.count.fetch_sub(1, AtomicOrdering::Relaxed);
                return Some(entry);
            }
        }
        None
    }

    /// Remove task from any level
    pub fn remove(&self, task_id: TaskId) -> Option<QueueEntry> {
        if let Some(level) = self.task_levels.write().remove(&task_id) {
            if let Some(entry) = self.levels[level as usize].lock().remove(task_id) {
                self.count.fetch_sub(1, AtomicOrdering::Relaxed);
                return Some(entry);
            }
        }
        None
    }

    /// Requeue task at next level (demotion)
    pub fn requeue_demote(&self, mut entry: QueueEntry) {
        let current_level = self
            .task_levels
            .read()
            .get(&entry.task_id)
            .copied()
            .unwrap_or(0);
        let new_level = ((current_level as usize) + 1).min(self.num_levels - 1);

        entry.level = new_level as u8;
        entry.time_slice = self.time_slices[new_level];

        self.insert(entry, new_level);
    }

    /// Requeue task at same level
    pub fn requeue_same(&self, entry: QueueEntry) {
        let level = self
            .task_levels
            .read()
            .get(&entry.task_id)
            .copied()
            .unwrap_or(0);
        self.insert(entry, level as usize);
    }

    /// Reset all tasks to level 0 (priority boost)
    pub fn boost_all(&self) {
        let mut all_entries = Vec::new();

        // Collect all entries
        for level in &self.levels {
            let mut queue = level.lock();
            while let Some(entry) = queue.pop_front() {
                all_entries.push(entry);
            }
        }

        self.task_levels.write().clear();
        self.count.store(0, AtomicOrdering::Relaxed);

        // Re-insert all at level 0
        for mut entry in all_entries {
            entry.level = 0;
            entry.time_slice = self.time_slices[0];
            entry.wait_ticks = 0;
            self.insert(entry, 0);
        }
    }

    /// Get time slice for level
    pub fn time_slice(&self, level: usize) -> Nanoseconds {
        self.time_slices
            .get(level)
            .copied()
            .unwrap_or(Nanoseconds::from_millis(10))
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count.load(AtomicOrdering::Relaxed) == 0
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.count.load(AtomicOrdering::Relaxed) as usize
    }

    /// Get count per level
    pub fn level_counts(&self) -> Vec<usize> {
        self.levels.iter().map(|l| l.lock().len()).collect()
    }
}

// =============================================================================
// Queue Manager
// =============================================================================

/// The main queue manager
pub struct QueueManager {
    /// Configuration
    config: RwLock<QueueConfig>,
    /// Real-time queues (priority -> queue)
    realtime: Vec<Mutex<FifoQueue>>,
    /// Deadline queue
    deadline: Mutex<DeadlineQueue>,
    /// Interactive queue (fair)
    interactive: Mutex<FairQueue>,
    /// Normal queue (MLFQ)
    normal: MultiLevelQueue,
    /// Batch queue
    batch: Mutex<FifoQueue>,
    /// Background queue
    background: Mutex<FifoQueue>,
    /// Idle queue
    idle: Mutex<FifoQueue>,
    /// Task to class mapping
    task_class: RwLock<BTreeMap<TaskId, QueueClass>>,
    /// Statistics
    stats: QueueStats,
    /// Last aging tick
    last_aging: AtomicU64,
    /// Tick counter
    tick_counter: AtomicU64,
}

/// Queue class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueClass {
    RealTime(u8), // Priority level
    Deadline,
    Interactive,
    Normal,
    Batch,
    Background,
    Idle,
}

/// Queue statistics
#[derive(Debug, Default)]
struct QueueStats {
    enqueues: AtomicU64,
    dequeues: AtomicU64,
    promotions: AtomicU64,
    demotions: AtomicU64,
    boosts: AtomicU64,
    inherits: AtomicU64,
}

impl QueueManager {
    /// Create new queue manager
    pub fn new(config: QueueConfig) -> Self {
        // Create RT queues
        let mut realtime = Vec::with_capacity(config.realtime_levels as usize);
        for _ in 0..config.realtime_levels {
            realtime.push(Mutex::new(FifoQueue::new()));
        }

        Self {
            realtime,
            deadline: Mutex::new(DeadlineQueue::new()),
            interactive: Mutex::new(FairQueue::new()),
            normal: MultiLevelQueue::new(config.normal_levels as usize),
            batch: Mutex::new(FifoQueue::new()),
            background: Mutex::new(FifoQueue::new()),
            idle: Mutex::new(FifoQueue::new()),
            task_class: RwLock::new(BTreeMap::new()),
            stats: QueueStats::default(),
            last_aging: AtomicU64::new(0),
            tick_counter: AtomicU64::new(0),
            config: RwLock::new(config),
        }
    }

    /// Enqueue task
    pub fn enqueue(&self, entry: QueueEntry, class: QueueClass) {
        self.task_class.write().insert(entry.task_id, class);
        self.stats.enqueues.fetch_add(1, AtomicOrdering::Relaxed);

        match class {
            QueueClass::RealTime(prio) => {
                let idx = (prio as usize).min(self.realtime.len() - 1);
                self.realtime[idx].lock().push_back(entry);
            },
            QueueClass::Deadline => {
                if let Some(deadline) = entry.deadline {
                    self.deadline
                        .lock()
                        .insert(entry.task_id, deadline, entry.effective_priority);
                }
            },
            QueueClass::Interactive => {
                self.interactive.lock().insert(entry);
            },
            QueueClass::Normal => {
                self.normal.insert(entry, 0);
            },
            QueueClass::Batch => {
                self.batch.lock().push_back(entry);
            },
            QueueClass::Background => {
                self.background.lock().push_back(entry);
            },
            QueueClass::Idle => {
                self.idle.lock().push_back(entry);
            },
        }
    }

    /// Dequeue next task
    pub fn dequeue(&self) -> Option<(QueueEntry, QueueClass)> {
        // 1. Check deadline queue first
        if let Some(task_id) = self.deadline.lock().pop() {
            if let Some(class) = self.task_class.write().remove(&task_id) {
                self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
                // Create synthetic entry
                return Some((QueueEntry::new(task_id, 99), class));
            }
        }

        // 2. Check real-time queues (highest priority first)
        for (prio, queue) in self.realtime.iter().enumerate().rev() {
            if let Some(entry) = queue.lock().pop_front() {
                self.task_class.write().remove(&entry.task_id);
                self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
                return Some((entry, QueueClass::RealTime(prio as u8)));
            }
        }

        // 3. Check interactive queue
        if let Some(entry) = self.interactive.lock().pick_next() {
            self.task_class.write().remove(&entry.task_id);
            self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
            return Some((entry, QueueClass::Interactive));
        }

        // 4. Check normal MLFQ
        if let Some(entry) = self.normal.pop_next() {
            self.task_class.write().remove(&entry.task_id);
            self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
            return Some((entry, QueueClass::Normal));
        }

        // 5. Check batch queue
        if let Some(entry) = self.batch.lock().pop_front() {
            self.task_class.write().remove(&entry.task_id);
            self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
            return Some((entry, QueueClass::Batch));
        }

        // 6. Check background queue
        if let Some(entry) = self.background.lock().pop_front() {
            self.task_class.write().remove(&entry.task_id);
            self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
            return Some((entry, QueueClass::Background));
        }

        // 7. Check idle queue
        if let Some(entry) = self.idle.lock().pop_front() {
            self.task_class.write().remove(&entry.task_id);
            self.stats.dequeues.fetch_add(1, AtomicOrdering::Relaxed);
            return Some((entry, QueueClass::Idle));
        }

        None
    }

    /// Remove task from any queue
    pub fn remove(&self, task_id: TaskId) -> bool {
        if let Some(class) = self.task_class.write().remove(&task_id) {
            match class {
                QueueClass::RealTime(prio) => {
                    let idx = (prio as usize).min(self.realtime.len() - 1);
                    return self.realtime[idx].lock().remove(task_id).is_some();
                },
                QueueClass::Deadline => {
                    return self.deadline.lock().remove(task_id);
                },
                QueueClass::Interactive => {
                    return self.interactive.lock().remove(task_id).is_some();
                },
                QueueClass::Normal => {
                    return self.normal.remove(task_id).is_some();
                },
                QueueClass::Batch => {
                    return self.batch.lock().remove(task_id).is_some();
                },
                QueueClass::Background => {
                    return self.background.lock().remove(task_id).is_some();
                },
                QueueClass::Idle => {
                    return self.idle.lock().remove(task_id).is_some();
                },
            }
        }
        false
    }

    /// Requeue task after running
    pub fn requeue(&self, entry: QueueEntry, class: QueueClass, demote: bool) {
        let task_id = entry.task_id;
        match class {
            QueueClass::Normal if demote => {
                self.stats.demotions.fetch_add(1, AtomicOrdering::Relaxed);
                self.normal.requeue_demote(entry);
                self.task_class.write().insert(task_id, class);
            },
            _ => {
                self.enqueue(entry, class);
            },
        }
    }

    /// Tick handler (for aging)
    pub fn tick(&self) {
        let tick = self.tick_counter.fetch_add(1, AtomicOrdering::Relaxed);

        let config = self.config.read();
        if !config.aging_enabled {
            return;
        }

        // Periodic aging
        if tick % config.aging_interval == 0 {
            // Age normal queue entries
            // Boost is handled separately
        }

        // Periodic boost (every N * aging_interval)
        if tick % (config.aging_interval * 10) == 0 {
            self.normal.boost_all();
            self.stats.boosts.fetch_add(1, AtomicOrdering::Relaxed);
        }
    }

    /// Inherit priority (for priority inheritance)
    pub fn inherit_priority(&self, task_id: TaskId, priority: i16) {
        let config = self.config.read();
        if !config.priority_inheritance {
            return;
        }

        if let Some(class) = self.task_class.read().get(&task_id) {
            // Only for normal tasks
            if *class == QueueClass::Normal {
                if let Some(mut entry) = self.normal.remove(task_id) {
                    entry.inherit_priority(priority);
                    self.normal.insert(entry, 0);
                    self.stats.inherits.fetch_add(1, AtomicOrdering::Relaxed);
                }
            }
        }
    }

    /// Reset inherited priority
    pub fn reset_priority(&self, task_id: TaskId) {
        if let Some(class) = self.task_class.read().get(&task_id) {
            if *class == QueueClass::Normal {
                if let Some(mut entry) = self.normal.remove(task_id) {
                    let level = entry.level;
                    entry.reset_priority();
                    self.normal.insert(entry, level as usize);
                }
            }
        }
    }

    /// Get queue statistics
    pub fn statistics(&self) -> QueueStatistics {
        QueueStatistics {
            enqueues: self.stats.enqueues.load(AtomicOrdering::Relaxed),
            dequeues: self.stats.dequeues.load(AtomicOrdering::Relaxed),
            promotions: self.stats.promotions.load(AtomicOrdering::Relaxed),
            demotions: self.stats.demotions.load(AtomicOrdering::Relaxed),
            boosts: self.stats.boosts.load(AtomicOrdering::Relaxed),
            inherits: self.stats.inherits.load(AtomicOrdering::Relaxed),
            rt_count: self.realtime.iter().map(|q| q.lock().len()).sum(),
            deadline_count: self.deadline.lock().len(),
            interactive_count: self.interactive.lock().len(),
            normal_counts: self.normal.level_counts(),
            batch_count: self.batch.lock().len(),
            background_count: self.background.lock().len(),
            idle_count: self.idle.lock().len(),
        }
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new(QueueConfig::default())
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStatistics {
    pub enqueues: u64,
    pub dequeues: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub boosts: u64,
    pub inherits: u64,
    pub rt_count: usize,
    pub deadline_count: usize,
    pub interactive_count: usize,
    pub normal_counts: Vec<usize>,
    pub batch_count: usize,
    pub background_count: usize,
    pub idle_count: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fifo_queue() {
        let mut queue = FifoQueue::new();

        queue.push_back(QueueEntry::new(TaskId::new(1), 10));
        queue.push_back(QueueEntry::new(TaskId::new(2), 20));

        assert_eq!(queue.len(), 2);

        let entry = queue.pop_front().unwrap();
        assert_eq!(entry.task_id, TaskId::new(1));

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_fair_queue() {
        let mut queue = FairQueue::new();

        let mut entry1 = QueueEntry::new(TaskId::new(1), 10);
        entry1.vruntime = 100;
        let mut entry2 = QueueEntry::new(TaskId::new(2), 10);
        entry2.vruntime = 50;

        queue.insert(entry1);
        queue.insert(entry2);

        // Lower vruntime should come first
        let first = queue.pick_next().unwrap();
        assert_eq!(first.task_id, TaskId::new(2));
    }

    #[test]
    fn test_deadline_queue() {
        let mut queue = DeadlineQueue::new();

        queue.insert(TaskId::new(1), Nanoseconds::from_millis(100), 50);
        queue.insert(TaskId::new(2), Nanoseconds::from_millis(50), 50);
        queue.insert(TaskId::new(3), Nanoseconds::from_millis(200), 50);

        // Earlier deadline should come first
        assert_eq!(queue.pop(), Some(TaskId::new(2)));
        assert_eq!(queue.pop(), Some(TaskId::new(1)));
        assert_eq!(queue.pop(), Some(TaskId::new(3)));
    }

    #[test]
    fn test_queue_manager() {
        let manager = QueueManager::default();

        let entry1 = QueueEntry::new(TaskId::new(1), 10).interactive();
        let entry2 = QueueEntry::new(TaskId::new(2), 90).realtime(90);

        manager.enqueue(entry1, QueueClass::Interactive);
        manager.enqueue(entry2, QueueClass::RealTime(90));

        // RT should come first
        let (first, class) = manager.dequeue().unwrap();
        assert!(matches!(class, QueueClass::RealTime(_)));

        let (second, class) = manager.dequeue().unwrap();
        assert_eq!(class, QueueClass::Interactive);
    }
}

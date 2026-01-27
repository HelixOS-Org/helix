//! # Scheduler Run Queues
//!
//! Various run queue implementations for schedulers.

use crate::ThreadId;
use super::Priority;
use super::traits::RunQueue;
use alloc::collections::{BinaryHeap, VecDeque, BTreeMap};
use alloc::vec::Vec;
use core::cmp::Ordering;

/// Thread entry in a queue
#[derive(Debug, Clone)]
pub struct QueueEntry {
    /// Thread ID
    pub id: ThreadId,
    /// Priority
    pub priority: Priority,
    /// Timestamp when added
    pub enqueue_time: u64,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier enqueue time
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.enqueue_time.cmp(&self.enqueue_time),
            ord => ord,
        }
    }
}

/// Simple FIFO run queue
pub struct FifoQueue {
    queue: VecDeque<ThreadId>,
}

impl FifoQueue {
    /// Create a new FIFO queue
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl Default for FifoQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl RunQueue for FifoQueue {
    fn enqueue(&mut self, id: ThreadId, _priority: Priority) {
        self.queue.push_back(id);
    }

    fn dequeue(&mut self) -> Option<ThreadId> {
        self.queue.pop_front()
    }

    fn peek(&self) -> Option<ThreadId> {
        self.queue.front().copied()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn remove(&mut self, id: ThreadId) -> bool {
        if let Some(pos) = self.queue.iter().position(|&x| x == id) {
            self.queue.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Priority queue based run queue
pub struct PriorityQueue {
    heap: BinaryHeap<QueueEntry>,
    time: u64,
}

impl PriorityQueue {
    /// Create a new priority queue
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            time: 0,
        }
    }
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl RunQueue for PriorityQueue {
    fn enqueue(&mut self, id: ThreadId, priority: Priority) {
        self.time += 1;
        self.heap.push(QueueEntry {
            id,
            priority,
            enqueue_time: self.time,
        });
    }

    fn dequeue(&mut self) -> Option<ThreadId> {
        self.heap.pop().map(|e| e.id)
    }

    fn peek(&self) -> Option<ThreadId> {
        self.heap.peek().map(|e| e.id)
    }

    fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    fn len(&self) -> usize {
        self.heap.len()
    }

    fn remove(&mut self, id: ThreadId) -> bool {
        let old_len = self.heap.len();
        let entries: Vec<_> = self.heap.drain().filter(|e| e.id != id).collect();
        for entry in entries {
            self.heap.push(entry);
        }
        self.heap.len() < old_len
    }
}

/// Multi-level feedback queue
pub struct MultilevelQueue {
    /// Queues by priority level
    levels: Vec<VecDeque<ThreadId>>,
    /// Number of levels
    num_levels: usize,
}

impl MultilevelQueue {
    /// Create a new multi-level queue
    pub fn new(num_levels: usize) -> Self {
        Self {
            levels: (0..num_levels).map(|_| VecDeque::new()).collect(),
            num_levels,
        }
    }

    /// Map priority to level
    fn priority_to_level(&self, priority: Priority) -> usize {
        let p = priority.static_priority() as usize;
        (p * self.num_levels / 140).min(self.num_levels - 1)
    }
}

impl RunQueue for MultilevelQueue {
    fn enqueue(&mut self, id: ThreadId, priority: Priority) {
        let level = self.priority_to_level(priority);
        self.levels[level].push_back(id);
    }

    fn dequeue(&mut self) -> Option<ThreadId> {
        for level in &mut self.levels {
            if let Some(id) = level.pop_front() {
                return Some(id);
            }
        }
        None
    }

    fn peek(&self) -> Option<ThreadId> {
        for level in &self.levels {
            if let Some(&id) = level.front() {
                return Some(id);
            }
        }
        None
    }

    fn is_empty(&self) -> bool {
        self.levels.iter().all(|l| l.is_empty())
    }

    fn len(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    fn remove(&mut self, id: ThreadId) -> bool {
        for level in &mut self.levels {
            if let Some(pos) = level.iter().position(|&x| x == id) {
                level.remove(pos);
                return true;
            }
        }
        false
    }
}

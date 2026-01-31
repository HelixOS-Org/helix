//! # Cognitive Queue Management
//!
//! Manages queues for cognitive processing.
//! Supports multiple queue types and policies.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// QUEUE TYPES
// ============================================================================

/// Queue item
#[derive(Debug, Clone)]
pub struct QueueItem<T> {
    /// Item ID
    pub id: u64,
    /// Item data
    pub data: T,
    /// Source domain
    pub source: DomainId,
    /// Enqueue time
    pub enqueue_time: Timestamp,
    /// Priority
    pub priority: u32,
    /// TTL (cycles)
    pub ttl: Option<u64>,
    /// Retry count
    pub retry_count: u32,
}

/// Queue policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePolicy {
    /// First in, first out
    Fifo,
    /// Last in, first out (stack)
    Lifo,
    /// Priority-based
    Priority,
    /// Fair queuing
    Fair,
}

/// Queue configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Queue name
    pub name: String,
    /// Maximum size
    pub max_size: usize,
    /// Queue policy
    pub policy: QueuePolicy,
    /// Maximum retries
    pub max_retries: u32,
    /// Retry delay (cycles)
    pub retry_delay: u64,
    /// Default TTL (cycles)
    pub default_ttl: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            name: "default".into(),
            max_size: 10000,
            policy: QueuePolicy::Fifo,
            max_retries: 3,
            retry_delay: 10,
            default_ttl: 1000,
        }
    }
}

/// Queue overflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Drop new items
    DropNew,
    /// Drop old items
    DropOld,
    /// Drop lowest priority
    DropLowest,
    /// Block (return error)
    Block,
}

// ============================================================================
// COGNITIVE QUEUE
// ============================================================================

/// A cognitive processing queue
pub struct CognitiveQueue<T: Clone> {
    /// Queue ID
    id: u64,
    /// Configuration
    config: QueueConfig,
    /// Items
    items: VecDeque<QueueItem<T>>,
    /// Retry queue
    retry_queue: Vec<(u64, QueueItem<T>)>, // (retry_at_cycle, item)
    /// Next item ID
    next_id: AtomicU64,
    /// Current cycle
    current_cycle: u64,
    /// Overflow policy
    overflow_policy: OverflowPolicy,
    /// Statistics
    stats: QueueStats,
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Total items enqueued
    pub total_enqueued: u64,
    /// Total items dequeued
    pub total_dequeued: u64,
    /// Total items expired
    pub total_expired: u64,
    /// Total items dropped
    pub total_dropped: u64,
    /// Total retries
    pub total_retries: u64,
    /// Current size
    pub current_size: u64,
    /// Peak size
    pub peak_size: u64,
    /// Average wait time (cycles)
    pub avg_wait_cycles: f32,
}

impl<T: Clone> CognitiveQueue<T> {
    /// Create a new queue
    pub fn new(id: u64, config: QueueConfig) -> Self {
        Self {
            id,
            config,
            items: VecDeque::new(),
            retry_queue: Vec::new(),
            next_id: AtomicU64::new(1),
            current_cycle: 0,
            overflow_policy: OverflowPolicy::DropOld,
            stats: QueueStats::default(),
        }
    }

    /// Enqueue an item
    pub fn enqueue(
        &mut self,
        data: T,
        source: DomainId,
        priority: u32,
    ) -> Result<u64, &'static str> {
        // Check capacity
        if self.items.len() >= self.config.max_size {
            match self.overflow_policy {
                OverflowPolicy::DropNew => {
                    self.stats.total_dropped += 1;
                    return Err("Queue full, item dropped");
                },
                OverflowPolicy::DropOld => {
                    self.items.pop_front();
                    self.stats.total_dropped += 1;
                },
                OverflowPolicy::DropLowest => {
                    // Find and remove lowest priority
                    let lowest = self
                        .items
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, item)| item.priority)
                        .map(|(i, _)| i);

                    if let Some(idx) = lowest {
                        self.items.remove(idx);
                        self.stats.total_dropped += 1;
                    }
                },
                OverflowPolicy::Block => {
                    return Err("Queue full");
                },
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let item = QueueItem {
            id,
            data,
            source,
            enqueue_time: Timestamp::now(),
            priority,
            ttl: Some(self.config.default_ttl),
            retry_count: 0,
        };

        // Insert based on policy
        match self.config.policy {
            QueuePolicy::Fifo | QueuePolicy::Fair => {
                self.items.push_back(item);
            },
            QueuePolicy::Lifo => {
                self.items.push_front(item);
            },
            QueuePolicy::Priority => {
                // Insert in priority order
                let pos = self
                    .items
                    .iter()
                    .position(|i| i.priority < priority)
                    .unwrap_or(self.items.len());
                self.items.insert(pos, item);
            },
        }

        self.stats.total_enqueued += 1;
        self.stats.current_size = self.items.len() as u64;
        if self.stats.current_size > self.stats.peak_size {
            self.stats.peak_size = self.stats.current_size;
        }

        Ok(id)
    }

    /// Dequeue an item
    pub fn dequeue(&mut self) -> Option<QueueItem<T>> {
        let item = match self.config.policy {
            QueuePolicy::Fifo | QueuePolicy::Lifo | QueuePolicy::Priority => self.items.pop_front(),
            QueuePolicy::Fair => {
                // Fair: round-robin by source
                self.dequeue_fair()
            },
        };

        if let Some(ref i) = item {
            let wait_cycles = self.current_cycle - i.enqueue_time.as_cycles();
            self.stats.avg_wait_cycles = (self.stats.avg_wait_cycles
                * self.stats.total_dequeued as f32
                + wait_cycles as f32)
                / (self.stats.total_dequeued + 1) as f32;

            self.stats.total_dequeued += 1;
            self.stats.current_size = self.items.len() as u64;
        }

        item
    }

    /// Fair dequeue - round-robin by source
    fn dequeue_fair(&mut self) -> Option<QueueItem<T>> {
        // Simple implementation: take from source with most items
        let sources: Vec<_> = self.items.iter().map(|i| i.source).collect();

        if sources.is_empty() {
            return None;
        }

        // Count items per source
        let mut counts: BTreeMap<DomainId, usize> = BTreeMap::new();
        for source in &sources {
            *counts.entry(*source).or_default() += 1;
        }

        // Get source with most items
        let max_source = counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(source, _)| *source);

        if let Some(source) = max_source {
            let pos = self.items.iter().position(|i| i.source == source);
            if let Some(p) = pos {
                return self.items.remove(p);
            }
        }

        self.items.pop_front()
    }

    /// Peek at front item
    pub fn peek(&self) -> Option<&QueueItem<T>> {
        self.items.front()
    }

    /// Retry an item
    pub fn retry(&mut self, mut item: QueueItem<T>) -> bool {
        if item.retry_count >= self.config.max_retries {
            self.stats.total_dropped += 1;
            return false;
        }

        item.retry_count += 1;
        let retry_at = self.current_cycle + self.config.retry_delay;
        self.retry_queue.push((retry_at, item));
        self.stats.total_retries += 1;

        true
    }

    /// Process retries
    pub fn process_retries(&mut self) {
        let current = self.current_cycle;

        // Move ready retries back to main queue
        let ready: Vec<_> = self
            .retry_queue
            .iter()
            .enumerate()
            .filter(|(_, (at, _))| *at <= current)
            .map(|(i, _)| i)
            .collect();

        // Remove in reverse order to maintain indices
        for idx in ready.into_iter().rev() {
            let (_, item) = self.retry_queue.remove(idx);
            // Re-enqueue with same priority
            let _ = self.enqueue(item.data, item.source, item.priority);
        }
    }

    /// Process tick - handle TTL and retries
    pub fn tick(&mut self) {
        self.current_cycle += 1;

        // Expire items
        let current = self.current_cycle;
        let initial_len = self.items.len();

        self.items.retain(|item| {
            if let Some(ttl) = item.ttl {
                let age = current - item.enqueue_time.as_cycles();
                age < ttl
            } else {
                true
            }
        });

        self.stats.total_expired += (initial_len - self.items.len()) as u64;
        self.stats.current_size = self.items.len() as u64;

        // Process retries
        self.process_retries();
    }

    /// Get queue ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get queue name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get retry queue length
    pub fn retry_len(&self) -> usize {
        self.retry_queue.len()
    }

    /// Set overflow policy
    pub fn set_overflow_policy(&mut self, policy: OverflowPolicy) {
        self.overflow_policy = policy;
    }

    /// Get statistics
    pub fn stats(&self) -> &QueueStats {
        &self.stats
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.items.clear();
        self.retry_queue.clear();
        self.stats.current_size = 0;
    }

    /// Get items by source
    pub fn items_by_source(&self, source: DomainId) -> Vec<&QueueItem<T>> {
        self.items.iter().filter(|i| i.source == source).collect()
    }
}

// ============================================================================
// QUEUE MANAGER
// ============================================================================

/// Manages multiple queues
pub struct QueueManager {
    /// Queue registry (type-erased via names)
    queues: BTreeMap<String, QueueInfo>,
    /// Next queue ID
    next_id: AtomicU64,
}

/// Queue information
#[derive(Debug, Clone)]
pub struct QueueInfo {
    /// Queue ID
    pub id: u64,
    /// Queue name
    pub name: String,
    /// Creation time
    pub created: Timestamp,
    /// Policy
    pub policy: QueuePolicy,
    /// Max size
    pub max_size: usize,
}

impl QueueManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Register a queue
    pub fn register(&mut self, name: &str, policy: QueuePolicy, max_size: usize) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let info = QueueInfo {
            id,
            name: name.into(),
            created: Timestamp::now(),
            policy,
            max_size,
        };

        self.queues.insert(name.into(), info);
        id
    }

    /// Unregister a queue
    pub fn unregister(&mut self, name: &str) -> bool {
        self.queues.remove(name).is_some()
    }

    /// Get queue info
    pub fn get_info(&self, name: &str) -> Option<&QueueInfo> {
        self.queues.get(name)
    }

    /// List all queues
    pub fn list(&self) -> Vec<&QueueInfo> {
        self.queues.values().collect()
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MULTI-LEVEL QUEUE
// ============================================================================

/// Multi-level feedback queue
pub struct MultiLevelQueue<T: Clone> {
    /// Levels
    levels: Vec<CognitiveQueue<T>>,
    /// Quantum per level (cycles)
    quantums: Vec<u64>,
    /// Current level
    current_level: usize,
    /// Items processed at current level
    processed_at_level: u64,
}

impl<T: Clone> MultiLevelQueue<T> {
    /// Create a new multi-level queue
    pub fn new(level_count: usize, base_quantum: u64) -> Self {
        let mut levels = Vec::with_capacity(level_count);
        let mut quantums = Vec::with_capacity(level_count);

        for i in 0..level_count {
            let config = QueueConfig {
                name: format!("level_{}", i),
                policy: QueuePolicy::Fifo,
                ..Default::default()
            };
            levels.push(CognitiveQueue::new(i as u64, config));

            // Each level has longer quantum
            quantums.push(base_quantum * (1 << i));
        }

        Self {
            levels,
            quantums,
            current_level: 0,
            processed_at_level: 0,
        }
    }

    /// Enqueue to highest priority level
    pub fn enqueue(&mut self, data: T, source: DomainId) -> Result<u64, &'static str> {
        if self.levels.is_empty() {
            return Err("No levels configured");
        }
        self.levels[0].enqueue(data, source, 100)
    }

    /// Dequeue with feedback
    pub fn dequeue(&mut self) -> Option<(QueueItem<T>, usize)> {
        // Try current level first
        for level in self.current_level..self.levels.len() {
            if let Some(item) = self.levels[level].dequeue() {
                self.processed_at_level += 1;

                // Check if quantum exhausted
                if self.processed_at_level >= self.quantums[level] {
                    self.current_level = (level + 1) % self.levels.len();
                    self.processed_at_level = 0;
                }

                return Some((item, level));
            }
        }

        // Wrap around to lower levels
        for level in 0..self.current_level {
            if let Some(item) = self.levels[level].dequeue() {
                return Some((item, level));
            }
        }

        None
    }

    /// Demote an item to lower priority level
    pub fn demote(&mut self, item: QueueItem<T>, from_level: usize) -> Result<u64, &'static str> {
        let next_level = (from_level + 1).min(self.levels.len() - 1);
        self.levels[next_level].enqueue(item.data, item.source, item.priority)
    }

    /// Promote an item to higher priority level
    pub fn promote(&mut self, item: QueueItem<T>, from_level: usize) -> Result<u64, &'static str> {
        let prev_level = from_level.saturating_sub(1);
        self.levels[prev_level].enqueue(item.data, item.source, item.priority)
    }

    /// Get level count
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Get total items
    pub fn total_items(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Get items per level
    pub fn items_per_level(&self) -> Vec<usize> {
        self.levels.iter().map(|l| l.len()).collect()
    }

    /// Tick all levels
    pub fn tick(&mut self) {
        for level in &mut self.levels {
            level.tick();
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fifo_queue() {
        let config = QueueConfig {
            name: "test".into(),
            policy: QueuePolicy::Fifo,
            ..Default::default()
        };
        let mut queue: CognitiveQueue<i32> = CognitiveQueue::new(1, config);

        let domain = DomainId::new(1);
        queue.enqueue(1, domain, 100).unwrap();
        queue.enqueue(2, domain, 100).unwrap();
        queue.enqueue(3, domain, 100).unwrap();

        assert_eq!(queue.dequeue().unwrap().data, 1);
        assert_eq!(queue.dequeue().unwrap().data, 2);
        assert_eq!(queue.dequeue().unwrap().data, 3);
    }

    #[test]
    fn test_priority_queue() {
        let config = QueueConfig {
            name: "test".into(),
            policy: QueuePolicy::Priority,
            ..Default::default()
        };
        let mut queue: CognitiveQueue<i32> = CognitiveQueue::new(1, config);

        let domain = DomainId::new(1);
        queue.enqueue(1, domain, 50).unwrap();
        queue.enqueue(2, domain, 100).unwrap();
        queue.enqueue(3, domain, 75).unwrap();

        // Highest priority first
        assert_eq!(queue.dequeue().unwrap().data, 2);
        assert_eq!(queue.dequeue().unwrap().data, 3);
        assert_eq!(queue.dequeue().unwrap().data, 1);
    }

    #[test]
    fn test_multi_level_queue() {
        let mut mlq: MultiLevelQueue<i32> = MultiLevelQueue::new(3, 10);

        let domain = DomainId::new(1);
        mlq.enqueue(1, domain).unwrap();
        mlq.enqueue(2, domain).unwrap();

        let (item, level) = mlq.dequeue().unwrap();
        assert_eq!(item.data, 1);
        assert_eq!(level, 0);

        // Demote
        mlq.demote(item, 0).unwrap();

        // Next dequeue should be item 2 (still at level 0)
        let (item2, level2) = mlq.dequeue().unwrap();
        assert_eq!(item2.data, 2);
        assert_eq!(level2, 0);
    }

    #[test]
    fn test_overflow_policy() {
        let config = QueueConfig {
            name: "test".into(),
            max_size: 2,
            policy: QueuePolicy::Fifo,
            ..Default::default()
        };
        let mut queue: CognitiveQueue<i32> = CognitiveQueue::new(1, config);
        queue.set_overflow_policy(OverflowPolicy::DropOld);

        let domain = DomainId::new(1);
        queue.enqueue(1, domain, 100).unwrap();
        queue.enqueue(2, domain, 100).unwrap();
        queue.enqueue(3, domain, 100).unwrap(); // Should drop 1

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.dequeue().unwrap().data, 2);
    }
}

//! # Memory Buffer
//!
//! Implements efficient memory buffering with multiple eviction strategies.
//! Supports working memory management.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// BUFFER TYPES
// ============================================================================

/// Buffer entry
#[derive(Debug, Clone)]
pub struct BufferEntry<T> {
    /// Entry ID
    pub id: u64,
    /// Key
    pub key: String,
    /// Value
    pub value: T,
    /// Priority
    pub priority: f64,
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Access count
    pub access_count: u64,
    /// Size in bytes
    pub size: usize,
}

/// Eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    Lru,      // Least Recently Used
    Lfu,      // Least Frequently Used
    Fifo,     // First In First Out
    Priority, // By priority
    Size,     // Largest first
    Random,   // Random eviction
}

/// Buffer statistics
#[derive(Debug, Clone, Default)]
pub struct BufferStats {
    /// Hits
    pub hits: u64,
    /// Misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Insertions
    pub insertions: u64,
}

impl BufferStats {
    /// Hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Buffer configuration
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum capacity (entries)
    pub max_capacity: usize,
    /// Maximum size (bytes)
    pub max_size: usize,
    /// Eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Eviction batch size
    pub eviction_batch: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            max_size: 1024 * 1024, // 1MB
            eviction_policy: EvictionPolicy::Lru,
            eviction_batch: 10,
        }
    }
}

// ============================================================================
// MEMORY BUFFER
// ============================================================================

/// Generic memory buffer
pub struct MemoryBuffer<T: Clone> {
    /// Entries by key
    entries: BTreeMap<String, BufferEntry<T>>,
    /// Key order for FIFO
    key_order: Vec<String>,
    /// Current size
    current_size: usize,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: BufferConfig,
    /// Statistics
    stats: BufferStats,
}

impl<T: Clone> MemoryBuffer<T> {
    /// Create new buffer
    pub fn new(config: BufferConfig) -> Self {
        Self {
            entries: BTreeMap::new(),
            key_order: Vec::new(),
            current_size: 0,
            next_id: AtomicU64::new(1),
            config,
            stats: BufferStats::default(),
        }
    }

    /// Get value
    pub fn get(&mut self, key: &str) -> Option<&T> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = Timestamp::now();
            entry.access_count += 1;
            self.stats.hits += 1;
            Some(&entry.value)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Get entry
    pub fn get_entry(&mut self, key: &str) -> Option<&BufferEntry<T>> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = Timestamp::now();
            entry.access_count += 1;
            self.stats.hits += 1;
            Some(entry)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Put value
    pub fn put(&mut self, key: &str, value: T, size: usize) -> u64 {
        self.put_with_priority(key, value, size, 1.0)
    }

    /// Put with priority
    pub fn put_with_priority(&mut self, key: &str, value: T, size: usize, priority: f64) -> u64 {
        // Check if need to evict
        while self.entries.len() >= self.config.max_capacity
            || self.current_size + size > self.config.max_size
        {
            if !self.evict() {
                break;
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        // Update if exists
        if let Some(entry) = self.entries.get_mut(key) {
            self.current_size -= entry.size;
            entry.value = value;
            entry.size = size;
            entry.priority = priority;
            entry.last_accessed = now;
            self.current_size += size;
            return entry.id;
        }

        let entry = BufferEntry {
            id,
            key: key.into(),
            value,
            priority,
            created: now,
            last_accessed: now,
            access_count: 0,
            size,
        };

        self.entries.insert(key.into(), entry);
        self.key_order.push(key.into());
        self.current_size += size;
        self.stats.insertions += 1;

        id
    }

    /// Remove entry
    pub fn remove(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size -= entry.size;
            self.key_order.retain(|k| k != key);
            Some(entry.value)
        } else {
            None
        }
    }

    /// Evict one entry
    fn evict(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::Lru => self.find_lru(),
            EvictionPolicy::Lfu => self.find_lfu(),
            EvictionPolicy::Fifo => self.find_fifo(),
            EvictionPolicy::Priority => self.find_lowest_priority(),
            EvictionPolicy::Size => self.find_largest(),
            EvictionPolicy::Random => self.find_random(),
        };

        if let Some(key) = key_to_evict {
            self.remove(&key);
            self.stats.evictions += 1;
            true
        } else {
            false
        }
    }

    fn find_lru(&self) -> Option<String> {
        self.entries
            .values()
            .min_by_key(|e| e.last_accessed.0)
            .map(|e| e.key.clone())
    }

    fn find_lfu(&self) -> Option<String> {
        self.entries
            .values()
            .min_by_key(|e| e.access_count)
            .map(|e| e.key.clone())
    }

    fn find_fifo(&self) -> Option<String> {
        self.key_order.first().cloned()
    }

    fn find_lowest_priority(&self) -> Option<String> {
        self.entries
            .values()
            .min_by(|a, b| {
                a.priority
                    .partial_cmp(&b.priority)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|e| e.key.clone())
    }

    fn find_largest(&self) -> Option<String> {
        self.entries
            .values()
            .max_by_key(|e| e.size)
            .map(|e| e.key.clone())
    }

    fn find_random(&self) -> Option<String> {
        // Pseudo-random for no_std
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        let idx = (self.next_id.load(Ordering::Relaxed) as usize * 7919) % len;
        self.entries.keys().nth(idx).cloned()
    }

    /// Update priority
    pub fn update_priority(&mut self, key: &str, priority: f64) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.priority = priority;
        }
    }

    /// Contains key
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Current size
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current byte size
    pub fn byte_size(&self) -> usize {
        self.current_size
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.entries.clear();
        self.key_order.clear();
        self.current_size = 0;
    }

    /// Get statistics
    pub fn stats(&self) -> &BufferStats {
        &self.stats
    }

    /// List keys
    pub fn keys(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }
}

// ============================================================================
// RING BUFFER
// ============================================================================

/// Fixed-size ring buffer
pub struct RingBuffer<T: Clone> {
    /// Data
    data: Vec<Option<T>>,
    /// Write position
    write_pos: usize,
    /// Read position
    read_pos: usize,
    /// Count
    count: usize,
    /// Capacity
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// Create new ring buffer
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(None);
        }

        Self {
            data,
            write_pos: 0,
            read_pos: 0,
            count: 0,
            capacity,
        }
    }

    /// Push value
    pub fn push(&mut self, value: T) {
        self.data[self.write_pos] = Some(value);
        self.write_pos = (self.write_pos + 1) % self.capacity;

        if self.count < self.capacity {
            self.count += 1;
        } else {
            self.read_pos = (self.read_pos + 1) % self.capacity;
        }
    }

    /// Pop value
    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        let value = self.data[self.read_pos].take();
        self.read_pos = (self.read_pos + 1) % self.capacity;
        self.count -= 1;

        value
    }

    /// Peek at front
    pub fn peek(&self) -> Option<&T> {
        if self.count == 0 {
            None
        } else {
            self.data[self.read_pos].as_ref()
        }
    }

    /// Get at index
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.count {
            return None;
        }

        let actual_idx = (self.read_pos + index) % self.capacity;
        self.data[actual_idx].as_ref()
    }

    /// Current count
    pub fn len(&self) -> usize {
        self.count
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Is full
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }

    /// Clear
    pub fn clear(&mut self) {
        for item in &mut self.data {
            *item = None;
        }
        self.write_pos = 0;
        self.read_pos = 0;
        self.count = 0;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let mut buffer: MemoryBuffer<i32> = MemoryBuffer::new(BufferConfig::default());

        buffer.put("key1", 42, 4);
        let value = buffer.get("key1");

        assert_eq!(value, Some(&42));
    }

    #[test]
    fn test_eviction_lru() {
        let config = BufferConfig {
            max_capacity: 3,
            eviction_policy: EvictionPolicy::Lru,
            ..Default::default()
        };

        let mut buffer: MemoryBuffer<i32> = MemoryBuffer::new(config);

        buffer.put("a", 1, 4);
        buffer.put("b", 2, 4);
        buffer.put("c", 3, 4);

        // Access a and c
        buffer.get("a");
        buffer.get("c");

        // Add new - should evict b (LRU)
        buffer.put("d", 4, 4);

        assert!(buffer.contains("a"));
        assert!(!buffer.contains("b"));
        assert!(buffer.contains("c"));
        assert!(buffer.contains("d"));
    }

    #[test]
    fn test_eviction_fifo() {
        let config = BufferConfig {
            max_capacity: 3,
            eviction_policy: EvictionPolicy::Fifo,
            ..Default::default()
        };

        let mut buffer: MemoryBuffer<i32> = MemoryBuffer::new(config);

        buffer.put("a", 1, 4);
        buffer.put("b", 2, 4);
        buffer.put("c", 3, 4);

        // Add new - should evict a (FIFO)
        buffer.put("d", 4, 4);

        assert!(!buffer.contains("a"));
        assert!(buffer.contains("b"));
        assert!(buffer.contains("c"));
        assert!(buffer.contains("d"));
    }

    #[test]
    fn test_priority_eviction() {
        let config = BufferConfig {
            max_capacity: 3,
            eviction_policy: EvictionPolicy::Priority,
            ..Default::default()
        };

        let mut buffer: MemoryBuffer<i32> = MemoryBuffer::new(config);

        buffer.put_with_priority("a", 1, 4, 0.5);
        buffer.put_with_priority("b", 2, 4, 0.1); // Lowest priority
        buffer.put_with_priority("c", 3, 4, 0.9);

        // Add new - should evict b (lowest priority)
        buffer.put_with_priority("d", 4, 4, 0.7);

        assert!(buffer.contains("a"));
        assert!(!buffer.contains("b"));
        assert!(buffer.contains("c"));
        assert!(buffer.contains("d"));
    }

    #[test]
    fn test_hit_rate() {
        let mut buffer: MemoryBuffer<i32> = MemoryBuffer::new(BufferConfig::default());

        buffer.put("a", 1, 4);

        buffer.get("a"); // Hit
        buffer.get("a"); // Hit
        buffer.get("b"); // Miss

        let stats = buffer.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_ring_buffer() {
        let mut ring: RingBuffer<i32> = RingBuffer::new(3);

        ring.push(1);
        ring.push(2);
        ring.push(3);

        assert_eq!(ring.len(), 3);
        assert!(ring.is_full());

        // Overwrites oldest
        ring.push(4);

        assert_eq!(ring.pop(), Some(2));
        assert_eq!(ring.pop(), Some(3));
        assert_eq!(ring.pop(), Some(4));
        assert!(ring.is_empty());
    }

    #[test]
    fn test_ring_buffer_peek() {
        let mut ring: RingBuffer<i32> = RingBuffer::new(5);

        ring.push(10);
        ring.push(20);
        ring.push(30);

        assert_eq!(ring.peek(), Some(&10));
        assert_eq!(ring.get(1), Some(&20));
        assert_eq!(ring.get(2), Some(&30));
    }
}

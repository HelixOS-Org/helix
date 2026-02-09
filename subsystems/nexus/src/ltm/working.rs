//! Working Memory
//!
//! This module provides short-term memory for active processing.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{MemoryId, PatternId, Timestamp};

/// Working memory item
#[derive(Debug, Clone)]
pub struct WorkingMemoryItem {
    /// Item ID
    pub id: MemoryId,
    /// Created timestamp
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Content
    pub content: WorkingMemoryContent,
    /// Priority (1-10)
    pub priority: u8,
    /// Time to live (nanoseconds, 0 = forever)
    pub ttl_ns: u64,
}

impl WorkingMemoryItem {
    /// Check if expired
    #[inline(always)]
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.ttl_ns > 0 && self.created.0 + self.ttl_ns < current_time
    }
}

/// Working memory content
#[derive(Debug, Clone)]
pub enum WorkingMemoryContent {
    /// Current event being processed
    CurrentEvent {
        event_type: String,
        data: BTreeMap<String, String>,
    },
    /// Recent decision
    RecentDecision { decision: String, outcome: String },
    /// Active pattern match
    ActivePattern {
        pattern_id: PatternId,
        match_score: f32,
    },
    /// Pending action
    PendingAction { action: String, deadline: Timestamp },
    /// Context variable
    Context { key: String, value: String },
}

impl WorkingMemoryContent {
    /// Create current event
    #[inline(always)]
    pub fn event(event_type: String, data: BTreeMap<String, String>) -> Self {
        Self::CurrentEvent { event_type, data }
    }

    /// Create decision
    #[inline(always)]
    pub fn decision(decision: String, outcome: String) -> Self {
        Self::RecentDecision { decision, outcome }
    }

    /// Create pattern match
    #[inline]
    pub fn pattern(pattern_id: PatternId, match_score: f32) -> Self {
        Self::ActivePattern {
            pattern_id,
            match_score,
        }
    }

    /// Create pending action
    #[inline(always)]
    pub fn action(action: String, deadline: Timestamp) -> Self {
        Self::PendingAction { action, deadline }
    }

    /// Create context
    #[inline(always)]
    pub fn context(key: String, value: String) -> Self {
        Self::Context { key, value }
    }
}

/// Working memory
pub struct WorkingMemory {
    /// Items
    items: BTreeMap<MemoryId, WorkingMemoryItem>,
    /// Counter
    counter: AtomicU64,
    /// Max items
    max_items: usize,
    /// Current time (for TTL checks)
    current_time: AtomicU64,
}

impl WorkingMemory {
    /// Create new working memory
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
            counter: AtomicU64::new(0),
            max_items: 1000,
            current_time: AtomicU64::new(0),
        }
    }

    /// Store item
    pub fn store(&mut self, content: WorkingMemoryContent, priority: u8, ttl_ns: u64) -> MemoryId {
        let id = MemoryId(self.counter.fetch_add(1, Ordering::Relaxed));
        let now = Timestamp::new(self.current_time.load(Ordering::Relaxed));

        let item = WorkingMemoryItem {
            id,
            created: now,
            last_accessed: now,
            content,
            priority,
            ttl_ns,
        };

        self.items.insert(id, item);
        self.maybe_evict();

        id
    }

    /// Get item
    #[inline]
    pub fn get(&mut self, id: MemoryId) -> Option<&WorkingMemoryItem> {
        if let Some(item) = self.items.get_mut(&id) {
            item.last_accessed = Timestamp::new(self.current_time.load(Ordering::Relaxed));
            return Some(item);
        }
        None
    }

    /// Get item immutably
    #[inline(always)]
    pub fn peek(&self, id: MemoryId) -> Option<&WorkingMemoryItem> {
        self.items.get(&id)
    }

    /// Remove item
    #[inline(always)]
    pub fn remove(&mut self, id: MemoryId) -> Option<WorkingMemoryItem> {
        self.items.remove(&id)
    }

    /// Update time
    #[inline(always)]
    pub fn update_time(&mut self, time_ns: u64) {
        self.current_time.store(time_ns, Ordering::Relaxed);
        self.expire_items();
    }

    /// Expire items past TTL
    fn expire_items(&mut self) {
        let now = self.current_time.load(Ordering::Relaxed);
        let to_remove: Vec<_> = self
            .items
            .iter()
            .filter(|(_, item)| item.is_expired(now))
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            self.items.remove(&id);
        }
    }

    /// Evict low priority items
    fn maybe_evict(&mut self) {
        if self.items.len() <= self.max_items {
            return;
        }

        // Sort by priority (asc), then last_accessed (asc)
        let mut items: Vec<_> = self
            .items
            .iter()
            .map(|(id, item)| (*id, item.priority, item.last_accessed))
            .collect();

        items.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));

        // Remove 10%
        let to_remove = self.max_items / 10;
        for (id, _, _) in items.into_iter().take(to_remove) {
            self.items.remove(&id);
        }
    }

    /// Item count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// All items
    #[inline(always)]
    pub fn all_items(&self) -> impl Iterator<Item = &WorkingMemoryItem> {
        self.items.values()
    }

    /// Find by content type - patterns
    #[inline]
    pub fn find_patterns(&self) -> Vec<&WorkingMemoryItem> {
        self.items
            .values()
            .filter(|i| matches!(i.content, WorkingMemoryContent::ActivePattern { .. }))
            .collect()
    }

    /// Find by content type - events
    #[inline]
    pub fn find_events(&self) -> Vec<&WorkingMemoryItem> {
        self.items
            .values()
            .filter(|i| matches!(i.content, WorkingMemoryContent::CurrentEvent { .. }))
            .collect()
    }

    /// Find by content type - decisions
    #[inline]
    pub fn find_decisions(&self) -> Vec<&WorkingMemoryItem> {
        self.items
            .values()
            .filter(|i| matches!(i.content, WorkingMemoryContent::RecentDecision { .. }))
            .collect()
    }

    /// Find by content type - pending actions
    #[inline]
    pub fn find_pending_actions(&self) -> Vec<&WorkingMemoryItem> {
        self.items
            .values()
            .filter(|i| matches!(i.content, WorkingMemoryContent::PendingAction { .. }))
            .collect()
    }

    /// Get context value
    #[inline]
    pub fn get_context(&self, key: &str) -> Option<&str> {
        for item in self.items.values() {
            if let WorkingMemoryContent::Context { key: k, value } = &item.content {
                if k == key {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Clear all items
    #[inline(always)]
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for WorkingMemory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WorkingMemory")
            .field("items", &self.items.len())
            .field("max_items", &self.max_items)
            .finish()
    }
}

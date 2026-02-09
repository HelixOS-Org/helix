//! Priority event queue

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::event::NexusEvent;
use super::types::EventPriority;

// ============================================================================
// EVENT QUEUE
// ============================================================================

/// Priority event queue
#[repr(align(64))]
pub struct EventQueue {
    /// Queues by priority level
    queues: [Vec<NexusEvent>; 6],
    /// Maximum queue size
    max_size: usize,
    /// Events dropped due to overflow
    dropped: u64,
    /// Total events processed
    processed: u64,
}

impl EventQueue {
    /// Create a new event queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queues: Default::default(),
            max_size,
            dropped: 0,
            processed: 0,
        }
    }

    /// Push an event to the queue
    #[inline]
    pub fn push(&mut self, event: NexusEvent) -> bool {
        let queue = &mut self.queues[event.priority as usize];

        if queue.len() >= self.max_size {
            self.dropped += 1;
            return false;
        }

        queue.push(event);
        true
    }

    /// Pop the highest priority event
    #[inline]
    pub fn pop(&mut self) -> Option<NexusEvent> {
        // Start from highest priority
        for priority in (0..6).rev() {
            if let Some(event) = self.queues[priority].pop() {
                self.processed += 1;
                return Some(event);
            }
        }
        None
    }

    /// Get total pending events
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }

    /// Check if queue is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.queues.iter().all(|q| q.is_empty())
    }

    /// Get pending events by priority
    #[inline(always)]
    pub fn pending_by_priority(&self, priority: EventPriority) -> usize {
        self.queues[priority as usize].len()
    }

    /// Get total dropped events
    #[inline(always)]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Get total processed events
    #[inline(always)]
    pub fn processed(&self) -> u64 {
        self.processed
    }

    /// Clear all events
    #[inline]
    pub fn clear(&mut self) {
        for queue in &mut self.queues {
            queue.clear();
        }
    }
}

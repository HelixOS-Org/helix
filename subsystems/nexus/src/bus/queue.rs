//! Message Queue
//!
//! Priority queue implementation for messages.

#![allow(dead_code)]

use alloc::vec::Vec;

use super::message::{Message, MessagePriority};
use crate::types::Timestamp;

// ============================================================================
// MESSAGE QUEUE
// ============================================================================

/// Priority queue for messages
pub struct MessageQueue {
    /// Queues by priority (0=Low, 1=Normal, 2=High, 3=Urgent, 4=Critical)
    queues: [Vec<Message>; 5],
    /// Total message count
    count: usize,
    /// Max queue size
    max_size: usize,
}

impl MessageQueue {
    /// Create new queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queues: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            count: 0,
            max_size,
        }
    }

    /// Push message (returns true if accepted)
    pub fn push(&mut self, message: Message) -> bool {
        if self.count >= self.max_size {
            // Drop lowest priority message if full
            if !self.drop_lowest() {
                return false;
            }
        }

        let idx = message.priority as usize;
        self.queues[idx].push(message);
        self.count += 1;
        true
    }

    /// Drop lowest priority message to make room
    fn drop_lowest(&mut self) -> bool {
        for queue in &mut self.queues {
            if !queue.is_empty() {
                queue.remove(0);
                self.count -= 1;
                return true;
            }
        }
        false
    }

    /// Pop highest priority message
    pub fn pop(&mut self) -> Option<Message> {
        // Check from highest to lowest priority
        for queue in self.queues.iter_mut().rev() {
            if !queue.is_empty() {
                self.count -= 1;
                return Some(queue.remove(0));
            }
        }
        None
    }

    /// Pop all messages at or above given priority
    pub fn pop_priority(&mut self, min_priority: MessagePriority) -> Vec<Message> {
        let mut result = Vec::new();
        let min_idx = min_priority as usize;

        for idx in (min_idx..5).rev() {
            while !self.queues[idx].is_empty() {
                self.count -= 1;
                result.push(self.queues[idx].remove(0));
            }
        }

        result
    }

    /// Peek at highest priority message
    pub fn peek(&self) -> Option<&Message> {
        for queue in self.queues.iter().rev() {
            if !queue.is_empty() {
                return queue.first();
            }
        }
        None
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.count
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get count at priority level
    pub fn len_at_priority(&self, priority: MessagePriority) -> usize {
        self.queues[priority as usize].len()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        for queue in &mut self.queues {
            queue.clear();
        }
        self.count = 0;
    }

    /// Expire old messages
    pub fn expire(&mut self, now: Timestamp) -> usize {
        let mut expired = 0;
        for queue in &mut self.queues {
            let before = queue.len();
            queue.retain(|m| !m.is_expired(now));
            expired += before - queue.len();
        }
        self.count -= expired;
        expired
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.max_size.saturating_sub(self.count)
    }

    /// Drain all messages
    pub fn drain(&mut self) -> Vec<Message> {
        let mut result = Vec::with_capacity(self.count);
        for queue in self.queues.iter_mut().rev() {
            result.append(queue);
        }
        self.count = 0;
        result
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new(10000)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::domain::Domain;
    use super::super::message::MessagePayload;
    use super::*;

    fn make_msg(priority: MessagePriority) -> Message {
        Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        )
        .with_priority(priority)
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = MessageQueue::new(100);

        queue.push(make_msg(MessagePriority::Low));
        queue.push(make_msg(MessagePriority::High));
        queue.push(make_msg(MessagePriority::Normal));

        // Should get high priority first
        assert_eq!(queue.pop().unwrap().priority, MessagePriority::High);
        assert_eq!(queue.pop().unwrap().priority, MessagePriority::Normal);
        assert_eq!(queue.pop().unwrap().priority, MessagePriority::Low);
    }

    #[test]
    fn test_capacity() {
        let mut queue = MessageQueue::new(3);

        queue.push(make_msg(MessagePriority::Low));
        queue.push(make_msg(MessagePriority::Low));
        queue.push(make_msg(MessagePriority::Low));

        assert_eq!(queue.len(), 3);

        // Should drop lowest priority to make room
        queue.push(make_msg(MessagePriority::High));
        assert_eq!(queue.len(), 3);

        // First message should be high priority
        assert_eq!(queue.pop().unwrap().priority, MessagePriority::High);
    }

    #[test]
    fn test_pop_priority() {
        let mut queue = MessageQueue::new(100);

        queue.push(make_msg(MessagePriority::Low));
        queue.push(make_msg(MessagePriority::Normal));
        queue.push(make_msg(MessagePriority::High));
        queue.push(make_msg(MessagePriority::Urgent));

        let urgent = queue.pop_priority(MessagePriority::High);
        assert_eq!(urgent.len(), 2); // High + Urgent
        assert_eq!(queue.len(), 2); // Low + Normal remaining
    }

    #[test]
    fn test_drain() {
        let mut queue = MessageQueue::new(100);

        queue.push(make_msg(MessagePriority::Low));
        queue.push(make_msg(MessagePriority::High));

        let all = queue.drain();
        assert_eq!(all.len(), 2);
        assert!(queue.is_empty());
    }
}

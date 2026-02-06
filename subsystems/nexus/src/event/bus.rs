//! Event bus for distributing events

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::event::NexusEvent;
use super::handler::{EventHandler, EventHandlerResult, EventSubscription};
use super::kind::NexusEventKind;
use super::queue::EventQueue;

// ============================================================================
// EVENT BUS STATS
// ============================================================================

/// Event bus statistics
#[derive(Debug, Clone)]
pub struct EventBusStats {
    /// Total events processed
    pub events_processed: u64,
    /// Total events delivered to handlers
    pub events_delivered: u64,
    /// Total events dropped
    pub events_dropped: u64,
    /// Current pending events
    pub pending: usize,
    /// Number of registered handlers
    pub handlers: usize,
}

// ============================================================================
// EVENT BUS
// ============================================================================

/// Event bus for distributing events to handlers
pub struct EventBus {
    /// Registered handlers
    handlers: Vec<Box<dyn EventHandler>>,
    /// Event queue
    queue: EventQueue,
    /// Events processed count
    events_processed: u64,
    /// Events delivered count
    events_delivered: u64,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(queue_size: usize) -> Self {
        Self {
            handlers: Vec::new(),
            queue: EventQueue::new(queue_size),
            events_processed: 0,
            events_delivered: 0,
        }
    }

    /// Register an event handler
    pub fn register(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
        // Sort by priority (descending)
        self.handlers
            .sort_by_key(|h| core::cmp::Reverse(h.priority()));
    }

    /// Publish an event
    pub fn publish(&mut self, event: NexusEvent) -> bool {
        self.queue.push(event)
    }

    /// Process one event
    pub fn process_one(&mut self) -> bool {
        if let Some(event) = self.queue.pop() {
            self.events_processed += 1;
            self.deliver(&event);
            true
        } else {
            false
        }
    }

    /// Process events with a budget
    pub fn process_with_budget(&mut self, max_events: usize) -> usize {
        let mut processed = 0;
        while processed < max_events {
            if !self.process_one() {
                break;
            }
            processed += 1;
        }
        processed
    }

    /// Deliver event to handlers
    fn deliver(&mut self, event: &NexusEvent) {
        // First collect which handlers should receive the event (immutable borrow)
        let should_deliver: Vec<bool> = self
            .handlers
            .iter()
            .map(|h| Self::should_deliver_to(h.as_ref(), event))
            .collect();

        // Then deliver to those handlers (mutable borrow)
        for (idx, handler) in self.handlers.iter_mut().enumerate() {
            if should_deliver[idx] {
                let result = handler.handle(event);
                self.events_delivered += 1;

                if result == EventHandlerResult::Stop {
                    break;
                }
            }
        }
    }

    /// Check if event should be delivered to handler (static version)
    fn should_deliver_to(handler: &dyn EventHandler, event: &NexusEvent) -> bool {
        let subscriptions = handler.subscriptions();

        if subscriptions.is_empty() {
            return true;
        }

        for sub in subscriptions {
            match sub {
                EventSubscription::All => return true,
                EventSubscription::MinPriority(min) if event.priority >= *min => return true,
                EventSubscription::FromComponent(comp) if event.source == Some(*comp) => {
                    return true;
                },
                EventSubscription::Predictions if event.is_prediction() => return true,
                EventSubscription::Healing if event.is_healing() => return true,
                EventSubscription::Anomalies
                    if matches!(event.kind, NexusEventKind::AnomalyDetected { .. }) =>
                {
                    return true;
                },
                EventSubscription::Custom(name) => {
                    if let NexusEventKind::Custom { name: n, .. } = &event.kind {
                        if n == name {
                            return true;
                        }
                    }
                },
                _ => {},
            }
        }

        false
    }

    /// Check if event should be delivered to handler
    #[allow(dead_code)]
    fn should_deliver(&self, handler: &dyn EventHandler, event: &NexusEvent) -> bool {
        let subscriptions = handler.subscriptions();

        if subscriptions.is_empty() {
            return true;
        }

        for sub in subscriptions {
            match sub {
                EventSubscription::All => return true,
                EventSubscription::MinPriority(min) if event.priority >= *min => return true,
                EventSubscription::FromComponent(comp) if event.source == Some(*comp) => {
                    return true;
                },
                EventSubscription::Predictions if event.is_prediction() => return true,
                EventSubscription::Healing if event.is_healing() => return true,
                EventSubscription::Anomalies
                    if matches!(event.kind, NexusEventKind::AnomalyDetected { .. }) =>
                {
                    return true;
                },
                EventSubscription::Custom(name) => {
                    if let NexusEventKind::Custom { name: n, .. } = &event.kind {
                        if n == name {
                            return true;
                        }
                    }
                },
                _ => {},
            }
        }

        false
    }

    /// Get pending event count
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Get statistics
    pub fn stats(&self) -> EventBusStats {
        EventBusStats {
            events_processed: self.events_processed,
            events_delivered: self.events_delivered,
            events_dropped: self.queue.dropped(),
            pending: self.queue.len(),
            handlers: self.handlers.len(),
        }
    }
}

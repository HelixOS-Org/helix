//! Event handler trait and subscription

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use super::event::NexusEvent;
use super::types::EventPriority;
use crate::core::ComponentId;

// ============================================================================
// EVENT HANDLER
// ============================================================================

/// Trait for event handlers
pub trait EventHandler: Send + Sync {
    /// Get handler name
    fn name(&self) -> &'static str;

    /// Handle an event
    fn handle(&mut self, event: &NexusEvent) -> EventHandlerResult;

    /// Get subscribed event kinds (empty = all events)
    fn subscriptions(&self) -> &[EventSubscription] {
        &[]
    }

    /// Get handler priority (higher = called first)
    fn priority(&self) -> i32 {
        0
    }
}

/// Result of event handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventHandlerResult {
    /// Event handled, continue processing
    Continue,
    /// Event handled, stop processing
    Stop,
    /// Event not handled
    Skip,
    /// Error during handling
    Error,
}

/// Event subscription filter
#[derive(Debug, Clone)]
pub enum EventSubscription {
    /// Subscribe to all events
    All,
    /// Subscribe to events of a priority or higher
    MinPriority(EventPriority),
    /// Subscribe to events from a specific component
    FromComponent(ComponentId),
    /// Subscribe to prediction events
    Predictions,
    /// Subscribe to healing events
    Healing,
    /// Subscribe to anomaly events
    Anomalies,
    /// Subscribe to custom events by name
    Custom(String),
}

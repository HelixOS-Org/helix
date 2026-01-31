//! System hooks

#![allow(dead_code)]

use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// SYSTEM HOOK TRAIT
// ============================================================================

/// Hook for intercepting system events
pub trait SystemHook: Send + Sync {
    /// Called when a component starts
    fn on_component_start(&self, component: ComponentId);

    /// Called when a component stops
    fn on_component_stop(&self, component: ComponentId);

    /// Called when an error occurs
    fn on_error(&self, component: ComponentId, error: &str);

    /// Called on each tick
    fn on_tick(&self, timestamp: NexusTimestamp);

    /// Get hook name
    fn name(&self) -> &str;
}

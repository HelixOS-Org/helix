//! NEXUS subsystem trait.

use crate::error::NexusResult;
use crate::event::NexusEvent;

/// Trait for NEXUS subsystems
pub trait NexusSubsystem: Send + Sync {
    /// Get the subsystem name
    fn name(&self) -> &'static str;

    /// Initialize the subsystem
    fn init(&mut self) -> NexusResult<()>;

    /// Shutdown the subsystem
    fn shutdown(&mut self) -> NexusResult<()>;

    /// Process a tick
    fn tick(&mut self) -> NexusResult<()>;

    /// Handle an event
    fn handle_event(&mut self, event: &NexusEvent) -> NexusResult<()>;

    /// Get subsystem health (0.0 - 1.0)
    fn health(&self) -> f32;

    /// Check if subsystem is ready
    fn is_ready(&self) -> bool;
}

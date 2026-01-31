//! Base Component Trait
//!
//! The fundamental trait that all NEXUS components must implement.

#![allow(dead_code)]

use crate::types::{ComponentId, DomainId, NexusResult, Version};

// ============================================================================
// BASE COMPONENT TRAIT
// ============================================================================

/// Base trait for all NEXUS components
pub trait NexusComponent: Send + Sync {
    /// Get component identifier
    fn id(&self) -> ComponentId;

    /// Get component name
    fn name(&self) -> &str;

    /// Get component domain
    fn domain(&self) -> DomainId;

    /// Get component version
    fn version(&self) -> Version;

    /// Check if component is healthy
    fn is_healthy(&self) -> bool;

    /// Get component status
    fn status(&self) -> ComponentStatus;

    /// Initialize the component
    fn init(&mut self) -> NexusResult<()>;

    /// Shutdown the component
    fn shutdown(&mut self) -> NexusResult<()>;

    /// Reset component to initial state
    fn reset(&mut self) -> NexusResult<()>;
}

// ============================================================================
// COMPONENT STATUS
// ============================================================================

/// Component status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentStatus {
    /// Not yet initialized
    Uninitialized,
    /// Initializing
    Initializing,
    /// Ready and healthy
    Ready,
    /// Running
    Running,
    /// Degraded but functional
    Degraded,
    /// Paused
    Paused,
    /// Shutting down
    ShuttingDown,
    /// Stopped
    Stopped,
    /// Failed
    Failed,
}

impl ComponentStatus {
    /// Is the component operational
    pub const fn is_operational(&self) -> bool {
        matches!(self, Self::Ready | Self::Running | Self::Degraded)
    }

    /// Is the component stopped or failed
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }

    /// Is the component transitioning
    pub const fn is_transitioning(&self) -> bool {
        matches!(self, Self::Initializing | Self::ShuttingDown)
    }

    /// Can be started
    pub const fn can_start(&self) -> bool {
        matches!(self, Self::Ready | Self::Stopped | Self::Uninitialized)
    }

    /// Get status name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Uninitialized => "uninitialized",
            Self::Initializing => "initializing",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Degraded => "degraded",
            Self::Paused => "paused",
            Self::ShuttingDown => "shutting_down",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

impl Default for ComponentStatus {
    fn default() -> Self {
        Self::Uninitialized
    }
}

impl core::fmt::Display for ComponentStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_status() {
        assert!(ComponentStatus::Ready.is_operational());
        assert!(ComponentStatus::Running.is_operational());
        assert!(ComponentStatus::Degraded.is_operational());
        assert!(!ComponentStatus::Failed.is_operational());
    }

    #[test]
    fn test_component_status_transitions() {
        assert!(ComponentStatus::Initializing.is_transitioning());
        assert!(ComponentStatus::ShuttingDown.is_transitioning());
        assert!(!ComponentStatus::Running.is_transitioning());
    }

    #[test]
    fn test_component_status_terminal() {
        assert!(ComponentStatus::Stopped.is_terminal());
        assert!(ComponentStatus::Failed.is_terminal());
        assert!(!ComponentStatus::Running.is_terminal());
    }
}

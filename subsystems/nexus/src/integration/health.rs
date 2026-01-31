//! Health probes and checks

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// HEALTH STATUS
// ============================================================================

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Component is degraded but functional
    Degraded,
    /// Component is unhealthy
    Unhealthy,
    /// Component is unknown/unreachable
    Unknown,
}

// ============================================================================
// HEALTH CHECK RESULT
// ============================================================================

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Component ID
    pub component: ComponentId,
    /// Status
    pub status: HealthStatus,
    /// Health value (0.0 - 1.0)
    pub health: f32,
    /// Message
    pub message: Option<String>,
    /// Duration of check (cycles)
    pub duration: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

impl HealthCheckResult {
    /// Create a healthy result
    pub fn healthy(component: ComponentId) -> Self {
        Self {
            component,
            status: HealthStatus::Healthy,
            health: 1.0,
            message: None,
            duration: 0,
            timestamp: NexusTimestamp::now(),
        }
    }

    /// Create an unhealthy result
    pub fn unhealthy(component: ComponentId, message: impl Into<String>) -> Self {
        Self {
            component,
            status: HealthStatus::Unhealthy,
            health: 0.0,
            message: Some(message.into()),
            duration: 0,
            timestamp: NexusTimestamp::now(),
        }
    }

    /// Set health value
    pub fn with_health(mut self, health: f32) -> Self {
        self.health = health.clamp(0.0, 1.0);
        self.status = if health >= 0.8 {
            HealthStatus::Healthy
        } else if health >= 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }
}

// ============================================================================
// HEALTH PROBE TRAIT
// ============================================================================

/// Health probe for a component
pub trait HealthProbe: Send + Sync {
    /// Get component ID
    fn component_id(&self) -> ComponentId;

    /// Get current health (0.0 - 1.0)
    fn health(&self) -> f32;

    /// Get health status
    fn status(&self) -> HealthStatus;

    /// Run a health check
    fn check(&self) -> HealthCheckResult;

    /// Get component name
    fn name(&self) -> &str;
}

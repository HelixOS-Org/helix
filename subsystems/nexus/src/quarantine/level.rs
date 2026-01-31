//! Quarantine levels and reasons

#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::core::ComponentId;

// ============================================================================
// QUARANTINE LEVEL
// ============================================================================

/// Level of quarantine isolation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuarantineLevel {
    /// Monitored - component runs but is watched closely
    Monitored = 0,
    /// Degraded - component runs with reduced functionality
    Degraded = 1,
    /// Restricted - limited operations allowed
    Restricted = 2,
    /// Isolated - no interaction with other components
    Isolated = 3,
    /// Suspended - component is completely stopped
    Suspended = 4,
}

impl QuarantineLevel {
    /// Get from numeric value
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Monitored,
            1 => Self::Degraded,
            2 => Self::Restricted,
            3 => Self::Isolated,
            _ => Self::Suspended,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Monitored => "Monitored",
            Self::Degraded => "Degraded",
            Self::Restricted => "Restricted",
            Self::Isolated => "Isolated",
            Self::Suspended => "Suspended",
        }
    }

    /// Can component still process requests?
    pub fn can_process(&self) -> bool {
        matches!(self, Self::Monitored | Self::Degraded | Self::Restricted)
    }

    /// Can component communicate with others?
    pub fn can_communicate(&self) -> bool {
        matches!(self, Self::Monitored | Self::Degraded)
    }
}

// ============================================================================
// QUARANTINE REASON
// ============================================================================

/// Reason for quarantine
#[derive(Debug, Clone)]
pub enum QuarantineReason {
    /// Repeated failures
    RepeatedFailures { count: u32 },
    /// Health below threshold
    LowHealth { health: f32, threshold: f32 },
    /// Healing failed
    HealingFailed { attempts: u32 },
    /// Anomaly detected
    AnomalyDetected { anomaly_type: String },
    /// Resource exhaustion
    ResourceExhaustion { resource: String },
    /// Security violation
    SecurityViolation { description: String },
    /// Manual quarantine
    Manual { reason: String },
    /// Dependency quarantine (cascade)
    DependencyCascade { source: ComponentId },
}

impl QuarantineReason {
    /// Get display description
    pub fn description(&self) -> String {
        match self {
            Self::RepeatedFailures { count } => format!("{} repeated failures", count),
            Self::LowHealth { health, threshold } => {
                format!(
                    "Health {:.1}% below threshold {:.1}%",
                    health * 100.0,
                    threshold * 100.0
                )
            }
            Self::HealingFailed { attempts } => {
                format!("Healing failed after {} attempts", attempts)
            }
            Self::AnomalyDetected { anomaly_type } => format!("Anomaly: {}", anomaly_type),
            Self::ResourceExhaustion { resource } => format!("Resource exhausted: {}", resource),
            Self::SecurityViolation { description } => format!("Security: {}", description),
            Self::Manual { reason } => format!("Manual: {}", reason),
            Self::DependencyCascade { source } => format!("Dependency cascade from {:?}", source),
        }
    }

    /// Get recommended quarantine level
    pub fn recommended_level(&self) -> QuarantineLevel {
        match self {
            Self::RepeatedFailures { count } if *count >= 5 => QuarantineLevel::Suspended,
            Self::RepeatedFailures { count } if *count >= 3 => QuarantineLevel::Isolated,
            Self::RepeatedFailures { .. } => QuarantineLevel::Restricted,
            Self::LowHealth { health, .. } if *health < 0.2 => QuarantineLevel::Suspended,
            Self::LowHealth { health, .. } if *health < 0.5 => QuarantineLevel::Isolated,
            Self::LowHealth { .. } => QuarantineLevel::Degraded,
            Self::HealingFailed { .. } => QuarantineLevel::Suspended,
            Self::AnomalyDetected { .. } => QuarantineLevel::Restricted,
            Self::ResourceExhaustion { .. } => QuarantineLevel::Isolated,
            Self::SecurityViolation { .. } => QuarantineLevel::Suspended,
            Self::Manual { .. } => QuarantineLevel::Isolated,
            Self::DependencyCascade { .. } => QuarantineLevel::Restricted,
        }
    }
}

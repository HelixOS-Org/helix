//! Core healing types
//!
//! This module defines fundamental types for self-healing including
//! healing strategies and their properties.

#![allow(dead_code)]

use crate::predict::RecommendedAction;

/// Strategy for healing a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealingStrategy {
    /// Do nothing (continue monitoring)
    Monitor,
    /// Soft reset (restart component)
    SoftReset,
    /// Hard reset (force restart)
    HardReset,
    /// Micro-rollback to last checkpoint
    MicroRollback,
    /// Full rollback to earlier checkpoint
    FullRollback,
    /// Reconstruct state from journal
    Reconstruct,
    /// Substitute with backup component
    Substitute,
    /// Quarantine the component
    Quarantine,
    /// Enter survival mode
    SurvivalMode,
}

impl HealingStrategy {
    /// Get priority (higher = try first)
    pub fn priority(&self) -> u8 {
        match self {
            Self::Monitor => 0,
            Self::SoftReset => 1,
            Self::HardReset => 2,
            Self::MicroRollback => 3,
            Self::FullRollback => 4,
            Self::Reconstruct => 5,
            Self::Substitute => 6,
            Self::Quarantine => 7,
            Self::SurvivalMode => 8,
        }
    }

    /// Get estimated time to complete (ms)
    pub fn estimated_time_ms(&self) -> u64 {
        match self {
            Self::Monitor => 0,
            Self::SoftReset => 10,
            Self::HardReset => 50,
            Self::MicroRollback => 50,
            Self::FullRollback => 200,
            Self::Reconstruct => 500,
            Self::Substitute => 500,
            Self::Quarantine => 10,
            Self::SurvivalMode => 100,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Monitor => "Monitor",
            Self::SoftReset => "Soft Reset",
            Self::HardReset => "Hard Reset",
            Self::MicroRollback => "Micro-Rollback",
            Self::FullRollback => "Full Rollback",
            Self::Reconstruct => "Reconstruct",
            Self::Substitute => "Substitute",
            Self::Quarantine => "Quarantine",
            Self::SurvivalMode => "Survival Mode",
        }
    }

    /// From recommended action
    pub fn from_action(action: RecommendedAction) -> Self {
        match action {
            RecommendedAction::Monitor => Self::Monitor,
            RecommendedAction::Alert | RecommendedAction::Prepare => Self::Monitor,
            RecommendedAction::SoftRecover => Self::SoftReset,
            RecommendedAction::HardRecover => Self::HardReset,
            RecommendedAction::Rollback => Self::MicroRollback,
            RecommendedAction::Quarantine => Self::Quarantine,
            RecommendedAction::SurvivalMode => Self::SurvivalMode,
        }
    }

    /// Get escalation strategy (next to try if this fails)
    pub fn escalation(&self) -> Option<Self> {
        match self {
            Self::SoftReset => Some(Self::HardReset),
            Self::HardReset => Some(Self::MicroRollback),
            Self::MicroRollback => Some(Self::FullRollback),
            Self::FullRollback => Some(Self::Reconstruct),
            Self::Reconstruct => Some(Self::Substitute),
            Self::Substitute => Some(Self::Quarantine),
            Self::Quarantine => Some(Self::SurvivalMode),
            _ => None,
        }
    }
}

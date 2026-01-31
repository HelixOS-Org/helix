//! Healing result tracking
//!
//! This module provides the HealingResult struct for tracking
//! the outcome of healing operations.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use super::types::HealingStrategy;
use crate::core::{ComponentId, NexusTimestamp};

/// Result of a healing operation
#[derive(Debug, Clone)]
pub struct HealingResult {
    /// Component that was healed
    pub component: ComponentId,
    /// Strategy used
    pub strategy: HealingStrategy,
    /// Whether healing succeeded
    pub success: bool,
    /// Time taken (cycles)
    pub duration_cycles: u64,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Next strategy to try if this failed
    pub escalation: Option<HealingStrategy>,
}

impl HealingResult {
    /// Create a successful result
    pub fn success(component: ComponentId, strategy: HealingStrategy, duration: u64) -> Self {
        Self {
            component,
            strategy,
            success: true,
            duration_cycles: duration,
            message: "Healing successful".into(),
            timestamp: NexusTimestamp::now(),
            escalation: None,
        }
    }

    /// Create a failed result
    pub fn failure(
        component: ComponentId,
        strategy: HealingStrategy,
        duration: u64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            component,
            strategy,
            success: false,
            duration_cycles: duration,
            message: message.into(),
            timestamp: NexusTimestamp::now(),
            escalation: strategy.escalation(),
        }
    }

    /// Create a result with custom message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Check if escalation is possible
    pub fn can_escalate(&self) -> bool {
        self.escalation.is_some()
    }
}

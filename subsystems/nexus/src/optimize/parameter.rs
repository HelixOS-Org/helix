//! Optimization parameters

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

// ============================================================================
// OPTIMIZATION PARAMETER
// ============================================================================

/// A tunable parameter
#[derive(Debug, Clone)]
pub struct OptimizationParameter {
    /// Parameter name
    pub name: String,
    /// Current value
    pub value: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Default value
    pub default: f64,
    /// Description
    pub description: String,
}

impl OptimizationParameter {
    /// Create a new parameter
    pub fn new(name: impl Into<String>, default: f64, min: f64, max: f64) -> Self {
        Self {
            name: name.into(),
            value: default,
            min,
            max,
            default,
            description: String::new(),
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set value (clamped to range)
    #[inline(always)]
    pub fn set(&mut self, value: f64) {
        self.value = value.clamp(self.min, self.max);
    }

    /// Reset to default
    #[inline(always)]
    pub fn reset(&mut self) {
        self.value = self.default;
    }

    /// Get normalized value (0.0 - 1.0)
    #[inline]
    pub fn normalized(&self) -> f64 {
        if self.max == self.min {
            0.5
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }
}

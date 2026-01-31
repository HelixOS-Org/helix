//! Fault target definitions

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use crate::core::ComponentId;

// ============================================================================
// FAULT TARGET
// ============================================================================

/// Target for fault injection
#[derive(Debug, Clone)]
pub enum FaultTarget {
    /// All components
    Global,
    /// Specific component
    Component(ComponentId),
    /// Specific function (by name)
    Function(String),
    /// Specific path/location
    Path(String),
    /// Random selection
    Random { probability: f32 },
}

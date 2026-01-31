//! Property specification
//!
//! This module provides the Property struct for defining properties
//! to verify with pre/post conditions.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::PropertyType;

/// A property to verify
#[derive(Debug, Clone)]
pub struct Property {
    /// Property ID
    pub id: u64,
    /// Property name
    pub name: String,
    /// Description
    pub description: String,
    /// Property type
    pub prop_type: PropertyType,
    /// Is critical?
    pub critical: bool,
    /// Pre-conditions
    pub preconditions: Vec<String>,
    /// Post-conditions
    pub postconditions: Vec<String>,
}

impl Property {
    /// Create a new property
    pub fn new(name: impl Into<String>, prop_type: PropertyType) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            description: String::new(),
            prop_type,
            critical: false,
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as critical
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Add precondition
    pub fn requires(mut self, condition: impl Into<String>) -> Self {
        self.preconditions.push(condition.into());
        self
    }

    /// Add postcondition
    pub fn ensures(mut self, condition: impl Into<String>) -> Self {
        self.postconditions.push(condition.into());
        self
    }
}

/// Create a simple safety property
pub fn safety_property(name: impl Into<String>, desc: impl Into<String>) -> Property {
    Property::new(name, PropertyType::Safety).with_description(desc)
}

/// Create an invariant property
pub fn invariant(name: impl Into<String>, desc: impl Into<String>) -> Property {
    Property::new(name, PropertyType::Invariant)
        .with_description(desc)
        .critical()
}

/// Create a progress property
pub fn progress_property(name: impl Into<String>, desc: impl Into<String>) -> Property {
    Property::new(name, PropertyType::Progress).with_description(desc)
}

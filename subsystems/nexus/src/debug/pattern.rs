//! Bug pattern definitions

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// BUG CATEGORY
// ============================================================================

/// Bug category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BugCategory {
    /// Memory issues (leak, corruption, etc.)
    Memory,
    /// Concurrency issues (race, deadlock, etc.)
    Concurrency,
    /// Resource issues (exhaustion, leak)
    Resource,
    /// Logic errors
    Logic,
    /// API misuse
    ApiMisuse,
    /// Configuration errors
    Configuration,
    /// Hardware issues
    Hardware,
    /// Unknown
    Unknown,
}

// ============================================================================
// BUG SEVERITY
// ============================================================================

/// Bug severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BugSeverity {
    /// Low - minor issue
    Low      = 0,
    /// Medium - moderate issue
    Medium   = 1,
    /// High - significant issue
    High     = 2,
    /// Critical - system threatening
    Critical = 3,
}

// ============================================================================
// BUG PATTERN
// ============================================================================

/// A recognizable bug pattern
#[derive(Debug, Clone)]
pub struct BugPattern {
    /// Pattern ID
    pub id: u64,
    /// Pattern name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: BugCategory,
    /// Severity
    pub severity: BugSeverity,
    /// Symptoms (regex-like patterns in error messages)
    pub symptoms: Vec<String>,
    /// Suggested fixes
    pub fixes: Vec<String>,
    /// Related patterns
    pub related: Vec<u64>,
}

impl BugPattern {
    /// Create a new pattern
    pub fn new(name: impl Into<String>, category: BugCategory) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            description: String::new(),
            category,
            severity: BugSeverity::Medium,
            symptoms: Vec::new(),
            fixes: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set severity
    pub fn with_severity(mut self, severity: BugSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add symptom
    pub fn with_symptom(mut self, symptom: impl Into<String>) -> Self {
        self.symptoms.push(symptom.into());
        self
    }

    /// Add fix suggestion
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fixes.push(fix.into());
        self
    }

    /// Match against an error message
    pub fn matches(&self, error: &str) -> bool {
        let error_lower = error.to_lowercase();
        self.symptoms
            .iter()
            .any(|s| error_lower.contains(&s.to_lowercase()))
    }
}

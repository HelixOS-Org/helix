//! Diagnosis and fix suggestions

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::pattern::BugPattern;
use crate::core::NexusTimestamp;

// ============================================================================
// FIX TYPE
// ============================================================================

/// Type of fix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixType {
    /// Configuration change
    Configuration,
    /// Code fix
    Code,
    /// Resource adjustment
    Resource,
    /// Restart/reset
    Restart,
    /// Upgrade/update
    Update,
    /// Workaround
    Workaround,
}

// ============================================================================
// FIX
// ============================================================================

/// A suggested fix
#[derive(Debug, Clone)]
pub struct Fix {
    /// Fix description
    pub description: String,
    /// Fix type
    pub fix_type: FixType,
    /// Confidence
    pub confidence: f64,
    /// Automatic fix available?
    pub automatic: bool,
    /// Steps to implement
    pub steps: Vec<String>,
}

impl Fix {
    /// Create a new fix
    pub fn new(description: impl Into<String>, fix_type: FixType) -> Self {
        Self {
            description: description.into(),
            fix_type,
            confidence: 0.5,
            automatic: false,
            steps: Vec::new(),
        }
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Mark as automatic
    pub fn automatic(mut self) -> Self {
        self.automatic = true;
        self
    }

    /// Add step
    pub fn with_step(mut self, step: impl Into<String>) -> Self {
        self.steps.push(step.into());
        self
    }
}

// ============================================================================
// DIAGNOSIS
// ============================================================================

/// A diagnosis of an issue
#[derive(Debug, Clone)]
pub struct Diagnosis {
    /// Diagnosis ID
    pub id: u64,
    /// Matched pattern (if any)
    pub pattern: Option<BugPattern>,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
    /// Root cause analysis
    pub root_cause: String,
    /// Suggested fixes
    pub fixes: Vec<Fix>,
    /// Related issues
    pub related: Vec<String>,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

impl Diagnosis {
    /// Create a new diagnosis
    pub fn new(root_cause: impl Into<String>, confidence: f64) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            pattern: None,
            confidence: confidence.clamp(0.0, 1.0),
            root_cause: root_cause.into(),
            fixes: Vec::new(),
            related: Vec::new(),
            timestamp: NexusTimestamp::now(),
        }
    }

    /// Set matched pattern
    pub fn with_pattern(mut self, pattern: BugPattern) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Add a fix
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Add related issue
    pub fn with_related(mut self, issue: impl Into<String>) -> Self {
        self.related.push(issue.into());
        self
    }
}

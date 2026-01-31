//! Signal Analysis Types
//!
//! Analysis results, issues, and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{ProcessId, SignalAction, SignalNumber, SignalPattern};

/// Signal analysis result
#[derive(Debug, Clone)]
pub struct SignalAnalysis {
    /// Process ID
    pub pid: ProcessId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Detected issues
    pub issues: Vec<SignalIssue>,
    /// Active patterns
    pub patterns: Vec<SignalPattern>,
    /// Recommendations
    pub recommendations: Vec<SignalRecommendation>,
}

impl SignalAnalysis {
    /// Create new analysis
    pub fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            health_score: 100.0,
            issues: Vec::new(),
            patterns: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    /// Check if healthy
    pub fn is_healthy(&self) -> bool {
        self.health_score >= 80.0
    }

    /// Check if has critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.issues.iter().any(|i| i.severity >= 8)
    }
}

/// Signal issue
#[derive(Debug, Clone)]
pub struct SignalIssue {
    /// Issue type
    pub issue_type: SignalIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Involved signal
    pub signal: Option<SignalNumber>,
}

impl SignalIssue {
    /// Create new issue
    pub fn new(issue_type: SignalIssueType, severity: u8, description: String) -> Self {
        Self {
            issue_type,
            severity,
            description,
            signal: None,
        }
    }

    /// Set involved signal
    pub fn with_signal(mut self, signo: SignalNumber) -> Self {
        self.signal = Some(signo);
        self
    }
}

/// Signal issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalIssueType {
    /// Signal storm detected
    Storm,
    /// Slow signal handler
    SlowHandler,
    /// Async-unsafe handler
    UnsafeHandler,
    /// Queue overflow
    QueueOverflow,
    /// Nested signal handlers
    NestedHandlers,
    /// Signal delivery failure
    DeliveryFailure,
    /// High failure rate
    HighFailureRate,
    /// Signal loss
    SignalLoss,
}

/// Signal recommendation
#[derive(Debug, Clone)]
pub struct SignalRecommendation {
    /// Action type
    pub action: SignalAction,
    /// Signal number
    pub signal: Option<SignalNumber>,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

impl SignalRecommendation {
    /// Create new recommendation
    pub fn new(action: SignalAction, reason: String) -> Self {
        Self {
            action,
            signal: None,
            expected_improvement: 0.0,
            reason,
        }
    }

    /// Set signal
    pub fn with_signal(mut self, signo: SignalNumber) -> Self {
        self.signal = Some(signo);
        self
    }

    /// Set expected improvement
    pub fn with_improvement(mut self, improvement: f32) -> Self {
        self.expected_improvement = improvement;
        self
    }
}

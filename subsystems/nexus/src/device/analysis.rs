//! Device Analysis
//!
//! Device health analysis and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::DeviceId;

/// Device analysis result
#[derive(Debug, Clone)]
pub struct DeviceAnalysis {
    /// Device ID
    pub device_id: DeviceId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Issues detected
    pub issues: Vec<DeviceIssue>,
    /// Recommendations
    pub recommendations: Vec<DeviceRecommendation>,
    /// Power efficiency score (0-100)
    pub power_efficiency: f32,
}

/// Device issue
#[derive(Debug, Clone)]
pub struct DeviceIssue {
    /// Issue type
    pub issue_type: DeviceIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Device issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceIssueType {
    /// No driver bound
    NoDriver,
    /// Driver probe failed
    ProbeFailed,
    /// Device in error state
    ErrorState,
    /// Power state issues
    PowerIssue,
    /// Frequent hotplug events
    HotplugStorm,
    /// Deferred probe timeout
    DeferredTimeout,
}

/// Device recommendation
#[derive(Debug, Clone)]
pub struct DeviceRecommendation {
    /// Action type
    pub action: DeviceAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Device actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    /// Reprobe device
    Reprobe,
    /// Load fallback driver
    LoadFallback,
    /// Reset device
    Reset,
    /// Change power policy
    ChangePowerPolicy,
    /// Disable device
    Disable,
}

//! Kprobe Analysis
//!
//! Analysis results and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{FunctionStats, KprobeId};

/// Kprobe analysis result
#[derive(Debug, Clone)]
pub struct KprobeAnalysis {
    /// Kprobe ID
    pub kprobe_id: KprobeId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Hit rate (hits per second)
    pub hit_rate: f32,
    /// Function stats (if available)
    pub function_stats: Option<FunctionStats>,
    /// Issues detected
    pub issues: Vec<KprobeIssue>,
    /// Recommendations
    pub recommendations: Vec<KprobeRecommendation>,
}

/// Kprobe issue
#[derive(Debug, Clone)]
pub struct KprobeIssue {
    /// Issue type
    pub issue_type: KprobeIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Kprobe issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KprobeIssueType {
    /// High overhead
    HighOverhead,
    /// Never hit
    NeverHit,
    /// Too many misses
    TooManyMisses,
    /// Unstable probe
    UnstableProbe,
    /// Performance impact
    PerformanceImpact,
}

impl KprobeIssueType {
    /// Get issue type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::HighOverhead => "high_overhead",
            Self::NeverHit => "never_hit",
            Self::TooManyMisses => "too_many_misses",
            Self::UnstableProbe => "unstable_probe",
            Self::PerformanceImpact => "performance_impact",
        }
    }
}

/// Kprobe recommendation
#[derive(Debug, Clone)]
pub struct KprobeRecommendation {
    /// Action
    pub action: KprobeAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Kprobe actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KprobeAction {
    /// Remove unused probe
    RemoveUnused,
    /// Add filter
    AddFilter,
    /// Use tracepoint instead
    UseTracepoint,
    /// Optimize handler
    OptimizeHandler,
    /// Reduce sampling
    ReduceSampling,
}

impl KprobeAction {
    /// Get action name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::RemoveUnused => "remove_unused",
            Self::AddFilter => "add_filter",
            Self::UseTracepoint => "use_tracepoint",
            Self::OptimizeHandler => "optimize_handler",
            Self::ReduceSampling => "reduce_sampling",
        }
    }
}

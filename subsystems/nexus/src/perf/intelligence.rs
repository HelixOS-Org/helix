//! Performance Intelligence
//!
//! AI-powered performance analysis and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{EventConfig, EventId, PerfManager, PerfMetrics, Pmu, PmuId, WorkloadAnalysis};

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

/// Performance analysis
#[derive(Debug, Clone)]
pub struct PerfAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Efficiency score (0-100)
    pub efficiency_score: f32,
    /// Workload analysis
    pub workload: Option<WorkloadAnalysis>,
    /// Issues
    pub issues: Vec<PerfIssue>,
    /// Recommendations
    pub recommendations: Vec<PerfRecommendation>,
}

/// Performance issue
#[derive(Debug, Clone)]
pub struct PerfIssue {
    /// Issue type
    pub issue_type: PerfIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Performance issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfIssueType {
    /// Low IPC
    LowIpc,
    /// High cache misses
    HighCacheMisses,
    /// High branch misses
    HighBranchMisses,
    /// Counter multiplexing
    Multiplexing,
    /// No core PMU
    NoCorePmu,
    /// Counter overflow
    CounterOverflow,
}

/// Performance recommendation
#[derive(Debug, Clone)]
pub struct PerfRecommendation {
    /// Action
    pub action: PerfAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Performance action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfAction {
    /// Profile cache
    ProfileCache,
    /// Optimize branches
    OptimizeBranches,
    /// Reduce working set
    ReduceWorkingSet,
    /// Use prefetch
    UsePrefetch,
    /// Vectorize
    Vectorize,
    /// Reduce counters
    ReduceCounters,
}

// ============================================================================
// PERFORMANCE INTELLIGENCE
// ============================================================================

/// Performance Intelligence
pub struct PerfIntelligence {
    /// Manager
    manager: PerfManager,
    /// Current metrics
    current_metrics: PerfMetrics,
}

impl PerfIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: PerfManager::new(),
            current_metrics: PerfMetrics::new(),
        }
    }

    /// Register PMU
    #[inline(always)]
    pub fn register_pmu(&mut self, pmu: Pmu) {
        self.manager.register_pmu(pmu);
    }

    /// Create event
    #[inline(always)]
    pub fn create_event(&mut self, config: EventConfig, pmu: PmuId) -> EventId {
        self.manager.create_event(config, pmu)
    }

    /// Update metrics
    #[inline(always)]
    pub fn update_metrics(&mut self, metrics: PerfMetrics) {
        self.current_metrics = metrics;
    }

    /// Analyze performance
    pub fn analyze(&self) -> PerfAnalysis {
        let mut health_score = 100.0f32;
        let mut efficiency_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check IPC
        if let Some(ipc) = self.current_metrics.ipc {
            if ipc < 0.5 {
                health_score -= 30.0;
                efficiency_score -= 25.0;
                issues.push(PerfIssue {
                    issue_type: PerfIssueType::LowIpc,
                    severity: 7,
                    description: alloc::format!("Low IPC ({:.2}) indicates stalls", ipc),
                });
                recommendations.push(PerfRecommendation {
                    action: PerfAction::ProfileCache,
                    expected_improvement: 20.0,
                    reason: String::from("Profile cache to identify memory bottlenecks"),
                });
            }
        }

        // Check cache misses
        if let Some(miss_rate) = self.current_metrics.cache_miss_rate {
            if miss_rate > 20.0 {
                health_score -= 25.0;
                issues.push(PerfIssue {
                    issue_type: PerfIssueType::HighCacheMisses,
                    severity: 8,
                    description: alloc::format!("High cache miss rate: {:.1}%", miss_rate),
                });
                recommendations.push(PerfRecommendation {
                    action: PerfAction::ReduceWorkingSet,
                    expected_improvement: 25.0,
                    reason: String::from("Reduce working set size or improve cache locality"),
                });
            } else if miss_rate > 10.0 {
                health_score -= 10.0;
                issues.push(PerfIssue {
                    issue_type: PerfIssueType::HighCacheMisses,
                    severity: 5,
                    description: alloc::format!("Moderate cache miss rate: {:.1}%", miss_rate),
                });
            }
        }

        // Check branch misses
        if let Some(miss_rate) = self.current_metrics.branch_miss_rate {
            if miss_rate > 10.0 {
                health_score -= 15.0;
                issues.push(PerfIssue {
                    issue_type: PerfIssueType::HighBranchMisses,
                    severity: 6,
                    description: alloc::format!("High branch miss rate: {:.1}%", miss_rate),
                });
                recommendations.push(PerfRecommendation {
                    action: PerfAction::OptimizeBranches,
                    expected_improvement: 15.0,
                    reason: String::from("Consider branch-free code or better branch hints"),
                });
            }
        }

        // Check PMU availability
        if self.manager.core_pmu().is_none() {
            health_score -= 10.0;
            issues.push(PerfIssue {
                issue_type: PerfIssueType::NoCorePmu,
                severity: 4,
                description: String::from("No core PMU registered for hardware counters"),
            });
        }

        // Check for multiplexing
        for event in self.manager.events.values() {
            let ratio = event.mux_ratio();
            if ratio > 1.5 {
                issues.push(PerfIssue {
                    issue_type: PerfIssueType::Multiplexing,
                    severity: 4,
                    description: alloc::format!(
                        "Event {} has high multiplexing ratio: {:.2}",
                        event.config.event_type.name(),
                        ratio
                    ),
                });
            }
        }

        // Workload analysis
        let workload = WorkloadAnalysis::from_metrics(&self.current_metrics);

        health_score = health_score.max(0.0);
        efficiency_score = efficiency_score.max(0.0);

        PerfAnalysis {
            health_score,
            efficiency_score,
            workload: Some(workload),
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &PerfManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut PerfManager {
        &mut self.manager
    }

    /// Get current metrics
    #[inline(always)]
    pub fn metrics(&self) -> &PerfMetrics {
        &self.current_metrics
    }
}

impl Default for PerfIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

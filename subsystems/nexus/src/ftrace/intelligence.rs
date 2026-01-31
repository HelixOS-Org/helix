//! Ftrace intelligence and analysis.

use alloc::string::String;
use alloc::vec::Vec;

use super::latency::{LatencyRecord, LatencyType};
use super::manager::FtraceManager;
use super::tracer::TracerType;
use super::types::CpuId;

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

/// Ftrace analysis
#[derive(Debug, Clone)]
pub struct FtraceAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Trace quality score (0-100)
    pub trace_quality: f32,
    /// Hottest functions
    pub hot_functions: Vec<HotFunction>,
    /// Latency issues
    pub latency_issues: Vec<LatencyIssue>,
    /// Issues
    pub issues: Vec<FtraceIssue>,
    /// Recommendations
    pub recommendations: Vec<FtraceRecommendation>,
}

/// Hot function
#[derive(Debug, Clone)]
pub struct HotFunction {
    /// Name
    pub name: String,
    /// Total time (ns)
    pub total_time_ns: u64,
    /// Self time (ns)
    pub self_time_ns: u64,
    /// Call count
    pub call_count: u64,
    /// Percentage of total
    pub percentage: f32,
}

/// Latency issue
#[derive(Debug, Clone)]
pub struct LatencyIssue {
    /// Latency type
    pub latency_type: LatencyType,
    /// Duration (ns)
    pub duration_ns: u64,
    /// CPU
    pub cpu: CpuId,
    /// Function
    pub function: Option<String>,
    /// Severity (1-10)
    pub severity: u8,
}

/// Ftrace issue
#[derive(Debug, Clone)]
pub struct FtraceIssue {
    /// Issue type
    pub issue_type: FtraceIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Ftrace issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtraceIssueType {
    /// High IRQ latency
    HighIrqLatency,
    /// High preemption latency
    HighPreemptLatency,
    /// Lost entries
    LostEntries,
    /// Hot function
    HotFunction,
    /// Deep call stack
    DeepCallStack,
}

/// Ftrace recommendation
#[derive(Debug, Clone)]
pub struct FtraceRecommendation {
    /// Action
    pub action: FtraceAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Ftrace action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtraceAction {
    /// Reduce IRQ disabled sections
    ReduceIrqOff,
    /// Reduce preemption disabled sections
    ReducePreemptOff,
    /// Increase buffer size
    IncreaseBuffer,
    /// Optimize hot function
    OptimizeFunction,
    /// Add tracepoints
    AddTracepoints,
}

// ============================================================================
// FTRACE INTELLIGENCE
// ============================================================================

/// Ftrace Intelligence
pub struct FtraceIntelligence {
    /// Manager
    manager: FtraceManager,
}

impl FtraceIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: FtraceManager::new(),
        }
    }

    /// Set tracer
    pub fn set_tracer(&mut self, tracer: TracerType) {
        self.manager.set_tracer(tracer);
    }

    /// Start tracing
    pub fn start(&self) {
        self.manager.start();
    }

    /// Stop tracing
    pub fn stop(&self) {
        self.manager.stop();
    }

    /// Record latency
    pub fn record_latency(&mut self, record: LatencyRecord) {
        self.manager.record_latency(record);
    }

    /// Analyze ftrace data
    pub fn analyze(&self) -> FtraceAnalysis {
        let mut health_score = 100.0f32;
        let mut trace_quality = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        let mut latency_issues = Vec::new();

        // Check for lost entries
        let mut total_lost = 0u64;
        for buffer in self.manager.buffers.values() {
            total_lost += buffer.lost_entries();
        }
        if total_lost > 0 {
            let loss_rate = total_lost as f32
                / self
                    .manager
                    .buffers
                    .values()
                    .map(|b| b.entry_count())
                    .sum::<u64>()
                    .max(1) as f32
                * 100.0;

            if loss_rate > 5.0 {
                trace_quality -= 30.0;
                issues.push(FtraceIssue {
                    issue_type: FtraceIssueType::LostEntries,
                    severity: 7,
                    description: alloc::format!("High trace entry loss rate: {:.1}%", loss_rate),
                });
                recommendations.push(FtraceRecommendation {
                    action: FtraceAction::IncreaseBuffer,
                    expected_improvement: 20.0,
                    reason: String::from("Increase trace buffer size to reduce lost entries"),
                });
            }
        }

        // Check latency issues
        for record in &self.manager.latency_records {
            let threshold_ns = match record.latency_type {
                LatencyType::IrqOff => 100_000,        // 100us
                LatencyType::PreemptOff => 500_000,    // 500us
                LatencyType::IrqPreemptOff => 100_000, // 100us
                LatencyType::Wakeup => 1_000_000,      // 1ms
                LatencyType::Hardware => 10_000,       // 10us
            };

            if record.duration_ns > threshold_ns {
                let severity = if record.duration_ns > threshold_ns * 10 {
                    9
                } else if record.duration_ns > threshold_ns * 5 {
                    7
                } else {
                    5
                };

                health_score -= (severity as f32) * 2.0;

                latency_issues.push(LatencyIssue {
                    latency_type: record.latency_type,
                    duration_ns: record.duration_ns,
                    cpu: record.cpu,
                    function: record.start_func.clone(),
                    severity,
                });

                let issue_type = match record.latency_type {
                    LatencyType::IrqOff | LatencyType::IrqPreemptOff => {
                        FtraceIssueType::HighIrqLatency
                    },
                    _ => FtraceIssueType::HighPreemptLatency,
                };

                issues.push(FtraceIssue {
                    issue_type,
                    severity,
                    description: alloc::format!(
                        "{} latency: {}us on CPU {}",
                        record.latency_type.name(),
                        record.duration_us(),
                        record.cpu.0
                    ),
                });
            }
        }

        // Analyze hot functions
        let hot_functions = self
            .manager
            .call_graph
            .hottest(10)
            .iter()
            .map(|node| HotFunction {
                name: node.name.clone(),
                total_time_ns: node.total_time_ns,
                self_time_ns: node.self_time_ns,
                call_count: node.call_count,
                percentage: if self.manager.call_graph.total_time_ns > 0 {
                    node.total_time_ns as f32 / self.manager.call_graph.total_time_ns as f32 * 100.0
                } else {
                    0.0
                },
            })
            .collect();

        // Check for very hot functions
        for node in self.manager.call_graph.hottest(5) {
            let pct = if self.manager.call_graph.total_time_ns > 0 {
                node.total_time_ns as f32 / self.manager.call_graph.total_time_ns as f32 * 100.0
            } else {
                0.0
            };

            if pct > 20.0 {
                issues.push(FtraceIssue {
                    issue_type: FtraceIssueType::HotFunction,
                    severity: 6,
                    description: alloc::format!(
                        "Function {} takes {:.1}% of traced time",
                        node.name,
                        pct
                    ),
                });
                recommendations.push(FtraceRecommendation {
                    action: FtraceAction::OptimizeFunction,
                    expected_improvement: 15.0,
                    reason: alloc::format!("Optimize {} to improve performance", node.name),
                });
            }
        }

        health_score = health_score.max(0.0);
        trace_quality = trace_quality.max(0.0);

        FtraceAnalysis {
            health_score,
            trace_quality,
            hot_functions,
            latency_issues,
            issues,
            recommendations,
        }
    }

    /// Get manager
    pub fn manager(&self) -> &FtraceManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut FtraceManager {
        &mut self.manager
    }
}

impl Default for FtraceIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

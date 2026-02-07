//! # Dynamic Resource Adaptation
//!
//! Adjusts kernel resource allocation in real-time based on application
//! profiles. Changes scheduler priority, memory policies, I/O scheduling,
//! and network QoS dynamically.

use alloc::vec::Vec;

use super::classify::WorkloadCategory;
use super::profile::{AppLifecyclePhase, ProcessProfile};

// ============================================================================
// ADAPTATION TYPES
// ============================================================================

/// Target resource to adjust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceTarget {
    /// CPU scheduling parameters
    CpuScheduler,
    /// Memory allocation policies
    MemoryPolicy,
    /// I/O scheduling priority
    IoScheduler,
    /// Network QoS
    NetworkQos,
    /// Cache allocation
    CachePartition,
    /// NUMA placement
    NumaPlacement,
    /// Power management
    PowerManagement,
}

/// A specific adjustment to make
#[derive(Debug, Clone)]
pub struct ResourceAdjustment {
    /// What to adjust
    pub target: ResourceTarget,
    /// Parameter name
    pub parameter: &'static str,
    /// New value
    pub value: AdjustmentValue,
    /// Priority of this adjustment (higher = more important)
    pub priority: u32,
    /// Expected benefit (0.0 - 1.0)
    pub expected_benefit: f64,
}

/// Value for an adjustment
#[derive(Debug, Clone)]
pub enum AdjustmentValue {
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// String value
    Text(&'static str),
}

/// An adaptation action to take
#[derive(Debug, Clone)]
pub struct AdaptationAction {
    /// Process ID to apply to
    pub pid: u64,
    /// Category that triggered this
    pub triggered_by: WorkloadCategory,
    /// Adjustments to make
    pub adjustments: Vec<ResourceAdjustment>,
    /// Expected overall improvement
    pub expected_improvement: f64,
    /// Whether this is reversible
    pub reversible: bool,
}

// ============================================================================
// ADAPTATION ENGINE
// ============================================================================

/// The adaptation engine — computes optimal resource adjustments
/// based on process profiles.
pub struct AdaptationEngine {
    /// Active adaptations (pid -> actions)
    active: Vec<(u64, AdaptationAction)>,
    /// Total adaptations applied
    total_applied: u64,
    /// Total adaptations reverted
    total_reverted: u64,
    /// Whether adaptation is enabled
    enabled: bool,
}

impl AdaptationEngine {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            enabled: true,
            total_applied: 0,
            total_reverted: 0,
        }
    }

    /// Compute adaptations for a process profile
    pub fn compute_adaptations(&self, profile: &ProcessProfile) -> Vec<AdaptationAction> {
        if !self.enabled || !profile.is_mature() {
            return Vec::new();
        }

        let mut actions = Vec::new();

        // CPU adaptations
        if let Some(action) = self.cpu_adaptation(profile) {
            actions.push(action);
        }

        // Memory adaptations
        if let Some(action) = self.memory_adaptation(profile) {
            actions.push(action);
        }

        // I/O adaptations
        if let Some(action) = self.io_adaptation(profile) {
            actions.push(action);
        }

        // Network adaptations
        if let Some(action) = self.network_adaptation(profile) {
            actions.push(action);
        }

        // Phase-specific adaptations
        if let Some(action) = self.phase_adaptation(profile) {
            actions.push(action);
        }

        actions
    }

    /// Apply an adaptation action
    pub fn apply(&mut self, action: AdaptationAction) {
        let pid = action.pid;
        self.active.push((pid, action));
        self.total_applied += 1;
    }

    /// Revert adaptations for a process
    pub fn revert(&mut self, pid: u64) {
        let count_before = self.active.len();
        self.active.retain(|(p, _)| *p != pid);
        self.total_reverted += (count_before - self.active.len()) as u64;
    }

    /// Get active adaptations for a process
    pub fn active_for(&self, pid: u64) -> Vec<&AdaptationAction> {
        self.active
            .iter()
            .filter(|(p, _)| *p == pid)
            .map(|(_, a)| a)
            .collect()
    }

    fn cpu_adaptation(&self, profile: &ProcessProfile) -> Option<AdaptationAction> {
        let mut adjustments = Vec::new();

        if profile.cpu.is_compute_bound {
            // Compute-bound: increase time slice, pin to dedicated cores
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::CpuScheduler,
                parameter: "time_slice_ms",
                value: AdjustmentValue::Integer(20),
                priority: 5,
                expected_benefit: 0.15,
            });
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::CpuScheduler,
                parameter: "cpu_affinity",
                value: AdjustmentValue::Text("performance_cores"),
                priority: 3,
                expected_benefit: 0.10,
            });
        } else if profile.cpu.is_bursty {
            // Bursty: keep on same core for cache locality
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::CpuScheduler,
                parameter: "migration_cost",
                value: AdjustmentValue::Integer(500_000), // 500µs
                priority: 3,
                expected_benefit: 0.08,
            });
        }

        if profile.cpu.typical_thread_count > 4 {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::CpuScheduler,
                parameter: "group_scheduling",
                value: AdjustmentValue::Boolean(true),
                priority: 4,
                expected_benefit: 0.12,
            });
        }

        if adjustments.is_empty() {
            return None;
        }

        Some(AdaptationAction {
            pid: profile.pid,
            triggered_by: WorkloadCategory::CpuBound,
            expected_improvement: adjustments.iter().map(|a| a.expected_benefit).sum::<f64>().min(0.5),
            adjustments,
            reversible: true,
        })
    }

    fn memory_adaptation(&self, profile: &ProcessProfile) -> Option<AdaptationAction> {
        let mut adjustments = Vec::new();

        if profile.memory.should_use_huge_pages() {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::MemoryPolicy,
                parameter: "transparent_huge_pages",
                value: AdjustmentValue::Text("always"),
                priority: 5,
                expected_benefit: 0.15,
            });
        }

        if profile.memory.likely_leak() {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::MemoryPolicy,
                parameter: "oom_score_adj",
                value: AdjustmentValue::Integer(300),
                priority: 7,
                expected_benefit: 0.05,
            });
        }

        if profile.memory.working_set > 256 * 1024 * 1024 {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::NumaPlacement,
                parameter: "numa_bind",
                value: AdjustmentValue::Text("local_node"),
                priority: 4,
                expected_benefit: 0.10,
            });
        }

        if adjustments.is_empty() {
            return None;
        }

        Some(AdaptationAction {
            pid: profile.pid,
            triggered_by: WorkloadCategory::MemoryBound,
            expected_improvement: adjustments.iter().map(|a| a.expected_benefit).sum::<f64>().min(0.5),
            adjustments,
            reversible: true,
        })
    }

    fn io_adaptation(&self, profile: &ProcessProfile) -> Option<AdaptationAction> {
        let mut adjustments = Vec::new();

        if profile.io.is_io_intensive() {
            let readahead = profile.io.optimal_readahead();
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::IoScheduler,
                parameter: "readahead_kb",
                value: AdjustmentValue::Integer((readahead / 1024) as i64),
                priority: 4,
                expected_benefit: 0.20,
            });
        }

        if profile.io.sequential_reads {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::IoScheduler,
                parameter: "io_priority",
                value: AdjustmentValue::Text("best_effort_high"),
                priority: 3,
                expected_benefit: 0.10,
            });
        }

        if profile.io.frequent_fsync {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::IoScheduler,
                parameter: "write_barrier_mode",
                value: AdjustmentValue::Text("coalesced"),
                priority: 4,
                expected_benefit: 0.15,
            });
        }

        if adjustments.is_empty() {
            return None;
        }

        Some(AdaptationAction {
            pid: profile.pid,
            triggered_by: WorkloadCategory::IoBound,
            expected_improvement: adjustments.iter().map(|a| a.expected_benefit).sum::<f64>().min(0.5),
            adjustments,
            reversible: true,
        })
    }

    fn network_adaptation(&self, profile: &ProcessProfile) -> Option<AdaptationAction> {
        if !profile.network.is_network_intensive() {
            return None;
        }

        let mut adjustments = Vec::new();

        if profile.network.is_server {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::NetworkQos,
                parameter: "tcp_fastopen",
                value: AdjustmentValue::Boolean(true),
                priority: 3,
                expected_benefit: 0.08,
            });
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::NetworkQos,
                parameter: "listen_backlog",
                value: AdjustmentValue::Integer(4096),
                priority: 4,
                expected_benefit: 0.10,
            });
        }

        if profile.network.active_connections > 1000 {
            adjustments.push(ResourceAdjustment {
                target: ResourceTarget::NetworkQos,
                parameter: "socket_buffer_size",
                value: AdjustmentValue::Integer(256 * 1024),
                priority: 4,
                expected_benefit: 0.12,
            });
        }

        Some(AdaptationAction {
            pid: profile.pid,
            triggered_by: WorkloadCategory::NetworkBound,
            expected_improvement: adjustments.iter().map(|a| a.expected_benefit).sum::<f64>().min(0.5),
            adjustments,
            reversible: true,
        })
    }

    fn phase_adaptation(&self, profile: &ProcessProfile) -> Option<AdaptationAction> {
        match profile.phase {
            AppLifecyclePhase::Startup => {
                // During startup: prioritize I/O for library loading
                Some(AdaptationAction {
                    pid: profile.pid,
                    triggered_by: WorkloadCategory::IoBound,
                    adjustments: alloc::vec![ResourceAdjustment {
                        target: ResourceTarget::IoScheduler,
                        parameter: "startup_io_boost",
                        value: AdjustmentValue::Boolean(true),
                        priority: 6,
                        expected_benefit: 0.20,
                    }],
                    expected_improvement: 0.20,
                    reversible: true,
                })
            }
            AppLifecyclePhase::Idle => {
                // During idle: release resources
                Some(AdaptationAction {
                    pid: profile.pid,
                    triggered_by: WorkloadCategory::Unknown,
                    adjustments: alloc::vec![
                        ResourceAdjustment {
                            target: ResourceTarget::PowerManagement,
                            parameter: "allow_deep_sleep",
                            value: AdjustmentValue::Boolean(true),
                            priority: 2,
                            expected_benefit: 0.05,
                        },
                        ResourceAdjustment {
                            target: ResourceTarget::CachePartition,
                            parameter: "cache_ways",
                            value: AdjustmentValue::Integer(1),
                            priority: 2,
                            expected_benefit: 0.03,
                        },
                    ],
                    expected_improvement: 0.08,
                    reversible: true,
                })
            }
            _ => None,
        }
    }
}

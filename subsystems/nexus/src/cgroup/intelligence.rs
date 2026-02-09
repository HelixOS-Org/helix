//! Cgroup Intelligence
//!
//! AI-powered cgroup analysis and optimization.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    CgroupId, CgroupInfo, CgroupVersion, CpuLimits, CpuUsage, EnforcementAction, HierarchyManager,
    HierarchyStats, IoUsage, LimitsEnforcer, MemoryLimits, MemoryPressure, MemoryUsage, ProcessId,
    ResourceAccountant, ResourceSample,
};

/// Cgroup analysis result
#[derive(Debug, Clone)]
pub struct CgroupAnalysis {
    /// Cgroup ID
    pub cgroup_id: CgroupId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Resource efficiency (0-100)
    pub efficiency: f32,
    /// Issues detected
    pub issues: Vec<CgroupIssue>,
    /// Recommendations
    pub recommendations: Vec<CgroupRecommendation>,
}

/// Cgroup issue
#[derive(Debug, Clone)]
pub struct CgroupIssue {
    /// Issue type
    pub issue_type: CgroupIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Cgroup issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupIssueType {
    /// High CPU throttling
    HighThrottling,
    /// Memory pressure
    MemoryPressure,
    /// OOM events
    OomEvents,
    /// PIDs limit near
    PidsLimitNear,
    /// Empty cgroup
    EmptyCgroup,
    /// Resource imbalance
    ResourceImbalance,
    /// Orphaned cgroup
    OrphanedCgroup,
}

/// Cgroup recommendation
#[derive(Debug, Clone)]
pub struct CgroupRecommendation {
    /// Action
    pub action: CgroupAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Cgroup actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupAction {
    /// Increase CPU quota
    IncreaseCpuQuota,
    /// Increase memory limit
    IncreaseMemoryLimit,
    /// Decrease limits
    DecreaseLimits,
    /// Migrate processes
    MigrateProcesses,
    /// Delete empty cgroup
    DeleteEmpty,
    /// Rebalance resources
    RebalanceResources,
}

/// Cgroup Intelligence - comprehensive cgroup analysis and management
pub struct CgroupIntelligence {
    /// Hierarchy manager
    hierarchy: HierarchyManager,
    /// Resource accountant
    accountant: ResourceAccountant,
    /// Limits enforcer
    enforcer: LimitsEnforcer,
    /// Total CPU quota allocated
    total_cpu_quota: AtomicU64,
    /// Total memory allocated
    total_memory_allocated: AtomicU64,
}

impl CgroupIntelligence {
    /// Create new cgroup intelligence
    pub fn new(version: CgroupVersion) -> Self {
        Self {
            hierarchy: HierarchyManager::new(version),
            accountant: ResourceAccountant::new(),
            enforcer: LimitsEnforcer::new(),
            total_cpu_quota: AtomicU64::new(0),
            total_memory_allocated: AtomicU64::new(0),
        }
    }

    /// Initialize
    #[inline(always)]
    pub fn initialize(&mut self, timestamp: u64) -> CgroupId {
        self.hierarchy.init_root(timestamp)
    }

    /// Create cgroup
    #[inline(always)]
    pub fn create_cgroup(
        &mut self,
        parent: CgroupId,
        name: String,
        timestamp: u64,
    ) -> Option<CgroupId> {
        self.hierarchy.create_cgroup(parent, name, timestamp)
    }

    /// Delete cgroup
    #[inline(always)]
    pub fn delete_cgroup(&mut self, id: CgroupId) -> bool {
        self.accountant.clear_samples(id);
        self.hierarchy.delete_cgroup(id)
    }

    /// Add process to cgroup
    #[inline(always)]
    pub fn add_process(&mut self, cgroup: CgroupId, pid: ProcessId) -> bool {
        self.hierarchy.add_process(cgroup, pid)
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: ProcessId) -> bool {
        self.hierarchy.remove_process(pid)
    }

    /// Set CPU limits
    #[inline]
    pub fn set_cpu_limits(&mut self, cgroup: CgroupId, limits: CpuLimits) -> bool {
        if let Some(info) = self.hierarchy.get_cgroup_mut(cgroup) {
            info.cpu_limits = limits;
            return true;
        }
        false
    }

    /// Set memory limits
    #[inline]
    pub fn set_memory_limits(&mut self, cgroup: CgroupId, limits: MemoryLimits) -> bool {
        if let Some(info) = self.hierarchy.get_cgroup_mut(cgroup) {
            info.memory_limits = limits;
            return true;
        }
        false
    }

    /// Update resource usage
    pub fn update_usage(
        &mut self,
        cgroup: CgroupId,
        cpu: CpuUsage,
        memory: MemoryUsage,
        io: IoUsage,
        timestamp: u64,
    ) {
        if let Some(info) = self.hierarchy.get_cgroup_mut(cgroup) {
            info.cpu_usage = cpu;
            info.memory_usage = memory;
            info.io_usage = io;
            info.updated_at = timestamp;

            let sample = ResourceSample {
                timestamp,
                cpu_ns: cpu.usage_ns,
                memory: memory.usage,
                io_bytes: io.total_bytes(),
            };
            self.accountant.record_sample(cgroup, sample);
        }
    }

    /// Enforce limits for cgroup
    pub fn enforce_limits(&mut self, cgroup: CgroupId, timestamp: u64) -> Vec<EnforcementAction> {
        let mut actions = Vec::new();

        let info = match self.hierarchy.get_cgroup(cgroup) {
            Some(i) => i.clone(),
            None => return actions,
        };

        let cpu_action = self.enforcer.enforce_cpu(&info, timestamp);
        if cpu_action != EnforcementAction::None {
            actions.push(cpu_action);
        }

        let mem_action = self.enforcer.enforce_memory(&info, timestamp);
        if mem_action != EnforcementAction::None {
            actions.push(mem_action);
        }

        let pids_action = self.enforcer.enforce_pids(&info, timestamp);
        if pids_action != EnforcementAction::None {
            actions.push(pids_action);
        }

        actions
    }

    /// Analyze cgroup
    pub fn analyze_cgroup(&self, cgroup: CgroupId) -> Option<CgroupAnalysis> {
        let info = self.hierarchy.get_cgroup(cgroup)?;
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check CPU throttling
        if info.cpu_usage.throttle_percent() > 20.0 {
            health_score -= 20.0;
            issues.push(CgroupIssue {
                issue_type: CgroupIssueType::HighThrottling,
                severity: 6,
                description: alloc::format!(
                    "High CPU throttling: {:.1}%",
                    info.cpu_usage.throttle_percent()
                ),
            });
            recommendations.push(CgroupRecommendation {
                action: CgroupAction::IncreaseCpuQuota,
                expected_improvement: 15.0,
                reason: String::from("Increase CPU quota to reduce throttling"),
            });
        }

        // Check memory pressure
        if info.memory_usage.pressure_level != MemoryPressure::None {
            let severity = match info.memory_usage.pressure_level {
                MemoryPressure::Low => 3,
                MemoryPressure::Medium => 5,
                MemoryPressure::Critical => 8,
                MemoryPressure::None => 0,
            };
            health_score -= severity as f32 * 3.0;
            issues.push(CgroupIssue {
                issue_type: CgroupIssueType::MemoryPressure,
                severity,
                description: String::from("Memory pressure detected"),
            });
        }

        // Check OOM events
        if info.memory_usage.oom_events > 0 {
            health_score -= 25.0;
            issues.push(CgroupIssue {
                issue_type: CgroupIssueType::OomEvents,
                severity: 9,
                description: alloc::format!("OOM events: {}", info.memory_usage.oom_events),
            });
            recommendations.push(CgroupRecommendation {
                action: CgroupAction::IncreaseMemoryLimit,
                expected_improvement: 25.0,
                reason: String::from("Increase memory limit to prevent OOM"),
            });
        }

        // Check PIDs utilization
        if info.pids_limits.utilization() > 0.9 {
            health_score -= 15.0;
            issues.push(CgroupIssue {
                issue_type: CgroupIssueType::PidsLimitNear,
                severity: 7,
                description: String::from("Approaching PIDs limit"),
            });
        }

        // Check for empty cgroups
        if info.is_empty() {
            issues.push(CgroupIssue {
                issue_type: CgroupIssueType::EmptyCgroup,
                severity: 2,
                description: String::from("Empty cgroup"),
            });
            recommendations.push(CgroupRecommendation {
                action: CgroupAction::DeleteEmpty,
                expected_improvement: 5.0,
                reason: String::from("Delete empty cgroup to clean up hierarchy"),
            });
        }

        health_score = health_score.max(0.0);
        let efficiency = self.calculate_efficiency(info);

        Some(CgroupAnalysis {
            cgroup_id: cgroup,
            health_score,
            efficiency,
            issues,
            recommendations,
        })
    }

    /// Calculate resource efficiency
    fn calculate_efficiency(&self, info: &CgroupInfo) -> f32 {
        let mut efficiency = 100.0;

        if info.cpu_limits.is_throttled() {
            let throttle_penalty = info.cpu_usage.throttle_percent() / 2.0;
            efficiency -= throttle_penalty;
        }

        if info.memory_limits.is_limited() {
            let limit = info.memory_limits.effective_limit();
            let usage = info.memory_usage.usage;
            if limit > 0 && limit != u64::MAX {
                let utilization = usage as f32 / limit as f32;
                if utilization < 0.1 {
                    efficiency -= 10.0;
                }
                if info.memory_usage.oom_events > 0 {
                    efficiency -= 20.0;
                }
            }
        }

        efficiency.max(0.0)
    }

    /// Get hierarchy manager
    #[inline(always)]
    pub fn hierarchy(&self) -> &HierarchyManager {
        &self.hierarchy
    }

    /// Get hierarchy manager mutably
    #[inline(always)]
    pub fn hierarchy_mut(&mut self) -> &mut HierarchyManager {
        &mut self.hierarchy
    }

    /// Get resource accountant
    #[inline(always)]
    pub fn accountant(&self) -> &ResourceAccountant {
        &self.accountant
    }

    /// Get limits enforcer
    #[inline(always)]
    pub fn enforcer(&self) -> &LimitsEnforcer {
        &self.enforcer
    }

    /// Get cgroup by ID
    #[inline(always)]
    pub fn get_cgroup(&self, id: CgroupId) -> Option<&CgroupInfo> {
        self.hierarchy.get_cgroup(id)
    }

    /// Get hierarchy statistics
    #[inline(always)]
    pub fn hierarchy_stats(&self) -> HierarchyStats {
        self.hierarchy.stats()
    }
}

impl Default for CgroupIntelligence {
    fn default() -> Self {
        Self::new(CgroupVersion::V2)
    }
}

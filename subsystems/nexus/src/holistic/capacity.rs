//! # Capacity Planning
//!
//! System capacity estimation and planning:
//! - Resource capacity modeling
//! - Saturation prediction
//! - Scaling recommendations
//! - Headroom analysis
//! - Growth trend analysis
//! - What-if scenario analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// CAPACITY MODEL
// ============================================================================

/// Resource being modeled
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapacityResource {
    /// CPU cores
    CpuCores,
    /// CPU time (percent)
    CpuTime,
    /// Physical memory (bytes)
    PhysicalMemory,
    /// Swap (bytes)
    Swap,
    /// Disk space (bytes)
    DiskSpace,
    /// Disk IOPS
    DiskIops,
    /// Disk bandwidth (bytes/sec)
    DiskBandwidth,
    /// Network bandwidth (bytes/sec)
    NetworkBandwidth,
    /// File descriptors
    FileDescriptors,
    /// Process count
    Processes,
    /// Thread count
    Threads,
}

/// Capacity state for a single resource
#[derive(Debug, Clone)]
pub struct ResourceCapacity {
    /// Resource type
    pub resource: CapacityResource,
    /// Total capacity
    pub total: u64,
    /// Currently used
    pub used: u64,
    /// Reserved (committed but not yet used)
    pub reserved: u64,
    /// Usage history (for trend analysis)
    history: VecDeque<UsageSample>,
    /// Max history entries
    max_history: usize,
}

/// A usage sample
#[derive(Debug, Clone, Copy)]
pub struct UsageSample {
    /// Usage value
    pub value: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl ResourceCapacity {
    pub fn new(resource: CapacityResource, total: u64) -> Self {
        Self {
            resource,
            total,
            used: 0,
            reserved: 0,
            history: VecDeque::new(),
            max_history: 1440, // 24 hours at 1-minute intervals
        }
    }

    /// Available capacity
    #[inline(always)]
    pub fn available(&self) -> u64 {
        self.total.saturating_sub(self.used).saturating_sub(self.reserved)
    }

    /// Utilization (0.0 - 1.0)
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.used as f64 / self.total as f64
    }

    /// Committed ratio (used + reserved)
    #[inline]
    pub fn committed_ratio(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.used + self.reserved) as f64 / self.total as f64
    }

    /// Headroom (percent available)
    #[inline]
    pub fn headroom(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.available() as f64 / self.total as f64 * 100.0
    }

    /// Record usage sample
    #[inline]
    pub fn record(&mut self, used: u64, timestamp: u64) {
        self.used = used;
        self.history.push_back(UsageSample {
            value: used,
            timestamp,
        });
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Trend (units per second)
    pub fn trend(&self) -> f64 {
        let len = self.history.len();
        if len < 10 {
            return 0.0;
        }

        // Use last 25% of samples for trend
        let window = len / 4;
        let recent = &self.history[len - window..];

        if recent.len() < 2 {
            return 0.0;
        }

        let first = recent[0];
        let last = recent[recent.len() - 1];

        let dt = last.timestamp.saturating_sub(first.timestamp);
        if dt == 0 {
            return 0.0;
        }

        (last.value as f64 - first.value as f64) / (dt as f64 / 1000.0)
    }

    /// Predict when resource will be exhausted (seconds from now, None if decreasing)
    pub fn time_to_exhaustion(&self) -> Option<u64> {
        let trend = self.trend();
        if trend <= 0.0 {
            return None; // Not growing
        }

        let remaining = self.available() as f64;
        if remaining <= 0.0 {
            return Some(0); // Already exhausted
        }

        Some((remaining / trend) as u64)
    }

    /// Peak usage in history
    #[inline]
    pub fn peak_usage(&self) -> u64 {
        self.history
            .iter()
            .map(|s| s.value)
            .max()
            .unwrap_or(self.used)
    }

    /// Average usage
    #[inline]
    pub fn avg_usage(&self) -> f64 {
        if self.history.is_empty() {
            return self.used as f64;
        }
        let sum: u64 = self.history.iter().map(|s| s.value).sum();
        sum as f64 / self.history.len() as f64
    }
}

// ============================================================================
// SCALING RECOMMENDATIONS
// ============================================================================

/// Scaling direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingDirection {
    /// Scale up (add more)
    ScaleUp,
    /// Scale down (reduce)
    ScaleDown,
    /// No change needed
    NoChange,
}

/// Scaling recommendation
#[derive(Debug, Clone)]
pub struct ScalingRecommendation {
    /// Resource to scale
    pub resource: CapacityResource,
    /// Direction
    pub direction: ScalingDirection,
    /// Urgency (0.0 = not urgent, 1.0 = critical)
    pub urgency: f64,
    /// Current utilization
    pub current_utilization: f64,
    /// Predicted utilization in 1 hour
    pub predicted_1h: f64,
    /// Predicted utilization in 24 hours
    pub predicted_24h: f64,
    /// Recommended new capacity
    pub recommended_capacity: u64,
    /// Time until critical (seconds, None if not applicable)
    pub time_to_critical: Option<u64>,
}

// ============================================================================
// WHAT-IF ANALYSIS
// ============================================================================

/// What-if scenario
#[derive(Debug, Clone)]
pub struct Scenario {
    /// Scenario name
    pub name: u32, // Simplified from String
    /// Additional processes
    pub additional_processes: u32,
    /// Additional memory per process (bytes)
    pub memory_per_process: u64,
    /// Additional CPU per process (percent * 100)
    pub cpu_per_process: u32,
    /// Additional I/O per process (IOPS)
    pub io_per_process: u64,
}

/// Scenario result
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// Scenario ID
    pub scenario_id: u32,
    /// Per-resource utilization after scenario
    pub utilizations: BTreeMap<u8, f64>,
    /// Resources that would be exhausted
    pub exhausted: Vec<CapacityResource>,
    /// Overall feasibility (0.0 = impossible, 1.0 = easy)
    pub feasibility: f64,
}

// ============================================================================
// CAPACITY PLANNER
// ============================================================================

/// Capacity planner thresholds
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Utilization threshold for warning (0.0 - 1.0)
    pub warning_threshold: f64,
    /// Utilization threshold for critical
    pub critical_threshold: f64,
    /// Target headroom (percent)
    pub target_headroom: f64,
    /// Prediction horizon (seconds)
    pub prediction_horizon_secs: u64,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 0.70,
            critical_threshold: 0.90,
            target_headroom: 30.0,
            prediction_horizon_secs: 86400, // 24 hours
        }
    }
}

/// System capacity planner
pub struct CapacityPlanner {
    /// Resource capacities
    resources: BTreeMap<u8, ResourceCapacity>,
    /// Configuration
    config: PlannerConfig,
    /// Scaling recommendations
    recommendations: Vec<ScalingRecommendation>,
    /// Total evaluations
    pub total_evaluations: u64,
}

impl CapacityPlanner {
    pub fn new(config: PlannerConfig) -> Self {
        Self {
            resources: BTreeMap::new(),
            config,
            recommendations: Vec::new(),
            total_evaluations: 0,
        }
    }

    /// Add resource to track
    #[inline(always)]
    pub fn add_resource(&mut self, resource: CapacityResource, total: u64) {
        self.resources
            .insert(resource as u8, ResourceCapacity::new(resource, total));
    }

    /// Update resource usage
    #[inline]
    pub fn update(&mut self, resource: CapacityResource, used: u64, timestamp: u64) {
        if let Some(rc) = self.resources.get_mut(&(resource as u8)) {
            rc.record(used, timestamp);
        }
    }

    /// Update reserved
    #[inline]
    pub fn update_reserved(&mut self, resource: CapacityResource, reserved: u64) {
        if let Some(rc) = self.resources.get_mut(&(resource as u8)) {
            rc.reserved = reserved;
        }
    }

    /// Evaluate and generate recommendations
    pub fn evaluate(&mut self) -> Vec<ScalingRecommendation> {
        self.total_evaluations += 1;
        self.recommendations.clear();

        let resources: Vec<(u8, f64, f64, Option<u64>, u64, CapacityResource)> = self
            .resources
            .iter()
            .map(|(&key, rc)| {
                let util = rc.utilization();
                let trend = rc.trend();
                let tte = rc.time_to_exhaustion();

                // Predict future utilization
                let predicted_1h = util + trend * 3600.0 / rc.total.max(1) as f64;
                (key, util, predicted_1h, tte, rc.total, rc.resource)
            })
            .collect();

        for (_key, util, predicted_1h, tte, total, resource) in resources {
            let predicted_24h = util + (predicted_1h - util) * 24.0;

            let urgency = if util >= self.config.critical_threshold {
                1.0
            } else if util >= self.config.warning_threshold {
                0.5 + 0.5 * (util - self.config.warning_threshold)
                    / (self.config.critical_threshold - self.config.warning_threshold)
            } else {
                0.0
            };

            let direction = if urgency > 0.0 || predicted_1h > self.config.warning_threshold {
                ScalingDirection::ScaleUp
            } else if util < 0.2 && predicted_1h < 0.3 {
                ScalingDirection::ScaleDown
            } else {
                ScalingDirection::NoChange
            };

            if direction != ScalingDirection::NoChange {
                let recommended = if direction == ScalingDirection::ScaleUp {
                    // Add enough for target headroom
                    let target_util = 1.0 - self.config.target_headroom / 100.0;
                    let current_used = (util * total as f64) as u64;
                    if target_util > 0.0 {
                        (current_used as f64 / target_util) as u64
                    } else {
                        total * 2
                    }
                } else {
                    // Scale down: target 50% util
                    let current_used = (util * total as f64) as u64;
                    current_used * 2
                };

                self.recommendations.push(ScalingRecommendation {
                    resource,
                    direction,
                    urgency,
                    current_utilization: util,
                    predicted_1h,
                    predicted_24h,
                    recommended_capacity: recommended,
                    time_to_critical: tte,
                });
            }
        }

        self.recommendations.sort_by(|a, b| {
            b.urgency
                .partial_cmp(&a.urgency)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        self.recommendations.clone()
    }

    /// Run what-if scenario
    pub fn what_if(&self, scenario: &Scenario) -> ScenarioResult {
        let mut utilizations = BTreeMap::new();
        let mut exhausted = Vec::new();

        let additional_cpu = scenario.additional_processes as u64
            * scenario.cpu_per_process as u64;
        let additional_mem = scenario.additional_processes as u64
            * scenario.memory_per_process;
        let additional_io = scenario.additional_processes as u64
            * scenario.io_per_process;

        for (&key, rc) in &self.resources {
            let additional = match rc.resource {
                CapacityResource::CpuTime => additional_cpu,
                CapacityResource::PhysicalMemory => additional_mem,
                CapacityResource::DiskIops => additional_io,
                _ => 0,
            };

            let new_used = rc.used + additional;
            let new_util = if rc.total > 0 {
                new_used as f64 / rc.total as f64
            } else {
                0.0
            };

            utilizations.insert(key, new_util);

            if new_util >= 1.0 {
                exhausted.push(rc.resource);
            }
        }

        let feasibility = if exhausted.is_empty() {
            let max_util = utilizations
                .values()
                .copied()
                .fold(0.0f64, f64::max);
            1.0 - max_util
        } else {
            0.0
        };

        ScenarioResult {
            scenario_id: scenario.name,
            utilizations,
            exhausted,
            feasibility,
        }
    }

    /// Get resource capacity
    #[inline(always)]
    pub fn get_resource(&self, resource: CapacityResource) -> Option<&ResourceCapacity> {
        self.resources.get(&(resource as u8))
    }

    /// Resource count
    #[inline(always)]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Get recommendations
    #[inline(always)]
    pub fn recommendations(&self) -> &[ScalingRecommendation] {
        &self.recommendations
    }
}

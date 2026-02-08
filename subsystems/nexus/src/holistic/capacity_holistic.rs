//! # Holistic Capacity Planner
//!
//! Long-term system capacity planning:
//! - Growth trend analysis
//! - Capacity runway estimation
//! - Bottleneck prediction
//! - What-if scenario modeling
//! - Right-sizing recommendations

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CAPACITY TYPES
// ============================================================================

/// Resource dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityResource {
    /// CPU cores
    CpuCores,
    /// CPU time
    CpuTime,
    /// Memory bytes
    Memory,
    /// Disk space
    DiskSpace,
    /// Disk IOPS
    DiskIops,
    /// Network bandwidth
    NetworkBandwidth,
}

/// Capacity trend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityTrend {
    /// Decreasing usage
    Declining,
    /// Stable usage
    Stable,
    /// Slowly increasing
    GrowingSlow,
    /// Fast increasing
    GrowingFast,
    /// Exponential growth
    Exponential,
}

/// Capacity health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityHealth {
    /// Plenty of headroom (>50%)
    Healthy,
    /// Moderate headroom (30-50%)
    Adequate,
    /// Low headroom (10-30%)
    Warning,
    /// Very low (<10%)
    Critical,
    /// At or over capacity
    Exhausted,
}

// ============================================================================
// TIME SERIES
// ============================================================================

/// Capacity data point
#[derive(Debug, Clone)]
pub struct CapacityDataPoint {
    /// Timestamp
    pub timestamp: u64,
    /// Usage (absolute)
    pub usage: f64,
    /// Capacity (total)
    pub capacity: f64,
}

/// Capacity time series
#[derive(Debug)]
pub struct CapacityTimeSeries {
    /// Data points
    points: Vec<CapacityDataPoint>,
    /// Max points
    max_points: usize,
}

impl CapacityTimeSeries {
    pub fn new(max_points: usize) -> Self {
        Self {
            points: Vec::new(),
            max_points,
        }
    }

    /// Add point
    pub fn add(&mut self, point: CapacityDataPoint) {
        if self.points.len() >= self.max_points {
            self.points.remove(0);
        }
        self.points.push(point);
    }

    /// Current utilization
    pub fn current_utilization(&self) -> f64 {
        self.points.last().map(|p| {
            if p.capacity <= 0.0 { 0.0 } else { p.usage / p.capacity }
        }).unwrap_or(0.0)
    }

    /// Average utilization over last N points
    pub fn avg_utilization(&self, n: usize) -> f64 {
        let start = self.points.len().saturating_sub(n);
        let recent = &self.points[start..];
        if recent.is_empty() {
            return 0.0;
        }
        let sum: f64 = recent.iter().map(|p| {
            if p.capacity <= 0.0 { 0.0 } else { p.usage / p.capacity }
        }).sum();
        sum / recent.len() as f64
    }

    /// Growth rate (per period)
    pub fn growth_rate(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let first = &self.points[0];
        let last = self.points.last().unwrap();
        let time_delta = (last.timestamp - first.timestamp) as f64;
        if time_delta <= 0.0 {
            return 0.0;
        }
        (last.usage - first.usage) / time_delta
    }

    /// Trend detection
    pub fn trend(&self) -> CapacityTrend {
        let rate = self.growth_rate();
        if rate < -0.001 {
            CapacityTrend::Declining
        } else if rate < 0.001 {
            CapacityTrend::Stable
        } else if rate < 0.01 {
            CapacityTrend::GrowingSlow
        } else if rate < 0.1 {
            CapacityTrend::GrowingFast
        } else {
            CapacityTrend::Exponential
        }
    }

    /// Estimate time until capacity exhaustion (ns)
    pub fn time_to_exhaustion(&self) -> Option<u64> {
        if self.points.len() < 2 {
            return None;
        }
        let rate = self.growth_rate();
        if rate <= 0.0 {
            return None; // not growing
        }
        let last = self.points.last()?;
        let headroom = last.capacity - last.usage;
        if headroom <= 0.0 {
            return Some(0); // already exhausted
        }
        Some((headroom / rate) as u64)
    }

    /// Points count
    pub fn len(&self) -> usize {
        self.points.len()
    }
}

// ============================================================================
// SCENARIO MODELING
// ============================================================================

/// What-if scenario
#[derive(Debug, Clone)]
pub struct CapacityScenario {
    /// Name
    pub name: alloc::string::String,
    /// Changes: resource -> multiplier
    pub changes: BTreeMap<u8, f64>,
    /// Projected headroom after changes
    pub projected_headroom: BTreeMap<u8, f64>,
}

impl CapacityScenario {
    pub fn new(name: alloc::string::String) -> Self {
        Self {
            name,
            changes: BTreeMap::new(),
            projected_headroom: BTreeMap::new(),
        }
    }

    /// Add resource change (multiplier, e.g. 2.0 = double)
    pub fn add_change(&mut self, resource: CapacityResource, multiplier: f64) {
        self.changes.insert(resource as u8, multiplier);
    }
}

// ============================================================================
// RECOMMENDATION
// ============================================================================

/// Right-sizing recommendation
#[derive(Debug, Clone)]
pub struct SizingRecommendation {
    /// Resource
    pub resource: CapacityResource,
    /// Current capacity
    pub current: f64,
    /// Recommended capacity
    pub recommended: f64,
    /// Confidence
    pub confidence: f64,
    /// Reason
    pub reason: alloc::string::String,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Capacity planner stats
#[derive(Debug, Clone, Default)]
pub struct HolisticCapacityStats {
    /// Tracked resources
    pub tracked_resources: usize,
    /// Resources at warning
    pub warning_count: usize,
    /// Resources at critical
    pub critical_count: usize,
    /// Shortest runway (ns)
    pub shortest_runway_ns: Option<u64>,
}

/// Holistic capacity planner
pub struct HolisticCapacityEngine {
    /// Per-resource time series
    series: BTreeMap<u8, CapacityTimeSeries>,
    /// Resource capacities
    capacities: BTreeMap<u8, f64>,
    /// Stats
    stats: HolisticCapacityStats,
}

impl HolisticCapacityEngine {
    pub fn new() -> Self {
        Self {
            series: BTreeMap::new(),
            capacities: BTreeMap::new(),
            stats: HolisticCapacityStats::default(),
        }
    }

    /// Register resource
    pub fn register_resource(&mut self, resource: CapacityResource, capacity: f64) {
        self.series.insert(resource as u8, CapacityTimeSeries::new(1440)); // 24h at 1min intervals
        self.capacities.insert(resource as u8, capacity);
        self.update_stats();
    }

    /// Record usage
    pub fn record_usage(&mut self, resource: CapacityResource, usage: f64, now: u64) {
        let key = resource as u8;
        let capacity = self.capacities.get(&key).copied().unwrap_or(0.0);
        if let Some(ts) = self.series.get_mut(&key) {
            ts.add(CapacityDataPoint {
                timestamp: now,
                usage,
                capacity,
            });
        }
        self.update_stats();
    }

    /// Get capacity health for resource
    pub fn health(&self, resource: CapacityResource) -> CapacityHealth {
        let key = resource as u8;
        let util = self.series.get(&key)
            .map(|ts| ts.current_utilization())
            .unwrap_or(0.0);
        if util >= 1.0 {
            CapacityHealth::Exhausted
        } else if util >= 0.9 {
            CapacityHealth::Critical
        } else if util >= 0.7 {
            CapacityHealth::Warning
        } else if util >= 0.5 {
            CapacityHealth::Adequate
        } else {
            CapacityHealth::Healthy
        }
    }

    /// Get trend
    pub fn trend(&self, resource: CapacityResource) -> CapacityTrend {
        let key = resource as u8;
        self.series.get(&key)
            .map(|ts| ts.trend())
            .unwrap_or(CapacityTrend::Stable)
    }

    /// Time to exhaustion
    pub fn time_to_exhaustion(&self, resource: CapacityResource) -> Option<u64> {
        let key = resource as u8;
        self.series.get(&key)?.time_to_exhaustion()
    }

    /// Generate recommendations
    pub fn recommendations(&self) -> Vec<SizingRecommendation> {
        let mut recs = Vec::new();
        for (&key, ts) in &self.series {
            let util = ts.current_utilization();
            let capacity = self.capacities.get(&key).copied().unwrap_or(0.0);
            let resource = match key {
                0 => CapacityResource::CpuCores,
                1 => CapacityResource::CpuTime,
                2 => CapacityResource::Memory,
                3 => CapacityResource::DiskSpace,
                4 => CapacityResource::DiskIops,
                _ => CapacityResource::NetworkBandwidth,
            };

            if util > 0.8 {
                recs.push(SizingRecommendation {
                    resource,
                    current: capacity,
                    recommended: capacity * 1.5,
                    confidence: if ts.len() > 100 { 0.8 } else { 0.5 },
                    reason: alloc::string::String::from("High utilization, consider scaling up"),
                });
            } else if util < 0.2 && capacity > 0.0 {
                recs.push(SizingRecommendation {
                    resource,
                    current: capacity,
                    recommended: capacity * 0.6,
                    confidence: if ts.len() > 100 { 0.7 } else { 0.4 },
                    reason: alloc::string::String::from("Low utilization, consider scaling down"),
                });
            }
        }
        recs
    }

    /// Evaluate scenario
    pub fn evaluate_scenario(&self, scenario: &mut CapacityScenario) {
        for (&key, ts) in &self.series {
            let base_capacity = self.capacities.get(&key).copied().unwrap_or(0.0);
            let multiplier = scenario.changes.get(&key).copied().unwrap_or(1.0);
            let new_capacity = base_capacity * multiplier;
            let usage = ts.points.last().map(|p| p.usage).unwrap_or(0.0);
            let headroom = if new_capacity > 0.0 {
                (new_capacity - usage) / new_capacity
            } else {
                0.0
            };
            scenario.projected_headroom.insert(key, headroom);
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_resources = self.series.len();
        self.stats.warning_count = 0;
        self.stats.critical_count = 0;
        self.stats.shortest_runway_ns = None;

        for (&key, ts) in &self.series {
            let util = ts.current_utilization();
            if util >= 0.9 {
                self.stats.critical_count += 1;
            } else if util >= 0.7 {
                self.stats.warning_count += 1;
            }
            if let Some(tte) = ts.time_to_exhaustion() {
                match self.stats.shortest_runway_ns {
                    Some(current) if tte < current => {
                        self.stats.shortest_runway_ns = Some(tte);
                    }
                    None => {
                        self.stats.shortest_runway_ns = Some(tte);
                    }
                    _ => {}
                }
            }
            let _ = key;
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticCapacityStats {
        &self.stats
    }
}

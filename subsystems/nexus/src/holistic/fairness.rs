//! # Fairness Enforcement
//!
//! System-wide fairness guarantees:
//! - CPU time fairness across processes and groups
//! - Memory allocation fairness
//! - I/O bandwidth fairness
//! - Network bandwidth fairness
//! - Multi-resource fairness (DRF - Dominant Resource Fairness)
//! - Fairness metrics and reporting
//! - Anti-starvation mechanisms

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RESOURCE SHARES
// ============================================================================

/// Resource type for fairness
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FairnessResource {
    /// CPU time
    Cpu,
    /// Memory
    Memory,
    /// Disk I/O bandwidth
    DiskIo,
    /// Network bandwidth
    Network,
}

/// A process/group's resource share
#[derive(Debug, Clone)]
pub struct ResourceShare {
    /// Share weight (default 1024)
    pub weight: u32,
    /// Minimum guarantee (absolute units)
    pub min_guarantee: u64,
    /// Maximum limit (0 = unlimited)
    pub max_limit: u64,
    /// Current allocation
    pub current_allocation: u64,
    /// Total capacity of the resource
    pub total_capacity: u64,
}

impl ResourceShare {
    pub fn new(weight: u32, total_capacity: u64) -> Self {
        Self {
            weight,
            min_guarantee: 0,
            max_limit: 0,
            current_allocation: 0,
            total_capacity,
        }
    }

    /// Fair share based on weight
    #[inline]
    pub fn fair_share(&self, total_weight: u32) -> u64 {
        if total_weight == 0 {
            return 0;
        }
        (self.total_capacity * self.weight as u64) / total_weight as u64
    }

    /// Is over-allocated?
    #[inline(always)]
    pub fn over_allocated(&self, total_weight: u32) -> bool {
        self.current_allocation > self.fair_share(total_weight)
    }

    /// Allocation ratio (current / fair_share)
    #[inline]
    pub fn allocation_ratio(&self, total_weight: u32) -> f64 {
        let fair = self.fair_share(total_weight);
        if fair == 0 {
            return 0.0;
        }
        self.current_allocation as f64 / fair as f64
    }
}

// ============================================================================
// FAIRNESS TRACKING
// ============================================================================

/// Per-process fairness state
#[derive(Debug, Clone)]
pub struct ProcessFairness {
    /// PID
    pub pid: u64,
    /// Group ID (for cgroup-like grouping)
    pub group_id: u64,
    /// Resource shares
    pub shares: BTreeMap<u8, ResourceShare>,
    /// Dominant resource type
    pub dominant_resource: FairnessResource,
    /// Dominant share ratio
    pub dominant_ratio: f64,
    /// Wait time accumulator (microseconds)
    pub wait_time_us: u64,
    /// Starved flag
    pub starved: bool,
    /// Starvation counter
    pub starvation_count: u32,
    /// Last scheduled time
    pub last_scheduled: u64,
}

impl ProcessFairness {
    pub fn new(pid: u64, group_id: u64) -> Self {
        Self {
            pid,
            group_id,
            shares: BTreeMap::new(),
            dominant_resource: FairnessResource::Cpu,
            dominant_ratio: 0.0,
            wait_time_us: 0,
            starved: false,
            starvation_count: 0,
            last_scheduled: 0,
        }
    }

    /// Set resource share
    #[inline(always)]
    pub fn set_share(&mut self, resource: FairnessResource, share: ResourceShare) {
        self.shares.insert(resource as u8, share);
    }

    /// Compute dominant resource (DRF)
    pub fn compute_dominant(&mut self, total_weights: &BTreeMap<u8, u32>) {
        let mut max_ratio = 0.0f64;
        let mut max_resource = FairnessResource::Cpu;

        for (&key, share) in &self.shares {
            let total_w = total_weights.get(&key).copied().unwrap_or(1024);
            let ratio = share.allocation_ratio(total_w);
            if ratio > max_ratio {
                max_ratio = ratio;
                max_resource = match key {
                    0 => FairnessResource::Cpu,
                    1 => FairnessResource::Memory,
                    2 => FairnessResource::DiskIo,
                    3 => FairnessResource::Network,
                    _ => FairnessResource::Cpu,
                };
            }
        }

        self.dominant_resource = max_resource;
        self.dominant_ratio = max_ratio;
    }
}

// ============================================================================
// GINI COEFFICIENT
// ============================================================================

/// Fairness metrics using Gini coefficient and Jain's index
#[repr(align(64))]
pub struct FairnessMetrics;

impl FairnessMetrics {
    /// Compute Gini coefficient (0 = perfect equality, 1 = perfect inequality)
    pub fn gini_coefficient(values: &[u64]) -> f64 {
        let n = values.len();
        if n < 2 {
            return 0.0;
        }

        let mut sorted = values.to_vec();
        sorted.sort();

        let sum: u64 = sorted.iter().sum();
        if sum == 0 {
            return 0.0;
        }

        let mut gini_num = 0.0f64;
        for (i, &val) in sorted.iter().enumerate() {
            gini_num += (2.0 * (i + 1) as f64 - n as f64 - 1.0) * val as f64;
        }

        gini_num / (n as f64 * sum as f64)
    }

    /// Compute Jain's fairness index (0 = unfair, 1 = perfectly fair)
    pub fn jains_index(values: &[u64]) -> f64 {
        let n = values.len();
        if n == 0 {
            return 1.0;
        }

        let sum: f64 = values.iter().map(|&v| v as f64).sum();
        let sum_sq: f64 = values.iter().map(|&v| (v as f64) * (v as f64)).sum();

        if sum_sq < 1e-10 {
            return 1.0;
        }

        (sum * sum) / (n as f64 * sum_sq)
    }

    /// Coefficient of variation (lower = more fair)
    pub fn coefficient_of_variation(values: &[u64]) -> f64 {
        let n = values.len();
        if n < 2 {
            return 0.0;
        }

        let mean: f64 = values.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
        if mean < 1e-10 {
            return 0.0;
        }

        let variance: f64 = values
            .iter()
            .map(|&v| {
                let d = v as f64 - mean;
                d * d
            })
            .sum::<f64>()
            / (n - 1) as f64;

        libm::sqrt(variance) / mean
    }
}

// ============================================================================
// ANTI-STARVATION
// ============================================================================

/// Starvation detection configuration
#[derive(Debug, Clone)]
pub struct StarvationConfig {
    /// Maximum wait time before boost (microseconds)
    pub max_wait_us: u64,
    /// Priority boost on starvation
    pub boost_amount: u32,
    /// Boost decay rate (per second)
    pub boost_decay: u32,
    /// Max consecutive starvations before escalation
    pub max_starvations: u32,
}

impl Default for StarvationConfig {
    fn default() -> Self {
        Self {
            max_wait_us: 100_000, // 100ms
            boost_amount: 5,
            boost_decay: 1,
            max_starvations: 10,
        }
    }
}

// ============================================================================
// FAIRNESS ENGINE
// ============================================================================

/// Fairness report
#[derive(Debug, Clone)]
pub struct FairnessReport {
    /// Gini coefficient per resource
    pub gini: BTreeMap<u8, f64>,
    /// Jain's index per resource
    pub jains: BTreeMap<u8, f64>,
    /// Starved process count
    pub starved_count: u32,
    /// Most over-allocated PID
    pub most_over_pid: Option<u64>,
    /// Most under-allocated PID
    pub most_under_pid: Option<u64>,
    /// Timestamp
    pub timestamp: u64,
}

/// Fairness enforcement engine
pub struct FairnessEngine {
    /// Per-process fairness state
    processes: BTreeMap<u64, ProcessFairness>,
    /// Total weights per resource
    total_weights: BTreeMap<u8, u32>,
    /// Starvation config
    starvation_config: StarvationConfig,
    /// Active boosts (pid → remaining boost)
    boosts: LinearMap<u32, 64>,
    /// Total evaluations
    pub total_evaluations: u64,
    /// Total anti-starvation boosts
    pub total_boosts: u64,
}

impl FairnessEngine {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            total_weights: BTreeMap::new(),
            starvation_config: StarvationConfig::default(),
            boosts: LinearMap::new(),
            total_evaluations: 0,
            total_boosts: 0,
        }
    }

    /// Register a process
    #[inline]
    pub fn register(&mut self, pid: u64, group_id: u64) {
        self.processes
            .entry(pid)
            .or_insert_with(|| ProcessFairness::new(pid, group_id));
    }

    /// Unregister a process
    #[inline]
    pub fn unregister(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.boosts.remove(pid);
        self.recalculate_weights();
    }

    /// Set resource share for a process
    #[inline]
    pub fn set_share(
        &mut self,
        pid: u64,
        resource: FairnessResource,
        weight: u32,
        total_capacity: u64,
    ) {
        if let Some(pf) = self.processes.get_mut(&pid) {
            pf.set_share(resource, ResourceShare::new(weight, total_capacity));
        }
        self.recalculate_weights();
    }

    /// Update current allocation
    #[inline]
    pub fn update_allocation(
        &mut self,
        pid: u64,
        resource: FairnessResource,
        allocation: u64,
    ) {
        if let Some(pf) = self.processes.get_mut(&pid) {
            if let Some(share) = pf.shares.get_mut(&(resource as u8)) {
                share.current_allocation = allocation;
            }
        }
    }

    /// Recalculate total weights
    fn recalculate_weights(&mut self) {
        self.total_weights.clear();
        for pf in self.processes.values() {
            for (&key, share) in &pf.shares {
                *self.total_weights.entry(key).or_insert(0) += share.weight;
            }
        }
    }

    /// Evaluate fairness
    pub fn evaluate(&mut self, timestamp: u64) -> FairnessReport {
        self.total_evaluations += 1;

        // Compute dominant resource for each process
        let total_weights = self.total_weights.clone();
        for pf in self.processes.values_mut() {
            pf.compute_dominant(&total_weights);
        }

        // Detect starvation
        for pf in self.processes.values_mut() {
            if pf.wait_time_us > self.starvation_config.max_wait_us {
                pf.starved = true;
                pf.starvation_count += 1;
            } else {
                pf.starved = false;
            }
        }

        // Build fairness report
        let mut gini = BTreeMap::new();
        let mut jains = BTreeMap::new();

        for resource_key in 0..4u8 {
            let allocations: Vec<u64> = self
                .processes
                .values()
                .filter_map(|pf| pf.shares.get(&resource_key))
                .map(|s| s.current_allocation)
                .collect();

            if !allocations.is_empty() {
                gini.insert(resource_key, FairnessMetrics::gini_coefficient(&allocations));
                jains.insert(resource_key, FairnessMetrics::jains_index(&allocations));
            }
        }

        let starved_count = self
            .processes
            .values()
            .filter(|p| p.starved)
            .count() as u32;

        let most_over_pid = self
            .processes
            .values()
            .max_by(|a, b| {
                a.dominant_ratio
                    .partial_cmp(&b.dominant_ratio)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|p| p.pid);

        let most_under_pid = self
            .processes
            .values()
            .filter(|p| p.dominant_ratio > 0.0)
            .min_by(|a, b| {
                a.dominant_ratio
                    .partial_cmp(&b.dominant_ratio)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|p| p.pid);

        FairnessReport {
            gini,
            jains,
            starved_count,
            most_over_pid,
            most_under_pid,
            timestamp,
        }
    }

    /// Apply anti-starvation boost
    pub fn apply_anti_starvation(&mut self) {
        let starved_pids: Vec<u64> = self
            .processes
            .values()
            .filter(|p| p.starved)
            .map(|p| p.pid)
            .collect();

        for pid in starved_pids {
            self.boosts.add(pid, self);
            self.total_boosts += 1;
        }
    }

    /// Get boost for process
    #[inline(always)]
    pub fn get_boost(&self, pid: u64) -> u32 {
        self.boosts.get(pid).copied().unwrap_or(0)
    }

    /// Decay boosts
    #[inline]
    pub fn decay_boosts(&mut self) {
        let decay = self.starvation_config.boost_decay;
        self.boosts.retain(|_, boost| {
            *boost = boost.saturating_sub(decay);
            *boost > 0
        });
    }

    /// Process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

// ============================================================================
// Merged from fairness_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FairnessResource {
    Cpu,
    Memory,
    IoReadBw,
    IoWriteBw,
    NetworkBw,
    GpuTime,
}

/// Fairness violation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairnessViolationSeverity {
    /// Minor deviation
    Low,
    /// Noticeable unfairness
    Medium,
    /// Severe unfairness
    High,
    /// Critical — some entities starved
    Critical,
}

/// Per-entity resource allocation
#[derive(Debug, Clone)]
pub struct EntityAllocation {
    pub entity_id: u64,
    pub weight: u32,
    /// Actual allocation per resource (fraction 0.0-1.0 of total)
    pub allocations: BTreeMap<FairnessResource, f64>,
    /// Demand per resource
    pub demands: BTreeMap<FairnessResource, f64>,
    /// Dominant resource share
    pub dominant_share: f64,
}

impl EntityAllocation {
    pub fn new(entity_id: u64, weight: u32) -> Self {
        Self {
            entity_id,
            weight,
            allocations: BTreeMap::new(),
            demands: BTreeMap::new(),
            dominant_share: 0.0,
        }
    }

    #[inline(always)]
    pub fn set_allocation(&mut self, resource: FairnessResource, fraction: f64) {
        self.allocations.insert(resource, fraction.clamp(0.0, 1.0));
        self.recompute_dominant();
    }

    #[inline(always)]
    pub fn set_demand(&mut self, resource: FairnessResource, fraction: f64) {
        self.demands.insert(resource, fraction.clamp(0.0, 1.0));
    }

    fn recompute_dominant(&mut self) {
        self.dominant_share = self
            .allocations
            .values()
            .copied()
            .fold(0.0f64, |a, b| if b > a { b } else { a });
    }

    /// Satisfaction ratio (avg allocation / demand)
    pub fn satisfaction(&self) -> f64 {
        let mut total_sat = 0.0;
        let mut count = 0;
        for (res, &alloc) in &self.allocations {
            if let Some(&demand) = self.demands.get(res) {
                if demand > 0.001 {
                    total_sat += (alloc / demand).min(1.0);
                    count += 1;
                }
            }
        }
        if count == 0 {
            1.0
        } else {
            total_sat / count as f64
        }
    }
}

/// Fairness violation
#[derive(Debug, Clone)]
pub struct FairnessViolation {
    pub entity_id: u64,
    pub resource: FairnessResource,
    pub expected_share: f64,
    pub actual_share: f64,
    pub severity: FairnessViolationSeverity,
    pub timestamp: u64,
}

/// Envy relationship
#[derive(Debug, Clone)]
pub struct EnvyPair {
    pub envier: u64,
    pub envied: u64,
    pub envy_degree: f64,
}

/// Max-min allocation result
#[derive(Debug, Clone)]
pub struct MaxMinResult {
    pub entity_id: u64,
    pub resource: FairnessResource,
    pub maxmin_share: f64,
    pub current_share: f64,
}

/// Fairness V2 stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticFairnessV2Stats {
    pub tracked_entities: usize,
    pub jain_index_cpu: f64,
    pub jain_index_memory: f64,
    pub jain_index_overall: f64,
    pub drf_max_dominant: f64,
    pub drf_min_dominant: f64,
    pub envy_pairs: usize,
    pub active_violations: usize,
    pub avg_satisfaction: f64,
}

/// Holistic Fairness V2 Engine
pub struct HolisticFairnessV2 {
    entities: BTreeMap<u64, EntityAllocation>,
    violations: Vec<FairnessViolation>,
    envy_pairs: Vec<EnvyPair>,
    violation_threshold: f64,
    stats: HolisticFairnessV2Stats,
}

impl HolisticFairnessV2 {
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            violations: Vec::new(),
            envy_pairs: Vec::new(),
            violation_threshold: 0.15, // 15% deviation triggers violation
            stats: HolisticFairnessV2Stats::default(),
        }
    }

    #[inline(always)]
    pub fn add_entity(&mut self, entity_id: u64, weight: u32) {
        self.entities
            .insert(entity_id, EntityAllocation::new(entity_id, weight));
    }

    #[inline(always)]
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.entities.remove(&entity_id);
    }

    #[inline]
    pub fn update_allocation(&mut self, entity_id: u64, resource: FairnessResource, fraction: f64) {
        if let Some(e) = self.entities.get_mut(&entity_id) {
            e.set_allocation(resource, fraction);
        }
    }

    #[inline]
    pub fn update_demand(&mut self, entity_id: u64, resource: FairnessResource, fraction: f64) {
        if let Some(e) = self.entities.get_mut(&entity_id) {
            e.set_demand(resource, fraction);
        }
    }

    /// Compute Jain's fairness index for a specific resource
    pub fn jain_index(&self, resource: FairnessResource) -> f64 {
        let shares: Vec<f64> = self
            .entities
            .values()
            .filter_map(|e| e.allocations.get(&resource).copied())
            .collect();
        if shares.is_empty() {
            return 1.0;
        }
        let n = shares.len() as f64;
        let sum: f64 = shares.iter().sum();
        let sum_sq: f64 = shares.iter().map(|x| x * x).sum();
        if sum_sq < 1e-12 {
            return 1.0;
        }
        (sum * sum) / (n * sum_sq)
    }

    /// Compute DRF dominant shares and find min/max
    pub fn drf_analysis(&self) -> (f64, f64) {
        let dominants: Vec<f64> = self.entities.values().map(|e| e.dominant_share).collect();
        if dominants.is_empty() {
            return (0.0, 0.0);
        }
        let max_d = dominants
            .iter()
            .copied()
            .fold(0.0f64, |a, b| if b > a { b } else { a });
        let min_d = dominants
            .iter()
            .copied()
            .fold(f64::MAX, |a, b| if b < a { b } else { a });
        (min_d, max_d)
    }

    /// Detect envy-freeness violations
    pub fn detect_envy(&mut self) {
        self.envy_pairs.clear();
        let ids: Vec<u64> = self.entities.keys().copied().collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = &self.entities[&ids[i]];
                let b = &self.entities[&ids[j]];

                // a envies b if a would prefer b's allocation
                let a_envy = self.compute_envy(a, b);
                let b_envy = self.compute_envy(b, a);

                if a_envy > 0.05 {
                    self.envy_pairs.push(EnvyPair {
                        envier: ids[i],
                        envied: ids[j],
                        envy_degree: a_envy,
                    });
                }
                if b_envy > 0.05 {
                    self.envy_pairs.push(EnvyPair {
                        envier: ids[j],
                        envied: ids[i],
                        envy_degree: b_envy,
                    });
                }
            }
        }
    }

    fn compute_envy(&self, envier: &EntityAllocation, envied: &EntityAllocation) -> f64 {
        // Envy = how much better envier would be with envied's allocation
        let own_sat = envier.satisfaction();
        // Simulate: would envier prefer envied's allocation?
        let mut simulated_sat = 0.0;
        let mut count = 0;
        for (res, &demand) in &envier.demands {
            if demand > 0.001 {
                let envied_alloc = envied.allocations.get(res).copied().unwrap_or(0.0);
                simulated_sat += (envied_alloc / demand).min(1.0);
                count += 1;
            }
        }
        let sim = if count == 0 {
            1.0
        } else {
            simulated_sat / count as f64
        };
        (sim - own_sat).max(0.0)
    }

    /// Detect violations: entities whose share deviates from weighted fair share
    pub fn detect_violations(&mut self, now: u64) {
        self.violations.clear();
        if self.entities.is_empty() {
            return;
        }

        let total_weight: u32 = self.entities.values().map(|e| e.weight).sum();
        if total_weight == 0 {
            return;
        }

        for (_, entity) in &self.entities {
            let expected_share = entity.weight as f64 / total_weight as f64;
            for (&resource, &actual) in &entity.allocations {
                let deviation = libm::fabs(actual - expected_share);
                if deviation > self.violation_threshold * expected_share.max(0.01) {
                    let severity = if deviation > 0.5 {
                        FairnessViolationSeverity::Critical
                    } else if deviation > 0.3 {
                        FairnessViolationSeverity::High
                    } else if deviation > 0.15 {
                        FairnessViolationSeverity::Medium
                    } else {
                        FairnessViolationSeverity::Low
                    };

                    self.violations.push(FairnessViolation {
                        entity_id: entity.entity_id,
                        resource,
                        expected_share,
                        actual_share: actual,
                        severity,
                        timestamp: now,
                    });
                }
            }
        }
    }

    /// Max-min fair share computation for a single resource
    pub fn maxmin_shares(&self, resource: FairnessResource) -> Vec<MaxMinResult> {
        let demands: Vec<(u64, f64)> = self
            .entities
            .iter()
            .map(|(&id, e)| (id, e.demands.get(&resource).copied().unwrap_or(0.0)))
            .collect();
        if demands.is_empty() {
            return Vec::new();
        }

        let n = demands.len() as f64;
        let total_capacity = 1.0; // normalized
        let equal_share = total_capacity / n;

        let mut results = Vec::new();
        for &(id, demand) in &demands {
            let maxmin = if demand <= equal_share {
                demand
            } else {
                equal_share
            };
            let current = self
                .entities
                .get(&id)
                .and_then(|e| e.allocations.get(&resource).copied())
                .unwrap_or(0.0);
            results.push(MaxMinResult {
                entity_id: id,
                resource,
                maxmin_share: maxmin,
                current_share: current,
            });
        }
        results
    }

    /// Full recompute of all fairness metrics
    pub fn recompute(&mut self, now: u64) {
        self.detect_violations(now);
        self.detect_envy();

        let jain_cpu = self.jain_index(FairnessResource::Cpu);
        let jain_mem = self.jain_index(FairnessResource::Memory);
        let (drf_min, drf_max) = self.drf_analysis();

        let avg_sat = if self.entities.is_empty() {
            1.0
        } else {
            self.entities
                .values()
                .map(|e| e.satisfaction())
                .sum::<f64>()
                / self.entities.len() as f64
        };

        self.stats = HolisticFairnessV2Stats {
            tracked_entities: self.entities.len(),
            jain_index_cpu: jain_cpu,
            jain_index_memory: jain_mem,
            jain_index_overall: (jain_cpu + jain_mem) / 2.0,
            drf_max_dominant: drf_max,
            drf_min_dominant: drf_min,
            envy_pairs: self.envy_pairs.len(),
            active_violations: self.violations.len(),
            avg_satisfaction: avg_sat,
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticFairnessV2Stats {
        &self.stats
    }

    #[inline(always)]
    pub fn violations(&self) -> &[FairnessViolation] {
        &self.violations
    }

    #[inline(always)]
    pub fn envy_pairs(&self) -> &[EnvyPair] {
        &self.envy_pairs
    }
}

//! # Cooperative Fair-Share Scheduling
//!
//! Proportional fair sharing of resources:
//! - Weighted fair queuing
//! - Hierarchical fair share
//! - Lag tracking for fairness
//! - Virtual time computation
//! - Share redistribution
//! - Starvation detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SHARE TYPES
// ============================================================================

/// Share weight
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShareWeight(pub u32);

impl ShareWeight {
    pub const MIN: ShareWeight = ShareWeight(1);
    pub const DEFAULT: ShareWeight = ShareWeight(1024);
    pub const MAX: ShareWeight = ShareWeight(65536);

    /// Proportion of total
    #[inline]
    pub fn proportion(&self, total_weight: u32) -> f64 {
        if total_weight == 0 {
            return 0.0;
        }
        self.0 as f64 / total_weight as f64
    }
}

// ============================================================================
// VIRTUAL TIME
// ============================================================================

/// Virtual time counter
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualTime(pub u64);

impl VirtualTime {
    #[inline(always)]
    pub fn zero() -> Self {
        Self(0)
    }

    /// Advance by wall-clock time weighted by shares
    #[inline]
    pub fn advance(&mut self, wall_us: u64, weight: ShareWeight, total_weight: u32) {
        if weight.0 == 0 || total_weight == 0 {
            return;
        }
        // Virtual time advances faster for lower-weight entities
        let vt_increment = wall_us * total_weight as u64 / weight.0 as u64;
        self.0 += vt_increment;
    }

    /// Lag relative to ideal fair share
    #[inline(always)]
    pub fn lag(&self, ideal: &VirtualTime) -> i64 {
        ideal.0 as i64 - self.0 as i64
    }
}

// ============================================================================
// FAIR SHARE ENTITY
// ============================================================================

/// Fair share entity (process or group)
#[derive(Debug, Clone)]
pub struct FairShareEntity {
    /// Entity ID
    pub id: u64,
    /// Is group (contains children)
    pub is_group: bool,
    /// Parent group ID
    pub parent: Option<u64>,
    /// Share weight
    pub weight: ShareWeight,
    /// Virtual runtime
    pub vruntime: VirtualTime,
    /// Actual CPU time used (us)
    pub actual_us: u64,
    /// Ideal CPU time (based on shares)
    pub ideal_us: u64,
    /// Lag (ideal - actual, positive = under-served)
    pub lag_us: i64,
    /// Active (runnable)
    pub active: bool,
    /// Children (if group)
    pub children: Vec<u64>,
    /// Min vruntime seen
    pub min_vruntime: VirtualTime,
}

impl FairShareEntity {
    pub fn new_process(id: u64, weight: ShareWeight) -> Self {
        Self {
            id,
            is_group: false,
            parent: None,
            weight,
            vruntime: VirtualTime::zero(),
            actual_us: 0,
            ideal_us: 0,
            lag_us: 0,
            active: true,
            children: Vec::new(),
            min_vruntime: VirtualTime::zero(),
        }
    }

    pub fn new_group(id: u64, weight: ShareWeight) -> Self {
        Self {
            id,
            is_group: true,
            parent: None,
            weight,
            vruntime: VirtualTime::zero(),
            actual_us: 0,
            ideal_us: 0,
            lag_us: 0,
            active: true,
            children: Vec::new(),
            min_vruntime: VirtualTime::zero(),
        }
    }

    /// Fairness ratio (actual/ideal, 1.0 = perfect)
    #[inline]
    pub fn fairness_ratio(&self) -> f64 {
        if self.ideal_us == 0 {
            return 1.0;
        }
        self.actual_us as f64 / self.ideal_us as f64
    }

    /// Is under-served
    #[inline(always)]
    pub fn is_underserved(&self) -> bool {
        self.lag_us > 0
    }

    /// Is over-served
    #[inline(always)]
    pub fn is_overserved(&self) -> bool {
        self.lag_us < 0
    }
}

// ============================================================================
// FAIRNESS METRICS
// ============================================================================

/// System-wide fairness metrics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FairnessMetrics {
    /// Jain's fairness index (0-1, 1 = perfect)
    pub jains_index: f64,
    /// Max-min fairness violation (max over-service %)
    pub max_overservice_pct: f64,
    /// Max under-service (%)
    pub max_underservice_pct: f64,
    /// Number of starved entities
    pub starved_count: usize,
    /// Total entities
    pub total_entities: usize,
}

impl FairnessMetrics {
    /// Compute from entity list
    pub fn compute(entities: &[&FairShareEntity]) -> Self {
        if entities.is_empty() {
            return Self {
                jains_index: 1.0,
                max_overservice_pct: 0.0,
                max_underservice_pct: 0.0,
                starved_count: 0,
                total_entities: 0,
            };
        }

        let n = entities.len() as f64;
        let ratios: Vec<f64> = entities.iter().map(|e| e.fairness_ratio()).collect();

        let sum: f64 = ratios.iter().sum();
        let sum_sq: f64 = ratios.iter().map(|r| r * r).sum();

        let jains = if sum_sq * n > 0.0 {
            (sum * sum) / (n * sum_sq)
        } else {
            1.0
        };

        let max_over = ratios.iter().fold(0.0f64, |a, &r| {
            let over = (r - 1.0) * 100.0;
            if over > a { over } else { a }
        });

        let max_under = ratios.iter().fold(0.0f64, |a, &r| {
            let under = (1.0 - r) * 100.0;
            if under > a { under } else { a }
        });

        let starved = entities
            .iter()
            .filter(|e| e.actual_us == 0 && e.ideal_us > 0)
            .count();

        Self {
            jains_index: jains,
            max_overservice_pct: max_over,
            max_underservice_pct: max_under,
            starved_count: starved,
            total_entities: entities.len(),
        }
    }
}

// ============================================================================
// FAIR SHARE SCHEDULER
// ============================================================================

/// Fair share scheduler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FairShareStats {
    /// Total entities
    pub total_entities: usize,
    /// Active entities
    pub active_entities: usize,
    /// Groups
    pub groups: usize,
    /// Jain's fairness index
    pub jains_index_pct: u32,
    /// Context switches for fairness
    pub fairness_switches: u64,
    /// Redistributions
    pub redistributions: u64,
}

/// Cooperative fair-share scheduler
pub struct CoopFairShareScheduler {
    /// Entities by ID
    entities: BTreeMap<u64, FairShareEntity>,
    /// Root group IDs
    root_groups: Vec<u64>,
    /// Total weight (active only)
    total_weight: u32,
    /// Global virtual time
    global_vtime: VirtualTime,
    /// Scheduling period (us)
    period_us: u64,
    /// Minimum granularity (us)
    min_granularity_us: u64,
    /// Starvation threshold (us without running)
    starvation_threshold_us: u64,
    /// Stats
    stats: FairShareStats,
    /// Next entity ID
    next_id: u64,
}

impl CoopFairShareScheduler {
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            root_groups: Vec::new(),
            total_weight: 0,
            global_vtime: VirtualTime::zero(),
            period_us: 10_000, // 10ms
            min_granularity_us: 750,
            starvation_threshold_us: 100_000,
            stats: FairShareStats::default(),
            next_id: 1,
        }
    }

    /// Add process
    pub fn add_process(&mut self, weight: ShareWeight, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut entity = FairShareEntity::new_process(id, weight);
        entity.parent = parent;
        entity.vruntime = self.global_vtime;

        if let Some(parent_id) = parent {
            if let Some(group) = self.entities.get_mut(&parent_id) {
                group.children.push(id);
            }
        }

        self.entities.insert(id, entity);
        self.recalculate_weights();
        self.update_stats();
        id
    }

    /// Add group
    pub fn add_group(&mut self, weight: ShareWeight, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut entity = FairShareEntity::new_group(id, weight);
        entity.parent = parent;

        if parent.is_none() {
            self.root_groups.push(id);
        } else if let Some(parent_id) = parent {
            if let Some(group) = self.entities.get_mut(&parent_id) {
                group.children.push(id);
            }
        }

        self.entities.insert(id, entity);
        self.update_stats();
        id
    }

    /// Pick next entity to run (lowest vruntime)
    pub fn pick_next(&self) -> Option<u64> {
        self.entities
            .iter()
            .filter(|(_, e)| e.active && !e.is_group && e.vruntime.0 > 0)
            .min_by_key(|(_, e)| e.vruntime)
            .map(|(id, _)| *id)
            .or_else(|| {
                // Fallback: any active process
                self.entities
                    .iter()
                    .filter(|(_, e)| e.active && !e.is_group)
                    .min_by_key(|(_, e)| e.vruntime)
                    .map(|(id, _)| *id)
            })
    }

    /// Account execution time
    pub fn account(&mut self, entity_id: u64, wall_us: u64) {
        let tw = self.total_weight;

        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.vruntime.advance(wall_us, entity.weight, tw);
            entity.actual_us += wall_us;
        }

        // Advance global time
        if tw > 0 {
            self.global_vtime.0 += wall_us;
        }

        // Update ideal times for all active entities
        let total_wall: u64 = self.entities.values().map(|e| e.actual_us).sum();
        for entity in self.entities.values_mut() {
            if entity.active && !entity.is_group && tw > 0 {
                entity.ideal_us =
                    (total_wall as f64 * entity.weight.proportion(tw)) as u64;
                entity.lag_us = entity.ideal_us as i64 - entity.actual_us as i64;
            }
        }
    }

    /// Timeslice for entity
    pub fn timeslice_us(&self, entity_id: u64) -> u64 {
        if let Some(entity) = self.entities.get(&entity_id) {
            if self.total_weight == 0 {
                return self.period_us;
            }
            let share = entity.weight.proportion(self.total_weight);
            let slice = (self.period_us as f64 * share) as u64;
            slice.max(self.min_granularity_us)
        } else {
            self.min_granularity_us
        }
    }

    /// Detect starvation
    #[inline]
    pub fn detect_starvation(&self) -> Vec<u64> {
        let threshold = self.starvation_threshold_us;
        self.entities
            .iter()
            .filter(|(_, e)| {
                e.active && !e.is_group && e.actual_us == 0 && e.ideal_us > threshold
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Compute fairness metrics
    #[inline]
    pub fn fairness_metrics(&self) -> FairnessMetrics {
        let active: Vec<&FairShareEntity> = self
            .entities
            .values()
            .filter(|e| e.active && !e.is_group)
            .collect();
        FairnessMetrics::compute(&active)
    }

    /// Set entity active/inactive
    #[inline]
    pub fn set_active(&mut self, entity_id: u64, active: bool) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.active = active;
            if active {
                // Place at current global vruntime to avoid starvation
                entity.vruntime = self.global_vtime;
            }
        }
        self.recalculate_weights();
        self.update_stats();
    }

    /// Change weight
    #[inline]
    pub fn set_weight(&mut self, entity_id: u64, weight: ShareWeight) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.weight = weight;
        }
        self.recalculate_weights();
    }

    fn recalculate_weights(&mut self) {
        self.total_weight = self
            .entities
            .values()
            .filter(|e| e.active && !e.is_group)
            .map(|e| e.weight.0)
            .sum();
    }

    fn update_stats(&mut self) {
        self.stats.total_entities = self.entities.len();
        self.stats.active_entities = self.entities.values().filter(|e| e.active && !e.is_group).count();
        self.stats.groups = self.entities.values().filter(|e| e.is_group).count();

        let metrics = self.fairness_metrics();
        self.stats.jains_index_pct = (metrics.jains_index * 100.0) as u32;
    }

    /// Get entity
    #[inline(always)]
    pub fn entity(&self, id: u64) -> Option<&FairShareEntity> {
        self.entities.get(&id)
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &FairShareStats {
        &self.stats
    }

    /// Remove entity
    pub fn remove(&mut self, id: u64) {
        if let Some(entity) = self.entities.remove(&id) {
            // Remove from parent
            if let Some(parent_id) = entity.parent {
                if let Some(parent) = self.entities.get_mut(&parent_id) {
                    parent.children.retain(|&c| c != id);
                }
            }
            self.root_groups.retain(|&g| g != id);
        }
        self.recalculate_weights();
        self.update_stats();
    }
}

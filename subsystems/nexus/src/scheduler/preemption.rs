//! Preemption intelligence.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::WorkloadType;
use crate::core::NexusTimestamp;

// ============================================================================
// PREEMPTION INTELLIGENCE
// ============================================================================

/// Smart preemption decision maker
pub struct PreemptionIntelligence {
    /// Preemption history
    history: Vec<PreemptionRecord>,
    /// Max history size
    max_history: usize,
    /// Learned cost model
    cost_model: PreemptionCostModel,
}

/// Record of a preemption event
#[derive(Debug, Clone)]
struct PreemptionRecord {
    #[allow(dead_code)]
    preempted_type: WorkloadType,
    #[allow(dead_code)]
    preemptor_type: WorkloadType,
    #[allow(dead_code)]
    cache_state: CacheState,
    #[allow(dead_code)]
    overhead: u64,
    #[allow(dead_code)]
    timestamp: NexusTimestamp,
}

/// Cache state before preemption
#[derive(Debug, Clone)]
struct CacheState {
    l1_dirty: u32,
    l2_dirty: u32,
    l3_working_set: u64,
}

/// Preemption cost model
#[derive(Debug, Clone)]
struct PreemptionCostModel {
    base_costs: BTreeMap<u8, u64>,
    #[allow(dead_code)]
    cache_multiplier: f64,
    #[allow(dead_code)]
    numa_cost: u64,
}

impl Default for PreemptionCostModel {
    fn default() -> Self {
        let mut base_costs = BTreeMap::new();
        base_costs.insert(WorkloadType::CpuBound as u8, 5000);
        base_costs.insert(WorkloadType::IoBound as u8, 1000);
        base_costs.insert(WorkloadType::MemoryBound as u8, 10000);
        base_costs.insert(WorkloadType::Interactive as u8, 500);
        base_costs.insert(WorkloadType::RealTime as u8, 200);
        base_costs.insert(WorkloadType::Background as u8, 2000);

        Self {
            base_costs,
            cache_multiplier: 1.5,
            numa_cost: 50000,
        }
    }
}

impl PreemptionIntelligence {
    /// Create new preemption intelligence
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 1000,
            cost_model: PreemptionCostModel::default(),
        }
    }

    /// Estimate preemption cost
    pub fn estimate_cost(
        &self,
        preempted: WorkloadType,
        preemptor: WorkloadType,
        runtime_so_far: u64,
    ) -> u64 {
        let base = self
            .cost_model
            .base_costs
            .get(&(preempted as u8))
            .copied()
            .unwrap_or(3000);

        let runtime_factor = (runtime_so_far as f64 / 10000.0).min(3.0);

        let interaction_cost = match (preempted, preemptor) {
            (WorkloadType::MemoryBound, WorkloadType::MemoryBound) => 2.0,
            (WorkloadType::CpuBound, WorkloadType::Interactive) => 1.5,
            (WorkloadType::RealTime, _) => 0.5,
            _ => 1.0,
        };

        (base as f64 * runtime_factor * interaction_cost) as u64
    }

    /// Should preempt current task?
    pub fn should_preempt(
        &self,
        current: WorkloadType,
        current_runtime: u64,
        current_remaining: u64,
        incoming: WorkloadType,
        incoming_priority: i32,
        current_priority: i32,
    ) -> bool {
        let priority_diff = incoming_priority - current_priority;

        if incoming == WorkloadType::RealTime && current != WorkloadType::RealTime {
            return true;
        }

        if current == WorkloadType::RealTime && incoming != WorkloadType::RealTime {
            return false;
        }

        let cost = self.estimate_cost(current, incoming, current_runtime);

        if incoming == WorkloadType::Interactive && current_runtime > 10000 {
            return true;
        }

        if priority_diff > 5 {
            return true;
        }

        if current_remaining < cost {
            return false;
        }

        priority_diff > 0
    }

    /// Record preemption outcome
    pub fn record_preemption(
        &mut self,
        preempted: WorkloadType,
        preemptor: WorkloadType,
        actual_overhead: u64,
    ) {
        let record = PreemptionRecord {
            preempted_type: preempted,
            preemptor_type: preemptor,
            cache_state: CacheState {
                l1_dirty: 0,
                l2_dirty: 0,
                l3_working_set: 0,
            },
            overhead: actual_overhead,
            timestamp: NexusTimestamp::now(),
        };

        self.history.push(record);

        let key = preempted as u8;
        let current_cost = self
            .cost_model
            .base_costs
            .get(&key)
            .copied()
            .unwrap_or(3000);
        let new_cost = (current_cost as f64 * 0.9 + actual_overhead as f64 * 0.1) as u64;
        self.cost_model.base_costs.insert(key, new_cost);

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
}

impl Default for PreemptionIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

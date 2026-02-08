// SPDX-License-Identifier: GPL-2.0
//! Coop fair_share — weighted fair sharing scheduler.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Share type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareType {
    Proportional,
    MaxMin,
    DominantResource,
    WeightedFair,
}

/// Entity state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareEntityState {
    Active,
    Idle,
    Starved,
    Throttled,
    Removed,
}

/// Resource dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDim {
    Cpu,
    Memory,
    Io,
    Network,
    Custom(u32),
}

/// Share entity
#[derive(Debug, Clone)]
pub struct ShareEntity {
    pub id: u64,
    pub weight: u32,
    pub state: ShareEntityState,
    pub vruntime: u64,
    pub actual_runtime: u64,
    pub allocated: BTreeMap<u8, u64>,
    pub demand: BTreeMap<u8, u64>,
    pub min_share: BTreeMap<u8, u64>,
    pub max_share: BTreeMap<u8, u64>,
    pub starvation_ns: u64,
}

impl ShareEntity {
    pub fn new(id: u64, weight: u32) -> Self {
        Self {
            id, weight: weight.max(1), state: ShareEntityState::Active,
            vruntime: 0, actual_runtime: 0,
            allocated: BTreeMap::new(), demand: BTreeMap::new(),
            min_share: BTreeMap::new(), max_share: BTreeMap::new(),
            starvation_ns: 0,
        }
    }

    pub fn update_vruntime(&mut self, delta_ns: u64) {
        let weighted = if self.weight == 0 { delta_ns } else {
            (delta_ns * 1024) / self.weight as u64
        };
        self.vruntime += weighted;
        self.actual_runtime += delta_ns;
    }

    pub fn fair_ratio(&self, total_weight: u32) -> f64 {
        if total_weight == 0 { return 0.0; }
        self.weight as f64 / total_weight as f64
    }

    pub fn satisfaction(&self, dim: u8) -> f64 {
        let demand = self.demand.get(&dim).copied().unwrap_or(0);
        let alloc = self.allocated.get(&dim).copied().unwrap_or(0);
        if demand == 0 { return 1.0; }
        alloc as f64 / demand as f64
    }
}

/// Allocation result
#[derive(Debug, Clone)]
pub struct AllocationResult {
    pub entity_id: u64,
    pub allocations: BTreeMap<u8, u64>,
    pub satisfied: bool,
}

/// Stats
#[derive(Debug, Clone)]
pub struct FairShareStats {
    pub total_entities: u32,
    pub active_entities: u32,
    pub starved_entities: u32,
    pub total_weight: u32,
    pub avg_satisfaction: f64,
    pub jains_fairness: f64,
}

/// Main fair share scheduler
pub struct CoopFairShare {
    entities: BTreeMap<u64, ShareEntity>,
    total_resources: BTreeMap<u8, u64>,
    share_type: ShareType,
    next_id: u64,
}

impl CoopFairShare {
    pub fn new(share_type: ShareType) -> Self {
        Self { entities: BTreeMap::new(), total_resources: BTreeMap::new(), share_type, next_id: 1 }
    }

    pub fn add_entity(&mut self, weight: u32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(id, ShareEntity::new(id, weight));
        id
    }

    pub fn set_demand(&mut self, entity: u64, dim: u8, amount: u64) {
        if let Some(e) = self.entities.get_mut(&entity) { e.demand.insert(dim, amount); }
    }

    pub fn set_total_resource(&mut self, dim: u8, amount: u64) {
        self.total_resources.insert(dim, amount);
    }

    pub fn allocate(&mut self, dim: u8) -> Vec<AllocationResult> {
        let total = self.total_resources.get(&dim).copied().unwrap_or(0);
        let total_weight: u32 = self.entities.values().filter(|e| e.state == ShareEntityState::Active).map(|e| e.weight).sum();
        let mut results = Vec::new();

        for entity in self.entities.values_mut() {
            if entity.state != ShareEntityState::Active { continue; }
            let fair = if total_weight == 0 { 0 } else {
                (total * entity.weight as u64) / total_weight as u64
            };
            let demand = entity.demand.get(&dim).copied().unwrap_or(0);
            let alloc = fair.min(demand);
            entity.allocated.insert(dim, alloc);
            results.push(AllocationResult {
                entity_id: entity.id,
                allocations: {
                    let mut m = BTreeMap::new();
                    m.insert(dim, alloc);
                    m
                },
                satisfied: alloc >= demand,
            });
        }
        results
    }

    pub fn stats(&self) -> FairShareStats {
        let active = self.entities.values().filter(|e| e.state == ShareEntityState::Active).count() as u32;
        let starved = self.entities.values().filter(|e| e.state == ShareEntityState::Starved).count() as u32;
        let total_weight: u32 = self.entities.values().map(|e| e.weight).sum();

        // Jain's fairness index: (sum(xi))^2 / (n * sum(xi^2))
        let satisfactions: Vec<f64> = self.entities.values()
            .filter(|e| e.state == ShareEntityState::Active)
            .map(|e| {
                let dims: Vec<f64> = e.demand.keys().map(|d| e.satisfaction(*d)).collect();
                if dims.is_empty() { 1.0 } else { dims.iter().sum::<f64>() / dims.len() as f64 }
            }).collect();
        let n = satisfactions.len() as f64;
        let sum: f64 = satisfactions.iter().sum();
        let sum_sq: f64 = satisfactions.iter().map(|x| x * x).sum();
        let jains = if n == 0.0 || sum_sq == 0.0 { 1.0 } else { (sum * sum) / (n * sum_sq) };
        let avg_sat = if n == 0.0 { 1.0 } else { sum / n };

        FairShareStats {
            total_entities: self.entities.len() as u32, active_entities: active,
            starved_entities: starved, total_weight,
            avg_satisfaction: avg_sat, jains_fairness: jains,
        }
    }
}

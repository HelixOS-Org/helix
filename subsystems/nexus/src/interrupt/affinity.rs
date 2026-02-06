//! IRQ affinity optimization
//!
//! This module provides NUMA-aware IRQ affinity optimization to balance
//! interrupt load across CPUs while respecting memory locality constraints.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::stats::IrqStats;
use super::types::{CpuId, Irq};
use crate::core::NexusTimestamp;

/// Optimizes IRQ to CPU affinity
pub struct AffinityOptimizer {
    /// Current affinities
    affinities: BTreeMap<Irq, Vec<CpuId>>,
    /// CPU loads
    cpu_loads: BTreeMap<CpuId, f64>,
    /// IRQ stats
    irq_stats: BTreeMap<Irq, IrqStats>,
    /// NUMA topology
    numa_nodes: Vec<Vec<CpuId>>,
    /// Optimization history
    history: Vec<AffinityChange>,
    /// Max history
    max_history: usize,
}

/// Affinity change record
#[derive(Debug, Clone)]
pub struct AffinityChange {
    /// IRQ
    pub irq: Irq,
    /// Old CPUs
    pub old_cpus: Vec<CpuId>,
    /// New CPUs
    pub new_cpus: Vec<CpuId>,
    /// Reason
    pub reason: String,
    /// Timestamp
    pub timestamp: u64,
}

impl AffinityOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            affinities: BTreeMap::new(),
            cpu_loads: BTreeMap::new(),
            irq_stats: BTreeMap::new(),
            numa_nodes: Vec::new(),
            history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Set NUMA topology
    pub fn set_numa_topology(&mut self, nodes: Vec<Vec<CpuId>>) {
        self.numa_nodes = nodes;
    }

    /// Update CPU load
    pub fn update_cpu_load(&mut self, cpu: CpuId, load: f64) {
        self.cpu_loads.insert(cpu, load);
    }

    /// Update IRQ stats
    pub fn update_irq_stats(&mut self, irq: Irq, stats: IrqStats) {
        self.irq_stats.insert(irq, stats);
    }

    /// Set current affinity
    pub fn set_affinity(&mut self, irq: Irq, cpus: Vec<CpuId>) {
        self.affinities.insert(irq, cpus);
    }

    /// Get optimized affinity for IRQ
    pub fn optimize(&mut self, irq: Irq) -> Option<Vec<CpuId>> {
        let current = self.affinities.get(&irq)?.clone();
        let stats = self.irq_stats.get(&irq)?;

        // Check if rebalancing is needed
        let imbalance = stats.load_imbalance();
        if imbalance < 0.3 {
            return None; // Already balanced
        }

        // Find least loaded CPUs
        let mut candidates: Vec<_> = self.cpu_loads.iter().collect();
        candidates.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

        // NUMA-aware selection
        let new_cpus: Vec<CpuId> = if self.numa_nodes.is_empty() {
            // No NUMA, just pick least loaded
            candidates
                .iter()
                .take(current.len())
                .map(|&(&cpu, _)| cpu)
                .collect()
        } else {
            // NUMA-aware: try to keep on same node
            let current_node = self.find_numa_node(current.first().copied().unwrap_or(0));
            let node_cpus: Vec<_> = self
                .numa_nodes
                .get(current_node as usize)
                .cloned()
                .unwrap_or_default();

            let mut selected = Vec::new();
            // First, try CPUs from same NUMA node
            for &cpu in &node_cpus {
                if selected.len() >= current.len() {
                    break;
                }
                if !selected.contains(&cpu) {
                    selected.push(cpu);
                }
            }

            // Fill remaining from other nodes
            for &(&cpu, _) in &candidates {
                if selected.len() >= current.len() {
                    break;
                }
                if !selected.contains(&cpu) {
                    selected.push(cpu);
                }
            }

            selected
        };

        if new_cpus != current {
            self.record_change(irq, current, new_cpus.clone(), "Load balancing");
            Some(new_cpus)
        } else {
            None
        }
    }

    /// Find NUMA node for CPU
    fn find_numa_node(&self, cpu: CpuId) -> u32 {
        for (node, cpus) in self.numa_nodes.iter().enumerate() {
            if cpus.contains(&cpu) {
                return node as u32;
            }
        }
        0
    }

    /// Record affinity change
    fn record_change(&mut self, irq: Irq, old: Vec<CpuId>, new: Vec<CpuId>, reason: &str) {
        self.history.push(AffinityChange {
            irq,
            old_cpus: old,
            new_cpus: new.clone(),
            reason: String::from(reason),
            timestamp: NexusTimestamp::now().raw(),
        });

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        self.affinities.insert(irq, new);
    }

    /// Get change history
    pub fn history(&self) -> &[AffinityChange] {
        &self.history
    }

    /// Get current affinity
    pub fn get_affinity(&self, irq: Irq) -> Option<&Vec<CpuId>> {
        self.affinities.get(&irq)
    }
}

impl Default for AffinityOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

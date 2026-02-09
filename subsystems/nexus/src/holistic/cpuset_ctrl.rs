// SPDX-License-Identifier: GPL-2.0
//! Holistic cpuset_ctrl — cpuset controller for CPU/memory affinity management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Cpuset partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpusetPartition {
    /// Member (default)
    Member,
    /// Root partition
    Root,
    /// Isolated partition
    Isolated,
}

/// CPU distribution policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuDistPolicy {
    /// Spread tasks across CPUs
    Spread,
    /// Pack tasks on fewest CPUs
    Pack,
    /// Let scheduler decide
    Auto,
}

/// Memory placement policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemPlacePolicy {
    /// Allocate from cpuset's nodes
    Default,
    /// Spread across nodes round-robin
    Interleave,
    /// Prefer local node
    PreferLocal,
    /// Bind to specific node
    Bind,
}

/// A cpuset
#[derive(Debug)]
pub struct Cpuset {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub cpus: Vec<u32>,
    pub mems: Vec<u32>,
    pub partition: CpusetPartition,
    pub cpu_exclusive: bool,
    pub mem_exclusive: bool,
    pub mem_hardwall: bool,
    pub sched_load_balance: bool,
    pub spread_page: bool,
    pub spread_slab: bool,
    pub cpu_dist: CpuDistPolicy,
    pub mem_place: MemPlacePolicy,
    pub task_count: u32,
    pub children: Vec<u64>,
    pub effective_cpus: Vec<u32>,
    pub effective_mems: Vec<u32>,
}

impl Cpuset {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id, name, parent_id: None,
            cpus: Vec::new(), mems: Vec::new(),
            partition: CpusetPartition::Member,
            cpu_exclusive: false, mem_exclusive: false,
            mem_hardwall: false, sched_load_balance: true,
            spread_page: false, spread_slab: false,
            cpu_dist: CpuDistPolicy::Auto,
            mem_place: MemPlacePolicy::Default,
            task_count: 0, children: Vec::new(),
            effective_cpus: Vec::new(), effective_mems: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn cpu_count(&self) -> usize {
        self.effective_cpus.len()
    }

    #[inline(always)]
    pub fn mem_node_count(&self) -> usize {
        self.effective_mems.len()
    }

    #[inline(always)]
    pub fn has_cpu(&self, cpu: u32) -> bool {
        self.effective_cpus.contains(&cpu)
    }

    #[inline(always)]
    pub fn has_mem_node(&self, node: u32) -> bool {
        self.effective_mems.contains(&node)
    }

    #[inline(always)]
    pub fn is_root_partition(&self) -> bool {
        self.partition == CpusetPartition::Root
    }

    #[inline]
    pub fn tasks_per_cpu(&self) -> f64 {
        let cpus = self.cpu_count();
        if cpus == 0 { return 0.0; }
        self.task_count as f64 / cpus as f64
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.task_count == 0 && self.children.is_empty()
    }

    #[inline]
    pub fn recalculate_effective(&mut self, parent_cpus: &[u32], parent_mems: &[u32]) {
        self.effective_cpus = self.cpus.iter()
            .filter(|c| parent_cpus.contains(c))
            .copied()
            .collect();
        self.effective_mems = self.mems.iter()
            .filter(|m| parent_mems.contains(m))
            .copied()
            .collect();
    }
}

/// Migration caused by cpuset changes
#[derive(Debug, Clone)]
pub struct CpusetMigration {
    pub pid: u32,
    pub from_cpuset: u64,
    pub to_cpuset: u64,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub timestamp: u64,
}

/// Cpuset violation
#[derive(Debug, Clone)]
pub struct CpusetViolation {
    pub pid: u32,
    pub cpuset_id: u64,
    pub violation_type: ViolationType,
    pub timestamp: u64,
}

/// Violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    CpuOutOfSet,
    MemOutOfSet,
    ExclusiveConflict,
    EmptyEffective,
}

/// Cpuset stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpusetStats {
    pub total_cpusets: u32,
    pub root_partitions: u32,
    pub isolated_partitions: u32,
    pub total_tasks_managed: u64,
    pub migrations: u64,
    pub violations: u64,
    pub exclusive_cpus: u32,
}

/// Main cpuset controller
pub struct HolisticCpusetCtrl {
    cpusets: BTreeMap<u64, Cpuset>,
    migrations: VecDeque<CpusetMigration>,
    violations: VecDeque<CpusetViolation>,
    max_history: usize,
    next_id: u64,
    stats: CpusetStats,
    system_cpus: Vec<u32>,
    system_mems: Vec<u32>,
}

impl HolisticCpusetCtrl {
    pub fn new(system_cpus: Vec<u32>, system_mems: Vec<u32>) -> Self {
        Self {
            cpusets: BTreeMap::new(),
            migrations: VecDeque::new(),
            violations: VecDeque::new(),
            max_history: 2048,
            next_id: 1,
            stats: CpusetStats {
                total_cpusets: 0, root_partitions: 0,
                isolated_partitions: 0, total_tasks_managed: 0,
                migrations: 0, violations: 0, exclusive_cpus: 0,
            },
            system_cpus,
            system_mems,
        }
    }

    pub fn create_cpuset(&mut self, name: String, parent_id: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut cs = Cpuset::new(id, name);
        cs.parent_id = parent_id;
        // inherit from parent
        if let Some(pid) = parent_id {
            if let Some(parent) = self.cpusets.get(&pid) {
                cs.cpus = parent.effective_cpus.clone();
                cs.mems = parent.effective_mems.clone();
                cs.effective_cpus = parent.effective_cpus.clone();
                cs.effective_mems = parent.effective_mems.clone();
            }
        } else {
            cs.cpus = self.system_cpus.clone();
            cs.mems = self.system_mems.clone();
            cs.effective_cpus = self.system_cpus.clone();
            cs.effective_mems = self.system_mems.clone();
        }
        self.cpusets.insert(id, cs);
        if let Some(pid) = parent_id {
            if let Some(parent) = self.cpusets.get_mut(&pid) {
                parent.children.push(id);
            }
        }
        self.stats.total_cpusets += 1;
        id
    }

    pub fn set_cpus(&mut self, cpuset_id: u64, cpus: Vec<u32>) {
        if let Some(cs) = self.cpusets.get_mut(&cpuset_id) {
            cs.cpus = cpus;
            // recalculate effective
            let parent_cpus = cs.parent_id
                .and_then(|pid| self.cpusets.get(&pid))
                .map(|p| p.effective_cpus.clone())
                .unwrap_or_else(|| self.system_cpus.clone());
            let parent_mems = cs.parent_id
                .and_then(|pid| self.cpusets.get(&pid))
                .map(|p| p.effective_mems.clone())
                .unwrap_or_else(|| self.system_mems.clone());
            if let Some(cs) = self.cpusets.get_mut(&cpuset_id) {
                cs.recalculate_effective(&parent_cpus, &parent_mems);
            }
        }
    }

    pub fn set_mems(&mut self, cpuset_id: u64, mems: Vec<u32>) {
        if let Some(cs) = self.cpusets.get_mut(&cpuset_id) {
            cs.mems = mems;
            let parent_cpus = cs.parent_id
                .and_then(|pid| self.cpusets.get(&pid))
                .map(|p| p.effective_cpus.clone())
                .unwrap_or_else(|| self.system_cpus.clone());
            let parent_mems = cs.parent_id
                .and_then(|pid| self.cpusets.get(&pid))
                .map(|p| p.effective_mems.clone())
                .unwrap_or_else(|| self.system_mems.clone());
            if let Some(cs) = self.cpusets.get_mut(&cpuset_id) {
                cs.recalculate_effective(&parent_cpus, &parent_mems);
            }
        }
    }

    pub fn set_partition(&mut self, cpuset_id: u64, partition: CpusetPartition) {
        if let Some(cs) = self.cpusets.get_mut(&cpuset_id) {
            let old = cs.partition;
            cs.partition = partition;
            match (old, partition) {
                (_, CpusetPartition::Root) => self.stats.root_partitions += 1,
                (CpusetPartition::Root, _) => {
                    if self.stats.root_partitions > 0 { self.stats.root_partitions -= 1; }
                }
                _ => {}
            }
            match (old, partition) {
                (_, CpusetPartition::Isolated) => self.stats.isolated_partitions += 1,
                (CpusetPartition::Isolated, _) => {
                    if self.stats.isolated_partitions > 0 { self.stats.isolated_partitions -= 1; }
                }
                _ => {}
            }
        }
    }

    #[inline]
    pub fn record_migration(&mut self, mig: CpusetMigration) {
        self.stats.migrations += 1;
        if self.migrations.len() >= self.max_history {
            self.migrations.pop_front();
        }
        self.migrations.push_back(mig);
    }

    #[inline]
    pub fn record_violation(&mut self, vio: CpusetViolation) {
        self.stats.violations += 1;
        if self.violations.len() >= self.max_history {
            self.violations.pop_front();
        }
        self.violations.push_back(vio);
    }

    #[inline]
    pub fn overloaded_cpusets(&self, threshold: f64) -> Vec<(u64, f64)> {
        self.cpusets.iter()
            .filter(|(_, cs)| cs.tasks_per_cpu() > threshold)
            .map(|(&id, cs)| (id, cs.tasks_per_cpu()))
            .collect()
    }

    #[inline]
    pub fn empty_cpusets(&self) -> Vec<u64> {
        self.cpusets.iter()
            .filter(|(_, cs)| cs.is_empty())
            .map(|(&id, _)| id)
            .collect()
    }

    #[inline(always)]
    pub fn get_cpuset(&self, id: u64) -> Option<&Cpuset> {
        self.cpusets.get(&id)
    }

    #[inline(always)]
    pub fn stats(&self) -> &CpusetStats {
        &self.stats
    }
}

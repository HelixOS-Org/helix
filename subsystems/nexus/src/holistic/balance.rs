//! # Resource Balancer
//!
//! Dynamically balances resources across all processes and subsystems
//! to maintain system health and meet optimization goals.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RESOURCE ALLOCATION
// ============================================================================

/// Resource allocation for a single process
#[derive(Debug, Clone, Copy)]
pub struct ResourceAllocation {
    /// Process ID
    pub pid: u64,
    /// CPU share (0.0 - cores)
    pub cpu_share: f64,
    /// Memory limit (bytes)
    pub memory_limit: u64,
    /// I/O bandwidth limit (bytes/sec), 0 = unlimited
    pub io_limit: u64,
    /// Priority class
    pub priority_class: PriorityClass,
    /// Whether this allocation is guaranteed (vs. best-effort)
    pub guaranteed: bool,
}

/// Priority classification for resource allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityClass {
    /// System-critical (kernel, drivers)
    Critical,
    /// Real-time processes
    Realtime,
    /// High-priority interactive
    Interactive,
    /// Normal workloads
    Normal,
    /// Background/batch workloads
    Background,
    /// Idle-only (scavenger)
    Idle,
}

impl PriorityClass {
    /// Weight factor for resource distribution
    pub fn weight(&self) -> f64 {
        match self {
            Self::Critical => 10.0,
            Self::Realtime => 8.0,
            Self::Interactive => 5.0,
            Self::Normal => 3.0,
            Self::Background => 1.0,
            Self::Idle => 0.5,
        }
    }
}

/// Imbalance detected in the system
#[derive(Debug, Clone)]
pub struct Imbalance {
    /// Process that is over-consuming
    pub over_pid: u64,
    /// Process that is starved
    pub under_pid: u64,
    /// Resource type
    pub resource: ImbalanceResource,
    /// Severity (0.0 - 1.0)
    pub severity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImbalanceResource {
    Cpu,
    Memory,
    Io,
}

// ============================================================================
// RESOURCE BALANCER
// ============================================================================

/// The resource balancer
pub struct ResourceBalancer {
    /// Current allocations per PID
    allocations: BTreeMap<u64, ResourceAllocation>,
    /// Total CPU cores
    total_cpu: u32,
    /// Total memory (bytes)
    total_memory: u64,
    /// Reserved CPU for kernel (share)
    reserved_cpu: f64,
    /// Reserved memory for kernel (bytes)
    reserved_memory: u64,
    /// Rebalance count
    rebalance_count: u64,
}

impl ResourceBalancer {
    pub fn new(total_cpu: u32, total_memory: u64) -> Self {
        Self {
            allocations: BTreeMap::new(),
            total_cpu,
            total_memory,
            reserved_cpu: 0.5,                 // 0.5 core reserved for kernel
            reserved_memory: 64 * 1024 * 1024, // 64MB for kernel
            rebalance_count: 0,
        }
    }

    /// Set allocation for a process
    pub fn set_allocation(&mut self, alloc: ResourceAllocation) {
        self.allocations.insert(alloc.pid, alloc);
    }

    /// Remove a process allocation
    pub fn remove(&mut self, pid: u64) -> Option<ResourceAllocation> {
        self.allocations.remove(&pid)
    }

    /// Get allocation for a process
    pub fn get_allocation(&self, pid: u64) -> Option<&ResourceAllocation> {
        self.allocations.get(&pid)
    }

    /// Available resources after kernel reservation
    pub fn available_resources(&self) -> (f64, u64) {
        let cpu = self.total_cpu as f64 - self.reserved_cpu;
        let mem = self.total_memory.saturating_sub(self.reserved_memory);
        (cpu, mem)
    }

    /// Currently committed resources
    pub fn committed_resources(&self) -> (f64, u64, u64) {
        let mut cpu = 0.0;
        let mut mem = 0u64;
        let mut io = 0u64;
        for alloc in self.allocations.values() {
            cpu += alloc.cpu_share;
            mem = mem.saturating_add(alloc.memory_limit);
            io = io.saturating_add(alloc.io_limit);
        }
        (cpu, mem, io)
    }

    /// Detect imbalances in resource allocation
    pub fn detect_imbalances(&self, usage: &BTreeMap<u64, (f64, u64)>) -> Vec<Imbalance> {
        let mut imbalances = Vec::new();

        // Find over-consumers and under-consumers
        let mut over_cpu: Option<(u64, f64)> = None;
        let mut under_cpu: Option<(u64, f64)> = None;

        for (&pid, alloc) in &self.allocations {
            if let Some(&(actual_cpu, _actual_mem)) = usage.get(&pid) {
                let ratio = if alloc.cpu_share > 0.0 {
                    actual_cpu / alloc.cpu_share
                } else {
                    0.0
                };

                // Over-consuming: using >150% of allocation
                if ratio > 1.5 {
                    if over_cpu.map_or(true, |(_, r)| ratio > r) {
                        over_cpu = Some((pid, ratio));
                    }
                }
                // Under-consuming: using <30% of allocation
                if ratio < 0.3 && alloc.cpu_share > 0.1 {
                    if under_cpu.map_or(true, |(_, r)| ratio < r) {
                        under_cpu = Some((pid, ratio));
                    }
                }
            }
        }

        if let (Some((over_pid, over_ratio)), Some((under_pid, _))) = (over_cpu, under_cpu) {
            imbalances.push(Imbalance {
                over_pid,
                under_pid,
                resource: ImbalanceResource::Cpu,
                severity: ((over_ratio - 1.0) / 2.0).min(1.0),
            });
        }

        imbalances
    }

    /// Rebalance resources using weighted fair sharing
    pub fn rebalance(&mut self) -> Vec<(u64, ResourceAllocation)> {
        self.rebalance_count += 1;

        let (avail_cpu, avail_mem) = self.available_resources();

        // Calculate total weight
        let total_weight: f64 = self
            .allocations
            .values()
            .map(|a| a.priority_class.weight())
            .sum();

        if total_weight < 0.001 {
            return Vec::new();
        }

        let mut changes = Vec::new();

        for alloc in self.allocations.values_mut() {
            if alloc.guaranteed {
                continue; // Don't touch guaranteed allocations
            }

            let weight_ratio = alloc.priority_class.weight() / total_weight;
            let new_cpu = avail_cpu * weight_ratio;
            let new_mem = (avail_mem as f64 * weight_ratio) as u64;

            let cpu_changed = (new_cpu - alloc.cpu_share).abs() > 0.01;
            let mem_changed =
                (new_mem as i64 - alloc.memory_limit as i64).unsigned_abs() > 1024 * 1024;

            if cpu_changed || mem_changed {
                alloc.cpu_share = new_cpu;
                alloc.memory_limit = new_mem;
                changes.push((alloc.pid, *alloc));
            }
        }

        changes
    }

    /// Number of tracked processes
    pub fn process_count(&self) -> usize {
        self.allocations.len()
    }

    /// Total rebalances performed
    pub fn rebalance_count(&self) -> u64 {
        self.rebalance_count
    }
}

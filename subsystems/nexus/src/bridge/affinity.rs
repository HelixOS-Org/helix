//! # Bridge Affinity Tracker
//!
//! Syscall affinity and CPU binding intelligence:
//! - Track which CPUs service specific syscalls
//! - Detect affinity patterns for optimization
//! - Cache-warm CPU selection
//! - Migration cost estimation
//! - NUMA-aware syscall routing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// CPU affinity class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityClass {
    /// Bound to specific CPU
    Pinned,
    /// Preferred CPU set
    Preferred,
    /// Any CPU in NUMA node
    NumaLocal,
    /// Any CPU
    Unrestricted,
}

/// Affinity change reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityChangeReason {
    LoadBalance,
    ThermalThrottle,
    PowerSave,
    UserRequest,
    CacheOptimize,
    NumaMigration,
}

/// Per-syscall-per-CPU tracking
#[derive(Debug, Clone)]
pub struct SyscallCpuAffinity {
    /// Syscall number
    pub syscall_nr: u32,
    /// CPU -> invocation count
    cpu_counts: BTreeMap<u32, u64>,
    /// CPU -> total latency (ns)
    cpu_latency_sum: BTreeMap<u32, u64>,
    /// Last CPU used
    pub last_cpu: u32,
    /// Preferred CPU (lowest avg latency)
    pub preferred_cpu: Option<u32>,
    /// Total invocations
    pub total_calls: u64,
}

impl SyscallCpuAffinity {
    pub fn new(syscall_nr: u32) -> Self {
        Self {
            syscall_nr,
            cpu_counts: BTreeMap::new(),
            cpu_latency_sum: BTreeMap::new(),
            last_cpu: 0,
            preferred_cpu: None,
            total_calls: 0,
        }
    }

    /// Record syscall on CPU
    pub fn record(&mut self, cpu: u32, latency_ns: u64) {
        *self.cpu_counts.entry(cpu).or_insert(0) += 1;
        *self.cpu_latency_sum.entry(cpu).or_insert(0) += latency_ns;
        self.last_cpu = cpu;
        self.total_calls += 1;
        self.update_preferred();
    }

    fn update_preferred(&mut self) {
        let mut best_cpu = None;
        let mut best_avg = f64::MAX;
        for (&cpu, &count) in &self.cpu_counts {
            if count > 2 {
                let total_lat = self.cpu_latency_sum.get(&cpu).copied().unwrap_or(0);
                let avg = total_lat as f64 / count as f64;
                if avg < best_avg {
                    best_avg = avg;
                    best_cpu = Some(cpu);
                }
            }
        }
        self.preferred_cpu = best_cpu;
    }

    /// Concentration ratio (fraction on most-used CPU)
    pub fn concentration(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        let max_count = self.cpu_counts.values().copied().max().unwrap_or(0);
        max_count as f64 / self.total_calls as f64
    }

    /// Average latency on a given CPU
    pub fn avg_latency_on(&self, cpu: u32) -> f64 {
        let count = self.cpu_counts.get(&cpu).copied().unwrap_or(0);
        if count == 0 {
            return f64::MAX;
        }
        let total = self.cpu_latency_sum.get(&cpu).copied().unwrap_or(0);
        total as f64 / count as f64
    }
}

/// Process affinity profile
#[derive(Debug, Clone)]
pub struct ProcessAffinityProfile {
    /// PID
    pub pid: u64,
    /// Affinity class
    pub class: AffinityClass,
    /// Allowed CPU mask (bit per CPU, up to 64)
    pub cpu_mask: u64,
    /// CPU residence time (ns) per CPU
    residence: BTreeMap<u32, u64>,
    /// Migration count
    pub migrations: u64,
    /// Last migration timestamp
    pub last_migration_ns: u64,
}

impl ProcessAffinityProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            class: AffinityClass::Unrestricted,
            cpu_mask: u64::MAX,
            residence: BTreeMap::new(),
            migrations: 0,
            last_migration_ns: 0,
        }
    }

    /// Record time on CPU
    pub fn record_residence(&mut self, cpu: u32, duration_ns: u64) {
        *self.residence.entry(cpu).or_insert(0) += duration_ns;
    }

    /// Record migration
    pub fn record_migration(&mut self, _from: u32, _to: u32, now_ns: u64) {
        self.migrations += 1;
        self.last_migration_ns = now_ns;
    }

    /// Home CPU (most time spent)
    pub fn home_cpu(&self) -> Option<u32> {
        self.residence.iter()
            .max_by_key(|&(_, &v)| v)
            .map(|(&k, _)| k)
    }

    /// CPU spread (number of distinct CPUs used)
    pub fn cpu_spread(&self) -> usize {
        self.residence.len()
    }
}

/// Affinity tracker stats
#[derive(Debug, Clone, Default)]
pub struct BridgeAffinityStats {
    pub tracked_syscalls: usize,
    pub tracked_processes: usize,
    pub total_migrations: u64,
    pub avg_concentration: f64,
}

/// Bridge affinity tracker
pub struct BridgeAffinityTracker {
    /// Per-syscall affinity
    syscall_affinity: BTreeMap<u32, SyscallCpuAffinity>,
    /// Per-process profiles
    process_profiles: BTreeMap<u64, ProcessAffinityProfile>,
    /// Stats
    stats: BridgeAffinityStats,
}

impl BridgeAffinityTracker {
    pub fn new() -> Self {
        Self {
            syscall_affinity: BTreeMap::new(),
            process_profiles: BTreeMap::new(),
            stats: BridgeAffinityStats::default(),
        }
    }

    /// Record syscall execution
    pub fn record_syscall(&mut self, syscall_nr: u32, cpu: u32, latency_ns: u64) {
        self.syscall_affinity
            .entry(syscall_nr)
            .or_insert_with(|| SyscallCpuAffinity::new(syscall_nr))
            .record(cpu, latency_ns);
        self.update_stats();
    }

    /// Get preferred CPU for syscall
    pub fn preferred_cpu(&self, syscall_nr: u32) -> Option<u32> {
        self.syscall_affinity.get(&syscall_nr)
            .and_then(|a| a.preferred_cpu)
    }

    /// Top concentrated syscalls
    pub fn most_concentrated(&self, n: usize) -> Vec<(u32, f64)> {
        let mut entries: Vec<(u32, f64)> = self.syscall_affinity.iter()
            .map(|(&nr, a)| (nr, a.concentration()))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        entries.truncate(n);
        entries
    }

    fn update_stats(&mut self) {
        self.stats.tracked_syscalls = self.syscall_affinity.len();
        self.stats.tracked_processes = self.process_profiles.len();
        self.stats.total_migrations = self.process_profiles.values()
            .map(|p| p.migrations)
            .sum();
        if !self.syscall_affinity.is_empty() {
            self.stats.avg_concentration = self.syscall_affinity.values()
                .map(|a| a.concentration())
                .sum::<f64>() / self.syscall_affinity.len() as f64;
        }
    }

    pub fn stats(&self) -> &BridgeAffinityStats {
        &self.stats
    }
}

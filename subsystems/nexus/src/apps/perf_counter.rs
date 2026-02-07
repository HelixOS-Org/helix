//! # App Performance Counter Profiler
//!
//! Hardware performance counter profiling per application:
//! - IPC (instructions per cycle) tracking
//! - Cache miss rate analysis
//! - Branch prediction profiling
//! - TLB miss attribution
//! - Memory bandwidth per process

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COUNTER TYPES
// ============================================================================

/// Hardware counter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwCounter {
    /// CPU cycles
    Cycles,
    /// Instructions retired
    Instructions,
    /// Cache references
    CacheReferences,
    /// Cache misses
    CacheMisses,
    /// Branch instructions
    Branches,
    /// Branch misses
    BranchMisses,
    /// L1 data cache loads
    L1dLoads,
    /// L1 data cache misses
    L1dMisses,
    /// L2 cache misses
    L2Misses,
    /// LLC (Last Level Cache) misses
    LlcMisses,
    /// TLB misses
    DtlbMisses,
    /// iTLB misses
    ItlbMisses,
    /// Page walks
    PageWalks,
    /// Stalled cycles frontend
    StalledFrontend,
    /// Stalled cycles backend
    StalledBackend,
}

/// Performance bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfBottleneck {
    /// CPU bound
    CpuBound,
    /// Memory bound (cache misses)
    MemoryBound,
    /// Branch misprediction bound
    BranchBound,
    /// Frontend stall (instruction fetch)
    FrontendBound,
    /// Backend stall (execution units)
    BackendBound,
    /// TLB pressure
    TlbBound,
    /// Balanced/no bottleneck
    Balanced,
}

// ============================================================================
// COUNTER SNAPSHOT
// ============================================================================

/// Counter values snapshot
#[derive(Debug, Clone, Default)]
pub struct CounterSnapshot {
    /// Cycles
    pub cycles: u64,
    /// Instructions
    pub instructions: u64,
    /// Cache references
    pub cache_refs: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Branches
    pub branches: u64,
    /// Branch misses
    pub branch_misses: u64,
    /// L1d loads
    pub l1d_loads: u64,
    /// L1d misses
    pub l1d_misses: u64,
    /// L2 misses
    pub l2_misses: u64,
    /// LLC misses
    pub llc_misses: u64,
    /// dTLB misses
    pub dtlb_misses: u64,
    /// iTLB misses
    pub itlb_misses: u64,
    /// Stalled frontend cycles
    pub stalled_frontend: u64,
    /// Stalled backend cycles
    pub stalled_backend: u64,
    /// Timestamp (ns)
    pub timestamp_ns: u64,
}

impl CounterSnapshot {
    /// IPC
    pub fn ipc(&self) -> f64 {
        if self.cycles == 0 {
            return 0.0;
        }
        self.instructions as f64 / self.cycles as f64
    }

    /// Cache miss rate
    pub fn cache_miss_rate(&self) -> f64 {
        if self.cache_refs == 0 {
            return 0.0;
        }
        self.cache_misses as f64 / self.cache_refs as f64
    }

    /// Branch misprediction rate
    pub fn branch_miss_rate(&self) -> f64 {
        if self.branches == 0 {
            return 0.0;
        }
        self.branch_misses as f64 / self.branches as f64
    }

    /// L1d miss rate
    pub fn l1d_miss_rate(&self) -> f64 {
        if self.l1d_loads == 0 {
            return 0.0;
        }
        self.l1d_misses as f64 / self.l1d_loads as f64
    }

    /// Frontend stall ratio
    pub fn frontend_stall_ratio(&self) -> f64 {
        if self.cycles == 0 {
            return 0.0;
        }
        self.stalled_frontend as f64 / self.cycles as f64
    }

    /// Backend stall ratio
    pub fn backend_stall_ratio(&self) -> f64 {
        if self.cycles == 0 {
            return 0.0;
        }
        self.stalled_backend as f64 / self.cycles as f64
    }

    /// Delta from previous snapshot
    pub fn delta(&self, prev: &CounterSnapshot) -> CounterSnapshot {
        CounterSnapshot {
            cycles: self.cycles.saturating_sub(prev.cycles),
            instructions: self.instructions.saturating_sub(prev.instructions),
            cache_refs: self.cache_refs.saturating_sub(prev.cache_refs),
            cache_misses: self.cache_misses.saturating_sub(prev.cache_misses),
            branches: self.branches.saturating_sub(prev.branches),
            branch_misses: self.branch_misses.saturating_sub(prev.branch_misses),
            l1d_loads: self.l1d_loads.saturating_sub(prev.l1d_loads),
            l1d_misses: self.l1d_misses.saturating_sub(prev.l1d_misses),
            l2_misses: self.l2_misses.saturating_sub(prev.l2_misses),
            llc_misses: self.llc_misses.saturating_sub(prev.llc_misses),
            dtlb_misses: self.dtlb_misses.saturating_sub(prev.dtlb_misses),
            itlb_misses: self.itlb_misses.saturating_sub(prev.itlb_misses),
            stalled_frontend: self.stalled_frontend.saturating_sub(prev.stalled_frontend),
            stalled_backend: self.stalled_backend.saturating_sub(prev.stalled_backend),
            timestamp_ns: self.timestamp_ns,
        }
    }
}

// ============================================================================
// PER-PROCESS PERF
// ============================================================================

/// Per-process perf counter tracker
#[derive(Debug)]
pub struct ProcessPerfProfile {
    /// PID
    pub pid: u64,
    /// Current snapshot
    pub current: CounterSnapshot,
    /// Previous snapshot (for delta)
    pub previous: CounterSnapshot,
    /// History of IPC samples
    ipc_history: Vec<f64>,
    /// Detected bottleneck
    pub bottleneck: PerfBottleneck,
}

impl ProcessPerfProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            current: CounterSnapshot::default(),
            previous: CounterSnapshot::default(),
            ipc_history: Vec::new(),
            bottleneck: PerfBottleneck::Balanced,
        }
    }

    /// Update with new snapshot
    pub fn update(&mut self, snapshot: CounterSnapshot) {
        self.previous = core::mem::replace(&mut self.current, snapshot);
        let delta = self.current.delta(&self.previous);
        let ipc = delta.ipc();
        if self.ipc_history.len() >= 128 {
            self.ipc_history.remove(0);
        }
        self.ipc_history.push(ipc);
        self.detect_bottleneck(&delta);
    }

    /// Detect bottleneck from delta
    fn detect_bottleneck(&mut self, delta: &CounterSnapshot) {
        let frontend = delta.frontend_stall_ratio();
        let backend = delta.backend_stall_ratio();
        let cache_miss = delta.cache_miss_rate();
        let branch_miss = delta.branch_miss_rate();
        let ipc = delta.ipc();

        // High IPC = CPU bound
        if ipc > 2.0 && cache_miss < 0.02 && branch_miss < 0.02 {
            self.bottleneck = PerfBottleneck::CpuBound;
            return;
        }

        // High cache miss = memory bound
        if cache_miss > 0.1 || delta.llc_misses > delta.cycles / 100 {
            self.bottleneck = PerfBottleneck::MemoryBound;
            return;
        }

        // High branch miss
        if branch_miss > 0.05 {
            self.bottleneck = PerfBottleneck::BranchBound;
            return;
        }

        // TLB heavy
        if delta.dtlb_misses + delta.itlb_misses > delta.cycles / 50 {
            self.bottleneck = PerfBottleneck::TlbBound;
            return;
        }

        // Frontend stall
        if frontend > 0.3 {
            self.bottleneck = PerfBottleneck::FrontendBound;
            return;
        }

        // Backend stall
        if backend > 0.3 {
            self.bottleneck = PerfBottleneck::BackendBound;
            return;
        }

        self.bottleneck = PerfBottleneck::Balanced;
    }

    /// Average IPC
    pub fn avg_ipc(&self) -> f64 {
        if self.ipc_history.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.ipc_history.iter().sum();
        sum / self.ipc_history.len() as f64
    }

    /// IPC trend (positive = improving)
    pub fn ipc_trend(&self) -> f64 {
        if self.ipc_history.len() < 4 {
            return 0.0;
        }
        let n = self.ipc_history.len();
        let recent: f64 = self.ipc_history[n - 2..].iter().sum::<f64>() / 2.0;
        let earlier: f64 = self.ipc_history[..2].iter().sum::<f64>() / 2.0;
        recent - earlier
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Perf counter profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppPerfCounterStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// CPU-bound processes
    pub cpu_bound: usize,
    /// Memory-bound processes
    pub memory_bound: usize,
    /// Average IPC
    pub avg_ipc: f64,
}

/// App perf counter profiler
pub struct AppPerfCounterProfiler {
    /// Per-process profiles
    processes: BTreeMap<u64, ProcessPerfProfile>,
    /// Stats
    stats: AppPerfCounterStats,
}

impl AppPerfCounterProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppPerfCounterStats::default(),
        }
    }

    /// Get/create process
    pub fn process(&mut self, pid: u64) -> &mut ProcessPerfProfile {
        self.processes.entry(pid).or_insert_with(|| ProcessPerfProfile::new(pid))
    }

    /// Update process counters
    pub fn update(&mut self, pid: u64, snapshot: CounterSnapshot) {
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessPerfProfile::new(pid));
        proc.update(snapshot);
        self.update_stats();
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    /// Get bottleneck distribution
    pub fn bottleneck_distribution(&self) -> BTreeMap<u8, usize> {
        let mut dist = BTreeMap::new();
        for proc in self.processes.values() {
            let key = proc.bottleneck as u8;
            *dist.entry(key).or_insert(0) += 1;
        }
        dist
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.cpu_bound = self.processes.values()
            .filter(|p| p.bottleneck == PerfBottleneck::CpuBound)
            .count();
        self.stats.memory_bound = self.processes.values()
            .filter(|p| p.bottleneck == PerfBottleneck::MemoryBound)
            .count();
        if !self.processes.is_empty() {
            self.stats.avg_ipc = self.processes.values()
                .map(|p| p.avg_ipc())
                .sum::<f64>() / self.processes.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &AppPerfCounterStats {
        &self.stats
    }
}

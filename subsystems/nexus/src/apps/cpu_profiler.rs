//! # Apps CPU Profiler
//!
//! Per-application CPU profiling and analysis:
//! - Instruction-level sampling
//! - Function hotspot detection
//! - IPC (instructions per cycle) tracking
//! - Branch misprediction analysis
//! - Cache miss attribution
//! - Call stack sampling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Sample type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleType {
    Cpu,
    CacheMiss,
    BranchMiss,
    TlbMiss,
    PageFault,
    ContextSwitch,
}

/// Hotspot entry
#[derive(Debug, Clone)]
pub struct Hotspot {
    pub addr: u64,
    pub samples: u64,
    pub pct: f64,
    pub ipc: f64,
    pub cache_miss_rate: f64,
    pub branch_miss_rate: f64,
}

/// Call stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub addr: u64,
    pub depth: u16,
}

/// Sampled call stack
#[derive(Debug, Clone)]
pub struct CallStackSample {
    pub ts: u64,
    pub cpu: u32,
    pub tid: u64,
    pub frames: Vec<StackFrame>,
    pub sample_type: SampleType,
}

/// Per-thread CPU profile
#[derive(Debug, Clone)]
pub struct ThreadCpuProfile {
    pub tid: u64,
    pub pid: u64,
    pub total_samples: u64,
    pub on_cpu_samples: u64,
    pub off_cpu_samples: u64,
    pub instructions: u64,
    pub cycles: u64,
    pub cache_refs: u64,
    pub cache_misses: u64,
    pub branch_refs: u64,
    pub branch_misses: u64,
    pub context_switches: u64,
    pub migrations: u64,
    pub hotspots: Vec<Hotspot>,
}

impl ThreadCpuProfile {
    pub fn new(tid: u64, pid: u64) -> Self {
        Self {
            tid, pid, total_samples: 0, on_cpu_samples: 0, off_cpu_samples: 0,
            instructions: 0, cycles: 0, cache_refs: 0, cache_misses: 0,
            branch_refs: 0, branch_misses: 0, context_switches: 0,
            migrations: 0, hotspots: Vec::new(),
        }
    }

    pub fn ipc(&self) -> f64 { if self.cycles == 0 { 0.0 } else { self.instructions as f64 / self.cycles as f64 } }
    pub fn cache_miss_rate(&self) -> f64 { if self.cache_refs == 0 { 0.0 } else { self.cache_misses as f64 / self.cache_refs as f64 * 100.0 } }
    pub fn branch_miss_rate(&self) -> f64 { if self.branch_refs == 0 { 0.0 } else { self.branch_misses as f64 / self.branch_refs as f64 * 100.0 } }
    pub fn on_cpu_pct(&self) -> f64 { if self.total_samples == 0 { 0.0 } else { self.on_cpu_samples as f64 / self.total_samples as f64 * 100.0 } }

    pub fn record_sample(&mut self, addr: u64, on_cpu: bool) {
        self.total_samples += 1;
        if on_cpu { self.on_cpu_samples += 1; } else { self.off_cpu_samples += 1; }
        let found = self.hotspots.iter_mut().find(|h| h.addr == addr);
        if let Some(h) = found { h.samples += 1; }
        else { self.hotspots.push(Hotspot { addr, samples: 1, pct: 0.0, ipc: 0.0, cache_miss_rate: 0.0, branch_miss_rate: 0.0 }); }
    }

    pub fn recompute_hotspots(&mut self) {
        let total = self.total_samples.max(1) as f64;
        for h in &mut self.hotspots { h.pct = h.samples as f64 / total * 100.0; }
        self.hotspots.sort_by(|a, b| b.samples.cmp(&a.samples));
        if self.hotspots.len() > 64 { self.hotspots.truncate(64); }
    }
}

/// Per-process CPU profile
#[derive(Debug, Clone)]
pub struct ProcessCpuProfile {
    pub pid: u64,
    pub threads: BTreeMap<u64, ThreadCpuProfile>,
    pub total_instructions: u64,
    pub total_cycles: u64,
    pub total_samples: u64,
}

impl ProcessCpuProfile {
    pub fn new(pid: u64) -> Self {
        Self { pid, threads: BTreeMap::new(), total_instructions: 0, total_cycles: 0, total_samples: 0 }
    }

    pub fn ipc(&self) -> f64 { if self.total_cycles == 0 { 0.0 } else { self.total_instructions as f64 / self.total_cycles as f64 } }

    pub fn aggregate(&mut self) {
        self.total_instructions = self.threads.values().map(|t| t.instructions).sum();
        self.total_cycles = self.threads.values().map(|t| t.cycles).sum();
        self.total_samples = self.threads.values().map(|t| t.total_samples).sum();
    }
}

/// CPU profiler stats
#[derive(Debug, Clone, Default)]
pub struct CpuProfilerStats {
    pub tracked_processes: usize,
    pub tracked_threads: usize,
    pub total_samples: u64,
    pub avg_ipc: f64,
    pub avg_cache_miss: f64,
    pub avg_branch_miss: f64,
    pub stack_samples: u64,
}

/// Apps CPU profiler
pub struct AppsCpuProfiler {
    processes: BTreeMap<u64, ProcessCpuProfile>,
    stacks: Vec<CallStackSample>,
    sample_period_ns: u64,
    stats: CpuProfilerStats,
}

impl AppsCpuProfiler {
    pub fn new(period_ns: u64) -> Self {
        Self { processes: BTreeMap::new(), stacks: Vec::new(), sample_period_ns: period_ns, stats: CpuProfilerStats::default() }
    }

    pub fn track(&mut self, pid: u64) { self.processes.entry(pid).or_insert_with(|| ProcessCpuProfile::new(pid)); }

    pub fn record_sample(&mut self, pid: u64, tid: u64, addr: u64, on_cpu: bool) {
        let proc_profile = self.processes.entry(pid).or_insert_with(|| ProcessCpuProfile::new(pid));
        let thread = proc_profile.threads.entry(tid).or_insert_with(|| ThreadCpuProfile::new(tid, pid));
        thread.record_sample(addr, on_cpu);
    }

    pub fn record_hw_counters(&mut self, pid: u64, tid: u64, instr: u64, cycles: u64, cache_ref: u64, cache_miss: u64, br_ref: u64, br_miss: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            let t = p.threads.entry(tid).or_insert_with(|| ThreadCpuProfile::new(tid, pid));
            t.instructions += instr; t.cycles += cycles;
            t.cache_refs += cache_ref; t.cache_misses += cache_miss;
            t.branch_refs += br_ref; t.branch_misses += br_miss;
        }
    }

    pub fn record_stack(&mut self, sample: CallStackSample) { self.stacks.push(sample); }

    pub fn recompute(&mut self) {
        for p in self.processes.values_mut() {
            for t in p.threads.values_mut() { t.recompute_hotspots(); }
            p.aggregate();
        }
        self.stats.tracked_processes = self.processes.len();
        self.stats.tracked_threads = self.processes.values().map(|p| p.threads.len()).sum();
        self.stats.total_samples = self.processes.values().map(|p| p.total_samples).sum();
        self.stats.stack_samples = self.stacks.len() as u64;
        if !self.processes.is_empty() {
            let n = self.processes.len() as f64;
            self.stats.avg_ipc = self.processes.values().map(|p| p.ipc()).sum::<f64>() / n;
            self.stats.avg_cache_miss = self.processes.values().flat_map(|p| p.threads.values()).map(|t| t.cache_miss_rate()).sum::<f64>() / self.stats.tracked_threads.max(1) as f64;
            self.stats.avg_branch_miss = self.processes.values().flat_map(|p| p.threads.values()).map(|t| t.branch_miss_rate()).sum::<f64>() / self.stats.tracked_threads.max(1) as f64;
        }
    }

    pub fn process(&self, pid: u64) -> Option<&ProcessCpuProfile> { self.processes.get(&pid) }
    pub fn stats(&self) -> &CpuProfilerStats { &self.stats }
}

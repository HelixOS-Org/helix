//! # Apps Context Switch Profiler
//!
//! Process context switch behavior analysis:
//! - Voluntary vs involuntary switch tracking
//! - Switch frequency patterns
//! - Cache pollution from switches
//! - Affinity violation counting
//! - Co-scheduling opportunity detection

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Switch type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchType {
    Voluntary,
    Involuntary,
    PreemptionTick,
    PreemptionWakeup,
    Yield,
    IoWait,
    SleepWait,
}

/// Switch record
#[derive(Debug, Clone)]
pub struct SwitchRecord {
    pub pid: u64,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub switch_type: SwitchType,
    pub timestamp_ns: u64,
    pub runtime_ns: u64,
}

/// Per-process switch profile
#[derive(Debug)]
pub struct ProcessSwitchProfile {
    pub pid: u64,
    pub voluntary: u64,
    pub involuntary: u64,
    pub total_switches: u64,
    pub total_runtime_ns: u64,
    pub avg_runtime_ema_ns: f64,
    pub switch_rate_ema: f64,
    pub cross_cpu_switches: u64,
    pub last_cpu: u32,
    pub last_switch_ns: u64,
    /// Co-run partners (pid -> shared count)
    corun_partners: LinearMap<u64, 64>,
}

impl ProcessSwitchProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            voluntary: 0,
            involuntary: 0,
            total_switches: 0,
            total_runtime_ns: 0,
            avg_runtime_ema_ns: 0.0,
            switch_rate_ema: 0.0,
            cross_cpu_switches: 0,
            last_cpu: 0,
            last_switch_ns: 0,
            corun_partners: LinearMap::new(),
        }
    }

    /// Record switch
    #[inline]
    pub fn record_switch(&mut self, record: &SwitchRecord) {
        self.total_switches += 1;
        self.total_runtime_ns += record.runtime_ns;
        self.avg_runtime_ema_ns = 0.9 * self.avg_runtime_ema_ns + 0.1 * record.runtime_ns as f64;

        match record.switch_type {
            SwitchType::Voluntary | SwitchType::Yield | SwitchType::IoWait | SwitchType::SleepWait => {
                self.voluntary += 1;
            }
            SwitchType::Involuntary | SwitchType::PreemptionTick | SwitchType::PreemptionWakeup => {
                self.involuntary += 1;
            }
        }

        if record.from_cpu != record.to_cpu {
            self.cross_cpu_switches += 1;
        }

        if self.last_switch_ns > 0 {
            let interval = record.timestamp_ns.saturating_sub(self.last_switch_ns) as f64 / 1_000_000_000.0;
            if interval > 0.0 {
                let rate = 1.0 / interval;
                self.switch_rate_ema = 0.9 * self.switch_rate_ema + 0.1 * rate;
            }
        }

        self.last_cpu = record.to_cpu;
        self.last_switch_ns = record.timestamp_ns;
    }

    /// Record co-run (was on same CPU or core as another process)
    #[inline(always)]
    pub fn record_corun(&mut self, partner_pid: u64) {
        self.corun_partners.add(partner_pid, 1);
    }

    /// Voluntary ratio
    #[inline(always)]
    pub fn voluntary_ratio(&self) -> f64 {
        if self.total_switches == 0 { return 0.0; }
        self.voluntary as f64 / self.total_switches as f64
    }

    /// CPU migration ratio
    #[inline(always)]
    pub fn migration_ratio(&self) -> f64 {
        if self.total_switches == 0 { return 0.0; }
        self.cross_cpu_switches as f64 / self.total_switches as f64
    }

    /// Is CPU-bound? (high involuntary switches)
    #[inline(always)]
    pub fn is_cpu_bound(&self) -> bool {
        self.voluntary_ratio() < 0.3 && self.switch_rate_ema > 100.0
    }

    /// Is IO-bound? (high voluntary switches)
    #[inline(always)]
    pub fn is_io_bound(&self) -> bool {
        self.voluntary_ratio() > 0.8
    }

    /// Top co-run partners
    #[inline]
    pub fn top_corun_partners(&self, n: usize) -> Vec<(u64, u64)> {
        let mut partners: Vec<(u64, u64)> = self.corun_partners.iter()
            .map(|(pid, count)| (pid, count))
            .collect();
        partners.sort_by(|a, b| b.1.cmp(&a.1));
        partners.truncate(n);
        partners
    }
}

/// Context switch profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppCtxSwitchStats {
    pub tracked_processes: usize,
    pub total_switches: u64,
    pub avg_voluntary_ratio: f64,
    pub avg_migration_ratio: f64,
    pub cpu_bound_count: usize,
    pub io_bound_count: usize,
}

/// App context switch profiler
pub struct AppCtxSwitchProfiler {
    processes: BTreeMap<u64, ProcessSwitchProfile>,
    stats: AppCtxSwitchStats,
}

impl AppCtxSwitchProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppCtxSwitchStats::default(),
        }
    }

    /// Record switch
    #[inline]
    pub fn record(&mut self, record: &SwitchRecord) {
        self.processes.entry(record.pid)
            .or_insert_with(|| ProcessSwitchProfile::new(record.pid))
            .record_switch(record);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_switches = self.processes.values().map(|p| p.total_switches).sum();
        if !self.processes.is_empty() {
            self.stats.avg_voluntary_ratio = self.processes.values()
                .map(|p| p.voluntary_ratio())
                .sum::<f64>() / self.processes.len() as f64;
            self.stats.avg_migration_ratio = self.processes.values()
                .map(|p| p.migration_ratio())
                .sum::<f64>() / self.processes.len() as f64;
        }
        self.stats.cpu_bound_count = self.processes.values().filter(|p| p.is_cpu_bound()).count();
        self.stats.io_bound_count = self.processes.values().filter(|p| p.is_io_bound()).count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppCtxSwitchStats {
        &self.stats
    }
}

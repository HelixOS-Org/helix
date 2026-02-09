//! # Apps Task Stats
//!
//! Per-task statistics aggregation:
//! - CPU accounting (user/system/guest time)
//! - I/O accounting (read/write bytes, syscalls)
//! - Memory accounting (RSS, VM, page faults)
//! - Delay accounting (CPU, I/O, swap, memory reclaim)
//! - Context switch counting
//! - Task lifetime tracking

extern crate alloc;

use alloc::collections::BTreeMap;

/// CPU time breakdown
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuAccounting {
    pub user_ns: u64,
    pub system_ns: u64,
    pub guest_ns: u64,
    pub irq_ns: u64,
    pub softirq_ns: u64,
    pub idle_ns: u64,
}

impl CpuAccounting {
    #[inline(always)]
    pub fn total_ns(&self) -> u64 { self.user_ns + self.system_ns + self.guest_ns + self.irq_ns + self.softirq_ns }
    #[inline]
    pub fn user_ratio(&self) -> f64 {
        let t = self.total_ns();
        if t == 0 { return 0.0; }
        self.user_ns as f64 / t as f64
    }
    #[inline]
    pub fn system_ratio(&self) -> f64 {
        let t = self.total_ns();
        if t == 0 { return 0.0; }
        self.system_ns as f64 / t as f64
    }
}

/// I/O accounting
#[derive(Debug, Clone, Copy, Default)]
pub struct IoAccounting {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_syscalls: u64,
    pub write_syscalls: u64,
    pub cancelled_write_bytes: u64,
    pub read_wait_ns: u64,
    pub write_wait_ns: u64,
}

impl IoAccounting {
    #[inline(always)]
    pub fn total_bytes(&self) -> u64 { self.read_bytes + self.write_bytes }
    #[inline(always)]
    pub fn total_syscalls(&self) -> u64 { self.read_syscalls + self.write_syscalls }
    #[inline(always)]
    pub fn avg_read_size(&self) -> f64 {
        if self.read_syscalls == 0 { return 0.0; }
        self.read_bytes as f64 / self.read_syscalls as f64
    }
    #[inline(always)]
    pub fn avg_write_size(&self) -> f64 {
        if self.write_syscalls == 0 { return 0.0; }
        self.write_bytes as f64 / self.write_syscalls as f64
    }
}

/// Memory accounting
#[derive(Debug, Clone, Copy, Default)]
pub struct MemAccounting {
    pub rss_pages: u64,
    pub vm_pages: u64,
    pub shared_pages: u64,
    pub stack_pages: u64,
    pub minor_faults: u64,
    pub major_faults: u64,
    pub peak_rss: u64,
    pub swap_pages: u64,
}

impl MemAccounting {
    #[inline(always)]
    pub fn rss_bytes(&self) -> u64 { self.rss_pages * 4096 }
    #[inline(always)]
    pub fn vm_bytes(&self) -> u64 { self.vm_pages * 4096 }
    #[inline]
    pub fn fault_rate(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 { return 0.0; }
        let total = self.minor_faults + self.major_faults;
        (total as f64 * 1_000_000_000.0) / elapsed_ns as f64
    }
    #[inline]
    pub fn major_fault_ratio(&self) -> f64 {
        let total = self.minor_faults + self.major_faults;
        if total == 0 { return 0.0; }
        self.major_faults as f64 / total as f64
    }
}

/// Delay accounting
#[derive(Debug, Clone, Copy, Default)]
pub struct DelayAccounting {
    pub cpu_delay_ns: u64,
    pub cpu_delay_count: u32,
    pub io_delay_ns: u64,
    pub io_delay_count: u32,
    pub swap_delay_ns: u64,
    pub swap_delay_count: u32,
    pub reclaim_delay_ns: u64,
    pub reclaim_delay_count: u32,
    pub thrashing_delay_ns: u64,
    pub thrashing_delay_count: u32,
}

impl DelayAccounting {
    #[inline(always)]
    pub fn total_delay_ns(&self) -> u64 {
        self.cpu_delay_ns + self.io_delay_ns + self.swap_delay_ns + self.reclaim_delay_ns + self.thrashing_delay_ns
    }
    #[inline(always)]
    pub fn avg_cpu_delay(&self) -> f64 {
        if self.cpu_delay_count == 0 { return 0.0; }
        self.cpu_delay_ns as f64 / self.cpu_delay_count as f64
    }
    #[inline(always)]
    pub fn avg_io_delay(&self) -> f64 {
        if self.io_delay_count == 0 { return 0.0; }
        self.io_delay_ns as f64 / self.io_delay_count as f64
    }
    #[inline]
    pub fn dominant_delay(&self) -> &'static str {
        let delays = [
            (self.cpu_delay_ns, "cpu"),
            (self.io_delay_ns, "io"),
            (self.swap_delay_ns, "swap"),
            (self.reclaim_delay_ns, "reclaim"),
            (self.thrashing_delay_ns, "thrashing"),
        ];
        delays.iter().max_by_key(|(ns, _)| *ns).map(|(_, name)| *name).unwrap_or("none")
    }
}

/// Full task statistics
#[derive(Debug, Clone)]
pub struct TaskStatEntry {
    pub pid: u64,
    pub tgid: u64,
    pub cpu: CpuAccounting,
    pub io: IoAccounting,
    pub mem: MemAccounting,
    pub delay: DelayAccounting,
    pub voluntary_ctx_switches: u64,
    pub involuntary_ctx_switches: u64,
    pub created_ts: u64,
    pub last_update_ts: u64,
    pub cpu_id: u32,
    pub nice: i8,
}

impl TaskStatEntry {
    pub fn new(pid: u64, tgid: u64, ts: u64) -> Self {
        Self {
            pid, tgid,
            cpu: CpuAccounting::default(), io: IoAccounting::default(),
            mem: MemAccounting::default(), delay: DelayAccounting::default(),
            voluntary_ctx_switches: 0, involuntary_ctx_switches: 0,
            created_ts: ts, last_update_ts: ts, cpu_id: 0, nice: 0,
        }
    }

    #[inline(always)]
    pub fn total_ctx_switches(&self) -> u64 { self.voluntary_ctx_switches + self.involuntary_ctx_switches }
    #[inline(always)]
    pub fn lifetime_ns(&self) -> u64 { self.last_update_ts.saturating_sub(self.created_ts) }

    #[inline]
    pub fn cpu_utilization(&self) -> f64 {
        let lt = self.lifetime_ns();
        if lt == 0 { return 0.0; }
        self.cpu.total_ns() as f64 / lt as f64
    }

    #[inline(always)]
    pub fn is_io_bound(&self) -> bool { self.delay.io_delay_ns > self.delay.cpu_delay_ns * 2 }
    #[inline(always)]
    pub fn is_cpu_bound(&self) -> bool { self.cpu_utilization() > 0.8 }
}

/// Task stats aggregator stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TaskStatsStats {
    pub tracked_tasks: usize,
    pub total_cpu_ns: u64,
    pub total_io_bytes: u64,
    pub total_rss_pages: u64,
    pub total_ctx_switches: u64,
    pub io_bound_tasks: usize,
    pub cpu_bound_tasks: usize,
    pub high_delay_tasks: usize,
}

/// Apps task stats aggregator
#[repr(align(64))]
pub struct AppsTaskStats {
    tasks: BTreeMap<u64, TaskStatEntry>,
    stats: TaskStatsStats,
}

impl AppsTaskStats {
    pub fn new() -> Self { Self { tasks: BTreeMap::new(), stats: TaskStatsStats::default() } }

    #[inline(always)]
    pub fn register(&mut self, pid: u64, tgid: u64, ts: u64) {
        self.tasks.insert(pid, TaskStatEntry::new(pid, tgid, ts));
    }

    #[inline]
    pub fn update_cpu(&mut self, pid: u64, user: u64, system: u64, ts: u64) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            t.cpu.user_ns = user;
            t.cpu.system_ns = system;
            t.last_update_ts = ts;
        }
    }

    #[inline]
    pub fn update_io(&mut self, pid: u64, read_bytes: u64, write_bytes: u64, ts: u64) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            t.io.read_bytes = read_bytes;
            t.io.write_bytes = write_bytes;
            t.last_update_ts = ts;
        }
    }

    #[inline]
    pub fn update_mem(&mut self, pid: u64, rss: u64, vm: u64, ts: u64) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            t.mem.rss_pages = rss;
            t.mem.vm_pages = vm;
            if rss > t.mem.peak_rss { t.mem.peak_rss = rss; }
            t.last_update_ts = ts;
        }
    }

    #[inline]
    pub fn record_ctx_switch(&mut self, pid: u64, voluntary: bool) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            if voluntary { t.voluntary_ctx_switches += 1; }
            else { t.involuntary_ctx_switches += 1; }
        }
    }

    #[inline]
    pub fn record_delay(&mut self, pid: u64, cpu: u64, io: u64, swap: u64) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            if cpu > 0 { t.delay.cpu_delay_ns += cpu; t.delay.cpu_delay_count += 1; }
            if io > 0 { t.delay.io_delay_ns += io; t.delay.io_delay_count += 1; }
            if swap > 0 { t.delay.swap_delay_ns += swap; t.delay.swap_delay_count += 1; }
        }
    }

    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) { self.tasks.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.tracked_tasks = self.tasks.len();
        self.stats.total_cpu_ns = self.tasks.values().map(|t| t.cpu.total_ns()).sum();
        self.stats.total_io_bytes = self.tasks.values().map(|t| t.io.total_bytes()).sum();
        self.stats.total_rss_pages = self.tasks.values().map(|t| t.mem.rss_pages).sum();
        self.stats.total_ctx_switches = self.tasks.values().map(|t| t.total_ctx_switches()).sum();
        self.stats.io_bound_tasks = self.tasks.values().filter(|t| t.is_io_bound()).count();
        self.stats.cpu_bound_tasks = self.tasks.values().filter(|t| t.is_cpu_bound()).count();
        self.stats.high_delay_tasks = self.tasks.values().filter(|t| t.delay.total_delay_ns() > 1_000_000_000).count();
    }

    #[inline(always)]
    pub fn task(&self, pid: u64) -> Option<&TaskStatEntry> { self.tasks.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &TaskStatsStats { &self.stats }
}

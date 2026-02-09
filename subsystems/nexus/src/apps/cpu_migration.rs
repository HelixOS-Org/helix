//! # Application CPU Migration Tracker
//!
//! Tracks per-process CPU migration patterns:
//! - Cross-core and cross-NUMA migration frequency
//! - Migration cost estimation
//! - Affinity violation detection
//! - Cache-cold penalty tracking
//! - Migration reason classification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Migration type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuMigrationKind {
    /// Same-core different HT
    IntraCore,
    /// Different core same package
    IntraPackage,
    /// Different package same NUMA
    IntraNuma,
    /// Cross-NUMA migration
    CrossNuma,
    /// Forced by hotplug/offline
    Forced,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuMigrationReason {
    LoadBalance,
    AffinityChange,
    CpuOffline,
    WakeupMigration,
    ExecBalance,
    NumaBalance,
    UserRequest,
    ThermalThrottle,
    Unknown,
}

/// Single migration event
#[derive(Debug, Clone)]
pub struct CpuMigrationEvent {
    pub thread_id: u64,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub kind: CpuMigrationKind,
    pub reason: CpuMigrationReason,
    pub timestamp: u64,
    pub estimated_cost_ns: u64,
}

impl CpuMigrationEvent {
    pub fn new(thread_id: u64, from: u32, to: u32, kind: CpuMigrationKind, reason: CpuMigrationReason) -> Self {
        let cost = match kind {
            CpuMigrationKind::IntraCore => 500,
            CpuMigrationKind::IntraPackage => 5_000,
            CpuMigrationKind::IntraNuma => 15_000,
            CpuMigrationKind::CrossNuma => 50_000,
            CpuMigrationKind::Forced => 100_000,
        };
        Self {
            thread_id,
            from_cpu: from,
            to_cpu: to,
            kind,
            reason,
            timestamp: 0,
            estimated_cost_ns: cost,
        }
    }
}

/// Per-thread migration history
#[derive(Debug, Clone)]
pub struct ThreadCpuMigrationHistory {
    pub thread_id: u64,
    pub current_cpu: u32,
    pub home_cpu: u32,
    pub total_migrations: u64,
    pub cross_numa_count: u64,
    pub recent_events: VecDeque<CpuMigrationEvent>,
    pub accumulated_cost_ns: u64,
    pub last_migration_ts: u64,
    pub bounce_count: u32,
    pub max_recent: usize,
}

impl ThreadCpuMigrationHistory {
    pub fn new(thread_id: u64, initial_cpu: u32, max_recent: usize) -> Self {
        Self {
            thread_id,
            current_cpu: initial_cpu,
            home_cpu: initial_cpu,
            total_migrations: 0,
            cross_numa_count: 0,
            recent_events: VecDeque::new(),
            accumulated_cost_ns: 0,
            last_migration_ts: 0,
            bounce_count: 0,
            max_recent,
        }
    }

    pub fn record(&mut self, event: CpuMigrationEvent) {
        self.current_cpu = event.to_cpu;
        self.total_migrations += 1;
        self.accumulated_cost_ns += event.estimated_cost_ns;

        if event.kind == CpuMigrationKind::CrossNuma {
            self.cross_numa_count += 1;
        }

        // Detect bouncing (back-and-forth)
        if self.recent_events.len() >= 2 {
            let prev = &self.recent_events[self.recent_events.len() - 1];
            if prev.from_cpu == event.to_cpu && prev.to_cpu == event.from_cpu {
                self.bounce_count += 1;
            }
        }

        self.last_migration_ts = event.timestamp;
        self.recent_events.push_back(event);
        if self.recent_events.len() > self.max_recent {
            self.recent_events.pop_front();
        }
    }

    pub fn migration_rate(&self, window_ns: u64, now: u64) -> f64 {
        if window_ns == 0 { return 0.0; }
        let cutoff = now.saturating_sub(window_ns);
        let recent = self.recent_events.iter().filter(|e| e.timestamp >= cutoff).count();
        recent as f64 / (window_ns as f64 / 1_000_000_000.0)
    }

    pub fn is_bouncing(&self) -> bool {
        self.bounce_count > 5
    }

    pub fn is_away_from_home(&self) -> bool {
        self.current_cpu != self.home_cpu
    }

    pub fn cross_numa_ratio(&self) -> f64 {
        if self.total_migrations == 0 { return 0.0; }
        self.cross_numa_count as f64 / self.total_migrations as f64
    }
}

/// Per-process migration summary
#[derive(Debug, Clone)]
pub struct ProcessCpuMigrationProfile {
    pub pid: u64,
    pub threads: BTreeMap<u64, ThreadCpuMigrationHistory>,
    pub total_migrations: u64,
    pub total_cross_numa: u64,
    pub total_cost_ns: u64,
}

impl ProcessCpuMigrationProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            threads: BTreeMap::new(),
            total_migrations: 0,
            total_cross_numa: 0,
            total_cost_ns: 0,
        }
    }

    pub fn register_thread(&mut self, thread_id: u64, cpu: u32) {
        self.threads.entry(thread_id)
            .or_insert_with(|| ThreadCpuMigrationHistory::new(thread_id, cpu, 32));
    }

    pub fn record(&mut self, event: CpuMigrationEvent) {
        let tid = event.thread_id;
        let cost = event.estimated_cost_ns;
        let is_cross = event.kind == CpuMigrationKind::CrossNuma;

        if let Some(history) = self.threads.get_mut(&tid) {
            history.record(event);
        }

        self.total_migrations += 1;
        self.total_cost_ns += cost;
        if is_cross { self.total_cross_numa += 1; }
    }

    pub fn most_migrated_thread(&self) -> Option<u64> {
        self.threads.values()
            .max_by_key(|h| h.total_migrations)
            .map(|h| h.thread_id)
    }

    pub fn bouncing_threads(&self) -> Vec<u64> {
        self.threads.values()
            .filter(|h| h.is_bouncing())
            .map(|h| h.thread_id)
            .collect()
    }
}

/// App migration tracker stats
#[derive(Debug, Clone, Default)]
pub struct AppCpuMigrationTrackerStats {
    pub total_processes: usize,
    pub total_threads: usize,
    pub total_migrations: u64,
    pub total_cross_numa: u64,
    pub total_bouncing: usize,
    pub total_cost_ns: u64,
}

/// Application CPU Migration Tracker
pub struct AppCpuMigrationTracker {
    processes: BTreeMap<u64, ProcessCpuMigrationProfile>,
    stats: AppCpuMigrationTrackerStats,
}

impl AppCpuMigrationTracker {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppCpuMigrationTrackerStats::default(),
        }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.processes.entry(pid).or_insert_with(|| ProcessCpuMigrationProfile::new(pid));
        self.recompute();
    }

    pub fn register_thread(&mut self, pid: u64, thread_id: u64, cpu: u32) {
        if let Some(proc_prof) = self.processes.get_mut(&pid) {
            proc_prof.register_thread(thread_id, cpu);
        }
        self.recompute();
    }

    pub fn record(&mut self, pid: u64, event: CpuMigrationEvent) {
        if let Some(proc_prof) = self.processes.get_mut(&pid) {
            proc_prof.record(event);
        }
        self.recompute();
    }

    pub fn bouncing_threads(&self) -> Vec<(u64, u64)> {
        let mut result = Vec::new();
        for proc_prof in self.processes.values() {
            for tid in proc_prof.bouncing_threads() {
                result.push((proc_prof.pid, tid));
            }
        }
        result
    }

    pub fn high_migration_processes(&self, threshold: u64) -> Vec<u64> {
        self.processes.values()
            .filter(|p| p.total_migrations > threshold)
            .map(|p| p.pid)
            .collect()
    }

    fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.total_threads = self.processes.values().map(|p| p.threads.len()).sum();
        self.stats.total_migrations = self.processes.values().map(|p| p.total_migrations).sum();
        self.stats.total_cross_numa = self.processes.values().map(|p| p.total_cross_numa).sum();
        self.stats.total_cost_ns = self.processes.values().map(|p| p.total_cost_ns).sum();
        self.stats.total_bouncing = self.processes.values()
            .flat_map(|p| p.threads.values())
            .filter(|h| h.is_bouncing())
            .count();
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessCpuMigrationProfile> {
        self.processes.get(&pid)
    }

    pub fn stats(&self) -> &AppCpuMigrationTrackerStats {
        &self.stats
    }

    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.recompute();
    }
}

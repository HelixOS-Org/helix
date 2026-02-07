//! # Application Migration Analysis
//!
//! Process migration between CPUs/NUMA nodes:
//! - Migration cost estimation
//! - Cache affinity tracking
//! - NUMA migration decisions
//! - Migration history analysis
//! - Placement optimization
//! - Live migration coordination

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// MIGRATION TYPES
// ============================================================================

/// Migration target type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationTarget {
    /// CPU core
    Cpu(u32),
    /// NUMA node
    NumaNode(u32),
    /// CPU cluster/package
    Package(u32),
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    /// Load balancing
    LoadBalance,
    /// Thermal hotspot
    Thermal,
    /// NUMA optimization
    NumaOptimize,
    /// Cache affinity
    CacheAffinity,
    /// Power savings
    PowerSaving,
    /// User request (affinity mask)
    UserAffinity,
    /// Starvation avoidance
    Starvation,
    /// Frequency scaling
    FrequencyScaling,
    /// Capacity overflow
    CapacityOverflow,
}

/// Migration decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDecision {
    /// Allow migration
    Allow,
    /// Deny (cost too high)
    Deny,
    /// Defer (try later)
    Defer,
    /// Allow with penalty
    AllowWithPenalty,
}

// ============================================================================
// CACHE AFFINITY
// ============================================================================

/// Cache affinity state for a process
#[derive(Debug, Clone)]
pub struct CacheAffinity {
    /// Last CPU
    pub last_cpu: u32,
    /// Hot cache line estimate (KB)
    pub hot_cache_kb: u64,
    /// Time on current CPU (ms)
    pub time_on_cpu_ms: u64,
    /// Last migration timestamp
    pub last_migration: u64,
    /// Cache warmth (0.0-1.0)
    pub warmth: f64,
}

impl CacheAffinity {
    pub fn new(cpu: u32) -> Self {
        Self {
            last_cpu: cpu,
            hot_cache_kb: 0,
            time_on_cpu_ms: 0,
            last_migration: 0,
            warmth: 0.0,
        }
    }

    /// Update warmth based on time on CPU
    pub fn update_warmth(&mut self, elapsed_ms: u64) {
        self.time_on_cpu_ms += elapsed_ms;
        // Warmth saturates over ~50ms
        self.warmth = 1.0 - libm::exp(-(self.time_on_cpu_ms as f64) / 50.0);
    }

    /// Reset on migration
    pub fn reset(&mut self, new_cpu: u32, now: u64) {
        self.last_cpu = new_cpu;
        self.time_on_cpu_ms = 0;
        self.last_migration = now;
        self.warmth = 0.0;
    }

    /// Estimated migration cost (arbitrary units)
    pub fn migration_cost(&self) -> u64 {
        (self.hot_cache_kb as f64 * self.warmth) as u64
    }
}

// ============================================================================
// MIGRATION EVENT
// ============================================================================

/// Migration event record
#[derive(Debug, Clone)]
pub struct MigrationEvent {
    /// Process ID
    pub pid: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Source CPU
    pub from_cpu: u32,
    /// Destination CPU
    pub to_cpu: u32,
    /// Source NUMA node
    pub from_numa: u32,
    /// Destination NUMA node
    pub to_numa: u32,
    /// Reason
    pub reason: MigrationReason,
    /// Estimated cost
    pub cost: u64,
    /// Performance impact measured afterwards (ns slowdown)
    pub measured_impact_ns: u64,
}

impl MigrationEvent {
    /// Was cross-NUMA
    pub fn is_cross_numa(&self) -> bool {
        self.from_numa != self.to_numa
    }
}

// ============================================================================
// PROCESS MIGRATION PROFILE
// ============================================================================

/// Migration profile per process
#[derive(Debug, Clone)]
pub struct ProcessMigrationProfile {
    /// Process ID
    pub pid: u64,
    /// Cache affinity state
    pub cache_affinity: CacheAffinity,
    /// Total migrations
    pub total_migrations: u64,
    /// Cross-NUMA migrations
    pub cross_numa_migrations: u64,
    /// Average migration cost
    pub avg_cost: u64,
    /// Preferred CPU (most time spent)
    pub preferred_cpu: u32,
    /// NUMA preference
    pub preferred_numa: u32,
    /// Migration frequency (per second)
    pub migration_rate: f64,
    /// Recent events
    history: Vec<MigrationEvent>,
    /// Max history
    max_history: usize,
}

impl ProcessMigrationProfile {
    pub fn new(pid: u64, cpu: u32, numa: u32) -> Self {
        Self {
            pid,
            cache_affinity: CacheAffinity::new(cpu),
            total_migrations: 0,
            cross_numa_migrations: 0,
            avg_cost: 0,
            preferred_cpu: cpu,
            preferred_numa: numa,
            migration_rate: 0.0,
            history: Vec::new(),
            max_history: 64,
        }
    }

    /// Record migration
    pub fn record(&mut self, event: MigrationEvent) {
        self.total_migrations += 1;
        if event.is_cross_numa() {
            self.cross_numa_migrations += 1;
        }

        // Running average cost
        self.avg_cost =
            (self.avg_cost * (self.total_migrations - 1) + event.cost) / self.total_migrations;

        self.cache_affinity.reset(event.to_cpu, event.timestamp);

        self.history.push(event);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Cross-NUMA ratio
    pub fn cross_numa_ratio(&self) -> f64 {
        if self.total_migrations == 0 {
            return 0.0;
        }
        self.cross_numa_migrations as f64 / self.total_migrations as f64
    }

    /// Is migrating too frequently (thrashing)
    pub fn is_thrashing(&self, threshold_per_sec: f64) -> bool {
        self.migration_rate > threshold_per_sec
    }
}

// ============================================================================
// PLACEMENT SCORE
// ============================================================================

/// Placement candidate
#[derive(Debug, Clone)]
pub struct PlacementCandidate {
    /// Target CPU
    pub cpu: u32,
    /// Target NUMA node
    pub numa: u32,
    /// Load on target
    pub load_pct: u32,
    /// Cache benefit
    pub cache_benefit: f64,
    /// NUMA locality benefit
    pub numa_benefit: f64,
    /// Power benefit
    pub power_benefit: f64,
    /// Composite score (higher = better)
    pub score: f64,
}

/// Placement decision
#[derive(Debug, Clone)]
pub struct PlacementDecision {
    /// Process ID
    pub pid: u64,
    /// Selected candidate
    pub selected: PlacementCandidate,
    /// All candidates considered
    pub candidates_considered: usize,
    /// Decision reason
    pub reason: MigrationReason,
}

// ============================================================================
// MIGRATION POLICY
// ============================================================================

/// Migration policy settings
#[derive(Debug, Clone)]
pub struct MigrationPolicy {
    /// Minimum cache warmth to discourage migration
    pub cache_warmth_threshold: f64,
    /// Maximum migration rate before throttling
    pub max_migrations_per_sec: f64,
    /// Cross-NUMA penalty multiplier
    pub numa_penalty: f64,
    /// Load imbalance threshold to trigger migration
    pub load_imbalance_threshold: u32,
    /// Minimum time on CPU before migration (ms)
    pub min_residence_ms: u64,
    /// Enable NUMA-aware placement
    pub numa_aware: bool,
}

impl Default for MigrationPolicy {
    fn default() -> Self {
        Self {
            cache_warmth_threshold: 0.7,
            max_migrations_per_sec: 10.0,
            numa_penalty: 2.0,
            load_imbalance_threshold: 20,
            min_residence_ms: 5,
            numa_aware: true,
        }
    }
}

// ============================================================================
// MIGRATION ANALYZER
// ============================================================================

/// Migration analyzer statistics
#[derive(Debug, Clone, Default)]
pub struct MigrationStats {
    /// Total migrations
    pub total: u64,
    /// Denied migrations
    pub denied: u64,
    /// Deferred migrations
    pub deferred: u64,
    /// Cross-NUMA migrations
    pub cross_numa: u64,
    /// Thrashing detections
    pub thrashing_events: u64,
    /// Average cost
    pub avg_cost: u64,
}

/// Application migration analyzer
pub struct AppMigrationAnalyzer {
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessMigrationProfile>,
    /// Migration policy
    policy: MigrationPolicy,
    /// Per-CPU load (percent)
    cpu_loads: BTreeMap<u32, u32>,
    /// CPU to NUMA mapping
    cpu_to_numa: BTreeMap<u32, u32>,
    /// Stats
    stats: MigrationStats,
    /// Window for rate calculation (ms)
    rate_window_ms: u64,
}

impl AppMigrationAnalyzer {
    pub fn new(policy: MigrationPolicy) -> Self {
        Self {
            profiles: BTreeMap::new(),
            policy,
            cpu_loads: BTreeMap::new(),
            cpu_to_numa: BTreeMap::new(),
            stats: MigrationStats::default(),
            rate_window_ms: 1000,
        }
    }

    /// Register CPU to NUMA mapping
    pub fn set_cpu_numa(&mut self, cpu: u32, numa: u32) {
        self.cpu_to_numa.insert(cpu, numa);
    }

    /// Update CPU load
    pub fn update_cpu_load(&mut self, cpu: u32, load_pct: u32) {
        self.cpu_loads.insert(cpu, load_pct);
    }

    /// Register process
    pub fn register(&mut self, pid: u64, cpu: u32) {
        let numa = self.cpu_to_numa.get(&cpu).copied().unwrap_or(0);
        self.profiles
            .insert(pid, ProcessMigrationProfile::new(pid, cpu, numa));
    }

    /// Evaluate migration
    pub fn evaluate_migration(
        &self,
        pid: u64,
        target_cpu: u32,
        reason: MigrationReason,
        now: u64,
    ) -> MigrationDecision {
        let profile = match self.profiles.get(&pid) {
            Some(p) => p,
            None => return MigrationDecision::Allow,
        };

        // Check minimum residence time
        if profile.cache_affinity.time_on_cpu_ms < self.policy.min_residence_ms {
            return MigrationDecision::Defer;
        }

        // Check thrashing
        if profile.is_thrashing(self.policy.max_migrations_per_sec) {
            return MigrationDecision::Deny;
        }

        // Cache warmth check
        if profile.cache_affinity.warmth > self.policy.cache_warmth_threshold {
            let target_numa = self.cpu_to_numa.get(&target_cpu).copied().unwrap_or(0);
            if target_numa != profile.preferred_numa {
                return MigrationDecision::AllowWithPenalty;
            }
        }

        // Check if target is less loaded
        let current_cpu = profile.cache_affinity.last_cpu;
        let current_load = self.cpu_loads.get(&current_cpu).copied().unwrap_or(50);
        let target_load = self.cpu_loads.get(&target_cpu).copied().unwrap_or(50);

        if target_load >= current_load {
            match reason {
                MigrationReason::LoadBalance => return MigrationDecision::Deny,
                _ => {},
            }
        }

        MigrationDecision::Allow
    }

    /// Record completed migration
    pub fn record_migration(&mut self, event: MigrationEvent) {
        self.stats.total += 1;
        if event.is_cross_numa() {
            self.stats.cross_numa += 1;
        }

        // Running average cost
        if self.stats.total > 0 {
            self.stats.avg_cost =
                (self.stats.avg_cost * (self.stats.total - 1) + event.cost) / self.stats.total;
        }

        if let Some(profile) = self.profiles.get_mut(&event.pid) {
            profile.record(event);
        }
    }

    /// Find best placement for process
    pub fn find_placement(&self, pid: u64, reason: MigrationReason) -> Option<PlacementDecision> {
        let profile = self.profiles.get(&pid)?;
        let current_cpu = profile.cache_affinity.last_cpu;
        let current_numa = profile.preferred_numa;

        let mut candidates = Vec::new();

        for (&cpu, &load) in &self.cpu_loads {
            if cpu == current_cpu {
                continue;
            }

            let numa = self.cpu_to_numa.get(&cpu).copied().unwrap_or(0);
            let same_numa = numa == current_numa;

            let cache_benefit = if same_numa { 0.5 } else { 0.0 };
            let numa_benefit = if same_numa { 1.0 } else { 0.2 };
            let power_benefit = if load < 50 { 0.3 } else { 0.0 };

            let load_score = 1.0 - (load as f64 / 100.0);
            let score =
                load_score * 0.4 + cache_benefit * 0.3 + numa_benefit * 0.2 + power_benefit * 0.1;

            candidates.push(PlacementCandidate {
                cpu,
                numa,
                load_pct: load,
                cache_benefit,
                numa_benefit,
                power_benefit,
                score,
            });
        }

        // Sort by score descending (use integer comparison)
        candidates.sort_by(|a, b| {
            let sa = (a.score * 10000.0) as i64;
            let sb = (b.score * 10000.0) as i64;
            sb.cmp(&sa)
        });

        let selected = candidates.first()?.clone();
        let count = candidates.len();

        Some(PlacementDecision {
            pid,
            selected,
            candidates_considered: count,
            reason,
        })
    }

    /// Update migration rate for process
    pub fn update_rate(&mut self, pid: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            let window = self.rate_window_ms;
            if let Some(first) = profile.history.first() {
                if let Some(last) = profile.history.last() {
                    let span = last.timestamp.saturating_sub(first.timestamp);
                    if span > 0 && span >= window {
                        profile.migration_rate =
                            profile.history.len() as f64 / (span as f64 / 1000.0);
                    }
                }
            }
        }
    }

    /// Check all for thrashing
    pub fn detect_thrashing(&mut self) -> Vec<u64> {
        let threshold = self.policy.max_migrations_per_sec;
        let mut thrashing = Vec::new();

        for (pid, profile) in &self.profiles {
            if profile.is_thrashing(threshold) {
                thrashing.push(*pid);
                self.stats.thrashing_events += 1;
            }
        }

        thrashing
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessMigrationProfile> {
        self.profiles.get(&pid)
    }

    /// Get stats
    pub fn stats(&self) -> &MigrationStats {
        &self.stats
    }

    /// Unregister
    pub fn unregister(&mut self, pid: u64) {
        self.profiles.remove(&pid);
    }
}

//! # Application CPU Affinity Management
//!
//! Per-application CPU affinity tracking:
//! - Affinity mask management
//! - Core preference analysis
//! - Migration tracking
//! - Affinity policy enforcement
//! - Performance-aware placement

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use crate::fast::array_map::ArrayMap;

// ============================================================================
// CPU CORE TYPE
// ============================================================================

/// CPU core type (for hybrid architectures)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoreType {
    /// Performance core
    Performance,
    /// Efficiency core
    Efficiency,
    /// Mixed/unknown
    General,
}

/// Core descriptor
#[derive(Debug, Clone)]
pub struct CoreDescriptor {
    /// Core ID
    pub id: u32,
    /// Core type
    pub core_type: CoreType,
    /// NUMA node
    pub numa_node: u32,
    /// Physical package
    pub package: u32,
    /// Sibling (SMT pair)
    pub sibling: Option<u32>,
    /// Current frequency (MHz)
    pub freq_mhz: u32,
    /// Max frequency (MHz)
    pub max_freq_mhz: u32,
}

impl CoreDescriptor {
    pub fn new(id: u32, core_type: CoreType) -> Self {
        Self {
            id,
            core_type,
            numa_node: 0,
            package: 0,
            sibling: None,
            freq_mhz: 0,
            max_freq_mhz: 0,
        }
    }

    /// Frequency ratio
    #[inline]
    pub fn freq_ratio(&self) -> f64 {
        if self.max_freq_mhz == 0 {
            return 0.0;
        }
        self.freq_mhz as f64 / self.max_freq_mhz as f64
    }
}

// ============================================================================
// AFFINITY MASK
// ============================================================================

/// CPU affinity mask (bitmap)
#[derive(Debug, Clone)]
pub struct AffinityMask {
    /// Bits (each u64 represents 64 CPUs)
    bits: Vec<u64>,
    /// Total CPUs
    total_cpus: u32,
}

impl AffinityMask {
    pub fn new(total_cpus: u32) -> Self {
        let words = ((total_cpus as usize) + 63) / 64;
        Self {
            bits: alloc::vec![0u64; words],
            total_cpus,
        }
    }

    /// Set all CPUs
    #[inline]
    pub fn set_all(&mut self) {
        for i in 0..self.total_cpus {
            self.set(i);
        }
    }

    /// Set a CPU
    #[inline]
    pub fn set(&mut self, cpu: u32) {
        if cpu < self.total_cpus {
            let word = cpu as usize / 64;
            let bit = cpu as usize % 64;
            if word < self.bits.len() {
                self.bits[word] |= 1u64 << bit;
            }
        }
    }

    /// Clear a CPU
    #[inline]
    pub fn clear(&mut self, cpu: u32) {
        if cpu < self.total_cpus {
            let word = cpu as usize / 64;
            let bit = cpu as usize % 64;
            if word < self.bits.len() {
                self.bits[word] &= !(1u64 << bit);
            }
        }
    }

    /// Test if CPU is set
    pub fn is_set(&self, cpu: u32) -> bool {
        if cpu >= self.total_cpus {
            return false;
        }
        let word = cpu as usize / 64;
        let bit = cpu as usize % 64;
        if word < self.bits.len() {
            (self.bits[word] & (1u64 << bit)) != 0
        } else {
            false
        }
    }

    /// Count set CPUs
    #[inline(always)]
    pub fn count(&self) -> u32 {
        self.bits.iter().map(|w| w.count_ones()).sum()
    }

    /// Iterate over set CPUs
    #[inline]
    pub fn iter_set(&self) -> Vec<u32> {
        let mut result = Vec::new();
        for cpu in 0..self.total_cpus {
            if self.is_set(cpu) {
                result.push(cpu);
            }
        }
        result
    }

    /// Intersection with another mask
    #[inline]
    pub fn intersect(&self, other: &AffinityMask) -> AffinityMask {
        let min_len = self.bits.len().min(other.bits.len());
        let mut result = AffinityMask::new(self.total_cpus.min(other.total_cpus));
        for i in 0..min_len {
            result.bits[i] = self.bits[i] & other.bits[i];
        }
        result
    }

    /// Union with another mask
    pub fn union(&self, other: &AffinityMask) -> AffinityMask {
        let max_len = self.bits.len().max(other.bits.len());
        let mut result = AffinityMask::new(self.total_cpus.max(other.total_cpus));
        for i in 0..max_len {
            let a = if i < self.bits.len() { self.bits[i] } else { 0 };
            let b = if i < other.bits.len() {
                other.bits[i]
            } else {
                0
            };
            result.bits[i] = a | b;
        }
        result
    }
}

// ============================================================================
// AFFINITY POLICY
// ============================================================================

/// Affinity policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityPolicy {
    /// System-managed (no constraint)
    SystemManaged,
    /// Soft affinity (preferred)
    Preferred,
    /// Hard affinity (required)
    Required,
    /// Exclusive (no sharing)
    Exclusive,
}

/// Migration event
#[derive(Debug, Clone)]
pub struct MigrationEvent {
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub thread_id: u64,
    /// From core
    pub from_core: u32,
    /// To core
    pub to_core: u32,
    /// Timestamp
    pub timestamp: u64,
    /// Voluntary
    pub voluntary: bool,
}

// ============================================================================
// PROCESS AFFINITY PROFILE
// ============================================================================

/// Per-process affinity profile
#[derive(Debug, Clone)]
pub struct ProcessAffinityProfile {
    /// Process ID
    pub pid: u64,
    /// Current affinity mask
    pub mask: AffinityMask,
    /// Policy
    pub policy: AffinityPolicy,
    /// Core usage histogram (core -> time_ns)
    pub core_usage: ArrayMap<u64, 32>,
    /// Migration count
    pub migrations: u64,
    /// Last core
    pub last_core: u32,
    /// Preferred core type
    pub preferred_type: CoreType,
    /// Total runtime (ns)
    pub total_runtime_ns: u64,
}

impl ProcessAffinityProfile {
    pub fn new(pid: u64, total_cpus: u32) -> Self {
        let mut mask = AffinityMask::new(total_cpus);
        mask.set_all();
        Self {
            pid,
            mask,
            policy: AffinityPolicy::SystemManaged,
            core_usage: ArrayMap::new(0),
            migrations: 0,
            last_core: 0,
            preferred_type: CoreType::General,
            total_runtime_ns: 0,
        }
    }

    /// Record core usage
    #[inline]
    pub fn record_usage(&mut self, core: u32, duration_ns: u64) {
        if core != self.last_core && self.total_runtime_ns > 0 {
            self.migrations += 1;
        }
        self.core_usage.add(core as usize, duration_ns);
        self.total_runtime_ns += duration_ns;
        self.last_core = core;
    }

    /// Most-used core
    #[inline]
    pub fn most_used_core(&self) -> Option<u32> {
        self.core_usage
            .iter()
            .max_by_key(|(_, time)| *time)
            .map(|(core, _)| core as u32)
    }

    /// Migration rate (per second)
    #[inline]
    pub fn migration_rate(&self) -> f64 {
        if self.total_runtime_ns == 0 {
            return 0.0;
        }
        self.migrations as f64 / (self.total_runtime_ns as f64 / 1_000_000_000.0)
    }

    /// Core spread (number of unique cores used)
    #[inline(always)]
    pub fn core_spread(&self) -> usize {
        self.core_usage.len()
    }

    /// Concentration ratio (fraction on most-used core)
    #[inline]
    pub fn concentration_ratio(&self) -> f64 {
        if self.total_runtime_ns == 0 {
            return 0.0;
        }
        let max_time = self.core_usage.values().max().unwrap_or(0);
        max_time as f64 / self.total_runtime_ns as f64
    }
}

// ============================================================================
// AFFINITY MANAGER
// ============================================================================

/// App affinity stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppAffinityStats {
    /// Tracked processes
    pub process_count: usize,
    /// Total migrations
    pub total_migrations: u64,
    /// Active policies
    pub policy_count: usize,
    /// Exclusive cores in use
    pub exclusive_cores: u32,
    /// Average concentration
    pub avg_concentration: f64,
}

/// Application affinity manager
pub struct AppAffinityManager {
    /// Core descriptors
    cores: BTreeMap<u32, CoreDescriptor>,
    /// Process profiles
    profiles: BTreeMap<u64, ProcessAffinityProfile>,
    /// Exclusive assignments (core -> pid)
    exclusive: ArrayMap<u64, 32>,
    /// Migration log
    migration_log: VecDeque<MigrationEvent>,
    /// Max log size
    max_log: usize,
    /// Stats
    stats: AppAffinityStats,
}

impl AppAffinityManager {
    pub fn new() -> Self {
        Self {
            cores: BTreeMap::new(),
            profiles: BTreeMap::new(),
            exclusive: ArrayMap::new(0),
            migration_log: VecDeque::new(),
            max_log: 1024,
            stats: AppAffinityStats::default(),
        }
    }

    /// Register core
    #[inline(always)]
    pub fn register_core(&mut self, desc: CoreDescriptor) {
        self.cores.insert(desc.id, desc);
    }

    /// Register process
    #[inline]
    pub fn register_process(&mut self, pid: u64) {
        let total_cpus = self.cores.len() as u32;
        self.profiles
            .insert(pid, ProcessAffinityProfile::new(pid, total_cpus.max(1)));
        self.stats.process_count = self.profiles.len();
    }

    /// Set affinity
    #[inline]
    pub fn set_affinity(&mut self, pid: u64, mask: AffinityMask, policy: AffinityPolicy) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.mask = mask;
            profile.policy = policy;
        }
    }

    /// Set exclusive cores
    pub fn set_exclusive(&mut self, pid: u64, cores: &[u32]) -> bool {
        // Check no conflicts
        for &core in cores {
            if let Some(existing) = self.exclusive.try_get(core as usize) {
                if existing != pid {
                    return false;
                }
            }
        }
        for &core in cores {
            self.exclusive.insert(core, pid);
        }
        self.stats.exclusive_cores = self.exclusive.len() as u32;
        true
    }

    /// Record usage
    pub fn record_usage(&mut self, pid: u64, core: u32, duration_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            let old_core = profile.last_core;
            profile.record_usage(core, duration_ns);
            if core != old_core && profile.total_runtime_ns > duration_ns {
                self.migration_log.push_back(MigrationEvent {
                    pid,
                    thread_id: 0,
                    from_core: old_core,
                    to_core: core,
                    timestamp: 0,
                    voluntary: false,
                });
                if self.migration_log.len() > self.max_log {
                    self.migration_log.remove(0);
                }
                self.stats.total_migrations += 1;
            }
        }
    }

    /// Find best core for process
    pub fn find_best_core(&self, pid: u64) -> Option<u32> {
        let profile = self.profiles.get(&pid)?;

        // Respect affinity mask
        let allowed = profile.mask.iter_set();
        if allowed.is_empty() {
            return None;
        }

        // Prefer core type
        let preferred_type = profile.preferred_type;
        let mut best_core = None;
        let mut best_score = 0u64;

        for &core_id in &allowed {
            if let Some(core) = self.cores.get(&core_id) {
                let mut score = 100u64;
                if core.core_type == preferred_type {
                    score += 50;
                }
                // Prefer less-used cores
                let usage = profile.core_usage.get(core_id);
                if usage > 0 {
                    score += 25; // warm cache
                }
                // Avoid exclusive cores owned by others
                if let Some(owner) = self.exclusive.try_get(core_id as usize) {
                    if owner != pid {
                        continue;
                    }
                    score += 100; // our exclusive core
                }
                if best_core.is_none() || score > best_score {
                    best_core = Some(core_id);
                    best_score = score;
                }
            }
        }

        best_core
    }

    /// Update stats
    pub fn update_stats(&mut self) {
        let count = self.profiles.len();
        if count == 0 {
            self.stats.avg_concentration = 0.0;
            return;
        }
        let total: f64 = self
            .profiles
            .values()
            .map(|p| p.concentration_ratio())
            .sum();
        self.stats.avg_concentration = total / count as f64;
    }

    /// Get profile
    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessAffinityProfile> {
        self.profiles.get(&pid)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppAffinityStats {
        &self.stats
    }
}

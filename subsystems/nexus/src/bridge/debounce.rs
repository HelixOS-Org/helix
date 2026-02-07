//! # Bridge Debounce System
//!
//! Syscall debouncing and deduplication:
//! - Rapid-fire syscall suppression
//! - Idempotency detection
//! - Coalescing repeated operations
//! - Adaptive debounce intervals
//! - Per-process debounce tracking

extern crate alloc;

use alloc::collections::BTreeMap;

// ============================================================================
// DEBOUNCE TYPES
// ============================================================================

/// Debounce strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebounceStrategy {
    /// Fixed interval
    Fixed,
    /// Adaptive (adjusts based on frequency)
    Adaptive,
    /// Exponential backoff
    Exponential,
}

/// Debounce result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebounceResult {
    /// Allow through
    Allow,
    /// Suppress (debounced)
    Suppress,
    /// Coalesce with previous
    Coalesce,
}

// ============================================================================
// DEBOUNCE ENTRY
// ============================================================================

/// Per-key debounce state
#[derive(Debug, Clone)]
pub struct DebounceEntry {
    /// Last allowed timestamp
    pub last_allowed: u64,
    /// Last attempt timestamp
    pub last_attempt: u64,
    /// Suppression count (consecutive)
    pub suppressed: u64,
    /// Total attempts
    pub total_attempts: u64,
    /// Total allowed
    pub total_allowed: u64,
    /// Current interval (ns)
    pub interval_ns: u64,
    /// Base interval (ns)
    pub base_interval_ns: u64,
    /// Max interval (ns)
    pub max_interval_ns: u64,
    /// Strategy
    pub strategy: DebounceStrategy,
    /// Last value hash (for idempotency)
    pub last_value_hash: u64,
}

impl DebounceEntry {
    pub fn new(base_interval_ns: u64, strategy: DebounceStrategy) -> Self {
        Self {
            last_allowed: 0,
            last_attempt: 0,
            suppressed: 0,
            total_attempts: 0,
            total_allowed: 0,
            interval_ns: base_interval_ns,
            base_interval_ns,
            max_interval_ns: base_interval_ns * 64,
            strategy,
            last_value_hash: 0,
        }
    }

    /// Check if should allow
    pub fn check(&mut self, now: u64, value_hash: u64) -> DebounceResult {
        self.total_attempts += 1;
        self.last_attempt = now;

        // Idempotency check
        if value_hash == self.last_value_hash && self.total_allowed > 0 {
            let elapsed = now.saturating_sub(self.last_allowed);
            if elapsed < self.interval_ns {
                self.suppressed += 1;
                return DebounceResult::Coalesce;
            }
        }

        let elapsed = now.saturating_sub(self.last_allowed);
        if elapsed < self.interval_ns {
            self.suppressed += 1;
            self.adapt_interval();
            return DebounceResult::Suppress;
        }

        // Allow
        self.last_allowed = now;
        self.total_allowed += 1;
        self.last_value_hash = value_hash;
        self.suppressed = 0;
        self.reset_interval();
        DebounceResult::Allow
    }

    /// Adapt interval based on strategy
    fn adapt_interval(&mut self) {
        match self.strategy {
            DebounceStrategy::Fixed => {}
            DebounceStrategy::Adaptive => {
                // Increase interval when lots of suppression
                if self.suppressed > 5 {
                    self.interval_ns = (self.interval_ns * 3 / 2).min(self.max_interval_ns);
                }
            }
            DebounceStrategy::Exponential => {
                self.interval_ns = (self.interval_ns * 2).min(self.max_interval_ns);
            }
        }
    }

    /// Reset interval after allow
    fn reset_interval(&mut self) {
        match self.strategy {
            DebounceStrategy::Fixed => {}
            DebounceStrategy::Adaptive => {
                // Slowly decrease
                self.interval_ns = (self.interval_ns * 3 / 4).max(self.base_interval_ns);
            }
            DebounceStrategy::Exponential => {
                self.interval_ns = self.base_interval_ns;
            }
        }
    }

    /// Suppression rate
    pub fn suppression_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            return 0.0;
        }
        1.0 - (self.total_allowed as f64 / self.total_attempts as f64)
    }
}

// ============================================================================
// PROCESS DEBOUNCE
// ============================================================================

/// Per-process debounce tracker
#[derive(Debug)]
pub struct ProcessDebounce {
    /// Process id
    pub pid: u64,
    /// Per-syscall debounce
    entries: BTreeMap<u32, DebounceEntry>,
    /// Default interval
    pub default_interval_ns: u64,
    /// Default strategy
    pub default_strategy: DebounceStrategy,
}

impl ProcessDebounce {
    pub fn new(pid: u64, default_interval_ns: u64, strategy: DebounceStrategy) -> Self {
        Self {
            pid,
            entries: BTreeMap::new(),
            default_interval_ns,
            default_strategy: strategy,
        }
    }

    /// Check syscall
    pub fn check(&mut self, syscall_nr: u32, now: u64, value_hash: u64) -> DebounceResult {
        let entry = self.entries.entry(syscall_nr).or_insert_with(|| {
            DebounceEntry::new(self.default_interval_ns, self.default_strategy)
        });
        entry.check(now, value_hash)
    }

    /// Get entry
    pub fn entry(&self, syscall_nr: u32) -> Option<&DebounceEntry> {
        self.entries.get(&syscall_nr)
    }

    /// Total suppression rate
    pub fn overall_suppression_rate(&self) -> f64 {
        let total_attempts: u64 = self.entries.values().map(|e| e.total_attempts).sum();
        let total_allowed: u64 = self.entries.values().map(|e| e.total_allowed).sum();
        if total_attempts == 0 {
            return 0.0;
        }
        1.0 - (total_allowed as f64 / total_attempts as f64)
    }
}

// ============================================================================
// DEBOUNCE ENGINE
// ============================================================================

/// Debounce stats
#[derive(Debug, Clone, Default)]
pub struct BridgeDebounceStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total checks
    pub total_checks: u64,
    /// Total suppressed
    pub total_suppressed: u64,
    /// Total coalesced
    pub total_coalesced: u64,
}

/// Bridge debounce manager
pub struct BridgeDebounceManager {
    /// Per-process trackers
    processes: BTreeMap<u64, ProcessDebounce>,
    /// Default interval (ns)
    pub default_interval_ns: u64,
    /// Default strategy
    pub default_strategy: DebounceStrategy,
    /// Stats
    stats: BridgeDebounceStats,
}

impl BridgeDebounceManager {
    pub fn new(default_interval_ns: u64) -> Self {
        Self {
            processes: BTreeMap::new(),
            default_interval_ns,
            default_strategy: DebounceStrategy::Adaptive,
            stats: BridgeDebounceStats::default(),
        }
    }

    /// Check syscall
    pub fn check(&mut self, pid: u64, syscall_nr: u32, now: u64, value_hash: u64) -> DebounceResult {
        let interval = self.default_interval_ns;
        let strategy = self.default_strategy;
        let tracker = self.processes.entry(pid).or_insert_with(|| {
            ProcessDebounce::new(pid, interval, strategy)
        });
        let result = tracker.check(syscall_nr, now, value_hash);
        self.stats.total_checks += 1;
        match result {
            DebounceResult::Suppress => self.stats.total_suppressed += 1,
            DebounceResult::Coalesce => self.stats.total_coalesced += 1,
            DebounceResult::Allow => {}
        }
        self.stats.tracked_processes = self.processes.len();
        result
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.stats.tracked_processes = self.processes.len();
    }

    /// Get process tracker
    pub fn process(&self, pid: u64) -> Option<&ProcessDebounce> {
        self.processes.get(&pid)
    }

    /// Stats
    pub fn stats(&self) -> &BridgeDebounceStats {
        &self.stats
    }
}

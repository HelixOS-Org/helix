//! # Bridge Syscall Table
//!
//! Dynamic syscall table management:
//! - Syscall registration and dispatch table
//! - Per-syscall handler metadata
//! - Syscall number allocation
//! - Versioned syscall entries
//! - Hot-patching support
//! - Performance counters per syscall

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Syscall category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallCategory {
    Process,
    Memory,
    FileSystem,
    Network,
    Ipc,
    Signal,
    Timer,
    Device,
    Security,
    Debug,
    Custom,
}

/// Syscall flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallTableFlag {
    /// No special flags
    None,
    /// Can restart after signal
    Restartable,
    /// Uses file descriptor argument
    UsesFd,
    /// May block
    MayBlock,
    /// Requires privilege
    Privileged,
    /// Deprecated
    Deprecated,
    /// New / experimental
    Experimental,
}

/// Syscall table entry
#[derive(Debug, Clone)]
pub struct SyscallEntry {
    pub nr: u32,
    pub name_hash: u64,
    pub category: SyscallCategory,
    pub nr_args: u8,
    pub flags: Vec<SyscallTableFlag>,
    pub version: u32,
    pub enabled: bool,
    pub total_calls: u64,
    pub total_errors: u64,
    pub total_time_ns: u64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
}

impl SyscallEntry {
    pub fn new(nr: u32, name_hash: u64, category: SyscallCategory, nr_args: u8) -> Self {
        Self {
            nr,
            name_hash,
            category,
            nr_args,
            flags: Vec::new(),
            version: 1,
            enabled: true,
            total_calls: 0,
            total_errors: 0,
            total_time_ns: 0,
            min_time_ns: u64::MAX,
            max_time_ns: 0,
        }
    }

    #[inline]
    pub fn record_call(&mut self, duration_ns: u64, is_error: bool) {
        self.total_calls += 1;
        self.total_time_ns += duration_ns;
        if is_error { self.total_errors += 1; }
        if duration_ns < self.min_time_ns { self.min_time_ns = duration_ns; }
        if duration_ns > self.max_time_ns { self.max_time_ns = duration_ns; }
    }

    #[inline(always)]
    pub fn avg_time_ns(&self) -> f64 {
        if self.total_calls == 0 { return 0.0; }
        self.total_time_ns as f64 / self.total_calls as f64
    }

    #[inline(always)]
    pub fn error_rate(&self) -> f64 {
        if self.total_calls == 0 { return 0.0; }
        self.total_errors as f64 / self.total_calls as f64
    }

    #[inline(always)]
    pub fn is_restartable(&self) -> bool {
        self.flags.contains(&SyscallTableFlag::Restartable)
    }

    #[inline(always)]
    pub fn is_privileged(&self) -> bool {
        self.flags.contains(&SyscallTableFlag::Privileged)
    }
}

/// Syscall number range for allocation
#[derive(Debug, Clone)]
pub struct SyscallRange {
    pub start: u32,
    pub end: u32,
    pub category: SyscallCategory,
    pub allocated: Vec<u32>,
}

impl SyscallRange {
    pub fn new(start: u32, end: u32, category: SyscallCategory) -> Self {
        Self { start, end, category, allocated: Vec::new() }
    }

    #[inline]
    pub fn allocate(&mut self) -> Option<u32> {
        for nr in self.start..=self.end {
            if !self.allocated.contains(&nr) {
                self.allocated.push(nr);
                return Some(nr);
            }
        }
        None
    }

    #[inline(always)]
    pub fn free(&mut self, nr: u32) {
        self.allocated.retain(|&n| n != nr);
    }
}

/// Hot-patch entry
#[derive(Debug, Clone)]
pub struct HotPatch {
    pub syscall_nr: u32,
    pub old_version: u32,
    pub new_version: u32,
    pub applied_ts: u64,
    pub rollback_available: bool,
}

/// Syscall table stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeSyscallTableStats {
    pub total_entries: usize,
    pub enabled_entries: usize,
    pub total_calls: u64,
    pub total_errors: u64,
    pub busiest_syscall: u32,
    pub slowest_syscall: u32,
    pub hot_patches_applied: usize,
}

/// Bridge Syscall Table
#[repr(align(64))]
pub struct BridgeSyscallTable {
    entries: BTreeMap<u32, SyscallEntry>,
    ranges: Vec<SyscallRange>,
    patches: Vec<HotPatch>,
    stats: BridgeSyscallTableStats,
}

impl BridgeSyscallTable {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            ranges: Vec::new(),
            patches: Vec::new(),
            stats: BridgeSyscallTableStats::default(),
        }
    }

    #[inline(always)]
    pub fn register(&mut self, entry: SyscallEntry) {
        self.entries.insert(entry.nr, entry);
        self.recompute();
    }

    #[inline(always)]
    pub fn unregister(&mut self, nr: u32) {
        self.entries.remove(&nr);
        self.recompute();
    }

    #[inline(always)]
    pub fn enable(&mut self, nr: u32) {
        if let Some(e) = self.entries.get_mut(&nr) { e.enabled = true; }
        self.recompute();
    }

    #[inline(always)]
    pub fn disable(&mut self, nr: u32) {
        if let Some(e) = self.entries.get_mut(&nr) { e.enabled = false; }
        self.recompute();
    }

    /// Record a syscall invocation
    #[inline]
    pub fn record(&mut self, nr: u32, duration_ns: u64, is_error: bool) {
        if let Some(e) = self.entries.get_mut(&nr) {
            e.record_call(duration_ns, is_error);
        }
    }

    /// Lookup syscall entry
    #[inline(always)]
    pub fn lookup(&self, nr: u32) -> Option<&SyscallEntry> {
        self.entries.get(&nr)
    }

    /// Find by name hash
    #[inline(always)]
    pub fn find_by_hash(&self, hash: u64) -> Option<&SyscallEntry> {
        self.entries.values().find(|e| e.name_hash == hash)
    }

    /// Add allocation range
    #[inline(always)]
    pub fn add_range(&mut self, range: SyscallRange) {
        self.ranges.push(range);
    }

    /// Allocate a new syscall number from a category range
    #[inline]
    pub fn allocate_nr(&mut self, category: SyscallCategory) -> Option<u32> {
        for range in &mut self.ranges {
            if range.category == category {
                return range.allocate();
            }
        }
        None
    }

    /// Apply hot-patch
    pub fn hot_patch(&mut self, nr: u32, new_version: u32, now: u64) -> bool {
        if let Some(entry) = self.entries.get_mut(&nr) {
            let patch = HotPatch {
                syscall_nr: nr,
                old_version: entry.version,
                new_version,
                applied_ts: now,
                rollback_available: true,
            };
            entry.version = new_version;
            self.patches.push(patch);
            self.recompute();
            true
        } else { false }
    }

    /// Get top N busiest syscalls
    #[inline]
    pub fn top_syscalls(&self, n: usize) -> Vec<(u32, u64)> {
        let mut sorted: Vec<(u32, u64)> = self.entries.iter()
            .map(|(&nr, e)| (nr, e.total_calls))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    fn recompute(&mut self) {
        let total_calls: u64 = self.entries.values().map(|e| e.total_calls).sum();
        let total_errors: u64 = self.entries.values().map(|e| e.total_errors).sum();

        let busiest = self.entries.values()
            .max_by_key(|e| e.total_calls)
            .map(|e| e.nr)
            .unwrap_or(0);

        let slowest = self.entries.values()
            .filter(|e| e.total_calls > 0)
            .max_by(|a, b| a.avg_time_ns().partial_cmp(&b.avg_time_ns()).unwrap_or(core::cmp::Ordering::Equal))
            .map(|e| e.nr)
            .unwrap_or(0);

        self.stats = BridgeSyscallTableStats {
            total_entries: self.entries.len(),
            enabled_entries: self.entries.values().filter(|e| e.enabled).count(),
            total_calls,
            total_errors,
            busiest_syscall: busiest,
            slowest_syscall: slowest,
            hot_patches_applied: self.patches.len(),
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeSyscallTableStats {
        &self.stats
    }
}

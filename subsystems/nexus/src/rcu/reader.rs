//! RCU Reader Tracking
//!
//! This module provides reader tracking for quiescent state detection.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use super::CpuId;

/// Reader lock information
#[derive(Debug, Clone)]
pub struct ReaderInfo {
    /// CPU ID
    pub cpu_id: CpuId,
    /// Nesting depth
    pub nesting_depth: u32,
    /// Entry timestamp
    pub entry_ns: u64,
    /// Last quiescent state timestamp
    pub last_qs_ns: u64,
    /// Is in extended quiescent state
    pub in_eqs: bool,
    /// Critical section count
    pub cs_count: u64,
    /// Total time in critical sections
    pub total_cs_time_ns: u64,
}

impl ReaderInfo {
    /// Create new reader info
    pub fn new(cpu_id: CpuId) -> Self {
        Self {
            cpu_id,
            nesting_depth: 0,
            entry_ns: 0,
            last_qs_ns: 0,
            in_eqs: false,
            cs_count: 0,
            total_cs_time_ns: 0,
        }
    }

    /// Check if in critical section
    #[inline(always)]
    pub fn in_critical_section(&self) -> bool {
        self.nesting_depth > 0
    }

    /// Get average critical section duration
    #[inline]
    pub fn avg_cs_duration_ns(&self) -> u64 {
        if self.cs_count == 0 {
            return 0;
        }
        self.total_cs_time_ns / self.cs_count
    }
}

/// Reader tracking for quiescent state detection
pub struct ReaderTracker {
    /// Per-CPU reader information
    pub(crate) readers: BTreeMap<CpuId, ReaderInfo>,
    /// Online CPU count
    online_cpus: u32,
    /// Total critical sections entered
    total_cs_entries: AtomicU64,
    /// Total critical sections exited
    total_cs_exits: AtomicU64,
    /// Long critical section threshold (nanoseconds)
    long_cs_threshold_ns: u64,
    /// Long critical section count
    long_cs_count: AtomicU64,
    /// Maximum observed nesting depth
    max_nesting_depth: u32,
}

impl ReaderTracker {
    /// Create new reader tracker
    pub fn new() -> Self {
        Self {
            readers: BTreeMap::new(),
            online_cpus: 0,
            total_cs_entries: AtomicU64::new(0),
            total_cs_exits: AtomicU64::new(0),
            long_cs_threshold_ns: 10_000_000, // 10ms
            long_cs_count: AtomicU64::new(0),
            max_nesting_depth: 0,
        }
    }

    /// Register CPU
    #[inline]
    pub fn register_cpu(&mut self, cpu_id: CpuId) {
        self.readers
            .entry(cpu_id)
            .or_insert_with(|| ReaderInfo::new(cpu_id));
        self.online_cpus += 1;
    }

    /// Unregister CPU
    #[inline(always)]
    pub fn unregister_cpu(&mut self, cpu_id: CpuId) {
        self.readers.remove(&cpu_id);
        self.online_cpus = self.online_cpus.saturating_sub(1);
    }

    /// Record critical section entry
    pub fn record_cs_entry(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        if let Some(reader) = self.readers.get_mut(&cpu_id) {
            if reader.nesting_depth == 0 {
                reader.entry_ns = timestamp_ns;
            }
            reader.nesting_depth += 1;
            reader.cs_count += 1;

            if reader.nesting_depth > self.max_nesting_depth {
                self.max_nesting_depth = reader.nesting_depth;
            }
        }
        self.total_cs_entries.fetch_add(1, Ordering::Relaxed);
    }

    /// Record critical section exit
    pub fn record_cs_exit(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        if let Some(reader) = self.readers.get_mut(&cpu_id) {
            reader.nesting_depth = reader.nesting_depth.saturating_sub(1);

            if reader.nesting_depth == 0 {
                let duration = timestamp_ns.saturating_sub(reader.entry_ns);
                reader.total_cs_time_ns += duration;

                if duration > self.long_cs_threshold_ns {
                    self.long_cs_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.total_cs_exits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record quiescent state
    #[inline]
    pub fn record_qs(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        if let Some(reader) = self.readers.get_mut(&cpu_id) {
            reader.last_qs_ns = timestamp_ns;
            reader.in_eqs = false;
        }
    }

    /// Record extended quiescent state entry
    #[inline]
    pub fn record_eqs_entry(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        if let Some(reader) = self.readers.get_mut(&cpu_id) {
            reader.in_eqs = true;
            reader.last_qs_ns = timestamp_ns;
        }
    }

    /// Record extended quiescent state exit
    #[inline]
    pub fn record_eqs_exit(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        if let Some(reader) = self.readers.get_mut(&cpu_id) {
            reader.in_eqs = false;
            reader.last_qs_ns = timestamp_ns;
        }
    }

    /// Get CPUs that have passed quiescent state since timestamp
    #[inline]
    pub fn get_qs_cpus(&self, since_ns: u64) -> Vec<CpuId> {
        self.readers
            .iter()
            .filter(|(_, r)| r.last_qs_ns >= since_ns || r.in_eqs)
            .map(|(cpu, _)| *cpu)
            .collect()
    }

    /// Get CPUs still in critical section
    #[inline]
    pub fn get_blocking_cpus(&self) -> Vec<CpuId> {
        self.readers
            .iter()
            .filter(|(_, r)| r.in_critical_section())
            .map(|(cpu, _)| *cpu)
            .collect()
    }

    /// Get reader info for CPU
    #[inline(always)]
    pub fn get_reader(&self, cpu_id: CpuId) -> Option<&ReaderInfo> {
        self.readers.get(&cpu_id)
    }

    /// Get online CPU count
    #[inline(always)]
    pub fn online_cpu_count(&self) -> u32 {
        self.online_cpus
    }

    /// Get long critical section count
    #[inline(always)]
    pub fn long_cs_count(&self) -> u64 {
        self.long_cs_count.load(Ordering::Relaxed)
    }

    /// Get maximum nesting depth observed
    #[inline(always)]
    pub fn max_nesting_depth(&self) -> u32 {
        self.max_nesting_depth
    }

    /// Set long CS threshold
    #[inline(always)]
    pub fn set_long_cs_threshold(&mut self, threshold_ns: u64) {
        self.long_cs_threshold_ns = threshold_ns;
    }
}

impl Default for ReaderTracker {
    fn default() -> Self {
        Self::new()
    }
}

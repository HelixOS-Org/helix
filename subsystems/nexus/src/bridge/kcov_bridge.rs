//! # Bridge KCOV Bridge
//!
//! Kernel coverage tracing bridging:
//! - Code coverage collection per task
//! - Branch coverage and comparison tracking
//! - Coverage buffer management
//! - Fuzzer feedback integration
//! - Coverage merging and deduplication
//! - Hot path identification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coverage mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcovMode {
    Disabled,
    TracePC,
    TraceCmp,
}

/// Comparison type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpType {
    Const8,
    Const16,
    Const32,
    Const64,
    Switch,
}

/// Comparison record
#[derive(Debug, Clone, Copy)]
pub struct CmpRecord {
    pub pc: u64,
    pub cmp_type: CmpType,
    pub arg1: u64,
    pub arg2: u64,
    pub is_const: bool,
}

/// Coverage entry
#[derive(Debug, Clone, Copy)]
pub struct CovEntry {
    pub pc: u64,
    pub hit_count: u32,
    pub first_hit_ts: u64,
    pub last_hit_ts: u64,
}

impl CovEntry {
    pub fn new(pc: u64, ts: u64) -> Self {
        Self { pc, hit_count: 1, first_hit_ts: ts, last_hit_ts: ts }
    }

    #[inline(always)]
    pub fn record_hit(&mut self, ts: u64) {
        self.hit_count += 1;
        self.last_hit_ts = ts;
    }
}

/// Per-task coverage buffer
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TaskCovBuffer {
    pub task_id: u64,
    pub mode: KcovMode,
    pub pc_entries: Vec<u64>,
    pub cmp_entries: Vec<CmpRecord>,
    pub max_entries: usize,
    pub overflow_count: u64,
    pub enabled: bool,
    pub created_ts: u64,
}

impl TaskCovBuffer {
    pub fn new(task_id: u64, max: usize, ts: u64) -> Self {
        Self {
            task_id, mode: KcovMode::Disabled, pc_entries: Vec::new(),
            cmp_entries: Vec::new(), max_entries: max, overflow_count: 0,
            enabled: false, created_ts: ts,
        }
    }

    #[inline]
    pub fn enable(&mut self, mode: KcovMode) {
        self.mode = mode;
        self.enabled = true;
        self.pc_entries.clear();
        self.cmp_entries.clear();
    }

    #[inline(always)]
    pub fn disable(&mut self) {
        self.mode = KcovMode::Disabled;
        self.enabled = false;
    }

    #[inline]
    pub fn record_pc(&mut self, pc: u64) {
        if !self.enabled || self.mode != KcovMode::TracePC { return; }
        if self.pc_entries.len() >= self.max_entries {
            self.overflow_count += 1;
            return;
        }
        self.pc_entries.push(pc);
    }

    #[inline]
    pub fn record_cmp(&mut self, pc: u64, cmp_type: CmpType, arg1: u64, arg2: u64, is_const: bool) {
        if !self.enabled || self.mode != KcovMode::TraceCmp { return; }
        if self.cmp_entries.len() >= self.max_entries {
            self.overflow_count += 1;
            return;
        }
        self.cmp_entries.push(CmpRecord { pc, cmp_type, arg1, arg2, is_const });
    }

    #[inline]
    pub fn entry_count(&self) -> usize {
        match self.mode {
            KcovMode::TracePC => self.pc_entries.len(),
            KcovMode::TraceCmp => self.cmp_entries.len(),
            KcovMode::Disabled => 0,
        }
    }

    #[inline(always)]
    pub fn fill_ratio(&self) -> f64 {
        if self.max_entries == 0 { return 0.0; }
        self.entry_count() as f64 / self.max_entries as f64
    }
}

/// Merged coverage database
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoverageDatabase {
    pub entries: BTreeMap<u64, CovEntry>,
    pub total_pcs_seen: u64,
    pub last_new_pc_ts: u64,
}

impl CoverageDatabase {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new(), total_pcs_seen: 0, last_new_pc_ts: 0 }
    }

    pub fn merge_buffer(&mut self, pcs: &[u64], ts: u64) -> u64 {
        let mut new_pcs = 0u64;
        for &pc in pcs {
            self.total_pcs_seen += 1;
            if let Some(entry) = self.entries.get_mut(&pc) {
                entry.record_hit(ts);
            } else {
                self.entries.insert(pc, CovEntry::new(pc, ts));
                new_pcs += 1;
                self.last_new_pc_ts = ts;
            }
        }
        new_pcs
    }

    #[inline(always)]
    pub fn unique_pcs(&self) -> usize { self.entries.len() }

    #[inline]
    pub fn hot_pcs(&self, min_hits: u32) -> Vec<u64> {
        self.entries.iter()
            .filter(|(_, e)| e.hit_count >= min_hits)
            .map(|(&pc, _)| pc)
            .collect()
    }

    #[inline]
    pub fn coverage_bitmap(&self, size: usize) -> Vec<u8> {
        let mut bitmap = alloc::vec![0u8; size];
        for &pc in self.entries.keys() {
            let idx = (pc as usize) % (size * 8);
            bitmap[idx / 8] |= 1 << (idx % 8);
        }
        bitmap
    }

    #[inline]
    pub fn edge_hash(from: u64, to: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= from;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= to;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }
}

/// KCOV bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct KcovBridgeStats {
    pub active_buffers: usize,
    pub total_buffers: usize,
    pub unique_pcs: usize,
    pub total_pcs_recorded: u64,
    pub total_cmps_recorded: u64,
    pub total_overflows: u64,
    pub coverage_growth_rate: f64,
}

/// Bridge KCOV manager
#[repr(align(64))]
pub struct BridgeKcovBridge {
    buffers: BTreeMap<u64, TaskCovBuffer>,
    database: CoverageDatabase,
    prev_unique: usize,
    stats: KcovBridgeStats,
}

impl BridgeKcovBridge {
    pub fn new() -> Self {
        Self {
            buffers: BTreeMap::new(), database: CoverageDatabase::new(),
            prev_unique: 0, stats: KcovBridgeStats::default(),
        }
    }

    #[inline(always)]
    pub fn create_buffer(&mut self, task_id: u64, max_entries: usize, ts: u64) {
        self.buffers.insert(task_id, TaskCovBuffer::new(task_id, max_entries, ts));
    }

    #[inline(always)]
    pub fn enable(&mut self, task_id: u64, mode: KcovMode) {
        if let Some(buf) = self.buffers.get_mut(&task_id) { buf.enable(mode); }
    }

    #[inline(always)]
    pub fn disable(&mut self, task_id: u64) {
        if let Some(buf) = self.buffers.get_mut(&task_id) { buf.disable(); }
    }

    #[inline(always)]
    pub fn record_pc(&mut self, task_id: u64, pc: u64) {
        if let Some(buf) = self.buffers.get_mut(&task_id) { buf.record_pc(pc); }
    }

    #[inline(always)]
    pub fn record_cmp(&mut self, task_id: u64, pc: u64, cmp_type: CmpType, arg1: u64, arg2: u64, is_const: bool) {
        if let Some(buf) = self.buffers.get_mut(&task_id) { buf.record_cmp(pc, cmp_type, arg1, arg2, is_const); }
    }

    #[inline]
    pub fn flush_to_database(&mut self, task_id: u64, ts: u64) -> u64 {
        if let Some(buf) = self.buffers.get_mut(&task_id) {
            let pcs = buf.pc_entries.clone();
            buf.pc_entries.clear();
            self.database.merge_buffer(&pcs, ts)
        } else { 0 }
    }

    #[inline]
    pub fn destroy_buffer(&mut self, task_id: u64, ts: u64) {
        if let Some(buf) = self.buffers.remove(&task_id) {
            if !buf.pc_entries.is_empty() {
                self.database.merge_buffer(&buf.pc_entries, ts);
            }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.active_buffers = self.buffers.values().filter(|b| b.enabled).count();
        self.stats.total_buffers = self.buffers.len();
        self.stats.unique_pcs = self.database.unique_pcs();
        self.stats.total_pcs_recorded = self.database.total_pcs_seen;
        self.stats.total_cmps_recorded = self.buffers.values().map(|b| b.cmp_entries.len() as u64).sum();
        self.stats.total_overflows = self.buffers.values().map(|b| b.overflow_count).sum();
        let current = self.database.unique_pcs();
        if self.prev_unique > 0 && current > self.prev_unique {
            self.stats.coverage_growth_rate = (current - self.prev_unique) as f64 / self.prev_unique as f64;
        }
        self.prev_unique = current;
    }

    #[inline(always)]
    pub fn database(&self) -> &CoverageDatabase { &self.database }
    #[inline(always)]
    pub fn buffer(&self, task_id: u64) -> Option<&TaskCovBuffer> { self.buffers.get(&task_id) }
    #[inline(always)]
    pub fn stats(&self) -> &KcovBridgeStats { &self.stats }
}

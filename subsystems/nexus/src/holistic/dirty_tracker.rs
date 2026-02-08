//! # Holistic Dirty Page Tracker
//!
//! Dirty page tracking for holistic memory management:
//! - Per-process dirty page accounting
//! - Rate-limited writeback scheduling
//! - Background vs foreground dirty limits
//! - Dirty page aging and clustering
//! - I/O device-aware throttling
//! - Dirty ratio monitoring and enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Dirty page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyState {
    Clean,
    Dirty,
    Writeback,
    WritebackDone,
    Reclaim,
}

/// Writeback priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WritebackPriority {
    Background,
    Normal,
    Foreground,
    Emergency,
}

/// Per-page dirty tracking entry
#[derive(Debug, Clone)]
pub struct DirtyPage {
    pub pfn: u64,
    pub state: DirtyState,
    pub owner_pid: u32,
    pub inode_id: u64,
    pub dirtied_ts: u64,
    pub age_ns: u64,
    pub write_count: u32,
    pub priority: WritebackPriority,
}

impl DirtyPage {
    pub fn new(pfn: u64, pid: u32, inode: u64, ts: u64) -> Self {
        Self {
            pfn, state: DirtyState::Dirty, owner_pid: pid,
            inode_id: inode, dirtied_ts: ts, age_ns: 0,
            write_count: 1, priority: WritebackPriority::Normal,
        }
    }

    pub fn reclassify(&mut self, now: u64) {
        self.age_ns = now.saturating_sub(self.dirtied_ts);
        if self.age_ns > 30_000_000_000 {
            self.priority = WritebackPriority::Emergency;
        } else if self.age_ns > 10_000_000_000 {
            self.priority = WritebackPriority::Foreground;
        } else if self.age_ns > 5_000_000_000 {
            self.priority = WritebackPriority::Normal;
        } else {
            self.priority = WritebackPriority::Background;
        }
    }

    pub fn re_dirty(&mut self, ts: u64) {
        self.state = DirtyState::Dirty;
        self.write_count += 1;
        if self.write_count > 10 { self.priority = WritebackPriority::Background; }
        self.dirtied_ts = ts;
    }
}

/// Per-process dirty state
#[derive(Debug, Clone)]
pub struct ProcessDirtyState {
    pub pid: u32,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub total_dirtied: u64,
    pub total_written_back: u64,
    pub dirty_limit: u64,
    pub throttled: bool,
    pub throttle_start_ts: u64,
}

impl ProcessDirtyState {
    pub fn new(pid: u32, limit: u64) -> Self {
        Self {
            pid, dirty_pages: 0, writeback_pages: 0,
            total_dirtied: 0, total_written_back: 0,
            dirty_limit: limit, throttled: false, throttle_start_ts: 0,
        }
    }

    pub fn dirty_ratio(&self) -> f64 {
        if self.dirty_limit == 0 { return 0.0; }
        self.dirty_pages as f64 / self.dirty_limit as f64
    }

    pub fn needs_throttle(&self) -> bool { self.dirty_pages >= self.dirty_limit }
}

/// Dirty limits configuration
#[derive(Debug, Clone)]
pub struct DirtyLimits {
    pub global_dirty_limit_pages: u64,
    pub global_bg_limit_pages: u64,
    pub per_process_limit_pages: u64,
    pub dirty_expire_ns: u64,
    pub writeback_interval_ns: u64,
}

impl Default for DirtyLimits {
    fn default() -> Self {
        Self {
            global_dirty_limit_pages: 262144,  // ~1GB with 4K pages
            global_bg_limit_pages: 131072,     // ~512MB
            per_process_limit_pages: 32768,    // ~128MB
            dirty_expire_ns: 30_000_000_000,   // 30s
            writeback_interval_ns: 5_000_000_000, // 5s
        }
    }
}

/// Writeback batch
#[derive(Debug, Clone)]
pub struct WritebackBatch {
    pub batch_id: u64,
    pub pages: Vec<u64>,
    pub priority: WritebackPriority,
    pub inode_id: u64,
    pub started_ts: u64,
    pub completed_ts: Option<u64>,
}

/// Dirty tracker stats
#[derive(Debug, Clone, Default)]
pub struct DirtyTrackerStats {
    pub total_dirty: u64,
    pub total_writeback: u64,
    pub global_dirty_ratio: f64,
    pub tracked_processes: usize,
    pub throttled_processes: usize,
    pub total_batches_issued: u64,
    pub total_pages_written: u64,
    pub avg_dirty_age_ns: f64,
    pub emergency_writebacks: u64,
}

/// Holistic dirty page tracker
pub struct HolisticDirtyTracker {
    pages: BTreeMap<u64, DirtyPage>,
    processes: BTreeMap<u32, ProcessDirtyState>,
    batches: Vec<WritebackBatch>,
    limits: DirtyLimits,
    next_batch_id: u64,
    total_system_pages: u64,
    emergency_writebacks: u64,
    stats: DirtyTrackerStats,
}

impl HolisticDirtyTracker {
    pub fn new(total_pages: u64) -> Self {
        Self {
            pages: BTreeMap::new(), processes: BTreeMap::new(),
            batches: Vec::new(), limits: DirtyLimits::default(),
            next_batch_id: 1, total_system_pages: total_pages,
            emergency_writebacks: 0, stats: DirtyTrackerStats::default(),
        }
    }

    pub fn register_process(&mut self, pid: u32) {
        self.processes.insert(pid, ProcessDirtyState::new(pid, self.limits.per_process_limit_pages));
    }

    pub fn mark_dirty(&mut self, pfn: u64, pid: u32, inode: u64, ts: u64) {
        if let Some(page) = self.pages.get_mut(&pfn) {
            page.re_dirty(ts);
        } else {
            self.pages.insert(pfn, DirtyPage::new(pfn, pid, inode, ts));
        }
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.dirty_pages += 1;
            proc.total_dirtied += 1;
        }
    }

    pub fn complete_writeback(&mut self, pfn: u64) {
        if let Some(page) = self.pages.get_mut(&pfn) {
            let pid = page.owner_pid;
            page.state = DirtyState::Clean;
            if let Some(proc) = self.processes.get_mut(&pid) {
                proc.dirty_pages = proc.dirty_pages.saturating_sub(1);
                proc.total_written_back += 1;
            }
        }
        self.pages.remove(&pfn);
    }

    pub fn schedule_writeback(&mut self, now: u64, max_batch: usize) -> Vec<WritebackBatch> {
        // Age all pages
        for page in self.pages.values_mut() { page.reclassify(now); }

        // Check global limits
        let dirty_count = self.pages.values().filter(|p| p.state == DirtyState::Dirty).count() as u64;
        let needs_emergency = dirty_count > self.limits.global_dirty_limit_pages;

        // Group by inode, sorted by priority
        let mut by_inode: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for page in self.pages.values() {
            if page.state == DirtyState::Dirty {
                by_inode.entry(page.inode_id).or_default().push(page.pfn);
            }
        }

        let mut result = Vec::new();
        for (inode, pfns) in &by_inode {
            let batch_pages: Vec<u64> = pfns.iter().take(max_batch).copied().collect();
            if batch_pages.is_empty() { continue; }
            let prio = if needs_emergency {
                self.emergency_writebacks += 1;
                WritebackPriority::Emergency
            } else {
                WritebackPriority::Background
            };
            let bid = self.next_batch_id; self.next_batch_id += 1;
            let batch = WritebackBatch {
                batch_id: bid, pages: batch_pages, priority: prio,
                inode_id: *inode, started_ts: now, completed_ts: None,
            };
            result.push(batch.clone());
            self.batches.push(batch);
        }

        // Mark pages as writeback
        for batch in &result {
            for &pfn in &batch.pages {
                if let Some(p) = self.pages.get_mut(&pfn) { p.state = DirtyState::Writeback; }
            }
        }

        // Update throttle state
        for proc in self.processes.values_mut() {
            if proc.needs_throttle() && !proc.throttled {
                proc.throttled = true; proc.throttle_start_ts = now;
            } else if !proc.needs_throttle() && proc.throttled {
                proc.throttled = false;
            }
        }

        result
    }

    pub fn recompute(&mut self) {
        self.stats.total_dirty = self.pages.values().filter(|p| p.state == DirtyState::Dirty).count() as u64;
        self.stats.total_writeback = self.pages.values().filter(|p| p.state == DirtyState::Writeback).count() as u64;
        self.stats.global_dirty_ratio = if self.total_system_pages > 0 {
            self.stats.total_dirty as f64 / self.total_system_pages as f64
        } else { 0.0 };
        self.stats.tracked_processes = self.processes.len();
        self.stats.throttled_processes = self.processes.values().filter(|p| p.throttled).count();
        self.stats.total_batches_issued = self.batches.len() as u64;
        self.stats.total_pages_written = self.batches.iter().map(|b| b.pages.len() as u64).sum();
        if !self.pages.is_empty() {
            self.stats.avg_dirty_age_ns = self.pages.values().map(|p| p.age_ns as f64).sum::<f64>() / self.pages.len() as f64;
        }
        self.stats.emergency_writebacks = self.emergency_writebacks;
    }

    pub fn stats(&self) -> &DirtyTrackerStats { &self.stats }
}

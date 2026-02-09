//! # Holistic Writeback Controller
//!
//! Dirty page writeback control with holistic awareness:
//! - BDI (backing device info) bandwidth estimation
//! - Per-device dirty throttling
//! - Global dirty limits and ratios
//! - Writeback thread management
//! - Periodic/background/threshold writeback
//! - Inode writeback state tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Writeback reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackReason {
    Background,
    Periodic,
    ThresholdExceeded,
    Sync,
    FsSync,
    DirectReclaim,
    Shutdown,
}

/// Writeback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackState {
    Idle,
    Active,
    Throttled,
    Congested,
}

/// Backing device info
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BdiState {
    pub id: u64,
    pub write_bw_bps: u64,
    pub avg_write_bw: u64,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub reclaimable_pages: u64,
    pub dirty_exceeded: bool,
    pub max_ratio: u32,
    pub min_ratio: u32,
    pub state: WritebackState,
    pub completions: u64,
}

impl BdiState {
    pub fn new(id: u64) -> Self {
        Self { id, write_bw_bps: 0, avg_write_bw: 0, dirty_pages: 0, writeback_pages: 0, reclaimable_pages: 0, dirty_exceeded: false, max_ratio: 100, min_ratio: 0, state: WritebackState::Idle, completions: 0 }
    }

    #[inline]
    pub fn update_bw(&mut self, bw: u64) {
        if self.avg_write_bw == 0 { self.avg_write_bw = bw; }
        else { self.avg_write_bw = (self.avg_write_bw * 7 + bw) / 8; }
        self.write_bw_bps = bw;
    }

    #[inline(always)]
    pub fn dirty_ratio(&self) -> f64 {
        let total = self.dirty_pages + self.writeback_pages;
        if total + self.reclaimable_pages == 0 { 0.0 } else { self.dirty_pages as f64 / (total + self.reclaimable_pages) as f64 }
    }
}

/// Global dirty limits
#[derive(Debug, Clone)]
pub struct DirtyLimits {
    pub dirty_ratio: u32,
    pub dirty_background_ratio: u32,
    pub dirty_bytes_limit: u64,
    pub dirty_background_bytes: u64,
    pub dirty_writeback_interval_ms: u64,
    pub dirty_expire_interval_ms: u64,
}

impl DirtyLimits {
    #[inline(always)]
    pub fn default_limits() -> Self {
        Self { dirty_ratio: 20, dirty_background_ratio: 10, dirty_bytes_limit: 0, dirty_background_bytes: 0, dirty_writeback_interval_ms: 500, dirty_expire_interval_ms: 3000 }
    }

    #[inline(always)]
    pub fn thresh_pages(&self, total: u64) -> u64 { total * self.dirty_ratio as u64 / 100 }
    #[inline(always)]
    pub fn bg_thresh_pages(&self, total: u64) -> u64 { total * self.dirty_background_ratio as u64 / 100 }
}

/// Writeback work item
#[derive(Debug, Clone)]
pub struct WritebackWork {
    pub id: u64,
    pub bdi_id: u64,
    pub reason: WritebackReason,
    pub nr_to_write: u64,
    pub nr_written: u64,
    pub nr_skipped: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub complete: bool,
}

impl WritebackWork {
    pub fn new(id: u64, bdi: u64, reason: WritebackReason, nr: u64, ts: u64) -> Self {
        Self { id, bdi_id: bdi, reason, nr_to_write: nr, nr_written: 0, nr_skipped: 0, start_ts: ts, end_ts: 0, complete: false }
    }

    #[inline(always)]
    pub fn progress(&mut self, written: u64, skipped: u64) { self.nr_written += written; self.nr_skipped += skipped; }
    #[inline(always)]
    pub fn finish(&mut self, ts: u64) { self.complete = true; self.end_ts = ts; }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.end_ts.saturating_sub(self.start_ts) }
}

/// Inode writeback state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeWbState {
    pub inode: u64,
    pub bdi_id: u64,
    pub dirty_pages: u64,
    pub under_writeback: bool,
    pub dirty_ts: u64,
}

/// Throttle info for a dirtying task
#[derive(Debug, Clone)]
pub struct ThrottleInfo {
    pub task_id: u64,
    pub pause_ns: u64,
    pub dirty_rate_bps: u64,
    pub bdi_id: u64,
    pub ts: u64,
}

/// Writeback stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct WritebackStats {
    pub total_dirty_pages: u64,
    pub total_writeback_pages: u64,
    pub bdis: usize,
    pub active_work: usize,
    pub pages_written: u64,
    pub pages_skipped: u64,
    pub throttled_tasks: usize,
    pub total_bandwidth_bps: u64,
}

/// Holistic writeback controller
pub struct HolisticWritebackCtrl {
    bdis: BTreeMap<u64, BdiState>,
    limits: DirtyLimits,
    work_items: BTreeMap<u64, WritebackWork>,
    inodes: BTreeMap<u64, InodeWbState>,
    throttles: Vec<ThrottleInfo>,
    stats: WritebackStats,
    next_work_id: u64,
    total_memory_pages: u64,
}

impl HolisticWritebackCtrl {
    pub fn new(total_pages: u64) -> Self {
        Self {
            bdis: BTreeMap::new(), limits: DirtyLimits::default_limits(),
            work_items: BTreeMap::new(), inodes: BTreeMap::new(),
            throttles: Vec::new(), stats: WritebackStats::default(),
            next_work_id: 1, total_memory_pages: total_pages,
        }
    }

    #[inline(always)]
    pub fn add_bdi(&mut self, id: u64) { self.bdis.insert(id, BdiState::new(id)); }

    #[inline(always)]
    pub fn update_bdi_dirty(&mut self, bdi: u64, dirty: u64, wb: u64) {
        if let Some(b) = self.bdis.get_mut(&bdi) { b.dirty_pages = dirty; b.writeback_pages = wb; }
    }

    #[inline(always)]
    pub fn update_bdi_bw(&mut self, bdi: u64, bw: u64) {
        if let Some(b) = self.bdis.get_mut(&bdi) { b.update_bw(bw); }
    }

    #[inline(always)]
    pub fn should_writeback(&self) -> bool {
        let total_dirty: u64 = self.bdis.values().map(|b| b.dirty_pages).sum();
        total_dirty >= self.limits.bg_thresh_pages(self.total_memory_pages)
    }

    #[inline(always)]
    pub fn should_throttle(&self) -> bool {
        let total_dirty: u64 = self.bdis.values().map(|b| b.dirty_pages).sum();
        total_dirty >= self.limits.thresh_pages(self.total_memory_pages)
    }

    #[inline]
    pub fn submit_work(&mut self, bdi: u64, reason: WritebackReason, nr: u64, ts: u64) -> u64 {
        let id = self.next_work_id; self.next_work_id += 1;
        self.work_items.insert(id, WritebackWork::new(id, bdi, reason, nr, ts));
        id
    }

    #[inline(always)]
    pub fn progress_work(&mut self, work_id: u64, written: u64, skipped: u64) {
        if let Some(w) = self.work_items.get_mut(&work_id) { w.progress(written, skipped); }
    }

    #[inline]
    pub fn complete_work(&mut self, work_id: u64, ts: u64) {
        if let Some(w) = self.work_items.get_mut(&work_id) {
            w.finish(ts);
            if let Some(b) = self.bdis.get_mut(&w.bdi_id) { b.completions += 1; }
        }
    }

    #[inline(always)]
    pub fn throttle_task(&mut self, task: u64, bdi: u64, pause_ns: u64, rate: u64, ts: u64) {
        self.throttles.push(ThrottleInfo { task_id: task, pause_ns, dirty_rate_bps: rate, bdi_id: bdi, ts });
    }

    #[inline(always)]
    pub fn track_inode(&mut self, inode: u64, bdi: u64, dirty: u64, ts: u64) {
        self.inodes.insert(inode, InodeWbState { inode, bdi_id: bdi, dirty_pages: dirty, under_writeback: false, dirty_ts: ts });
    }

    #[inline(always)]
    pub fn set_limits(&mut self, limits: DirtyLimits) { self.limits = limits; }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_dirty_pages = self.bdis.values().map(|b| b.dirty_pages).sum();
        self.stats.total_writeback_pages = self.bdis.values().map(|b| b.writeback_pages).sum();
        self.stats.bdis = self.bdis.len();
        self.stats.active_work = self.work_items.values().filter(|w| !w.complete).count();
        self.stats.pages_written = self.work_items.values().map(|w| w.nr_written).sum();
        self.stats.pages_skipped = self.work_items.values().map(|w| w.nr_skipped).sum();
        self.stats.throttled_tasks = self.throttles.len();
        self.stats.total_bandwidth_bps = self.bdis.values().map(|b| b.avg_write_bw).sum();
    }

    #[inline(always)]
    pub fn bdi(&self, id: u64) -> Option<&BdiState> { self.bdis.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &WritebackStats { &self.stats }
    #[inline(always)]
    pub fn limits(&self) -> &DirtyLimits { &self.limits }
}

// ============================================================================
// Merged from writeback_ctrl_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WbV2State {
    Idle,
    Running,
    Throttled,
    Congested,
    Completing,
    Errored,
}

/// Dirty page position relative to thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WbV2ThrottleZone {
    Freerun,
    Dirty,
    Background,
    Hard,
    Emergency,
}

/// Bandwidth estimation for a BDI.
#[derive(Debug, Clone)]
pub struct WbV2Bandwidth {
    pub avg_write_bps: u64,
    pub dirty_ratelimit: u64,
    pub balanced_dirty_ratelimit: u64,
    pub pos_ratio: f64,
    pub task_ratelimit: u64,
    pub smoothed_rate: u64,
    pub sample_count: u64,
}

impl WbV2Bandwidth {
    pub fn new() -> Self {
        Self {
            avg_write_bps: 0,
            dirty_ratelimit: 1024 * 1024,
            balanced_dirty_ratelimit: 1024 * 1024,
            pos_ratio: 1.0,
            task_ratelimit: 1024 * 1024,
            smoothed_rate: 0,
            sample_count: 0,
        }
    }

    pub fn update_estimate(&mut self, bytes_written: u64, elapsed_ms: u64) {
        if elapsed_ms == 0 {
            return;
        }
        let bps = (bytes_written * 1000) / elapsed_ms;
        self.sample_count += 1;
        if self.sample_count == 1 {
            self.smoothed_rate = bps;
        } else {
            // Exponential moving average: 7/8 old + 1/8 new
            self.smoothed_rate = (self.smoothed_rate * 7 + bps) / 8;
        }
        self.avg_write_bps = self.smoothed_rate;
    }

    pub fn compute_pos_ratio(&mut self, dirty_pages: u64, thresh: u64, bg_thresh: u64) {
        if thresh == 0 {
            self.pos_ratio = 0.0;
            return;
        }
        let setpoint = (thresh + bg_thresh) / 2;
        if dirty_pages <= setpoint {
            self.pos_ratio = 1.0;
        } else if dirty_pages >= thresh {
            self.pos_ratio = 0.0;
        } else {
            let range = thresh - setpoint;
            let excess = dirty_pages - setpoint;
            self.pos_ratio = 1.0 - (excess as f64 / range as f64);
        }
    }
}

/// A backing device info (BDI) writeback entry.
#[derive(Debug, Clone)]
pub struct WbV2BdiEntry {
    pub bdi_id: u64,
    pub name: String,
    pub state: WbV2State,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub reclaimable_pages: u64,
    pub bandwidth: WbV2Bandwidth,
    pub dirty_threshold: u64,
    pub bg_threshold: u64,
    pub hard_threshold: u64,
    pub congestion_count: u64,
    pub completed_writes: u64,
    pub error_count: u64,
}

impl WbV2BdiEntry {
    pub fn new(bdi_id: u64, name: String) -> Self {
        Self {
            bdi_id,
            name,
            state: WbV2State::Idle,
            dirty_pages: 0,
            writeback_pages: 0,
            reclaimable_pages: 0,
            bandwidth: WbV2Bandwidth::new(),
            dirty_threshold: 4096,
            bg_threshold: 2048,
            hard_threshold: 8192,
            congestion_count: 0,
            completed_writes: 0,
            error_count: 0,
        }
    }

    pub fn throttle_zone(&self) -> WbV2ThrottleZone {
        if self.dirty_pages >= self.hard_threshold {
            WbV2ThrottleZone::Emergency
        } else if self.dirty_pages >= self.dirty_threshold {
            WbV2ThrottleZone::Hard
        } else if self.dirty_pages >= self.bg_threshold {
            WbV2ThrottleZone::Background
        } else if self.dirty_pages > self.bg_threshold / 2 {
            WbV2ThrottleZone::Dirty
        } else {
            WbV2ThrottleZone::Freerun
        }
    }

    #[inline]
    pub fn mark_dirty(&mut self, pages: u64) {
        self.dirty_pages += pages;
        if self.dirty_pages >= self.dirty_threshold {
            self.state = WbV2State::Throttled;
        }
    }

    #[inline]
    pub fn complete_writeback(&mut self, pages: u64) {
        self.writeback_pages = self.writeback_pages.saturating_sub(pages);
        self.dirty_pages = self.dirty_pages.saturating_sub(pages);
        self.completed_writes += pages;
        if self.dirty_pages < self.bg_threshold {
            self.state = WbV2State::Idle;
        }
    }
}

/// Per-cgroup writeback context.
#[derive(Debug, Clone)]
pub struct WbV2CgroupCtx {
    pub cgroup_id: u64,
    pub dirty_pages: u64,
    pub dirty_limit: u64,
    pub throttled_tasks: u64,
    pub total_written: u64,
}

impl WbV2CgroupCtx {
    pub fn new(cgroup_id: u64) -> Self {
        Self {
            cgroup_id,
            dirty_pages: 0,
            dirty_limit: 4096,
            throttled_tasks: 0,
            total_written: 0,
        }
    }

    #[inline(always)]
    pub fn should_throttle(&self) -> bool {
        self.dirty_pages >= self.dirty_limit
    }
}

/// Statistics for the writeback controller V2.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WritebackCtrlV2Stats {
    pub total_bdis: u64,
    pub total_cgroup_contexts: u64,
    pub total_dirty_pages: u64,
    pub total_writeback_pages: u64,
    pub throttle_events: u64,
    pub congestion_events: u64,
    pub bandwidth_samples: u64,
    pub pages_written: u64,
}

/// Main holistic writeback controller V2.
pub struct HolisticWritebackCtrlV2 {
    pub bdis: BTreeMap<u64, WbV2BdiEntry>,
    pub cgroup_contexts: BTreeMap<u64, WbV2CgroupCtx>,
    pub global_dirty_pages: u64,
    pub global_dirty_limit: u64,
    pub next_bdi_id: u64,
    pub stats: WritebackCtrlV2Stats,
}

impl HolisticWritebackCtrlV2 {
    pub fn new() -> Self {
        Self {
            bdis: BTreeMap::new(),
            cgroup_contexts: BTreeMap::new(),
            global_dirty_pages: 0,
            global_dirty_limit: 65536,
            next_bdi_id: 1,
            stats: WritebackCtrlV2Stats {
                total_bdis: 0,
                total_cgroup_contexts: 0,
                total_dirty_pages: 0,
                total_writeback_pages: 0,
                throttle_events: 0,
                congestion_events: 0,
                bandwidth_samples: 0,
                pages_written: 0,
            },
        }
    }

    #[inline]
    pub fn register_bdi(&mut self, name: String) -> u64 {
        let id = self.next_bdi_id;
        self.next_bdi_id += 1;
        let entry = WbV2BdiEntry::new(id, name);
        self.bdis.insert(id, entry);
        self.stats.total_bdis += 1;
        id
    }

    #[inline]
    pub fn register_cgroup(&mut self, cgroup_id: u64) {
        if !self.cgroup_contexts.contains_key(&cgroup_id) {
            let ctx = WbV2CgroupCtx::new(cgroup_id);
            self.cgroup_contexts.insert(cgroup_id, ctx);
            self.stats.total_cgroup_contexts += 1;
        }
    }

    #[inline]
    pub fn mark_dirty(&mut self, bdi_id: u64, pages: u64) -> bool {
        if let Some(bdi) = self.bdis.get_mut(&bdi_id) {
            bdi.mark_dirty(pages);
            self.global_dirty_pages += pages;
            self.stats.total_dirty_pages += pages;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn complete_writeback(&mut self, bdi_id: u64, pages: u64) -> bool {
        if let Some(bdi) = self.bdis.get_mut(&bdi_id) {
            bdi.complete_writeback(pages);
            self.global_dirty_pages = self.global_dirty_pages.saturating_sub(pages);
            self.stats.pages_written += pages;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn bdi_count(&self) -> usize {
        self.bdis.len()
    }

    #[inline(always)]
    pub fn cgroup_count(&self) -> usize {
        self.cgroup_contexts.len()
    }
}

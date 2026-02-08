//! # Apps RSS Tracker
//!
//! Resident Set Size tracking and analysis:
//! - Per-process RSS evolution over time
//! - RSS growth rate detection
//! - Peak RSS tracking
//! - RSS breakdown by VMA type
//! - Memory bloat detection
//! - Proportional set size (PSS) estimation
//! - Shared vs private memory ratio

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// VMA category for RSS breakdown
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmaCategory {
    /// Code/text segment
    Code,
    /// Data/BSS
    Data,
    /// Heap (brk)
    Heap,
    /// Anonymous mmap
    AnonMmap,
    /// File-backed mmap
    FileMmap,
    /// Stack
    Stack,
    /// Shared memory (shmem)
    SharedMem,
    /// GPU/device memory
    Device,
    /// Other
    Other,
}

/// RSS sample
#[derive(Debug, Clone)]
pub struct RssSample {
    pub timestamp_ns: u64,
    pub rss_pages: u64,
    pub shared_pages: u64,
    pub private_pages: u64,
    pub swap_pages: u64,
}

/// VMA RSS entry
#[derive(Debug, Clone)]
pub struct VmaRssEntry {
    pub start_addr: u64,
    pub end_addr: u64,
    pub category: VmaCategory,
    pub rss_pages: u64,
    pub shared_pages: u64,
    pub pss_pages_x1000: u64, // PSS * 1000 for precision
    pub referenced: bool,
}

impl VmaRssEntry {
    pub fn size_pages(&self) -> u64 {
        (self.end_addr - self.start_addr) / 4096
    }

    pub fn residency_ratio(&self) -> f64 {
        let total = self.size_pages();
        if total == 0 { 0.0 } else { self.rss_pages as f64 / total as f64 }
    }

    pub fn pss_pages(&self) -> f64 {
        self.pss_pages_x1000 as f64 / 1000.0
    }
}

/// Per-process RSS profile
#[derive(Debug)]
pub struct ProcessRssProfile {
    pub pid: u64,
    pub current_rss_pages: u64,
    pub peak_rss_pages: u64,
    pub current_swap_pages: u64,
    /// RSS history (ring buffer, last 128 samples)
    rss_history: Vec<RssSample>,
    history_head: usize,
    /// VMA breakdown
    vma_breakdown: BTreeMap<u64, VmaRssEntry>,
    /// Per-category aggregate
    category_rss: BTreeMap<u8, u64>,
    /// Growth rate (pages per second, EMA)
    pub growth_rate: f64,
    pub total_shared_pages: u64,
    pub total_private_pages: u64,
    pub sample_count: u64,
}

impl ProcessRssProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            current_rss_pages: 0,
            peak_rss_pages: 0,
            current_swap_pages: 0,
            rss_history: Vec::new(),
            history_head: 0,
            vma_breakdown: BTreeMap::new(),
            category_rss: BTreeMap::new(),
            growth_rate: 0.0,
            total_shared_pages: 0,
            total_private_pages: 0,
            sample_count: 0,
        }
    }

    /// Record an RSS sample
    pub fn record_sample(&mut self, sample: RssSample) {
        let prev_rss = self.current_rss_pages;
        let prev_ts = self.rss_history.last().map(|s| s.timestamp_ns).unwrap_or(sample.timestamp_ns);

        self.current_rss_pages = sample.rss_pages;
        self.current_swap_pages = sample.swap_pages;
        self.total_shared_pages = sample.shared_pages;
        self.total_private_pages = sample.private_pages;

        if sample.rss_pages > self.peak_rss_pages {
            self.peak_rss_pages = sample.rss_pages;
        }

        // Growth rate
        let dt = sample.timestamp_ns.saturating_sub(prev_ts);
        if dt > 0 && self.sample_count > 0 {
            let instant_rate = (sample.rss_pages as f64 - prev_rss as f64)
                / (dt as f64 / 1_000_000_000.0);
            self.growth_rate = 0.8 * self.growth_rate + 0.2 * instant_rate;
        }

        // Ring buffer
        if self.rss_history.len() < 128 {
            self.rss_history.push(sample);
        } else {
            self.rss_history[self.history_head] = sample;
            self.history_head = (self.history_head + 1) % 128;
        }
        self.sample_count += 1;
    }

    /// Update VMA breakdown
    pub fn update_vma(&mut self, entry: VmaRssEntry) {
        self.vma_breakdown.insert(entry.start_addr, entry);
        self.recompute_categories();
    }

    fn recompute_categories(&mut self) {
        self.category_rss.clear();
        for entry in self.vma_breakdown.values() {
            *self.category_rss.entry(entry.category as u8).or_insert(0) += entry.rss_pages;
        }
    }

    /// RSS by category
    pub fn rss_by_category(&self) -> Vec<(VmaCategory, u64)> {
        let cats = [
            VmaCategory::Code, VmaCategory::Data, VmaCategory::Heap,
            VmaCategory::AnonMmap, VmaCategory::FileMmap, VmaCategory::Stack,
            VmaCategory::SharedMem, VmaCategory::Device, VmaCategory::Other,
        ];
        cats.iter()
            .filter_map(|&cat| {
                self.category_rss.get(&(cat as u8)).map(|&pages| (cat, pages))
            })
            .collect()
    }

    /// Shared memory ratio
    pub fn shared_ratio(&self) -> f64 {
        if self.current_rss_pages == 0 { 0.0 } else {
            self.total_shared_pages as f64 / self.current_rss_pages as f64
        }
    }

    /// Is RSS growing monotonically? (potential leak)
    pub fn is_monotonic_growth(&self) -> bool {
        if self.rss_history.len() < 16 {
            return false;
        }
        let mut sorted: Vec<u64> = self.rss_history.iter()
            .map(|s| s.rss_pages)
            .collect();
        // Check last 16 samples
        let start = if sorted.len() > 16 { sorted.len() - 16 } else { 0 };
        let recent = &sorted[start..];
        let increases = recent.windows(2).filter(|w| w[1] >= w[0]).count();
        increases as f64 / (recent.len() - 1) as f64 > 0.85
    }

    /// Memory bloat ratio: current / peak
    pub fn bloat_ratio(&self) -> f64 {
        if self.peak_rss_pages == 0 { 0.0 } else {
            self.current_rss_pages as f64 / self.peak_rss_pages as f64
        }
    }

    /// Estimated PSS (proportional set size)
    pub fn estimated_pss_pages(&self) -> f64 {
        self.vma_breakdown.values()
            .map(|v| v.pss_pages())
            .sum()
    }

    /// Swap pressure (swap / (rss + swap))
    pub fn swap_pressure(&self) -> f64 {
        let total = self.current_rss_pages + self.current_swap_pages;
        if total == 0 { 0.0 } else { self.current_swap_pages as f64 / total as f64 }
    }

    /// Reclaimable pages (file-backed with low residency)
    pub fn reclaimable_estimate(&self) -> u64 {
        self.vma_breakdown.values()
            .filter(|v| v.category == VmaCategory::FileMmap && !v.referenced)
            .map(|v| v.rss_pages)
            .sum()
    }
}

/// RSS tracker global stats
#[derive(Debug, Clone, Default)]
pub struct AppRssTrackerStats {
    pub tracked_processes: usize,
    pub total_rss_pages: u64,
    pub total_swap_pages: u64,
    pub peak_rss_pages: u64,
    pub growing_processes: usize,
    pub high_swap_count: usize,
    pub avg_shared_ratio: f64,
}

/// App RSS Tracker
pub struct AppRssTracker {
    processes: BTreeMap<u64, ProcessRssProfile>,
    stats: AppRssTrackerStats,
}

impl AppRssTracker {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppRssTrackerStats::default(),
        }
    }

    pub fn record_sample(&mut self, pid: u64, sample: RssSample) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessRssProfile::new(pid))
            .record_sample(sample);
        self.update_stats();
    }

    pub fn update_vma(&mut self, pid: u64, entry: VmaRssEntry) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessRssProfile::new(pid))
            .update_vma(entry);
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_rss_pages = self.processes.values()
            .map(|p| p.current_rss_pages).sum();
        self.stats.total_swap_pages = self.processes.values()
            .map(|p| p.current_swap_pages).sum();
        self.stats.peak_rss_pages = self.processes.values()
            .map(|p| p.peak_rss_pages).max().unwrap_or(0);
        self.stats.growing_processes = self.processes.values()
            .filter(|p| p.is_monotonic_growth()).count();
        self.stats.high_swap_count = self.processes.values()
            .filter(|p| p.swap_pressure() > 0.3).count();
        if !self.processes.is_empty() {
            self.stats.avg_shared_ratio = self.processes.values()
                .map(|p| p.shared_ratio())
                .sum::<f64>() / self.processes.len() as f64;
        }
    }

    pub fn stats(&self) -> &AppRssTrackerStats {
        &self.stats
    }

    /// Top RSS consumers
    pub fn top_consumers(&self, n: usize) -> Vec<(u64, u64)> {
        let mut procs: Vec<(u64, u64)> = self.processes.iter()
            .map(|(&pid, p)| (pid, p.current_rss_pages))
            .collect();
        procs.sort_by(|a, b| b.1.cmp(&a.1));
        procs.truncate(n);
        procs
    }

    /// Processes with monotonic RSS growth (potential leaks)
    pub fn leaking_candidates(&self) -> Vec<u64> {
        self.processes.iter()
            .filter(|(_, p)| p.is_monotonic_growth() && p.current_rss_pages > 1024)
            .map(|(&pid, _)| pid)
            .collect()
    }
}

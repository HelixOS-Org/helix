//! # Holistic RSS Tracker
//!
//! Resident Set Size tracking and management:
//! - Per-process RSS accounting (anon, file, shmem)
//! - RSS limit enforcement and soft/hard limits
//! - Working set size estimation
//! - RSS growth rate prediction
//! - Memory pressure contribution tracking
//! - RSS-based OOM scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RSS component type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RssComponent {
    AnonPages,
    FilePages,
    ShmemPages,
    SwappedPages,
    PageTablePages,
}

/// RSS limit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RssLimitType {
    Soft,
    Hard,
    None,
}

/// Per-process RSS state
#[derive(Debug, Clone)]
pub struct ProcessRss {
    pub pid: u32,
    pub anon_pages: u64,
    pub file_pages: u64,
    pub shmem_pages: u64,
    pub swapped_pages: u64,
    pub pgtable_pages: u64,
    pub soft_limit_pages: Option<u64>,
    pub hard_limit_pages: Option<u64>,
    pub peak_rss_pages: u64,
    pub growth_rate_pages_per_sec: f64,
    pub last_sample_ts: u64,
    pub prev_total: u64,
    pub oom_score: u32,
    pub samples: Vec<(u64, u64)>, // (timestamp, total_rss)
}

impl ProcessRss {
    pub fn new(pid: u32) -> Self {
        Self {
            pid, anon_pages: 0, file_pages: 0, shmem_pages: 0,
            swapped_pages: 0, pgtable_pages: 0,
            soft_limit_pages: None, hard_limit_pages: None,
            peak_rss_pages: 0, growth_rate_pages_per_sec: 0.0,
            last_sample_ts: 0, prev_total: 0, oom_score: 0,
            samples: Vec::new(),
        }
    }

    pub fn total_rss(&self) -> u64 {
        self.anon_pages + self.file_pages + self.shmem_pages
    }

    pub fn total_with_swap(&self) -> u64 {
        self.total_rss() + self.swapped_pages
    }

    pub fn total_bytes(&self) -> u64 { self.total_rss() * 4096 }

    pub fn update(&mut self, anon: u64, file: u64, shmem: u64, swap: u64, pt: u64, ts: u64) {
        self.anon_pages = anon;
        self.file_pages = file;
        self.shmem_pages = shmem;
        self.swapped_pages = swap;
        self.pgtable_pages = pt;

        let total = self.total_rss();
        if total > self.peak_rss_pages { self.peak_rss_pages = total; }

        // Growth rate
        if self.last_sample_ts > 0 {
            let dt = ts.saturating_sub(self.last_sample_ts) as f64 / 1_000_000_000.0;
            if dt > 0.0 {
                let delta = total as f64 - self.prev_total as f64;
                self.growth_rate_pages_per_sec = delta / dt;
            }
        }

        self.prev_total = total;
        self.last_sample_ts = ts;
        self.samples.push((ts, total));
        if self.samples.len() > 100 { self.samples.remove(0); }
    }

    pub fn exceeds_soft_limit(&self) -> bool {
        self.soft_limit_pages.map(|l| self.total_rss() > l).unwrap_or(false)
    }

    pub fn exceeds_hard_limit(&self) -> bool {
        self.hard_limit_pages.map(|l| self.total_rss() > l).unwrap_or(false)
    }

    pub fn limit_status(&self) -> RssLimitType {
        if self.exceeds_hard_limit() { RssLimitType::Hard }
        else if self.exceeds_soft_limit() { RssLimitType::Soft }
        else { RssLimitType::None }
    }

    pub fn compute_oom_score(&mut self, total_mem_pages: u64) {
        if total_mem_pages == 0 { self.oom_score = 0; return; }
        // Score 0-1000 based on RSS proportion
        let ratio = self.total_rss() as f64 / total_mem_pages as f64;
        let base_score = (ratio * 1000.0) as u32;
        // Bonus for rapid growth
        let growth_bonus = if self.growth_rate_pages_per_sec > 1000.0 { 100 } else { 0 };
        self.oom_score = (base_score + growth_bonus).min(1000);
    }

    pub fn working_set_estimate(&self) -> u64 {
        // Estimate working set from recent samples
        if self.samples.len() < 2 { return self.total_rss(); }
        let recent: Vec<u64> = self.samples.iter().rev().take(10).map(|(_, rss)| *rss).collect();
        let avg = recent.iter().sum::<u64>() / recent.len() as u64;
        avg
    }
}

/// System-wide RSS summary
#[derive(Debug, Clone)]
pub struct SystemRssSummary {
    pub total_anon_pages: u64,
    pub total_file_pages: u64,
    pub total_shmem_pages: u64,
    pub total_swap_pages: u64,
    pub total_rss_pages: u64,
    pub total_memory_pages: u64,
    pub rss_utilization: f64,
}

/// RSS tracker stats
#[derive(Debug, Clone, Default)]
pub struct RssTrackerStats {
    pub tracked_processes: usize,
    pub total_rss_pages: u64,
    pub total_rss_bytes: u64,
    pub soft_limit_violators: usize,
    pub hard_limit_violators: usize,
    pub avg_rss_pages: f64,
    pub max_rss_pages: u64,
    pub avg_growth_rate: f64,
    pub top_oom_score: u32,
    pub system_rss_ratio: f64,
}

/// Holistic RSS tracker
pub struct HolisticRssTracker {
    processes: BTreeMap<u32, ProcessRss>,
    total_memory_pages: u64,
    stats: RssTrackerStats,
}

impl HolisticRssTracker {
    pub fn new(total_pages: u64) -> Self {
        Self {
            processes: BTreeMap::new(), total_memory_pages: total_pages,
            stats: RssTrackerStats::default(),
        }
    }

    pub fn register(&mut self, pid: u32) {
        self.processes.insert(pid, ProcessRss::new(pid));
    }

    pub fn unregister(&mut self, pid: u32) { self.processes.remove(&pid); }

    pub fn update(&mut self, pid: u32, anon: u64, file: u64, shmem: u64, swap: u64, pt: u64, ts: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.update(anon, file, shmem, swap, pt, ts);
        }
    }

    pub fn set_limits(&mut self, pid: u32, soft: Option<u64>, hard: Option<u64>) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.soft_limit_pages = soft;
            proc.hard_limit_pages = hard;
        }
    }

    pub fn compute_oom_scores(&mut self) {
        let total = self.total_memory_pages;
        for proc in self.processes.values_mut() { proc.compute_oom_score(total); }
    }

    pub fn oom_candidates(&self, n: usize) -> Vec<(u32, u32)> {
        let mut sorted: Vec<(u32, u32)> = self.processes.values().map(|p| (p.pid, p.oom_score)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    pub fn system_summary(&self) -> SystemRssSummary {
        let total_anon: u64 = self.processes.values().map(|p| p.anon_pages).sum();
        let total_file: u64 = self.processes.values().map(|p| p.file_pages).sum();
        let total_shmem: u64 = self.processes.values().map(|p| p.shmem_pages).sum();
        let total_swap: u64 = self.processes.values().map(|p| p.swapped_pages).sum();
        let total_rss = total_anon + total_file + total_shmem;
        SystemRssSummary {
            total_anon_pages: total_anon, total_file_pages: total_file,
            total_shmem_pages: total_shmem, total_swap_pages: total_swap,
            total_rss_pages: total_rss, total_memory_pages: self.total_memory_pages,
            rss_utilization: if self.total_memory_pages > 0 { total_rss as f64 / self.total_memory_pages as f64 } else { 0.0 },
        }
    }

    pub fn process(&self, pid: u32) -> Option<&ProcessRss> { self.processes.get(&pid) }

    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_rss_pages = self.processes.values().map(|p| p.total_rss()).sum();
        self.stats.total_rss_bytes = self.stats.total_rss_pages * 4096;
        self.stats.soft_limit_violators = self.processes.values().filter(|p| p.exceeds_soft_limit()).count();
        self.stats.hard_limit_violators = self.processes.values().filter(|p| p.exceeds_hard_limit()).count();
        if !self.processes.is_empty() {
            let n = self.processes.len() as f64;
            self.stats.avg_rss_pages = self.stats.total_rss_pages as f64 / n;
            self.stats.max_rss_pages = self.processes.values().map(|p| p.total_rss()).max().unwrap_or(0);
            self.stats.avg_growth_rate = self.processes.values().map(|p| p.growth_rate_pages_per_sec).sum::<f64>() / n;
            self.stats.top_oom_score = self.processes.values().map(|p| p.oom_score).max().unwrap_or(0);
        }
        self.stats.system_rss_ratio = if self.total_memory_pages > 0 {
            self.stats.total_rss_pages as f64 / self.total_memory_pages as f64
        } else { 0.0 };
    }

    pub fn stats(&self) -> &RssTrackerStats { &self.stats }
}

// SPDX-License-Identifier: MIT
//! # Holistic Memory Unmap Tracking
//!
//! System-wide unmap pattern analysis:
//! - Global memory reclamation rate monitoring
//! - System-wide leak detection heuristics
//! - Address space recycling efficiency dashboard
//! - Unmap storm detection (bulk teardown events)
//! - Post-exit cleanup coordinator

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimPhase { Idle, Gradual, Active, Storm, PostExit }

#[derive(Debug, Clone)]
pub struct SystemLeakReport {
    pub pid: u64,
    pub suspected_leaked_bytes: u64,
    pub region_count: u32,
    pub age_ticks: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct UnmapStorm {
    pub start_time: u64,
    pub end_time: u64,
    pub unmaps_count: u64,
    pub bytes_freed: u64,
    pub pids_involved: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct RecycleEfficiency {
    pub total_freed: u64,
    pub total_reused: u64,
    pub avg_recycle_latency: u64,
}

impl RecycleEfficiency {
    #[inline(always)]
    pub fn ratio(&self) -> f64 {
        if self.total_freed == 0 { return 0.0; }
        self.total_reused as f64 / self.total_freed as f64
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MunmapHolisticStats {
    pub total_unmaps: u64,
    pub total_bytes_freed: u64,
    pub reclaim_rate_bps: u64,
    pub active_leaks: u64,
    pub storms_detected: u64,
    pub recycle_efficiency: f64,
}

pub struct MunmapHolisticManager {
    /// Per-process unmap rate: pid → (recent_unmaps, recent_bytes)
    unmap_rates: BTreeMap<u64, (u64, u64)>,
    leak_reports: Vec<SystemLeakReport>,
    storms: Vec<UnmapStorm>,
    recycle: RecycleEfficiency,
    /// Sliding window of unmap events for storm detection
    recent_unmaps: VecDeque<(u64, u64, u64)>, // (timestamp, pid, bytes)
    window_size: usize,
    storm_threshold: u64, // unmaps per window to trigger storm
    phase: ReclaimPhase,
    stats: MunmapHolisticStats,
}

impl MunmapHolisticManager {
    pub fn new(storm_threshold: u64) -> Self {
        Self {
            unmap_rates: BTreeMap::new(),
            leak_reports: Vec::new(),
            storms: Vec::new(),
            recycle: RecycleEfficiency {
                total_freed: 0, total_reused: 0, avg_recycle_latency: 0,
            },
            recent_unmaps: VecDeque::new(),
            window_size: 1024,
            storm_threshold,
            phase: ReclaimPhase::Idle,
            stats: MunmapHolisticStats::default(),
        }
    }

    /// Record a system-wide unmap event
    pub fn record_unmap(&mut self, pid: u64, bytes: u64, now: u64) {
        self.stats.total_unmaps += 1;
        self.stats.total_bytes_freed += bytes;

        let rate = self.unmap_rates.entry(pid).or_insert((0, 0));
        rate.0 += 1;
        rate.1 += bytes;

        self.recent_unmaps.push_back((now, pid, bytes));
        if self.recent_unmaps.len() > self.window_size {
            self.recent_unmaps.pop_front();
        }

        self.detect_storm(now);
    }

    fn detect_storm(&mut self, now: u64) {
        if self.recent_unmaps.len() < 10 { return; }

        // Count unmaps in last 100ms
        let cutoff = now.saturating_sub(100_000_000);
        let window: Vec<_> = self.recent_unmaps.iter()
            .filter(|&&(ts, _, _)| ts >= cutoff)
            .collect();

        if window.len() as u64 > self.storm_threshold {
            self.phase = ReclaimPhase::Storm;
            let bytes: u64 = window.iter().map(|&&(_, _, b)| b).sum();
            let pids: Vec<u64> = window.iter().map(|&&(_, p, _)| p).collect();
            self.storms.push(UnmapStorm {
                start_time: cutoff, end_time: now,
                unmaps_count: window.len() as u64,
                bytes_freed: bytes, pids_involved: pids,
            });
            self.stats.storms_detected += 1;
        } else if self.phase == ReclaimPhase::Storm {
            self.phase = ReclaimPhase::Active;
        }
    }

    /// Report a suspected memory leak
    #[inline(always)]
    pub fn report_leak(&mut self, report: SystemLeakReport) {
        self.stats.active_leaks += 1;
        self.leak_reports.push(report);
    }

    /// Get top leak suspects sorted by confidence
    #[inline]
    pub fn top_leak_suspects(&self, n: usize) -> Vec<&SystemLeakReport> {
        let mut sorted: Vec<_> = self.leak_reports.iter().collect();
        sorted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence)
            .unwrap_or(core::cmp::Ordering::Equal));
        sorted.into_iter().take(n).collect()
    }

    /// Record address recycling
    #[inline]
    pub fn record_recycle(&mut self, bytes: u64, latency_ns: u64) {
        self.recycle.total_reused += bytes;
        self.recycle.avg_recycle_latency = (self.recycle.avg_recycle_latency * 7 + latency_ns) / 8;
        self.stats.recycle_efficiency = self.recycle.ratio();
    }

    /// Compute global reclaim rate (bytes per second)
    #[inline]
    pub fn compute_reclaim_rate(&mut self, window_ns: u64) -> u64 {
        if window_ns == 0 { return 0; }
        let rate = (self.stats.total_bytes_freed * 1_000_000_000) / window_ns;
        self.stats.reclaim_rate_bps = rate;
        rate
    }

    /// Process exit cleanup: bulk reclaim
    #[inline]
    pub fn process_exit(&mut self, pid: u64) {
        if let Some((_, bytes)) = self.unmap_rates.remove(&pid) {
            self.phase = ReclaimPhase::PostExit;
            self.recycle.total_freed += bytes;
        }
        self.leak_reports.retain(|r| r.pid != pid);
    }

    #[inline(always)]
    pub fn phase(&self) -> ReclaimPhase { self.phase }
    #[inline(always)]
    pub fn stats(&self) -> &MunmapHolisticStats { &self.stats }
}

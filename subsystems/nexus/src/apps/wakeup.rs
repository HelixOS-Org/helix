//! # Apps Wakeup
//!
//! Wakeup chain profiling and analysis:
//! - Wakeup source tracking (who wakes whom)
//! - Wakeup latency measurement
//! - Chain depth analysis
//! - IPI tracking
//! - Cross-CPU wakeup cost
//! - Wakeup storm detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wakeup source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupSource {
    /// Direct from another thread
    Thread,
    /// From interrupt handler
    Interrupt,
    /// Timer expiry
    Timer,
    /// IO completion
    IoCompletion,
    /// Signal delivery
    Signal,
    /// Futex wake
    Futex,
    /// IPI (inter-processor interrupt)
    Ipi,
    /// Unknown
    Unknown,
}

/// A single wakeup event
#[derive(Debug, Clone)]
pub struct WakeupEvent {
    pub waker_tid: u64,
    pub wakee_tid: u64,
    pub source: WakeupSource,
    pub waker_cpu: u32,
    pub wakee_cpu: u32,
    pub timestamp_ns: u64,
    /// Time from wakeup to actually running
    pub latency_ns: u64,
}

impl WakeupEvent {
    /// Is this a cross-CPU wakeup?
    #[inline(always)]
    pub fn is_cross_cpu(&self) -> bool {
        self.waker_cpu != self.wakee_cpu
    }
}

/// Wakeup edge (who wakes whom, aggregated)
#[derive(Debug, Clone)]
pub struct WakeupEdge {
    pub waker_tid: u64,
    pub wakee_tid: u64,
    pub count: u64,
    pub total_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub cross_cpu_count: u64,
}

impl WakeupEdge {
    pub fn new(waker: u64, wakee: u64) -> Self {
        Self {
            waker_tid: waker,
            wakee_tid: wakee,
            count: 0,
            total_latency_ns: 0,
            min_latency_ns: u64::MAX,
            max_latency_ns: 0,
            cross_cpu_count: 0,
        }
    }

    pub fn record(&mut self, latency_ns: u64, cross_cpu: bool) {
        self.count += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns < self.min_latency_ns {
            self.min_latency_ns = latency_ns;
        }
        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }
        if cross_cpu {
            self.cross_cpu_count += 1;
        }
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.total_latency_ns as f64 / self.count as f64 }
    }

    #[inline(always)]
    pub fn cross_cpu_ratio(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.cross_cpu_count as f64 / self.count as f64 }
    }
}

/// Per-thread wakeup stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ThreadWakeupStats {
    pub tid: u64,
    /// Times this thread was woken
    pub times_woken: u64,
    /// Times this thread woke others
    pub times_waking: u64,
    /// Total wakeup latency experienced
    pub total_wakeup_latency_ns: u64,
    /// Wakeup sources experienced
    source_counts: BTreeMap<u8, u64>,
    /// Recent wakeup timestamps (ring buffer, last 32)
    recent_wakeups: Vec<u64>,
    recent_head: usize,
}

impl ThreadWakeupStats {
    pub fn new(tid: u64) -> Self {
        Self {
            tid,
            times_woken: 0,
            times_waking: 0,
            total_wakeup_latency_ns: 0,
            source_counts: BTreeMap::new(),
            recent_wakeups: Vec::new(),
            recent_head: 0,
        }
    }

    pub fn record_woken(&mut self, latency_ns: u64, source: WakeupSource, ts: u64) {
        self.times_woken += 1;
        self.total_wakeup_latency_ns += latency_ns;
        *self.source_counts.entry(source as u8).or_insert(0) += 1;

        if self.recent_wakeups.len() < 32 {
            self.recent_wakeups.push(ts);
        } else {
            self.recent_wakeups[self.recent_head] = ts;
            self.recent_head = (self.recent_head + 1) % 32;
        }
    }

    #[inline(always)]
    pub fn record_waking(&mut self) {
        self.times_waking += 1;
    }

    #[inline]
    pub fn avg_wakeup_latency_ns(&self) -> f64 {
        if self.times_woken == 0 { 0.0 } else {
            self.total_wakeup_latency_ns as f64 / self.times_woken as f64
        }
    }

    /// Detect wakeup storm (many wakeups in short interval)
    pub fn is_wakeup_storm(&self) -> bool {
        if self.recent_wakeups.len() < 16 {
            return false;
        }
        // Check if 16+ wakeups in last 1ms
        let mut timestamps: Vec<u64> = self.recent_wakeups.clone();
        timestamps.sort();
        let window = 1_000_000; // 1ms
        for i in 0..timestamps.len().saturating_sub(15) {
            if timestamps[i + 15] - timestamps[i] < window {
                return true;
            }
        }
        false
    }

    /// Wakeup frequency (per second, estimated from recent)
    pub fn wakeup_frequency(&self) -> f64 {
        if self.recent_wakeups.len() < 2 {
            return 0.0;
        }
        let mut timestamps: Vec<u64> = self.recent_wakeups.clone();
        timestamps.sort();
        let span = timestamps.last().unwrap_or(&0) - timestamps.first().unwrap_or(&0);
        if span == 0 {
            return 0.0;
        }
        (timestamps.len() as f64 - 1.0) * 1_000_000_000.0 / span as f64
    }
}

/// Wakeup chain (sequence of wakeups forming a critical path)
#[derive(Debug, Clone)]
pub struct WakeupChain {
    pub chain: Vec<u64>,
    pub total_latency_ns: u64,
}

impl WakeupChain {
    #[inline(always)]
    pub fn depth(&self) -> usize {
        self.chain.len()
    }
}

/// Wakeup profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppWakeupProfilerStats {
    pub total_wakeups: u64,
    pub cross_cpu_wakeups: u64,
    pub avg_wakeup_latency_ns: f64,
    pub wakeup_storm_threads: usize,
    pub tracked_threads: usize,
    pub unique_edges: usize,
}

/// App Wakeup Profiler
pub struct AppWakeupProfiler {
    threads: BTreeMap<u64, ThreadWakeupStats>,
    /// Wakeup edges: (waker, wakee) -> stats
    edges: BTreeMap<(u64, u64), WakeupEdge>,
    stats: AppWakeupProfilerStats,
}

impl AppWakeupProfiler {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            edges: BTreeMap::new(),
            stats: AppWakeupProfilerStats::default(),
        }
    }

    pub fn record_wakeup(&mut self, event: &WakeupEvent) {
        // Update wakee
        self.threads.entry(event.wakee_tid)
            .or_insert_with(|| ThreadWakeupStats::new(event.wakee_tid))
            .record_woken(event.latency_ns, event.source, event.timestamp_ns);

        // Update waker
        self.threads.entry(event.waker_tid)
            .or_insert_with(|| ThreadWakeupStats::new(event.waker_tid))
            .record_waking();

        // Update edge
        let key = (event.waker_tid, event.wakee_tid);
        self.edges.entry(key)
            .or_insert_with(|| WakeupEdge::new(event.waker_tid, event.wakee_tid))
            .record(event.latency_ns, event.is_cross_cpu());

        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_threads = self.threads.len();
        self.stats.unique_edges = self.edges.len();
        self.stats.total_wakeups = self.threads.values().map(|t| t.times_woken).sum();
        self.stats.cross_cpu_wakeups = self.edges.values().map(|e| e.cross_cpu_count).sum();

        let total_latency: u64 = self.threads.values()
            .map(|t| t.total_wakeup_latency_ns)
            .sum();
        if self.stats.total_wakeups > 0 {
            self.stats.avg_wakeup_latency_ns =
                total_latency as f64 / self.stats.total_wakeups as f64;
        }
        self.stats.wakeup_storm_threads = self.threads.values()
            .filter(|t| t.is_wakeup_storm())
            .count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppWakeupProfilerStats {
        &self.stats
    }

    /// Most frequent wakeup edges
    #[inline]
    pub fn top_edges(&self, n: usize) -> Vec<&WakeupEdge> {
        let mut edges: Vec<&WakeupEdge> = self.edges.values().collect();
        edges.sort_by(|a, b| b.count.cmp(&a.count));
        edges.truncate(n);
        edges
    }

    /// Threads with wakeup storms
    #[inline]
    pub fn storm_threads(&self) -> Vec<u64> {
        self.threads.iter()
            .filter(|(_, t)| t.is_wakeup_storm())
            .map(|(&tid, _)| tid)
            .collect()
    }

    /// Build critical wakeup chain from a thread
    pub fn build_chain(&self, start_tid: u64, max_depth: usize) -> WakeupChain {
        let mut chain = Vec::new();
        let mut current = start_tid;
        let mut total_lat = 0u64;

        for _ in 0..max_depth {
            chain.push(current);
            // Find the most frequent waker of current
            let next = self.edges.iter()
                .filter(|((_, wakee), _)| *wakee == current)
                .max_by_key(|(_, e)| e.count)
                .map(|((waker, _), e)| {
                    total_lat += e.total_latency_ns / e.count.max(1);
                    *waker
                });
            match next {
                Some(w) if !chain.contains(&w) => current = w,
                _ => break,
            }
        }
        chain.reverse();
        WakeupChain { chain, total_latency_ns: total_lat }
    }
}

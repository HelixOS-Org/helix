//! # Bridge Syscall Profiler V2
//!
//! Advanced per-syscall profiling with detailed metrics:
//! - Per-syscall latency histograms
//! - Syscall argument pattern analysis
//! - Error return tracking by errno
//! - Syscall pair correlation (sequences)
//! - Caller-site attribution
//! - Hot path detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Latency bucket
#[derive(Debug, Clone)]
pub struct LatencyBucketBridge {
    pub min_ns: u64,
    pub max_ns: u64,
    pub count: u64,
}

/// Latency histogram
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    buckets: Vec<LatencyBucketBridge>,
    total_samples: u64,
    sum_ns: u64,
    min_ns: u64,
    max_ns: u64,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        let boundaries = [
            100, 500, 1_000, 5_000, 10_000, 50_000, 100_000,
            500_000, 1_000_000, 5_000_000, 10_000_000, 50_000_000, u64::MAX,
        ];
        let mut buckets = Vec::new();
        let mut prev = 0u64;
        for &b in &boundaries {
            buckets.push(LatencyBucketBridge { min_ns: prev, max_ns: b, count: 0 });
            prev = b;
        }
        Self { buckets, total_samples: 0, sum_ns: 0, min_ns: u64::MAX, max_ns: 0 }
    }

    pub fn record(&mut self, ns: u64) {
        self.total_samples += 1;
        self.sum_ns += ns;
        if ns < self.min_ns { self.min_ns = ns; }
        if ns > self.max_ns { self.max_ns = ns; }
        for bucket in &mut self.buckets {
            if ns >= bucket.min_ns && ns < bucket.max_ns {
                bucket.count += 1;
                return;
            }
        }
    }

    pub fn avg_ns(&self) -> u64 {
        if self.total_samples == 0 { return 0; }
        self.sum_ns / self.total_samples
    }

    pub fn percentile(&self, pct: f64) -> u64 {
        let target = (self.total_samples as f64 * pct / 100.0) as u64;
        let mut cumulative = 0u64;
        for bucket in &self.buckets {
            cumulative += bucket.count;
            if cumulative >= target {
                return bucket.max_ns;
            }
        }
        self.max_ns
    }

    pub fn p50(&self) -> u64 { self.percentile(50.0) }
    pub fn p95(&self) -> u64 { self.percentile(95.0) }
    pub fn p99(&self) -> u64 { self.percentile(99.0) }
}

/// Error tracking by errno
#[derive(Debug, Clone)]
pub struct ErrnoTracker {
    pub errors: BTreeMap<i32, u64>, // errno → count
    pub total_errors: u64,
    pub total_calls: u64,
}

impl ErrnoTracker {
    pub fn new() -> Self {
        Self { errors: BTreeMap::new(), total_errors: 0, total_calls: 0 }
    }

    pub fn record_success(&mut self) {
        self.total_calls += 1;
    }

    pub fn record_error(&mut self, errno: i32) {
        self.total_calls += 1;
        self.total_errors += 1;
        *self.errors.entry(errno).or_insert(0) += 1;
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_calls == 0 { return 0.0; }
        self.total_errors as f64 / self.total_calls as f64
    }

    pub fn top_error(&self) -> Option<(i32, u64)> {
        self.errors.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&errno, &count)| (errno, count))
    }
}

/// Syscall pair (for sequence analysis)
#[derive(Debug, Clone)]
pub struct SyscallPair {
    pub first_nr: u32,
    pub second_nr: u32,
    pub count: u64,
    pub avg_gap_ns: u64,
}

/// Per-syscall profile
#[derive(Debug, Clone)]
pub struct SyscallProfileV2 {
    pub syscall_nr: u32,
    pub call_count: u64,
    pub latency: LatencyHistogram,
    pub errors: ErrnoTracker,
    pub last_call_ns: u64,
    pub caller_sites: BTreeMap<u64, u64>, // return_addr → count
}

impl SyscallProfileV2 {
    pub fn new(nr: u32) -> Self {
        Self {
            syscall_nr: nr,
            call_count: 0,
            latency: LatencyHistogram::new(),
            errors: ErrnoTracker::new(),
            last_call_ns: 0,
            caller_sites: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, latency_ns: u64, result: i64, caller_addr: u64, now: u64) {
        self.call_count += 1;
        self.latency.record(latency_ns);
        self.last_call_ns = now;

        if result < 0 {
            self.errors.record_error((-result) as i32);
        } else {
            self.errors.record_success();
        }

        *self.caller_sites.entry(caller_addr).or_insert(0) += 1;
    }

    pub fn top_caller(&self) -> Option<(u64, u64)> {
        self.caller_sites.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&addr, &count)| (addr, count))
    }
}

/// Bridge Syscall Profiler V2 stats
#[derive(Debug, Clone, Default)]
pub struct BridgeSyscallProfilerV2Stats {
    pub total_syscalls_tracked: usize,
    pub total_calls: u64,
    pub total_errors: u64,
    pub global_avg_latency_ns: u64,
    pub hottest_syscall_nr: u32,
    pub hottest_syscall_count: u64,
}

/// Bridge Syscall Profiler V2
pub struct BridgeSyscallProfilerV2 {
    profiles: BTreeMap<u32, SyscallProfileV2>,
    pairs: Vec<SyscallPair>,
    last_syscall_per_task: BTreeMap<u64, (u32, u64)>, // task_id → (nr, timestamp)
    stats: BridgeSyscallProfilerV2Stats,
}

impl BridgeSyscallProfilerV2 {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            pairs: Vec::new(),
            last_syscall_per_task: BTreeMap::new(),
            stats: BridgeSyscallProfilerV2Stats::default(),
        }
    }

    pub fn record(&mut self, task_id: u64, syscall_nr: u32, latency_ns: u64, result: i64, caller: u64, now: u64) {
        let profile = self.profiles.entry(syscall_nr)
            .or_insert_with(|| SyscallProfileV2::new(syscall_nr));
        profile.record(latency_ns, result, caller, now);

        // Track pairs
        if let Some(&(prev_nr, prev_ts)) = self.last_syscall_per_task.get(&task_id) {
            let gap = now.saturating_sub(prev_ts);
            let found = self.pairs.iter_mut()
                .find(|p| p.first_nr == prev_nr && p.second_nr == syscall_nr);
            if let Some(pair) = found {
                let n = pair.count;
                pair.avg_gap_ns = (pair.avg_gap_ns * n + gap) / (n + 1);
                pair.count += 1;
            } else {
                self.pairs.push(SyscallPair {
                    first_nr: prev_nr,
                    second_nr: syscall_nr,
                    count: 1,
                    avg_gap_ns: gap,
                });
            }
        }
        self.last_syscall_per_task.insert(task_id, (syscall_nr, now));
    }

    /// Find hot pairs (common sequences)
    pub fn hot_pairs(&self, min_count: u64) -> Vec<&SyscallPair> {
        self.pairs.iter().filter(|p| p.count >= min_count).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_syscalls_tracked = self.profiles.len();
        self.stats.total_calls = self.profiles.values().map(|p| p.call_count).sum();
        self.stats.total_errors = self.profiles.values().map(|p| p.errors.total_errors).sum();

        let sum_lat: u64 = self.profiles.values().map(|p| p.latency.sum_ns).sum();
        self.stats.global_avg_latency_ns = if self.stats.total_calls > 0 {
            sum_lat / self.stats.total_calls
        } else { 0 };

        if let Some((nr, profile)) = self.profiles.iter().max_by_key(|(_, p)| p.call_count) {
            self.stats.hottest_syscall_nr = *nr;
            self.stats.hottest_syscall_count = profile.call_count;
        }
    }

    pub fn profile(&self, nr: u32) -> Option<&SyscallProfileV2> { self.profiles.get(&nr) }
    pub fn stats(&self) -> &BridgeSyscallProfilerV2Stats { &self.stats }
}

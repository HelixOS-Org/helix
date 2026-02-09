//! # Application Signal Profiler
//!
//! Per-process signal delivery and handling analysis:
//! - Signal delivery latency tracking
//! - Signal frequency per type
//! - Handler execution time measurement
//! - Signal queue overflow detection
//! - Signal coalescing analysis
//! - SA_RESTART behavior correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Signal category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalCategoryApps {
    Terminal,    // SIGTERM, SIGKILL, SIGINT
    Fault,       // SIGSEGV, SIGBUS, SIGFPE
    Io,          // SIGIO, SIGPOLL, SIGURG
    Timer,       // SIGALRM, SIGVTALRM, SIGPROF
    Child,       // SIGCHLD
    User,        // SIGUSR1, SIGUSR2
    Control,     // SIGSTOP, SIGCONT, SIGTSTP
    RealTime,    // SIGRTMIN..SIGRTMAX
}

/// Signal delivery state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDeliveryState {
    Queued,
    Delivered,
    Ignored,
    Blocked,
    Coalesced,
    QueueOverflow,
}

/// Per-signal-number stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SignalNumStats {
    pub signum: u32,
    pub category: SignalCategoryApps,
    pub generated: u64,
    pub delivered: u64,
    pub ignored: u64,
    pub blocked: u64,
    pub coalesced: u64,
    pub overflows: u64,
    pub total_delivery_ns: u64,
    pub max_delivery_ns: u64,
    pub total_handler_ns: u64,
    pub max_handler_ns: u64,
    pub restart_count: u64,
}

impl SignalNumStats {
    pub fn new(signum: u32, category: SignalCategoryApps) -> Self {
        Self {
            signum,
            category,
            generated: 0,
            delivered: 0,
            ignored: 0,
            blocked: 0,
            coalesced: 0,
            overflows: 0,
            total_delivery_ns: 0,
            max_delivery_ns: 0,
            total_handler_ns: 0,
            max_handler_ns: 0,
            restart_count: 0,
        }
    }

    #[inline(always)]
    pub fn delivery_ratio(&self) -> f64 {
        if self.generated == 0 { return 0.0; }
        self.delivered as f64 / self.generated as f64
    }

    #[inline(always)]
    pub fn avg_delivery_ns(&self) -> u64 {
        if self.delivered == 0 { return 0; }
        self.total_delivery_ns / self.delivered
    }

    #[inline(always)]
    pub fn avg_handler_ns(&self) -> u64 {
        if self.delivered == 0 { return 0; }
        self.total_handler_ns / self.delivered
    }

    #[inline(always)]
    pub fn coalesce_ratio(&self) -> f64 {
        if self.generated == 0 { return 0.0; }
        self.coalesced as f64 / self.generated as f64
    }

    #[inline(always)]
    pub fn record_generate(&mut self) {
        self.generated += 1;
    }

    #[inline]
    pub fn record_delivery(&mut self, delivery_ns: u64, handler_ns: u64) {
        self.delivered += 1;
        self.total_delivery_ns += delivery_ns;
        if delivery_ns > self.max_delivery_ns { self.max_delivery_ns = delivery_ns; }
        self.total_handler_ns += handler_ns;
        if handler_ns > self.max_handler_ns { self.max_handler_ns = handler_ns; }
    }
}

/// Per-process signal profile
#[derive(Debug, Clone)]
pub struct ProcessSignalProfile {
    pub pid: u64,
    pub signal_stats: BTreeMap<u32, SignalNumStats>,
    pub total_generated: u64,
    pub total_delivered: u64,
    pub total_ignored: u64,
    pub total_blocked: u64,
    pub total_coalesced: u64,
    pub total_overflows: u64,
    pub pending_mask: u64,
    pub blocked_mask: u64,
}

impl ProcessSignalProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            signal_stats: BTreeMap::new(),
            total_generated: 0,
            total_delivered: 0,
            total_ignored: 0,
            total_blocked: 0,
            total_coalesced: 0,
            total_overflows: 0,
            pending_mask: 0,
            blocked_mask: 0,
        }
    }

    fn get_or_create(&mut self, signum: u32) -> &mut SignalNumStats {
        let cat = Self::categorize(signum);
        self.signal_stats.entry(signum)
            .or_insert_with(|| SignalNumStats::new(signum, cat))
    }

    fn categorize(signum: u32) -> SignalCategoryApps {
        match signum {
            1 | 2 | 3 | 9 | 15 => SignalCategoryApps::Terminal,
            4 | 7 | 8 | 11 => SignalCategoryApps::Fault,
            23 | 29 => SignalCategoryApps::Io,
            14 | 26 | 27 => SignalCategoryApps::Timer,
            17 | 20 => SignalCategoryApps::Child,
            10 | 12 => SignalCategoryApps::User,
            18 | 19 | 25 => SignalCategoryApps::Control,
            34..=64 => SignalCategoryApps::RealTime,
            _ => SignalCategoryApps::User,
        }
    }

    #[inline(always)]
    pub fn record_generate(&mut self, signum: u32) {
        self.get_or_create(signum).record_generate();
        self.total_generated += 1;
    }

    #[inline(always)]
    pub fn record_delivery(&mut self, signum: u32, delivery_ns: u64, handler_ns: u64) {
        self.get_or_create(signum).record_delivery(delivery_ns, handler_ns);
        self.total_delivered += 1;
    }

    #[inline(always)]
    pub fn record_coalesce(&mut self, signum: u32) {
        self.get_or_create(signum).coalesced += 1;
        self.total_coalesced += 1;
    }

    #[inline(always)]
    pub fn record_overflow(&mut self, signum: u32) {
        self.get_or_create(signum).overflows += 1;
        self.total_overflows += 1;
    }

    #[inline(always)]
    pub fn record_blocked(&mut self, signum: u32) {
        self.get_or_create(signum).blocked += 1;
        self.total_blocked += 1;
    }

    #[inline(always)]
    pub fn record_ignored(&mut self, signum: u32) {
        self.get_or_create(signum).ignored += 1;
        self.total_ignored += 1;
    }

    #[inline]
    pub fn most_frequent_signal(&self) -> Option<u32> {
        self.signal_stats.values()
            .max_by_key(|s| s.generated)
            .map(|s| s.signum)
    }

    #[inline]
    pub fn signals_by_category(&self, cat: SignalCategoryApps) -> Vec<&SignalNumStats> {
        self.signal_stats.values()
            .filter(|s| s.category == cat)
            .collect()
    }
}

/// App signal profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppSignalProfilerStats {
    pub total_processes: usize,
    pub total_generated: u64,
    pub total_delivered: u64,
    pub total_coalesced: u64,
    pub total_overflows: u64,
    pub unique_signals: usize,
}

/// Application Signal Profiler
pub struct AppSignalProfiler {
    profiles: BTreeMap<u64, ProcessSignalProfile>,
    stats: AppSignalProfilerStats,
}

impl AppSignalProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppSignalProfilerStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessSignalProfile::new(pid));
    }

    #[inline]
    pub fn record_generate(&mut self, pid: u64, signum: u32) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_generate(signum);
        }
    }

    #[inline]
    pub fn record_delivery(&mut self, pid: u64, signum: u32, delivery_ns: u64, handler_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_delivery(signum, delivery_ns, handler_ns);
        }
    }

    #[inline]
    pub fn record_coalesce(&mut self, pid: u64, signum: u32) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_coalesce(signum);
        }
    }

    #[inline]
    pub fn record_overflow(&mut self, pid: u64, signum: u32) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_overflow(signum);
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_generated = self.profiles.values().map(|p| p.total_generated).sum();
        self.stats.total_delivered = self.profiles.values().map(|p| p.total_delivered).sum();
        self.stats.total_coalesced = self.profiles.values().map(|p| p.total_coalesced).sum();
        self.stats.total_overflows = self.profiles.values().map(|p| p.total_overflows).sum();

        let mut sigs = alloc::collections::BTreeSet::new();
        for prof in self.profiles.values() {
            for key in prof.signal_stats.keys() {
                sigs.insert(*key);
            }
        }
        self.stats.unique_signals = sigs.len();
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessSignalProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppSignalProfilerStats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}

//! # Holistic CFS Tuner
//!
//! Completely Fair Scheduler holistic tuning:
//! - Dynamic CFS parameter tuning based on global state
//! - latency_ns, min_granularity, wakeup_granularity adjustment
//! - Per-CPU runqueue imbalance detection
//! - Vruntime drift correction
//! - Scheduling latency histogram
//! - Group scheduling weight optimization

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// CFS tunable parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfsTunable {
    /// sched_latency_ns
    LatencyNs,
    /// sched_min_granularity_ns
    MinGranularity,
    /// sched_wakeup_granularity_ns
    WakeupGranularity,
    /// sched_migration_cost_ns
    MigrationCost,
    /// sched_nr_migrate
    NrMigrate,
}

/// Tuning direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuningDirection {
    /// Increase value
    Increase,
    /// Decrease value
    Decrease,
    /// Keep current
    Hold,
}

/// Per-CPU CFS stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuCfsStats {
    pub cpu_id: u32,
    pub nr_running: u32,
    pub nr_switches: u64,
    pub min_vruntime: u64,
    pub max_vruntime: u64,
    pub total_weight: u64,
    pub avg_latency_ns: f64,
    pub avg_wait_ns: f64,
    pub load_avg: f64,
}

impl CpuCfsStats {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            nr_running: 0,
            nr_switches: 0,
            min_vruntime: 0,
            max_vruntime: 0,
            total_weight: 0,
            avg_latency_ns: 0.0,
            avg_wait_ns: 0.0,
            load_avg: 0.0,
        }
    }

    /// Vruntime spread (indicator of fairness)
    #[inline(always)]
    pub fn vruntime_spread(&self) -> u64 {
        self.max_vruntime.saturating_sub(self.min_vruntime)
    }

    /// Is this CPU overloaded?
    #[inline(always)]
    pub fn is_overloaded(&self) -> bool {
        self.nr_running > 4 && self.load_avg > 1.5
    }

    /// Is this CPU idle?
    #[inline(always)]
    pub fn is_idle(&self) -> bool {
        self.nr_running == 0
    }
}

/// CFS parameter set
#[derive(Debug, Clone)]
pub struct CfsParameters {
    pub latency_ns: u64,
    pub min_granularity_ns: u64,
    pub wakeup_granularity_ns: u64,
    pub migration_cost_ns: u64,
    pub nr_migrate: u32,
}

impl CfsParameters {
    #[inline]
    pub fn default_params() -> Self {
        Self {
            latency_ns: 6_000_000,            // 6ms
            min_granularity_ns: 750_000,      // 750us
            wakeup_granularity_ns: 1_000_000, // 1ms
            migration_cost_ns: 500_000,       // 500us
            nr_migrate: 32,
        }
    }
}

/// Tuning recommendation
#[derive(Debug, Clone)]
pub struct TuningRecommendation {
    pub tunable: CfsTunable,
    pub direction: TuningDirection,
    pub current_value: u64,
    pub recommended_value: u64,
    pub confidence: f64,
    pub reason: u32,
}

/// Scheduling latency histogram
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    /// Buckets: [0-1us, 1-10us, 10-100us, 100us-1ms, 1-10ms, 10-100ms, >100ms]
    pub buckets: [u64; 7],
    pub total_samples: u64,
    pub sum_ns: u64,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        Self {
            buckets: [0; 7],
            total_samples: 0,
            sum_ns: 0,
        }
    }

    pub fn record(&mut self, latency_ns: u64) {
        self.total_samples += 1;
        self.sum_ns += latency_ns;
        let bucket = if latency_ns < 1_000 {
            0
        } else if latency_ns < 10_000 {
            1
        } else if latency_ns < 100_000 {
            2
        } else if latency_ns < 1_000_000 {
            3
        } else if latency_ns < 10_000_000 {
            4
        } else if latency_ns < 100_000_000 {
            5
        } else {
            6
        };
        self.buckets[bucket] += 1;
    }

    #[inline]
    pub fn avg_ns(&self) -> f64 {
        if self.total_samples == 0 {
            0.0
        } else {
            self.sum_ns as f64 / self.total_samples as f64
        }
    }

    /// P95 estimate (from histogram)
    pub fn p95_bucket_idx(&self) -> usize {
        if self.total_samples == 0 {
            return 0;
        }
        let target = self.total_samples * 95 / 100;
        let mut cum = 0;
        for (i, &count) in self.buckets.iter().enumerate() {
            cum += count;
            if cum >= target {
                return i;
            }
        }
        6
    }
}

/// CFS tuner stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticCfsTunerStats {
    pub tracked_cpus: usize,
    pub overloaded_cpus: usize,
    pub idle_cpus: usize,
    pub avg_vruntime_spread: f64,
    pub avg_scheduling_latency_ns: f64,
    pub load_imbalance: f64,
    pub pending_recommendations: usize,
}

/// Holistic CFS Tuner
pub struct HolisticCfsTuner {
    cpus: BTreeMap<u32, CpuCfsStats>,
    params: CfsParameters,
    latency_hist: LatencyHistogram,
    recommendations: Vec<TuningRecommendation>,
    stats: HolisticCfsTunerStats,
}

impl HolisticCfsTuner {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            params: CfsParameters::default_params(),
            latency_hist: LatencyHistogram::new(),
            recommendations: Vec::new(),
            stats: HolisticCfsTunerStats::default(),
        }
    }

    #[inline(always)]
    pub fn update_cpu(&mut self, stats: CpuCfsStats) {
        self.cpus.insert(stats.cpu_id, stats);
        self.recompute();
    }

    #[inline(always)]
    pub fn record_latency(&mut self, latency_ns: u64) {
        self.latency_hist.record(latency_ns);
    }

    fn recompute(&mut self) {
        self.recommendations.clear();
        if self.cpus.is_empty() {
            return;
        }

        // Compute load imbalance
        let avg_load: f64 =
            self.cpus.values().map(|c| c.load_avg).sum::<f64>() / self.cpus.len() as f64;
        let max_load = self
            .cpus
            .values()
            .map(|c| c.load_avg)
            .fold(0.0f64, |a, b| if b > a { b } else { a });
        let min_load = self
            .cpus
            .values()
            .map(|c| c.load_avg)
            .fold(f64::MAX, |a, b| if b < a { b } else { a });
        let imbalance = if avg_load > 0.01 {
            (max_load - min_load) / avg_load
        } else {
            0.0
        };

        // Avg vruntime spread
        let avg_spread: f64 = self
            .cpus
            .values()
            .map(|c| c.vruntime_spread() as f64)
            .sum::<f64>()
            / self.cpus.len() as f64;

        // Latency analysis
        let avg_lat = self.latency_hist.avg_ns();
        let p95_idx = self.latency_hist.p95_bucket_idx();

        // Generate recommendations
        if avg_lat > 10_000_000.0 {
            // High latency → reduce sched_latency
            self.recommendations.push(TuningRecommendation {
                tunable: CfsTunable::LatencyNs,
                direction: TuningDirection::Decrease,
                current_value: self.params.latency_ns,
                recommended_value: (self.params.latency_ns * 3 / 4).max(1_000_000),
                confidence: 0.8,
                reason: 1,
            });
        }

        if imbalance > 0.5 {
            // High imbalance → reduce migration cost
            self.recommendations.push(TuningRecommendation {
                tunable: CfsTunable::MigrationCost,
                direction: TuningDirection::Decrease,
                current_value: self.params.migration_cost_ns,
                recommended_value: (self.params.migration_cost_ns / 2).max(100_000),
                confidence: 0.7,
                reason: 2,
            });
        }

        if avg_spread > 50_000_000 {
            // High vruntime spread → increase granularity
            self.recommendations.push(TuningRecommendation {
                tunable: CfsTunable::MinGranularity,
                direction: TuningDirection::Increase,
                current_value: self.params.min_granularity_ns,
                recommended_value: (self.params.min_granularity_ns * 3 / 2).min(4_000_000),
                confidence: 0.6,
                reason: 3,
            });
        }

        self.stats.tracked_cpus = self.cpus.len();
        self.stats.overloaded_cpus = self.cpus.values().filter(|c| c.is_overloaded()).count();
        self.stats.idle_cpus = self.cpus.values().filter(|c| c.is_idle()).count();
        self.stats.avg_vruntime_spread = avg_spread;
        self.stats.avg_scheduling_latency_ns = avg_lat;
        self.stats.load_imbalance = imbalance;
        self.stats.pending_recommendations = self.recommendations.len();
    }

    pub fn apply_recommendation(&mut self, idx: usize) -> bool {
        if idx >= self.recommendations.len() {
            return false;
        }
        let rec = &self.recommendations[idx];
        match rec.tunable {
            CfsTunable::LatencyNs => self.params.latency_ns = rec.recommended_value,
            CfsTunable::MinGranularity => self.params.min_granularity_ns = rec.recommended_value,
            CfsTunable::WakeupGranularity => {
                self.params.wakeup_granularity_ns = rec.recommended_value
            },
            CfsTunable::MigrationCost => self.params.migration_cost_ns = rec.recommended_value,
            CfsTunable::NrMigrate => self.params.nr_migrate = rec.recommended_value as u32,
        }
        true
    }

    #[inline(always)]
    pub fn params(&self) -> &CfsParameters {
        &self.params
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticCfsTunerStats {
        &self.stats
    }

    #[inline(always)]
    pub fn recommendations(&self) -> &[TuningRecommendation] {
        &self.recommendations
    }
}

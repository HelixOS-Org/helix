//! # Application Energy Profiling
//!
//! Per-application energy consumption analysis:
//! - CPU energy attribution
//! - Wake-up frequency tracking
//! - Idle efficiency scoring
//! - Power state recommendations
//! - Energy budget management
//! - Battery impact estimation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// ENERGY COMPONENTS
// ============================================================================

/// Energy component
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnergyComponent {
    /// CPU computation
    Cpu,
    /// Memory accesses
    Memory,
    /// Disk I/O
    DiskIo,
    /// Network I/O
    NetworkIo,
    /// GPU usage
    Gpu,
    /// Wakeup overhead
    Wakeup,
    /// Timer overhead
    Timer,
    /// Interrupt handling
    Interrupt,
}

/// Energy unit (micro-joules)
pub type MicroJoules = u64;

/// Power unit (milliwatts)
pub type MilliWatts = u32;

// ============================================================================
// ENERGY SAMPLE
// ============================================================================

/// Energy sample for a process
#[derive(Debug, Clone)]
pub struct EnergySample {
    /// Process ID
    pub pid: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Per-component energy (micro-joules since last sample)
    pub components: BTreeMap<u8, MicroJoules>,
    /// Total energy
    pub total_uj: MicroJoules,
    /// Duration of sample period (ms)
    pub period_ms: u64,
}

impl EnergySample {
    /// Average power (milliwatts)
    #[inline]
    pub fn avg_power_mw(&self) -> MilliWatts {
        if self.period_ms == 0 {
            return 0;
        }
        // P = E / t
        (self.total_uj * 1000 / self.period_ms) as MilliWatts
    }
}

// ============================================================================
// WAKEUP TRACKING
// ============================================================================

/// Wakeup reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WakeupReason {
    /// Timer expired
    Timer,
    /// I/O ready
    IoReady,
    /// Signal received
    Signal,
    /// IPC message
    Ipc,
    /// Scheduler decision
    Scheduler,
    /// User input
    UserInput,
    /// Unknown
    Unknown,
}

/// Wakeup event
#[derive(Debug, Clone)]
pub struct WakeupEvent {
    /// Process ID
    pub pid: u64,
    /// Reason
    pub reason: WakeupReason,
    /// Timestamp
    pub timestamp: u64,
    /// Time spent idle before wakeup (us)
    pub idle_before_us: u64,
    /// Time spent active after wakeup (us)
    pub active_after_us: u64,
}

/// Wakeup statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct WakeupStats {
    /// Total wakeups
    pub total: u64,
    /// By reason
    pub by_reason: BTreeMap<u8, u64>,
    /// Average wakeups per second
    pub wakeups_per_sec: f64,
    /// Average idle duration (us)
    pub avg_idle_us: u64,
    /// Average active duration (us)
    pub avg_active_us: u64,
    /// Unnecessary wakeups (woke up but did nothing)
    pub spurious: u64,
}

// ============================================================================
// ENERGY PROFILE
// ============================================================================

/// Energy efficiency rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnergyRating {
    /// Excellent - minimal energy use
    Excellent = 0,
    /// Good
    Good      = 1,
    /// Average
    Average   = 2,
    /// Poor
    Poor      = 3,
    /// Very poor - energy hog
    VeryPoor  = 4,
}

/// Per-process energy profile
#[derive(Debug, Clone)]
pub struct ProcessEnergyProfile {
    /// Process ID
    pub pid: u64,
    /// Overall rating
    pub rating: EnergyRating,
    /// Average power (mW)
    pub avg_power_mw: MilliWatts,
    /// Peak power (mW)
    pub peak_power_mw: MilliWatts,
    /// Total energy consumed (uJ)
    pub total_energy_uj: MicroJoules,
    /// Per-component breakdown (percent)
    pub component_pct: BTreeMap<u8, u32>,
    /// Wakeup stats
    pub wakeup_stats: WakeupStats,
    /// Idle efficiency (percent of time truly idle)
    pub idle_efficiency: f64,
    /// Estimated battery impact (percent per hour)
    pub battery_impact_pct_per_hour: f64,
}

/// Energy recommendation
#[derive(Debug, Clone)]
pub struct EnergyRecommendation {
    /// Process ID
    pub pid: u64,
    /// Recommendation type
    pub rec_type: EnergyRecType,
    /// Expected savings (mW)
    pub expected_savings_mw: MilliWatts,
    /// Priority
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyRecType {
    /// Reduce timer frequency
    ReduceTimerFreq,
    /// Coalesce wakeups
    CoalesceWakeups,
    /// Use epoll instead of poll/select
    UseEpoll,
    /// Batch I/O operations
    BatchIo,
    /// Reduce polling
    ReducePolling,
    /// Allow deeper C-states
    DeeperCstates,
    /// Reduce CPU frequency
    LowerFrequency,
    /// Defer work to batch
    DeferWork,
}

// ============================================================================
// ENERGY BUDGET
// ============================================================================

/// Energy budget for a process
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Process ID
    pub pid: u64,
    /// Budget (mW)
    pub budget_mw: MilliWatts,
    /// Current usage (mW)
    pub current_mw: MilliWatts,
    /// Over budget
    pub over_budget: bool,
    /// Grace period remaining (ms)
    pub grace_ms: u64,
}

// ============================================================================
// ENERGY ANALYZER
// ============================================================================

/// Application energy analyzer
pub struct AppEnergyAnalyzer {
    /// Per-process energy history
    samples: BTreeMap<u64, Vec<EnergySample>>,
    /// Per-process wakeup events
    wakeups: BTreeMap<u64, Vec<WakeupEvent>>,
    /// Profiles
    profiles: BTreeMap<u64, ProcessEnergyProfile>,
    /// Budgets
    budgets: BTreeMap<u64, EnergyBudget>,
    /// Max samples per process
    max_samples: usize,
    /// Max wakeup events per process
    max_wakeups: usize,
    /// Battery capacity (mWh, 0 = AC power)
    pub battery_capacity_mwh: u64,
    /// Total processes tracked
    pub tracked_processes: usize,
}

impl AppEnergyAnalyzer {
    pub fn new(battery_capacity_mwh: u64) -> Self {
        Self {
            samples: BTreeMap::new(),
            wakeups: BTreeMap::new(),
            profiles: BTreeMap::new(),
            budgets: BTreeMap::new(),
            max_samples: 1440,
            max_wakeups: 500,
            battery_capacity_mwh,
            tracked_processes: 0,
        }
    }

    /// Record energy sample
    pub fn record_sample(&mut self, sample: EnergySample) {
        let pid = sample.pid;
        let history = self.samples.entry(pid).or_insert_with(Vec::new);
        history.push(sample);
        if history.len() > self.max_samples {
            history.pop_front();
        }

        if !self.profiles.contains_key(&pid) {
            self.tracked_processes += 1;
        }
    }

    /// Record wakeup event
    #[inline]
    pub fn record_wakeup(&mut self, event: WakeupEvent) {
        let pid = event.pid;
        let events = self.wakeups.entry(pid).or_insert_with(Vec::new);
        events.push(event);
        if events.len() > self.max_wakeups {
            events.pop_front();
        }
    }

    /// Analyze process energy
    pub fn analyze(&mut self, pid: u64) -> Option<&ProcessEnergyProfile> {
        let samples = self.samples.get(&pid)?;
        if samples.is_empty() {
            return None;
        }

        let total_energy: u64 = samples.iter().map(|s| s.total_uj).sum();
        let total_period: u64 = samples.iter().map(|s| s.period_ms).sum();

        let avg_power = if total_period > 0 {
            (total_energy * 1000 / total_period) as MilliWatts
        } else {
            0
        };

        let peak_power = samples.iter().map(|s| s.avg_power_mw()).max().unwrap_or(0);

        // Component breakdown
        let mut component_total: BTreeMap<u8, u64> = BTreeMap::new();
        for sample in samples {
            for (&comp, &energy) in &sample.components {
                *component_total.entry(comp).or_insert(0) += energy;
            }
        }

        let mut component_pct = BTreeMap::new();
        if total_energy > 0 {
            for (&comp, &energy) in &component_total {
                component_pct.insert(comp, (energy * 100 / total_energy) as u32);
            }
        }

        // Wakeup stats
        let wakeup_stats = self.compute_wakeup_stats(pid, total_period);

        // Rating
        let rating = if avg_power < 10 {
            EnergyRating::Excellent
        } else if avg_power < 100 {
            EnergyRating::Good
        } else if avg_power < 500 {
            EnergyRating::Average
        } else if avg_power < 2000 {
            EnergyRating::Poor
        } else {
            EnergyRating::VeryPoor
        };

        // Idle efficiency
        let idle_efficiency = if let Some(events) = self.wakeups.get(&pid) {
            let total_active: u64 = events.iter().map(|e| e.active_after_us).sum();
            let total_time = total_period * 1000; // Convert to us
            if total_time > 0 {
                1.0 - (total_active as f64 / total_time as f64)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Battery impact
        let battery_impact = if self.battery_capacity_mwh > 0 {
            avg_power as f64 / self.battery_capacity_mwh as f64 * 100.0
        } else {
            0.0
        };

        self.profiles.insert(pid, ProcessEnergyProfile {
            pid,
            rating,
            avg_power_mw: avg_power,
            peak_power_mw: peak_power,
            total_energy_uj: total_energy,
            component_pct,
            wakeup_stats,
            idle_efficiency,
            battery_impact_pct_per_hour: battery_impact,
        });

        self.profiles.get(&pid)
    }

    /// Compute wakeup stats for process
    fn compute_wakeup_stats(&self, pid: u64, total_period_ms: u64) -> WakeupStats {
        let events = match self.wakeups.get(&pid) {
            Some(e) => e,
            None => return WakeupStats::default(),
        };

        let mut stats = WakeupStats::default();
        stats.total = events.len() as u64;

        for event in events {
            *stats.by_reason.entry(event.reason as u8).or_insert(0) += 1;
            stats.avg_idle_us += event.idle_before_us;
            stats.avg_active_us += event.active_after_us;

            if event.active_after_us < 10 {
                stats.spurious += 1;
            }
        }

        if !events.is_empty() {
            stats.avg_idle_us /= events.len() as u64;
            stats.avg_active_us /= events.len() as u64;
        }

        if total_period_ms > 0 {
            stats.wakeups_per_sec = events.len() as f64 / (total_period_ms as f64 / 1000.0);
        }

        stats
    }

    /// Set energy budget
    #[inline]
    pub fn set_budget(&mut self, pid: u64, budget_mw: MilliWatts) {
        self.budgets.insert(pid, EnergyBudget {
            pid,
            budget_mw,
            current_mw: self.profiles.get(&pid).map(|p| p.avg_power_mw).unwrap_or(0),
            over_budget: false,
            grace_ms: 5000,
        });
    }

    /// Get recommendations for process
    pub fn recommendations(&self, pid: u64) -> Vec<EnergyRecommendation> {
        let mut recs = Vec::new();
        let profile = match self.profiles.get(&pid) {
            Some(p) => p,
            None => return recs,
        };

        if profile.wakeup_stats.wakeups_per_sec > 100.0 {
            recs.push(EnergyRecommendation {
                pid,
                rec_type: EnergyRecType::CoalesceWakeups,
                expected_savings_mw: (profile.avg_power_mw / 10).max(1),
                priority: 1,
            });
        }

        if profile.wakeup_stats.spurious > profile.wakeup_stats.total / 4 {
            recs.push(EnergyRecommendation {
                pid,
                rec_type: EnergyRecType::ReducePolling,
                expected_savings_mw: (profile.avg_power_mw / 5).max(1),
                priority: 2,
            });
        }

        if profile.idle_efficiency < 0.5 {
            recs.push(EnergyRecommendation {
                pid,
                rec_type: EnergyRecType::DeeperCstates,
                expected_savings_mw: (profile.avg_power_mw / 4).max(1),
                priority: 3,
            });
        }

        recs
    }

    /// Get profile
    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessEnergyProfile> {
        self.profiles.get(&pid)
    }

    /// Unregister
    #[inline]
    pub fn unregister(&mut self, pid: u64) {
        self.samples.remove(&pid);
        self.wakeups.remove(&pid);
        self.profiles.remove(&pid);
        self.budgets.remove(&pid);
        self.tracked_processes = self.tracked_processes.saturating_sub(1);
    }
}

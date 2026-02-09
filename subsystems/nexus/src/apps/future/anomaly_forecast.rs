// SPDX-License-Identifier: GPL-2.0
//! # Apps Anomaly Forecast Engine
//!
//! Predict application anomalies *before* they happen. By tracking behavioral
//! baselines and detecting precursor patterns — rising fault rates, allocation
//! runaway, syscall frequency spikes — the engine issues early warnings for
//! impending crashes, hangs, and memory thrashing.
//!
//! Each warning carries a lead time (how far in advance it was issued),
//! confidence, and a suggested prevention action. After the fact, warnings
//! are validated against actual events so the engine learns which precursors
//! are truly predictive.
//!
//! This is the apps engine seeing disasters before they arrive.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_APPS: usize = 256;
const MAX_WARNINGS: usize = 512;
const MAX_PRECURSORS: usize = 64;
const BASELINE_WINDOW: usize = 128;
const ANOMALY_SIGMA_THRESHOLD: f64 = 2.5;
const EMA_ALPHA: f64 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xdead_c0de_face_b00c;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn ema_update(current: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * current
}

fn abs_f64(v: f64) -> f64 {
    if v < 0.0 { -v } else { v }
}

fn sqrt_f64(v: f64) -> f64 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v;
    for _ in 0..25 {
        g = 0.5 * (g + v / g);
    }
    g
}

// ============================================================================
// ANOMALY TYPES
// ============================================================================

/// Types of anomalies the engine can forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalyType {
    Crash,
    Hang,
    MemoryThrash,
    CpuRunaway,
    IoStall,
    ResourceLeak,
    Deadlock,
    StackOverflow,
}

impl AnomalyType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Crash => "crash",
            Self::Hang => "hang",
            Self::MemoryThrash => "mem_thrash",
            Self::CpuRunaway => "cpu_runaway",
            Self::IoStall => "io_stall",
            Self::ResourceLeak => "res_leak",
            Self::Deadlock => "deadlock",
            Self::StackOverflow => "stack_overflow",
        }
    }

    fn severity(&self) -> f64 {
        match self {
            Self::Crash => 1.0,
            Self::Hang => 0.8,
            Self::MemoryThrash => 0.7,
            Self::CpuRunaway => 0.6,
            Self::IoStall => 0.5,
            Self::ResourceLeak => 0.65,
            Self::Deadlock => 0.9,
            Self::StackOverflow => 0.95,
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 8 {
            0 => Self::Crash,
            1 => Self::Hang,
            2 => Self::MemoryThrash,
            3 => Self::CpuRunaway,
            4 => Self::IoStall,
            5 => Self::ResourceLeak,
            6 => Self::Deadlock,
            _ => Self::StackOverflow,
        }
    }
}

// ============================================================================
// PREVENTION ACTION
// ============================================================================

/// Suggested prevention action when an anomaly is forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreventionAction {
    ThrottleCpu,
    LimitMemory,
    PreemptiveCheckpoint,
    KillAndRestart,
    IoRateLimit,
    ForceGarbageCollect,
    IncreaseStackSize,
    BreakLockChain,
}

impl PreventionAction {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ThrottleCpu => "throttle_cpu",
            Self::LimitMemory => "limit_memory",
            Self::PreemptiveCheckpoint => "checkpoint",
            Self::KillAndRestart => "kill_restart",
            Self::IoRateLimit => "io_rate_limit",
            Self::ForceGarbageCollect => "force_gc",
            Self::IncreaseStackSize => "increase_stack",
            Self::BreakLockChain => "break_locks",
        }
    }

    fn for_anomaly(anomaly: AnomalyType) -> Self {
        match anomaly {
            AnomalyType::Crash => Self::PreemptiveCheckpoint,
            AnomalyType::Hang => Self::KillAndRestart,
            AnomalyType::MemoryThrash => Self::LimitMemory,
            AnomalyType::CpuRunaway => Self::ThrottleCpu,
            AnomalyType::IoStall => Self::IoRateLimit,
            AnomalyType::ResourceLeak => Self::ForceGarbageCollect,
            AnomalyType::Deadlock => Self::BreakLockChain,
            AnomalyType::StackOverflow => Self::IncreaseStackSize,
        }
    }
}

// ============================================================================
// ANOMALY WARNING
// ============================================================================

/// An early warning of a predicted anomaly.
#[derive(Debug, Clone)]
pub struct AnomalyWarning {
    pub app_id: u64,
    pub anomaly_type: AnomalyType,
    pub lead_time_ticks: u64,
    pub confidence: f64,
    pub severity: f64,
    pub prevention: PreventionAction,
    pub tick_issued: u64,
    pub validated: Option<bool>,
    pub warning_hash: u64,
}

// ============================================================================
// BEHAVIORAL BASELINE
// ============================================================================

#[derive(Debug, Clone)]
struct BehavioralBaseline {
    ema_cpu: f64,
    ema_memory: f64,
    ema_io: f64,
    ema_syscalls: f64,
    ema_faults: f64,
    var_cpu: f64,
    var_memory: f64,
    var_io: f64,
    var_syscalls: f64,
    var_faults: f64,
    sample_count: u64,
}

impl BehavioralBaseline {
    fn new() -> Self {
        Self {
            ema_cpu: 0.0,
            ema_memory: 0.0,
            ema_io: 0.0,
            ema_syscalls: 0.0,
            ema_faults: 0.0,
            var_cpu: 1.0,
            var_memory: 1.0,
            var_io: 1.0,
            var_syscalls: 1.0,
            var_faults: 1.0,
            sample_count: 0,
        }
    }

    fn update(&mut self, cpu: f64, memory: f64, io: f64, syscalls: f64, faults: f64) {
        let alpha = EMA_ALPHA;
        // Update variance with Welford-like online approach via EMA
        let diff_cpu = cpu - self.ema_cpu;
        self.var_cpu = ema_update(self.var_cpu, diff_cpu * diff_cpu, alpha);
        let diff_mem = memory - self.ema_memory;
        self.var_memory = ema_update(self.var_memory, diff_mem * diff_mem, alpha);
        let diff_io = io - self.ema_io;
        self.var_io = ema_update(self.var_io, diff_io * diff_io, alpha);
        let diff_sc = syscalls - self.ema_syscalls;
        self.var_syscalls = ema_update(self.var_syscalls, diff_sc * diff_sc, alpha);
        let diff_f = faults - self.ema_faults;
        self.var_faults = ema_update(self.var_faults, diff_f * diff_f, alpha);

        self.ema_cpu = ema_update(self.ema_cpu, cpu, alpha);
        self.ema_memory = ema_update(self.ema_memory, memory, alpha);
        self.ema_io = ema_update(self.ema_io, io, alpha);
        self.ema_syscalls = ema_update(self.ema_syscalls, syscalls, alpha);
        self.ema_faults = ema_update(self.ema_faults, faults, alpha);
        self.sample_count += 1;
    }

    fn z_score(&self, metric_ema: f64, metric_var: f64, value: f64) -> f64 {
        let std = sqrt_f64(metric_var);
        if std < 0.001 {
            return 0.0;
        }
        (value - metric_ema) / std
    }

    fn cpu_z(&self, value: f64) -> f64 {
        self.z_score(self.ema_cpu, self.var_cpu, value)
    }
    fn memory_z(&self, value: f64) -> f64 {
        self.z_score(self.ema_memory, self.var_memory, value)
    }
    fn io_z(&self, value: f64) -> f64 {
        self.z_score(self.ema_io, self.var_io, value)
    }
    fn syscall_z(&self, value: f64) -> f64 {
        self.z_score(self.ema_syscalls, self.var_syscalls, value)
    }
    fn fault_z(&self, value: f64) -> f64 {
        self.z_score(self.ema_faults, self.var_faults, value)
    }
}

// ============================================================================
// PER-APP ANOMALY STATE
// ============================================================================

#[derive(Debug, Clone)]
struct AppAnomalyState {
    app_id: u64,
    baseline: BehavioralBaseline,
    warnings: Vec<AnomalyWarning>,
    total_warnings_issued: u64,
    total_warnings_validated: u64,
    true_positives: u64,
    false_positives: u64,
    ema_precision: f64,
}

impl AppAnomalyState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            baseline: BehavioralBaseline::new(),
            warnings: Vec::new(),
            total_warnings_issued: 0,
            total_warnings_validated: 0,
            true_positives: 0,
            false_positives: 0,
            ema_precision: 0.5,
        }
    }

    fn issue_warning(&mut self, anomaly: AnomalyType, confidence: f64, tick: u64, lead_time: u64) {
        let hash_data = [
            &self.app_id.to_le_bytes()[..],
            &tick.to_le_bytes()[..],
            anomaly.as_str().as_bytes(),
        ]
        .concat();
        let warning_hash = fnv1a_hash(&hash_data);

        let warning = AnomalyWarning {
            app_id: self.app_id,
            anomaly_type: anomaly,
            lead_time_ticks: lead_time,
            confidence,
            severity: anomaly.severity(),
            prevention: PreventionAction::for_anomaly(anomaly),
            tick_issued: tick,
            validated: None,
            warning_hash,
        };

        if self.warnings.len() >= MAX_WARNINGS {
            self.warnings.remove(0);
        }
        self.warnings.push(warning);
        self.total_warnings_issued += 1;
    }

    fn validate_warning(&mut self, warning_hash: u64, actually_happened: bool) {
        for w in &mut self.warnings {
            if w.warning_hash == warning_hash && w.validated.is_none() {
                w.validated = Some(actually_happened);
                self.total_warnings_validated += 1;
                if actually_happened {
                    self.true_positives += 1;
                } else {
                    self.false_positives += 1;
                }
                let prec = if self.total_warnings_validated > 0 {
                    self.true_positives as f64 / self.total_warnings_validated as f64
                } else {
                    0.5
                };
                self.ema_precision = ema_update(self.ema_precision, prec, EMA_ALPHA);
                return;
            }
        }
    }
}

// ============================================================================
// ANOMALY FORECAST STATS
// ============================================================================

/// Engine-level statistics for anomaly forecasting.
#[derive(Debug, Clone)]
pub struct AnomalyForecastStats {
    pub total_warnings_issued: u64,
    pub total_warnings_validated: u64,
    pub global_precision: f64,
    pub total_crash_precursors: u64,
    pub total_hang_detections: u64,
    pub total_thrash_predictions: u64,
    pub average_lead_time: f64,
    pub prevention_actions_suggested: u64,
}

impl AnomalyForecastStats {
    fn new() -> Self {
        Self {
            total_warnings_issued: 0,
            total_warnings_validated: 0,
            global_precision: 0.5,
            total_crash_precursors: 0,
            total_hang_detections: 0,
            total_thrash_predictions: 0,
            average_lead_time: 0.0,
            prevention_actions_suggested: 0,
        }
    }
}

// ============================================================================
// APPS ANOMALY FORECAST ENGINE
// ============================================================================

/// Anomaly forecast engine for application behavior.
///
/// Tracks behavioral baselines, detects precursor patterns, issues early
/// warnings, and validates predictions against actual outcomes.
pub struct AppsAnomalyForecast {
    app_states: BTreeMap<u64, AppAnomalyState>,
    stats: AnomalyForecastStats,
    rng_state: u64,
    tick: u64,
    ema_lead_time: f64,
    ema_precision: f64,
}

impl AppsAnomalyForecast {
    /// Create a new anomaly forecast engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: AnomalyForecastStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_lead_time: 10.0,
            ema_precision: 0.5,
        }
    }

    /// Update the behavioral baseline for an app with current metrics.
    pub fn update_baseline(
        &mut self,
        app_id: u64,
        cpu: f64,
        memory: f64,
        io: f64,
        syscalls: f64,
        faults: f64,
    ) {
        self.tick += 1;
        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            self.app_states.insert(app_id, AppAnomalyState::new(app_id));
        }
        let state = self.app_states.get_mut(&app_id).unwrap();
        state.baseline.update(cpu, memory, io, syscalls, faults);
    }

    /// Forecast potential anomalies for an app given current observations.
    ///
    /// Returns a list of warnings for any metric that deviates beyond the
    /// anomaly threshold from baseline.
    pub fn forecast_app_anomaly(
        &mut self,
        app_id: u64,
        cpu: f64,
        memory: f64,
        io: f64,
        syscalls: f64,
        faults: f64,
    ) -> Vec<AnomalyWarning> {
        let state = match self.app_states.get_mut(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let bl = &state.baseline;
        if bl.sample_count < 10 {
            return Vec::new();
        }

        let mut warnings = Vec::new();
        let lead = 50u64; // default lead time estimate

        let cpu_z = bl.cpu_z(cpu);
        if cpu_z > ANOMALY_SIGMA_THRESHOLD {
            let conf = (cpu_z - ANOMALY_SIGMA_THRESHOLD) / (cpu_z + 1.0);
            state.issue_warning(AnomalyType::CpuRunaway, conf.min(0.95), self.tick, lead);
            if let Some(w) = state.warnings.last() {
                warnings.push(w.clone());
            }
        }

        let mem_z = bl.memory_z(memory);
        if mem_z > ANOMALY_SIGMA_THRESHOLD {
            let conf = (mem_z - ANOMALY_SIGMA_THRESHOLD) / (mem_z + 1.0);
            state.issue_warning(AnomalyType::MemoryThrash, conf.min(0.95), self.tick, lead);
            if let Some(w) = state.warnings.last() {
                warnings.push(w.clone());
            }
            self.stats.total_thrash_predictions += 1;
        }

        let io_z = bl.io_z(io);
        if io_z > ANOMALY_SIGMA_THRESHOLD {
            let conf = (io_z - ANOMALY_SIGMA_THRESHOLD) / (io_z + 1.0);
            state.issue_warning(AnomalyType::IoStall, conf.min(0.95), self.tick, lead);
            if let Some(w) = state.warnings.last() {
                warnings.push(w.clone());
            }
        }

        let fault_z = bl.fault_z(faults);
        if fault_z > ANOMALY_SIGMA_THRESHOLD * 1.2 {
            let conf = (fault_z - ANOMALY_SIGMA_THRESHOLD) / (fault_z + 1.0);
            state.issue_warning(AnomalyType::Crash, conf.min(0.95), self.tick, lead * 2);
            if let Some(w) = state.warnings.last() {
                warnings.push(w.clone());
            }
            self.stats.total_crash_precursors += 1;
        }

        self.stats.total_warnings_issued += warnings.len() as u64;
        self.stats.prevention_actions_suggested += warnings.len() as u64;

        for w in &warnings {
            self.ema_lead_time = ema_update(self.ema_lead_time, w.lead_time_ticks as f64, EMA_ALPHA);
        }
        self.stats.average_lead_time = self.ema_lead_time;

        warnings
    }

    /// Detect crash precursors: rising fault rate + memory pressure.
    pub fn crash_precursor(&self, app_id: u64, faults: f64, memory: f64) -> Option<f64> {
        let state = self.app_states.get(&app_id)?;
        let bl = &state.baseline;
        if bl.sample_count < 10 {
            return None;
        }

        let fault_z = bl.fault_z(faults);
        let mem_z = bl.memory_z(memory);

        if fault_z > 1.5 && mem_z > 1.5 {
            let combined = (fault_z + mem_z) / 2.0;
            let prob = (combined - 1.5) / (combined + 1.0);
            Some(prob.min(0.99))
        } else {
            None
        }
    }

    /// Detect hang precursors: syscall rate drops to near zero + CPU still active.
    pub fn hang_detection(&mut self, app_id: u64, syscalls: f64, cpu: f64) -> Option<f64> {
        let state = self.app_states.get(&app_id)?;
        let bl = &state.baseline;
        if bl.sample_count < 10 {
            return None;
        }

        let sc_z = bl.syscall_z(syscalls);
        let cpu_z = bl.cpu_z(cpu);

        // Hang: syscalls drop (negative z) while CPU stays normal or high
        if sc_z < -ANOMALY_SIGMA_THRESHOLD && cpu_z > -0.5 {
            let hang_signal = abs_f64(sc_z) - ANOMALY_SIGMA_THRESHOLD;
            let prob = hang_signal / (hang_signal + 2.0);
            self.stats.total_hang_detections += 1;
            Some(prob.min(0.95))
        } else {
            None
        }
    }

    /// Predict memory thrashing: rapid alternation of alloc/free with rising fault rate.
    pub fn thrash_prediction(&self, app_id: u64, memory: f64, faults: f64) -> Option<f64> {
        let state = self.app_states.get(&app_id)?;
        let bl = &state.baseline;
        if bl.sample_count < 10 {
            return None;
        }

        let mem_z = bl.memory_z(memory);
        let fault_z = bl.fault_z(faults);

        if mem_z > 1.0 && fault_z > 2.0 {
            let signal = (mem_z + fault_z) / 2.0;
            let prob = signal / (signal + 3.0);
            Some(prob.min(0.95))
        } else {
            None
        }
    }

    /// Get the most recent early warning for an app, if any.
    pub fn early_warning(&self, app_id: u64) -> Option<&AnomalyWarning> {
        let state = self.app_states.get(&app_id)?;
        state.warnings.last()
    }

    /// Suggest a prevention action for a given anomaly type.
    pub fn prevention_action(&self, anomaly: AnomalyType) -> PreventionAction {
        PreventionAction::for_anomaly(anomaly)
    }

    /// Validate a previously issued warning.
    pub fn validate_warning(&mut self, app_id: u64, warning_hash: u64, happened: bool) {
        self.stats.total_warnings_validated += 1;
        if let Some(state) = self.app_states.get_mut(&app_id) {
            state.validate_warning(warning_hash, happened);
            self.ema_precision = ema_update(self.ema_precision, state.ema_precision, EMA_ALPHA);
            self.stats.global_precision = self.ema_precision;
        }
    }

    /// Return a snapshot of engine statistics.
    pub fn stats(&self) -> &AnomalyForecastStats {
        &self.stats
    }

    /// Number of tracked apps.
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }

    /// Global EMA-smoothed warning precision.
    pub fn global_precision(&self) -> f64 {
        self.ema_precision
    }
}

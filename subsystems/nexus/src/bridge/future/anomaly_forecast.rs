// SPDX-License-Identifier: GPL-2.0
//! # Bridge Anomaly Forecast
//!
//! Predicts anomalies BEFORE they happen. Detects precursors to syscall storms,
//! latency spikes, and resource exhaustion. Maintains a library of known
//! precursor patterns and scans incoming event streams for early matches. The
//! bridge doesn't just react to anomalies — it sees them coming and raises
//! early warnings with enough lead time to prevent them.
//!
//! The best anomaly is the one that never happens.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PRECURSORS: usize = 128;
const MAX_ACTIVE_WARNINGS: usize = 64;
const MAX_HISTORY: usize = 1024;
const PATTERN_WINDOW: usize = 32;
const EMA_ALPHA: f32 = 0.08;
const SEVERITY_THRESHOLD: f32 = 0.3;
const FALSE_ALARM_DECAY: f32 = 0.998;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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

// ============================================================================
// ANOMALY TYPE
// ============================================================================

/// Types of anomalies the engine can forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalyType {
    /// Sudden burst of syscalls overwhelming the bridge
    SyscallStorm,
    /// Latency spike affecting response times
    LatencySpike,
    /// Memory or resource exhaustion approaching
    ResourceExhaustion,
    /// Deadlock or livelock conditions forming
    DeadlockRisk,
    /// Abnormal process behavior pattern
    BehaviorAnomaly,
    /// IPC channel congestion
    IpcCongestion,
}

// ============================================================================
// ANOMALY PRECURSOR
// ============================================================================

/// A known precursor pattern that precedes an anomaly.
#[derive(Debug, Clone)]
pub struct AnomalyPrecursor {
    /// Hash identifying this precursor pattern
    pub pattern: u64,
    /// The anomaly type this precursor indicates
    pub anomaly_type: AnomalyType,
    /// Average lead time in ticks before the anomaly manifests
    pub lead_time_ticks: u64,
    /// Severity of the predicted anomaly (0.0 to 1.0)
    pub severity: f32,
    /// Confidence in this precursor (0.0 to 1.0)
    pub confidence: f32,
    /// Number of times this precursor was observed before an actual anomaly
    true_positives: u64,
    /// Number of times this precursor fired but no anomaly followed
    false_positives: u64,
    /// Sequence of event hashes that form the precursor pattern
    pattern_sequence: Vec<u64>,
    /// Lead time EMA
    lead_time_ema: f32,
}

impl AnomalyPrecursor {
    fn new(pattern: u64, anomaly_type: AnomalyType, sequence: Vec<u64>) -> Self {
        Self {
            pattern,
            anomaly_type,
            lead_time_ticks: 100,
            severity: 0.5,
            confidence: 0.1,
            true_positives: 0,
            false_positives: 0,
            pattern_sequence: sequence,
            lead_time_ema: 100.0,
        }
    }

    fn record_true_positive(&mut self, actual_lead_time: u64) {
        self.true_positives += 1;
        self.lead_time_ema = self.lead_time_ema * (1.0 - EMA_ALPHA)
            + actual_lead_time as f32 * EMA_ALPHA;
        self.lead_time_ticks = self.lead_time_ema as u64;
        self.update_confidence();
    }

    fn record_false_positive(&mut self) {
        self.false_positives += 1;
        self.update_confidence();
    }

    fn update_confidence(&mut self) {
        let total = self.true_positives + self.false_positives;
        if total > 0 {
            self.confidence = self.true_positives as f32 / total as f32;
        }
    }

    fn precision(&self) -> f32 {
        let total = self.true_positives + self.false_positives;
        if total > 0 {
            self.true_positives as f32 / total as f32
        } else {
            0.0
        }
    }
}

// ============================================================================
// EARLY WARNING
// ============================================================================

/// An active early warning: an anomaly has been predicted but hasn't occurred yet.
#[derive(Debug, Clone)]
pub struct EarlyWarning {
    /// Warning identifier
    pub warning_id: u64,
    /// The precursor that triggered this warning
    pub precursor_pattern: u64,
    /// Type of predicted anomaly
    pub anomaly_type: AnomalyType,
    /// Estimated ticks until anomaly manifests
    pub estimated_ticks_remaining: u64,
    /// Severity of the predicted anomaly
    pub severity: f32,
    /// Confidence in this warning
    pub confidence: f32,
    /// Tick when the warning was issued
    pub issued_tick: u64,
    /// Whether this warning has been resolved (true positive or expired)
    pub resolved: bool,
}

// ============================================================================
// EVENT STREAM
// ============================================================================

/// Sliding window of recent events for pattern matching.
#[derive(Debug, Clone)]
struct EventStream {
    /// Recent event hashes
    events: Vec<u64>,
    /// Recent event ticks
    ticks: Vec<u64>,
    /// Rate estimation per event type (EMA)
    rates: BTreeMap<u64, f32>,
    /// Burstiness: variance of inter-event times
    burstiness_ema: f32,
}

impl EventStream {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            ticks: Vec::new(),
            rates: BTreeMap::new(),
            burstiness_ema: 0.0,
        }
    }

    fn push(&mut self, event_hash: u64, tick: u64) {
        self.events.push(event_hash);
        self.ticks.push(tick);

        if self.events.len() > MAX_HISTORY {
            self.events.remove(0);
            self.ticks.remove(0);
        }

        // Update rate estimate
        let rate = self.rates.entry(event_hash).or_insert(0.0);
        *rate = *rate * (1.0 - EMA_ALPHA) + EMA_ALPHA;

        // Update burstiness
        if self.ticks.len() >= 2 {
            let last = self.ticks[self.ticks.len() - 1];
            let prev = self.ticks[self.ticks.len() - 2];
            let gap = last.saturating_sub(prev) as f32;
            let mean_gap = if self.ticks.len() > 1 {
                let total_span = self.ticks.last().unwrap_or(&0)
                    .saturating_sub(*self.ticks.first().unwrap_or(&0));
                total_span as f32 / self.ticks.len() as f32
            } else {
                gap
            };
            let burst_val = (gap - mean_gap) * (gap - mean_gap);
            self.burstiness_ema = self.burstiness_ema * (1.0 - EMA_ALPHA) + burst_val * EMA_ALPHA;
        }
    }

    fn recent_window(&self, n: usize) -> &[u64] {
        let start = if self.events.len() > n { self.events.len() - n } else { 0 };
        &self.events[start..]
    }

    fn recent_rate(&self, event_hash: u64) -> f32 {
        self.rates.get(&event_hash).copied().unwrap_or(0.0)
    }
}

// ============================================================================
// ANOMALY FORECAST STATS
// ============================================================================

/// Statistics for the anomaly forecast engine.
#[derive(Debug, Clone)]
pub struct AnomalyForecastStats {
    pub total_precursors: u32,
    pub total_warnings_issued: u64,
    pub total_true_positives: u64,
    pub total_false_positives: u64,
    pub false_alarm_rate: f32,
    pub avg_lead_time: f32,
    pub avg_severity: f32,
    pub active_warnings: u32,
    pub anomalies_prevented: u64,
}

impl AnomalyForecastStats {
    fn new() -> Self {
        Self {
            total_precursors: 0,
            total_warnings_issued: 0,
            total_true_positives: 0,
            total_false_positives: 0,
            false_alarm_rate: 0.0,
            avg_lead_time: 0.0,
            avg_severity: 0.0,
            active_warnings: 0,
            anomalies_prevented: 0,
        }
    }
}

// ============================================================================
// BRIDGE ANOMALY FORECAST
// ============================================================================

/// Anomaly forecasting engine for the syscall bridge.
///
/// Detects precursors to anomalies in the event stream and issues early
/// warnings with enough lead time to take preventive action.
pub struct BridgeAnomalyForecast {
    /// Library of known precursor patterns
    precursors: BTreeMap<u64, AnomalyPrecursor>,
    /// Active early warnings
    warnings: Vec<EarlyWarning>,
    /// Event stream for pattern matching
    stream: EventStream,
    /// Running statistics
    stats: AnomalyForecastStats,
    /// PRNG state
    rng: u64,
    /// Current tick
    tick: u64,
    /// Warning ID counter
    next_warning_id: u64,
}

impl BridgeAnomalyForecast {
    /// Create a new anomaly forecast engine.
    pub fn new() -> Self {
        Self {
            precursors: BTreeMap::new(),
            warnings: Vec::new(),
            stream: EventStream::new(),
            stats: AnomalyForecastStats::new(),
            rng: 0xA40F_FC57_DEAD_BEEF,
            tick: 0,
            next_warning_id: 1,
        }
    }

    /// Register a known precursor pattern.
    pub fn register_precursor(
        &mut self,
        anomaly_type: AnomalyType,
        pattern_sequence: Vec<u64>,
        initial_lead_time: u64,
        initial_severity: f32,
    ) {
        if self.precursors.len() >= MAX_PRECURSORS {
            // Evict lowest confidence precursor
            let mut min_conf = f32::INFINITY;
            let mut min_key = 0u64;
            for (k, v) in &self.precursors {
                if v.confidence < min_conf {
                    min_conf = v.confidence;
                    min_key = *k;
                }
            }
            self.precursors.remove(&min_key);
        }

        let mut hash_data = Vec::new();
        for &h in &pattern_sequence {
            hash_data.extend_from_slice(&h.to_le_bytes());
        }
        let pattern_hash = fnv1a_hash(&hash_data);

        let mut precursor = AnomalyPrecursor::new(pattern_hash, anomaly_type, pattern_sequence);
        precursor.lead_time_ticks = initial_lead_time;
        precursor.severity = initial_severity;

        self.precursors.insert(pattern_hash, precursor);
        self.stats.total_precursors = self.precursors.len() as u32;
    }

    /// Process an incoming event and check for precursor matches.
    pub fn process_event(&mut self, event_hash: u64, tick: u64) -> Vec<EarlyWarning> {
        self.tick = tick;
        self.stream.push(event_hash, tick);
        self.expire_warnings();

        let new_warnings = self.scan_for_precursors();
        for w in &new_warnings {
            self.warnings.push(w.clone());
            self.stats.total_warnings_issued += 1;
        }

        self.stats.active_warnings = self.warnings.iter().filter(|w| !w.resolved).count() as u32;
        new_warnings
    }

    fn scan_for_precursors(&mut self) -> Vec<EarlyWarning> {
        let mut new_warnings = Vec::new();
        let window = self.stream.recent_window(PATTERN_WINDOW);
        if window.is_empty() {
            return new_warnings;
        }

        for precursor in self.precursors.values() {
            if precursor.confidence < 0.05 {
                continue;
            }
            if self.is_already_warned(precursor.pattern) {
                continue;
            }

            let match_score = self.pattern_match_score(&precursor.pattern_sequence, window);
            if match_score > 0.6 {
                let severity = precursor.severity * match_score;
                if severity >= SEVERITY_THRESHOLD {
                    let warning = EarlyWarning {
                        warning_id: self.next_warning_id,
                        precursor_pattern: precursor.pattern,
                        anomaly_type: precursor.anomaly_type,
                        estimated_ticks_remaining: precursor.lead_time_ticks,
                        severity,
                        confidence: precursor.confidence * match_score,
                        issued_tick: self.tick,
                        resolved: false,
                    };
                    self.next_warning_id += 1;
                    new_warnings.push(warning);
                }
            }
        }

        if new_warnings.len() > MAX_ACTIVE_WARNINGS {
            new_warnings.sort_by(|a, b| {
                b.severity.partial_cmp(&a.severity).unwrap_or(core::cmp::Ordering::Equal)
            });
            new_warnings.truncate(MAX_ACTIVE_WARNINGS);
        }
        new_warnings
    }

    fn pattern_match_score(&self, pattern: &[u64], window: &[u64]) -> f32 {
        if pattern.is_empty() || window.is_empty() {
            return 0.0;
        }

        // Subsequence matching: how many pattern elements appear in order in the window
        let mut matched = 0usize;
        let mut window_idx = 0usize;

        for &pat_elem in pattern {
            while window_idx < window.len() {
                if window[window_idx] == pat_elem {
                    matched += 1;
                    window_idx += 1;
                    break;
                }
                window_idx += 1;
            }
            if window_idx >= window.len() {
                break;
            }
        }

        matched as f32 / pattern.len() as f32
    }

    fn is_already_warned(&self, pattern: u64) -> bool {
        self.warnings.iter().any(|w| w.precursor_pattern == pattern && !w.resolved)
    }

    fn expire_warnings(&mut self) {
        for warning in self.warnings.iter_mut() {
            if warning.resolved {
                continue;
            }
            let elapsed = self.tick.saturating_sub(warning.issued_tick);
            if elapsed > warning.estimated_ticks_remaining * 3 {
                // Expired without anomaly: false positive
                warning.resolved = true;
                self.stats.total_false_positives += 1;
                if let Some(precursor) = self.precursors.get_mut(&warning.precursor_pattern) {
                    precursor.record_false_positive();
                }
            }
        }
        // Clean up old resolved warnings
        self.warnings.retain(|w| !w.resolved || self.tick.saturating_sub(w.issued_tick) < 10000);
    }

    /// Forecast the most likely anomaly type in the near future.
    pub fn forecast_anomaly(&self) -> Option<(AnomalyType, f32, f32)> {
        // Return the highest-severity active warning
        let mut best: Option<(AnomalyType, f32, f32)> = None;
        for w in &self.warnings {
            if w.resolved {
                continue;
            }
            match &best {
                Some((_, s, _)) if w.severity <= *s => {}
                _ => best = Some((w.anomaly_type, w.severity, w.confidence)),
            }
        }
        best
    }

    /// Detect precursor patterns in the current event stream.
    pub fn detect_precursor(&self) -> Vec<(u64, f32)> {
        let window = self.stream.recent_window(PATTERN_WINDOW);
        let mut matches = Vec::new();
        for precursor in self.precursors.values() {
            let score = self.pattern_match_score(&precursor.pattern_sequence, window);
            if score > 0.3 {
                matches.push((precursor.pattern, score));
            }
        }
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        matches
    }

    /// Get the precursor library sorted by confidence.
    pub fn precursor_library(&self) -> Vec<&AnomalyPrecursor> {
        let mut result: Vec<&AnomalyPrecursor> = self.precursors.values().collect();
        result.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(core::cmp::Ordering::Equal)
        });
        result
    }

    /// Get active early warnings.
    pub fn early_warning(&self) -> Vec<&EarlyWarning> {
        self.warnings.iter().filter(|w| !w.resolved).collect()
    }

    /// Compute the false alarm rate across all precursors.
    pub fn false_alarm_rate(&self) -> f32 {
        let total = self.stats.total_true_positives + self.stats.total_false_positives;
        if total > 0 {
            self.stats.total_false_positives as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Record that a preventive action successfully averted an anomaly.
    pub fn anomaly_prevention(&mut self, warning_id: u64, actual_lead_time: u64) {
        self.stats.anomalies_prevented += 1;
        for warning in self.warnings.iter_mut() {
            if warning.warning_id == warning_id && !warning.resolved {
                warning.resolved = true;
                self.stats.total_true_positives += 1;
                if let Some(precursor) = self.precursors.get_mut(&warning.precursor_pattern) {
                    precursor.record_true_positive(actual_lead_time);
                }
                break;
            }
        }
        self.update_false_alarm_rate();
    }

    fn update_false_alarm_rate(&mut self) {
        let total = self.stats.total_true_positives + self.stats.total_false_positives;
        if total > 0 {
            self.stats.false_alarm_rate =
                self.stats.total_false_positives as f32 / total as f32;
        }
    }

    /// Get statistics.
    pub fn stats(&self) -> &AnomalyForecastStats {
        &self.stats
    }

    /// Get burstiness of the event stream.
    pub fn stream_burstiness(&self) -> f32 {
        self.stream.burstiness_ema
    }
}

//! Signal Pattern Detection
//!
//! Detects patterns in signal flow such as storms, bursts, and ping-pong.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{ProcessId, SignalNumber};

/// Detected signal pattern
#[derive(Debug, Clone)]
pub struct SignalPattern {
    /// Pattern type
    pub pattern_type: PatternType,
    /// Involved signals
    pub signals: Vec<SignalNumber>,
    /// Involved processes
    pub processes: Vec<ProcessId>,
    /// Pattern confidence (0-1)
    pub confidence: f32,
    /// Detection timestamp
    pub detected_at: u64,
    /// Occurrence count
    pub occurrences: u64,
}

/// Pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Signal storm (high frequency)
    Storm,
    /// Ping-pong between processes
    PingPong,
    /// Cascade (process dies, children get signals)
    Cascade,
    /// Periodic signals
    Periodic,
    /// Signal escalation (TERM -> KILL)
    Escalation,
    /// Signal burst (multiple signals quickly)
    Burst,
    /// Death spiral (process keeps crashing)
    DeathSpiral,
}

/// Signal event for pattern detection
#[derive(Debug, Clone, Copy)]
pub(crate) struct SignalEvent {
    pub signo: SignalNumber,
    pub sender: ProcessId,
    pub receiver: ProcessId,
    pub timestamp: u64,
}

/// Signal pattern detector
pub struct SignalPatternDetector {
    /// Recent signal events
    events: Vec<SignalEvent>,
    /// Maximum events to track
    max_events: usize,
    /// Detected patterns
    patterns: Vec<SignalPattern>,
    /// Storm threshold (signals per second)
    storm_threshold: f32,
    /// Burst threshold (signals in window)
    burst_threshold: u32,
    /// Burst window (nanoseconds)
    burst_window_ns: u64,
    /// Pattern detection count
    patterns_detected: u64,
}

impl SignalPatternDetector {
    /// Create new pattern detector
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(1000),
            max_events: 1000,
            patterns: Vec::new(),
            storm_threshold: 100.0,
            burst_threshold: 10,
            burst_window_ns: 100_000_000, // 100ms
            patterns_detected: 0,
        }
    }

    /// Record signal event
    pub fn record_event(
        &mut self,
        signo: SignalNumber,
        sender: ProcessId,
        receiver: ProcessId,
        timestamp: u64,
    ) {
        let event = SignalEvent {
            signo,
            sender,
            receiver,
            timestamp,
        };

        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);

        // Run pattern detection
        self.detect_patterns(timestamp);
    }

    /// Detect patterns in recent events
    fn detect_patterns(&mut self, current_time: u64) {
        self.detect_storm(current_time);
        self.detect_burst(current_time);
        self.detect_ping_pong();
        self.detect_escalation();
    }

    /// Detect signal storm
    fn detect_storm(&mut self, current_time: u64) {
        let window_start = current_time.saturating_sub(1_000_000_000); // 1 second
        let recent_count = self
            .events
            .iter()
            .filter(|e| e.timestamp >= window_start)
            .count();

        let rate = recent_count as f32;
        if rate > self.storm_threshold {
            // Group by receiver
            let mut receivers: BTreeMap<ProcessId, u32> = BTreeMap::new();
            for event in self.events.iter().filter(|e| e.timestamp >= window_start) {
                *receivers.entry(event.receiver).or_default() += 1;
            }

            for (pid, count) in receivers {
                if count as f32 > self.storm_threshold / 2.0 {
                    let pattern = SignalPattern {
                        pattern_type: PatternType::Storm,
                        signals: vec![],
                        processes: vec![pid],
                        confidence: (count as f32 / self.storm_threshold).min(1.0),
                        detected_at: current_time,
                        occurrences: count as u64,
                    };
                    self.add_pattern(pattern);
                }
            }
        }
    }

    /// Detect signal burst
    fn detect_burst(&mut self, current_time: u64) {
        let window_start = current_time.saturating_sub(self.burst_window_ns);

        // Group by (sender, receiver) pair
        let mut pairs: BTreeMap<(ProcessId, ProcessId), Vec<SignalNumber>> = BTreeMap::new();
        for event in self.events.iter().filter(|e| e.timestamp >= window_start) {
            pairs
                .entry((event.sender, event.receiver))
                .or_default()
                .push(event.signo);
        }

        for ((sender, receiver), signals) in pairs {
            if signals.len() >= self.burst_threshold as usize {
                let pattern = SignalPattern {
                    pattern_type: PatternType::Burst,
                    signals,
                    processes: vec![sender, receiver],
                    confidence: 0.8,
                    detected_at: current_time,
                    occurrences: 1,
                };
                self.add_pattern(pattern);
            }
        }
    }

    /// Detect ping-pong pattern
    fn detect_ping_pong(&mut self) {
        if self.events.len() < 4 {
            return;
        }

        // Look for A->B, B->A, A->B, B->A sequence
        for window in self.events.windows(4) {
            let a = window[0].sender;
            let b = window[0].receiver;

            if window[1].sender == b
                && window[1].receiver == a
                && window[2].sender == a
                && window[2].receiver == b
                && window[3].sender == b
                && window[3].receiver == a
            {
                let pattern = SignalPattern {
                    pattern_type: PatternType::PingPong,
                    signals: window.iter().map(|e| e.signo).collect(),
                    processes: vec![a, b],
                    confidence: 0.9,
                    detected_at: window[3].timestamp,
                    occurrences: 1,
                };
                self.add_pattern(pattern);
            }
        }
    }

    /// Detect signal escalation (TERM -> KILL)
    fn detect_escalation(&mut self) {
        if self.events.len() < 2 {
            return;
        }

        for window in self.events.windows(2) {
            if window[0].signo == SignalNumber::SIGTERM
                && window[1].signo == SignalNumber::SIGKILL
                && window[0].receiver == window[1].receiver
            {
                let pattern = SignalPattern {
                    pattern_type: PatternType::Escalation,
                    signals: vec![SignalNumber::SIGTERM, SignalNumber::SIGKILL],
                    processes: vec![window[0].receiver],
                    confidence: 0.95,
                    detected_at: window[1].timestamp,
                    occurrences: 1,
                };
                self.add_pattern(pattern);
            }
        }
    }

    /// Add detected pattern
    fn add_pattern(&mut self, pattern: SignalPattern) {
        // Check if similar pattern exists
        for existing in &mut self.patterns {
            if existing.pattern_type == pattern.pattern_type
                && existing.processes == pattern.processes
            {
                existing.occurrences += 1;
                existing.detected_at = pattern.detected_at;
                return;
            }
        }

        self.patterns.push(pattern);
        self.patterns_detected += 1;
    }

    /// Get active patterns
    pub fn get_patterns(&self) -> &[SignalPattern] {
        &self.patterns
    }

    /// Clear old patterns
    pub fn cleanup(&mut self, max_age_ns: u64, current_time: u64) {
        let cutoff = current_time.saturating_sub(max_age_ns);
        self.patterns.retain(|p| p.detected_at >= cutoff);
        self.events.retain(|e| e.timestamp >= cutoff);
    }

    /// Get patterns detected count
    pub fn patterns_detected(&self) -> u64 {
        self.patterns_detected
    }

    /// Set storm threshold
    pub fn set_storm_threshold(&mut self, threshold: f32) {
        self.storm_threshold = threshold;
    }

    /// Set burst threshold
    pub fn set_burst_threshold(&mut self, threshold: u32) {
        self.burst_threshold = threshold;
    }
}

impl Default for SignalPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

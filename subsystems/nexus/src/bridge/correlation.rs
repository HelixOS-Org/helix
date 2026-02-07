//! # Bridge Correlation Engine
//!
//! Syscall correlation and relationship tracking:
//! - Temporal correlation between syscalls
//! - Causal chain detection
//! - Cross-process correlation
//! - Pattern-based correlation rules
//! - Anomaly correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CORRELATION TYPES
// ============================================================================

/// Correlation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallCorrelationType {
    /// Temporal (happen close in time)
    Temporal,
    /// Causal (one causes the other)
    Causal,
    /// Resource (share a resource)
    Resource,
    /// Process (same process chain)
    Process,
    /// Pattern (match a known pattern)
    Pattern,
}

/// Correlation strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrelationStrength {
    /// Weak correlation
    Weak,
    /// Moderate
    Moderate,
    /// Strong
    Strong,
    /// Definite
    Definite,
}

// ============================================================================
// CORRELATION ENTRY
// ============================================================================

/// A syscall event for correlation
#[derive(Debug, Clone)]
pub struct SyscallEvent {
    /// Event id
    pub id: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Process id
    pub pid: u64,
    /// Thread id
    pub tid: u64,
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Return value
    pub result: i64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// File descriptor (if relevant)
    pub fd: Option<i32>,
}

/// A correlation link between two events
#[derive(Debug, Clone)]
pub struct CorrelationLink {
    /// Source event id
    pub source: u64,
    /// Target event id
    pub target: u64,
    /// Correlation type
    pub correlation_type: SyscallCorrelationType,
    /// Strength
    pub strength: CorrelationStrength,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
}

// ============================================================================
// TEMPORAL WINDOW
// ============================================================================

/// Sliding window for temporal correlation
#[derive(Debug)]
pub struct TemporalWindow {
    /// Window duration (ns)
    pub window_ns: u64,
    /// Events in window
    events: Vec<SyscallEvent>,
    /// Max events to keep
    max_events: usize,
}

impl TemporalWindow {
    pub fn new(window_ns: u64, max_events: usize) -> Self {
        Self {
            window_ns,
            events: Vec::new(),
            max_events,
        }
    }

    /// Add event
    pub fn add(&mut self, event: SyscallEvent) {
        let cutoff = event.timestamp.saturating_sub(self.window_ns);
        self.events.retain(|e| e.timestamp >= cutoff);
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Find events within time range of a timestamp
    pub fn find_near(&self, timestamp: u64, range_ns: u64) -> Vec<&SyscallEvent> {
        let lo = timestamp.saturating_sub(range_ns);
        let hi = timestamp.saturating_add(range_ns);
        self.events.iter().filter(|e| e.timestamp >= lo && e.timestamp <= hi).collect()
    }

    /// Find events by pid
    pub fn find_by_pid(&self, pid: u64) -> Vec<&SyscallEvent> {
        self.events.iter().filter(|e| e.pid == pid).collect()
    }

    /// Current event count
    pub fn count(&self) -> usize {
        self.events.len()
    }
}

// ============================================================================
// CORRELATION RULES
// ============================================================================

/// A correlation rule
#[derive(Debug, Clone)]
pub struct CorrelationRule {
    /// Rule id
    pub id: u64,
    /// Source syscall number (None = any)
    pub source_syscall: Option<u32>,
    /// Target syscall number (None = any)
    pub target_syscall: Option<u32>,
    /// Max time between (ns)
    pub max_gap_ns: u64,
    /// Must be same process?
    pub same_process: bool,
    /// Correlation type
    pub correlation_type: SyscallCorrelationType,
    /// Strength if matched
    pub strength: CorrelationStrength,
}

impl CorrelationRule {
    pub fn new(id: u64, corr_type: SyscallCorrelationType) -> Self {
        Self {
            id,
            source_syscall: None,
            target_syscall: None,
            max_gap_ns: 1_000_000, // 1ms default
            same_process: false,
            correlation_type: corr_type,
            strength: CorrelationStrength::Moderate,
        }
    }

    /// Check if two events match this rule
    pub fn matches(&self, source: &SyscallEvent, target: &SyscallEvent) -> bool {
        if let Some(src_nr) = self.source_syscall {
            if source.syscall_nr != src_nr {
                return false;
            }
        }
        if let Some(tgt_nr) = self.target_syscall {
            if target.syscall_nr != tgt_nr {
                return false;
            }
        }
        if self.same_process && source.pid != target.pid {
            return false;
        }
        let gap = if target.timestamp > source.timestamp {
            target.timestamp - source.timestamp
        } else {
            source.timestamp - target.timestamp
        };
        gap <= self.max_gap_ns
    }
}

// ============================================================================
// CO-OCCURRENCE MATRIX
// ============================================================================

/// Syscall co-occurrence tracker
#[derive(Debug)]
pub struct CoOccurrenceMatrix {
    /// Counts: (syscall_a, syscall_b) -> count
    counts: BTreeMap<u64, u64>,
    /// Per-syscall total
    totals: BTreeMap<u32, u64>,
}

impl CoOccurrenceMatrix {
    pub fn new() -> Self {
        Self {
            counts: BTreeMap::new(),
            totals: BTreeMap::new(),
        }
    }

    fn pair_key(a: u32, b: u32) -> u64 {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        ((lo as u64) << 32) | (hi as u64)
    }

    /// Record co-occurrence
    pub fn record(&mut self, a: u32, b: u32) {
        let key = Self::pair_key(a, b);
        *self.counts.entry(key).or_insert(0) += 1;
        *self.totals.entry(a).or_insert(0) += 1;
        if a != b {
            *self.totals.entry(b).or_insert(0) += 1;
        }
    }

    /// Get co-occurrence count
    pub fn count(&self, a: u32, b: u32) -> u64 {
        let key = Self::pair_key(a, b);
        self.counts.get(&key).copied().unwrap_or(0)
    }

    /// Correlation coefficient (Jaccard index)
    pub fn correlation(&self, a: u32, b: u32) -> f64 {
        let co = self.count(a, b) as f64;
        let ta = self.totals.get(&a).copied().unwrap_or(0) as f64;
        let tb = self.totals.get(&b).copied().unwrap_or(0) as f64;
        let union = ta + tb - co;
        if union <= 0.0 {
            return 0.0;
        }
        co / union
    }
}

// ============================================================================
// CORRELATION ENGINE
// ============================================================================

/// Correlation stats
#[derive(Debug, Clone, Default)]
pub struct BridgeCorrelationStats {
    /// Events processed
    pub events_processed: u64,
    /// Correlations found
    pub correlations_found: u64,
    /// Active rules
    pub active_rules: usize,
}

/// Bridge correlation engine
pub struct BridgeCorrelationEngine {
    /// Temporal window
    window: TemporalWindow,
    /// Correlation rules
    rules: Vec<CorrelationRule>,
    /// Co-occurrence matrix
    cooccurrence: CoOccurrenceMatrix,
    /// Found correlations (recent)
    correlations: Vec<CorrelationLink>,
    /// Max stored correlations
    max_correlations: usize,
    /// Stats
    stats: BridgeCorrelationStats,
}

impl BridgeCorrelationEngine {
    pub fn new(window_ns: u64) -> Self {
        Self {
            window: TemporalWindow::new(window_ns, 10000),
            rules: Vec::new(),
            cooccurrence: CoOccurrenceMatrix::new(),
            correlations: Vec::new(),
            max_correlations: 10000,
            stats: BridgeCorrelationStats::default(),
        }
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: CorrelationRule) {
        self.rules.push(rule);
        self.stats.active_rules = self.rules.len();
    }

    /// Process event
    pub fn process(&mut self, event: SyscallEvent) {
        self.stats.events_processed += 1;

        // Check rules against recent events
        let nearby = self.window.find_near(event.timestamp, 1_000_000);
        for existing in nearby {
            for rule in &self.rules {
                if rule.matches(existing, &event) {
                    let link = CorrelationLink {
                        source: existing.id,
                        target: event.id,
                        correlation_type: rule.correlation_type,
                        strength: rule.strength,
                        confidence: 0.8,
                    };
                    if self.correlations.len() >= self.max_correlations {
                        self.correlations.remove(0);
                    }
                    self.correlations.push(link);
                    self.stats.correlations_found += 1;
                }
            }
            // Update co-occurrence
            self.cooccurrence.record(existing.syscall_nr, event.syscall_nr);
        }

        self.window.add(event);
    }

    /// Recent correlations
    pub fn recent_correlations(&self, max: usize) -> &[CorrelationLink] {
        let start = if self.correlations.len() > max {
            self.correlations.len() - max
        } else {
            0
        };
        &self.correlations[start..]
    }

    /// Co-occurrence correlation
    pub fn syscall_correlation(&self, a: u32, b: u32) -> f64 {
        self.cooccurrence.correlation(a, b)
    }

    /// Stats
    pub fn stats(&self) -> &BridgeCorrelationStats {
        &self.stats
    }
}

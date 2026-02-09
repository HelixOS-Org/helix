//! # Holistic Event Correlator
//!
//! System-wide event correlation and complex event processing:
//! - Event stream management
//! - Pattern matching on event sequences
//! - Root cause correlation
//! - Event aggregation windows
//! - Causal chain detection

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// EVENT TYPES
// ============================================================================

/// Event source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSource {
    /// Scheduler
    Scheduler,
    /// Memory subsystem
    Memory,
    /// I/O subsystem
    Io,
    /// Network
    Network,
    /// IPC
    Ipc,
    /// Security
    Security,
    /// Power management
    Power,
    /// Hardware
    Hardware,
    /// Application
    Application,
}

/// Event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    /// Debug/trace
    Debug,
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

/// An event in the system
#[derive(Debug, Clone)]
pub struct SystemEvent {
    /// Unique event id
    pub id: u64,
    /// Source
    pub source: EventSource,
    /// Severity
    pub severity: EventSeverity,
    /// Event type code
    pub event_code: u32,
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Affected entity (pid, device id, etc.)
    pub entity: u64,
    /// Key-value data (encoded as hash->value)
    pub data: LinearMap<u64, 64>,
}

impl SystemEvent {
    pub fn new(
        id: u64,
        source: EventSource,
        severity: EventSeverity,
        event_code: u32,
        timestamp: u64,
        entity: u64,
    ) -> Self {
        Self {
            id,
            source,
            severity,
            event_code,
            timestamp,
            entity,
            data: LinearMap::new(),
        }
    }

    /// Add data
    #[inline(always)]
    pub fn with_data(mut self, key: u64, value: u64) -> Self {
        self.data.insert(key, value);
        self
    }
}

// ============================================================================
// EVENT PATTERN
// ============================================================================

/// Pattern match condition
#[derive(Debug, Clone)]
pub struct PatternCondition {
    /// Source filter
    pub source: Option<EventSource>,
    /// Severity minimum
    pub min_severity: Option<EventSeverity>,
    /// Event code filter
    pub event_code: Option<u32>,
    /// Entity filter
    pub entity: Option<u64>,
}

impl PatternCondition {
    pub fn new() -> Self {
        Self {
            source: None,
            min_severity: None,
            event_code: None,
            entity: None,
        }
    }

    /// Match event
    pub fn matches(&self, event: &SystemEvent) -> bool {
        if let Some(src) = self.source {
            if event.source != src {
                return false;
            }
        }
        if let Some(min_sev) = self.min_severity {
            if event.severity < min_sev {
                return false;
            }
        }
        if let Some(code) = self.event_code {
            if event.event_code != code {
                return false;
            }
        }
        if let Some(ent) = self.entity {
            if event.entity != ent {
                return false;
            }
        }
        true
    }
}

/// Event pattern (sequence of conditions with time constraints)
#[derive(Debug, Clone)]
pub struct EventPattern {
    /// Pattern id
    pub id: u64,
    /// Conditions (must match in order)
    pub conditions: Vec<PatternCondition>,
    /// Maximum time window (ns)
    pub window_ns: u64,
    /// Description
    pub description: String,
    /// Match count
    pub match_count: u64,
}

impl EventPattern {
    pub fn new(id: u64, window_ns: u64, description: String) -> Self {
        Self {
            id,
            conditions: Vec::new(),
            window_ns,
            description,
            match_count: 0,
        }
    }

    #[inline(always)]
    pub fn add_condition(&mut self, condition: PatternCondition) {
        self.conditions.push(condition);
    }
}

// ============================================================================
// CORRELATION
// ============================================================================

/// Correlated event group
#[derive(Debug, Clone)]
pub struct CorrelatedGroup {
    /// Group id
    pub id: u64,
    /// Events in this group
    pub event_ids: Vec<u64>,
    /// Root cause event id
    pub root_cause: Option<u64>,
    /// Time span
    pub start_time: u64,
    pub end_time: u64,
    /// Matched pattern
    pub pattern_id: Option<u64>,
}

impl CorrelatedGroup {
    pub fn new(id: u64, start_time: u64) -> Self {
        Self {
            id,
            event_ids: Vec::new(),
            root_cause: None,
            start_time,
            end_time: start_time,
            pattern_id: None,
        }
    }

    /// Duration
    #[inline(always)]
    pub fn duration_ns(&self) -> u64 {
        self.end_time.saturating_sub(self.start_time)
    }

    /// Event count
    #[inline(always)]
    pub fn event_count(&self) -> usize {
        self.event_ids.len()
    }
}

// ============================================================================
// AGGREGATION WINDOW
// ============================================================================

/// Time-based aggregation window
#[derive(Debug)]
pub struct AggregationWindow {
    /// Window duration (ns)
    pub duration_ns: u64,
    /// Current window start
    pub window_start: u64,
    /// Events in current window
    pub events: Vec<SystemEvent>,
    /// Per-source counts
    source_counts: BTreeMap<u8, u64>,
    /// Per-severity counts
    severity_counts: BTreeMap<u8, u64>,
}

impl AggregationWindow {
    pub fn new(duration_ns: u64) -> Self {
        Self {
            duration_ns,
            window_start: 0,
            events: Vec::new(),
            source_counts: BTreeMap::new(),
            severity_counts: BTreeMap::new(),
        }
    }

    /// Ingest event
    pub fn ingest(&mut self, event: SystemEvent) {
        if self.events.is_empty() {
            self.window_start = event.timestamp;
        }
        // Check window
        if event.timestamp.saturating_sub(self.window_start) >= self.duration_ns {
            self.flush();
            self.window_start = event.timestamp;
        }
        *self.source_counts.entry(event.source as u8).or_insert(0) += 1;
        *self.severity_counts.entry(event.severity as u8).or_insert(0) += 1;
        self.events.push(event);
    }

    /// Flush window
    #[inline]
    pub fn flush(&mut self) {
        self.events.clear();
        self.source_counts.clear();
        self.severity_counts.clear();
    }

    /// Event count in current window
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.events.len()
    }

    /// Event rate (events per second)
    #[inline]
    pub fn rate(&self) -> f64 {
        if self.events.is_empty() {
            return 0.0;
        }
        let elapsed = self.events.last().unwrap().timestamp.saturating_sub(self.window_start);
        if elapsed == 0 {
            return 0.0;
        }
        self.events.len() as f64 / (elapsed as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// EVENT ENGINE
// ============================================================================

/// Event stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticEventStats {
    /// Total events processed
    pub total_events: u64,
    /// Patterns registered
    pub pattern_count: usize,
    /// Correlated groups
    pub correlation_groups: usize,
    /// Current event rate
    pub event_rate: f64,
}

/// Holistic event correlator
pub struct HolisticEventEngine {
    /// Patterns
    patterns: Vec<EventPattern>,
    /// Correlated groups
    groups: Vec<CorrelatedGroup>,
    /// Aggregation window
    window: AggregationWindow,
    /// Next ids
    next_event_id: u64,
    next_group_id: u64,
    /// Stats
    stats: HolisticEventStats,
}

impl HolisticEventEngine {
    pub fn new(window_ns: u64) -> Self {
        Self {
            patterns: Vec::new(),
            groups: Vec::new(),
            window: AggregationWindow::new(window_ns),
            next_event_id: 1,
            next_group_id: 1,
            stats: HolisticEventStats::default(),
        }
    }

    /// Register pattern
    #[inline]
    pub fn register_pattern(&mut self, mut pattern: EventPattern) {
        pattern.id = self.patterns.len() as u64 + 1;
        self.patterns.push(pattern);
        self.update_stats();
    }

    /// Emit event
    pub fn emit(&mut self, mut event: SystemEvent) {
        event.id = self.next_event_id;
        self.next_event_id += 1;
        self.stats.total_events += 1;

        // Check against patterns (simple single-event match for now)
        for pattern in &mut self.patterns {
            if !pattern.conditions.is_empty() && pattern.conditions[0].matches(&event) {
                pattern.match_count += 1;
            }
        }

        self.window.ingest(event);
        self.stats.event_rate = self.window.rate();
    }

    /// Create correlation group
    #[inline]
    pub fn correlate(&mut self, event_ids: Vec<u64>, root_cause: Option<u64>, now: u64) -> u64 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        let mut group = CorrelatedGroup::new(id, now);
        group.event_ids = event_ids;
        group.root_cause = root_cause;
        self.groups.push(group);
        self.update_stats();
        id
    }

    /// Get recent groups
    #[inline]
    pub fn recent_groups(&self, max: usize) -> &[CorrelatedGroup] {
        let start = if self.groups.len() > max {
            self.groups.len() - max
        } else {
            0
        };
        &self.groups[start..]
    }

    fn update_stats(&mut self) {
        self.stats.pattern_count = self.patterns.len();
        self.stats.correlation_groups = self.groups.len();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticEventStats {
        &self.stats
    }
}

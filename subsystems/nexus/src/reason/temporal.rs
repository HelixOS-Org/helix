//! # Temporal Reasoning
//!
//! Reasons about time-based events and sequences.
//! Implements temporal logic and interval algebra.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// TEMPORAL TYPES
// ============================================================================

/// Time point
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimePoint(pub u64);

/// Time interval
#[derive(Debug, Clone, Copy)]
pub struct TimeInterval {
    /// Start
    pub start: TimePoint,
    /// End
    pub end: TimePoint,
}

/// Temporal event
#[derive(Debug, Clone)]
pub struct TemporalEvent {
    /// Event ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Interval
    pub interval: TimeInterval,
    /// Properties
    pub properties: BTreeMap<String, String>,
    /// Created
    pub created: Timestamp,
}

/// Allen's interval relation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalRelation {
    Before,
    After,
    Meets,
    MetBy,
    Overlaps,
    OverlappedBy,
    Starts,
    StartedBy,
    During,
    Contains,
    Finishes,
    FinishedBy,
    Equals,
}

/// Temporal constraint
#[derive(Debug, Clone)]
pub struct TemporalConstraint {
    /// Constraint ID
    pub id: u64,
    /// Event A
    pub event_a: u64,
    /// Event B
    pub event_b: u64,
    /// Allowed relations
    pub relations: Vec<IntervalRelation>,
}

/// Temporal pattern
#[derive(Debug, Clone)]
pub struct TemporalPattern {
    /// Pattern ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Event sequence
    pub sequence: Vec<PatternEvent>,
    /// Constraints
    pub constraints: Vec<SequenceConstraint>,
}

/// Pattern event
#[derive(Debug, Clone)]
pub struct PatternEvent {
    /// Name pattern
    pub name_pattern: String,
    /// Required properties
    pub properties: BTreeMap<String, String>,
}

/// Sequence constraint
#[derive(Debug, Clone)]
pub struct SequenceConstraint {
    /// From index
    pub from: usize,
    /// To index
    pub to: usize,
    /// Maximum gap
    pub max_gap: Option<u64>,
}

/// Temporal query result
#[derive(Debug, Clone)]
pub struct TemporalQuery {
    /// Query type
    pub query_type: QueryType,
    /// Events found
    pub events: Vec<u64>,
    /// Intervals
    pub intervals: Vec<TimeInterval>,
}

/// Query type
#[derive(Debug, Clone)]
pub enum QueryType {
    Before(TimePoint),
    After(TimePoint),
    Between(TimePoint, TimePoint),
    RelatedTo(u64, IntervalRelation),
    MatchingPattern(u64),
}

// ============================================================================
// TEMPORAL REASONER
// ============================================================================

/// Temporal reasoner
pub struct TemporalReasoner {
    /// Events
    events: BTreeMap<u64, TemporalEvent>,
    /// Constraints
    constraints: Vec<TemporalConstraint>,
    /// Patterns
    patterns: BTreeMap<u64, TemporalPattern>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: TemporalConfig,
    /// Statistics
    stats: TemporalStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct TemporalConfig {
    /// Maximum events
    pub max_events: usize,
    /// Time tolerance
    pub tolerance: u64,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            max_events: 10000,
            tolerance: 1000, // 1 microsecond
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct TemporalStats {
    /// Events tracked
    pub events_tracked: u64,
    /// Constraints checked
    pub constraints_checked: u64,
    /// Patterns matched
    pub patterns_matched: u64,
}

impl TemporalReasoner {
    /// Create new reasoner
    pub fn new(config: TemporalConfig) -> Self {
        Self {
            events: BTreeMap::new(),
            constraints: Vec::new(),
            patterns: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: TemporalStats::default(),
        }
    }

    /// Add event
    pub fn add_event(
        &mut self,
        name: &str,
        start: TimePoint,
        end: TimePoint,
        properties: BTreeMap<String, String>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let event = TemporalEvent {
            id,
            name: name.into(),
            interval: TimeInterval { start, end },
            properties,
            created: Timestamp::now(),
        };

        self.events.insert(id, event);
        self.stats.events_tracked += 1;

        id
    }

    /// Add point event
    pub fn add_point_event(&mut self, name: &str, time: TimePoint) -> u64 {
        self.add_event(name, time, time, BTreeMap::new())
    }

    /// Get event
    pub fn get_event(&self, id: u64) -> Option<&TemporalEvent> {
        self.events.get(&id)
    }

    /// Compute interval relation
    pub fn relation(&self, a: u64, b: u64) -> Option<IntervalRelation> {
        let event_a = self.events.get(&a)?;
        let event_b = self.events.get(&b)?;

        Some(self.compute_relation(&event_a.interval, &event_b.interval))
    }

    fn compute_relation(&self, a: &TimeInterval, b: &TimeInterval) -> IntervalRelation {
        let tol = self.config.tolerance;

        // Before: a ends before b starts
        if a.end.0 + tol < b.start.0 {
            return IntervalRelation::Before;
        }

        // After: a starts after b ends
        if a.start.0 > b.end.0 + tol {
            return IntervalRelation::After;
        }

        // Meets: a ends when b starts
        if (a.end.0 as i64 - b.start.0 as i64).unsigned_abs() <= tol {
            return IntervalRelation::Meets;
        }

        // Met by: a starts when b ends
        if (a.start.0 as i64 - b.end.0 as i64).unsigned_abs() <= tol {
            return IntervalRelation::MetBy;
        }

        // Equals
        if (a.start.0 as i64 - b.start.0 as i64).unsigned_abs() <= tol &&
           (a.end.0 as i64 - b.end.0 as i64).unsigned_abs() <= tol {
            return IntervalRelation::Equals;
        }

        // Starts: same start, a ends before b
        if (a.start.0 as i64 - b.start.0 as i64).unsigned_abs() <= tol && a.end.0 < b.end.0 {
            return IntervalRelation::Starts;
        }

        // Started by: same start, a ends after b
        if (a.start.0 as i64 - b.start.0 as i64).unsigned_abs() <= tol && a.end.0 > b.end.0 {
            return IntervalRelation::StartedBy;
        }

        // Finishes: same end, a starts after b
        if (a.end.0 as i64 - b.end.0 as i64).unsigned_abs() <= tol && a.start.0 > b.start.0 {
            return IntervalRelation::Finishes;
        }

        // Finished by: same end, a starts before b
        if (a.end.0 as i64 - b.end.0 as i64).unsigned_abs() <= tol && a.start.0 < b.start.0 {
            return IntervalRelation::FinishedBy;
        }

        // During: a is contained in b
        if a.start.0 > b.start.0 && a.end.0 < b.end.0 {
            return IntervalRelation::During;
        }

        // Contains: b is contained in a
        if a.start.0 < b.start.0 && a.end.0 > b.end.0 {
            return IntervalRelation::Contains;
        }

        // Overlaps: a starts before b and ends during b
        if a.start.0 < b.start.0 && a.end.0 > b.start.0 && a.end.0 < b.end.0 {
            return IntervalRelation::Overlaps;
        }

        // Overlapped by: b starts before a and ends during a
        if b.start.0 < a.start.0 && b.end.0 > a.start.0 && b.end.0 < a.end.0 {
            return IntervalRelation::OverlappedBy;
        }

        IntervalRelation::Equals
    }

    /// Add constraint
    pub fn add_constraint(
        &mut self,
        event_a: u64,
        event_b: u64,
        relations: Vec<IntervalRelation>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let constraint = TemporalConstraint {
            id,
            event_a,
            event_b,
            relations,
        };

        self.constraints.push(constraint);
        id
    }

    /// Check constraints
    pub fn check_constraints(&mut self) -> Vec<(u64, bool)> {
        self.stats.constraints_checked += self.constraints.len() as u64;

        self.constraints.iter()
            .filter_map(|c| {
                let rel = self.relation(c.event_a, c.event_b)?;
                Some((c.id, c.relations.contains(&rel)))
            })
            .collect()
    }

    /// Add pattern
    pub fn add_pattern(&mut self, name: &str, sequence: Vec<PatternEvent>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let pattern = TemporalPattern {
            id,
            name: name.into(),
            sequence,
            constraints: Vec::new(),
        };

        self.patterns.insert(id, pattern);
        id
    }

    /// Find pattern matches
    pub fn find_pattern(&mut self, pattern_id: u64) -> Vec<Vec<u64>> {
        let pattern = match self.patterns.get(&pattern_id) {
            Some(p) => p.clone(),
            None => return Vec::new(),
        };

        let mut matches = Vec::new();
        let mut current_match = Vec::new();

        // Get events sorted by start time
        let mut events: Vec<&TemporalEvent> = self.events.values().collect();
        events.sort_by(|a, b| a.interval.start.cmp(&b.interval.start));

        self.find_pattern_recursive(&pattern, &events, 0, &mut current_match, &mut matches);

        if !matches.is_empty() {
            self.stats.patterns_matched += 1;
        }

        matches
    }

    fn find_pattern_recursive(
        &self,
        pattern: &TemporalPattern,
        events: &[&TemporalEvent],
        pattern_idx: usize,
        current: &mut Vec<u64>,
        matches: &mut Vec<Vec<u64>>,
    ) {
        if pattern_idx >= pattern.sequence.len() {
            matches.push(current.clone());
            return;
        }

        let pat_event = &pattern.sequence[pattern_idx];

        for event in events {
            if self.event_matches_pattern(event, pat_event) {
                // Check ordering if not first
                if let Some(&prev_id) = current.last() {
                    if let Some(prev) = self.events.get(&prev_id) {
                        if event.interval.start.0 < prev.interval.end.0 {
                            continue;
                        }
                    }
                }

                current.push(event.id);
                self.find_pattern_recursive(pattern, events, pattern_idx + 1, current, matches);
                current.pop();
            }
        }
    }

    fn event_matches_pattern(&self, event: &TemporalEvent, pattern: &PatternEvent) -> bool {
        // Check name pattern
        if !pattern.name_pattern.is_empty() && !event.name.contains(&pattern.name_pattern) {
            return false;
        }

        // Check properties
        for (key, val) in &pattern.properties {
            if event.properties.get(key) != Some(val) {
                return false;
            }
        }

        true
    }

    /// Query before
    pub fn before(&self, time: TimePoint) -> Vec<&TemporalEvent> {
        self.events.values()
            .filter(|e| e.interval.end.0 < time.0)
            .collect()
    }

    /// Query after
    pub fn after(&self, time: TimePoint) -> Vec<&TemporalEvent> {
        self.events.values()
            .filter(|e| e.interval.start.0 > time.0)
            .collect()
    }

    /// Query between
    pub fn between(&self, start: TimePoint, end: TimePoint) -> Vec<&TemporalEvent> {
        self.events.values()
            .filter(|e| e.interval.start.0 >= start.0 && e.interval.end.0 <= end.0)
            .collect()
    }

    /// Query overlapping
    pub fn overlapping(&self, interval: TimeInterval) -> Vec<&TemporalEvent> {
        self.events.values()
            .filter(|e| {
                e.interval.start.0 < interval.end.0 && e.interval.end.0 > interval.start.0
            })
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &TemporalStats {
        &self.stats
    }
}

impl Default for TemporalReasoner {
    fn default() -> Self {
        Self::new(TemporalConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_event() {
        let mut reasoner = TemporalReasoner::default();

        let id = reasoner.add_event("test", TimePoint(0), TimePoint(10), BTreeMap::new());
        assert!(reasoner.get_event(id).is_some());
    }

    #[test]
    fn test_before_relation() {
        let mut reasoner = TemporalReasoner::default();

        let a = reasoner.add_event("a", TimePoint(0), TimePoint(5), BTreeMap::new());
        let b = reasoner.add_event("b", TimePoint(10), TimePoint(15), BTreeMap::new());

        let rel = reasoner.relation(a, b).unwrap();
        assert_eq!(rel, IntervalRelation::Before);
    }

    #[test]
    fn test_during_relation() {
        let mut reasoner = TemporalReasoner::default();

        let a = reasoner.add_event("a", TimePoint(5), TimePoint(10), BTreeMap::new());
        let b = reasoner.add_event("b", TimePoint(0), TimePoint(20), BTreeMap::new());

        let rel = reasoner.relation(a, b).unwrap();
        assert_eq!(rel, IntervalRelation::During);
    }

    #[test]
    fn test_constraint() {
        let mut reasoner = TemporalReasoner::default();

        let a = reasoner.add_event("a", TimePoint(0), TimePoint(5), BTreeMap::new());
        let b = reasoner.add_event("b", TimePoint(10), TimePoint(15), BTreeMap::new());

        reasoner.add_constraint(a, b, vec![IntervalRelation::Before]);

        let results = reasoner.check_constraints();
        assert!(results.iter().all(|(_, satisfied)| *satisfied));
    }

    #[test]
    fn test_query_before() {
        let mut reasoner = TemporalReasoner::default();

        reasoner.add_event("early", TimePoint(0), TimePoint(5), BTreeMap::new());
        reasoner.add_event("late", TimePoint(20), TimePoint(25), BTreeMap::new());

        let before = reasoner.before(TimePoint(10));
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].name, "early");
    }

    #[test]
    fn test_pattern() {
        let mut reasoner = TemporalReasoner::default();

        reasoner.add_event("start", TimePoint(0), TimePoint(5), BTreeMap::new());
        reasoner.add_event("middle", TimePoint(10), TimePoint(15), BTreeMap::new());
        reasoner.add_event("end", TimePoint(20), TimePoint(25), BTreeMap::new());

        let pattern_id = reasoner.add_pattern("sequence", vec![
            PatternEvent { name_pattern: "start".into(), properties: BTreeMap::new() },
            PatternEvent { name_pattern: "middle".into(), properties: BTreeMap::new() },
            PatternEvent { name_pattern: "end".into(), properties: BTreeMap::new() },
        ]);

        let matches = reasoner.find_pattern(pattern_id);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].len(), 3);
    }
}

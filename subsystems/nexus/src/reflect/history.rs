//! # Reflection History
//!
//! Tracks and analyzes historical performance.
//! Enables learning from past decisions and outcomes.
//!
//! Part of Year 2 COGNITION - Reflection Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// HISTORY TYPES
// ============================================================================

/// History entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Entry ID
    pub id: u64,
    /// Event type
    pub event_type: EventType,
    /// Description
    pub description: String,
    /// Context
    pub context: BTreeMap<String, ContextValue>,
    /// Outcome
    pub outcome: Option<Outcome>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Tags
    pub tags: Vec<String>,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventType {
    Decision,
    Action,
    Observation,
    Learning,
    Error,
    Recovery,
    Milestone,
}

/// Context value
#[derive(Debug, Clone)]
pub enum ContextValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    List(Vec<String>),
}

/// Outcome
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Success
    pub success: bool,
    /// Score
    pub score: f64,
    /// Metrics
    pub metrics: BTreeMap<String, f64>,
    /// Notes
    pub notes: String,
}

/// History query
#[derive(Debug, Clone)]
pub struct HistoryQuery {
    /// Event types
    pub event_types: Option<Vec<EventType>>,
    /// Time range
    pub time_range: Option<(Timestamp, Timestamp)>,
    /// Tags
    pub tags: Option<Vec<String>>,
    /// Limit
    pub limit: Option<usize>,
    /// Offset
    pub offset: Option<usize>,
}

/// History analysis
#[derive(Debug, Clone)]
pub struct HistoryAnalysis {
    /// Period
    pub period: (Timestamp, Timestamp),
    /// Event counts
    pub event_counts: BTreeMap<String, u64>,
    /// Success rate
    pub success_rate: f64,
    /// Average score
    pub average_score: f64,
    /// Trends
    pub trends: Vec<Trend>,
    /// Patterns
    pub patterns: Vec<Pattern>,
}

/// Trend
#[derive(Debug, Clone)]
pub struct Trend {
    /// Metric name
    pub metric: String,
    /// Direction
    pub direction: TrendDirection,
    /// Magnitude
    pub magnitude: f64,
    /// Confidence
    pub confidence: f64,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Volatile,
}

/// Pattern
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Frequency
    pub frequency: f64,
    /// Correlation
    pub correlation: Option<f64>,
}

// ============================================================================
// HISTORY TRACKER
// ============================================================================

/// History tracker
pub struct HistoryTracker {
    /// Entries
    entries: BTreeMap<u64, HistoryEntry>,
    /// Index by type
    by_type: BTreeMap<EventType, Vec<u64>>,
    /// Index by tag
    by_tag: BTreeMap<String, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: HistoryConfig,
    /// Statistics
    stats: HistoryStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct HistoryConfig {
    /// Maximum entries
    pub max_entries: usize,
    /// Auto-cleanup
    pub auto_cleanup: bool,
    /// Retention period (ns)
    pub retention_ns: u64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            auto_cleanup: true,
            retention_ns: 86400_000_000_000 * 30, // 30 days
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct HistoryStats {
    /// Total entries
    pub total_entries: u64,
    /// Queries performed
    pub queries_performed: u64,
    /// Analyses performed
    pub analyses_performed: u64,
}

impl HistoryTracker {
    /// Create new tracker
    pub fn new(config: HistoryConfig) -> Self {
        Self {
            entries: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: HistoryStats::default(),
        }
    }

    /// Record entry
    pub fn record(
        &mut self,
        event_type: EventType,
        description: &str,
        context: BTreeMap<String, ContextValue>,
        tags: Vec<String>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let entry = HistoryEntry {
            id,
            event_type,
            description: description.into(),
            context,
            outcome: None,
            timestamp: Timestamp::now(),
            tags: tags.clone(),
        };

        // Index
        self.by_type.entry(event_type).or_insert_with(Vec::new).push(id);

        for tag in &tags {
            self.by_tag.entry(tag.clone()).or_insert_with(Vec::new).push(id);
        }

        self.entries.insert(id, entry);
        self.stats.total_entries += 1;

        // Cleanup if needed
        if self.config.auto_cleanup && self.entries.len() > self.config.max_entries {
            self.cleanup();
        }

        id
    }

    /// Set outcome
    pub fn set_outcome(&mut self, id: u64, outcome: Outcome) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.outcome = Some(outcome);
        }
    }

    /// Query entries
    pub fn query(&mut self, query: &HistoryQuery) -> Vec<&HistoryEntry> {
        self.stats.queries_performed += 1;

        let mut results: Vec<&HistoryEntry> = self.entries.values()
            .filter(|e| self.matches_query(e, query))
            .collect();

        // Sort by timestamp descending
        results.sort_by(|a, b| b.timestamp.0.cmp(&a.timestamp.0));

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        results.into_iter()
            .skip(offset)
            .take(limit)
            .collect()
    }

    fn matches_query(&self, entry: &HistoryEntry, query: &HistoryQuery) -> bool {
        // Check event type
        if let Some(ref types) = query.event_types {
            if !types.contains(&entry.event_type) {
                return false;
            }
        }

        // Check time range
        if let Some((start, end)) = query.time_range {
            if entry.timestamp.0 < start.0 || entry.timestamp.0 > end.0 {
                return false;
            }
        }

        // Check tags
        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|t| entry.tags.contains(t)) {
                return false;
            }
        }

        true
    }

    /// Get entry
    pub fn get(&self, id: u64) -> Option<&HistoryEntry> {
        self.entries.get(&id)
    }

    /// Analyze history
    pub fn analyze(&mut self, start: Timestamp, end: Timestamp) -> HistoryAnalysis {
        self.stats.analyses_performed += 1;

        let entries: Vec<&HistoryEntry> = self.entries.values()
            .filter(|e| e.timestamp.0 >= start.0 && e.timestamp.0 <= end.0)
            .collect();

        // Event counts
        let mut event_counts = BTreeMap::new();
        for entry in &entries {
            let key = format!("{:?}", entry.event_type);
            *event_counts.entry(key).or_insert(0u64) += 1;
        }

        // Success rate
        let with_outcome: Vec<_> = entries.iter()
            .filter_map(|e| e.outcome.as_ref())
            .collect();

        let success_count = with_outcome.iter().filter(|o| o.success).count();
        let success_rate = if with_outcome.is_empty() {
            0.0
        } else {
            success_count as f64 / with_outcome.len() as f64
        };

        // Average score
        let scores: Vec<f64> = with_outcome.iter().map(|o| o.score).collect();
        let average_score = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        };

        // Analyze trends
        let trends = self.analyze_trends(&entries);

        // Find patterns
        let patterns = self.find_patterns(&entries);

        HistoryAnalysis {
            period: (start, end),
            event_counts,
            success_rate,
            average_score,
            trends,
            patterns,
        }
    }

    fn analyze_trends(&self, entries: &[&HistoryEntry]) -> Vec<Trend> {
        let mut trends = Vec::new();

        // Analyze success rate trend over time
        if entries.len() >= 10 {
            let mid = entries.len() / 2;
            let first_half: Vec<_> = entries[..mid].iter()
                .filter_map(|e| e.outcome.as_ref())
                .collect();
            let second_half: Vec<_> = entries[mid..].iter()
                .filter_map(|e| e.outcome.as_ref())
                .collect();

            if !first_half.is_empty() && !second_half.is_empty() {
                let first_rate = first_half.iter().filter(|o| o.success).count() as f64 / first_half.len() as f64;
                let second_rate = second_half.iter().filter(|o| o.success).count() as f64 / second_half.len() as f64;

                let diff = second_rate - first_rate;

                let direction = if diff > 0.1 {
                    TrendDirection::Increasing
                } else if diff < -0.1 {
                    TrendDirection::Decreasing
                } else {
                    TrendDirection::Stable
                };

                trends.push(Trend {
                    metric: "success_rate".into(),
                    direction,
                    magnitude: diff.abs(),
                    confidence: 0.8,
                });
            }
        }

        trends
    }

    fn find_patterns(&self, entries: &[&HistoryEntry]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Find repeated sequences
        let mut event_sequences = BTreeMap::new();

        for window in entries.windows(3) {
            let seq: Vec<String> = window.iter()
                .map(|e| format!("{:?}", e.event_type))
                .collect();
            let key = seq.join("->");

            *event_sequences.entry(key).or_insert(0) += 1;
        }

        for (seq, count) in event_sequences {
            if count >= 3 {
                let frequency = count as f64 / entries.len() as f64;

                patterns.push(Pattern {
                    name: format!("Sequence: {}", seq),
                    description: format!("Event sequence {} occurs {} times", seq, count),
                    frequency,
                    correlation: None,
                });
            }
        }

        // Find error-recovery patterns
        let mut error_recovery_count = 0;
        for window in entries.windows(2) {
            if window[0].event_type == EventType::Error &&
               window[1].event_type == EventType::Recovery {
                error_recovery_count += 1;
            }
        }

        if error_recovery_count > 0 {
            patterns.push(Pattern {
                name: "Error-Recovery".into(),
                description: format!("{} error-recovery sequences detected", error_recovery_count),
                frequency: error_recovery_count as f64 / entries.len() as f64,
                correlation: Some(1.0),
            });
        }

        patterns
    }

    fn cleanup(&mut self) {
        let now = Timestamp::now().0;
        let cutoff = now.saturating_sub(self.config.retention_ns);

        // Remove old entries
        let to_remove: Vec<u64> = self.entries.iter()
            .filter(|(_, e)| e.timestamp.0 < cutoff)
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            self.entries.remove(&id);
        }

        // Rebuild indexes
        self.by_type.clear();
        self.by_tag.clear();

        for (id, entry) in &self.entries {
            self.by_type.entry(entry.event_type).or_insert_with(Vec::new).push(*id);

            for tag in &entry.tags {
                self.by_tag.entry(tag.clone()).or_insert_with(Vec::new).push(*id);
            }
        }
    }

    /// Get by type
    pub fn by_type(&self, event_type: EventType) -> Vec<&HistoryEntry> {
        self.by_type.get(&event_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.entries.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get by tag
    pub fn by_tag(&self, tag: &str) -> Vec<&HistoryEntry> {
        self.by_tag.get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.entries.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get statistics
    pub fn stats(&self) -> &HistoryStats {
        &self.stats
    }
}

impl Default for HistoryTracker {
    fn default() -> Self {
        Self::new(HistoryConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record() {
        let mut tracker = HistoryTracker::default();

        let id = tracker.record(
            EventType::Decision,
            "Made a decision",
            BTreeMap::new(),
            vec!["important".into()],
        );

        assert!(tracker.get(id).is_some());
    }

    #[test]
    fn test_set_outcome() {
        let mut tracker = HistoryTracker::default();

        let id = tracker.record(
            EventType::Action,
            "Performed action",
            BTreeMap::new(),
            Vec::new(),
        );

        tracker.set_outcome(id, Outcome {
            success: true,
            score: 0.9,
            metrics: BTreeMap::new(),
            notes: "Good".into(),
        });

        let entry = tracker.get(id).unwrap();
        assert!(entry.outcome.is_some());
        assert!(entry.outcome.as_ref().unwrap().success);
    }

    #[test]
    fn test_query() {
        let mut tracker = HistoryTracker::default();

        tracker.record(EventType::Decision, "D1", BTreeMap::new(), Vec::new());
        tracker.record(EventType::Action, "A1", BTreeMap::new(), Vec::new());
        tracker.record(EventType::Decision, "D2", BTreeMap::new(), Vec::new());

        let results = tracker.query(&HistoryQuery {
            event_types: Some(vec![EventType::Decision]),
            time_range: None,
            tags: None,
            limit: None,
            offset: None,
        });

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_by_tag() {
        let mut tracker = HistoryTracker::default();

        tracker.record(
            EventType::Action,
            "Tagged action",
            BTreeMap::new(),
            vec!["test".into()],
        );

        let results = tracker.by_tag("test");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_analyze() {
        let mut tracker = HistoryTracker::default();

        for i in 0..10 {
            let id = tracker.record(
                EventType::Action,
                "Action",
                BTreeMap::new(),
                Vec::new(),
            );

            tracker.set_outcome(id, Outcome {
                success: i % 2 == 0,
                score: (i as f64) / 10.0,
                metrics: BTreeMap::new(),
                notes: String::new(),
            });
        }

        let analysis = tracker.analyze(Timestamp(0), Timestamp(u64::MAX));

        assert_eq!(analysis.success_rate, 0.5);
        assert!(analysis.average_score > 0.0);
    }
}

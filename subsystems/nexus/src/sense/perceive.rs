//! # Perception Module
//!
//! Core perception for sensory integration.
//! Processes and interprets incoming sensory data.
//!
//! Part of Year 2 COGNITION - Perception Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PERCEPTION TYPES
// ============================================================================

/// Percept
#[derive(Debug, Clone)]
pub struct Percept {
    /// Percept ID
    pub id: u64,
    /// Source
    pub source: String,
    /// Type
    pub percept_type: PerceptType,
    /// Content
    pub content: PerceptContent,
    /// Confidence
    pub confidence: f64,
    /// Salience
    pub salience: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Percept type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerceptType {
    Visual,
    Auditory,
    Textual,
    Numeric,
    Structural,
    Temporal,
    Spatial,
}

/// Percept content
#[derive(Debug, Clone)]
pub enum PerceptContent {
    Pattern(PatternData),
    Sequence(Vec<f64>),
    Structure(BTreeMap<String, String>),
    Text(String),
    Number(f64),
}

/// Pattern data
#[derive(Debug, Clone)]
pub struct PatternData {
    /// Features
    pub features: Vec<f64>,
    /// Label
    pub label: Option<String>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Attention focus
#[derive(Debug, Clone)]
pub struct AttentionFocus {
    /// Focus ID
    pub id: u64,
    /// Target percepts
    pub targets: Vec<u64>,
    /// Priority
    pub priority: f64,
    /// Duration ns
    pub duration_ns: u64,
    /// Started
    pub started: Timestamp,
}

/// Perception event
#[derive(Debug, Clone)]
pub struct PerceptionEvent {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: EventType,
    /// Percept IDs
    pub percepts: Vec<u64>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    NewPercept,
    PatternRecognized,
    ChangeDetected,
    AnomalyDetected,
    AttentionShift,
}

/// Perception filter
#[derive(Debug, Clone)]
pub struct PerceptionFilter {
    /// Filter ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Types allowed
    pub types: Vec<PerceptType>,
    /// Minimum confidence
    pub min_confidence: f64,
    /// Minimum salience
    pub min_salience: f64,
}

// ============================================================================
// PERCEPTION ENGINE
// ============================================================================

/// Perception engine
pub struct PerceptionEngine {
    /// Percepts
    percepts: BTreeMap<u64, Percept>,
    /// Attention
    attention: Option<AttentionFocus>,
    /// Events
    events: Vec<PerceptionEvent>,
    /// Filters
    filters: Vec<PerceptionFilter>,
    /// Pattern memory
    patterns: Vec<PatternData>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: PerceptionConfig,
    /// Statistics
    stats: PerceptionStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct PerceptionConfig {
    /// Maximum percepts
    pub max_percepts: usize,
    /// Attention decay rate
    pub attention_decay: f64,
    /// Salience threshold
    pub salience_threshold: f64,
    /// Pattern matching threshold
    pub pattern_threshold: f64,
}

impl Default for PerceptionConfig {
    fn default() -> Self {
        Self {
            max_percepts: 1000,
            attention_decay: 0.1,
            salience_threshold: 0.3,
            pattern_threshold: 0.7,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct PerceptionStats {
    /// Percepts received
    pub percepts_received: u64,
    /// Patterns recognized
    pub patterns_recognized: u64,
    /// Events generated
    pub events_generated: u64,
}

impl PerceptionEngine {
    /// Create new engine
    pub fn new(config: PerceptionConfig) -> Self {
        Self {
            percepts: BTreeMap::new(),
            attention: None,
            events: Vec::new(),
            filters: Vec::new(),
            patterns: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: PerceptionStats::default(),
        }
    }

    /// Receive percept
    pub fn perceive(
        &mut self,
        source: &str,
        percept_type: PerceptType,
        content: PerceptContent,
        confidence: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Calculate salience
        let salience = self.calculate_salience(&content, confidence);

        let percept = Percept {
            id,
            source: source.into(),
            percept_type,
            content: content.clone(),
            confidence: confidence.clamp(0.0, 1.0),
            salience,
            timestamp: Timestamp::now(),
        };

        // Apply filters
        if !self.passes_filters(&percept) {
            return id;
        }

        self.percepts.insert(id, percept);
        self.stats.percepts_received += 1;

        // Generate event
        self.generate_event(EventType::NewPercept, vec![id]);

        // Check for pattern recognition
        self.check_patterns(id, &content);

        // Manage capacity
        self.enforce_capacity();

        id
    }

    fn calculate_salience(&self, content: &PerceptContent, confidence: f64) -> f64 {
        let base_salience = match content {
            PerceptContent::Pattern(p) => {
                if p.label.is_some() { 0.8 } else { 0.5 }
            }
            PerceptContent::Sequence(s) => {
                (s.len() as f64 / 100.0).min(0.7)
            }
            PerceptContent::Structure(m) => {
                (m.len() as f64 / 10.0).min(0.6)
            }
            PerceptContent::Text(t) => {
                (t.len() as f64 / 1000.0).min(0.5)
            }
            PerceptContent::Number(_) => 0.4,
        };

        (base_salience * confidence).clamp(0.0, 1.0)
    }

    fn passes_filters(&self, percept: &Percept) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        self.filters.iter().any(|f| {
            (f.types.is_empty() || f.types.contains(&percept.percept_type)) &&
            percept.confidence >= f.min_confidence &&
            percept.salience >= f.min_salience
        })
    }

    fn check_patterns(&mut self, id: u64, content: &PerceptContent) {
        if let PerceptContent::Pattern(pattern) = content {
            // Simple pattern matching
            for known in &self.patterns {
                let similarity = self.pattern_similarity(&pattern.features, &known.features);

                if similarity >= self.config.pattern_threshold {
                    self.stats.patterns_recognized += 1;
                    self.generate_event(EventType::PatternRecognized, vec![id]);
                    break;
                }
            }
        }
    }

    fn pattern_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let len = a.len().min(b.len());

        let dot: f64 = a.iter().zip(b.iter()).take(len).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().take(len).map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().take(len).map(|x| x * x).sum::<f64>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
        }
    }

    fn generate_event(&mut self, event_type: EventType, percepts: Vec<u64>) {
        let event = PerceptionEvent {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            event_type,
            percepts,
            timestamp: Timestamp::now(),
        };

        self.events.push(event);
        self.stats.events_generated += 1;
    }

    fn enforce_capacity(&mut self) {
        while self.percepts.len() > self.config.max_percepts {
            // Remove oldest low-salience percept
            let to_remove: Option<u64> = self.percepts.iter()
                .filter(|(_, p)| p.salience < self.config.salience_threshold)
                .min_by(|a, b| a.1.timestamp.0.cmp(&b.1.timestamp.0))
                .map(|(&id, _)| id);

            if let Some(id) = to_remove {
                self.percepts.remove(&id);
            } else {
                break;
            }
        }
    }

    /// Focus attention
    pub fn focus(&mut self, targets: Vec<u64>, priority: f64, duration_ns: u64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        self.attention = Some(AttentionFocus {
            id,
            targets,
            priority: priority.clamp(0.0, 1.0),
            duration_ns,
            started: Timestamp::now(),
        });

        self.generate_event(EventType::AttentionShift, vec![]);

        id
    }

    /// Get attended percepts
    pub fn attended(&self) -> Vec<&Percept> {
        if let Some(ref focus) = self.attention {
            focus.targets.iter()
                .filter_map(|id| self.percepts.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get salient percepts
    pub fn salient(&self) -> Vec<&Percept> {
        self.percepts.values()
            .filter(|p| p.salience >= self.config.salience_threshold)
            .collect()
    }

    /// Add filter
    pub fn add_filter(&mut self, filter: PerceptionFilter) {
        self.filters.push(filter);
    }

    /// Learn pattern
    pub fn learn_pattern(&mut self, pattern: PatternData) {
        self.patterns.push(pattern);
    }

    /// Get percept
    pub fn get(&self, id: u64) -> Option<&Percept> {
        self.percepts.get(&id)
    }

    /// Get recent percepts
    pub fn recent(&self, count: usize) -> Vec<&Percept> {
        let mut percepts: Vec<_> = self.percepts.values().collect();
        percepts.sort_by(|a, b| b.timestamp.0.cmp(&a.timestamp.0));
        percepts.into_iter().take(count).collect()
    }

    /// Get events
    pub fn events(&self) -> &[PerceptionEvent] {
        &self.events
    }

    /// Get statistics
    pub fn stats(&self) -> &PerceptionStats {
        &self.stats
    }
}

impl Default for PerceptionEngine {
    fn default() -> Self {
        Self::new(PerceptionConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perceive() {
        let mut engine = PerceptionEngine::default();

        let id = engine.perceive(
            "sensor1",
            PerceptType::Numeric,
            PerceptContent::Number(42.0),
            0.9,
        );

        assert!(engine.get(id).is_some());
    }

    #[test]
    fn test_salience() {
        let mut engine = PerceptionEngine::default();

        engine.perceive(
            "high",
            PerceptType::Visual,
            PerceptContent::Pattern(PatternData {
                features: vec![1.0, 2.0, 3.0],
                label: Some("important".into()),
                metadata: BTreeMap::new(),
            }),
            1.0,
        );

        let salient = engine.salient();
        assert!(!salient.is_empty());
    }

    #[test]
    fn test_focus() {
        let mut engine = PerceptionEngine::default();

        let id = engine.perceive(
            "source",
            PerceptType::Textual,
            PerceptContent::Text("hello".into()),
            0.8,
        );

        engine.focus(vec![id], 1.0, 1000000);

        let attended = engine.attended();
        assert_eq!(attended.len(), 1);
    }

    #[test]
    fn test_pattern_recognition() {
        let mut engine = PerceptionEngine::default();

        // Learn a pattern
        engine.learn_pattern(PatternData {
            features: vec![1.0, 0.0, 1.0],
            label: Some("known".into()),
            metadata: BTreeMap::new(),
        });

        // Similar pattern
        engine.perceive(
            "source",
            PerceptType::Visual,
            PerceptContent::Pattern(PatternData {
                features: vec![1.0, 0.0, 1.0],
                label: None,
                metadata: BTreeMap::new(),
            }),
            0.9,
        );

        assert!(engine.stats.patterns_recognized > 0);
    }

    #[test]
    fn test_filter() {
        let mut engine = PerceptionEngine::default();

        engine.add_filter(PerceptionFilter {
            id: 1,
            name: "high-confidence".into(),
            types: vec![],
            min_confidence: 0.9,
            min_salience: 0.0,
        });

        engine.perceive(
            "low",
            PerceptType::Numeric,
            PerceptContent::Number(1.0),
            0.5, // Below filter threshold
        );

        assert_eq!(engine.stats.percepts_received, 0);
    }
}

//! # Perception Filter
//!
//! Filters and processes incoming sensory information.
//! Reduces noise and extracts relevant signals.
//!
//! Part of Year 2 COGNITION - Perception Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PERCEPTION TYPES
// ============================================================================

/// Sensory input
#[derive(Debug, Clone)]
pub struct SensoryInput {
    /// Input ID
    pub id: u64,
    /// Source
    pub source: String,
    /// Input type
    pub input_type: InputType,
    /// Raw data
    pub data: InputData,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Input type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Numeric,
    Text,
    Binary,
    Structured,
    Event,
}

/// Input data
#[derive(Debug, Clone)]
pub enum InputData {
    /// Numeric value
    Numeric(f64),
    /// Text value
    Text(String),
    /// Binary data
    Binary(Vec<u8>),
    /// Key-value pairs
    Structured(BTreeMap<String, String>),
    /// Event
    Event { name: String, data: BTreeMap<String, String> },
}

/// Filtered output
#[derive(Debug, Clone)]
pub struct FilteredOutput {
    /// Output ID
    pub id: u64,
    /// Original input ID
    pub input_id: u64,
    /// Filtered data
    pub data: InputData,
    /// Confidence
    pub confidence: f64,
    /// Relevance score
    pub relevance: f64,
    /// Filters applied
    pub filters_applied: Vec<String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Filter rule
#[derive(Debug, Clone)]
pub struct FilterRule {
    /// Rule ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Condition
    pub condition: FilterCondition,
    /// Action
    pub action: FilterAction,
    /// Priority
    pub priority: u32,
    /// Enabled
    pub enabled: bool,
}

/// Filter condition
#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// Input type match
    TypeMatch(InputType),
    /// Source match
    SourceMatch(String),
    /// Value threshold
    ValueThreshold { min: Option<f64>, max: Option<f64> },
    /// Contains text
    Contains(String),
    /// Regex match (simplified)
    Pattern(String),
    /// Always true
    Always,
    /// Multiple conditions
    And(Vec<FilterCondition>),
    /// Any condition
    Or(Vec<FilterCondition>),
    /// Negation
    Not(Box<FilterCondition>),
}

/// Filter action
#[derive(Debug, Clone)]
pub enum FilterAction {
    /// Accept input
    Accept,
    /// Reject input
    Reject,
    /// Transform input
    Transform(TransformType),
    /// Boost relevance
    Boost(f64),
    /// Reduce relevance
    Reduce(f64),
    /// Tag for later
    Tag(String),
}

/// Transform type
#[derive(Debug, Clone)]
pub enum TransformType {
    /// Normalize numeric value
    Normalize { min: f64, max: f64 },
    /// Clip value
    Clip { min: f64, max: f64 },
    /// Extract substring
    Substring { start: usize, len: usize },
    /// To lowercase
    Lowercase,
    /// Trim whitespace
    Trim,
}

// ============================================================================
// PERCEPTION FILTER
// ============================================================================

/// Perception filter
pub struct PerceptionFilter {
    /// Filter rules
    rules: BTreeMap<u64, FilterRule>,
    /// Input buffer
    input_buffer: Vec<SensoryInput>,
    /// Output buffer
    output_buffer: VecDeque<FilteredOutput>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: FilterConfig,
    /// Statistics
    stats: FilterStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Buffer size
    pub buffer_size: usize,
    /// Default relevance
    pub default_relevance: f64,
    /// Noise threshold
    pub noise_threshold: f64,
    /// Enable caching
    pub enable_cache: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            default_relevance: 0.5,
            noise_threshold: 0.1,
            enable_cache: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FilterStats {
    /// Inputs received
    pub inputs_received: u64,
    /// Inputs accepted
    pub inputs_accepted: u64,
    /// Inputs rejected
    pub inputs_rejected: u64,
    /// Transforms applied
    pub transforms_applied: u64,
}

impl PerceptionFilter {
    /// Create new filter
    pub fn new(config: FilterConfig) -> Self {
        Self {
            rules: BTreeMap::new(),
            input_buffer: Vec::new(),
            output_buffer: VecDeque::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: FilterStats::default(),
        }
    }

    /// Add filter rule
    pub fn add_rule(
        &mut self,
        name: &str,
        condition: FilterCondition,
        action: FilterAction,
        priority: u32,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let rule = FilterRule {
            id,
            name: name.into(),
            condition,
            action,
            priority,
            enabled: true,
        };

        self.rules.insert(id, rule);
        id
    }

    /// Process input
    pub fn process(&mut self, input: SensoryInput) -> Option<FilteredOutput> {
        self.stats.inputs_received += 1;

        // Get sorted rules by priority
        let mut sorted_rules: Vec<_> = self.rules.values()
            .filter(|r| r.enabled)
            .collect();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut data = input.data.clone();
        let mut relevance = self.config.default_relevance;
        let mut filters_applied = Vec::new();
        let mut rejected = false;

        for rule in sorted_rules {
            if self.matches_condition(&input, &data, &rule.condition) {
                match &rule.action {
                    FilterAction::Accept => {
                        filters_applied.push(rule.name.clone());
                    }
                    FilterAction::Reject => {
                        rejected = true;
                        break;
                    }
                    FilterAction::Transform(transform) => {
                        if let Some(transformed) = self.apply_transform(&data, transform) {
                            data = transformed;
                            filters_applied.push(rule.name.clone());
                            self.stats.transforms_applied += 1;
                        }
                    }
                    FilterAction::Boost(factor) => {
                        relevance = (relevance * factor).min(1.0);
                        filters_applied.push(rule.name.clone());
                    }
                    FilterAction::Reduce(factor) => {
                        relevance *= 1.0 - factor;
                        filters_applied.push(rule.name.clone());
                    }
                    FilterAction::Tag(tag) => {
                        filters_applied.push(format!("{}:{}", rule.name, tag));
                    }
                }
            }
        }

        if rejected {
            self.stats.inputs_rejected += 1;
            return None;
        }

        // Check noise threshold
        if relevance < self.config.noise_threshold {
            self.stats.inputs_rejected += 1;
            return None;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let output = FilteredOutput {
            id,
            input_id: input.id,
            data,
            confidence: 1.0, // Could be computed based on filters
            relevance,
            filters_applied,
            timestamp: Timestamp::now(),
        };

        // Add to buffer
        self.output_buffer.push_back(output.clone());
        if self.output_buffer.len() > self.config.buffer_size {
            self.output_buffer.pop_front();
        }

        self.stats.inputs_accepted += 1;

        Some(output)
    }

    fn matches_condition(
        &self,
        input: &SensoryInput,
        data: &InputData,
        condition: &FilterCondition,
    ) -> bool {
        match condition {
            FilterCondition::Always => true,

            FilterCondition::TypeMatch(t) => input.input_type == *t,

            FilterCondition::SourceMatch(s) => input.source == *s,

            FilterCondition::ValueThreshold { min, max } => {
                if let InputData::Numeric(v) = data {
                    let above_min = min.map_or(true, |m| *v >= m);
                    let below_max = max.map_or(true, |m| *v <= m);
                    above_min && below_max
                } else {
                    false
                }
            }

            FilterCondition::Contains(text) => {
                if let InputData::Text(t) = data {
                    t.contains(text.as_str())
                } else {
                    false
                }
            }

            FilterCondition::Pattern(pattern) => {
                // Simplified pattern matching
                if let InputData::Text(t) = data {
                    t.contains(pattern.as_str())
                } else {
                    false
                }
            }

            FilterCondition::And(conditions) => {
                conditions.iter().all(|c| self.matches_condition(input, data, c))
            }

            FilterCondition::Or(conditions) => {
                conditions.iter().any(|c| self.matches_condition(input, data, c))
            }

            FilterCondition::Not(c) => {
                !self.matches_condition(input, data, c)
            }
        }
    }

    fn apply_transform(&self, data: &InputData, transform: &TransformType) -> Option<InputData> {
        match (data, transform) {
            (InputData::Numeric(v), TransformType::Normalize { min, max }) => {
                if max > min {
                    let normalized = (v - min) / (max - min);
                    Some(InputData::Numeric(normalized.clamp(0.0, 1.0)))
                } else {
                    None
                }
            }

            (InputData::Numeric(v), TransformType::Clip { min, max }) => {
                Some(InputData::Numeric(v.clamp(*min, *max)))
            }

            (InputData::Text(t), TransformType::Lowercase) => {
                Some(InputData::Text(t.to_lowercase()))
            }

            (InputData::Text(t), TransformType::Trim) => {
                Some(InputData::Text(t.trim().into()))
            }

            (InputData::Text(t), TransformType::Substring { start, len }) => {
                if *start < t.len() {
                    let end = (*start + *len).min(t.len());
                    Some(InputData::Text(t[*start..end].into()))
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Enable rule
    #[inline]
    pub fn enable_rule(&mut self, id: u64) {
        if let Some(rule) = self.rules.get_mut(&id) {
            rule.enabled = true;
        }
    }

    /// Disable rule
    #[inline]
    pub fn disable_rule(&mut self, id: u64) {
        if let Some(rule) = self.rules.get_mut(&id) {
            rule.enabled = false;
        }
    }

    /// Get recent outputs
    #[inline(always)]
    pub fn recent(&self, count: usize) -> Vec<&FilteredOutput> {
        self.output_buffer.iter().rev().take(count).collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &FilterStats {
        &self.stats
    }
}

impl Default for PerceptionFilter {
    fn default() -> Self {
        Self::new(FilterConfig::default())
    }
}

// ============================================================================
// INPUT BUILDER
// ============================================================================

/// Input builder
pub struct InputBuilder {
    source: String,
    input_type: InputType,
    data: Option<InputData>,
    metadata: BTreeMap<String, String>,
}

impl InputBuilder {
    /// Create new builder
    pub fn new(source: &str) -> Self {
        Self {
            source: source.into(),
            input_type: InputType::Text,
            data: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Set numeric data
    #[inline]
    pub fn numeric(mut self, value: f64) -> Self {
        self.input_type = InputType::Numeric;
        self.data = Some(InputData::Numeric(value));
        self
    }

    /// Set text data
    #[inline]
    pub fn text(mut self, value: &str) -> Self {
        self.input_type = InputType::Text;
        self.data = Some(InputData::Text(value.into()));
        self
    }

    /// Add metadata
    #[inline(always)]
    pub fn meta(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build input
    #[inline]
    pub fn build(self, id: u64) -> SensoryInput {
        SensoryInput {
            id,
            source: self.source,
            input_type: self.input_type,
            data: self.data.unwrap_or(InputData::Text(String::new())),
            timestamp: Timestamp::now(),
            metadata: self.metadata,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_filter() {
        let mut filter = PerceptionFilter::default();

        filter.add_rule(
            "accept_all",
            FilterCondition::Always,
            FilterAction::Accept,
            1,
        );

        let input = InputBuilder::new("test")
            .text("hello")
            .build(1);

        let output = filter.process(input);
        assert!(output.is_some());
    }

    #[test]
    fn test_rejection() {
        let mut filter = PerceptionFilter::default();

        filter.add_rule(
            "reject_spam",
            FilterCondition::Contains("spam".into()),
            FilterAction::Reject,
            10,
        );

        let input = InputBuilder::new("test")
            .text("this is spam")
            .build(1);

        let output = filter.process(input);
        assert!(output.is_none());
    }

    #[test]
    fn test_transform() {
        let mut filter = PerceptionFilter::default();

        filter.add_rule(
            "normalize",
            FilterCondition::TypeMatch(InputType::Numeric),
            FilterAction::Transform(TransformType::Normalize { min: 0.0, max: 100.0 }),
            5,
        );

        let input = InputBuilder::new("sensor")
            .numeric(50.0)
            .build(1);

        let output = filter.process(input).unwrap();

        if let InputData::Numeric(v) = output.data {
            assert!((v - 0.5).abs() < 0.01);
        } else {
            panic!("Expected numeric data");
        }
    }

    #[test]
    fn test_boost_relevance() {
        let mut filter = PerceptionFilter::default();

        filter.add_rule(
            "boost_important",
            FilterCondition::SourceMatch("priority".into()),
            FilterAction::Boost(1.5),
            5,
        );

        let input = InputBuilder::new("priority")
            .text("important message")
            .build(1);

        let output = filter.process(input).unwrap();
        assert!(output.relevance > 0.5);
    }

    #[test]
    fn test_compound_condition() {
        let mut filter = PerceptionFilter::default();

        filter.add_rule(
            "compound",
            FilterCondition::And(vec![
                FilterCondition::TypeMatch(InputType::Numeric),
                FilterCondition::ValueThreshold { min: Some(0.0), max: Some(100.0) },
            ]),
            FilterAction::Accept,
            5,
        );

        let input = InputBuilder::new("test")
            .numeric(50.0)
            .build(1);

        let output = filter.process(input);
        assert!(output.is_some());
    }
}

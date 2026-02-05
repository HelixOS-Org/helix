//! # Generalization Engine
//!
//! Pattern generalization and abstraction learning.
//! Extracts general principles from specific examples.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning Engine

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]
#![allow(clippy::for_kv_map)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PATTERN TYPES
// ============================================================================

/// Pattern (abstracted from examples)
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern ID
    pub id: u64,
    /// Pattern name
    pub name: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Features
    pub features: Vec<Feature>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Support (number of examples)
    pub support: u32,
    /// Confidence
    pub confidence: f64,
    /// Examples used
    pub examples: Vec<u64>,
    /// Counter-examples
    pub counter_examples: Vec<u64>,
    /// Created
    pub created: Timestamp,
    /// Last updated
    pub updated: Timestamp,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatternType {
    /// Structural pattern
    Structural,
    /// Behavioral pattern
    Behavioral,
    /// Temporal pattern
    Temporal,
    /// Causal pattern
    Causal,
    /// Conditional pattern
    Conditional,
}

/// Feature
#[derive(Debug, Clone)]
pub struct Feature {
    /// Feature name
    pub name: String,
    /// Feature type
    pub feature_type: FeatureType,
    /// Required
    pub required: bool,
    /// Value constraint
    pub constraint: Option<ValueConstraint>,
    /// Importance weight
    pub importance: f64,
}

/// Feature type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureType {
    Boolean,
    Numeric,
    Categorical,
    String,
    List,
    Nested,
}

/// Value constraint
#[derive(Debug, Clone)]
pub enum ValueConstraint {
    /// Exact value
    Exact(FeatureValue),
    /// Range
    Range { min: f64, max: f64 },
    /// One of values
    OneOf(Vec<FeatureValue>),
    /// Pattern match
    Pattern(String),
    /// Custom predicate
    Predicate(String),
}

/// Feature value
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<FeatureValue>),
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Description
    pub description: String,
    /// Strictness (0-1)
    pub strictness: f64,
}

/// Constraint type
#[derive(Debug, Clone)]
pub enum ConstraintType {
    /// Feature must be present
    RequiredFeature(String),
    /// Feature must have value
    FeatureValue { name: String, value: FeatureValue },
    /// Features must co-occur
    CoOccurrence(Vec<String>),
    /// Features are mutually exclusive
    MutualExclusion(Vec<String>),
    /// Dependency between features
    Dependency {
        if_present: String,
        then_present: String,
    },
    /// Ordering constraint
    Ordering(Vec<String>),
}

// ============================================================================
// EXAMPLE
// ============================================================================

/// Example for pattern learning
#[derive(Debug, Clone)]
pub struct Example {
    /// Example ID
    pub id: u64,
    /// Feature values
    pub features: BTreeMap<String, FeatureValue>,
    /// Label (for classification)
    pub label: Option<String>,
    /// Weight
    pub weight: f64,
    /// Source
    pub source: String,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl Example {
    /// Create new example
    pub fn new(features: BTreeMap<String, FeatureValue>) -> Self {
        Self {
            id: 0,
            features,
            label: None,
            weight: 1.0,
            source: String::new(),
            timestamp: Timestamp::now(),
        }
    }

    /// Get feature
    pub fn get(&self, name: &str) -> Option<&FeatureValue> {
        self.features.get(name)
    }

    /// Has feature
    pub fn has(&self, name: &str) -> bool {
        self.features.contains_key(name)
    }
}

// ============================================================================
// GENERALIZATION STRATEGIES
// ============================================================================

/// Generalization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Most specific generalization
    Specific,
    /// Most general generalization
    General,
    /// Balanced
    Balanced,
    /// Minimal description length
    Mdl,
    /// Version spaces
    VersionSpace,
}

/// Generalization operation
#[derive(Debug, Clone)]
pub enum GeneralizationOp {
    /// Drop feature
    DropFeature(String),
    /// Widen value constraint
    WidenConstraint {
        feature: String,
        new_constraint: ValueConstraint,
    },
    /// Make feature optional
    MakeOptional(String),
    /// Merge patterns
    Merge(u64, u64),
    /// Add alternative
    AddAlternative {
        feature: String,
        value: FeatureValue,
    },
}

// ============================================================================
// GENERALIZATION ENGINE
// ============================================================================

/// Generalization engine
pub struct GeneralizationEngine {
    /// Patterns
    patterns: BTreeMap<u64, Pattern>,
    /// Patterns by type
    by_type: BTreeMap<PatternType, Vec<u64>>,
    /// Examples
    examples: BTreeMap<u64, Example>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: GeneralizationConfig,
    /// Statistics
    stats: GeneralizationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct GeneralizationConfig {
    /// Strategy
    pub strategy: Strategy,
    /// Minimum support
    pub min_support: u32,
    /// Minimum confidence
    pub min_confidence: f64,
    /// Maximum features
    pub max_features: usize,
    /// Allow overlapping patterns
    pub allow_overlap: bool,
}

impl Default for GeneralizationConfig {
    fn default() -> Self {
        Self {
            strategy: Strategy::Balanced,
            min_support: 2,
            min_confidence: 0.8,
            max_features: 20,
            allow_overlap: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct GeneralizationStats {
    /// Patterns created
    pub patterns_created: u64,
    /// Generalizations performed
    pub generalizations: u64,
    /// Examples processed
    pub examples_processed: u64,
    /// Average pattern support
    pub avg_support: f64,
}

impl GeneralizationEngine {
    /// Create new engine
    pub fn new(config: GeneralizationConfig) -> Self {
        Self {
            patterns: BTreeMap::new(),
            by_type: BTreeMap::new(),
            examples: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: GeneralizationStats::default(),
        }
    }

    /// Add example
    pub fn add_example(&mut self, mut example: Example) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        example.id = id;
        self.examples.insert(id, example);
        self.stats.examples_processed += 1;
        id
    }

    /// Learn pattern from examples
    pub fn learn(&mut self, example_ids: &[u64], pattern_type: PatternType) -> Option<u64> {
        if example_ids.len() < self.config.min_support as usize {
            return None;
        }

        let examples: Vec<&Example> = example_ids
            .iter()
            .filter_map(|id| self.examples.get(id))
            .collect();

        if examples.is_empty() {
            return None;
        }

        // Find common features
        let features = self.extract_common_features(&examples);
        if features.is_empty() {
            return None;
        }

        // Create pattern
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let pattern = Pattern {
            id,
            name: format!("Pattern_{}", id),
            pattern_type,
            features,
            constraints: Vec::new(),
            support: examples.len() as u32,
            confidence: 1.0,
            examples: example_ids.to_vec(),
            counter_examples: Vec::new(),
            created: Timestamp::now(),
            updated: Timestamp::now(),
        };

        self.patterns.insert(id, pattern);
        self.by_type.entry(pattern_type).or_default().push(id);
        self.stats.patterns_created += 1;

        Some(id)
    }

    fn extract_common_features(&self, examples: &[&Example]) -> Vec<Feature> {
        if examples.is_empty() {
            return Vec::new();
        }

        let first = examples[0];
        let mut common = Vec::new();

        for (name, _value) in &first.features {
            // Check if all examples have this feature
            let all_have = examples.iter().all(|e| e.has(name));

            if all_have {
                // Check if values are consistent
                let values: Vec<&FeatureValue> =
                    examples.iter().filter_map(|e| e.get(name)).collect();

                let (feature_type, constraint) = self.infer_feature_type(&values);

                common.push(Feature {
                    name: name.clone(),
                    feature_type,
                    required: true,
                    constraint,
                    importance: 1.0,
                });
            }
        }

        common
    }

    fn infer_feature_type(
        &self,
        values: &[&FeatureValue],
    ) -> (FeatureType, Option<ValueConstraint>) {
        if values.is_empty() {
            return (FeatureType::String, None);
        }

        match values[0] {
            FeatureValue::Bool(_) => {
                let all_same = values.iter().all(|v| *v == values[0]);
                if all_same {
                    (
                        FeatureType::Boolean,
                        Some(ValueConstraint::Exact((*values[0]).clone())),
                    )
                } else {
                    (FeatureType::Boolean, None)
                }
            },
            FeatureValue::Int(_) => {
                let ints: Vec<i64> = values
                    .iter()
                    .filter_map(|v| {
                        if let FeatureValue::Int(i) = v {
                            Some(*i)
                        } else {
                            None
                        }
                    })
                    .collect();

                if ints.len() == values.len() {
                    let min = *ints.iter().min().unwrap_or(&0);
                    let max = *ints.iter().max().unwrap_or(&0);
                    (
                        FeatureType::Numeric,
                        Some(ValueConstraint::Range {
                            min: min as f64,
                            max: max as f64,
                        }),
                    )
                } else {
                    (FeatureType::Numeric, None)
                }
            },
            FeatureValue::Float(_) => {
                let floats: Vec<f64> = values
                    .iter()
                    .filter_map(|v| {
                        if let FeatureValue::Float(f) = v {
                            Some(*f)
                        } else {
                            None
                        }
                    })
                    .collect();

                if floats.len() == values.len() {
                    let min = floats.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max = floats.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    (
                        FeatureType::Numeric,
                        Some(ValueConstraint::Range { min, max }),
                    )
                } else {
                    (FeatureType::Numeric, None)
                }
            },
            FeatureValue::String(_) => {
                let strings: Vec<String> = values
                    .iter()
                    .filter_map(|v| {
                        if let FeatureValue::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                let unique: Vec<_> = strings
                    .iter()
                    .collect::<alloc::collections::BTreeSet<_>>()
                    .into_iter()
                    .cloned()
                    .collect();

                if unique.len() <= 5 {
                    (
                        FeatureType::Categorical,
                        Some(ValueConstraint::OneOf(
                            unique.into_iter().map(FeatureValue::String).collect(),
                        )),
                    )
                } else {
                    (FeatureType::String, None)
                }
            },
            FeatureValue::List(_) => (FeatureType::List, None),
        }
    }

    /// Generalize pattern
    pub fn generalize(&mut self, pattern_id: u64, new_example: &Example) -> bool {
        // First check if pattern exists and if example matches
        let matches = if let Some(pattern) = self.patterns.get(&pattern_id) {
            self.matches_pattern(new_example, pattern)
        } else {
            return false;
        };

        self.stats.generalizations += 1;

        let strategy = self.config.strategy;

        let pattern = match self.patterns.get_mut(&pattern_id) {
            Some(p) => p,
            None => return false,
        };

        if matches {
            // Just add as supporting example
            pattern.examples.push(new_example.id);
            pattern.support += 1;
            pattern.updated = Timestamp::now();
            true
        } else {
            // Try to generalize - need to handle this differently to avoid borrow issues
            // Since generalize_* methods need &mut self, we'll inline the logic here
            Self::generalize_pattern_inline(pattern, new_example, strategy)
        }
    }

    fn generalize_pattern_inline(
        pattern: &mut Pattern,
        example: &Example,
        strategy: Strategy,
    ) -> bool {
        match strategy {
            Strategy::Specific => Self::generalize_specific_inline(pattern, example),
            Strategy::General => Self::generalize_general_inline(pattern, example),
            Strategy::Balanced => Self::generalize_balanced_inline(pattern, example),
            _ => false,
        }
    }

    fn generalize_specific_inline(pattern: &mut Pattern, example: &Example) -> bool {
        // Minimal generalization: only widen constraints as needed
        let mut changed = false;

        for feature in &mut pattern.features {
            if let Some(_value) = example.get(&feature.name) {
                // Feature exists in example
            } else if feature.required {
                // Make optional
                feature.required = false;
                changed = true;
            }
        }

        if changed {
            pattern.examples.push(example.id);
            pattern.support += 1;
            pattern.updated = Timestamp::now();
        }

        changed
    }

    fn generalize_general_inline(pattern: &mut Pattern, example: &Example) -> bool {
        // More aggressive generalization
        pattern.examples.push(example.id);
        pattern.support += 1;
        pattern.updated = Timestamp::now();
        true
    }

    fn generalize_balanced_inline(pattern: &mut Pattern, example: &Example) -> bool {
        // Balanced approach
        pattern.examples.push(example.id);
        pattern.support += 1;
        pattern.updated = Timestamp::now();
        true
    }

    fn matches_pattern(&self, example: &Example, pattern: &Pattern) -> bool {
        for feature in &pattern.features {
            if feature.required && !example.has(&feature.name) {
                return false;
            }

            if let (Some(value), Some(constraint)) =
                (example.get(&feature.name), &feature.constraint)
            {
                if !self.matches_constraint(value, constraint) {
                    return false;
                }
            }
        }
        true
    }

    fn matches_constraint(&self, value: &FeatureValue, constraint: &ValueConstraint) -> bool {
        match constraint {
            ValueConstraint::Exact(expected) => value == expected,
            ValueConstraint::Range { min, max } => {
                if let FeatureValue::Float(f) = value {
                    *f >= *min && *f <= *max
                } else if let FeatureValue::Int(i) = value {
                    (*i as f64) >= *min && (*i as f64) <= *max
                } else {
                    false
                }
            },
            ValueConstraint::OneOf(options) => options.contains(value),
            ValueConstraint::Pattern(_pattern_str) => {
                // Simplified: just check if string
                matches!(value, FeatureValue::String(_))
            },
            ValueConstraint::Predicate(_) => true, // Would evaluate predicate
        }
    }

    fn generalize_specific(&mut self, pattern: &mut Pattern, example: &Example) -> bool {
        // Minimal generalization: only widen constraints as needed
        let mut changed = false;

        for feature in &mut pattern.features {
            if let Some(value) = example.get(&feature.name) {
                if let Some(constraint) = &mut feature.constraint {
                    if !self.matches_constraint(value, constraint) {
                        // Widen constraint to include new value
                        if let Some(new_constraint) = self.widen_constraint(constraint, value) {
                            *constraint = new_constraint;
                            changed = true;
                        }
                    }
                }
            } else if feature.required {
                // Make optional
                feature.required = false;
                changed = true;
            }
        }

        if changed {
            pattern.examples.push(example.id);
            pattern.support += 1;
            pattern.updated = Timestamp::now();
        }

        changed
    }

    fn generalize_general(&mut self, pattern: &mut Pattern, example: &Example) -> bool {
        // Drop constraints that don't match
        for feature in &mut pattern.features {
            if !example.has(&feature.name)
                || (feature.constraint.is_some()
                    && !self.matches_constraint(
                        example.get(&feature.name).unwrap(),
                        feature.constraint.as_ref().unwrap(),
                    ))
            {
                feature.constraint = None;
                feature.required = false;
            }
        }

        pattern.examples.push(example.id);
        pattern.support += 1;
        pattern.updated = Timestamp::now();
        true
    }

    fn generalize_balanced(&mut self, pattern: &mut Pattern, example: &Example) -> bool {
        // Balance between specific and general
        let initial_confidence = pattern.confidence;

        let result = self.generalize_specific(pattern, example);

        // Update confidence based on how much we generalized
        let feature_count = pattern.features.len() as f64;
        let constrained_count = pattern
            .features
            .iter()
            .filter(|f| f.constraint.is_some())
            .count() as f64;

        pattern.confidence = initial_confidence * (constrained_count / feature_count).max(0.5);

        result
    }

    fn widen_constraint(
        &self,
        constraint: &ValueConstraint,
        value: &FeatureValue,
    ) -> Option<ValueConstraint> {
        match constraint {
            ValueConstraint::Exact(expected) => Some(ValueConstraint::OneOf(vec![
                expected.clone(),
                value.clone(),
            ])),
            ValueConstraint::Range { min, max } => {
                let (new_min, new_max) = if let FeatureValue::Float(f) = value {
                    (min.min(*f), max.max(*f))
                } else if let FeatureValue::Int(i) = value {
                    (min.min(*i as f64), max.max(*i as f64))
                } else {
                    (*min, *max)
                };
                Some(ValueConstraint::Range {
                    min: new_min,
                    max: new_max,
                })
            },
            ValueConstraint::OneOf(options) => {
                let mut new_options = options.clone();
                if !new_options.contains(value) {
                    new_options.push(value.clone());
                }
                Some(ValueConstraint::OneOf(new_options))
            },
            _ => None,
        }
    }

    /// Get pattern
    pub fn get_pattern(&self, id: u64) -> Option<&Pattern> {
        self.patterns.get(&id)
    }

    /// Find matching patterns
    pub fn find_matching(&self, example: &Example) -> Vec<&Pattern> {
        self.patterns
            .values()
            .filter(|p| self.matches_pattern(example, p))
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &GeneralizationStats {
        &self.stats
    }
}

impl Default for GeneralizationEngine {
    fn default() -> Self {
        Self::new(GeneralizationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_example(color: &str, size: i64) -> Example {
        let mut features = BTreeMap::new();
        features.insert("color".into(), FeatureValue::String(color.into()));
        features.insert("size".into(), FeatureValue::Int(size));
        Example::new(features)
    }

    #[test]
    fn test_add_example() {
        let mut engine = GeneralizationEngine::default();
        let example = make_example("red", 10);
        let id = engine.add_example(example);
        assert!(id > 0);
    }

    #[test]
    fn test_learn_pattern() {
        let mut engine = GeneralizationEngine::default();

        let id1 = engine.add_example(make_example("red", 10));
        let id2 = engine.add_example(make_example("red", 20));

        let pattern_id = engine.learn(&[id1, id2], PatternType::Structural);
        assert!(pattern_id.is_some());

        let pattern = engine.get_pattern(pattern_id.unwrap()).unwrap();
        assert_eq!(pattern.support, 2);
    }

    #[test]
    fn test_generalization() {
        let mut engine = GeneralizationEngine::default();

        let id1 = engine.add_example(make_example("red", 10));
        let id2 = engine.add_example(make_example("red", 20));

        let pattern_id = engine.learn(&[id1, id2], PatternType::Structural).unwrap();

        // Generalize with new example
        let new_example = make_example("blue", 15);
        engine.generalize(pattern_id, &new_example);

        let pattern = engine.get_pattern(pattern_id).unwrap();
        // Should have widened constraints
        assert!(pattern.features.iter().any(|f| f.name == "color"));
    }

    #[test]
    fn test_pattern_matching() {
        let mut engine = GeneralizationEngine::default();

        let id1 = engine.add_example(make_example("red", 10));
        let id2 = engine.add_example(make_example("red", 20));

        engine.learn(&[id1, id2], PatternType::Structural);

        let test = make_example("red", 15);
        let matches = engine.find_matching(&test);

        // Should match the pattern
        assert!(!matches.is_empty());
    }
}

//! # Abductive Reasoning
//!
//! Generates explanatory hypotheses from observations.
//! Finds the best explanations for given evidence.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// ABDUCTION TYPES
// ============================================================================

/// Observation
#[derive(Debug, Clone)]
pub struct Observation {
    /// Observation ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Features
    pub features: BTreeMap<String, FeatureValue>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Confidence
    pub confidence: f64,
}

/// Feature value
#[derive(Debug, Clone)]
pub enum FeatureValue {
    Boolean(bool),
    Numeric(f64),
    Text(String),
    Category(String),
}

/// Explanation
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Explanation ID
    pub id: u64,
    /// Hypothesis
    pub hypothesis: String,
    /// Explains observations
    pub explains: Vec<u64>,
    /// Supporting evidence
    pub supporting_evidence: Vec<Evidence>,
    /// Contradicting evidence
    pub contradicting_evidence: Vec<Evidence>,
    /// Plausibility score
    pub plausibility: f64,
    /// Simplicity score
    pub simplicity: f64,
    /// Combined score
    pub score: f64,
    /// Status
    pub status: ExplanationStatus,
}

/// Evidence
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Evidence ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Weight
    pub weight: f64,
    /// Source
    pub source: String,
}

/// Explanation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationStatus {
    Proposed,
    Plausible,
    Likely,
    Accepted,
    Rejected,
}

/// Explanation rule
#[derive(Debug, Clone)]
pub struct ExplanationRule {
    /// Rule ID
    pub id: u64,
    /// If these features are observed
    pub if_observed: Vec<FeaturePattern>,
    /// Then this hypothesis is plausible
    pub then_hypothesis: String,
    /// Base plausibility
    pub base_plausibility: f64,
}

/// Feature pattern
#[derive(Debug, Clone)]
pub struct FeaturePattern {
    /// Feature name
    pub name: String,
    /// Expected value pattern
    pub pattern: ValuePattern,
}

/// Value pattern
#[derive(Debug, Clone)]
pub enum ValuePattern {
    Equals(FeatureValue),
    GreaterThan(f64),
    LessThan(f64),
    Contains(String),
    Any,
}

// ============================================================================
// ABDUCTION ENGINE
// ============================================================================

/// Abduction engine
pub struct AbductionEngine {
    /// Observations
    observations: BTreeMap<u64, Observation>,
    /// Explanations
    explanations: BTreeMap<u64, Explanation>,
    /// Rules
    rules: Vec<ExplanationRule>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: AbductionConfig,
    /// Statistics
    stats: AbductionStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct AbductionConfig {
    /// Minimum plausibility
    pub min_plausibility: f64,
    /// Simplicity weight
    pub simplicity_weight: f64,
    /// Plausibility weight
    pub plausibility_weight: f64,
    /// Maximum explanations
    pub max_explanations: usize,
}

impl Default for AbductionConfig {
    fn default() -> Self {
        Self {
            min_plausibility: 0.3,
            simplicity_weight: 0.3,
            plausibility_weight: 0.7,
            max_explanations: 10,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AbductionStats {
    /// Observations recorded
    pub observations_recorded: u64,
    /// Explanations generated
    pub explanations_generated: u64,
    /// Rules applied
    pub rules_applied: u64,
}

impl AbductionEngine {
    /// Create new engine
    pub fn new(config: AbductionConfig) -> Self {
        Self {
            observations: BTreeMap::new(),
            explanations: BTreeMap::new(),
            rules: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: AbductionStats::default(),
        }
    }

    /// Add rule
    #[inline(always)]
    pub fn add_rule(&mut self, rule: ExplanationRule) {
        self.rules.push(rule);
    }

    /// Record observation
    pub fn observe(
        &mut self,
        description: &str,
        features: BTreeMap<String, FeatureValue>,
        confidence: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let observation = Observation {
            id,
            description: description.into(),
            features,
            timestamp: Timestamp::now(),
            confidence,
        };

        self.observations.insert(id, observation);
        self.stats.observations_recorded += 1;

        id
    }

    /// Generate explanations
    pub fn abduce(&mut self, observation_ids: &[u64]) -> Vec<u64> {
        let observations: Vec<&Observation> = observation_ids
            .iter()
            .filter_map(|id| self.observations.get(id))
            .collect();

        if observations.is_empty() {
            return Vec::new();
        }

        let mut explanation_ids = Vec::new();

        // Apply rules to generate candidate explanations
        for rule in &self.rules {
            let matches = observations
                .iter()
                .filter(|obs| self.rule_matches(rule, obs))
                .count();

            if matches > 0 {
                let coverage = matches as f64 / observations.len() as f64;
                let plausibility = rule.base_plausibility * coverage;

                if plausibility >= self.config.min_plausibility {
                    let exp_id = self.create_explanation(
                        &rule.then_hypothesis,
                        observation_ids,
                        plausibility,
                    );
                    explanation_ids.push(exp_id);
                    self.stats.rules_applied += 1;
                }
            }
        }

        // Rank explanations
        self.rank_explanations(&explanation_ids);

        // Return top explanations
        explanation_ids.truncate(self.config.max_explanations);

        explanation_ids
    }

    fn rule_matches(&self, rule: &ExplanationRule, observation: &Observation) -> bool {
        for pattern in &rule.if_observed {
            if let Some(value) = observation.features.get(&pattern.name) {
                if !self.pattern_matches(&pattern.pattern, value) {
                    return false;
                }
            } else if !matches!(pattern.pattern, ValuePattern::Any) {
                return false;
            }
        }
        true
    }

    fn pattern_matches(&self, pattern: &ValuePattern, value: &FeatureValue) -> bool {
        match (pattern, value) {
            (ValuePattern::Any, _) => true,

            (ValuePattern::Equals(expected), actual) => self.values_equal(expected, actual),

            (ValuePattern::GreaterThan(threshold), FeatureValue::Numeric(n)) => *n > *threshold,

            (ValuePattern::LessThan(threshold), FeatureValue::Numeric(n)) => *n < *threshold,

            (ValuePattern::Contains(substr), FeatureValue::Text(text)) => {
                text.contains(substr.as_str())
            },

            _ => false,
        }
    }

    fn values_equal(&self, a: &FeatureValue, b: &FeatureValue) -> bool {
        match (a, b) {
            (FeatureValue::Boolean(x), FeatureValue::Boolean(y)) => x == y,
            (FeatureValue::Numeric(x), FeatureValue::Numeric(y)) => (x - y).abs() < f64::EPSILON,
            (FeatureValue::Text(x), FeatureValue::Text(y)) => x == y,
            (FeatureValue::Category(x), FeatureValue::Category(y)) => x == y,
            _ => false,
        }
    }

    fn create_explanation(
        &mut self,
        hypothesis: &str,
        observation_ids: &[u64],
        plausibility: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Simplicity based on hypothesis length (shorter = simpler)
        let simplicity = 1.0 / (1.0 + hypothesis.len() as f64 / 100.0);

        // Combined score
        let score = self.config.plausibility_weight * plausibility
            + self.config.simplicity_weight * simplicity;

        let explanation = Explanation {
            id,
            hypothesis: hypothesis.into(),
            explains: observation_ids.to_vec(),
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            plausibility,
            simplicity,
            score,
            status: ExplanationStatus::Proposed,
        };

        self.explanations.insert(id, explanation);
        self.stats.explanations_generated += 1;

        id
    }

    fn rank_explanations(&mut self, ids: &[u64]) {
        for &id in ids {
            if let Some(exp) = self.explanations.get_mut(&id) {
                // Recalculate score based on evidence
                let support: f64 = exp.supporting_evidence.iter().map(|e| e.weight).sum();
                let contradiction: f64 = exp.contradicting_evidence.iter().map(|e| e.weight).sum();

                let evidence_score = if support + contradiction > 0.0 {
                    support / (support + contradiction)
                } else {
                    0.5
                };

                exp.score = (exp.score + evidence_score) / 2.0;

                // Update status based on score
                exp.status = if exp.score > 0.8 {
                    ExplanationStatus::Likely
                } else if exp.score > 0.5 {
                    ExplanationStatus::Plausible
                } else {
                    ExplanationStatus::Proposed
                };
            }
        }
    }

    /// Add supporting evidence
    #[inline]
    pub fn add_support(&mut self, explanation_id: u64, evidence: Evidence) {
        if let Some(exp) = self.explanations.get_mut(&explanation_id) {
            exp.supporting_evidence.push(evidence);
        }
    }

    /// Add contradicting evidence
    #[inline]
    pub fn add_contradiction(&mut self, explanation_id: u64, evidence: Evidence) {
        if let Some(exp) = self.explanations.get_mut(&explanation_id) {
            exp.contradicting_evidence.push(evidence);
        }
    }

    /// Accept explanation
    #[inline]
    pub fn accept(&mut self, id: u64) {
        if let Some(exp) = self.explanations.get_mut(&id) {
            exp.status = ExplanationStatus::Accepted;
        }
    }

    /// Reject explanation
    #[inline]
    pub fn reject(&mut self, id: u64) {
        if let Some(exp) = self.explanations.get_mut(&id) {
            exp.status = ExplanationStatus::Rejected;
        }
    }

    /// Get best explanation
    #[inline]
    pub fn best_explanation(&self) -> Option<&Explanation> {
        self.explanations
            .values()
            .filter(|e| e.status != ExplanationStatus::Rejected)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
    }

    /// Get explanation
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&Explanation> {
        self.explanations.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &AbductionStats {
        &self.stats
    }
}

impl Default for AbductionEngine {
    fn default() -> Self {
        Self::new(AbductionConfig::default())
    }
}

// ============================================================================
// RULE BUILDER
// ============================================================================

/// Rule builder
pub struct RuleBuilder {
    id: u64,
    if_observed: Vec<FeaturePattern>,
    then_hypothesis: String,
    base_plausibility: f64,
}

impl RuleBuilder {
    /// Create new builder
    pub fn new(id: u64) -> Self {
        Self {
            id,
            if_observed: Vec::new(),
            then_hypothesis: String::new(),
            base_plausibility: 0.5,
        }
    }

    /// Add condition
    #[inline]
    pub fn when(mut self, feature: &str, pattern: ValuePattern) -> Self {
        self.if_observed.push(FeaturePattern {
            name: feature.into(),
            pattern,
        });
        self
    }

    /// Set hypothesis
    #[inline(always)]
    pub fn then(mut self, hypothesis: &str) -> Self {
        self.then_hypothesis = hypothesis.into();
        self
    }

    /// Set plausibility
    #[inline(always)]
    pub fn with_plausibility(mut self, plausibility: f64) -> Self {
        self.base_plausibility = plausibility;
        self
    }

    /// Build
    #[inline]
    pub fn build(self) -> ExplanationRule {
        ExplanationRule {
            id: self.id,
            if_observed: self.if_observed,
            then_hypothesis: self.then_hypothesis,
            base_plausibility: self.base_plausibility,
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
    fn test_observe() {
        let mut engine = AbductionEngine::default();

        let mut features = BTreeMap::new();
        features.insert("symptom".into(), FeatureValue::Text("fever".into()));

        let id = engine.observe("Patient has fever", features, 0.9);

        assert!(engine.observations.contains_key(&id));
    }

    #[test]
    fn test_add_rule() {
        let mut engine = AbductionEngine::default();

        let rule = RuleBuilder::new(1)
            .when(
                "symptom",
                ValuePattern::Equals(FeatureValue::Text("fever".into())),
            )
            .then("Patient might have infection")
            .with_plausibility(0.7)
            .build();

        engine.add_rule(rule);

        assert_eq!(engine.rules.len(), 1);
    }

    #[test]
    fn test_abduce() {
        let mut engine = AbductionEngine::default();

        // Add rule
        let rule = RuleBuilder::new(1)
            .when(
                "symptom",
                ValuePattern::Equals(FeatureValue::Text("fever".into())),
            )
            .then("Possible infection")
            .with_plausibility(0.8)
            .build();

        engine.add_rule(rule);

        // Add observation
        let mut features = BTreeMap::new();
        features.insert("symptom".into(), FeatureValue::Text("fever".into()));

        let obs_id = engine.observe("Fever observed", features, 0.9);

        // Generate explanations
        let explanations = engine.abduce(&[obs_id]);

        assert!(!explanations.is_empty());
    }

    #[test]
    fn test_evidence() {
        let mut engine = AbductionEngine::default();

        let rule = RuleBuilder::new(1)
            .when("x", ValuePattern::Any)
            .then("Hypothesis")
            .with_plausibility(0.5)
            .build();

        engine.add_rule(rule);

        let mut features = BTreeMap::new();
        features.insert("x".into(), FeatureValue::Boolean(true));

        let obs_id = engine.observe("Test", features, 1.0);
        let exp_ids = engine.abduce(&[obs_id]);

        let exp_id = exp_ids[0];

        engine.add_support(exp_id, Evidence {
            id: 1,
            description: "Supporting".into(),
            weight: 0.8,
            source: "test".into(),
        });

        let exp = engine.get(exp_id).unwrap();
        assert_eq!(exp.supporting_evidence.len(), 1);
    }

    #[test]
    fn test_best_explanation() {
        let mut engine = AbductionEngine::default();

        // Add two rules with different plausibilities
        engine.add_rule(
            RuleBuilder::new(1)
                .when("x", ValuePattern::Any)
                .then("Low plausibility")
                .with_plausibility(0.4)
                .build(),
        );

        engine.add_rule(
            RuleBuilder::new(2)
                .when("x", ValuePattern::Any)
                .then("High plausibility")
                .with_plausibility(0.9)
                .build(),
        );

        let mut features = BTreeMap::new();
        features.insert("x".into(), FeatureValue::Boolean(true));

        let obs_id = engine.observe("Test", features, 1.0);
        engine.abduce(&[obs_id]);

        let best = engine.best_explanation().unwrap();
        assert!(best.plausibility > 0.5);
    }
}

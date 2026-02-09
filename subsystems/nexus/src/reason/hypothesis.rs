//! # Hypothesis Generator
//!
//! Generates and evaluates hypotheses for reasoning.
//! Supports abductive reasoning and hypothesis testing.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning Engine

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
// HYPOTHESIS TYPES
// ============================================================================

/// Hypothesis
#[derive(Debug, Clone)]
pub struct Hypothesis {
    /// Hypothesis ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Type
    pub hypothesis_type: HypothesisType,
    /// Status
    pub status: HypothesisStatus,
    /// Antecedents (conditions)
    pub antecedents: Vec<Proposition>,
    /// Consequent (conclusion)
    pub consequent: Proposition,
    /// Confidence
    pub confidence: f64,
    /// Evidence
    pub evidence: Vec<Evidence>,
    /// Created
    pub created: Timestamp,
    /// Last evaluated
    pub last_evaluated: Option<Timestamp>,
}

/// Hypothesis type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisType {
    /// Causal hypothesis (X causes Y)
    Causal,
    /// Explanatory hypothesis (X explains Y)
    Explanatory,
    /// Predictive hypothesis (if X then Y)
    Predictive,
    /// Diagnostic hypothesis (Y because of X)
    Diagnostic,
    /// Comparative hypothesis (X is better than Y)
    Comparative,
    /// Correlational hypothesis (X correlates with Y)
    Correlational,
}

/// Hypothesis status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    /// Newly generated
    Proposed,
    /// Being tested
    Testing,
    /// Supported by evidence
    Supported,
    /// Refuted by evidence
    Refuted,
    /// Inconclusive evidence
    Inconclusive,
    /// Superseded by better hypothesis
    Superseded,
}

/// Proposition
#[derive(Debug, Clone)]
pub struct Proposition {
    /// Proposition ID
    pub id: u64,
    /// Subject
    pub subject: String,
    /// Predicate
    pub predicate: String,
    /// Object
    pub object: Option<String>,
    /// Modality
    pub modality: Modality,
    /// Truth value
    pub truth_value: TruthValue,
}

/// Modality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    /// Definitely true
    Necessary,
    /// Possibly true
    Possible,
    /// Actually true (in this case)
    Actual,
    /// Could be true
    Contingent,
}

/// Truth value
#[derive(Debug, Clone, Copy)]
pub enum TruthValue {
    /// Definitely true
    True,
    /// Definitely false
    False,
    /// Unknown
    Unknown,
    /// Probability
    Probability(f64),
}

/// Evidence
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Evidence ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Type
    pub evidence_type: EvidenceType,
    /// Weight
    pub weight: f64,
    /// Source
    pub source: String,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Evidence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    /// Direct observation
    Observation,
    /// Experimental result
    Experiment,
    /// Statistical data
    Statistical,
    /// Expert opinion
    Expert,
    /// Derived from other facts
    Derived,
    /// Counter-evidence
    Counter,
}

// ============================================================================
// HYPOTHESIS GENERATOR
// ============================================================================

/// Hypothesis generator
pub struct HypothesisGenerator {
    /// Generated hypotheses
    hypotheses: BTreeMap<u64, Hypothesis>,
    /// Propositions
    propositions: BTreeMap<u64, Proposition>,
    /// Evidence pool
    evidence: BTreeMap<u64, Evidence>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: GeneratorConfig,
    /// Statistics
    stats: GeneratorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Minimum confidence for generation
    pub min_confidence: f64,
    /// Maximum hypotheses per observation
    pub max_hypotheses: usize,
    /// Enable abductive reasoning
    pub enable_abduction: bool,
    /// Prune weak hypotheses
    pub prune_weak: bool,
    /// Weak threshold
    pub weak_threshold: f64,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.3,
            max_hypotheses: 10,
            enable_abduction: true,
            prune_weak: true,
            weak_threshold: 0.2,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct GeneratorStats {
    /// Hypotheses generated
    pub generated: u64,
    /// Hypotheses supported
    pub supported: u64,
    /// Hypotheses refuted
    pub refuted: u64,
    /// Evidence pieces
    pub evidence_count: u64,
}

impl HypothesisGenerator {
    /// Create new generator
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            hypotheses: BTreeMap::new(),
            propositions: BTreeMap::new(),
            evidence: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: GeneratorStats::default(),
        }
    }

    /// Generate hypothesis
    pub fn generate(
        &mut self,
        hypothesis_type: HypothesisType,
        description: &str,
        antecedents: Vec<Proposition>,
        consequent: Proposition,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let hypothesis = Hypothesis {
            id,
            description: description.into(),
            hypothesis_type,
            status: HypothesisStatus::Proposed,
            antecedents,
            consequent,
            confidence: self.config.min_confidence,
            evidence: Vec::new(),
            created: Timestamp::now(),
            last_evaluated: None,
        };

        self.hypotheses.insert(id, hypothesis);
        self.stats.generated += 1;

        id
    }

    /// Create proposition
    pub fn create_proposition(
        &mut self,
        subject: &str,
        predicate: &str,
        object: Option<&str>,
    ) -> Proposition {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let prop = Proposition {
            id,
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.map(String::from),
            modality: Modality::Actual,
            truth_value: TruthValue::Unknown,
        };

        self.propositions.insert(id, prop.clone());
        prop
    }

    /// Add evidence
    pub fn add_evidence(
        &mut self,
        hypothesis_id: u64,
        description: &str,
        evidence_type: EvidenceType,
        weight: f64,
        source: &str,
    ) -> Option<u64> {
        let hypothesis = self.hypotheses.get_mut(&hypothesis_id)?;

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let evidence = Evidence {
            id,
            description: description.into(),
            evidence_type,
            weight: weight.clamp(-1.0, 1.0),
            source: source.into(),
            timestamp: Timestamp::now(),
        };

        hypothesis.evidence.push(evidence.clone());
        self.evidence.insert(id, evidence);
        self.stats.evidence_count += 1;

        // Update confidence
        self.update_confidence(hypothesis_id);

        Some(id)
    }

    /// Update hypothesis confidence
    fn update_confidence(&mut self, hypothesis_id: u64) {
        if let Some(hypothesis) = self.hypotheses.get_mut(&hypothesis_id) {
            if hypothesis.evidence.is_empty() {
                return;
            }

            // Calculate weighted average of evidence
            let total_weight: f64 = hypothesis.evidence.iter()
                .map(|e| e.weight.abs())
                .sum();

            if total_weight > 0.0 {
                let weighted_sum: f64 = hypothesis.evidence.iter()
                    .map(|e| {
                        let sign = if e.evidence_type == EvidenceType::Counter { -1.0 } else { 1.0 };
                        sign * e.weight
                    })
                    .sum();

                // Normalize to [0, 1]
                hypothesis.confidence = ((weighted_sum / total_weight) + 1.0) / 2.0;
            }

            hypothesis.last_evaluated = Some(Timestamp::now());
        }
    }

    /// Evaluate hypothesis
    pub fn evaluate(&mut self, hypothesis_id: u64) -> Option<HypothesisStatus> {
        let hypothesis = self.hypotheses.get_mut(&hypothesis_id)?;

        let old_status = hypothesis.status;
        hypothesis.status = if hypothesis.confidence >= 0.7 {
            HypothesisStatus::Supported
        } else if hypothesis.confidence <= 0.3 {
            HypothesisStatus::Refuted
        } else {
            HypothesisStatus::Inconclusive
        };

        // Update stats
        if old_status != hypothesis.status {
            match hypothesis.status {
                HypothesisStatus::Supported => self.stats.supported += 1,
                HypothesisStatus::Refuted => self.stats.refuted += 1,
                _ => {}
            }
        }

        Some(hypothesis.status)
    }

    /// Generate abductive hypotheses
    pub fn abduct(&mut self, observation: &Proposition, knowledge: &[Proposition]) -> Vec<u64> {
        if !self.config.enable_abduction {
            return Vec::new();
        }

        let mut hypotheses = Vec::new();

        // Find propositions that could explain the observation
        for prop in knowledge {
            // Check if prop could cause observation
            if self.could_explain(prop, observation) {
                let id = self.generate(
                    HypothesisType::Explanatory,
                    &format!("{} explains {}", prop.predicate, observation.predicate),
                    vec![prop.clone()],
                    observation.clone(),
                );
                hypotheses.push(id);

                if hypotheses.len() >= self.config.max_hypotheses {
                    break;
                }
            }
        }

        hypotheses
    }

    fn could_explain(&self, cause: &Proposition, effect: &Proposition) -> bool {
        // Simplified check - in real impl would use causal models
        cause.subject == effect.subject ||
        cause.object.as_ref() == Some(&effect.subject)
    }

    /// Find best hypothesis
    #[inline]
    pub fn best_hypothesis(&self, hypothesis_type: Option<HypothesisType>) -> Option<&Hypothesis> {
        self.hypotheses.values()
            .filter(|h| {
                h.status == HypothesisStatus::Proposed ||
                h.status == HypothesisStatus::Supported
            })
            .filter(|h| hypothesis_type.map_or(true, |t| h.hypothesis_type == t))
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
    }

    /// Get competing hypotheses
    pub fn competing_hypotheses(&self, hypothesis_id: u64) -> Vec<&Hypothesis> {
        let target = match self.hypotheses.get(&hypothesis_id) {
            Some(h) => h,
            None => return Vec::new(),
        };

        self.hypotheses.values()
            .filter(|h| h.id != hypothesis_id)
            .filter(|h| h.hypothesis_type == target.hypothesis_type)
            .filter(|h| self.overlaps(&h.consequent, &target.consequent))
            .collect()
    }

    fn overlaps(&self, a: &Proposition, b: &Proposition) -> bool {
        a.subject == b.subject || a.predicate == b.predicate
    }

    /// Prune weak hypotheses
    pub fn prune(&mut self) -> usize {
        if !self.config.prune_weak {
            return 0;
        }

        let weak: Vec<u64> = self.hypotheses.iter()
            .filter(|(_, h)| {
                h.confidence < self.config.weak_threshold &&
                h.status != HypothesisStatus::Supported
            })
            .map(|(&id, _)| id)
            .collect();

        let count = weak.len();

        for id in weak {
            self.hypotheses.remove(&id);
        }

        count
    }

    /// Get hypothesis
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&Hypothesis> {
        self.hypotheses.get(&id)
    }

    /// Get all hypotheses by status
    #[inline]
    pub fn by_status(&self, status: HypothesisStatus) -> Vec<&Hypothesis> {
        self.hypotheses.values()
            .filter(|h| h.status == status)
            .collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &GeneratorStats {
        &self.stats
    }
}

impl Default for HypothesisGenerator {
    fn default() -> Self {
        Self::new(GeneratorConfig::default())
    }
}

// ============================================================================
// HYPOTHESIS BUILDER
// ============================================================================

/// Hypothesis builder
pub struct HypothesisBuilder<'a> {
    generator: &'a mut HypothesisGenerator,
    hypothesis_type: HypothesisType,
    description: String,
    antecedents: Vec<Proposition>,
    consequent: Option<Proposition>,
}

impl<'a> HypothesisBuilder<'a> {
    /// Create new builder
    pub fn new(generator: &'a mut HypothesisGenerator, hypothesis_type: HypothesisType) -> Self {
        Self {
            generator,
            hypothesis_type,
            description: String::new(),
            antecedents: Vec::new(),
            consequent: None,
        }
    }

    /// Set description
    #[inline(always)]
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.into();
        self
    }

    /// Add antecedent
    #[inline]
    pub fn given(mut self, subject: &str, predicate: &str, object: Option<&str>) -> Self {
        let prop = self.generator.create_proposition(subject, predicate, object);
        self.antecedents.push(prop);
        self
    }

    /// Set consequent
    #[inline]
    pub fn then(mut self, subject: &str, predicate: &str, object: Option<&str>) -> Self {
        let prop = self.generator.create_proposition(subject, predicate, object);
        self.consequent = Some(prop);
        self
    }

    /// Build hypothesis
    #[inline]
    pub fn build(self) -> Option<u64> {
        let consequent = self.consequent?;

        Some(self.generator.generate(
            self.hypothesis_type,
            &self.description,
            self.antecedents,
            consequent,
        ))
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hypothesis() {
        let mut gen = HypothesisGenerator::default();

        let antecedent = gen.create_proposition("rain", "causes", Some("wet_ground"));
        let consequent = gen.create_proposition("ground", "is", Some("wet"));

        let id = gen.generate(
            HypothesisType::Causal,
            "Rain causes wet ground",
            vec![antecedent],
            consequent,
        );

        let h = gen.get(id).unwrap();
        assert_eq!(h.hypothesis_type, HypothesisType::Causal);
        assert_eq!(h.status, HypothesisStatus::Proposed);
    }

    #[test]
    fn test_add_evidence() {
        let mut gen = HypothesisGenerator::default();

        let consequent = gen.create_proposition("x", "is", Some("y"));
        let id = gen.generate(HypothesisType::Predictive, "test", vec![], consequent);

        gen.add_evidence(id, "observation 1", EvidenceType::Observation, 0.8, "experiment");

        let h = gen.get(id).unwrap();
        assert!(!h.evidence.is_empty());
    }

    #[test]
    fn test_evaluate() {
        let mut gen = HypothesisGenerator::default();

        let consequent = gen.create_proposition("x", "is", Some("y"));
        let id = gen.generate(HypothesisType::Predictive, "test", vec![], consequent);

        // Add strong evidence
        gen.add_evidence(id, "e1", EvidenceType::Observation, 0.9, "s1");
        gen.add_evidence(id, "e2", EvidenceType::Experiment, 0.8, "s2");

        let status = gen.evaluate(id).unwrap();
        assert_eq!(status, HypothesisStatus::Supported);
    }

    #[test]
    fn test_builder() {
        let mut gen = HypothesisGenerator::default();

        let id = HypothesisBuilder::new(&mut gen, HypothesisType::Causal)
            .description("Smoking causes cancer")
            .given("smoking", "exposure", None)
            .then("cancer", "develops", None)
            .build();

        assert!(id.is_some());
    }

    #[test]
    fn test_prune() {
        let mut gen = HypothesisGenerator::new(GeneratorConfig {
            prune_weak: true,
            weak_threshold: 0.5,
            ..Default::default()
        });

        // Create weak hypothesis
        let consequent = gen.create_proposition("x", "is", Some("y"));
        gen.generate(HypothesisType::Predictive, "weak", vec![], consequent);

        let pruned = gen.prune();
        assert!(pruned > 0);
    }
}

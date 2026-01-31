//! # Confidence Assessment
//!
//! Evaluates and tracks confidence in decisions and predictions.
//! Implements calibration and uncertainty quantification.
//!
//! Part of Year 2 COGNITION - Decision/Confidence

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CONFIDENCE TYPES
// ============================================================================

/// Confidence assessment
#[derive(Debug, Clone)]
pub struct ConfidenceAssessment {
    /// Assessment ID
    pub id: u64,
    /// Subject
    pub subject: String,
    /// Type
    pub assessment_type: AssessmentType,
    /// Confidence level
    pub confidence: f64,
    /// Uncertainty
    pub uncertainty: Uncertainty,
    /// Evidence
    pub evidence: Vec<Evidence>,
    /// Created
    pub created: Timestamp,
}

/// Assessment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentType {
    Prediction,
    Decision,
    Belief,
    Estimate,
    Hypothesis,
}

/// Uncertainty
#[derive(Debug, Clone)]
pub struct Uncertainty {
    /// Type
    pub uncertainty_type: UncertaintyType,
    /// Lower bound
    pub lower: f64,
    /// Upper bound
    pub upper: f64,
    /// Variance
    pub variance: f64,
}

/// Uncertainty type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyType {
    Epistemic, // Knowledge uncertainty
    Aleatoric, // Random uncertainty
    Mixed,
}

/// Evidence
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Evidence ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Strength
    pub strength: f64,
    /// Direction
    pub direction: EvidenceDirection,
}

/// Evidence direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDirection {
    Supporting,
    Opposing,
    Neutral,
}

/// Calibration record
#[derive(Debug, Clone)]
pub struct CalibrationRecord {
    /// Record ID
    pub id: u64,
    /// Predicted confidence
    pub predicted: f64,
    /// Actual outcome
    pub actual: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Calibration result
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Overall calibration score
    pub score: f64,
    /// Brier score
    pub brier_score: f64,
    /// Expected calibration error
    pub ece: f64,
    /// Bucket calibrations
    pub buckets: Vec<CalibrationBucket>,
}

/// Calibration bucket
#[derive(Debug, Clone)]
pub struct CalibrationBucket {
    /// Bucket range
    pub range: (f64, f64),
    /// Mean predicted
    pub mean_predicted: f64,
    /// Mean actual
    pub mean_actual: f64,
    /// Count
    pub count: u64,
}

// ============================================================================
// CONFIDENCE ENGINE
// ============================================================================

/// Confidence engine
pub struct ConfidenceEngine {
    /// Assessments
    assessments: BTreeMap<u64, ConfidenceAssessment>,
    /// Calibration records
    calibration: Vec<CalibrationRecord>,
    /// Confidence adjustments
    adjustments: BTreeMap<AssessmentType, f64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ConfidenceConfig,
    /// Statistics
    stats: ConfidenceStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ConfidenceConfig {
    /// Number of calibration buckets
    pub num_buckets: usize,
    /// Maximum calibration history
    pub max_history: usize,
    /// Default uncertainty
    pub default_uncertainty: f64,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            num_buckets: 10,
            max_history: 1000,
            default_uncertainty: 0.1,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ConfidenceStats {
    /// Assessments made
    pub assessments_made: u64,
    /// Calibrations recorded
    pub calibrations_recorded: u64,
    /// Average confidence
    pub avg_confidence: f64,
}

impl ConfidenceEngine {
    /// Create new engine
    pub fn new(config: ConfidenceConfig) -> Self {
        Self {
            assessments: BTreeMap::new(),
            calibration: Vec::new(),
            adjustments: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ConfidenceStats::default(),
        }
    }

    /// Assess confidence
    pub fn assess(
        &mut self,
        subject: &str,
        assessment_type: AssessmentType,
        base_confidence: f64,
        evidence: Vec<Evidence>,
    ) -> ConfidenceAssessment {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Calculate adjusted confidence
        let evidence_factor = self.calculate_evidence_factor(&evidence);
        let adjustment = self
            .adjustments
            .get(&assessment_type)
            .copied()
            .unwrap_or(0.0);

        let confidence = (base_confidence * evidence_factor + adjustment).clamp(0.0, 1.0);

        // Calculate uncertainty
        let uncertainty = self.calculate_uncertainty(&evidence, confidence);

        let assessment = ConfidenceAssessment {
            id,
            subject: subject.into(),
            assessment_type,
            confidence,
            uncertainty,
            evidence,
            created: Timestamp::now(),
        };

        // Update statistics
        self.stats.assessments_made += 1;
        let n = self.stats.assessments_made as f64;
        self.stats.avg_confidence = (self.stats.avg_confidence * (n - 1.0) + confidence) / n;

        self.assessments.insert(id, assessment.clone());

        assessment
    }

    fn calculate_evidence_factor(&self, evidence: &[Evidence]) -> f64 {
        if evidence.is_empty() {
            return 1.0;
        }

        let mut supporting_strength = 0.0;
        let mut opposing_strength = 0.0;

        for e in evidence {
            match e.direction {
                EvidenceDirection::Supporting => {
                    supporting_strength += e.strength;
                },
                EvidenceDirection::Opposing => {
                    opposing_strength += e.strength;
                },
                EvidenceDirection::Neutral => {},
            }
        }

        let net = supporting_strength - opposing_strength;
        let total = supporting_strength + opposing_strength + 1.0;

        1.0 + (net / total) * 0.5
    }

    fn calculate_uncertainty(&self, evidence: &[Evidence], confidence: f64) -> Uncertainty {
        // Epistemic uncertainty from lack of evidence
        let evidence_count = evidence.len() as f64;
        let epistemic = 1.0 / (1.0 + evidence_count);

        // Aleatoric from confidence variance
        let aleatoric = (confidence * (1.0 - confidence)).sqrt();

        let variance = epistemic + aleatoric;
        let half_width = (variance * 2.0).sqrt();

        let uncertainty_type = if epistemic > aleatoric {
            UncertaintyType::Epistemic
        } else if aleatoric > epistemic {
            UncertaintyType::Aleatoric
        } else {
            UncertaintyType::Mixed
        };

        Uncertainty {
            uncertainty_type,
            lower: (confidence - half_width).max(0.0),
            upper: (confidence + half_width).min(1.0),
            variance,
        }
    }

    /// Record outcome for calibration
    pub fn record_outcome(&mut self, assessment_id: u64, actual: f64) {
        if let Some(assessment) = self.assessments.get(&assessment_id) {
            let record = CalibrationRecord {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                predicted: assessment.confidence,
                actual: actual.clamp(0.0, 1.0),
                timestamp: Timestamp::now(),
            };

            self.calibration.push(record);
            self.stats.calibrations_recorded += 1;

            // Limit history
            while self.calibration.len() > self.config.max_history {
                self.calibration.remove(0);
            }

            // Update adjustments based on calibration error
            let error = actual - assessment.confidence;
            let current = self
                .adjustments
                .entry(assessment.assessment_type)
                .or_insert(0.0);
            *current = (*current + error * 0.1).clamp(-0.5, 0.5);
        }
    }

    /// Compute calibration
    pub fn compute_calibration(&self) -> CalibrationResult {
        if self.calibration.is_empty() {
            return CalibrationResult {
                score: 1.0,
                brier_score: 0.0,
                ece: 0.0,
                buckets: Vec::new(),
            };
        }

        // Create buckets
        let bucket_size = 1.0 / self.config.num_buckets as f64;
        let mut buckets = Vec::new();

        for i in 0..self.config.num_buckets {
            let lower = i as f64 * bucket_size;
            let upper = (i + 1) as f64 * bucket_size;

            let bucket_records: Vec<_> = self
                .calibration
                .iter()
                .filter(|r| r.predicted >= lower && r.predicted < upper)
                .collect();

            if !bucket_records.is_empty() {
                let mean_predicted: f64 = bucket_records.iter().map(|r| r.predicted).sum::<f64>()
                    / bucket_records.len() as f64;

                let mean_actual: f64 = bucket_records.iter().map(|r| r.actual).sum::<f64>()
                    / bucket_records.len() as f64;

                buckets.push(CalibrationBucket {
                    range: (lower, upper),
                    mean_predicted,
                    mean_actual,
                    count: bucket_records.len() as u64,
                });
            }
        }

        // Calculate Brier score
        let brier_score: f64 = self
            .calibration
            .iter()
            .map(|r| (r.predicted - r.actual).powi(2))
            .sum::<f64>()
            / self.calibration.len() as f64;

        // Calculate Expected Calibration Error
        let total_samples = self.calibration.len() as f64;
        let ece: f64 = buckets
            .iter()
            .map(|b| (b.count as f64 / total_samples) * (b.mean_predicted - b.mean_actual).abs())
            .sum();

        // Overall calibration score (1 - ECE)
        let score = 1.0 - ece;

        CalibrationResult {
            score,
            brier_score,
            ece,
            buckets,
        }
    }

    /// Combine confidences
    pub fn combine(&self, confidences: &[f64], weights: &[f64]) -> f64 {
        if confidences.is_empty() {
            return 0.5;
        }

        let weights = if weights.len() != confidences.len() {
            // Equal weights
            vec![1.0; confidences.len()]
        } else {
            weights.to_vec()
        };

        let total_weight: f64 = weights.iter().sum();

        if total_weight == 0.0 {
            return confidences.iter().sum::<f64>() / confidences.len() as f64;
        }

        confidences
            .iter()
            .zip(weights.iter())
            .map(|(c, w)| c * w)
            .sum::<f64>()
            / total_weight
    }

    /// Get assessment
    pub fn get(&self, id: u64) -> Option<&ConfidenceAssessment> {
        self.assessments.get(&id)
    }

    /// Get by type
    pub fn by_type(&self, assessment_type: AssessmentType) -> Vec<&ConfidenceAssessment> {
        self.assessments
            .values()
            .filter(|a| a.assessment_type == assessment_type)
            .collect()
    }

    /// Get high confidence assessments
    pub fn high_confidence(&self, threshold: f64) -> Vec<&ConfidenceAssessment> {
        self.assessments
            .values()
            .filter(|a| a.confidence >= threshold)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &ConfidenceStats {
        &self.stats
    }
}

impl Default for ConfidenceEngine {
    fn default() -> Self {
        Self::new(ConfidenceConfig::default())
    }
}

// ============================================================================
// BUILDER
// ============================================================================

/// Evidence builder
pub struct EvidenceBuilder {
    evidence: Vec<Evidence>,
    next_id: u64,
}

impl EvidenceBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            evidence: Vec::new(),
            next_id: 1,
        }
    }

    /// Add supporting evidence
    pub fn supporting(mut self, description: &str, strength: f64) -> Self {
        self.evidence.push(Evidence {
            id: self.next_id,
            description: description.into(),
            strength: strength.clamp(0.0, 1.0),
            direction: EvidenceDirection::Supporting,
        });
        self.next_id += 1;
        self
    }

    /// Add opposing evidence
    pub fn opposing(mut self, description: &str, strength: f64) -> Self {
        self.evidence.push(Evidence {
            id: self.next_id,
            description: description.into(),
            strength: strength.clamp(0.0, 1.0),
            direction: EvidenceDirection::Opposing,
        });
        self.next_id += 1;
        self
    }

    /// Add neutral evidence
    pub fn neutral(mut self, description: &str, strength: f64) -> Self {
        self.evidence.push(Evidence {
            id: self.next_id,
            description: description.into(),
            strength: strength.clamp(0.0, 1.0),
            direction: EvidenceDirection::Neutral,
        });
        self.next_id += 1;
        self
    }

    /// Build evidence list
    pub fn build(self) -> Vec<Evidence> {
        self.evidence
    }
}

impl Default for EvidenceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assess() {
        let mut engine = ConfidenceEngine::default();

        let assessment = engine.assess(
            "test prediction",
            AssessmentType::Prediction,
            0.8,
            Vec::new(),
        );

        assert!(assessment.confidence > 0.0);
    }

    #[test]
    fn test_evidence() {
        let mut engine = ConfidenceEngine::default();

        let evidence = EvidenceBuilder::new()
            .supporting("Strong support", 0.9)
            .opposing("Weak opposition", 0.2)
            .build();

        let assessment = engine.assess("test", AssessmentType::Decision, 0.7, evidence);

        // Should be higher due to net positive evidence
        assert!(assessment.confidence > 0.7);
    }

    #[test]
    fn test_uncertainty() {
        let mut engine = ConfidenceEngine::default();

        let assessment = engine.assess("uncertain", AssessmentType::Hypothesis, 0.5, Vec::new());

        assert!(assessment.uncertainty.lower < assessment.confidence);
        assert!(assessment.uncertainty.upper > assessment.confidence);
    }

    #[test]
    fn test_calibration() {
        let mut engine = ConfidenceEngine::default();

        for i in 0..10 {
            let confidence = (i as f64 + 1.0) / 11.0;
            let assessment = engine.assess(
                &format!("pred{}", i),
                AssessmentType::Prediction,
                confidence,
                Vec::new(),
            );

            // Record matching outcome (perfect calibration)
            engine.record_outcome(assessment.id, confidence);
        }

        let calibration = engine.compute_calibration();
        assert!(calibration.score > 0.8); // Should be well calibrated
    }

    #[test]
    fn test_combine() {
        let engine = ConfidenceEngine::default();

        let combined = engine.combine(&[0.8, 0.6, 0.9], &[1.0, 1.0, 1.0]);
        assert!((combined - 0.7667).abs() < 0.01);
    }

    #[test]
    fn test_weighted_combine() {
        let engine = ConfidenceEngine::default();

        let combined = engine.combine(&[0.8, 0.4], &[3.0, 1.0]);
        assert!((combined - 0.7).abs() < 0.01);
    }
}

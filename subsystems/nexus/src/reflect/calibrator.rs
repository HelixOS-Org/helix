//! Calibrator — Prediction and decision calibration
//!
//! The calibrator tracks prediction accuracy and decision outcomes
//! to identify systematic biases and recommend adjustments.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;

// ============================================================================
// PREDICTION ID
// ============================================================================

/// Prediction ID type
define_id!(PredictionId, "Prediction identifier");

// ============================================================================
// DECISION ID
// ============================================================================

/// Decision ID type
define_id!(DecisionId, "Decision identifier");

// ============================================================================
// PREDICTION RECORD
// ============================================================================

/// Record of a prediction
#[derive(Debug, Clone)]
pub struct PredictionRecord {
    /// Prediction ID
    pub id: PredictionId,
    /// What was predicted
    pub prediction: String,
    /// Confidence
    pub confidence: Confidence,
    /// Predicted at
    pub predicted_at: Timestamp,
    /// Should happen by
    pub deadline: Timestamp,
    /// Actual outcome
    pub outcome: Option<PredictionOutcome>,
}

impl PredictionRecord {
    /// Create new prediction
    pub fn new(
        prediction: impl Into<String>,
        confidence: Confidence,
        deadline: Timestamp,
    ) -> Self {
        Self {
            id: PredictionId::generate(),
            prediction: prediction.into(),
            confidence,
            predicted_at: Timestamp::now(),
            deadline,
            outcome: None,
        }
    }

    /// Has outcome?
    pub fn has_outcome(&self) -> bool {
        self.outcome.is_some()
    }

    /// Was correct?
    pub fn was_correct(&self) -> Option<bool> {
        self.outcome.as_ref().map(|o| o.correct)
    }
}

/// Prediction outcome
#[derive(Debug, Clone)]
pub struct PredictionOutcome {
    /// Was prediction correct
    pub correct: bool,
    /// Recorded at
    pub recorded_at: Timestamp,
    /// Notes
    pub notes: Option<String>,
}

impl PredictionOutcome {
    /// Create correct outcome
    pub fn correct() -> Self {
        Self {
            correct: true,
            recorded_at: Timestamp::now(),
            notes: None,
        }
    }

    /// Create incorrect outcome
    pub fn incorrect(notes: impl Into<String>) -> Self {
        Self {
            correct: false,
            recorded_at: Timestamp::now(),
            notes: Some(notes.into()),
        }
    }
}

// ============================================================================
// DECISION RECORD
// ============================================================================

/// Record of a decision
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    /// Decision ID
    pub id: DecisionId,
    /// Action taken
    pub action: String,
    /// Confidence
    pub confidence: Confidence,
    /// Decided at
    pub decided_at: Timestamp,
    /// Outcome
    pub outcome: Option<DecisionOutcome>,
}

impl DecisionRecord {
    /// Create new decision record
    pub fn new(action: impl Into<String>, confidence: Confidence) -> Self {
        Self {
            id: DecisionId::generate(),
            action: action.into(),
            confidence,
            decided_at: Timestamp::now(),
            outcome: None,
        }
    }

    /// Has outcome?
    pub fn has_outcome(&self) -> bool {
        self.outcome.is_some()
    }

    /// Was successful?
    pub fn was_successful(&self) -> Option<bool> {
        self.outcome.as_ref().map(|o| o.successful)
    }
}

/// Decision outcome
#[derive(Debug, Clone)]
pub struct DecisionOutcome {
    /// Was decision successful
    pub successful: bool,
    /// Impact observed
    pub impact: String,
    /// Recorded at
    pub recorded_at: Timestamp,
}

impl DecisionOutcome {
    /// Create successful outcome
    pub fn success(impact: impl Into<String>) -> Self {
        Self {
            successful: true,
            impact: impact.into(),
            recorded_at: Timestamp::now(),
        }
    }

    /// Create failed outcome
    pub fn failure(impact: impl Into<String>) -> Self {
        Self {
            successful: false,
            impact: impact.into(),
            recorded_at: Timestamp::now(),
        }
    }
}

// ============================================================================
// CALIBRATION REPORT
// ============================================================================

/// Calibration report
#[derive(Debug, Clone)]
pub struct CalibrationReport {
    /// Prediction accuracy (0.0 to 1.0)
    pub prediction_accuracy: f32,
    /// Decision accuracy (0.0 to 1.0)
    pub decision_accuracy: f32,
    /// Calibration error
    pub calibration_error: f32,
    /// Is overconfident
    pub overconfidence: bool,
    /// Is underconfident
    pub underconfidence: bool,
    /// Recommendations
    pub recommendations: Vec<CalibrationRecommendation>,
}

impl CalibrationReport {
    /// Is well calibrated?
    pub fn is_calibrated(&self) -> bool {
        self.calibration_error < 0.1
    }

    /// Needs adjustment?
    pub fn needs_adjustment(&self) -> bool {
        self.overconfidence || self.underconfidence
    }
}

impl Default for CalibrationReport {
    fn default() -> Self {
        Self {
            prediction_accuracy: 0.0,
            decision_accuracy: 0.0,
            calibration_error: 0.0,
            overconfidence: false,
            underconfidence: false,
            recommendations: Vec::new(),
        }
    }
}

/// Calibration recommendation
#[derive(Debug, Clone)]
pub struct CalibrationRecommendation {
    /// Parameter to adjust
    pub parameter: String,
    /// Current value
    pub current_value: f32,
    /// Recommended value
    pub recommended_value: f32,
    /// Reason
    pub reason: String,
}

impl CalibrationRecommendation {
    /// Create new recommendation
    pub fn new(
        parameter: impl Into<String>,
        current_value: f32,
        recommended_value: f32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            parameter: parameter.into(),
            current_value,
            recommended_value,
            reason: reason.into(),
        }
    }
}

// ============================================================================
// CALIBRATOR
// ============================================================================

/// Calibrator - adjusts cognitive parameters
pub struct Calibrator {
    /// Prediction tracking
    predictions: Vec<PredictionRecord>,
    /// Decision tracking
    decisions: Vec<DecisionRecord>,
    /// Maximum records
    max_records: usize,
    /// Calibration adjustments made
    adjustments: AtomicU64,
}

impl Calibrator {
    /// Create new calibrator
    pub fn new(max_records: usize) -> Self {
        Self {
            predictions: Vec::new(),
            decisions: Vec::new(),
            max_records,
            adjustments: AtomicU64::new(0),
        }
    }

    /// Record a prediction
    pub fn record_prediction(&mut self, prediction: PredictionRecord) {
        self.predictions.push(prediction);
        if self.predictions.len() > self.max_records {
            self.predictions.remove(0);
        }
    }

    /// Record prediction outcome
    pub fn record_prediction_outcome(
        &mut self,
        id: PredictionId,
        correct: bool,
        notes: Option<String>,
    ) {
        if let Some(pred) = self.predictions.iter_mut().find(|p| p.id == id) {
            pred.outcome = Some(PredictionOutcome {
                correct,
                recorded_at: Timestamp::now(),
                notes,
            });
        }
    }

    /// Record a decision
    pub fn record_decision(&mut self, decision: DecisionRecord) {
        self.decisions.push(decision);
        if self.decisions.len() > self.max_records {
            self.decisions.remove(0);
        }
    }

    /// Record decision outcome
    pub fn record_decision_outcome(
        &mut self,
        id: DecisionId,
        successful: bool,
        impact: String,
    ) {
        if let Some(dec) = self.decisions.iter_mut().find(|d| d.id == id) {
            dec.outcome = Some(DecisionOutcome {
                successful,
                impact,
                recorded_at: Timestamp::now(),
            });
        }
    }

    /// Calculate prediction accuracy
    pub fn prediction_accuracy(&self) -> CalibrationReport {
        let with_outcome: Vec<_> = self
            .predictions
            .iter()
            .filter(|p| p.outcome.is_some())
            .collect();

        if with_outcome.is_empty() {
            return CalibrationReport::default();
        }

        let correct_count = with_outcome
            .iter()
            .filter(|p| p.outcome.as_ref().map(|o| o.correct).unwrap_or(false))
            .count();

        let prediction_accuracy = correct_count as f32 / with_outcome.len() as f32;

        // Calculate calibration error
        let avg_confidence: f32 = with_outcome.iter().map(|p| p.confidence.value()).sum::<f32>()
            / with_outcome.len() as f32;

        let calibration_error = (avg_confidence - prediction_accuracy).abs();

        // Calculate decision accuracy
        let decisions_with_outcome: Vec<_> = self
            .decisions
            .iter()
            .filter(|d| d.outcome.is_some())
            .collect();

        let decision_accuracy = if decisions_with_outcome.is_empty() {
            0.0
        } else {
            let successful = decisions_with_outcome
                .iter()
                .filter(|d| d.outcome.as_ref().map(|o| o.successful).unwrap_or(false))
                .count();
            successful as f32 / decisions_with_outcome.len() as f32
        };

        let overconfidence = avg_confidence > prediction_accuracy + 0.1;
        let underconfidence = avg_confidence < prediction_accuracy - 0.1;

        let mut recommendations = Vec::new();

        if overconfidence {
            recommendations.push(CalibrationRecommendation {
                parameter: String::from("confidence_threshold"),
                current_value: avg_confidence,
                recommended_value: prediction_accuracy,
                reason: String::from("System is overconfident - lower confidence thresholds"),
            });
        }

        if underconfidence {
            recommendations.push(CalibrationRecommendation {
                parameter: String::from("confidence_threshold"),
                current_value: avg_confidence,
                recommended_value: prediction_accuracy,
                reason: String::from("System is underconfident - raise confidence thresholds"),
            });
        }

        CalibrationReport {
            prediction_accuracy,
            decision_accuracy,
            calibration_error,
            overconfidence,
            underconfidence,
            recommendations,
        }
    }

    /// Get prediction count
    pub fn prediction_count(&self) -> usize {
        self.predictions.len()
    }

    /// Get decision count
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }

    /// Get statistics
    pub fn stats(&self) -> CalibratorStats {
        CalibratorStats {
            predictions_tracked: self.predictions.len(),
            decisions_tracked: self.decisions.len(),
            adjustments_made: self.adjustments.load(Ordering::Relaxed),
        }
    }
}

impl Default for Calibrator {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Calibrator statistics
#[derive(Debug, Clone)]
pub struct CalibratorStats {
    /// Predictions tracked
    pub predictions_tracked: usize,
    /// Decisions tracked
    pub decisions_tracked: usize,
    /// Adjustments made
    pub adjustments_made: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrator() {
        let mut calibrator = Calibrator::new(100);

        // Record predictions
        for i in 0..10 {
            let pred = PredictionRecord {
                id: PredictionId::new(i),
                prediction: String::from("test"),
                confidence: Confidence::new(0.8),
                predicted_at: Timestamp::now(),
                deadline: Timestamp::now(),
                outcome: Some(PredictionOutcome {
                    correct: i < 8, // 80% correct
                    recorded_at: Timestamp::now(),
                    notes: None,
                }),
            };
            calibrator.record_prediction(pred);
        }

        let report = calibrator.prediction_accuracy();
        assert!((report.prediction_accuracy - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_prediction_record() {
        let pred = PredictionRecord::new(
            "CPU will spike",
            Confidence::new(0.9),
            Timestamp::now(),
        );
        assert!(!pred.has_outcome());
    }

    #[test]
    fn test_decision_record() {
        let dec = DecisionRecord::new("Throttle process", Confidence::new(0.85));
        assert!(!dec.has_outcome());
    }

    #[test]
    fn test_calibration_report() {
        let report = CalibrationReport::default();
        assert!(!report.needs_adjustment());
    }
}

//! # Meta Reflection
//!
//! Implements reflection about the reflection process itself.
//! Evaluates reasoning quality and metacognitive awareness.
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
// META REFLECTION TYPES
// ============================================================================

/// Metacognitive state
#[derive(Debug, Clone)]
pub struct MetacognitiveState {
    /// State ID
    pub id: u64,
    /// Confidence in reasoning
    pub reasoning_confidence: f64,
    /// Knowledge awareness
    pub knowledge_awareness: f64,
    /// Uncertainty acknowledgment
    pub uncertainty_level: f64,
    /// Bias awareness
    pub bias_awareness: f64,
    /// Active strategies
    pub active_strategies: Vec<String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Reflection quality
#[derive(Debug, Clone)]
pub struct ReflectionQuality {
    /// Quality ID
    pub id: u64,
    /// Depth
    pub depth: u32,
    /// Accuracy
    pub accuracy: f64,
    /// Completeness
    pub completeness: f64,
    /// Relevance
    pub relevance: f64,
    /// Overall score
    pub overall: f64,
}

/// Cognitive bias
#[derive(Debug, Clone)]
pub struct CognitiveBias {
    /// Bias ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub bias_type: BiasType,
    /// Severity
    pub severity: f64,
    /// Evidence
    pub evidence: Vec<String>,
    /// Detected
    pub detected: Timestamp,
}

/// Bias type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiasType {
    Confirmation,
    Anchoring,
    Availability,
    Hindsight,
    Overconfidence,
    Framing,
    Sunk_cost,
    Groupthink,
    Recency,
    Attribution,
}

/// Reasoning trace
#[derive(Debug, Clone)]
pub struct ReasoningTrace {
    /// Trace ID
    pub id: u64,
    /// Steps
    pub steps: Vec<ReasoningStep>,
    /// Quality assessment
    pub quality: Option<ReflectionQuality>,
    /// Created
    pub created: Timestamp,
}

/// Reasoning step
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    /// Step number
    pub step: u32,
    /// Description
    pub description: String,
    /// Confidence
    pub confidence: f64,
    /// Justification
    pub justification: String,
}

/// Self-assessment
#[derive(Debug, Clone)]
pub struct SelfAssessment {
    /// Assessment ID
    pub id: u64,
    /// Area
    pub area: String,
    /// Perceived ability
    pub perceived: f64,
    /// Actual performance
    pub actual: Option<f64>,
    /// Calibration error
    pub calibration_error: Option<f64>,
    /// Timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// META REFLECTOR
// ============================================================================

/// Meta reflector
pub struct MetaReflector {
    /// States
    states: Vec<MetacognitiveState>,
    /// Biases detected
    biases: Vec<CognitiveBias>,
    /// Traces
    traces: BTreeMap<u64, ReasoningTrace>,
    /// Assessments
    assessments: Vec<SelfAssessment>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: MetaConfig,
    /// Statistics
    stats: MetaStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MetaConfig {
    /// Bias detection enabled
    pub detect_biases: bool,
    /// Trace depth
    pub trace_depth: usize,
    /// Calibration window
    pub calibration_window: usize,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            detect_biases: true,
            trace_depth: 10,
            calibration_window: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct MetaStats {
    /// Reflections
    pub reflections: u64,
    /// Biases detected
    pub biases_detected: u64,
    /// Traces recorded
    pub traces_recorded: u64,
}

impl MetaReflector {
    /// Create new reflector
    pub fn new(config: MetaConfig) -> Self {
        Self {
            states: Vec::new(),
            biases: Vec::new(),
            traces: BTreeMap::new(),
            assessments: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: MetaStats::default(),
        }
    }

    /// Capture metacognitive state
    pub fn capture_state(&mut self) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Calculate awareness metrics
        let reasoning_confidence = self.calculate_reasoning_confidence();
        let knowledge_awareness = self.calculate_knowledge_awareness();
        let uncertainty_level = self.calculate_uncertainty();
        let bias_awareness = self.calculate_bias_awareness();

        let state = MetacognitiveState {
            id,
            reasoning_confidence,
            knowledge_awareness,
            uncertainty_level,
            bias_awareness,
            active_strategies: self.get_active_strategies(),
            timestamp: Timestamp::now(),
        };

        self.states.push(state);
        self.stats.reflections += 1;

        id
    }

    fn calculate_reasoning_confidence(&self) -> f64 {
        if self.traces.is_empty() {
            return 0.5;
        }

        let recent_traces: Vec<_> = self.traces.values()
            .rev()
            .take(10)
            .collect();

        let avg_confidence: f64 = recent_traces.iter()
            .flat_map(|t| t.steps.iter())
            .map(|s| s.confidence)
            .sum::<f64>() / recent_traces.len().max(1) as f64;

        avg_confidence.clamp(0.0, 1.0)
    }

    fn calculate_knowledge_awareness(&self) -> f64 {
        // Based on calibration of self-assessments
        let recent: Vec<_> = self.assessments.iter()
            .filter(|a| a.actual.is_some())
            .rev()
            .take(20)
            .collect();

        if recent.is_empty() {
            return 0.5;
        }

        let avg_error: f64 = recent.iter()
            .filter_map(|a| a.calibration_error)
            .sum::<f64>() / recent.len() as f64;

        // Lower error = higher awareness
        (1.0 - avg_error).clamp(0.0, 1.0)
    }

    fn calculate_uncertainty(&self) -> f64 {
        // Measure uncertainty acknowledgment
        let recent_traces: Vec<_> = self.traces.values()
            .rev()
            .take(10)
            .collect();

        if recent_traces.is_empty() {
            return 0.5;
        }

        // Count steps with moderate confidence (uncertainty acknowledged)
        let total_steps: usize = recent_traces.iter()
            .map(|t| t.steps.len())
            .sum();

        let uncertain_steps: usize = recent_traces.iter()
            .flat_map(|t| t.steps.iter())
            .filter(|s| s.confidence > 0.3 && s.confidence < 0.8)
            .count();

        if total_steps == 0 {
            0.5
        } else {
            uncertain_steps as f64 / total_steps as f64
        }
    }

    fn calculate_bias_awareness(&self) -> f64 {
        let recent_biases = self.biases.iter()
            .rev()
            .take(20)
            .count();

        // More detected biases = better awareness
        (recent_biases as f64 / 10.0).min(1.0)
    }

    fn get_active_strategies(&self) -> Vec<String> {
        let mut strategies = Vec::new();

        if self.config.detect_biases {
            strategies.push("bias_detection".into());
        }

        if !self.traces.is_empty() {
            strategies.push("reasoning_trace".into());
        }

        if !self.assessments.is_empty() {
            strategies.push("self_calibration".into());
        }

        strategies
    }

    /// Start reasoning trace
    pub fn start_trace(&mut self) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let trace = ReasoningTrace {
            id,
            steps: Vec::new(),
            quality: None,
            created: Timestamp::now(),
        };

        self.traces.insert(id, trace);
        self.stats.traces_recorded += 1;

        id
    }

    /// Add reasoning step
    pub fn add_step(
        &mut self,
        trace_id: u64,
        description: &str,
        confidence: f64,
        justification: &str,
    ) {
        if let Some(trace) = self.traces.get_mut(&trace_id) {
            let step = ReasoningStep {
                step: trace.steps.len() as u32 + 1,
                description: description.into(),
                confidence: confidence.clamp(0.0, 1.0),
                justification: justification.into(),
            };

            trace.steps.push(step);
        }
    }

    /// Evaluate trace quality
    pub fn evaluate_trace(&mut self, trace_id: u64) -> Option<ReflectionQuality> {
        let trace = self.traces.get(&trace_id)?;

        let depth = trace.steps.len() as u32;

        // Calculate metrics
        let accuracy = self.estimate_accuracy(trace);
        let completeness = self.estimate_completeness(trace);
        let relevance = self.estimate_relevance(trace);

        let overall = (accuracy + completeness + relevance) / 3.0;

        let quality = ReflectionQuality {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            depth,
            accuracy,
            completeness,
            relevance,
            overall,
        };

        // Store in trace
        if let Some(t) = self.traces.get_mut(&trace_id) {
            t.quality = Some(quality.clone());
        }

        Some(quality)
    }

    fn estimate_accuracy(&self, trace: &ReasoningTrace) -> f64 {
        // Based on confidence and justification presence
        if trace.steps.is_empty() {
            return 0.0;
        }

        let justified: usize = trace.steps.iter()
            .filter(|s| !s.justification.is_empty())
            .count();

        let avg_confidence: f64 = trace.steps.iter()
            .map(|s| s.confidence)
            .sum::<f64>() / trace.steps.len() as f64;

        let justification_ratio = justified as f64 / trace.steps.len() as f64;

        (avg_confidence * 0.5 + justification_ratio * 0.5).clamp(0.0, 1.0)
    }

    fn estimate_completeness(&self, trace: &ReasoningTrace) -> f64 {
        // Based on trace depth
        let expected_depth = self.config.trace_depth;
        (trace.steps.len() as f64 / expected_depth as f64).min(1.0)
    }

    fn estimate_relevance(&self, trace: &ReasoningTrace) -> f64 {
        // Simplified: assume all steps are relevant
        if trace.steps.is_empty() {
            0.0
        } else {
            0.8
        }
    }

    /// Detect bias
    pub fn detect_bias(
        &mut self,
        name: &str,
        bias_type: BiasType,
        severity: f64,
        evidence: Vec<String>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let bias = CognitiveBias {
            id,
            name: name.into(),
            bias_type,
            severity: severity.clamp(0.0, 1.0),
            evidence,
            detected: Timestamp::now(),
        };

        self.biases.push(bias);
        self.stats.biases_detected += 1;

        id
    }

    /// Check for confirmation bias
    pub fn check_confirmation_bias(&mut self, trace_id: u64) -> Option<u64> {
        let trace = self.traces.get(&trace_id)?;

        // Check if all steps have high confidence
        let all_confident = trace.steps.iter()
            .all(|s| s.confidence > 0.8);

        if all_confident && trace.steps.len() > 3 {
            Some(self.detect_bias(
                "Potential confirmation bias",
                BiasType::Confirmation,
                0.6,
                vec!["All steps have high confidence".into()],
            ))
        } else {
            None
        }
    }

    /// Self-assess
    pub fn self_assess(&mut self, area: &str, perceived_ability: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let assessment = SelfAssessment {
            id,
            area: area.into(),
            perceived: perceived_ability.clamp(0.0, 1.0),
            actual: None,
            calibration_error: None,
            timestamp: Timestamp::now(),
        };

        self.assessments.push(assessment);
        id
    }

    /// Update assessment with actual performance
    pub fn update_assessment(&mut self, id: u64, actual: f64) {
        if let Some(a) = self.assessments.iter_mut().find(|a| a.id == id) {
            a.actual = Some(actual.clamp(0.0, 1.0));
            a.calibration_error = Some((a.perceived - actual).abs());
        }
    }

    /// Calculate calibration
    pub fn calculate_calibration(&self) -> f64 {
        let recent: Vec<_> = self.assessments.iter()
            .filter(|a| a.calibration_error.is_some())
            .rev()
            .take(self.config.calibration_window)
            .collect();

        if recent.is_empty() {
            return 0.5;
        }

        let avg_error: f64 = recent.iter()
            .filter_map(|a| a.calibration_error)
            .sum::<f64>() / recent.len() as f64;

        (1.0 - avg_error).clamp(0.0, 1.0)
    }

    /// Get current state
    pub fn current_state(&self) -> Option<&MetacognitiveState> {
        self.states.last()
    }

    /// Get detected biases
    pub fn detected_biases(&self) -> &[CognitiveBias] {
        &self.biases
    }

    /// Get statistics
    pub fn stats(&self) -> &MetaStats {
        &self.stats
    }
}

impl Default for MetaReflector {
    fn default() -> Self {
        Self::new(MetaConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_state() {
        let mut reflector = MetaReflector::default();

        let id = reflector.capture_state();
        assert!(reflector.current_state().is_some());
    }

    #[test]
    fn test_reasoning_trace() {
        let mut reflector = MetaReflector::default();

        let trace_id = reflector.start_trace();
        reflector.add_step(trace_id, "Step 1", 0.8, "Because X");
        reflector.add_step(trace_id, "Step 2", 0.7, "Because Y");

        let trace = reflector.traces.get(&trace_id).unwrap();
        assert_eq!(trace.steps.len(), 2);
    }

    #[test]
    fn test_evaluate_trace() {
        let mut reflector = MetaReflector::default();

        let trace_id = reflector.start_trace();
        reflector.add_step(trace_id, "Analysis", 0.8, "Based on data");
        reflector.add_step(trace_id, "Conclusion", 0.9, "Follows from analysis");

        let quality = reflector.evaluate_trace(trace_id).unwrap();
        assert!(quality.overall > 0.0);
    }

    #[test]
    fn test_detect_bias() {
        let mut reflector = MetaReflector::default();

        let id = reflector.detect_bias(
            "Test bias",
            BiasType::Confirmation,
            0.7,
            vec!["evidence".into()],
        );

        assert_eq!(reflector.detected_biases().len(), 1);
    }

    #[test]
    fn test_self_assessment() {
        let mut reflector = MetaReflector::default();

        let id = reflector.self_assess("reasoning", 0.8);
        reflector.update_assessment(id, 0.6);

        let calibration = reflector.calculate_calibration();
        assert!(calibration < 1.0); // Some error expected
    }

    #[test]
    fn test_confirmation_bias_check() {
        let mut reflector = MetaReflector::default();

        let trace_id = reflector.start_trace();
        for i in 0..5 {
            reflector.add_step(trace_id, &format!("Step {}", i), 0.95, "Sure");
        }

        let bias = reflector.check_confirmation_bias(trace_id);
        assert!(bias.is_some()); // Should detect potential bias
    }
}

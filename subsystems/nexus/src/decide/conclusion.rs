//! Conclusion types from REASON domain
//!
//! Conclusions are the inputs to the DECIDE domain, coming from REASON.
//! They represent diagnoses, predictions, and insights that need decisions.

use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;

// ============================================================================
// CONCLUSION
// ============================================================================

/// A conclusion from the reasoning domain
#[derive(Debug, Clone)]
pub struct Conclusion {
    /// Conclusion ID
    pub id: ConclusionId,
    /// Type of conclusion
    pub conclusion_type: ConclusionType,
    /// Severity
    pub severity: Severity,
    /// Confidence in this conclusion
    pub confidence: Confidence,
    /// Summary
    pub summary: String,
    /// Detailed explanation
    pub explanation: String,
    /// Evidence supporting this conclusion
    pub evidence: Vec<EvidenceItem>,
    /// Suggested actions
    pub suggested_actions: Vec<SuggestedAction>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Time to live
    pub ttl: Duration,
}

impl Conclusion {
    /// Create a new conclusion
    pub fn new(
        conclusion_type: ConclusionType,
        severity: Severity,
        confidence: Confidence,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            id: ConclusionId::generate(),
            conclusion_type,
            severity,
            confidence,
            summary: summary.into(),
            explanation: String::new(),
            evidence: Vec::new(),
            suggested_actions: Vec::new(),
            timestamp: Timestamp::now(),
            ttl: Duration::from_secs(60),
        }
    }

    /// Set explanation
    #[inline(always)]
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = explanation.into();
        self
    }

    /// Add evidence
    #[inline(always)]
    pub fn with_evidence(mut self, evidence: EvidenceItem) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Add suggested action
    #[inline(always)]
    pub fn with_suggestion(mut self, action: SuggestedAction) -> Self {
        self.suggested_actions.push(action);
        self
    }

    /// Set TTL
    #[inline(always)]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Check if conclusion has expired
    #[inline(always)]
    pub fn is_expired(&self, now: Timestamp) -> bool {
        let expiry = self.timestamp.as_nanos() + self.ttl.as_nanos();
        now.as_nanos() > expiry
    }

    /// Get the strongest evidence
    #[inline]
    pub fn strongest_evidence(&self) -> Option<&EvidenceItem> {
        self.evidence.iter().max_by(|a, b| {
            a.weight.partial_cmp(&b.weight).unwrap_or(core::cmp::Ordering::Equal)
        })
    }
}

// ============================================================================
// CONCLUSION TYPE
// ============================================================================

/// Conclusion type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConclusionType {
    /// Something is wrong
    Diagnosis,
    /// Something will go wrong
    Prediction,
    /// A trend has been identified
    Trend,
    /// An opportunity for optimization
    Opportunity,
    /// A warning condition
    Warning,
    /// An information notice
    Information,
}

impl ConclusionType {
    /// Is this conclusion actionable?
    #[inline(always)]
    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Diagnosis | Self::Prediction | Self::Warning)
    }

    /// Is this conclusion urgent?
    #[inline(always)]
    pub fn is_urgent(&self) -> bool {
        matches!(self, Self::Diagnosis | Self::Prediction)
    }

    /// Get display name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Diagnosis => "Diagnosis",
            Self::Prediction => "Prediction",
            Self::Trend => "Trend",
            Self::Opportunity => "Opportunity",
            Self::Warning => "Warning",
            Self::Information => "Information",
        }
    }
}

// ============================================================================
// EVIDENCE
// ============================================================================

/// Evidence item supporting a conclusion
#[derive(Debug, Clone)]
pub struct EvidenceItem {
    /// Evidence type
    pub evidence_type: EvidenceType,
    /// Source of the evidence
    pub source: String,
    /// Value or description
    pub value: String,
    /// Weight (0.0 to 1.0)
    pub weight: f32,
}

impl EvidenceItem {
    /// Create new evidence
    pub fn new(evidence_type: EvidenceType, source: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            evidence_type,
            source: source.into(),
            value: value.into(),
            weight: 0.5,
        }
    }

    /// Set weight
    #[inline(always)]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Is this strong evidence?
    #[inline(always)]
    pub fn is_strong(&self) -> bool {
        self.weight >= 0.75
    }
}

/// Evidence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    /// Direct observation
    Observation,
    /// Metric measurement
    Metric,
    /// Pattern detection
    Pattern,
    /// Correlation analysis
    Correlation,
    /// Historical data
    History,
    /// Rule-based inference
    Rule,
}

impl EvidenceType {
    /// Get base weight for this type
    #[inline]
    pub fn base_weight(&self) -> f32 {
        match self {
            Self::Observation => 0.9,
            Self::Metric => 0.85,
            Self::Pattern => 0.7,
            Self::Correlation => 0.6,
            Self::History => 0.5,
            Self::Rule => 0.75,
        }
    }
}

// ============================================================================
// SUGGESTED ACTION
// ============================================================================

use super::options::{ActionType, Impact};

/// Suggested action from reasoning
#[derive(Debug, Clone)]
pub struct SuggestedAction {
    /// Action type
    pub action_type: ActionType,
    /// Description
    pub description: String,
    /// Expected impact
    pub expected_impact: Impact,
    /// Confidence in this suggestion
    pub confidence: Confidence,
}

impl SuggestedAction {
    /// Create a new suggestion
    pub fn new(action_type: ActionType, description: impl Into<String>) -> Self {
        Self {
            action_type,
            description: description.into(),
            expected_impact: Impact::default(),
            confidence: Confidence::MEDIUM,
        }
    }

    /// Set expected impact
    #[inline(always)]
    pub fn with_impact(mut self, impact: Impact) -> Self {
        self.expected_impact = impact;
        self
    }

    /// Set confidence
    #[inline(always)]
    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conclusion_creation() {
        let conclusion = Conclusion::new(
            ConclusionType::Diagnosis,
            Severity::Error,
            Confidence::HIGH,
            "Test conclusion",
        );

        assert!(conclusion.conclusion_type.is_actionable());
        assert!(conclusion.evidence.is_empty());
    }

    #[test]
    fn test_evidence_weight() {
        let evidence = EvidenceItem::new(EvidenceType::Observation, "cpu_probe", "100%")
            .with_weight(0.95);

        assert!(evidence.is_strong());
    }

    #[test]
    fn test_conclusion_type_properties() {
        assert!(ConclusionType::Diagnosis.is_urgent());
        assert!(!ConclusionType::Information.is_actionable());
    }
}

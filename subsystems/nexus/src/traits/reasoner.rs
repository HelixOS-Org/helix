//! Reasoner Traits
//!
//! Traits for the REASON domain - causal analysis and inference.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use super::component::NexusComponent;
use crate::types::{Confidence, Duration, NexusResult};

// ============================================================================
// REASONER TRAIT
// ============================================================================

/// Trait for reasoning components
pub trait Reasoner: NexusComponent {
    /// Evidence type
    type Evidence;
    /// Conclusion type
    type Conclusion;

    /// Reason about evidence and draw conclusions
    fn reason(&self, evidence: &[Self::Evidence]) -> NexusResult<Vec<Self::Conclusion>>;

    /// Get confidence in reasoning
    fn confidence(&self) -> Confidence;

    /// Explain a conclusion
    fn explain(&self, conclusion: &Self::Conclusion) -> String;

    /// Validate a conclusion
    fn validate(&self, conclusion: &Self::Conclusion) -> bool;
}

// ============================================================================
// CAUSAL REASONER TRAIT
// ============================================================================

/// Causal reasoner trait
pub trait CausalReasoner: Reasoner {
    /// Find causes for an effect
    fn find_causes(&self, effect: &Self::Evidence) -> Vec<CausalLink>;

    /// Predict effects of a cause
    fn predict_effects(&self, cause: &Self::Evidence) -> Vec<CausalLink>;

    /// Run counterfactual simulation
    fn counterfactual(
        &self,
        scenario: &Self::Evidence,
        intervention: &str,
    ) -> NexusResult<Self::Conclusion>;

    /// Get causal chain
    fn causal_chain(&self, start: &Self::Evidence, end: &Self::Evidence) -> Vec<CausalLink>;
}

/// Causal link between events
#[derive(Debug, Clone)]
pub struct CausalLink {
    /// Cause event description
    pub cause: String,
    /// Effect event description
    pub effect: String,
    /// Probability of causation (0.0 to 1.0)
    pub probability: f32,
    /// Time delay between cause and effect
    pub delay: Duration,
    /// Link type
    pub link_type: CausalLinkType,
    /// Evidence strength
    pub evidence_strength: Confidence,
}

impl CausalLink {
    /// Create new causal link
    pub fn new(cause: impl Into<String>, effect: impl Into<String>, probability: f32) -> Self {
        Self {
            cause: cause.into(),
            effect: effect.into(),
            probability: probability.clamp(0.0, 1.0),
            delay: Duration::ZERO,
            link_type: CausalLinkType::Direct,
            evidence_strength: Confidence::MEDIUM,
        }
    }

    /// With delay
    #[inline(always)]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// With link type
    #[inline(always)]
    pub fn with_type(mut self, link_type: CausalLinkType) -> Self {
        self.link_type = link_type;
        self
    }
}

/// Types of causal links
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CausalLinkType {
    /// Direct causation (A causes B)
    Direct,
    /// Indirect causation (A causes C causes B)
    Indirect,
    /// Contributing factor (A increases probability of B)
    Contributing,
    /// Necessary condition (B cannot happen without A)
    Necessary,
    /// Sufficient condition (A alone causes B)
    Sufficient,
    /// Correlation (A and B occur together)
    Correlation,
}

impl CausalLinkType {
    /// Get type name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Indirect => "indirect",
            Self::Contributing => "contributing",
            Self::Necessary => "necessary",
            Self::Sufficient => "sufficient",
            Self::Correlation => "correlation",
        }
    }

    /// Is this a true causal link (not just correlation)?
    #[inline(always)]
    pub const fn is_causal(&self) -> bool {
        !matches!(self, Self::Correlation)
    }
}

// ============================================================================
// TEMPORAL REASONER TRAIT
// ============================================================================

/// Temporal reasoner trait
pub trait TemporalReasoner: Reasoner {
    /// Predict future values
    fn predict(&self, horizon: Duration) -> NexusResult<Self::Conclusion>;

    /// Detect trend in data
    fn detect_trend(&self, data: &[Self::Evidence]) -> TrendInfo;

    /// Forecast with confidence intervals
    fn forecast(&self, horizon: Duration) -> NexusResult<Forecast>;

    /// Detect seasonality
    fn detect_seasonality(&self, data: &[Self::Evidence]) -> Option<Seasonality>;
}

/// Trend information
#[derive(Debug, Clone)]
pub struct TrendInfo {
    /// Direction
    pub direction: TrendDirection,
    /// Slope (rate of change)
    pub slope: f64,
    /// Confidence in trend
    pub confidence: Confidence,
    /// Is trend accelerating?
    pub accelerating: bool,
}

impl TrendInfo {
    /// Create stable trend
    #[inline]
    pub fn stable() -> Self {
        Self {
            direction: TrendDirection::Stable,
            slope: 0.0,
            confidence: Confidence::HIGH,
            accelerating: false,
        }
    }

    /// Is significant trend?
    #[inline(always)]
    pub fn is_significant(&self) -> bool {
        self.confidence.meets(Confidence::MEDIUM)
            && !matches!(self.direction, TrendDirection::Stable)
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrendDirection {
    /// Values increasing
    Rising,
    /// Values decreasing
    Falling,
    /// Values stable
    Stable,
    /// Values oscillating
    Oscillating,
    /// Cannot determine
    Unknown,
}

impl TrendDirection {
    /// Get direction name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rising => "rising",
            Self::Falling => "falling",
            Self::Stable => "stable",
            Self::Oscillating => "oscillating",
            Self::Unknown => "unknown",
        }
    }
}

/// Forecast result
#[derive(Debug, Clone)]
pub struct Forecast {
    /// Predicted value
    pub value: f64,
    /// Lower bound (e.g., 5th percentile)
    pub lower_bound: f64,
    /// Upper bound (e.g., 95th percentile)
    pub upper_bound: f64,
    /// Confidence in forecast
    pub confidence: Confidence,
    /// Forecast horizon
    pub horizon: Duration,
}

impl Forecast {
    /// Get prediction interval width
    #[inline(always)]
    pub fn interval_width(&self) -> f64 {
        self.upper_bound - self.lower_bound
    }

    /// Is value within interval?
    #[inline(always)]
    pub fn contains(&self, value: f64) -> bool {
        value >= self.lower_bound && value <= self.upper_bound
    }
}

/// Seasonality information
#[derive(Debug, Clone)]
pub struct Seasonality {
    /// Period of seasonality
    pub period: Duration,
    /// Amplitude
    pub amplitude: f64,
    /// Phase offset
    pub phase: f64,
    /// Confidence
    pub confidence: Confidence,
}

// ============================================================================
// HYPOTHESIS GENERATOR TRAIT
// ============================================================================

/// Hypothesis generator trait
pub trait HypothesisGenerator: NexusComponent {
    /// Evidence type
    type Evidence;
    /// Hypothesis type
    type Hypothesis;

    /// Generate hypotheses from evidence
    fn generate(&self, evidence: &[Self::Evidence]) -> Vec<Self::Hypothesis>;

    /// Test a hypothesis
    fn test(&self, hypothesis: &Self::Hypothesis, evidence: &[Self::Evidence]) -> HypothesisResult;

    /// Rank hypotheses by plausibility
    fn rank(&self, hypotheses: &[Self::Hypothesis]) -> Vec<(Self::Hypothesis, f32)>;
}

/// Hypothesis test result
#[derive(Debug, Clone)]
pub struct HypothesisResult {
    /// Is hypothesis supported?
    pub supported: bool,
    /// Evidence strength
    pub strength: Confidence,
    /// Supporting evidence
    pub supporting: Vec<String>,
    /// Contradicting evidence
    pub contradicting: Vec<String>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_link() {
        let link = CausalLink::new("high_load", "slow_response", 0.85)
            .with_delay(Duration::from_millis(100))
            .with_type(CausalLinkType::Direct);

        assert_eq!(link.probability, 0.85);
        assert!(link.link_type.is_causal());
    }

    #[test]
    fn test_trend_info() {
        let trend = TrendInfo::stable();
        assert!(!trend.is_significant());
    }

    #[test]
    fn test_forecast() {
        let forecast = Forecast {
            value: 50.0,
            lower_bound: 40.0,
            upper_bound: 60.0,
            confidence: Confidence::HIGH,
            horizon: Duration::from_secs(60),
        };

        assert_eq!(forecast.interval_width(), 20.0);
        assert!(forecast.contains(50.0));
        assert!(!forecast.contains(70.0));
    }
}

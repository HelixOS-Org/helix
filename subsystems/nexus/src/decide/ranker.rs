//! Priority Ranker — Scores and ranks options
//!
//! The ranker evaluates options based on multiple criteria including
//! urgency, impact, confidence, cost, and risk to produce a ranked list.

use alloc::vec::Vec;

use crate::types::*;
use super::options::{Option, ActionCost, ExpectedOutcome, RiskLevel};
use super::policy::PolicyResult;

// ============================================================================
// RANKING WEIGHTS
// ============================================================================

/// Ranking weights
#[derive(Debug, Clone)]
pub struct RankingWeights {
    /// Weight for urgency (0.0 to 1.0)
    pub urgency: f32,
    /// Weight for impact (0.0 to 1.0)
    pub impact: f32,
    /// Weight for confidence (0.0 to 1.0)
    pub confidence: f32,
    /// Weight for cost (0.0 to 1.0, lower cost is better)
    pub cost: f32,
    /// Weight for risk (0.0 to 1.0, lower risk is better)
    pub risk: f32,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            urgency: 0.25,
            impact: 0.25,
            confidence: 0.20,
            cost: 0.15,
            risk: 0.15,
        }
    }
}

impl RankingWeights {
    /// Create new weights
    pub fn new(urgency: f32, impact: f32, confidence: f32, cost: f32, risk: f32) -> Self {
        Self {
            urgency: urgency.clamp(0.0, 1.0),
            impact: impact.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            cost: cost.clamp(0.0, 1.0),
            risk: risk.clamp(0.0, 1.0),
        }
    }

    /// Create safety-first weights
    pub fn safety_first() -> Self {
        Self {
            urgency: 0.15,
            impact: 0.15,
            confidence: 0.20,
            cost: 0.10,
            risk: 0.40,
        }
    }

    /// Create performance-first weights
    pub fn performance_first() -> Self {
        Self {
            urgency: 0.30,
            impact: 0.35,
            confidence: 0.15,
            cost: 0.10,
            risk: 0.10,
        }
    }

    /// Create cost-conscious weights
    pub fn cost_conscious() -> Self {
        Self {
            urgency: 0.20,
            impact: 0.20,
            confidence: 0.15,
            cost: 0.35,
            risk: 0.10,
        }
    }

    /// Validate weights sum to approximately 1.0
    pub fn is_normalized(&self) -> bool {
        let sum = self.urgency + self.impact + self.confidence + self.cost + self.risk;
        (sum - 1.0).abs() < 0.01
    }

    /// Normalize weights
    pub fn normalize(&mut self) {
        let sum = self.urgency + self.impact + self.confidence + self.cost + self.risk;
        if sum > 0.0 {
            self.urgency /= sum;
            self.impact /= sum;
            self.confidence /= sum;
            self.cost /= sum;
            self.risk /= sum;
        }
    }
}

// ============================================================================
// RANKING CONTEXT
// ============================================================================

/// Context for ranking
#[derive(Debug, Clone)]
pub struct RankingContext {
    /// Severity of the situation
    pub severity: Severity,
    /// Confidence in the diagnosis
    pub confidence: Confidence,
    /// Time pressure
    pub time_pressure: bool,
    /// Available resources
    pub resources_available: bool,
}

impl RankingContext {
    /// Create new context
    pub fn new(severity: Severity, confidence: Confidence) -> Self {
        Self {
            severity,
            confidence,
            time_pressure: false,
            resources_available: true,
        }
    }

    /// Set time pressure
    pub fn with_time_pressure(mut self, pressure: bool) -> Self {
        self.time_pressure = pressure;
        self
    }

    /// Set resource availability
    pub fn with_resources(mut self, available: bool) -> Self {
        self.resources_available = available;
        self
    }

    /// Is this an emergency context?
    pub fn is_emergency(&self) -> bool {
        self.severity >= Severity::Critical || self.time_pressure
    }
}

// ============================================================================
// RANKED OPTION
// ============================================================================

/// An option with its ranking
#[derive(Debug, Clone)]
pub struct RankedOption {
    /// The option
    pub option: Option,
    /// Calculated score
    pub score: f32,
    /// Policy result
    pub policy_result: Option<PolicyResult>,
}

impl RankedOption {
    /// Create new ranked option
    pub fn new(option: Option) -> Self {
        Self {
            option,
            score: 0.0,
            policy_result: None,
        }
    }

    /// Set score
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    /// Set policy result
    pub fn with_policy(mut self, result: PolicyResult) -> Self {
        self.policy_result = Some(result);
        self
    }

    /// Is this option blocked by policy?
    pub fn is_blocked(&self) -> bool {
        self.policy_result
            .as_ref()
            .map(|r| !r.allowed)
            .unwrap_or(false)
    }

    /// Requires confirmation?
    pub fn requires_confirmation(&self) -> bool {
        self.policy_result
            .as_ref()
            .map(|r| r.requires_confirmation)
            .unwrap_or(false)
    }
}

// ============================================================================
// PRIORITY RANKER
// ============================================================================

/// Priority ranker - scores and ranks options
pub struct PriorityRanker {
    /// Weights for scoring
    weights: RankingWeights,
}

impl PriorityRanker {
    /// Create new ranker
    pub fn new(weights: RankingWeights) -> Self {
        Self { weights }
    }

    /// Get weights
    pub fn weights(&self) -> &RankingWeights {
        &self.weights
    }

    /// Set weights
    pub fn set_weights(&mut self, weights: RankingWeights) {
        self.weights = weights;
    }

    /// Rank options
    pub fn rank(&self, options: &mut [RankedOption], context: &RankingContext) {
        // Score each option
        for option in options.iter_mut() {
            option.score = self.calculate_score(&option.option, context);
        }

        // Sort by score (descending)
        options.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    /// Calculate score for an option
    fn calculate_score(&self, option: &Option, context: &RankingContext) -> f32 {
        let urgency_score = self.score_urgency(context.severity);
        let impact_score = self.score_impact(&option.expected_outcome);
        let confidence_score = context.confidence.value();
        let cost_score = self.score_cost(&option.cost);
        let risk_score = self.score_risk(&option.cost);

        self.weights.urgency * urgency_score
            + self.weights.impact * impact_score
            + self.weights.confidence * confidence_score
            + self.weights.cost * cost_score
            + self.weights.risk * risk_score
    }

    /// Score urgency based on severity
    fn score_urgency(&self, severity: Severity) -> f32 {
        match severity {
            Severity::Critical => 1.0,
            Severity::Error => 0.8,
            Severity::Warning => 0.5,
            Severity::Info => 0.2,
            Severity::Debug | Severity::Trace => 0.1,
        }
    }

    /// Score impact
    fn score_impact(&self, outcome: &ExpectedOutcome) -> f32 {
        outcome.success_probability
    }

    /// Score cost (lower is better)
    fn score_cost(&self, cost: &ActionCost) -> f32 {
        let cost_factor = (cost.cpu as f32 + cost.io as f32) / 200.0;
        1.0 - cost_factor.min(1.0)
    }

    /// Score risk (lower is better)
    fn score_risk(&self, cost: &ActionCost) -> f32 {
        match cost.risk {
            RiskLevel::Minimal => 1.0,
            RiskLevel::Low => 0.8,
            RiskLevel::Medium => 0.5,
            RiskLevel::High => 0.2,
            RiskLevel::Critical => 0.0,
        }
    }

    /// Get top N options
    pub fn top_n(&self, options: &[RankedOption], n: usize) -> Vec<&RankedOption> {
        options.iter().take(n).collect()
    }

    /// Get best option that passes policy
    pub fn best_allowed<'a>(&self, options: &'a [RankedOption]) -> Option<&'a RankedOption> {
        options.iter().find(|o| !o.is_blocked())
    }
}

impl Default for PriorityRanker {
    fn default() -> Self {
        Self::new(RankingWeights::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::options::{OptionId, ActionType, ActionTarget, ActionParameters, OptionSource};

    fn make_test_option(action_type: ActionType) -> Option {
        Option {
            id: OptionId::generate(),
            action_type,
            description: alloc::string::String::from("Test"),
            target: ActionTarget::System,
            parameters: ActionParameters::new(),
            expected_outcome: ExpectedOutcome::default(),
            reversible: true,
            cost: ActionCost::default(),
            source: OptionSource::Default,
        }
    }

    #[test]
    fn test_priority_ranker() {
        let ranker = PriorityRanker::default();
        let context = RankingContext {
            severity: Severity::Error,
            confidence: Confidence::HIGH,
            time_pressure: true,
            resources_available: true,
        };

        let noop = make_test_option(ActionType::NoOp);

        let mut ranked = vec![RankedOption {
            option: noop,
            score: 0.0,
            policy_result: None,
        }];

        ranker.rank(&mut ranked, &context);
        assert!(ranked[0].score > 0.0);
    }

    #[test]
    fn test_ranking_weights() {
        let mut weights = RankingWeights::default();
        weights.normalize();
        assert!(weights.is_normalized());
    }

    #[test]
    fn test_safety_first_weights() {
        let weights = RankingWeights::safety_first();
        assert!(weights.risk > weights.urgency);
    }
}

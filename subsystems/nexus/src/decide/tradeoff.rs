//! # Tradeoff Analysis
//!
//! Analyzes tradeoffs between competing objectives.
//! Implements multi-criteria decision analysis.
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// TRADEOFF TYPES
// ============================================================================

/// Tradeoff analysis
#[derive(Debug, Clone)]
pub struct TradeoffAnalysis {
    /// Analysis ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Alternatives
    pub alternatives: Vec<Alternative>,
    /// Criteria
    pub criteria: Vec<Criterion>,
    /// Scores matrix
    pub scores: BTreeMap<(u64, u64), f64>, // (alt_id, crit_id) -> score
    /// Created
    pub created: Timestamp,
}

/// Alternative
#[derive(Debug, Clone)]
pub struct Alternative {
    /// Alternative ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Attributes
    pub attributes: BTreeMap<String, f64>,
}

/// Criterion
#[derive(Debug, Clone)]
pub struct Criterion {
    /// Criterion ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Weight
    pub weight: f64,
    /// Direction
    pub direction: Direction,
    /// Threshold
    pub threshold: Option<f64>,
}

/// Optimization direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
}

/// Tradeoff result
#[derive(Debug, Clone)]
pub struct TradeoffResult {
    /// Analysis ID
    pub analysis_id: u64,
    /// Ranking
    pub ranking: Vec<RankedAlternative>,
    /// Pareto front
    pub pareto_front: Vec<u64>,
    /// Sensitivity
    pub sensitivity: LinearMap<f64, 64>,
    /// Method used
    pub method: AnalysisMethod,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Ranked alternative
#[derive(Debug, Clone)]
pub struct RankedAlternative {
    /// Alternative ID
    pub id: u64,
    /// Rank
    pub rank: usize,
    /// Score
    pub score: f64,
    /// Criterion scores
    pub criterion_scores: LinearMap<f64, 64>,
}

/// Analysis method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisMethod {
    /// Weighted sum
    WeightedSum,
    /// TOPSIS
    Topsis,
    /// AHP
    AnalyticHierarchy,
    /// Pareto
    ParetoOptimality,
    /// PROMETHEE
    Promethee,
}

/// Sensitivity analysis
#[derive(Debug, Clone)]
pub struct SensitivityResult {
    /// Criterion ID
    pub criterion_id: u64,
    /// Weight range
    pub weight_range: (f64, f64),
    /// Stability
    pub stability: f64,
    /// Critical points
    pub critical_points: Vec<f64>,
}

// ============================================================================
// TRADEOFF ANALYZER
// ============================================================================

/// Tradeoff analyzer
pub struct TradeoffAnalyzer {
    /// Analyses
    analyses: BTreeMap<u64, TradeoffAnalysis>,
    /// Results
    results: BTreeMap<u64, TradeoffResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: TradeoffConfig,
    /// Statistics
    stats: TradeoffStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct TradeoffConfig {
    /// Default method
    pub default_method: AnalysisMethod,
    /// Sensitivity steps
    pub sensitivity_steps: usize,
    /// Normalize scores
    pub normalize: bool,
}

impl Default for TradeoffConfig {
    fn default() -> Self {
        Self {
            default_method: AnalysisMethod::WeightedSum,
            sensitivity_steps: 10,
            normalize: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TradeoffStats {
    /// Analyses created
    pub analyses_created: u64,
    /// Analyses completed
    pub analyses_completed: u64,
}

impl TradeoffAnalyzer {
    /// Create new analyzer
    pub fn new(config: TradeoffConfig) -> Self {
        Self {
            analyses: BTreeMap::new(),
            results: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: TradeoffStats::default(),
        }
    }

    /// Create analysis
    pub fn create(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let analysis = TradeoffAnalysis {
            id,
            name: name.into(),
            alternatives: Vec::new(),
            criteria: Vec::new(),
            scores: BTreeMap::new(),
            created: Timestamp::now(),
        };

        self.analyses.insert(id, analysis);
        self.stats.analyses_created += 1;

        id
    }

    /// Add alternative
    pub fn add_alternative(
        &mut self,
        analysis_id: u64,
        name: &str,
        description: &str,
    ) -> u64 {
        let alt_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let alt = Alternative {
            id: alt_id,
            name: name.into(),
            description: description.into(),
            attributes: BTreeMap::new(),
        };

        if let Some(analysis) = self.analyses.get_mut(&analysis_id) {
            analysis.alternatives.push(alt);
        }

        alt_id
    }

    /// Add criterion
    pub fn add_criterion(
        &mut self,
        analysis_id: u64,
        name: &str,
        weight: f64,
        direction: Direction,
    ) -> u64 {
        let crit_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let crit = Criterion {
            id: crit_id,
            name: name.into(),
            weight: weight.clamp(0.0, 1.0),
            direction,
            threshold: None,
        };

        if let Some(analysis) = self.analyses.get_mut(&analysis_id) {
            analysis.criteria.push(crit);
        }

        crit_id
    }

    /// Set score
    #[inline]
    pub fn set_score(&mut self, analysis_id: u64, alt_id: u64, crit_id: u64, score: f64) {
        if let Some(analysis) = self.analyses.get_mut(&analysis_id) {
            analysis.scores.insert((alt_id, crit_id), score);
        }
    }

    /// Analyze with weighted sum
    pub fn analyze_weighted_sum(&mut self, analysis_id: u64) -> Option<TradeoffResult> {
        let analysis = self.analyses.get(&analysis_id)?.clone();

        // Normalize weights
        let total_weight: f64 = analysis.criteria.iter().map(|c| c.weight).sum();

        let normalized_weights: LinearMap<f64, 64> = if total_weight > 0.0 {
            analysis.criteria.iter()
                .map(|c| (c.id, c.weight / total_weight))
                .collect()
        } else {
            analysis.criteria.iter()
                .map(|c| (c.id, 1.0 / analysis.criteria.len() as f64))
                .collect()
        };

        // Normalize scores per criterion
        let normalized_scores = if self.config.normalize {
            self.normalize_scores(&analysis)
        } else {
            analysis.scores.clone()
        };

        // Calculate weighted scores
        let mut rankings: Vec<RankedAlternative> = analysis.alternatives.iter()
            .map(|alt| {
                let mut total = 0.0;
                let mut criterion_scores = BTreeMap::new();

                for crit in &analysis.criteria {
                    let raw_score = normalized_scores.get(&(alt.id, crit.id)).copied().unwrap_or(0.0);
                    let weight = normalized_weights.get(&crit.id).copied().unwrap_or(0.0);

                    let weighted = raw_score * weight;
                    total += weighted;
                    criterion_scores.insert(crit.id, raw_score);
                }

                RankedAlternative {
                    id: alt.id,
                    rank: 0,
                    score: total,
                    criterion_scores,
                }
            })
            .collect();

        // Sort and assign ranks
        rankings.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));

        for (i, r) in rankings.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        // Find Pareto front
        let pareto_front = self.find_pareto_front(&analysis, &normalized_scores);

        // Sensitivity analysis
        let sensitivity = self.analyze_sensitivity(&analysis, &rankings);

        let result = TradeoffResult {
            analysis_id,
            ranking: rankings,
            pareto_front,
            sensitivity,
            method: AnalysisMethod::WeightedSum,
            timestamp: Timestamp::now(),
        };

        self.results.insert(analysis_id, result.clone());
        self.stats.analyses_completed += 1;

        Some(result)
    }

    fn normalize_scores(&self, analysis: &TradeoffAnalysis) -> BTreeMap<(u64, u64), f64> {
        let mut normalized = BTreeMap::new();

        for crit in &analysis.criteria {
            // Get min/max for criterion
            let scores: Vec<f64> = analysis.alternatives.iter()
                .filter_map(|alt| analysis.scores.get(&(alt.id, crit.id)).copied())
                .collect();

            if scores.is_empty() {
                continue;
            }

            let min = scores.iter().copied().fold(f64::INFINITY, f64::min);
            let max = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let range = max - min;

            for alt in &analysis.alternatives {
                if let Some(&score) = analysis.scores.get(&(alt.id, crit.id)) {
                    let norm = if range > 0.0 {
                        let base_norm = (score - min) / range;
                        // Flip if minimizing
                        if crit.direction == Direction::Minimize {
                            1.0 - base_norm
                        } else {
                            base_norm
                        }
                    } else {
                        0.5
                    };
                    normalized.insert((alt.id, crit.id), norm);
                }
            }
        }

        normalized
    }

    fn find_pareto_front(
        &self,
        analysis: &TradeoffAnalysis,
        scores: &BTreeMap<(u64, u64), f64>,
    ) -> Vec<u64> {
        let mut pareto = Vec::new();

        for alt in &analysis.alternatives {
            let mut is_dominated = false;

            for other in &analysis.alternatives {
                if alt.id == other.id {
                    continue;
                }

                if self.dominates(other.id, alt.id, analysis, scores) {
                    is_dominated = true;
                    break;
                }
            }

            if !is_dominated {
                pareto.push(alt.id);
            }
        }

        pareto
    }

    fn dominates(
        &self,
        a: u64,
        b: u64,
        analysis: &TradeoffAnalysis,
        scores: &BTreeMap<(u64, u64), f64>,
    ) -> bool {
        let mut dominated = true;
        let mut strictly_better = false;

        for crit in &analysis.criteria {
            let score_a = scores.get(&(a, crit.id)).copied().unwrap_or(0.0);
            let score_b = scores.get(&(b, crit.id)).copied().unwrap_or(0.0);

            if score_a < score_b {
                dominated = false;
            }
            if score_a > score_b {
                strictly_better = true;
            }
        }

        dominated && strictly_better
    }

    fn analyze_sensitivity(
        &self,
        analysis: &TradeoffAnalysis,
        rankings: &[RankedAlternative],
    ) -> BTreeMap<u64, f64> {
        let mut sensitivity = BTreeMap::new();

        if rankings.len() < 2 {
            return sensitivity;
        }

        let winner = &rankings[0];
        let runner_up = &rankings[1];

        for crit in &analysis.criteria {
            let winner_score = winner.criterion_scores.get(&crit.id).copied().unwrap_or(0.0);
            let runner_score = runner_up.criterion_scores.get(&crit.id).copied().unwrap_or(0.0);

            // How much can the winner's score drop before runner-up wins
            let margin = (winner.score - runner_up.score).abs();
            let impact = (winner_score - runner_score).abs() * crit.weight;

            let sens = if impact > 0.0 { margin / impact } else { f64::INFINITY };
            sensitivity.insert(crit.id, sens.min(1.0));
        }

        sensitivity
    }

    /// Get analysis
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&TradeoffAnalysis> {
        self.analyses.get(&id)
    }

    /// Get result
    #[inline(always)]
    pub fn get_result(&self, id: u64) -> Option<&TradeoffResult> {
        self.results.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &TradeoffStats {
        &self.stats
    }
}

impl Default for TradeoffAnalyzer {
    fn default() -> Self {
        Self::new(TradeoffConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_analysis() {
        let mut analyzer = TradeoffAnalyzer::default();

        let id = analyzer.create("test");
        assert!(analyzer.get(id).is_some());
    }

    #[test]
    fn test_add_alternatives() {
        let mut analyzer = TradeoffAnalyzer::default();

        let id = analyzer.create("test");
        let alt1 = analyzer.add_alternative(id, "Option A", "First option");
        let alt2 = analyzer.add_alternative(id, "Option B", "Second option");

        let analysis = analyzer.get(id).unwrap();
        assert_eq!(analysis.alternatives.len(), 2);
    }

    #[test]
    fn test_weighted_sum() {
        let mut analyzer = TradeoffAnalyzer::default();

        let id = analyzer.create("test");

        let alt1 = analyzer.add_alternative(id, "A", "");
        let alt2 = analyzer.add_alternative(id, "B", "");

        let crit1 = analyzer.add_criterion(id, "Cost", 0.5, Direction::Minimize);
        let crit2 = analyzer.add_criterion(id, "Quality", 0.5, Direction::Maximize);

        analyzer.set_score(id, alt1, crit1, 100.0);
        analyzer.set_score(id, alt1, crit2, 80.0);
        analyzer.set_score(id, alt2, crit1, 50.0);
        analyzer.set_score(id, alt2, crit2, 60.0);

        let result = analyzer.analyze_weighted_sum(id).unwrap();

        assert_eq!(result.ranking.len(), 2);
        assert_eq!(result.ranking[0].rank, 1);
    }

    #[test]
    fn test_pareto_front() {
        let mut analyzer = TradeoffAnalyzer::default();

        let id = analyzer.create("test");

        let alt1 = analyzer.add_alternative(id, "A", "");
        let alt2 = analyzer.add_alternative(id, "B", "");

        let crit1 = analyzer.add_criterion(id, "X", 0.5, Direction::Maximize);
        let crit2 = analyzer.add_criterion(id, "Y", 0.5, Direction::Maximize);

        // A is better on X, B is better on Y -> both on Pareto front
        analyzer.set_score(id, alt1, crit1, 10.0);
        analyzer.set_score(id, alt1, crit2, 5.0);
        analyzer.set_score(id, alt2, crit1, 5.0);
        analyzer.set_score(id, alt2, crit2, 10.0);

        let result = analyzer.analyze_weighted_sum(id).unwrap();

        assert_eq!(result.pareto_front.len(), 2);
    }
}

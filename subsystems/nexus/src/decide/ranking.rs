//! # Decision Ranking
//!
//! Ranks and prioritizes decision alternatives.
//! Implements multi-criteria decision analysis.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// RANKING TYPES
// ============================================================================

/// Alternative
#[derive(Debug, Clone)]
pub struct Alternative {
    /// Alternative ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Scores
    pub scores: LinearMap<f64, 64>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
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
    /// Scale
    pub scale: Scale,
}

/// Direction (higher or lower is better)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
}

/// Scale
#[derive(Debug, Clone)]
pub enum Scale {
    Ratio,
    Interval { min: f64, max: f64 },
    Ordinal { levels: Vec<String> },
}

/// Ranking result
#[derive(Debug, Clone)]
pub struct RankingResult {
    /// Rankings
    pub rankings: Vec<RankedAlternative>,
    /// Method used
    pub method: RankingMethod,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Ranked alternative
#[derive(Debug, Clone)]
pub struct RankedAlternative {
    /// Alternative ID
    pub alternative_id: u64,
    /// Rank
    pub rank: usize,
    /// Aggregate score
    pub score: f64,
    /// Criterion scores (normalized)
    pub criterion_scores: LinearMap<f64, 64>,
}

/// Ranking method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingMethod {
    WeightedSum,
    TOPSIS,
    AHP,
    ELECTRE,
    PROMETHEE,
}

/// Comparison
#[derive(Debug, Clone)]
pub struct PairwiseComparison {
    /// Criterion or alternative A
    pub a: u64,
    /// Criterion or alternative B
    pub b: u64,
    /// Preference (>1 means A preferred)
    pub preference: f64,
}

/// Sensitivity analysis
#[derive(Debug, Clone)]
pub struct SensitivityAnalysis {
    /// Original ranking
    pub original: Vec<u64>,
    /// Weight variations
    pub weight_sensitivity: LinearMap<f64, 64>,
    /// Threshold for rank change
    pub threshold: LinearMap<f64, 64>,
}

// ============================================================================
// RANKING ENGINE
// ============================================================================

/// Ranking engine
pub struct RankingEngine {
    /// Alternatives
    alternatives: BTreeMap<u64, Alternative>,
    /// Criteria
    criteria: BTreeMap<u64, Criterion>,
    /// Results
    results: Vec<RankingResult>,
    /// Pairwise comparisons
    comparisons: Vec<PairwiseComparison>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: RankingConfig,
    /// Statistics
    stats: RankingStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct RankingConfig {
    /// Default method
    pub default_method: RankingMethod,
    /// Normalize scores
    pub normalize: bool,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            default_method: RankingMethod::WeightedSum,
            normalize: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RankingStats {
    /// Rankings performed
    pub rankings_performed: u64,
    /// Alternatives evaluated
    pub alternatives_evaluated: u64,
}

impl RankingEngine {
    /// Create new engine
    pub fn new(config: RankingConfig) -> Self {
        Self {
            alternatives: BTreeMap::new(),
            criteria: BTreeMap::new(),
            results: Vec::new(),
            comparisons: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: RankingStats::default(),
        }
    }

    /// Add criterion
    pub fn add_criterion(
        &mut self,
        name: &str,
        weight: f64,
        direction: Direction,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let criterion = Criterion {
            id,
            name: name.into(),
            weight: weight.clamp(0.0, 1.0),
            direction,
            scale: Scale::Ratio,
        };

        self.criteria.insert(id, criterion);

        id
    }

    /// Set criterion scale
    #[inline]
    pub fn set_scale(&mut self, criterion_id: u64, scale: Scale) {
        if let Some(criterion) = self.criteria.get_mut(&criterion_id) {
            criterion.scale = scale;
        }
    }

    /// Add alternative
    pub fn add_alternative(&mut self, name: &str, description: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let alternative = Alternative {
            id,
            name: name.into(),
            description: description.into(),
            scores: LinearMap::new(),
            metadata: BTreeMap::new(),
        };

        self.alternatives.insert(id, alternative);

        id
    }

    /// Set score
    #[inline]
    pub fn set_score(&mut self, alternative_id: u64, criterion_id: u64, score: f64) {
        if let Some(alt) = self.alternatives.get_mut(&alternative_id) {
            alt.scores.insert(criterion_id, score);
        }
    }

    /// Add pairwise comparison
    #[inline]
    pub fn add_comparison(&mut self, a: u64, b: u64, preference: f64) {
        self.comparisons.push(PairwiseComparison {
            a,
            b,
            preference,
        });
    }

    /// Rank using weighted sum
    pub fn rank_weighted_sum(&mut self) -> RankingResult {
        let mut ranked: Vec<RankedAlternative> = Vec::new();

        // Normalize weights
        let total_weight: f64 = self.criteria.values().map(|c| c.weight).sum();

        for alt in self.alternatives.values() {
            let mut score = 0.0;
            let mut criterion_scores = BTreeMap::new();

            for criterion in self.criteria.values() {
                if let Some(&raw_score) = alt.scores.get(&criterion.id) {
                    let normalized = self.normalize_score(raw_score, criterion);
                    let weight = criterion.weight / total_weight.max(0.001);

                    criterion_scores.insert(criterion.id, normalized);
                    score += normalized * weight;
                }
            }

            ranked.push(RankedAlternative {
                alternative_id: alt.id,
                rank: 0,
                score,
                criterion_scores,
            });
        }

        // Sort and assign ranks
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));

        for (i, alt) in ranked.iter_mut().enumerate() {
            alt.rank = i + 1;
        }

        self.stats.rankings_performed += 1;
        self.stats.alternatives_evaluated += ranked.len() as u64;

        let result = RankingResult {
            rankings: ranked,
            method: RankingMethod::WeightedSum,
            timestamp: Timestamp::now(),
        };

        self.results.push(result.clone());

        result
    }

    /// Rank using TOPSIS
    pub fn rank_topsis(&mut self) -> RankingResult {
        let mut ranked: Vec<RankedAlternative> = Vec::new();

        // Calculate normalized decision matrix
        let mut normalized_matrix: BTreeMap<u64, BTreeMap<u64, f64>> = BTreeMap::new();

        for criterion in self.criteria.values() {
            // Calculate sqrt of sum of squares
            let sum_sq: f64 = self.alternatives.values()
                .filter_map(|a| a.scores.get(&criterion.id))
                .map(|s| s * s)
                .sum();
            let norm_factor = sum_sq.sqrt().max(0.001);

            for alt in self.alternatives.values() {
                if let Some(&score) = alt.scores.get(&criterion.id) {
                    normalized_matrix.entry(alt.id)
                        .or_insert_with(BTreeMap::new)
                        .insert(criterion.id, score / norm_factor);
                }
            }
        }

        // Find ideal and anti-ideal solutions
        let mut ideal: LinearMap<f64, 64> = BTreeMap::new();
        let mut anti_ideal: LinearMap<f64, 64> = BTreeMap::new();

        for criterion in self.criteria.values() {
            let values: Vec<f64> = normalized_matrix.values()
                .filter_map(|m| m.get(&criterion.id))
                .copied()
                .collect();

            if !values.is_empty() {
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);

                match criterion.direction {
                    Direction::Maximize => {
                        ideal.insert(criterion.id, max);
                        anti_ideal.insert(criterion.id, min);
                    }
                    Direction::Minimize => {
                        ideal.insert(criterion.id, min);
                        anti_ideal.insert(criterion.id, max);
                    }
                }
            }
        }

        // Calculate distances and scores
        for alt in self.alternatives.values() {
            let alt_values = normalized_matrix.get(&alt.id);

            let mut dist_ideal = 0.0;
            let mut dist_anti_ideal = 0.0;

            for criterion in self.criteria.values() {
                if let Some(values) = alt_values {
                    if let Some(&v) = values.get(&criterion.id) {
                        let i = ideal.get(&criterion.id).copied().unwrap_or(0.0);
                        let ai = anti_ideal.get(&criterion.id).copied().unwrap_or(0.0);

                        dist_ideal += criterion.weight * (v - i) * (v - i);
                        dist_anti_ideal += criterion.weight * (v - ai) * (v - ai);
                    }
                }
            }

            dist_ideal = dist_ideal.sqrt();
            dist_anti_ideal = dist_anti_ideal.sqrt();

            let score = if (dist_ideal + dist_anti_ideal) > 0.0 {
                dist_anti_ideal / (dist_ideal + dist_anti_ideal)
            } else {
                0.5
            };

            let criterion_scores = alt_values.cloned().unwrap_or_default();

            ranked.push(RankedAlternative {
                alternative_id: alt.id,
                rank: 0,
                score,
                criterion_scores,
            });
        }

        // Sort and assign ranks
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));

        for (i, alt) in ranked.iter_mut().enumerate() {
            alt.rank = i + 1;
        }

        self.stats.rankings_performed += 1;
        self.stats.alternatives_evaluated += ranked.len() as u64;

        let result = RankingResult {
            rankings: ranked,
            method: RankingMethod::TOPSIS,
            timestamp: Timestamp::now(),
        };

        self.results.push(result.clone());

        result
    }

    fn normalize_score(&self, score: f64, criterion: &Criterion) -> f64 {
        let normalized = match &criterion.scale {
            Scale::Ratio => score.max(0.0).min(1.0),
            Scale::Interval { min, max } => {
                if max > min {
                    (score - min) / (max - min)
                } else {
                    0.5
                }
            }
            Scale::Ordinal { levels } => {
                // Assume score is index
                let idx = score as usize;
                if levels.is_empty() {
                    0.5
                } else {
                    idx as f64 / (levels.len() - 1).max(1) as f64
                }
            }
        };

        match criterion.direction {
            Direction::Maximize => normalized,
            Direction::Minimize => 1.0 - normalized,
        }
    }

    /// Rank using default method
    #[inline]
    pub fn rank(&mut self) -> RankingResult {
        match self.config.default_method {
            RankingMethod::WeightedSum => self.rank_weighted_sum(),
            RankingMethod::TOPSIS => self.rank_topsis(),
            _ => self.rank_weighted_sum(),
        }
    }

    /// Sensitivity analysis
    pub fn sensitivity_analysis(&self) -> SensitivityAnalysis {
        let original: Vec<u64> = self.results.last()
            .map(|r| r.rankings.iter().map(|a| a.alternative_id).collect())
            .unwrap_or_default();

        let mut weight_sensitivity = BTreeMap::new();
        let mut threshold = BTreeMap::new();

        for criterion in self.criteria.values() {
            // Simple sensitivity measure
            weight_sensitivity.insert(criterion.id, criterion.weight);
            threshold.insert(criterion.id, 0.1); // Placeholder
        }

        SensitivityAnalysis {
            original,
            weight_sensitivity,
            threshold,
        }
    }

    /// Get alternative
    #[inline(always)]
    pub fn get_alternative(&self, id: u64) -> Option<&Alternative> {
        self.alternatives.get(&id)
    }

    /// Get criterion
    #[inline(always)]
    pub fn get_criterion(&self, id: u64) -> Option<&Criterion> {
        self.criteria.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &RankingStats {
        &self.stats
    }
}

impl Default for RankingEngine {
    fn default() -> Self {
        Self::new(RankingConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_criterion() {
        let mut engine = RankingEngine::default();

        let id = engine.add_criterion("cost", 0.5, Direction::Minimize);
        assert!(engine.get_criterion(id).is_some());
    }

    #[test]
    fn test_add_alternative() {
        let mut engine = RankingEngine::default();

        let id = engine.add_alternative("Option A", "First option");
        assert!(engine.get_alternative(id).is_some());
    }

    #[test]
    fn test_weighted_sum() {
        let mut engine = RankingEngine::default();

        let c1 = engine.add_criterion("quality", 0.6, Direction::Maximize);
        let c2 = engine.add_criterion("cost", 0.4, Direction::Minimize);

        let a1 = engine.add_alternative("A", "");
        let a2 = engine.add_alternative("B", "");

        engine.set_score(a1, c1, 0.8);
        engine.set_score(a1, c2, 0.3);
        engine.set_score(a2, c1, 0.6);
        engine.set_score(a2, c2, 0.7);

        let result = engine.rank_weighted_sum();

        assert_eq!(result.rankings.len(), 2);
        assert_eq!(result.rankings[0].rank, 1);
    }

    #[test]
    fn test_topsis() {
        let mut engine = RankingEngine::default();

        let c1 = engine.add_criterion("quality", 0.5, Direction::Maximize);
        let c2 = engine.add_criterion("cost", 0.5, Direction::Minimize);

        let a1 = engine.add_alternative("A", "");
        let a2 = engine.add_alternative("B", "");

        engine.set_score(a1, c1, 0.9);
        engine.set_score(a1, c2, 0.5);
        engine.set_score(a2, c1, 0.7);
        engine.set_score(a2, c2, 0.3);

        let result = engine.rank_topsis();

        assert_eq!(result.rankings.len(), 2);
        assert_eq!(result.method, RankingMethod::TOPSIS);
    }

    #[test]
    fn test_scale_interval() {
        let mut engine = RankingEngine::default();

        let c = engine.add_criterion("temp", 1.0, Direction::Maximize);
        engine.set_scale(c, Scale::Interval { min: 0.0, max: 100.0 });

        let criterion = engine.get_criterion(c).unwrap();
        assert!(matches!(criterion.scale, Scale::Interval { .. }));
    }
}

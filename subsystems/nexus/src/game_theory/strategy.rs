//! Strategy representations for game theory.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use super::types::StrategyId;

/// Strategy representation
#[derive(Debug, Clone)]
pub struct Strategy {
    /// Strategy identifier
    pub id: StrategyId,
    /// Strategy name
    pub name: String,
    /// Action parameters
    pub params: Vec<f64>,
}

impl Strategy {
    /// Create a new strategy
    pub fn new(id: StrategyId, name: &str, params: Vec<f64>) -> Self {
        Self {
            id,
            name: String::from(name),
            params,
        }
    }

    /// Pure strategy for resource request
    #[inline]
    pub fn resource_request(id: StrategyId, amount: f64) -> Self {
        Self::new(
            id,
            &alloc::format!("request_{}", amount as u64),
            alloc::vec![amount],
        )
    }

    /// Cooperation strategy
    #[inline(always)]
    pub fn cooperate(id: StrategyId) -> Self {
        Self::new(id, "cooperate", alloc::vec![1.0])
    }

    /// Defection strategy
    #[inline(always)]
    pub fn defect(id: StrategyId) -> Self {
        Self::new(id, "defect", alloc::vec![0.0])
    }
}

/// Mixed strategy (probability distribution over pure strategies)
#[derive(Debug, Clone)]
pub struct MixedStrategy {
    /// Probabilities for each pure strategy
    pub probabilities: Vec<f64>,
}

impl MixedStrategy {
    /// Create a pure strategy (probability 1 on one strategy)
    #[inline]
    pub fn pure(strategy_idx: usize, num_strategies: usize) -> Self {
        let mut probs = alloc::vec![0.0; num_strategies];
        if strategy_idx < num_strategies {
            probs[strategy_idx] = 1.0;
        }
        Self {
            probabilities: probs,
        }
    }

    /// Create uniform mixed strategy
    #[inline]
    pub fn uniform(num_strategies: usize) -> Self {
        let prob = 1.0 / num_strategies as f64;
        Self {
            probabilities: alloc::vec![prob; num_strategies],
        }
    }

    /// Create from probability vector
    pub fn from_probs(probabilities: Vec<f64>) -> Self {
        let mut probs = probabilities;
        // Normalize
        let sum: f64 = probs.iter().sum();
        if sum > 1e-10 {
            for p in &mut probs {
                *p /= sum;
            }
        }
        Self {
            probabilities: probs,
        }
    }

    /// Sample a pure strategy from mixed
    #[inline]
    pub fn sample(&self, random: f64) -> usize {
        let mut cumulative = 0.0;
        for (i, &p) in self.probabilities.iter().enumerate() {
            cumulative += p;
            if random < cumulative {
                return i;
            }
        }
        self.probabilities.len() - 1
    }

    /// Get support (strategies with non-zero probability)
    #[inline]
    pub fn support(&self) -> Vec<usize> {
        self.probabilities
            .iter()
            .enumerate()
            .filter(|&(_, p)| *p > 1e-10)
            .map(|(i, _)| i)
            .collect()
    }
}

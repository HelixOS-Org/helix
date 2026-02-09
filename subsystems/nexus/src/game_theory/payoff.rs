//! Payoff functions and calculators for game theory.

extern crate alloc;
use alloc::vec::Vec;

use super::strategy::Strategy;
use super::types::{StrategyId, Utility};

/// Payoff function type
#[derive(Debug, Clone)]
pub struct PayoffFunction {
    /// Payoff matrix for 2-player games
    matrix: Option<Vec<Vec<(Utility, Utility)>>>,
    /// Custom payoff calculator
    custom: Option<PayoffCalculator>,
}

/// Custom payoff calculator
#[derive(Debug, Clone)]
pub struct PayoffCalculator {
    /// Game parameters
    params: Vec<f64>,
    /// Calculation type
    calc_type: PayoffType,
}

#[derive(Debug, Clone, Copy)]
pub enum PayoffType {
    /// Linear resource allocation
    LinearResource,
    /// Congestion with delay
    CongestionDelay,
    /// Bandwidth sharing
    BandwidthShare,
    /// CPU time sharing
    CpuShare,
    /// Memory allocation
    MemoryAlloc,
}

impl PayoffFunction {
    /// Create from matrix (2-player games)
    #[inline]
    pub fn from_matrix(matrix: Vec<Vec<(Utility, Utility)>>) -> Self {
        Self {
            matrix: Some(matrix),
            custom: None,
        }
    }

    /// Create for resource allocation
    #[inline]
    pub fn resource_allocation(total_resource: f64) -> Self {
        Self {
            matrix: None,
            custom: Some(PayoffCalculator {
                params: alloc::vec![total_resource],
                calc_type: PayoffType::LinearResource,
            }),
        }
    }

    /// Calculate payoffs for strategy profile
    pub fn calculate(
        &self,
        strategies: &[StrategyId],
        all_strategies: &[Vec<Strategy>],
    ) -> Vec<Utility> {
        if let Some(ref matrix) = self.matrix {
            // 2-player matrix game
            if strategies.len() >= 2 {
                let (u1, u2) = matrix
                    .get(strategies[0] as usize)
                    .and_then(|row| row.get(strategies[1] as usize))
                    .copied()
                    .unwrap_or((0.0, 0.0));
                return alloc::vec![u1, u2];
            }
        }

        if let Some(ref calc) = self.custom {
            return self.calculate_custom(calc, strategies, all_strategies);
        }

        alloc::vec![0.0; strategies.len()]
    }

    fn calculate_custom(
        &self,
        calc: &PayoffCalculator,
        strategies: &[StrategyId],
        all_strategies: &[Vec<Strategy>],
    ) -> Vec<Utility> {
        match calc.calc_type {
            PayoffType::LinearResource => {
                let total = calc.params.first().copied().unwrap_or(100.0);
                let requests: Vec<f64> = strategies
                    .iter()
                    .zip(all_strategies.iter())
                    .map(|(&sid, strats)| {
                        strats
                            .get(sid as usize)
                            .and_then(|s| s.params.first())
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect();

                let total_request: f64 = requests.iter().sum();
                if total_request <= total {
                    // All requests satisfied
                    requests
                } else {
                    // Proportional allocation
                    requests
                        .iter()
                        .map(|&r| r * total / total_request)
                        .collect()
                }
            },
            PayoffType::CongestionDelay => {
                // Higher congestion = lower utility
                let n = strategies.len() as f64;
                let base_utility = calc.params.first().copied().unwrap_or(10.0);
                alloc::vec![base_utility / n; strategies.len()]
            },
            _ => alloc::vec![0.0; strategies.len()],
        }
    }
}

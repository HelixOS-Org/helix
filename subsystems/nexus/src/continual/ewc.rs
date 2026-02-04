//! Elastic Weight Consolidation (EWC) implementation.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

/// Fisher information diagonal for EWC
#[derive(Debug, Clone)]
pub struct FisherInformation {
    /// Fisher diagonal values per parameter
    pub fisher: Vec<f64>,
    /// Optimal parameter values after task
    pub optimal_params: Vec<f64>,
    /// Task ID this Fisher was computed for
    pub task_id: u64,
}

impl FisherInformation {
    /// Create empty Fisher information
    pub fn new(param_count: usize, task_id: u64) -> Self {
        Self {
            fisher: vec![0.0; param_count],
            optimal_params: vec![0.0; param_count],
            task_id,
        }
    }

    /// Estimate Fisher information from gradients
    pub fn estimate(&mut self, gradients: &[Vec<f64>]) {
        if gradients.is_empty() {
            return;
        }

        let n_samples = gradients.len() as f64;

        // Fisher = E[grad * grad^T] diagonal
        for grad in gradients {
            for (i, &g) in grad.iter().enumerate() {
                if i < self.fisher.len() {
                    self.fisher[i] += g * g;
                }
            }
        }

        // Average
        for f in &mut self.fisher {
            *f /= n_samples;
        }
    }

    /// Store optimal parameters
    pub fn set_optimal(&mut self, params: &[f64]) {
        self.optimal_params = params.to_vec();
    }

    /// Compute EWC penalty for current parameters
    pub fn penalty(&self, current_params: &[f64], lambda: f64) -> f64 {
        let mut penalty = 0.0;

        for (i, (&current, &optimal)) in current_params
            .iter()
            .zip(self.optimal_params.iter())
            .enumerate()
        {
            if i < self.fisher.len() {
                let diff = current - optimal;
                penalty += self.fisher[i] * diff * diff;
            }
        }

        0.5 * lambda * penalty
    }

    /// Compute gradient of EWC penalty
    pub fn penalty_gradient(&self, current_params: &[f64], lambda: f64) -> Vec<f64> {
        let mut grad = vec![0.0; current_params.len()];

        for (i, (&current, &optimal)) in current_params
            .iter()
            .zip(self.optimal_params.iter())
            .enumerate()
        {
            if i < self.fisher.len() {
                grad[i] = lambda * self.fisher[i] * (current - optimal);
            }
        }

        grad
    }
}

/// EWC-based continual learner
pub struct EwcLearner {
    /// Fisher information per task
    pub fishers: Vec<FisherInformation>,
    /// Current parameters
    pub params: Vec<f64>,
    /// Lambda coefficient for EWC penalty
    pub lambda: f64,
    /// Online mode (accumulate Fisher)
    pub online: bool,
    /// Decay factor for online EWC
    pub gamma: f64,
}

impl EwcLearner {
    /// Create a new EWC learner
    pub fn new(param_count: usize, lambda: f64) -> Self {
        Self {
            fishers: Vec::new(),
            params: vec![0.0; param_count],
            lambda,
            online: false,
            gamma: 0.9,
        }
    }

    /// Enable online EWC mode
    pub fn enable_online(&mut self, gamma: f64) {
        self.online = true;
        self.gamma = gamma;
    }

    /// Register a completed task
    pub fn register_task(&mut self, task_id: u64, gradients: &[Vec<f64>], params: &[f64]) {
        let mut fisher = FisherInformation::new(params.len(), task_id);
        fisher.estimate(gradients);
        fisher.set_optimal(params);

        if self.online && !self.fishers.is_empty() {
            // Merge with previous Fisher using decay
            let prev = self.fishers.last_mut().unwrap();
            for (i, f) in fisher.fisher.iter_mut().enumerate() {
                if i < prev.fisher.len() {
                    *f = self.gamma * prev.fisher[i] + *f;
                }
            }
            // Update in place
            *self.fishers.last_mut().unwrap() = fisher;
        } else {
            self.fishers.push(fisher);
        }

        self.params = params.to_vec();
    }

    /// Compute total EWC penalty
    pub fn total_penalty(&self, current_params: &[f64]) -> f64 {
        self.fishers
            .iter()
            .map(|f| f.penalty(current_params, self.lambda))
            .sum()
    }

    /// Compute gradient including EWC penalty
    pub fn regularized_gradient(&self, current_params: &[f64], task_gradient: &[f64]) -> Vec<f64> {
        let mut grad = task_gradient.to_vec();

        for fisher in &self.fishers {
            let ewc_grad = fisher.penalty_gradient(current_params, self.lambda);
            for (g, eg) in grad.iter_mut().zip(ewc_grad.iter()) {
                *g += eg;
            }
        }

        grad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fisher_information() {
        let mut fisher = FisherInformation::new(3, 0);

        let gradients = vec![vec![0.1, 0.2, 0.3], vec![0.2, 0.1, 0.4]];

        fisher.estimate(&gradients);
        fisher.set_optimal(&[1.0, 2.0, 3.0]);

        let penalty = fisher.penalty(&[1.1, 2.1, 3.1], 1.0);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_ewc_learner() {
        let mut ewc = EwcLearner::new(3, 100.0);

        let gradients = vec![vec![0.1, 0.2, 0.3]];
        let params = vec![1.0, 2.0, 3.0];

        ewc.register_task(0, &gradients, &params);

        let penalty = ewc.total_penalty(&[1.1, 2.1, 3.1]);
        assert!(penalty > 0.0);

        let task_grad = vec![0.5, 0.5, 0.5];
        let reg_grad = ewc.regularized_gradient(&[1.1, 2.1, 3.1], &task_grad);
        assert_eq!(reg_grad.len(), 3);
    }
}

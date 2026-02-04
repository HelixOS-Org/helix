//! Projected Gradient Descent (PGD) attack implementation.

use crate::adversarial::perturbation::Perturbation;
use crate::adversarial::types::PerturbationType;
use crate::adversarial::utils::{lcg_next, sign};

/// Projected Gradient Descent (PGD)
#[derive(Debug, Clone)]
pub struct PGD {
    /// Perturbation budget
    pub epsilon: f64,
    /// Step size
    pub alpha: f64,
    /// Number of iterations
    pub iterations: usize,
    /// Perturbation type
    pub pert_type: PerturbationType,
    /// Random restarts
    pub restarts: usize,
}

impl PGD {
    /// Create a new PGD attacker
    pub fn new(epsilon: f64, alpha: f64, iterations: usize) -> Self {
        Self {
            epsilon,
            alpha,
            iterations,
            pert_type: PerturbationType::LInf,
            restarts: 1,
        }
    }

    /// Generate adversarial example
    pub fn attack<F>(&self, input: &[f64], mut grad_fn: F, seed: u64) -> Perturbation
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        let dim = input.len();
        let mut best_perturbation = Perturbation::new(dim, self.pert_type, self.epsilon);
        let mut best_loss = f64::NEG_INFINITY;

        let mut rng = seed;

        for _ in 0..self.restarts {
            let mut perturbation = Perturbation::new(dim, self.pert_type, self.epsilon);

            // Random initialization
            for d in &mut perturbation.delta {
                rng = lcg_next(rng);
                *d = (rng as f64 / u64::MAX as f64 - 0.5) * 2.0 * self.epsilon;
            }
            perturbation.project();

            for iter in 0..self.iterations {
                let adv_input = perturbation.apply(input);
                let gradient = grad_fn(&adv_input);

                // Update perturbation
                for (d, &g) in perturbation.delta.iter_mut().zip(gradient.iter()) {
                    *d += self.alpha * sign(g);
                }

                perturbation.project();
                perturbation.iterations = iter + 1;
            }

            // Compute loss (approximation)
            let adv_input = perturbation.apply(input);
            let final_grad = grad_fn(&adv_input);
            let loss: f64 = final_grad.iter().map(|&g| libm::fabs(g)).sum();

            if loss > best_loss {
                best_loss = loss;
                best_perturbation = perturbation;
            }
        }

        best_perturbation
    }
}

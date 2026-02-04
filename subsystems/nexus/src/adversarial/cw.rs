//! Carlini & Wagner (C&W) attack implementation.

use alloc::vec;
use alloc::vec::Vec;

use crate::adversarial::perturbation::Perturbation;
use crate::adversarial::types::PerturbationType;
use crate::adversarial::utils::lcg_next;

/// Carlini & Wagner (C&W) Attack
#[derive(Debug, Clone)]
pub struct CWAttack {
    /// Confidence parameter
    pub kappa: f64,
    /// Learning rate
    pub learning_rate: f64,
    /// Max iterations
    pub max_iterations: usize,
    /// Binary search steps
    pub binary_search_steps: usize,
    /// Initial c value
    pub initial_c: f64,
}

impl CWAttack {
    /// Create a new C&W attacker
    pub fn new() -> Self {
        Self {
            kappa: 0.0,
            learning_rate: 0.01,
            max_iterations: 1000,
            binary_search_steps: 9,
            initial_c: 0.001,
        }
    }

    /// Generate adversarial example
    pub fn attack<F, G>(
        &self,
        input: &[f64],
        mut loss_fn: F,
        mut grad_fn: G,
        seed: u64,
    ) -> Perturbation
    where
        F: FnMut(&[f64]) -> f64,
        G: FnMut(&[f64]) -> Vec<f64>,
    {
        let dim = input.len();
        let mut perturbation = Perturbation::new(dim, PerturbationType::L2, 0.0);

        let mut c = self.initial_c;
        let mut rng = seed;

        // Binary search over c
        let mut lower = 0.0;
        let mut upper = 1e10;

        for _ in 0..self.binary_search_steps {
            // Initialize w (tanh space)
            let mut w: Vec<f64> = input
                .iter()
                .map(|&x| {
                    let x_clamp = x.clamp(-0.999, 0.999);
                    0.5 * libm::log((1.0 + x_clamp) / (1.0 - x_clamp))
                })
                .collect();

            let mut best_l2 = f64::INFINITY;
            let mut best_delta = vec![0.0; dim];

            for iter in 0..self.max_iterations {
                // Compute adversarial example: x' = tanh(w) / 2 + 0.5
                let x_adv: Vec<f64> = w.iter().map(|&wi| libm::tanh(wi)).collect();

                // Delta
                let delta: Vec<f64> = x_adv
                    .iter()
                    .zip(input.iter())
                    .map(|(&xa, &x)| xa - x)
                    .collect();

                let l2_norm: f64 = libm::sqrt(delta.iter().map(|d| d * d).sum());

                // Loss and gradient
                let f_loss = loss_fn(&x_adv);
                let f_grad = grad_fn(&x_adv);

                // Total loss: ||delta||^2 + c * f(x')
                let _total_loss = l2_norm * l2_norm + c * f_loss;

                // Check for success
                if f_loss < 0.0 && l2_norm < best_l2 {
                    best_l2 = l2_norm;
                    best_delta = delta.clone();
                    perturbation.success = true;
                }

                // Gradient update
                for (wi, (&fi, &di)) in w.iter_mut().zip(f_grad.iter().zip(delta.iter())) {
                    let tanh_deriv = 1.0 - libm::tanh(*wi).powi(2);
                    let grad = 2.0 * di * tanh_deriv + c * fi * tanh_deriv;
                    *wi -= self.learning_rate * grad;
                }

                perturbation.iterations = iter + 1;
                rng = lcg_next(rng);
            }

            if perturbation.success {
                upper = c;
            } else {
                lower = c;
            }

            c = (lower + upper) / 2.0;
            perturbation.delta = best_delta;
        }

        perturbation.epsilon = perturbation.l2_norm();
        perturbation
    }
}

impl Default for CWAttack {
    fn default() -> Self {
        Self::new()
    }
}

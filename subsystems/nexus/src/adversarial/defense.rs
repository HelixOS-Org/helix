//! Adversarial defense mechanisms.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::adversarial::pgd::PGD;
use crate::adversarial::utils::{box_muller, inv_normal_cdf, lcg_next};

/// Input purification using denoising
#[derive(Debug, Clone)]
pub struct InputPurifier {
    /// Denoising strength
    pub strength: f64,
    /// Number of purification steps
    pub steps: usize,
    /// Noise scale for randomization
    pub noise_scale: f64,
}

impl InputPurifier {
    /// Create a new purifier
    pub fn new() -> Self {
        Self {
            strength: 0.1,
            steps: 10,
            noise_scale: 0.01,
        }
    }

    /// Purify input using gradient-based denoising
    pub fn purify<F>(&self, input: &[f64], mut energy_fn: F, seed: u64) -> Vec<f64>
    where
        F: FnMut(&[f64]) -> (f64, Vec<f64>), // Returns (energy, gradient)
    {
        let mut x = input.to_vec();
        let mut rng = seed;

        for step in 0..self.steps {
            // Add small noise for stochastic purification
            for xi in &mut x {
                rng = lcg_next(rng);
                *xi += (rng as f64 / u64::MAX as f64 - 0.5) * self.noise_scale;
            }

            // Compute gradient
            let (_, gradient) = energy_fn(&x);

            // Step size decay
            let step_size = self.strength / (1.0 + step as f64 * 0.1);

            // Gradient descent on energy
            for (xi, &gi) in x.iter_mut().zip(gradient.iter()) {
                *xi -= step_size * gi;
            }
        }

        x
    }

    /// Purify using median filtering
    pub fn purify_median(&self, input: &[f64], window_size: usize) -> Vec<f64> {
        let n = input.len();
        if n == 0 {
            return Vec::new();
        }

        let half = window_size / 2;
        let mut output = vec![0.0; n];

        for i in 0..n {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(n);

            let mut window: Vec<f64> = input[start..end].to_vec();
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

            output[i] = window[window.len() / 2];
        }

        output
    }
}

impl Default for InputPurifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Randomized smoothing for certified robustness
#[derive(Debug, Clone)]
pub struct RandomizedSmoothing {
    /// Noise standard deviation
    pub sigma: f64,
    /// Number of samples for certification
    pub n_samples: usize,
    /// Confidence level
    pub alpha: f64,
}

impl RandomizedSmoothing {
    /// Create a new smoothing certifier
    pub fn new(sigma: f64) -> Self {
        Self {
            sigma,
            n_samples: 1000,
            alpha: 0.001,
        }
    }

    /// Predict with smoothing
    pub fn predict<F>(&self, input: &[f64], mut classifier: F, seed: u64) -> (usize, f64)
    where
        F: FnMut(&[f64]) -> usize,
    {
        let mut rng = seed;
        let mut class_counts: BTreeMap<usize, usize> = BTreeMap::new();

        for _ in 0..self.n_samples {
            // Add Gaussian noise
            let noisy: Vec<f64> = input
                .iter()
                .map(|&x| {
                    rng = lcg_next(rng);
                    let z = box_muller(rng);
                    x + self.sigma * z
                })
                .collect();

            let pred = classifier(&noisy);
            *class_counts.entry(pred).or_insert(0) += 1;
        }

        // Get most common class
        let (top_class, top_count) = class_counts
            .iter()
            .max_by_key(|(_, &c)| c)
            .map(|(&c, &n)| (c, n))
            .unwrap_or((0, 0));

        let confidence = top_count as f64 / self.n_samples as f64;

        (top_class, confidence)
    }

    /// Certify robustness radius
    pub fn certify<F>(&self, input: &[f64], mut classifier: F, seed: u64) -> Option<f64>
    where
        F: FnMut(&[f64]) -> usize,
    {
        let (_, confidence) = self.predict(input, &mut classifier, seed);

        // Compute certification radius
        if confidence > 0.5 {
            // Inverse normal CDF approximation
            let p_a = confidence;
            let radius = self.sigma * inv_normal_cdf(p_a);

            if radius > 0.0 {
                return Some(radius);
            }
        }

        None
    }
}

/// Adversarial training wrapper
#[derive(Debug, Clone)]
pub struct AdversarialTraining {
    /// Attack epsilon
    pub epsilon: f64,
    /// Attack steps
    pub attack_steps: usize,
    /// Mix ratio (adversarial vs clean)
    pub mix_ratio: f64,
}

impl AdversarialTraining {
    /// Create a new adversarial training wrapper
    pub fn new(epsilon: f64) -> Self {
        Self {
            epsilon,
            attack_steps: 7,
            mix_ratio: 0.5,
        }
    }

    /// Generate adversarial batch
    pub fn generate_adversarial_batch<F>(
        &self,
        inputs: &[Vec<f64>],
        mut grad_fn: F,
        seed: u64,
    ) -> Vec<Vec<f64>>
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        let pgd = PGD::new(self.epsilon, self.epsilon / 4.0, self.attack_steps);

        inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let pert = pgd.attack(input, |x| grad_fn(x), seed + i as u64);
                pert.apply(input)
            })
            .collect()
    }
}

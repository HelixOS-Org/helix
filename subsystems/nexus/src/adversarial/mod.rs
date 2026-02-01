//! # Adversarial Defense Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary adversarial robustness system that
//! protects kernel AI models from adversarial attacks, ensuring security
//! and reliability of AI-powered kernel decisions.
//!
//! ## Key Features
//!
//! - **Adversarial Attack Detection**: Detect malicious inputs
//! - **Adversarial Training**: Train models to be robust
//! - **Input Purification**: Clean potentially adversarial inputs
//! - **Certified Defenses**: Provable robustness guarantees
//! - **Ensemble Diversity**: Multiple models for robustness
//! - **Anomaly Detection**: Detect out-of-distribution inputs
//!
//! ## Kernel Applications
//!
//! - Protect scheduler from adversarial workloads
//! - Secure resource allocation from manipulation
//! - Defend intrusion detection from evasion attacks
//! - Robust memory management under adversarial conditions

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default perturbation epsilon
const DEFAULT_EPSILON: f64 = 0.01;

/// Maximum perturbation iterations
const MAX_ATTACK_ITER: usize = 100;

/// Number of random samples for detection
const DETECTION_SAMPLES: usize = 50;

/// Ensemble size for voting
const ENSEMBLE_SIZE: usize = 5;

/// Input dimension limit
const MAX_INPUT_DIM: usize = 1024;

// ============================================================================
// ADVERSARIAL PERTURBATION TYPES
// ============================================================================

/// Types of adversarial perturbations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerturbationType {
    /// L-infinity bounded perturbations
    LInf,
    /// L2 bounded perturbations
    L2,
    /// L1 bounded perturbations
    L1,
    /// Patch-based perturbations
    Patch,
    /// Semantic perturbations
    Semantic,
}

/// Adversarial perturbation
#[derive(Debug, Clone)]
pub struct Perturbation {
    /// Perturbation vector
    pub delta: Vec<f64>,
    /// Perturbation type
    pub pert_type: PerturbationType,
    /// Magnitude
    pub epsilon: f64,
    /// Success indicator
    pub success: bool,
    /// Number of iterations used
    pub iterations: usize,
}

impl Perturbation {
    /// Create a new perturbation
    pub fn new(dim: usize, pert_type: PerturbationType, epsilon: f64) -> Self {
        Self {
            delta: vec![0.0; dim],
            pert_type,
            epsilon,
            success: false,
            iterations: 0,
        }
    }

    /// Get L-infinity norm
    pub fn linf_norm(&self) -> f64 {
        self.delta
            .iter()
            .map(|&x| libm::fabs(x))
            .fold(0.0, f64::max)
    }

    /// Get L2 norm
    pub fn l2_norm(&self) -> f64 {
        let sum_sq: f64 = self.delta.iter().map(|x| x * x).sum();
        libm::sqrt(sum_sq)
    }

    /// Get L1 norm
    pub fn l1_norm(&self) -> f64 {
        self.delta.iter().map(|&x| libm::fabs(x)).sum()
    }

    /// Project onto epsilon ball
    pub fn project(&mut self) {
        match self.pert_type {
            PerturbationType::LInf => {
                for d in &mut self.delta {
                    *d = d.clamp(-self.epsilon, self.epsilon);
                }
            },
            PerturbationType::L2 => {
                let norm = self.l2_norm();
                if norm > self.epsilon {
                    for d in &mut self.delta {
                        *d *= self.epsilon / norm;
                    }
                }
            },
            PerturbationType::L1 => {
                let norm = self.l1_norm();
                if norm > self.epsilon {
                    // Simplex projection (simplified)
                    for d in &mut self.delta {
                        *d *= self.epsilon / norm;
                    }
                }
            },
            _ => {},
        }
    }

    /// Apply perturbation to input
    pub fn apply(&self, input: &[f64]) -> Vec<f64> {
        input
            .iter()
            .zip(self.delta.iter())
            .map(|(&x, &d)| x + d)
            .collect()
    }
}

// ============================================================================
// ADVERSARIAL ATTACKS
// ============================================================================

/// Fast Gradient Sign Method (FGSM)
#[derive(Debug, Clone)]
pub struct FGSM {
    /// Perturbation budget
    pub epsilon: f64,
    /// Perturbation type
    pub pert_type: PerturbationType,
}

impl FGSM {
    /// Create a new FGSM attacker
    pub fn new(epsilon: f64) -> Self {
        Self {
            epsilon,
            pert_type: PerturbationType::LInf,
        }
    }

    /// Generate adversarial example
    pub fn attack(&self, input: &[f64], gradient: &[f64]) -> Perturbation {
        let mut perturbation = Perturbation::new(input.len(), self.pert_type, self.epsilon);

        for (d, &g) in perturbation.delta.iter_mut().zip(gradient.iter()) {
            *d = self.epsilon * sign(g);
        }

        perturbation.iterations = 1;
        perturbation
    }
}

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
                let total_loss = l2_norm * l2_norm + c * f_loss;

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

/// Auto-Attack (ensemble of attacks)
#[derive(Debug, Clone)]
pub struct AutoAttack {
    /// Epsilon bound
    pub epsilon: f64,
    /// Perturbation type
    pub pert_type: PerturbationType,
    /// Use APGD-CE
    pub use_apgd_ce: bool,
    /// Use APGD-DLR
    pub use_apgd_dlr: bool,
    /// Use FAB
    pub use_fab: bool,
    /// Use Square attack
    pub use_square: bool,
}

impl AutoAttack {
    /// Create a new AutoAttack
    pub fn new(epsilon: f64) -> Self {
        Self {
            epsilon,
            pert_type: PerturbationType::LInf,
            use_apgd_ce: true,
            use_apgd_dlr: true,
            use_fab: false,
            use_square: true,
        }
    }

    /// Run attack
    pub fn attack<F>(&self, input: &[f64], grad_fn: F, seed: u64) -> Perturbation
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let dim = input.len();
        let mut best_perturbation = Perturbation::new(dim, self.pert_type, self.epsilon);
        let mut best_norm = f64::INFINITY;

        // Run PGD with different settings
        if self.use_apgd_ce {
            let pgd = PGD::new(self.epsilon, self.epsilon / 4.0, 100);
            let pert = pgd.attack(input, |x| grad_fn(x), seed);

            if pert.l2_norm() < best_norm {
                best_norm = pert.l2_norm();
                best_perturbation = pert;
            }
        }

        if self.use_square {
            // Square attack (query-based)
            let pert = self.square_attack(input, seed);

            if pert.l2_norm() < best_norm && pert.success {
                best_perturbation = pert;
            }
        }

        best_perturbation
    }

    /// Square attack (simplified)
    fn square_attack(&self, input: &[f64], seed: u64) -> Perturbation {
        let dim = input.len();
        let mut perturbation = Perturbation::new(dim, self.pert_type, self.epsilon);

        let mut rng = seed;

        // Initialize with random corners
        for d in &mut perturbation.delta {
            rng = lcg_next(rng);
            *d = if rng % 2 == 0 {
                self.epsilon
            } else {
                -self.epsilon
            };
        }

        perturbation
    }
}

// ============================================================================
// ADVERSARIAL DETECTION
// ============================================================================

/// Detection result
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Is input adversarial?
    pub is_adversarial: bool,
    /// Confidence score
    pub confidence: f64,
    /// Detection method used
    pub method: String,
    /// Additional scores
    pub scores: BTreeMap<String, f64>,
}

impl DetectionResult {
    /// Create a new detection result
    pub fn new(is_adversarial: bool, confidence: f64, method: String) -> Self {
        Self {
            is_adversarial,
            confidence,
            method,
            scores: BTreeMap::new(),
        }
    }
}

/// Feature Squeezing detector
#[derive(Debug, Clone)]
pub struct FeatureSqueezer {
    /// Bit depth for color depth reduction
    pub bit_depth: u32,
    /// Blur radius
    pub blur_radius: f64,
    /// Detection threshold
    pub threshold: f64,
}

impl FeatureSqueezer {
    /// Create a new feature squeezer
    pub fn new() -> Self {
        Self {
            bit_depth: 4,
            blur_radius: 2.0,
            threshold: 0.1,
        }
    }

    /// Squeeze features (reduce bit depth)
    pub fn squeeze_bits(&self, input: &[f64]) -> Vec<f64> {
        let levels = (1 << self.bit_depth) as f64;

        input
            .iter()
            .map(|&x| {
                let normalized = (x + 1.0) / 2.0;
                let quantized = (normalized * levels).floor() / levels;
                quantized * 2.0 - 1.0
            })
            .collect()
    }

    /// Squeeze features (blur)
    pub fn squeeze_blur(&self, input: &[f64]) -> Vec<f64> {
        let n = input.len();
        if n == 0 {
            return Vec::new();
        }

        let kernel_size = (self.blur_radius * 2.0).ceil() as usize + 1;
        let half = kernel_size / 2;

        let mut output = vec![0.0; n];

        for i in 0..n {
            let mut sum = 0.0;
            let mut weight_sum = 0.0;

            for k in 0..kernel_size {
                let j = (i + k).saturating_sub(half);
                if j < n {
                    let dist = (k as f64 - half as f64).abs();
                    let weight =
                        libm::exp(-dist * dist / (2.0 * self.blur_radius * self.blur_radius));
                    sum += input[j] * weight;
                    weight_sum += weight;
                }
            }

            output[i] = sum / weight_sum.max(1e-10);
        }

        output
    }

    /// Detect adversarial input
    pub fn detect<F>(&self, input: &[f64], mut model_fn: F) -> DetectionResult
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        let original_output = model_fn(input);

        // Apply squeezing
        let squeezed_bits = self.squeeze_bits(input);
        let squeezed_blur = self.squeeze_blur(input);

        let output_bits = model_fn(&squeezed_bits);
        let output_blur = model_fn(&squeezed_blur);

        // Compute distances
        let dist_bits: f64 = original_output
            .iter()
            .zip(output_bits.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
            / original_output.len().max(1) as f64;

        let dist_blur: f64 = original_output
            .iter()
            .zip(output_blur.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
            / original_output.len().max(1) as f64;

        let max_dist = dist_bits.max(dist_blur);
        let is_adversarial = max_dist > self.threshold;

        let mut result = DetectionResult::new(
            is_adversarial,
            (max_dist / self.threshold).min(1.0),
            String::from("FeatureSqueezing"),
        );

        result.scores.insert(String::from("dist_bits"), dist_bits);
        result.scores.insert(String::from("dist_blur"), dist_blur);

        result
    }
}

impl Default for FeatureSqueezer {
    fn default() -> Self {
        Self::new()
    }
}

/// Local Intrinsic Dimensionality (LID) detector
#[derive(Debug, Clone)]
pub struct LIDDetector {
    /// Number of nearest neighbors
    pub k: usize,
    /// Threshold for detection
    pub threshold: f64,
    /// Reference samples (clean data)
    pub reference_lids: Vec<f64>,
}

impl LIDDetector {
    /// Create a new LID detector
    pub fn new(k: usize) -> Self {
        Self {
            k,
            threshold: 10.0,
            reference_lids: Vec::new(),
        }
    }

    /// Compute LID estimate
    pub fn compute_lid(&self, distances: &[f64]) -> f64 {
        if distances.len() < 2 {
            return 0.0;
        }

        // Sort distances
        let mut sorted: Vec<f64> = distances.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        // Take k nearest
        let k = self.k.min(sorted.len() - 1);
        let neighbors = &sorted[1..=k]; // Exclude self (distance 0)

        if neighbors.is_empty() || neighbors[neighbors.len() - 1] < 1e-10 {
            return 0.0;
        }

        // LID estimation using maximum likelihood
        let r_max = neighbors[neighbors.len() - 1];

        let log_sum: f64 = neighbors
            .iter()
            .map(|&r| libm::log((r + 1e-10) / r_max))
            .sum();

        -(k as f64) / log_sum
    }

    /// Fit detector on clean data
    pub fn fit(&mut self, clean_samples: &[Vec<f64>]) {
        self.reference_lids.clear();

        for sample in clean_samples {
            // Compute distances to other samples
            let distances: Vec<f64> = clean_samples
                .iter()
                .map(|other| {
                    sample
                        .iter()
                        .zip(other.iter())
                        .map(|(&a, &b)| (a - b).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
                .collect();

            let lid = self.compute_lid(&distances);
            self.reference_lids.push(lid);
        }

        // Set threshold based on percentile
        if !self.reference_lids.is_empty() {
            let mut sorted = self.reference_lids.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

            let idx = (sorted.len() as f64 * 0.99) as usize;
            self.threshold = sorted[idx.min(sorted.len() - 1)] * 1.5;
        }
    }

    /// Detect adversarial input
    pub fn detect(&self, input: &[f64], reference_set: &[Vec<f64>]) -> DetectionResult {
        // Compute distances to reference set
        let distances: Vec<f64> = reference_set
            .iter()
            .map(|ref_sample| {
                input
                    .iter()
                    .zip(ref_sample.iter())
                    .map(|(&a, &b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
            .collect();

        let lid = self.compute_lid(&distances);
        let is_adversarial = lid > self.threshold;

        let mut result = DetectionResult::new(
            is_adversarial,
            (lid / self.threshold).min(1.0),
            String::from("LID"),
        );

        result.scores.insert(String::from("lid"), lid);
        result
            .scores
            .insert(String::from("threshold"), self.threshold);

        result
    }
}

/// Mahalanobis Distance detector
#[derive(Debug, Clone)]
pub struct MahalanobisDetector {
    /// Class means
    pub means: Vec<Vec<f64>>,
    /// Shared covariance (inverse)
    pub precision: Vec<Vec<f64>>,
    /// Detection threshold
    pub threshold: f64,
    /// Dimensionality
    pub dim: usize,
}

impl MahalanobisDetector {
    /// Create a new detector
    pub fn new(dim: usize) -> Self {
        Self {
            means: Vec::new(),
            precision: Vec::new(),
            threshold: 100.0,
            dim,
        }
    }

    /// Fit detector on features
    pub fn fit(&mut self, features: &[Vec<f64>], labels: &[usize]) {
        if features.is_empty() {
            return;
        }

        let dim = features[0].len();
        self.dim = dim;

        // Compute class means
        let num_classes = labels.iter().max().map(|&m| m + 1).unwrap_or(1);
        let mut class_sums = vec![vec![0.0; dim]; num_classes];
        let mut class_counts = vec![0usize; num_classes];

        for (feat, &label) in features.iter().zip(labels.iter()) {
            if label < num_classes {
                for (s, &f) in class_sums[label].iter_mut().zip(feat.iter()) {
                    *s += f;
                }
                class_counts[label] += 1;
            }
        }

        self.means = class_sums
            .iter()
            .zip(class_counts.iter())
            .map(|(sum, &count)| {
                if count > 0 {
                    sum.iter().map(|&s| s / count as f64).collect()
                } else {
                    vec![0.0; dim]
                }
            })
            .collect();

        // Compute covariance (simplified: use identity for inverse)
        self.precision = (0..dim)
            .map(|i| (0..dim).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
    }

    /// Compute Mahalanobis distance
    pub fn distance(&self, input: &[f64], class: usize) -> f64 {
        if class >= self.means.len() || input.len() != self.dim {
            return f64::INFINITY;
        }

        let mean = &self.means[class];

        // (x - mu)^T * precision * (x - mu)
        let diff: Vec<f64> = input
            .iter()
            .zip(mean.iter())
            .map(|(&x, &m)| x - m)
            .collect();

        let mut result = 0.0;
        for (i, &di) in diff.iter().enumerate() {
            for (j, &dj) in diff.iter().enumerate() {
                if i < self.precision.len() && j < self.precision[i].len() {
                    result += di * self.precision[i][j] * dj;
                }
            }
        }

        libm::sqrt(result.max(0.0))
    }

    /// Detect adversarial input
    pub fn detect(&self, input: &[f64]) -> DetectionResult {
        // Compute minimum Mahalanobis distance across all classes
        let min_dist = self
            .means
            .iter()
            .enumerate()
            .map(|(c, _)| self.distance(input, c))
            .fold(f64::INFINITY, f64::min);

        let is_adversarial = min_dist > self.threshold;

        let mut result = DetectionResult::new(
            is_adversarial,
            (min_dist / self.threshold).min(1.0),
            String::from("Mahalanobis"),
        );

        result.scores.insert(String::from("min_distance"), min_dist);

        result
    }
}

// ============================================================================
// ADVERSARIAL DEFENSES
// ============================================================================

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
        let (predicted_class, confidence) = self.predict(input, &mut classifier, seed);

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

// ============================================================================
// ENSEMBLE DEFENSE
// ============================================================================

/// Diverse ensemble for robustness
#[derive(Debug, Clone)]
pub struct EnsembleDefense {
    /// Number of models
    pub num_models: usize,
    /// Diversity regularization strength
    pub diversity_weight: f64,
    /// Voting threshold
    pub voting_threshold: f64,
    /// Model predictions (stored for voting)
    predictions: Vec<Vec<f64>>,
}

impl EnsembleDefense {
    /// Create a new ensemble defense
    pub fn new(num_models: usize) -> Self {
        Self {
            num_models: num_models.min(ENSEMBLE_SIZE),
            diversity_weight: 0.1,
            voting_threshold: 0.5,
            predictions: Vec::new(),
        }
    }

    /// Aggregate predictions using voting
    pub fn vote(&mut self, model_predictions: &[Vec<f64>]) -> Vec<f64> {
        self.predictions = model_predictions.to_vec();

        if self.predictions.is_empty() {
            return Vec::new();
        }

        let dim = self.predictions[0].len();
        let num_models = self.predictions.len();

        // Average predictions
        let mut avg = vec![0.0; dim];

        for pred in &self.predictions {
            for (a, &p) in avg.iter_mut().zip(pred.iter()) {
                *a += p;
            }
        }

        for a in &mut avg {
            *a /= num_models as f64;
        }

        avg
    }

    /// Compute prediction variance (uncertainty)
    pub fn prediction_variance(&self) -> Vec<f64> {
        if self.predictions.len() < 2 {
            return vec![0.0; self.predictions.first().map(|p| p.len()).unwrap_or(0)];
        }

        let dim = self.predictions[0].len();
        let num = self.predictions.len() as f64;

        // Compute mean
        let mut mean = vec![0.0; dim];
        for pred in &self.predictions {
            for (m, &p) in mean.iter_mut().zip(pred.iter()) {
                *m += p;
            }
        }
        for m in &mut mean {
            *m /= num;
        }

        // Compute variance
        let mut variance = vec![0.0; dim];
        for pred in &self.predictions {
            for (v, (&p, &m)) in variance.iter_mut().zip(pred.iter().zip(mean.iter())) {
                *v += (p - m).powi(2);
            }
        }
        for v in &mut variance {
            *v /= num;
        }

        variance
    }

    /// Check for ensemble disagreement (potential attack indicator)
    pub fn check_disagreement(&self, threshold: f64) -> bool {
        let variance = self.prediction_variance();
        let max_var = variance.iter().fold(0.0, |a, &b| f64::max(a, b));

        max_var > threshold
    }
}

// ============================================================================
// KERNEL ADVERSARIAL DEFENSE
// ============================================================================

/// Types of kernel adversarial attacks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAttackType {
    /// Resource exhaustion
    ResourceExhaustion,
    /// Priority manipulation
    PriorityManipulation,
    /// Cache poisoning
    CachePoisoning,
    /// Timing attack
    TimingAttack,
    /// Evasion attack
    Evasion,
}

/// Kernel adversarial event
#[derive(Debug, Clone)]
pub struct AdversarialEvent {
    /// Event timestamp
    pub timestamp: u64,
    /// Attack type
    pub attack_type: KernelAttackType,
    /// Affected component
    pub component: String,
    /// Severity (0-1)
    pub severity: f64,
    /// Was it blocked?
    pub blocked: bool,
}

/// Kernel adversarial defense manager
pub struct KernelAdversarialDefense {
    /// Feature squeezer
    pub squeezer: FeatureSqueezer,
    /// LID detector
    pub lid_detector: LIDDetector,
    /// Input purifier
    pub purifier: InputPurifier,
    /// Ensemble defense
    pub ensemble: EnsembleDefense,
    /// Event log
    pub events: Vec<AdversarialEvent>,
    /// Is defense active?
    pub active: bool,
    /// Reference samples for detection
    reference_samples: Vec<Vec<f64>>,
}

impl KernelAdversarialDefense {
    /// Create a new kernel defense system
    pub fn new() -> Self {
        Self {
            squeezer: FeatureSqueezer::new(),
            lid_detector: LIDDetector::new(20),
            purifier: InputPurifier::new(),
            ensemble: EnsembleDefense::new(5),
            events: Vec::new(),
            active: true,
            reference_samples: Vec::new(),
        }
    }

    /// Add reference sample
    pub fn add_reference(&mut self, sample: Vec<f64>) {
        self.reference_samples.push(sample);

        // Limit size
        while self.reference_samples.len() > 1000 {
            self.reference_samples.remove(0);
        }
    }

    /// Detect potential attack
    pub fn detect(&mut self, input: &[f64], timestamp: u64) -> DetectionResult {
        if !self.active {
            return DetectionResult::new(false, 0.0, String::from("Disabled"));
        }

        // LID detection
        let lid_result = if !self.reference_samples.is_empty() {
            self.lid_detector.detect(input, &self.reference_samples)
        } else {
            DetectionResult::new(false, 0.0, String::from("NoReference"))
        };

        // Combined detection
        let is_adversarial = lid_result.is_adversarial;

        if is_adversarial {
            let event = AdversarialEvent {
                timestamp,
                attack_type: KernelAttackType::Evasion,
                component: String::from("unknown"),
                severity: lid_result.confidence,
                blocked: true,
            };
            self.events.push(event);
        }

        lid_result
    }

    /// Purify suspicious input
    pub fn purify<F>(&self, input: &[f64], energy_fn: F, seed: u64) -> Vec<f64>
    where
        F: FnMut(&[f64]) -> (f64, Vec<f64>),
    {
        if !self.active {
            return input.to_vec();
        }

        self.purifier.purify(input, energy_fn, seed)
    }

    /// Get defense statistics
    pub fn get_stats(&self) -> DefenseStats {
        let total = self.events.len();
        let blocked = self.events.iter().filter(|e| e.blocked).count();

        let by_type: BTreeMap<String, usize> =
            self.events.iter().fold(BTreeMap::new(), |mut acc, e| {
                let key = alloc::format!("{:?}", e.attack_type);
                *acc.entry(key).or_insert(0) += 1;
                acc
            });

        DefenseStats {
            total_attacks: total,
            blocked_attacks: blocked,
            block_rate: if total > 0 {
                blocked as f64 / total as f64
            } else {
                1.0
            },
            attacks_by_type: by_type,
        }
    }
}

impl Default for KernelAdversarialDefense {
    fn default() -> Self {
        Self::new()
    }
}

/// Defense statistics
#[derive(Debug, Clone)]
pub struct DefenseStats {
    /// Total attacks detected
    pub total_attacks: usize,
    /// Blocked attacks
    pub blocked_attacks: usize,
    /// Block rate
    pub block_rate: f64,
    /// Attacks by type
    pub attacks_by_type: BTreeMap<String, usize>,
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Sign function
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// LCG random number generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Box-Muller transform
fn box_muller(seed: u64) -> f64 {
    let u1 = (seed as f64 / u64::MAX as f64).max(1e-10);
    let seed2 = lcg_next(seed);
    let u2 = seed2 as f64 / u64::MAX as f64;

    libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * core::f64::consts::PI * u2)
}

/// Inverse normal CDF approximation
fn inv_normal_cdf(p: f64) -> f64 {
    // Approximation using Abramowitz and Stegun formula 26.2.23
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    let sign = if p < 0.5 { -1.0 } else { 1.0 };
    let p = if p < 0.5 { p } else { 1.0 - p };

    let t = libm::sqrt(-2.0 * libm::log(p));

    // Coefficients
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;

    let num = c0 + c1 * t + c2 * t * t;
    let den = 1.0 + d1 * t + d2 * t * t + d3 * t * t * t;

    sign * (t - num / den)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perturbation() {
        let mut pert = Perturbation::new(10, PerturbationType::LInf, 0.1);

        pert.delta = vec![0.2, -0.2, 0.05, 0.0, 0.15, -0.1, 0.0, 0.0, 0.0, 0.0];
        pert.project();

        for &d in &pert.delta {
            assert!(d >= -0.1 && d <= 0.1);
        }
    }

    #[test]
    fn test_l2_projection() {
        let mut pert = Perturbation::new(3, PerturbationType::L2, 1.0);

        pert.delta = vec![1.0, 1.0, 1.0];
        pert.project();

        assert!(pert.l2_norm() <= 1.0 + 1e-10);
    }

    #[test]
    fn test_fgsm() {
        let fgsm = FGSM::new(0.1);
        let input = vec![0.5; 10];
        let gradient = vec![1.0, -1.0, 0.5, -0.5, 0.0, 1.0, -1.0, 0.5, -0.5, 0.0];

        let pert = fgsm.attack(&input, &gradient);

        assert_eq!(pert.delta.len(), 10);
        assert!((pert.delta[0] - 0.1).abs() < 1e-10);
        assert!((pert.delta[1] + 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_pgd() {
        let pgd = PGD::new(0.1, 0.01, 10);
        let input = vec![0.5; 10];

        let pert = pgd.attack(&input, |x| vec![1.0; x.len()], 12345);

        assert_eq!(pert.delta.len(), 10);
        assert!(pert.linf_norm() <= 0.1 + 1e-10);
    }

    #[test]
    fn test_cw_attack() {
        let cw = CWAttack::new();

        assert!(cw.max_iterations > 0);
        assert!(cw.learning_rate > 0.0);
    }

    #[test]
    fn test_auto_attack() {
        let auto = AutoAttack::new(0.1);
        let input = vec![0.5; 10];

        let pert = auto.attack(&input, |x| vec![1.0; x.len()], 12345);

        assert_eq!(pert.delta.len(), 10);
    }

    #[test]
    fn test_feature_squeezer() {
        let squeezer = FeatureSqueezer::new();
        let input = vec![0.5, 0.25, 0.75, 0.125];

        let squeezed = squeezer.squeeze_bits(&input);

        assert_eq!(squeezed.len(), input.len());
    }

    #[test]
    fn test_feature_squeezer_blur() {
        let squeezer = FeatureSqueezer::new();
        let input = vec![0.0, 1.0, 0.0, 1.0, 0.0];

        let blurred = squeezer.squeeze_blur(&input);

        assert_eq!(blurred.len(), input.len());
        // Blurred values should be smoothed
        assert!(blurred[0] < 0.5);
    }

    #[test]
    fn test_lid_detector() {
        let mut detector = LIDDetector::new(5);

        let clean_samples: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0; 5]).collect();

        detector.fit(&clean_samples);

        let test = vec![0.5; 5];
        let result = detector.detect(&test, &clean_samples);

        assert!(result.scores.contains_key("lid"));
    }

    #[test]
    fn test_mahalanobis_detector() {
        let mut detector = MahalanobisDetector::new(5);

        let features: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0; 5]).collect();
        let labels: Vec<usize> = (0..100).map(|i| i % 3).collect();

        detector.fit(&features, &labels);

        assert_eq!(detector.means.len(), 3);
    }

    #[test]
    fn test_input_purifier() {
        let purifier = InputPurifier::new();
        let input = vec![0.5; 10];

        let purified = purifier.purify_median(&input, 3);

        assert_eq!(purified.len(), input.len());
    }

    #[test]
    fn test_randomized_smoothing() {
        let smoother = RandomizedSmoothing::new(0.25);
        let input = vec![0.5; 10];

        let (pred, confidence) = smoother.predict(&input, |_| 0, 12345);

        assert_eq!(pred, 0);
        assert!(confidence > 0.0);
    }

    #[test]
    fn test_adversarial_training() {
        let trainer = AdversarialTraining::new(0.1);
        let inputs = vec![vec![0.5; 10], vec![0.3; 10]];

        let adv_batch = trainer.generate_adversarial_batch(&inputs, |x| vec![1.0; x.len()], 12345);

        assert_eq!(adv_batch.len(), 2);
    }

    #[test]
    fn test_ensemble_defense() {
        let mut ensemble = EnsembleDefense::new(3);

        let predictions = vec![vec![0.9, 0.1], vec![0.8, 0.2], vec![0.85, 0.15]];

        let avg = ensemble.vote(&predictions);

        assert_eq!(avg.len(), 2);
        assert!(avg[0] > 0.8);
    }

    #[test]
    fn test_ensemble_variance() {
        let mut ensemble = EnsembleDefense::new(3);

        let predictions = vec![vec![0.9, 0.1], vec![0.8, 0.2], vec![0.85, 0.15]];

        ensemble.vote(&predictions);
        let variance = ensemble.prediction_variance();

        assert_eq!(variance.len(), 2);
        assert!(variance[0] < 0.1);
    }

    #[test]
    fn test_kernel_adversarial_defense() {
        let mut defense = KernelAdversarialDefense::new();

        // Add reference samples
        for i in 0..10 {
            defense.add_reference(vec![i as f64 / 10.0; 5]);
        }

        let input = vec![0.5; 5];
        let result = defense.detect(&input, 1000);

        assert!(result.method.len() > 0);
    }

    #[test]
    fn test_defense_stats() {
        let defense = KernelAdversarialDefense::new();
        let stats = defense.get_stats();

        assert_eq!(stats.total_attacks, 0);
        assert!((stats.block_rate - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_inv_normal_cdf() {
        // Should be approximately 0 for p=0.5
        assert!(inv_normal_cdf(0.5).abs() < 0.1);

        // Should be positive for p>0.5
        assert!(inv_normal_cdf(0.9) > 0.0);

        // Should be negative for p<0.5
        assert!(inv_normal_cdf(0.1) < 0.0);
    }
}

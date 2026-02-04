//! Adversarial detection mechanisms.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::adversarial::types::DetectionResult;

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

//! Latent state representation for the world model.

use alloc::vec;
use alloc::vec::Vec;

/// A latent state vector (compressed representation)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LatentState {
    /// State vector
    pub z: Vec<f64>,
    /// Uncertainty (variance per dimension)
    pub uncertainty: Vec<f64>,
    /// Timestamp
    pub timestamp: u64,
    /// Deterministic component
    pub h: Vec<f64>,
    /// Stochastic component
    pub s: Vec<f64>,
}

impl LatentState {
    /// Create a new latent state
    pub fn new(dim: usize) -> Self {
        Self {
            z: vec![0.0; dim],
            uncertainty: vec![1.0; dim],
            timestamp: 0,
            h: vec![0.0; dim / 2],
            s: vec![0.0; dim / 2],
        }
    }

    /// Create from vector
    pub fn from_vec(z: Vec<f64>) -> Self {
        let dim = z.len();
        let half = dim / 2;

        Self {
            z: z.clone(),
            uncertainty: vec![0.1; dim],
            timestamp: 0,
            h: z[..half].to_vec(),
            s: z[half..].to_vec(),
        }
    }

    /// Dimensionality
    #[inline(always)]
    pub fn dim(&self) -> usize {
        self.z.len()
    }

    /// Sample from distribution (for stochastic models)
    #[inline]
    pub fn sample(&self, noise: &[f64]) -> Vec<f64> {
        self.z
            .iter()
            .zip(self.uncertainty.iter())
            .zip(noise.iter())
            .map(|((&m, &v), &n)| m + libm::sqrt(v) * n)
            .collect()
    }

    /// Total uncertainty (sum of variances)
    #[inline(always)]
    pub fn total_uncertainty(&self) -> f64 {
        self.uncertainty.iter().sum()
    }

    /// Distance to another state
    #[inline]
    pub fn distance(&self, other: &LatentState) -> f64 {
        let sum_sq: f64 = self
            .z
            .iter()
            .zip(other.z.iter())
            .map(|(&a, &b)| (a - b).powi(2))
            .sum();

        libm::sqrt(sum_sq)
    }
}

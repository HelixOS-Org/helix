//! K-Means Clustering
//!
//! Workload clustering using K-Means algorithm.

use alloc::vec;
use alloc::vec::Vec;

use super::Lcg;

// ============================================================================
// K-MEANS CLUSTERING
// ============================================================================

/// K-Means clustering
pub struct KMeans {
    /// Number of clusters
    k: usize,
    /// Maximum iterations
    max_iter: usize,
    /// Centroids
    centroids: Vec<Vec<f64>>,
    /// Cluster assignments
    assignments: Vec<usize>,
    /// Inertia (sum of squared distances to centroids)
    inertia: f64,
    /// Is fitted?
    pub(crate) fitted: bool,
}

impl KMeans {
    /// Create new K-Means
    pub fn new(k: usize) -> Self {
        Self {
            k,
            max_iter: 100,
            centroids: Vec::new(),
            assignments: Vec::new(),
            inertia: f64::INFINITY,
            fitted: false,
        }
    }

    /// Set max iterations
    #[inline(always)]
    pub fn with_max_iter(mut self, max_iter: usize) -> Self {
        self.max_iter = max_iter;
        self
    }

    /// Fit the model
    pub fn fit(&mut self, data: &[Vec<f64>]) {
        if data.is_empty() || data[0].is_empty() {
            return;
        }

        let n_samples = data.len();
        let n_features = data[0].len();

        // Initialize centroids (k-means++ style)
        self.centroids = self.init_centroids(data);
        self.assignments = vec![0; n_samples];

        for _ in 0..self.max_iter {
            // Assign to nearest centroid
            let old_assignments = self.assignments.clone();
            for (i, sample) in data.iter().enumerate() {
                self.assignments[i] = self.nearest_centroid(sample);
            }

            // Check convergence
            if self.assignments == old_assignments {
                break;
            }

            // Update centroids
            self.centroids = vec![vec![0.0; n_features]; self.k];
            let mut counts = vec![0usize; self.k];

            for (i, sample) in data.iter().enumerate() {
                let c = self.assignments[i];
                counts[c] += 1;
                for (j, &val) in sample.iter().enumerate() {
                    self.centroids[c][j] += val;
                }
            }

            for (c, &count) in counts.iter().enumerate().take(self.k) {
                if count > 0 {
                    for val in self.centroids[c].iter_mut() {
                        *val /= count as f64;
                    }
                }
            }
        }

        // Calculate inertia
        self.inertia = data
            .iter()
            .enumerate()
            .map(|(i, sample)| {
                let c = self.assignments[i];
                self.distance_squared(sample, &self.centroids[c])
            })
            .sum();

        self.fitted = true;
    }

    fn init_centroids(&self, data: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut centroids = Vec::with_capacity(self.k);
        let mut rng = Lcg::new(42);

        // First centroid is random
        let first_idx = rng.next() as usize % data.len();
        centroids.push(data[first_idx].clone());

        // Remaining centroids by k-means++
        while centroids.len() < self.k {
            let distances: Vec<f64> = data
                .iter()
                .map(|sample| {
                    centroids
                        .iter()
                        .map(|c| self.distance_squared(sample, c))
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();

            let total: f64 = distances.iter().sum();
            let target = (rng.next() as f64 / u64::MAX as f64) * total;

            let mut cumsum = 0.0;
            for (i, &d) in distances.iter().enumerate() {
                cumsum += d;
                if cumsum >= target {
                    centroids.push(data[i].clone());
                    break;
                }
            }
        }

        centroids
    }

    fn nearest_centroid(&self, sample: &[f64]) -> usize {
        self.centroids
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = self.distance_squared(sample, a);
                let db = self.distance_squared(sample, b);
                da.partial_cmp(&db).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn distance_squared(&self, a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum()
    }

    /// Predict cluster for new sample
    #[inline]
    pub fn predict(&self, sample: &[f64]) -> Option<usize> {
        if !self.fitted {
            return None;
        }
        Some(self.nearest_centroid(sample))
    }

    /// Get centroids
    #[inline(always)]
    pub fn centroids(&self) -> &[Vec<f64>] {
        &self.centroids
    }

    /// Get cluster assignments
    #[inline(always)]
    pub fn labels(&self) -> &[usize] {
        &self.assignments
    }

    /// Get inertia
    #[inline(always)]
    pub fn inertia(&self) -> f64 {
        self.inertia
    }
}

impl Default for KMeans {
    fn default() -> Self {
        Self::new(3)
    }
}

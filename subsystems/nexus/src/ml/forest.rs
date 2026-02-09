//! Random Forest Ensemble
//!
//! Ensemble of decision trees for improved predictions.

use alloc::vec::Vec;

use super::{DecisionTree, FeatureVector, LabeledSample, Lcg};

// ============================================================================
// RANDOM FOREST
// ============================================================================

/// Random forest ensemble
pub struct RandomForest {
    /// Individual trees
    trees: Vec<DecisionTree>,
    /// Number of trees
    n_trees: usize,
    /// Max depth per tree
    max_depth: usize,
    /// Sample ratio for bagging
    sample_ratio: f64,
    /// Random seed
    seed: u64,
    /// Is fitted?
    fitted: bool,
}

impl RandomForest {
    /// Create a new random forest
    pub fn new(n_trees: usize, max_depth: usize) -> Self {
        Self {
            trees: Vec::new(),
            n_trees,
            max_depth,
            sample_ratio: 0.8,
            seed: 42,
            fitted: false,
        }
    }

    /// Set sample ratio
    #[inline(always)]
    pub fn with_sample_ratio(mut self, ratio: f64) -> Self {
        self.sample_ratio = ratio.clamp(0.1, 1.0);
        self
    }

    /// Set seed
    #[inline(always)]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fit the forest
    pub fn fit(&mut self, samples: &[LabeledSample]) {
        if samples.is_empty() {
            return;
        }

        self.trees.clear();
        let mut rng = Lcg::new(self.seed);

        for _ in 0..self.n_trees {
            // Bootstrap sample
            let sample_size = (samples.len() as f64 * self.sample_ratio) as usize;
            let bootstrap: Vec<LabeledSample> = (0..sample_size)
                .map(|_| {
                    let idx = rng.next() as usize % samples.len();
                    samples[idx].clone()
                })
                .collect();

            // Train tree
            let mut tree = DecisionTree::new(self.max_depth, 2);
            tree.fit(&bootstrap);
            self.trees.push(tree);
        }

        self.fitted = true;
    }

    /// Predict by majority vote / average
    pub fn predict(&self, features: &FeatureVector) -> Option<f64> {
        if !self.fitted || self.trees.is_empty() {
            return None;
        }

        let predictions: Vec<f64> = self
            .trees
            .iter()
            .filter_map(|t| t.predict(features))
            .collect();

        if predictions.is_empty() {
            return None;
        }

        // For classification, use majority vote
        // For regression, use mean
        Some(predictions.iter().sum::<f64>() / predictions.len() as f64)
    }

    /// Predict with confidence
    pub fn predict_proba(&self, features: &FeatureVector) -> Option<(f64, f64)> {
        if !self.fitted || self.trees.is_empty() {
            return None;
        }

        let predictions: Vec<(f64, f64)> = self
            .trees
            .iter()
            .filter_map(|t| t.predict_proba(features))
            .collect();

        if predictions.is_empty() {
            return None;
        }

        let avg_pred = predictions.iter().map(|(p, _)| p).sum::<f64>() / predictions.len() as f64;
        let avg_conf = predictions.iter().map(|(_, c)| c).sum::<f64>() / predictions.len() as f64;

        Some((avg_pred, avg_conf))
    }

    /// Get aggregated feature importances
    pub fn feature_importances(&self) -> Vec<f64> {
        if self.trees.is_empty() {
            return Vec::new();
        }

        let n_features = self
            .trees
            .iter()
            .map(|t| t.feature_importances().len())
            .max()
            .unwrap_or(0);

        let mut importances = alloc::vec![0.0; n_features];

        for tree in &self.trees {
            for (i, &imp) in tree.feature_importances().iter().enumerate() {
                if i < importances.len() {
                    importances[i] += imp;
                }
            }
        }

        let n_trees = self.trees.len() as f64;
        for imp in &mut importances {
            *imp /= n_trees;
        }

        importances
    }

    /// Number of trees
    #[inline(always)]
    pub fn n_trees(&self) -> usize {
        self.trees.len()
    }
}

impl Default for RandomForest {
    fn default() -> Self {
        Self::new(10, 5)
    }
}

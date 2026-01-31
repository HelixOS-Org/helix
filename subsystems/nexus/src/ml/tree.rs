//! Decision Tree Implementation
//!
//! Fast classification and regression using decision trees.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use super::{Feature, FeatureVector, LabeledSample};

// ============================================================================
// DECISION TREE
// ============================================================================

/// Decision tree node
#[derive(Debug, Clone)]
pub enum DecisionNode {
    /// Leaf node with prediction
    Leaf {
        /// Class label or regression value
        prediction: f64,
        /// Sample count at this leaf
        samples: usize,
        /// Confidence
        confidence: f64,
    },
    /// Split node
    Split {
        /// Feature index to split on
        feature: usize,
        /// Split threshold
        threshold: f64,
        /// Left child (feature <= threshold)
        left: usize,
        /// Right child (feature > threshold)
        right: usize,
        /// Feature importance contribution
        importance: f64,
    },
}

/// Decision tree classifier/regressor
pub struct DecisionTree {
    /// Nodes
    nodes: Vec<DecisionNode>,
    /// Maximum depth
    max_depth: usize,
    /// Minimum samples for split
    min_samples_split: usize,
    /// Feature importances
    feature_importances: Vec<f64>,
    /// Number of features
    n_features: usize,
    /// Is fitted?
    pub(crate) fitted: bool,
}

impl DecisionTree {
    /// Create a new decision tree
    pub fn new(max_depth: usize, min_samples_split: usize) -> Self {
        Self {
            nodes: Vec::new(),
            max_depth,
            min_samples_split,
            feature_importances: Vec::new(),
            n_features: 0,
            fitted: false,
        }
    }

    /// Fit the tree on training data
    pub fn fit(&mut self, samples: &[LabeledSample]) {
        if samples.is_empty() {
            return;
        }

        // Determine number of features
        self.n_features = samples
            .iter()
            .flat_map(|s| s.features.iter())
            .map(|f| f.index + 1)
            .max()
            .unwrap_or(0);

        self.feature_importances = vec![0.0; self.n_features];
        self.nodes.clear();

        // Build tree recursively
        let indices: Vec<usize> = (0..samples.len()).collect();
        self.build_tree(samples, &indices, 0);

        // Normalize feature importances
        let total: f64 = self.feature_importances.iter().sum();
        if total > 0.0 {
            for imp in &mut self.feature_importances {
                *imp /= total;
            }
        }

        self.fitted = true;
    }

    fn build_tree(&mut self, samples: &[LabeledSample], indices: &[usize], depth: usize) -> usize {
        let node_idx = self.nodes.len();

        // Check stopping conditions
        if depth >= self.max_depth || indices.len() < self.min_samples_split {
            let prediction = self.mean_label(samples, indices);
            let confidence = self.gini_impurity(samples, indices);
            self.nodes.push(DecisionNode::Leaf {
                prediction,
                samples: indices.len(),
                confidence: 1.0 - confidence,
            });
            return node_idx;
        }

        // Find best split
        if let Some((feature, threshold, left_indices, right_indices, importance)) =
            self.find_best_split(samples, indices)
        {
            // Placeholder for children
            self.nodes.push(DecisionNode::Split {
                feature,
                threshold,
                left: 0,
                right: 0,
                importance,
            });

            // Update feature importance
            if feature < self.feature_importances.len() {
                self.feature_importances[feature] += importance;
            }

            // Build children
            let left_idx = self.build_tree(samples, &left_indices, depth + 1);
            let right_idx = self.build_tree(samples, &right_indices, depth + 1);

            // Update node with children
            if let Some(DecisionNode::Split { left, right, .. }) = self.nodes.get_mut(node_idx) {
                *left = left_idx;
                *right = right_idx;
            }

            node_idx
        } else {
            // Can't split, make leaf
            let prediction = self.mean_label(samples, indices);
            self.nodes.push(DecisionNode::Leaf {
                prediction,
                samples: indices.len(),
                confidence: 1.0,
            });
            node_idx
        }
    }

    fn find_best_split(
        &self,
        samples: &[LabeledSample],
        indices: &[usize],
    ) -> Option<(usize, f64, Vec<usize>, Vec<usize>, f64)> {
        let mut best_gain = 0.0;
        let mut best_split = None;

        let parent_impurity = self.gini_impurity(samples, indices);

        for feature in 0..self.n_features {
            // Get unique values for this feature
            let mut values: Vec<f64> = indices
                .iter()
                .map(|&i| samples[i].features.get(feature))
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
            values.dedup();

            // Try each threshold
            for window in values.windows(2) {
                let threshold = (window[0] + window[1]) / 2.0;

                let (left, right): (Vec<usize>, Vec<usize>) = indices
                    .iter()
                    .copied()
                    .partition(|&i| samples[i].features.get(feature) <= threshold);

                if left.is_empty() || right.is_empty() {
                    continue;
                }

                // Calculate information gain
                let left_impurity = self.gini_impurity(samples, &left);
                let right_impurity = self.gini_impurity(samples, &right);

                let n = indices.len() as f64;
                let weighted_impurity = (left.len() as f64 / n) * left_impurity
                    + (right.len() as f64 / n) * right_impurity;

                let gain = parent_impurity - weighted_impurity;

                if gain > best_gain {
                    best_gain = gain;
                    best_split = Some((feature, threshold, left, right, gain));
                }
            }
        }

        best_split
    }

    fn gini_impurity(&self, samples: &[LabeledSample], indices: &[usize]) -> f64 {
        if indices.is_empty() {
            return 0.0;
        }

        let mut class_counts: BTreeMap<i64, usize> = BTreeMap::new();
        for &i in indices {
            let class = samples[i].label as i64;
            *class_counts.entry(class).or_insert(0) += 1;
        }

        let n = indices.len() as f64;
        let impurity: f64 = class_counts
            .values()
            .map(|&count| {
                let p = count as f64 / n;
                p * (1.0 - p)
            })
            .sum();

        impurity
    }

    fn mean_label(&self, samples: &[LabeledSample], indices: &[usize]) -> f64 {
        if indices.is_empty() {
            return 0.0;
        }

        let sum: f64 = indices.iter().map(|&i| samples[i].label).sum();
        sum / indices.len() as f64
    }

    /// Predict class/value for a sample
    pub fn predict(&self, features: &FeatureVector) -> Option<f64> {
        if !self.fitted || self.nodes.is_empty() {
            return None;
        }

        let mut node_idx = 0;
        loop {
            match &self.nodes[node_idx] {
                DecisionNode::Leaf { prediction, .. } => return Some(*prediction),
                DecisionNode::Split {
                    feature,
                    threshold,
                    left,
                    right,
                    ..
                } => {
                    if features.get(*feature) <= *threshold {
                        node_idx = *left;
                    } else {
                        node_idx = *right;
                    }
                },
            }
        }
    }

    /// Get prediction with confidence
    pub fn predict_proba(&self, features: &FeatureVector) -> Option<(f64, f64)> {
        if !self.fitted || self.nodes.is_empty() {
            return None;
        }

        let mut node_idx = 0;
        loop {
            match &self.nodes[node_idx] {
                DecisionNode::Leaf {
                    prediction,
                    confidence,
                    ..
                } => return Some((*prediction, *confidence)),
                DecisionNode::Split {
                    feature,
                    threshold,
                    left,
                    right,
                    ..
                } => {
                    if features.get(*feature) <= *threshold {
                        node_idx = *left;
                    } else {
                        node_idx = *right;
                    }
                },
            }
        }
    }

    /// Get feature importances
    pub fn feature_importances(&self) -> &[f64] {
        &self.feature_importances
    }

    /// Get number of nodes
    pub fn n_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get tree depth
    pub fn depth(&self) -> usize {
        self.compute_depth(0)
    }

    fn compute_depth(&self, node_idx: usize) -> usize {
        match self.nodes.get(node_idx) {
            Some(DecisionNode::Leaf { .. }) => 1,
            Some(DecisionNode::Split { left, right, .. }) => {
                1 + self.compute_depth(*left).max(self.compute_depth(*right))
            },
            None => 0,
        }
    }
}

impl Default for DecisionTree {
    fn default() -> Self {
        Self::new(10, 2)
    }
}

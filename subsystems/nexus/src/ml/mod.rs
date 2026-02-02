//! # Machine Learning Framework
//!
//! Lightweight ML primitives for kernel intelligence.
//!
//! ## Key Features
//!
//! - **Decision Trees**: Fast classification and regression
//! - **Neural Networks**: Tiny neural networks for inference
//! - **K-Means Clustering**: Workload clustering
//! - **Online Learning**: Incremental model updates
//! - **Feature Engineering**: Automatic feature extraction

// Submodules
mod clustering;
mod forest;
mod neural;
mod sgd;
mod tree;
mod types;
mod utils;

// Re-export core types
// Re-export clustering
pub use clustering::KMeans;
// Re-export random forest
pub use forest::RandomForest;
// Re-export neural network
pub use neural::{Activation, DenseLayer, TinyNN};
// Re-export SGD
pub use sgd::SGDClassifier;
// Re-export decision tree
pub use tree::{DecisionNode, DecisionTree};
pub use types::{Feature, FeatureVector, LabeledSample};
// Re-export utilities
pub use utils::{Lcg, ModelRegistry, sigmoid};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector() {
        let mut fv = FeatureVector::new();
        fv.add(0, 1.0);
        fv.add(2, 3.0);

        assert_eq!(fv.get(0), 1.0);
        assert_eq!(fv.get(1), 0.0);
        assert_eq!(fv.get(2), 3.0);
    }

    #[test]
    fn test_decision_tree() {
        let samples = vec![
            LabeledSample::new(FeatureVector::from_dense(vec![0.0, 0.0]), 0.0),
            LabeledSample::new(FeatureVector::from_dense(vec![1.0, 0.0]), 1.0),
            LabeledSample::new(FeatureVector::from_dense(vec![0.0, 1.0]), 1.0),
            LabeledSample::new(FeatureVector::from_dense(vec![1.0, 1.0]), 0.0),
        ];

        let mut tree = DecisionTree::new(5, 1);
        tree.fit(&samples);

        assert!(tree.fitted);
        assert!(tree.n_nodes() > 0);
    }

    #[test]
    fn test_kmeans() {
        let data = vec![vec![0.0, 0.0], vec![0.1, 0.1], vec![10.0, 10.0], vec![
            10.1, 10.1,
        ]];

        let mut kmeans = KMeans::new(2);
        kmeans.fit(&data);

        assert!(kmeans.fitted);
        assert_eq!(kmeans.centroids().len(), 2);
    }

    #[test]
    fn test_sgd() {
        let mut sgd = SGDClassifier::new(2, 0.1);

        let features = FeatureVector::from_dense(vec![1.0, 0.0]);
        sgd.partial_fit(&features, 1.0);

        assert!(sgd.n_updates() > 0);
    }

    #[test]
    fn test_tiny_nn() {
        let mut nn = TinyNN::new(0.1);
        nn.add_layer(DenseLayer::new(2, 4, Activation::ReLU));
        nn.add_layer(DenseLayer::new(4, 1, Activation::Sigmoid));

        let output = nn.predict(&[0.5, 0.5]);
        assert_eq!(output.len(), 1);
    }
}

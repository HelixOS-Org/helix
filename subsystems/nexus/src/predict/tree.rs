//! Decision tree for prediction
//!
//! This module provides the DecisionNode structure for building
//! fast, deterministic, and explainable decision trees for crash prediction.

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;

use super::feature::Feature;
use super::types::PredictionKind;

/// A node in the decision tree
#[derive(Debug, Clone)]
pub enum DecisionNode {
    /// Leaf node with prediction
    Leaf {
        /// Prediction kind
        kind: PredictionKind,
        /// Base confidence
        confidence: f32,
        /// Time to failure estimate (ms)
        time_to_failure_ms: u64,
    },
    /// Split node
    Split {
        /// Feature to split on
        feature_id: u16,
        /// Threshold value
        threshold: f64,
        /// Left child (< threshold)
        left: Box<DecisionNode>,
        /// Right child (>= threshold)
        right: Box<DecisionNode>,
    },
}

impl DecisionNode {
    /// Create a leaf node
    #[inline]
    pub fn leaf(kind: PredictionKind, confidence: f32, time_to_failure_ms: u64) -> Self {
        Self::Leaf {
            kind,
            confidence,
            time_to_failure_ms,
        }
    }

    /// Create a split node
    #[inline]
    pub fn split(feature_id: u16, threshold: f64, left: DecisionNode, right: DecisionNode) -> Self {
        Self::Split {
            feature_id,
            threshold,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Evaluate the tree with given features
    pub fn evaluate(&self, features: &[Feature]) -> Option<(PredictionKind, f32, u64)> {
        match self {
            Self::Leaf {
                kind,
                confidence,
                time_to_failure_ms,
            } => Some((*kind, *confidence, *time_to_failure_ms)),
            Self::Split {
                feature_id,
                threshold,
                left,
                right,
            } => {
                let feature = features.iter().find(|f| f.id == *feature_id)?;

                if feature.value < *threshold {
                    left.evaluate(features)
                } else {
                    right.evaluate(features)
                }
            },
        }
    }

    /// Get tree depth
    #[inline]
    pub fn depth(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { left, right, .. } => 1 + left.depth().max(right.depth()),
        }
    }

    /// Count nodes
    #[inline]
    pub fn node_count(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { left, right, .. } => 1 + left.node_count() + right.node_count(),
        }
    }

    /// Get all feature IDs used in the tree
    #[inline]
    pub fn feature_ids(&self) -> alloc::vec::Vec<u16> {
        let mut ids = alloc::vec::Vec::new();
        self.collect_feature_ids(&mut ids);
        ids
    }

    /// Collect feature IDs recursively
    fn collect_feature_ids(&self, ids: &mut alloc::vec::Vec<u16>) {
        match self {
            Self::Leaf { .. } => {},
            Self::Split {
                feature_id,
                left,
                right,
                ..
            } => {
                if !ids.contains(feature_id) {
                    ids.push(*feature_id);
                }
                left.collect_feature_ids(ids);
                right.collect_feature_ids(ids);
            },
        }
    }
}

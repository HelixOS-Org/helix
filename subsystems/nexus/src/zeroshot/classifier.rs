//! Attribute-based zero-shot classifier.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::zeroshot::types::{AttributeVector, ClassId, EmbeddingVector};

/// Attribute-based zero-shot classifier
#[derive(Debug, Clone)]
pub struct AttributeClassifier {
    /// Class attribute matrix (num_classes × attribute_dim)
    class_attributes: BTreeMap<ClassId, AttributeVector>,
    /// Seen class IDs
    pub(crate) seen_classes: Vec<ClassId>,
    /// Unseen class IDs
    unseen_classes: Vec<ClassId>,
    /// Compatibility function weights
    compatibility_weights: Vec<f64>,
    /// Calibration parameter for GZSL
    calibration: f64,
}

impl AttributeClassifier {
    /// Create a new attribute classifier
    pub fn new(attribute_dim: usize) -> Self {
        Self {
            class_attributes: BTreeMap::new(),
            seen_classes: Vec::new(),
            unseen_classes: Vec::new(),
            compatibility_weights: alloc::vec![1.0; attribute_dim],
            calibration: 0.5,
        }
    }

    /// Register a class with its attributes
    pub fn register_class(
        &mut self,
        class_id: ClassId,
        attributes: AttributeVector,
        is_seen: bool,
    ) {
        self.class_attributes.insert(class_id, attributes);
        if is_seen {
            if !self.seen_classes.contains(&class_id) {
                self.seen_classes.push(class_id);
            }
        } else if !self.unseen_classes.contains(&class_id) {
            self.unseen_classes.push(class_id);
        }
    }

    /// Compute compatibility score between embedding and class
    pub fn compatibility(&self, embedding: &EmbeddingVector, class_id: ClassId) -> f64 {
        if let Some(attrs) = self.class_attributes.get(&class_id) {
            let min_len = embedding
                .len()
                .min(attrs.len())
                .min(self.compatibility_weights.len());
            let mut score = 0.0;
            for i in 0..min_len {
                score += embedding[i] * attrs[i] * self.compatibility_weights[i];
            }
            score
        } else {
            f64::NEG_INFINITY
        }
    }

    /// Zero-shot classification (unseen classes only)
    pub fn classify_zsl(&self, embedding: &EmbeddingVector) -> Option<ClassId> {
        let mut best_class = None;
        let mut best_score = f64::NEG_INFINITY;

        for &class_id in &self.unseen_classes {
            let score = self.compatibility(embedding, class_id);
            if score > best_score {
                best_score = score;
                best_class = Some(class_id);
            }
        }

        best_class
    }

    /// Generalized zero-shot classification (seen + unseen)
    pub fn classify_gzsl(&self, embedding: &EmbeddingVector) -> Option<(ClassId, f64)> {
        let mut best_class = None;
        let mut best_score = f64::NEG_INFINITY;
        let mut scores: Vec<(ClassId, f64)> = Vec::new();

        // Score seen classes (with calibration penalty)
        for &class_id in &self.seen_classes {
            let score = self.compatibility(embedding, class_id) - self.calibration;
            scores.push((class_id, score));
            if score > best_score {
                best_score = score;
                best_class = Some(class_id);
            }
        }

        // Score unseen classes
        for &class_id in &self.unseen_classes {
            let score = self.compatibility(embedding, class_id);
            scores.push((class_id, score));
            if score > best_score {
                best_score = score;
                best_class = Some(class_id);
            }
        }

        best_class.map(|c| (c, best_score))
    }

    /// Set calibration parameter
    pub fn set_calibration(&mut self, calibration: f64) {
        self.calibration = calibration;
    }
}

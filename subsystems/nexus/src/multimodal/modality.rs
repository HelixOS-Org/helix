//! Modality input structures for multimodal fusion.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::multimodal::types::ModalityType;

/// A modality input
#[derive(Debug, Clone)]
pub struct ModalityInput {
    /// Modality type
    pub modality_type: ModalityType,
    /// Feature vector
    pub features: Vec<f64>,
    /// Timestamp
    pub timestamp: u64,
    /// Is this modality present?
    pub present: bool,
    /// Modality-specific metadata
    pub metadata: BTreeMap<String, f64>,
}

impl ModalityInput {
    /// Create a new modality input
    pub fn new(modality_type: ModalityType, features: Vec<f64>) -> Self {
        Self {
            modality_type,
            features,
            timestamp: 0,
            present: true,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a missing modality
    pub fn missing(modality_type: ModalityType, dim: usize) -> Self {
        Self {
            modality_type,
            features: vec![0.0; dim],
            timestamp: 0,
            present: false,
            metadata: BTreeMap::new(),
        }
    }

    /// Dimension of features
    pub fn dim(&self) -> usize {
        self.features.len()
    }
}

/// Multimodal input (collection of modalities)
#[derive(Debug, Clone)]
pub struct MultimodalInput {
    /// Modalities by type
    pub modalities: BTreeMap<ModalityType, ModalityInput>,
    /// Global timestamp
    pub timestamp: u64,
}

impl MultimodalInput {
    /// Create a new multimodal input
    pub fn new() -> Self {
        Self {
            modalities: BTreeMap::new(),
            timestamp: 0,
        }
    }

    /// Add a modality
    pub fn add(&mut self, input: ModalityInput) {
        self.modalities.insert(input.modality_type, input);
    }

    /// Get a modality
    pub fn get(&self, modality_type: ModalityType) -> Option<&ModalityInput> {
        self.modalities.get(&modality_type)
    }

    /// Number of modalities present
    pub fn num_present(&self) -> usize {
        self.modalities.values().filter(|m| m.present).count()
    }

    /// Check if a modality is present
    pub fn has(&self, modality_type: ModalityType) -> bool {
        self.modalities
            .get(&modality_type)
            .map(|m| m.present)
            .unwrap_or(false)
    }
}

impl Default for MultimodalInput {
    fn default() -> Self {
        Self::new()
    }
}

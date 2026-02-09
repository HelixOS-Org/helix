//! Handling strategies for missing modalities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::multimodal::types::{MissingModalityStrategy, ModalityType};

/// Missing modality handler
#[derive(Debug, Clone)]
pub struct MissingModalityHandler {
    /// Strategy
    pub strategy: MissingModalityStrategy,
    /// Learned defaults per modality
    pub defaults: BTreeMap<ModalityType, Vec<f64>>,
    /// Cross-modal imputation weights
    pub imputation_weights: Vec<Vec<f64>>,
}

impl MissingModalityHandler {
    /// Create a new handler
    pub fn new(strategy: MissingModalityStrategy) -> Self {
        Self {
            strategy,
            defaults: BTreeMap::new(),
            imputation_weights: Vec::new(),
        }
    }

    /// Set learned default for a modality
    #[inline(always)]
    pub fn set_default(&mut self, modality: ModalityType, default: Vec<f64>) {
        self.defaults.insert(modality, default);
    }

    /// Handle missing modality
    pub fn handle(
        &self,
        modality: ModalityType,
        dim: usize,
        other_modalities: &[&[f64]],
    ) -> Vec<f64> {
        match self.strategy {
            MissingModalityStrategy::Zero => {
                vec![0.0; dim]
            }
            MissingModalityStrategy::LearnedDefault => self
                .defaults
                .get(&modality)
                .cloned()
                .unwrap_or_else(|| vec![0.0; dim]),
            MissingModalityStrategy::Impute => self.impute(dim, other_modalities),
            MissingModalityStrategy::DropReweight => {
                vec![0.0; dim] // Will be handled by fusion
            }
        }
    }

    /// Impute from other modalities
    fn impute(&self, dim: usize, other_modalities: &[&[f64]]) -> Vec<f64> {
        if other_modalities.is_empty() {
            return vec![0.0; dim];
        }

        // Simple average of other modalities (projected to right dimension)
        let mut imputed = vec![0.0; dim];

        for modality in other_modalities {
            for (i, &v) in modality.iter().take(dim).enumerate() {
                imputed[i] += v;
            }
        }

        for v in &mut imputed {
            *v /= other_modalities.len() as f64;
        }

        imputed
    }
}

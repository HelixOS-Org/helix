//! Kernel multimodal fusion manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::multimodal::fusion::LateFusion;
use crate::multimodal::missing::MissingModalityHandler;
use crate::multimodal::modality::MultimodalInput;
use crate::multimodal::transformer::MultimodalTransformer;
use crate::multimodal::types::{
    MissingModalityStrategy, ModalityType, DEFAULT_ATTENTION_HEADS, DEFAULT_HIDDEN_DIM,
};

/// Kernel multimodal fusion manager
pub struct KernelMultimodalManager {
    /// Transformer model
    pub transformer: MultimodalTransformer,
    /// Late fusion backup
    pub late_fusion: LateFusion,
    /// Missing modality handler
    pub missing_handler: MissingModalityHandler,
    /// Modality importance scores
    pub importance_scores: BTreeMap<ModalityType, f64>,
    /// Recent fusion outputs
    pub fusion_history: VecDeque<Vec<f64>>,
    /// Maximum history size
    max_history: usize,
}

impl KernelMultimodalManager {
    /// Create a new kernel multimodal manager
    pub fn new() -> Self {
        let modality_dims = &[
            (ModalityType::Metrics, 32),
            (ModalityType::Logs, 64),
            (ModalityType::Events, 16),
            (ModalityType::Network, 24),
        ];

        let dims: Vec<usize> = modality_dims.iter().map(|(_, d)| *d).collect();

        Self {
            transformer: MultimodalTransformer::new(
                modality_dims,
                DEFAULT_HIDDEN_DIM,
                2,
                DEFAULT_ATTENTION_HEADS,
                12345,
            ),
            late_fusion: LateFusion::new(&dims, DEFAULT_HIDDEN_DIM, 12345),
            missing_handler: MissingModalityHandler::new(MissingModalityStrategy::LearnedDefault),
            importance_scores: BTreeMap::new(),
            fusion_history: VecDeque::new(),
            max_history: 100,
        }
    }

    /// Fuse multimodal input
    #[inline]
    pub fn fuse(&mut self, input: &MultimodalInput) -> Vec<f64> {
        let fused = self.transformer.forward(input);

        // Store in history
        if self.fusion_history.len() >= self.max_history {
            self.fusion_history.pop_front();
        }
        self.fusion_history.push_back(fused.clone());

        fused
    }

    /// Get modality importance
    #[inline]
    pub fn get_importance(&self, modality: ModalityType) -> f64 {
        self.importance_scores
            .get(&modality)
            .copied()
            .unwrap_or(1.0)
    }

    /// Update modality importance based on prediction quality
    #[inline(always)]
    pub fn update_importance(&mut self, modality: ModalityType, quality_delta: f64) {
        let current = self.importance_scores.entry(modality).or_insert(1.0);
        *current = (*current + quality_delta * 0.1).clamp(0.1, 2.0);
    }

    /// Get fusion statistics
    pub fn get_stats(&self) -> MultimodalStats {
        let avg_norm = if !self.fusion_history.is_empty() {
            let total: f64 = self
                .fusion_history
                .iter()
                .map(|v| libm::sqrt(v.iter().map(|x| x * x).sum()))
                .sum();
            total / self.fusion_history.len() as f64
        } else {
            0.0
        };

        MultimodalStats {
            num_modalities: self.transformer.encoders.len(),
            output_dim: self.transformer.output_dim,
            avg_output_norm: avg_norm,
            history_size: self.fusion_history.len(),
        }
    }
}

impl Default for KernelMultimodalManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Multimodal statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MultimodalStats {
    /// Number of modalities
    pub num_modalities: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Average output norm
    pub avg_output_norm: f64,
    /// History size
    pub history_size: usize,
}

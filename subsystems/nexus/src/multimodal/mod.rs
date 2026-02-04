//! # Multimodal Fusion Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary multimodal learning system that enables
//! the kernel to process and fuse information from multiple modalities
//! (metrics, logs, traces, events) for unified understanding.
//!
//! ## Key Features
//!
//! - **Early Fusion**: Concatenate modalities before processing
//! - **Late Fusion**: Process separately then combine decisions
//! - **Cross-Modal Attention**: Learn relationships between modalities
//! - **Modality Alignment**: Project modalities to shared space
//! - **Missing Modality Handling**: Robust to incomplete data
//! - **Dynamic Weighting**: Learn importance of each modality
//!
//! ## Kernel Applications
//!
//! - Unified system monitoring (metrics + logs + traces)
//! - Correlating events across different subsystems
//! - Holistic performance analysis
//! - Comprehensive anomaly detection

#![no_std]

// ============================================================================
// SUBMODULES
// ============================================================================

mod alignment;
mod attention;
mod encoder;
mod fusion;
mod manager;
mod missing;
mod modality;
mod transformer;
mod types;
mod utils;

// ============================================================================
// RE-EXPORTS
// ============================================================================

// Types and constants
pub use types::{
    MissingModalityStrategy, ModalityType, DEFAULT_ATTENTION_HEADS, DEFAULT_HIDDEN_DIM,
    MAX_MODALITIES, MAX_MODALITY_DIM,
};

// Modality input structures
pub use modality::{ModalityInput, MultimodalInput};

// Encoder
pub use encoder::ModalityEncoder;

// Fusion strategies
pub use fusion::{EarlyFusion, LateFusion};

// Attention mechanisms
pub use attention::{BiCrossModalAttention, CrossModalAttention};

// Alignment strategies
pub use alignment::{CCAAlignment, ContrastiveAlignment};

// Missing modality handling
pub use missing::MissingModalityHandler;

// Transformer architecture
pub use transformer::{MultimodalTransformer, MultimodalTransformerBlock};

// Kernel manager
pub use manager::{KernelMultimodalManager, MultimodalStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_input() {
        let input = ModalityInput::new(ModalityType::Metrics, alloc::vec![0.5; 10]);

        assert_eq!(input.modality_type, ModalityType::Metrics);
        assert_eq!(input.dim(), 10);
        assert!(input.present);
    }

    #[test]
    fn test_missing_modality() {
        let input = ModalityInput::missing(ModalityType::Logs, 20);

        assert!(!input.present);
        assert_eq!(input.dim(), 20);
    }

    #[test]
    fn test_multimodal_input() {
        let mut mm_input = MultimodalInput::new();

        mm_input.add(ModalityInput::new(ModalityType::Metrics, alloc::vec![0.5; 10]));
        mm_input.add(ModalityInput::new(ModalityType::Logs, alloc::vec![0.3; 20]));

        assert_eq!(mm_input.num_present(), 2);
        assert!(mm_input.has(ModalityType::Metrics));
        assert!(!mm_input.has(ModalityType::Events));
    }

    #[test]
    fn test_modality_encoder() {
        let encoder = ModalityEncoder::new(10, 32, 12345);
        let input = alloc::vec![0.5; 10];

        let encoded = encoder.encode(&input);

        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_early_fusion() {
        let fusion = EarlyFusion::new(&[10, 20, 15], 32, 64, 12345);

        let inputs: alloc::vec::Vec<alloc::vec::Vec<f64>> = alloc::vec![alloc::vec![0.5; 10], alloc::vec![0.3; 20], alloc::vec![0.7; 15]];

        let input_refs: alloc::vec::Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();
        let fused = fusion.fuse(&input_refs);

        assert_eq!(fused.len(), 32);
    }

    #[test]
    fn test_late_fusion() {
        let fusion = LateFusion::new(&[10, 20, 15], 32, 12345);

        let inputs: alloc::vec::Vec<alloc::vec::Vec<f64>> = alloc::vec![alloc::vec![0.5; 10], alloc::vec![0.3; 20], alloc::vec![0.7; 15]];

        let input_refs: alloc::vec::Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();
        let present = alloc::vec![true, true, true];

        let fused = fusion.fuse(&input_refs, &present);

        assert_eq!(fused.len(), 32);
    }

    #[test]
    fn test_late_fusion_missing() {
        let fusion = LateFusion::new(&[10, 20, 15], 32, 12345);

        let inputs: alloc::vec::Vec<alloc::vec::Vec<f64>> = alloc::vec![alloc::vec![0.5; 10], alloc::vec![0.3; 20], alloc::vec![0.7; 15]];

        let input_refs: alloc::vec::Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();
        let present = alloc::vec![true, false, true]; // Second modality missing

        let fused = fusion.fuse(&input_refs, &present);

        assert_eq!(fused.len(), 32);
    }

    #[test]
    fn test_cross_modal_attention() {
        let attention = CrossModalAttention::new(32, 4, 12345);

        let query = alloc::vec![0.5; 32];
        let key = alloc::vec![0.3; 32];
        let value = alloc::vec![0.7; 32];

        let attended = attention.attend(&query, &key, &value);

        assert_eq!(attended.len(), 32);
    }

    #[test]
    fn test_multi_attend() {
        let attention = CrossModalAttention::new(32, 4, 12345);

        let query = alloc::vec![0.5; 32];
        let keys = alloc::vec![alloc::vec![0.3; 32], alloc::vec![0.4; 32]];
        let values = alloc::vec![alloc::vec![0.7; 32], alloc::vec![0.8; 32]];

        let key_refs: alloc::vec::Vec<&[f64]> = keys.iter().map(|v| v.as_slice()).collect();
        let value_refs: alloc::vec::Vec<&[f64]> = values.iter().map(|v| v.as_slice()).collect();

        let attended = attention.multi_attend(&query, &key_refs, &value_refs);

        assert_eq!(attended.len(), 32);
    }

    #[test]
    fn test_bi_cross_modal_attention() {
        let attention = BiCrossModalAttention::new(32, 4, 12345);

        let mod_a = alloc::vec![0.5; 32];
        let mod_b = alloc::vec![0.3; 32];

        let (a_out, b_out) = attention.attend(&mod_a, &mod_b);

        assert_eq!(a_out.len(), 32);
        assert_eq!(b_out.len(), 32);
    }

    #[test]
    fn test_contrastive_alignment() {
        let alignment = ContrastiveAlignment::new(&[10, 20], 32, 12345);

        let inputs: alloc::vec::Vec<alloc::vec::Vec<f64>> = alloc::vec![alloc::vec![0.5; 10], alloc::vec![0.3; 20]];
        let input_refs: alloc::vec::Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();

        let projected = alignment.project_all(&input_refs);

        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0].len(), 32);
    }

    #[test]
    fn test_cca_alignment() {
        let alignment = CCAAlignment::new(10, 20, 16, 12345);

        let input_a = alloc::vec![0.5; 10];
        let input_b = alloc::vec![0.3; 20];

        let (proj_a, proj_b) = alignment.project(&input_a, &input_b);

        assert_eq!(proj_a.len(), 16);
        assert_eq!(proj_b.len(), 16);

        let corr = alignment.correlation(&input_a, &input_b);
        assert!(corr >= -1.0 && corr <= 1.0);
    }

    #[test]
    fn test_missing_handler() {
        let handler = MissingModalityHandler::new(MissingModalityStrategy::Zero);

        let imputed = handler.handle(ModalityType::Logs, 20, &[]);

        assert_eq!(imputed.len(), 20);
        assert!(imputed.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_missing_handler_impute() {
        let handler = MissingModalityHandler::new(MissingModalityStrategy::Impute);

        let other1 = alloc::vec![1.0; 10];
        let other2 = alloc::vec![2.0; 10];

        let imputed = handler.handle(ModalityType::Logs, 10, &[&other1, &other2]);

        assert_eq!(imputed.len(), 10);
        assert!((imputed[0] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_transformer_block() {
        let block = MultimodalTransformerBlock::new(32, 3, 4, 12345);

        let modalities = alloc::vec![alloc::vec![0.5; 32], alloc::vec![0.3; 32], alloc::vec![0.7; 32]];

        let output = block.forward(&modalities);

        assert_eq!(output.len(), 3);
        assert_eq!(output[0].len(), 32);
    }

    #[test]
    fn test_multimodal_transformer() {
        let modality_dims = &[(ModalityType::Metrics, 10), (ModalityType::Logs, 20)];

        let transformer = MultimodalTransformer::new(modality_dims, 32, 2, 4, 12345);

        let mut input = MultimodalInput::new();
        input.add(ModalityInput::new(ModalityType::Metrics, alloc::vec![0.5; 10]));
        input.add(ModalityInput::new(ModalityType::Logs, alloc::vec![0.3; 20]));

        let output = transformer.forward(&input);

        assert_eq!(output.len(), 32);
    }

    #[test]
    fn test_kernel_multimodal_manager() {
        let mut manager = KernelMultimodalManager::new();

        let mut input = MultimodalInput::new();
        input.add(ModalityInput::new(ModalityType::Metrics, alloc::vec![0.5; 32]));
        input.add(ModalityInput::new(ModalityType::Logs, alloc::vec![0.3; 64]));

        let fused = manager.fuse(&input);

        assert_eq!(fused.len(), DEFAULT_HIDDEN_DIM);
    }

    #[test]
    fn test_importance_update() {
        let mut manager = KernelMultimodalManager::new();

        manager.update_importance(ModalityType::Metrics, 0.5);

        let importance = manager.get_importance(ModalityType::Metrics);
        assert!(importance > 1.0);
    }

    #[test]
    fn test_multimodal_stats() {
        let manager = KernelMultimodalManager::new();
        let stats = manager.get_stats();

        assert_eq!(stats.num_modalities, 4);
        assert_eq!(stats.output_dim, DEFAULT_HIDDEN_DIM);
    }
}

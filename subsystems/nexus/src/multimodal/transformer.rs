//! Multimodal transformer architecture for cross-modal processing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::multimodal::attention::CrossModalAttention;
use crate::multimodal::encoder::ModalityEncoder;
use crate::multimodal::modality::MultimodalInput;
use crate::multimodal::types::ModalityType;
use crate::multimodal::utils::{create_layer, layer_norm, lcg_next, project_vec};

/// Multimodal transformer block
#[derive(Debug, Clone)]
pub struct MultimodalTransformerBlock {
    /// Self-attention per modality
    pub self_attention: Vec<CrossModalAttention>,
    /// Cross-attention between modalities
    pub cross_attention: CrossModalAttention,
    /// FFN weights
    pub ffn_w1: Vec<Vec<f64>>,
    pub ffn_w2: Vec<Vec<f64>>,
    /// Layer norm
    pub ln1_scale: Vec<f64>,
    pub ln1_bias: Vec<f64>,
    pub ln2_scale: Vec<f64>,
    pub ln2_bias: Vec<f64>,
}

impl MultimodalTransformerBlock {
    /// Create a new transformer block
    pub fn new(hidden_dim: usize, num_modalities: usize, num_heads: usize, seed: u64) -> Self {
        let mut rng = seed;

        let mut self_attention = Vec::new();
        for _ in 0..num_modalities {
            self_attention.push(CrossModalAttention::new(hidden_dim, num_heads, rng));
            rng = lcg_next(rng);
        }

        let cross_attention = CrossModalAttention::new(hidden_dim, num_heads, rng);
        rng = lcg_next(rng);

        let ffn_dim = hidden_dim * 4;
        let (ffn_w1, _, rng2) = create_layer(hidden_dim, ffn_dim, rng);
        let (ffn_w2, _, _) = create_layer(ffn_dim, hidden_dim, rng2);

        Self {
            self_attention,
            cross_attention,
            ffn_w1,
            ffn_w2,
            ln1_scale: vec![1.0; hidden_dim],
            ln1_bias: vec![0.0; hidden_dim],
            ln2_scale: vec![1.0; hidden_dim],
            ln2_bias: vec![0.0; hidden_dim],
        }
    }

    /// Forward pass
    pub fn forward(&self, modalities: &[Vec<f64>]) -> Vec<Vec<f64>> {
        if modalities.is_empty() {
            return Vec::new();
        }

        // Self-attention for each modality
        let mut after_self_attn: Vec<Vec<f64>> = Vec::new();

        for (i, modality) in modalities.iter().enumerate() {
            if i < self.self_attention.len() {
                let attended = self.self_attention[i].attend(modality, modality, modality);

                // Residual + LayerNorm
                let residual: Vec<f64> = modality
                    .iter()
                    .zip(attended.iter())
                    .map(|(&m, &a)| m + a)
                    .collect();

                let normalized = layer_norm(&residual, &self.ln1_scale, &self.ln1_bias);
                after_self_attn.push(normalized);
            } else {
                after_self_attn.push(modality.clone());
            }
        }

        // Cross-attention: each modality attends to all others
        let mut after_cross_attn: Vec<Vec<f64>> = Vec::new();

        for (i, modality) in after_self_attn.iter().enumerate() {
            // Get other modalities as keys/values
            let others: Vec<&[f64]> = after_self_attn
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, m)| m.as_slice())
                .collect();

            if !others.is_empty() {
                let attended = self
                    .cross_attention
                    .multi_attend(modality, &others, &others);

                // Residual
                let residual: Vec<f64> = modality
                    .iter()
                    .zip(attended.iter())
                    .map(|(&m, &a)| m + a)
                    .collect();

                after_cross_attn.push(residual);
            } else {
                after_cross_attn.push(modality.clone());
            }
        }

        // FFN for each modality
        let mut output: Vec<Vec<f64>> = Vec::new();

        for modality in &after_cross_attn {
            // FFN
            let hidden = project_vec(&self.ffn_w1, modality);
            let hidden_relu: Vec<f64> = hidden.iter().map(|&v| v.max(0.0)).collect();
            let ffn_out = project_vec(&self.ffn_w2, &hidden_relu);

            // Residual + LayerNorm
            let residual: Vec<f64> = modality
                .iter()
                .zip(ffn_out.iter())
                .map(|(&m, &f)| m + f)
                .collect();

            let normalized = layer_norm(&residual, &self.ln2_scale, &self.ln2_bias);
            output.push(normalized);
        }

        output
    }
}

/// Full multimodal transformer
#[derive(Debug, Clone)]
pub struct MultimodalTransformer {
    /// Modality encoders
    pub encoders: BTreeMap<ModalityType, ModalityEncoder>,
    /// Transformer blocks
    pub blocks: Vec<MultimodalTransformerBlock>,
    /// Output dimension
    pub output_dim: usize,
    /// Hidden dimension
    pub hidden_dim: usize,
}

impl MultimodalTransformer {
    /// Create a new multimodal transformer
    pub fn new(
        modality_dims: &[(ModalityType, usize)],
        hidden_dim: usize,
        num_blocks: usize,
        num_heads: usize,
        seed: u64,
    ) -> Self {
        let mut encoders = BTreeMap::new();
        let mut rng = seed;

        for (modality_type, dim) in modality_dims {
            encoders.insert(*modality_type, ModalityEncoder::new(*dim, hidden_dim, rng));
            rng = lcg_next(rng);
        }

        let mut blocks = Vec::new();
        for _ in 0..num_blocks {
            blocks.push(MultimodalTransformerBlock::new(
                hidden_dim,
                modality_dims.len(),
                num_heads,
                rng,
            ));
            rng = lcg_next(rng);
        }

        Self {
            encoders,
            blocks,
            output_dim: hidden_dim,
            hidden_dim,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &MultimodalInput) -> Vec<f64> {
        // Encode all present modalities
        let mut encoded: Vec<Vec<f64>> = Vec::new();

        for (modality_type, encoder) in &self.encoders {
            if let Some(modality_input) = input.get(*modality_type) {
                if modality_input.present {
                    encoded.push(encoder.encode(&modality_input.features));
                }
            }
        }

        if encoded.is_empty() {
            return vec![0.0; self.hidden_dim];
        }

        // Pass through transformer blocks
        let mut x = encoded;
        for block in &self.blocks {
            x = block.forward(&x);
        }

        // Pool (average)
        let mut pooled = vec![0.0; self.hidden_dim];
        for modality in &x {
            for (p, &m) in pooled.iter_mut().zip(modality.iter()) {
                *p += m;
            }
        }

        for p in &mut pooled {
            *p /= x.len() as f64;
        }

        pooled
    }
}

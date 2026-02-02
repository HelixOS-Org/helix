//! # Transformer Encoder
//!
//! BERT-style bidirectional encoder architecture.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::vec::Vec;

use super::layers::{PostNormBlock, PreNormBlock};
use super::types::{
    Dropout, Embedding, LayerNorm, PositionalEmbedding, Tensor2, TransformerConfig,
};

// ============================================================================
// ENCODER LAYER
// ============================================================================

/// Single encoder layer
pub struct EncoderLayer {
    /// Pre-norm block
    block: PreNormBlock,
}

impl EncoderLayer {
    /// Create new encoder layer
    pub fn new(config: &TransformerConfig, seed: u64) -> Self {
        Self {
            block: PreNormBlock::new(config, seed),
        }
    }

    /// Forward pass
    pub fn forward(&mut self, x: &Tensor2, mask: Option<&Tensor2>, training: bool) -> Tensor2 {
        self.block.forward(x, mask, training)
    }
}

// ============================================================================
// ENCODER STACK
// ============================================================================

/// Transformer encoder stack
pub struct Encoder {
    /// Encoder layers
    pub layers: Vec<EncoderLayer>,
    /// Number of layers
    pub n_layers: usize,
    /// Final layer norm
    pub final_norm: Option<LayerNorm>,
    /// Config
    pub config: TransformerConfig,
}

impl Encoder {
    /// Create new encoder
    pub fn new(config: TransformerConfig, seed: u64) -> Self {
        let mut layers = Vec::with_capacity(config.n_layers);

        for i in 0..config.n_layers {
            layers.push(EncoderLayer::new(
                &config,
                seed.wrapping_add(i as u64 * 1000),
            ));
        }

        let final_norm = if config.pre_norm {
            Some(LayerNorm::new(config.d_model, config.layer_norm_eps))
        } else {
            None
        };

        Self {
            layers,
            n_layers: config.n_layers,
            final_norm,
            config,
        }
    }

    /// Forward pass
    pub fn forward(&mut self, x: &Tensor2, mask: Option<&Tensor2>, training: bool) -> Tensor2 {
        let mut hidden = x.clone();

        for layer in &mut self.layers {
            hidden = layer.forward(&hidden, mask, training);
        }

        if let Some(ref norm) = self.final_norm {
            hidden = norm.forward(&hidden);
        }

        hidden
    }

    /// Get intermediate representations
    pub fn forward_with_intermediates(
        &mut self,
        x: &Tensor2,
        mask: Option<&Tensor2>,
        training: bool,
    ) -> Vec<Tensor2> {
        let mut intermediates = Vec::with_capacity(self.n_layers + 1);
        let mut hidden = x.clone();

        intermediates.push(hidden.clone());

        for layer in &mut self.layers {
            hidden = layer.forward(&hidden, mask, training);
            intermediates.push(hidden.clone());
        }

        if let Some(ref norm) = self.final_norm {
            let last = intermediates.len() - 1;
            intermediates[last] = norm.forward(&hidden);
        }

        intermediates
    }
}

// ============================================================================
// BERT-STYLE ENCODER MODEL
// ============================================================================

/// BERT-style encoder model
pub struct BertEncoder {
    /// Token embedding
    pub token_embed: Embedding,
    /// Position embedding
    pub pos_embed: PositionalEmbedding,
    /// Token type embedding (for segment A/B)
    pub type_embed: Option<Embedding>,
    /// Embedding layer norm
    pub embed_norm: LayerNorm,
    /// Embedding dropout
    pub embed_dropout: Dropout,
    /// Encoder stack
    pub encoder: Encoder,
    /// Pooler (for [CLS] token)
    pub pooler: Option<super::types::Linear>,
}

impl BertEncoder {
    /// Create new BERT encoder
    pub fn new(
        config: TransformerConfig,
        use_type_embed: bool,
        use_pooler: bool,
        seed: u64,
    ) -> Self {
        let token_embed = Embedding::new(config.vocab_size, config.d_model, seed);
        let pos_embed = PositionalEmbedding::learnable(
            config.max_seq_len,
            config.d_model,
            seed.wrapping_add(1),
        );

        let type_embed = if use_type_embed {
            Some(Embedding::new(2, config.d_model, seed.wrapping_add(2)))
        } else {
            None
        };

        let pooler = if use_pooler {
            Some(super::types::Linear::new(
                config.d_model,
                config.d_model,
                true,
                seed.wrapping_add(3),
            ))
        } else {
            None
        };

        Self {
            token_embed,
            pos_embed,
            type_embed,
            embed_norm: LayerNorm::new(config.d_model, config.layer_norm_eps),
            embed_dropout: Dropout::new(config.dropout),
            encoder: Encoder::new(config, seed.wrapping_add(100)),
            pooler,
        }
    }

    /// Create from config with defaults
    pub fn from_config(config: TransformerConfig, seed: u64) -> Self {
        Self::new(config, true, true, seed)
    }

    /// Forward pass for encoding
    pub fn forward(
        &mut self,
        input_ids: &[usize],
        type_ids: Option<&[usize]>,
        attention_mask: Option<&Tensor2>,
        training: bool,
    ) -> EncoderOutput {
        let seq_len = input_ids.len();

        // Token embeddings
        let token_embeddings = self.token_embed.forward(input_ids);

        // Position embeddings
        let pos_embeddings = self.pos_embed.forward(seq_len);

        // Add embeddings
        let mut embeddings = token_embeddings
            .add(&pos_embeddings)
            .unwrap_or(token_embeddings);

        // Type embeddings
        if let (Some(ref type_embed), Some(type_ids)) = (&self.type_embed, type_ids) {
            let type_embeddings = type_embed.forward(type_ids);
            embeddings = embeddings.add(&type_embeddings).unwrap_or(embeddings);
        }

        // Normalize and dropout
        embeddings = self.embed_norm.forward(&embeddings);
        embeddings = self.embed_dropout.forward(&embeddings, training);

        // Encode
        let last_hidden = self.encoder.forward(&embeddings, attention_mask, training);

        // Pooler output ([CLS] token representation)
        let pooled = if let Some(ref pooler) = self.pooler {
            let mut cls_token = Tensor2::new(1, self.encoder.config.d_model);
            for j in 0..self.encoder.config.d_model {
                cls_token.set(0, j, last_hidden.get(0, j));
            }
            let pooled = pooler.forward(&cls_token);
            // Apply tanh
            Some(pooled.apply(libm::tanh))
        } else {
            None
        };

        EncoderOutput {
            last_hidden_state: last_hidden,
            pooler_output: pooled,
        }
    }
}

/// Encoder output
#[derive(Debug)]
pub struct EncoderOutput {
    /// Last hidden state (seq_len, d_model)
    pub last_hidden_state: Tensor2,
    /// Pooled output from [CLS] token (1, d_model)
    pub pooler_output: Option<Tensor2>,
}

// ============================================================================
// SPECIALIZED ENCODERS
// ============================================================================

/// Vision Transformer (ViT) encoder
pub struct VisionEncoder {
    /// Patch embedding
    pub patch_embed: PatchEmbedding,
    /// Position embedding
    pub pos_embed: Tensor2,
    /// CLS token
    pub cls_token: Tensor2,
    /// Pre-norm
    pub norm: LayerNorm,
    /// Dropout
    pub dropout: Dropout,
    /// Encoder
    pub encoder: Encoder,
    /// Final norm
    pub final_norm: LayerNorm,
}

/// Patch embedding for vision
pub struct PatchEmbedding {
    /// Projection weight (patch_size^2 * channels, d_model)
    pub proj: super::types::Linear,
    /// Patch size
    pub patch_size: usize,
    /// Number of channels
    pub channels: usize,
    /// Image size
    pub image_size: usize,
    /// Number of patches
    pub num_patches: usize,
}

impl PatchEmbedding {
    /// Create patch embedding
    pub fn new(
        image_size: usize,
        patch_size: usize,
        channels: usize,
        d_model: usize,
        seed: u64,
    ) -> Self {
        let num_patches = (image_size / patch_size) * (image_size / patch_size);
        let patch_dim = patch_size * patch_size * channels;

        Self {
            proj: super::types::Linear::new(patch_dim, d_model, true, seed),
            patch_size,
            channels,
            image_size,
            num_patches,
        }
    }

    /// Forward: flatten patches and project
    /// Input: (height, width * channels) flattened image
    pub fn forward(&self, image: &Tensor2) -> Tensor2 {
        let patches_per_side = self.image_size / self.patch_size;
        let patch_dim = self.patch_size * self.patch_size * self.channels;

        let mut patches = Tensor2::new(self.num_patches, patch_dim);

        for py in 0..patches_per_side {
            for px in 0..patches_per_side {
                let patch_idx = py * patches_per_side + px;
                let mut flat_idx = 0;

                for y in 0..self.patch_size {
                    let img_y = py * self.patch_size + y;
                    for x in 0..self.patch_size {
                        for c in 0..self.channels {
                            let img_x = (px * self.patch_size + x) * self.channels + c;
                            if img_y < image.rows && img_x < image.cols {
                                patches.set(patch_idx, flat_idx, image.get(img_y, img_x));
                            }
                            flat_idx += 1;
                        }
                    }
                }
            }
        }

        self.proj.forward(&patches)
    }
}

impl VisionEncoder {
    /// Create vision encoder
    pub fn new(
        image_size: usize,
        patch_size: usize,
        channels: usize,
        config: TransformerConfig,
        seed: u64,
    ) -> Self {
        let num_patches = (image_size / patch_size) * (image_size / patch_size);

        // Create position embedding for patches + CLS token
        let pos_embed = Tensor2::random(num_patches + 1, config.d_model, seed);

        // CLS token
        let cls_token = Tensor2::random(1, config.d_model, seed.wrapping_add(1));

        Self {
            patch_embed: PatchEmbedding::new(
                image_size,
                patch_size,
                channels,
                config.d_model,
                seed.wrapping_add(2),
            ),
            pos_embed,
            cls_token,
            norm: LayerNorm::new(config.d_model, config.layer_norm_eps),
            dropout: Dropout::new(config.dropout),
            encoder: Encoder::new(config.clone(), seed.wrapping_add(100)),
            final_norm: LayerNorm::new(config.d_model, config.layer_norm_eps),
        }
    }

    /// Forward pass
    pub fn forward(&mut self, image: &Tensor2, training: bool) -> VisionEncoderOutput {
        let d_model = self.encoder.config.d_model;

        // Patch embedding
        let patch_embeddings = self.patch_embed.forward(image);
        let num_patches = patch_embeddings.rows;

        // Prepend CLS token
        let mut embeddings = Tensor2::new(num_patches + 1, d_model);

        // CLS token
        for j in 0..d_model {
            embeddings.set(0, j, self.cls_token.get(0, j));
        }

        // Patch embeddings
        for i in 0..num_patches {
            for j in 0..d_model {
                embeddings.set(i + 1, j, patch_embeddings.get(i, j));
            }
        }

        // Add position embeddings
        for i in 0..embeddings.rows.min(self.pos_embed.rows) {
            for j in 0..d_model {
                let val = embeddings.get(i, j) + self.pos_embed.get(i, j);
                embeddings.set(i, j, val);
            }
        }

        // Dropout
        embeddings = self.dropout.forward(&embeddings, training);

        // Encode
        let last_hidden = self.encoder.forward(&embeddings, None, training);
        let last_hidden = self.final_norm.forward(&last_hidden);

        // Extract CLS output
        let mut cls_output = Tensor2::new(1, d_model);
        for j in 0..d_model {
            cls_output.set(0, j, last_hidden.get(0, j));
        }

        VisionEncoderOutput {
            last_hidden_state: last_hidden,
            cls_output,
            patch_embeddings,
        }
    }
}

/// Vision encoder output
#[derive(Debug)]
pub struct VisionEncoderOutput {
    /// Last hidden state
    pub last_hidden_state: Tensor2,
    /// CLS token output
    pub cls_output: Tensor2,
    /// Patch embeddings before encoding
    pub patch_embeddings: Tensor2,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_layer() {
        let config = TransformerConfig::tiny();
        let mut layer = EncoderLayer::new(&config, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = layer.forward(&input, None, false);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_encoder_stack() {
        let config = TransformerConfig::tiny();
        let mut encoder = Encoder::new(config, 42);

        let input = Tensor2::random(16, 64, 43);
        let output = encoder.forward(&input, None, false);

        assert_eq!(output.rows, 16);
        assert_eq!(output.cols, 64);
    }

    #[test]
    fn test_bert_encoder() {
        let config = TransformerConfig::tiny();
        let mut bert = BertEncoder::from_config(config, 42);

        let input_ids = alloc::vec![1, 2, 3, 4, 5, 6, 7, 8];
        let output = bert.forward(&input_ids, None, None, false);

        assert_eq!(output.last_hidden_state.rows, 8);
        assert_eq!(output.last_hidden_state.cols, 64);
        assert!(output.pooler_output.is_some());
    }

    #[test]
    fn test_patch_embedding() {
        let patch_embed = PatchEmbedding::new(32, 8, 3, 64, 42);

        // 32x32 image with 3 channels
        let image = Tensor2::random(32, 32 * 3, 43);
        let patches = patch_embed.forward(&image);

        // 32/8 = 4 patches per side, 16 patches total
        assert_eq!(patches.rows, 16);
        assert_eq!(patches.cols, 64);
    }

    #[test]
    fn test_vision_encoder() {
        let config = TransformerConfig::tiny();
        let mut vit = VisionEncoder::new(32, 8, 3, config, 42);

        let image = Tensor2::random(32, 32 * 3, 43);
        let output = vit.forward(&image, false);

        // 16 patches + 1 CLS token
        assert_eq!(output.last_hidden_state.rows, 17);
        assert_eq!(output.cls_output.rows, 1);
    }
}

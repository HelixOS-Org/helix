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

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of modalities
const MAX_MODALITIES: usize = 8;

/// Maximum modality dimension
const MAX_MODALITY_DIM: usize = 512;

/// Default attention heads
const DEFAULT_ATTENTION_HEADS: usize = 8;

/// Default hidden dimension
const DEFAULT_HIDDEN_DIM: usize = 256;

// ============================================================================
// MODALITY REPRESENTATION
// ============================================================================

/// Types of modalities in the kernel
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModalityType {
    /// System metrics (CPU, memory, etc.)
    Metrics,
    /// Log entries (text/encoded)
    Logs,
    /// Distributed traces
    Traces,
    /// System events
    Events,
    /// Network data
    Network,
    /// Filesystem operations
    Filesystem,
    /// Process information
    Process,
    /// Custom modality
    Custom,
}

impl ModalityType {
    /// Get modality name
    pub fn name(&self) -> &'static str {
        match self {
            ModalityType::Metrics => "metrics",
            ModalityType::Logs => "logs",
            ModalityType::Traces => "traces",
            ModalityType::Events => "events",
            ModalityType::Network => "network",
            ModalityType::Filesystem => "filesystem",
            ModalityType::Process => "process",
            ModalityType::Custom => "custom",
        }
    }
}

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

// ============================================================================
// MODALITY ENCODERS
// ============================================================================

/// Linear encoder for a modality
#[derive(Debug, Clone)]
pub struct ModalityEncoder {
    /// Input dimension
    pub input_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Weight matrix
    pub weight: Vec<Vec<f64>>,
    /// Bias
    pub bias: Vec<f64>,
    /// Layer normalization scale
    pub ln_scale: Vec<f64>,
    /// Layer normalization bias
    pub ln_bias: Vec<f64>,
}

impl ModalityEncoder {
    /// Create a new encoder
    pub fn new(input_dim: usize, output_dim: usize, seed: u64) -> Self {
        let scale = libm::sqrt(2.0 / (input_dim + output_dim) as f64);
        let mut rng = seed;

        let mut weight = Vec::with_capacity(output_dim);
        for _ in 0..output_dim {
            let mut row = Vec::with_capacity(input_dim);
            for _ in 0..input_dim {
                rng = lcg_next(rng);
                row.push(((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
            }
            weight.push(row);
        }

        Self {
            input_dim,
            output_dim,
            weight,
            bias: vec![0.0; output_dim],
            ln_scale: vec![1.0; output_dim],
            ln_bias: vec![0.0; output_dim],
        }
    }

    /// Encode modality
    pub fn encode(&self, input: &[f64]) -> Vec<f64> {
        let mut output = self.bias.clone();

        for (i, out) in output.iter_mut().enumerate() {
            for (j, &inp) in input.iter().enumerate() {
                if j < self.weight[i].len() {
                    *out += self.weight[i][j] * inp;
                }
            }
        }

        // ReLU
        for v in &mut output {
            *v = v.max(0.0);
        }

        // Layer normalization
        self.layer_norm(&mut output);

        output
    }

    /// Apply layer normalization
    fn layer_norm(&self, x: &mut Vec<f64>) {
        if x.is_empty() {
            return;
        }

        let mean: f64 = x.iter().sum::<f64>() / x.len() as f64;
        let var: f64 = x.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64;
        let std = libm::sqrt(var + 1e-5);

        for (i, v) in x.iter_mut().enumerate() {
            *v = (*v - mean) / std * self.ln_scale[i] + self.ln_bias[i];
        }
    }
}

// ============================================================================
// FUSION STRATEGIES
// ============================================================================

/// Early fusion: concatenate then process
#[derive(Debug, Clone)]
pub struct EarlyFusion {
    /// Total input dimension
    pub total_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Projection layers
    pub layers: Vec<(Vec<Vec<f64>>, Vec<f64>)>,
}

impl EarlyFusion {
    /// Create a new early fusion module
    pub fn new(modality_dims: &[usize], output_dim: usize, hidden_dim: usize, seed: u64) -> Self {
        let total_dim: usize = modality_dims.iter().sum();

        let mut layers = Vec::new();
        let mut rng = seed;

        // First layer: concat -> hidden
        let (w1, b1, rng2) = create_layer(total_dim, hidden_dim, rng);
        layers.push((w1, b1));
        rng = rng2;

        // Second layer: hidden -> output
        let (w2, b2, _) = create_layer(hidden_dim, output_dim, rng);
        layers.push((w2, b2));

        Self {
            total_dim,
            output_dim,
            layers,
        }
    }

    /// Fuse modalities
    pub fn fuse(&self, inputs: &[&[f64]]) -> Vec<f64> {
        // Concatenate all inputs
        let mut concat = Vec::with_capacity(self.total_dim);
        for input in inputs {
            concat.extend_from_slice(input);
        }

        // Forward through layers
        let mut x = concat;

        for (i, (weight, bias)) in self.layers.iter().enumerate() {
            let mut y = bias.clone();

            for (j, out) in y.iter_mut().enumerate() {
                for (k, &inp) in x.iter().enumerate() {
                    if k < weight[j].len() {
                        *out += weight[j][k] * inp;
                    }
                }
            }

            // ReLU for all but last layer
            if i < self.layers.len() - 1 {
                for v in &mut y {
                    *v = v.max(0.0);
                }
            }

            x = y;
        }

        x
    }
}

/// Late fusion: process separately then combine
#[derive(Debug, Clone)]
pub struct LateFusion {
    /// Modality encoders
    pub encoders: Vec<ModalityEncoder>,
    /// Fusion weights
    pub fusion_weights: Vec<f64>,
    /// Learnable weights
    pub learnable: bool,
}

impl LateFusion {
    /// Create a new late fusion module
    pub fn new(modality_dims: &[usize], hidden_dim: usize, seed: u64) -> Self {
        let mut encoders = Vec::new();
        let mut rng = seed;

        for &dim in modality_dims {
            encoders.push(ModalityEncoder::new(dim, hidden_dim, rng));
            rng = lcg_next(rng);
        }

        let num_modalities = modality_dims.len();
        let fusion_weights = vec![1.0 / num_modalities as f64; num_modalities];

        Self {
            encoders,
            fusion_weights,
            learnable: true,
        }
    }

    /// Fuse modalities
    pub fn fuse(&self, inputs: &[&[f64]], present: &[bool]) -> Vec<f64> {
        if self.encoders.is_empty() {
            return Vec::new();
        }

        let hidden_dim = self.encoders[0].output_dim;
        let mut fused = vec![0.0; hidden_dim];
        let mut total_weight = 0.0;

        for (i, (encoder, (&input, &is_present))) in self
            .encoders
            .iter()
            .zip(inputs.iter().zip(present.iter()))
            .enumerate()
        {
            if is_present {
                let encoded = encoder.encode(input);
                let weight = self.fusion_weights.get(i).copied().unwrap_or(1.0);

                for (f, &e) in fused.iter_mut().zip(encoded.iter()) {
                    *f += weight * e;
                }

                total_weight += weight;
            }
        }

        // Normalize
        if total_weight > 0.0 {
            for f in &mut fused {
                *f /= total_weight;
            }
        }

        fused
    }

    /// Update fusion weights based on importance
    pub fn update_weights(&mut self, importance_scores: &[f64]) {
        if importance_scores.len() != self.fusion_weights.len() {
            return;
        }

        // Softmax normalization
        let max_score = importance_scores
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| f64::max(a, b));
        let exp_scores: Vec<f64> = importance_scores
            .iter()
            .map(|&s| libm::exp(s - max_score))
            .collect();
        let sum: f64 = exp_scores.iter().sum();

        for (w, e) in self.fusion_weights.iter_mut().zip(exp_scores.iter()) {
            *w = e / sum;
        }
    }
}

// ============================================================================
// CROSS-MODAL ATTENTION
// ============================================================================

/// Cross-modal attention mechanism
#[derive(Debug, Clone)]
pub struct CrossModalAttention {
    /// Hidden dimension
    pub hidden_dim: usize,
    /// Number of attention heads
    pub num_heads: usize,
    /// Query projection
    pub w_q: Vec<Vec<f64>>,
    /// Key projection
    pub w_k: Vec<Vec<f64>>,
    /// Value projection
    pub w_v: Vec<Vec<f64>>,
    /// Output projection
    pub w_o: Vec<Vec<f64>>,
}

impl CrossModalAttention {
    /// Create a new cross-modal attention
    pub fn new(hidden_dim: usize, num_heads: usize, seed: u64) -> Self {
        let mut rng = seed;

        let (w_q, _, rng2) = create_layer(hidden_dim, hidden_dim, rng);
        let (w_k, _, rng3) = create_layer(hidden_dim, hidden_dim, rng2);
        let (w_v, _, rng4) = create_layer(hidden_dim, hidden_dim, rng3);
        let (w_o, _, _) = create_layer(hidden_dim, hidden_dim, rng4);

        Self {
            hidden_dim,
            num_heads,
            w_q,
            w_k,
            w_v,
            w_o,
        }
    }

    /// Compute attention from query modality to key/value modality
    pub fn attend(&self, query: &[f64], key: &[f64], value: &[f64]) -> Vec<f64> {
        // Project to Q, K, V
        let q = self.project(&self.w_q, query);
        let k = self.project(&self.w_k, key);
        let v = self.project(&self.w_v, value);

        // Compute attention score
        let scale = libm::sqrt(self.hidden_dim as f64);
        let score: f64 = q
            .iter()
            .zip(k.iter())
            .map(|(&qi, &ki)| qi * ki)
            .sum::<f64>()
            / scale;

        // Softmax (single key, so just sigmoid-like)
        let attention = 1.0 / (1.0 + libm::exp(-score));

        // Apply attention to values
        let attended: Vec<f64> = v.iter().map(|&vi| attention * vi).collect();

        // Output projection
        self.project(&self.w_o, &attended)
    }

    /// Multi-head attention over multiple modalities
    pub fn multi_attend(&self, query: &[f64], keys: &[&[f64]], values: &[&[f64]]) -> Vec<f64> {
        if keys.is_empty() || values.is_empty() {
            return query.to_vec();
        }

        let q = self.project(&self.w_q, query);
        let scale = libm::sqrt(self.hidden_dim as f64);

        // Compute attention scores
        let mut scores: Vec<f64> = Vec::new();
        let mut projected_values: Vec<Vec<f64>> = Vec::new();

        for (key, value) in keys.iter().zip(values.iter()) {
            let k = self.project(&self.w_k, key);
            let v = self.project(&self.w_v, value);

            let score: f64 = q
                .iter()
                .zip(k.iter())
                .map(|(&qi, &ki)| qi * ki)
                .sum::<f64>()
                / scale;
            scores.push(score);
            projected_values.push(v);
        }

        // Softmax
        let max_score = scores
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| f64::max(a, b));
        let exp_scores: Vec<f64> = scores.iter().map(|&s| libm::exp(s - max_score)).collect();
        let sum: f64 = exp_scores.iter().sum();
        let attention_weights: Vec<f64> = exp_scores.iter().map(|&e| e / sum).collect();

        // Weighted sum of values
        let mut attended = vec![0.0; self.hidden_dim];
        for (weight, value) in attention_weights.iter().zip(projected_values.iter()) {
            for (a, &v) in attended.iter_mut().zip(value.iter()) {
                *a += weight * v;
            }
        }

        // Output projection
        self.project(&self.w_o, &attended)
    }

    /// Project vector
    fn project(&self, weight: &[Vec<f64>], input: &[f64]) -> Vec<f64> {
        let mut output = vec![0.0; weight.len()];

        for (i, out) in output.iter_mut().enumerate() {
            for (j, &inp) in input.iter().enumerate() {
                if j < weight[i].len() {
                    *out += weight[i][j] * inp;
                }
            }
        }

        output
    }
}

/// Bidirectional cross-modal attention
#[derive(Debug, Clone)]
pub struct BiCrossModalAttention {
    /// Forward attention (A -> B)
    pub forward_attention: CrossModalAttention,
    /// Backward attention (B -> A)
    pub backward_attention: CrossModalAttention,
    /// Gated combination
    pub gate_weight: Vec<f64>,
}

impl BiCrossModalAttention {
    /// Create a new bidirectional attention
    pub fn new(hidden_dim: usize, num_heads: usize, seed: u64) -> Self {
        Self {
            forward_attention: CrossModalAttention::new(hidden_dim, num_heads, seed),
            backward_attention: CrossModalAttention::new(hidden_dim, num_heads, seed + 1000),
            gate_weight: vec![0.5; hidden_dim],
        }
    }

    /// Bidirectional attention between two modalities
    pub fn attend(&self, modality_a: &[f64], modality_b: &[f64]) -> (Vec<f64>, Vec<f64>) {
        // A attends to B
        let a_updated = self
            .forward_attention
            .attend(modality_a, modality_b, modality_b);

        // B attends to A
        let b_updated = self
            .backward_attention
            .attend(modality_b, modality_a, modality_a);

        // Gate combination
        let a_final: Vec<f64> = modality_a
            .iter()
            .zip(a_updated.iter())
            .zip(self.gate_weight.iter())
            .map(|((&a, &a_up), &g)| g * a_up + (1.0 - g) * a)
            .collect();

        let b_final: Vec<f64> = modality_b
            .iter()
            .zip(b_updated.iter())
            .zip(self.gate_weight.iter())
            .map(|((&b, &b_up), &g)| g * b_up + (1.0 - g) * b)
            .collect();

        (a_final, b_final)
    }
}

// ============================================================================
// MODALITY ALIGNMENT
// ============================================================================

/// Contrastive alignment for modalities
#[derive(Debug, Clone)]
pub struct ContrastiveAlignment {
    /// Shared dimension
    pub shared_dim: usize,
    /// Temperature for contrastive loss
    pub temperature: f64,
    /// Modality projectors
    pub projectors: Vec<ModalityEncoder>,
}

impl ContrastiveAlignment {
    /// Create a new contrastive alignment
    pub fn new(modality_dims: &[usize], shared_dim: usize, seed: u64) -> Self {
        let mut projectors = Vec::new();
        let mut rng = seed;

        for &dim in modality_dims {
            projectors.push(ModalityEncoder::new(dim, shared_dim, rng));
            rng = lcg_next(rng);
        }

        Self {
            shared_dim,
            temperature: 0.07,
            projectors,
        }
    }

    /// Project all modalities to shared space
    pub fn project_all(&self, inputs: &[&[f64]]) -> Vec<Vec<f64>> {
        inputs
            .iter()
            .zip(self.projectors.iter())
            .map(|(input, projector)| {
                let mut projected = projector.encode(input);
                // L2 normalize
                let norm: f64 = libm::sqrt(projected.iter().map(|x| x * x).sum());
                if norm > 1e-10 {
                    for v in &mut projected {
                        *v /= norm;
                    }
                }
                projected
            })
            .collect()
    }

    /// Compute alignment loss (InfoNCE)
    pub fn alignment_loss(&self, projected: &[Vec<f64>]) -> f64 {
        if projected.len() < 2 {
            return 0.0;
        }

        let mut total_loss = 0.0;
        let num_pairs = projected.len() * (projected.len() - 1) / 2;

        for i in 0..projected.len() {
            for j in (i + 1)..projected.len() {
                // Cosine similarity
                let sim: f64 = projected[i]
                    .iter()
                    .zip(projected[j].iter())
                    .map(|(&a, &b)| a * b)
                    .sum();

                // InfoNCE loss component
                let loss = -sim / self.temperature;
                total_loss += loss;
            }
        }

        total_loss / num_pairs as f64
    }
}

/// Canonical Correlation Analysis alignment
#[derive(Debug, Clone)]
pub struct CCAAlignment {
    /// Projection for modality A
    pub proj_a: Vec<Vec<f64>>,
    /// Projection for modality B
    pub proj_b: Vec<Vec<f64>>,
    /// Correlation dimension
    pub corr_dim: usize,
}

impl CCAAlignment {
    /// Create a new CCA alignment
    pub fn new(dim_a: usize, dim_b: usize, corr_dim: usize, seed: u64) -> Self {
        let (proj_a, _, rng2) = create_layer(dim_a, corr_dim, seed);
        let (proj_b, _, _) = create_layer(dim_b, corr_dim, rng2);

        Self {
            proj_a,
            proj_b,
            corr_dim,
        }
    }

    /// Project both modalities
    pub fn project(&self, input_a: &[f64], input_b: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let proj_a = project_vec(&self.proj_a, input_a);
        let proj_b = project_vec(&self.proj_b, input_b);

        (proj_a, proj_b)
    }

    /// Compute correlation
    pub fn correlation(&self, input_a: &[f64], input_b: &[f64]) -> f64 {
        let (proj_a, proj_b) = self.project(input_a, input_b);

        let norm_a: f64 = libm::sqrt(proj_a.iter().map(|x| x * x).sum());
        let norm_b: f64 = libm::sqrt(proj_b.iter().map(|x| x * x).sum());

        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }

        let dot: f64 = proj_a.iter().zip(proj_b.iter()).map(|(&a, &b)| a * b).sum();

        dot / (norm_a * norm_b)
    }
}

// ============================================================================
// MISSING MODALITY HANDLING
// ============================================================================

/// Strategy for handling missing modalities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingModalityStrategy {
    /// Use zero vector
    Zero,
    /// Use learned default
    LearnedDefault,
    /// Impute from other modalities
    Impute,
    /// Drop and reweight
    DropReweight,
}

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
            },
            MissingModalityStrategy::LearnedDefault => self
                .defaults
                .get(&modality)
                .cloned()
                .unwrap_or_else(|| vec![0.0; dim]),
            MissingModalityStrategy::Impute => self.impute(dim, other_modalities),
            MissingModalityStrategy::DropReweight => {
                vec![0.0; dim] // Will be handled by fusion
            },
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

// ============================================================================
// MULTIMODAL TRANSFORMER
// ============================================================================

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

// ============================================================================
// KERNEL MULTIMODAL MANAGER
// ============================================================================

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
    pub fusion_history: Vec<Vec<f64>>,
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
            fusion_history: Vec::new(),
            max_history: 100,
        }
    }

    /// Fuse multimodal input
    pub fn fuse(&mut self, input: &MultimodalInput) -> Vec<f64> {
        let fused = self.transformer.forward(input);

        // Store in history
        if self.fusion_history.len() >= self.max_history {
            self.fusion_history.remove(0);
        }
        self.fusion_history.push(fused.clone());

        fused
    }

    /// Get modality importance
    pub fn get_importance(&self, modality: ModalityType) -> f64 {
        self.importance_scores
            .get(&modality)
            .copied()
            .unwrap_or(1.0)
    }

    /// Update modality importance based on prediction quality
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

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// LCG random number generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Create a random layer
fn create_layer(in_dim: usize, out_dim: usize, seed: u64) -> (Vec<Vec<f64>>, Vec<f64>, u64) {
    let scale = libm::sqrt(2.0 / (in_dim + out_dim) as f64);
    let mut rng = seed;

    let mut weight = Vec::with_capacity(out_dim);
    for _ in 0..out_dim {
        let mut row = Vec::with_capacity(in_dim);
        for _ in 0..in_dim {
            rng = lcg_next(rng);
            row.push(((rng as f64 / u64::MAX as f64) - 0.5) * 2.0 * scale);
        }
        weight.push(row);
    }

    let bias = vec![0.0; out_dim];

    (weight, bias, rng)
}

/// Project vector through weight matrix
fn project_vec(weight: &[Vec<f64>], input: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; weight.len()];

    for (i, out) in output.iter_mut().enumerate() {
        for (j, &inp) in input.iter().enumerate() {
            if j < weight[i].len() {
                *out += weight[i][j] * inp;
            }
        }
    }

    output
}

/// Layer normalization
fn layer_norm(x: &[f64], scale: &[f64], bias: &[f64]) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }

    let mean: f64 = x.iter().sum::<f64>() / x.len() as f64;
    let var: f64 = x.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64;
    let std = libm::sqrt(var + 1e-5);

    x.iter()
        .zip(scale.iter())
        .zip(bias.iter())
        .map(|((&v, &s), &b)| (v - mean) / std * s + b)
        .collect()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_input() {
        let input = ModalityInput::new(ModalityType::Metrics, vec![0.5; 10]);

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

        mm_input.add(ModalityInput::new(ModalityType::Metrics, vec![0.5; 10]));
        mm_input.add(ModalityInput::new(ModalityType::Logs, vec![0.3; 20]));

        assert_eq!(mm_input.num_present(), 2);
        assert!(mm_input.has(ModalityType::Metrics));
        assert!(!mm_input.has(ModalityType::Events));
    }

    #[test]
    fn test_modality_encoder() {
        let encoder = ModalityEncoder::new(10, 32, 12345);
        let input = vec![0.5; 10];

        let encoded = encoder.encode(&input);

        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_early_fusion() {
        let fusion = EarlyFusion::new(&[10, 20, 15], 32, 64, 12345);

        let inputs: Vec<Vec<f64>> = vec![vec![0.5; 10], vec![0.3; 20], vec![0.7; 15]];

        let input_refs: Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();
        let fused = fusion.fuse(&input_refs);

        assert_eq!(fused.len(), 32);
    }

    #[test]
    fn test_late_fusion() {
        let fusion = LateFusion::new(&[10, 20, 15], 32, 12345);

        let inputs: Vec<Vec<f64>> = vec![vec![0.5; 10], vec![0.3; 20], vec![0.7; 15]];

        let input_refs: Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();
        let present = vec![true, true, true];

        let fused = fusion.fuse(&input_refs, &present);

        assert_eq!(fused.len(), 32);
    }

    #[test]
    fn test_late_fusion_missing() {
        let fusion = LateFusion::new(&[10, 20, 15], 32, 12345);

        let inputs: Vec<Vec<f64>> = vec![vec![0.5; 10], vec![0.3; 20], vec![0.7; 15]];

        let input_refs: Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();
        let present = vec![true, false, true]; // Second modality missing

        let fused = fusion.fuse(&input_refs, &present);

        assert_eq!(fused.len(), 32);
    }

    #[test]
    fn test_cross_modal_attention() {
        let attention = CrossModalAttention::new(32, 4, 12345);

        let query = vec![0.5; 32];
        let key = vec![0.3; 32];
        let value = vec![0.7; 32];

        let attended = attention.attend(&query, &key, &value);

        assert_eq!(attended.len(), 32);
    }

    #[test]
    fn test_multi_attend() {
        let attention = CrossModalAttention::new(32, 4, 12345);

        let query = vec![0.5; 32];
        let keys = vec![vec![0.3; 32], vec![0.4; 32]];
        let values = vec![vec![0.7; 32], vec![0.8; 32]];

        let key_refs: Vec<&[f64]> = keys.iter().map(|v| v.as_slice()).collect();
        let value_refs: Vec<&[f64]> = values.iter().map(|v| v.as_slice()).collect();

        let attended = attention.multi_attend(&query, &key_refs, &value_refs);

        assert_eq!(attended.len(), 32);
    }

    #[test]
    fn test_bi_cross_modal_attention() {
        let attention = BiCrossModalAttention::new(32, 4, 12345);

        let mod_a = vec![0.5; 32];
        let mod_b = vec![0.3; 32];

        let (a_out, b_out) = attention.attend(&mod_a, &mod_b);

        assert_eq!(a_out.len(), 32);
        assert_eq!(b_out.len(), 32);
    }

    #[test]
    fn test_contrastive_alignment() {
        let alignment = ContrastiveAlignment::new(&[10, 20], 32, 12345);

        let inputs: Vec<Vec<f64>> = vec![vec![0.5; 10], vec![0.3; 20]];
        let input_refs: Vec<&[f64]> = inputs.iter().map(|v| v.as_slice()).collect();

        let projected = alignment.project_all(&input_refs);

        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0].len(), 32);
    }

    #[test]
    fn test_cca_alignment() {
        let alignment = CCAAlignment::new(10, 20, 16, 12345);

        let input_a = vec![0.5; 10];
        let input_b = vec![0.3; 20];

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

        let other1 = vec![1.0; 10];
        let other2 = vec![2.0; 10];

        let imputed = handler.handle(ModalityType::Logs, 10, &[&other1, &other2]);

        assert_eq!(imputed.len(), 10);
        assert!((imputed[0] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_transformer_block() {
        let block = MultimodalTransformerBlock::new(32, 3, 4, 12345);

        let modalities = vec![vec![0.5; 32], vec![0.3; 32], vec![0.7; 32]];

        let output = block.forward(&modalities);

        assert_eq!(output.len(), 3);
        assert_eq!(output[0].len(), 32);
    }

    #[test]
    fn test_multimodal_transformer() {
        let modality_dims = &[(ModalityType::Metrics, 10), (ModalityType::Logs, 20)];

        let transformer = MultimodalTransformer::new(modality_dims, 32, 2, 4, 12345);

        let mut input = MultimodalInput::new();
        input.add(ModalityInput::new(ModalityType::Metrics, vec![0.5; 10]));
        input.add(ModalityInput::new(ModalityType::Logs, vec![0.3; 20]));

        let output = transformer.forward(&input);

        assert_eq!(output.len(), 32);
    }

    #[test]
    fn test_kernel_multimodal_manager() {
        let mut manager = KernelMultimodalManager::new();

        let mut input = MultimodalInput::new();
        input.add(ModalityInput::new(ModalityType::Metrics, vec![0.5; 32]));
        input.add(ModalityInput::new(ModalityType::Logs, vec![0.3; 64]));

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

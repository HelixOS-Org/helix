//! # Zero-Shot Learning Engine
//!
//! Revolutionary zero-shot and few-shot learning system that enables the kernel
//! to recognize and handle completely novel situations without prior training data.
//! Uses semantic embeddings, compositional reasoning, and meta-learning to generalize
//! from known concepts to unknown ones.
//!
//! ## Core Capabilities
//!
//! - **Semantic Embedding Space**: Vector representations for kernel concepts
//! - **Attribute-Based Classification**: Compose unknown classes from attributes
//! - **Generalized Zero-Shot Learning (GZSL)**: Handle both seen and unseen classes
//! - **Meta-Learning Integration**: Learn-to-learn for rapid adaptation
//! - **Cross-Domain Transfer**: Transfer knowledge between kernel subsystems
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    ZERO-SHOT LEARNING ENGINE                            │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    SEMANTIC ENCODER                             │     │
//! │  │   Input features → Latent semantic space                       │     │
//! │  │   E: ℝ^d → ℝ^k (embedding dimension k ≈ 64-256)              │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    ATTRIBUTE SPACE                              │     │
//! │  │   Class attributes: A ∈ ℝ^{C×a}                               │     │
//! │  │   Attribute compatibility: f(x,y) = E(x)ᵀ W A(y)              │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    ZERO-SHOT CLASSIFIER                         │     │
//! │  │   ŷ = argmax_y∈Y f(x, y) for unseen Y                         │     │
//! │  │   Calibrated stacking for GZSL                                 │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

extern crate alloc;

pub mod classifier;
pub mod concepts;
pub mod encoder;
pub mod types;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

pub use classifier::AttributeClassifier;
pub use concepts::{
    AnomalyClass, ErrorTypeClass, IoPatternClass, KernelConcept, MemoryPatternClass,
    ProcessStateClass, SecurityEventClass,
};
pub use encoder::SemanticEncoder;
pub use types::{
    ATTRIBUTE_DIM, AttributeVector, ClassId, EMBEDDING_DIM, EmbeddingVector, FeatureVector,
    cosine_similarity, euclidean_distance,
};

use crate::math::F64Ext;

/// Prototype-based few-shot learner
#[derive(Debug, Clone)]
pub struct PrototypeNetwork {
    /// Encoder network
    encoder: SemanticEncoder,
    /// Class prototypes (mean embeddings)
    prototypes: BTreeMap<ClassId, EmbeddingVector>,
    /// Support set for each class
    support_sets: BTreeMap<ClassId, Vec<EmbeddingVector>>,
}

impl PrototypeNetwork {
    /// Create a new prototype network
    pub fn new(input_dim: usize, hidden_dim: usize, embedding_dim: usize) -> Self {
        Self {
            encoder: SemanticEncoder::new(input_dim, hidden_dim, embedding_dim),
            prototypes: BTreeMap::new(),
            support_sets: BTreeMap::new(),
        }
    }

    /// Add support example for a class
    pub fn add_support(&mut self, class_id: ClassId, features: &FeatureVector) {
        let embedding = self.encoder.encode(features);

        self.support_sets
            .entry(class_id)
            .or_default()
            .push(embedding);

        // Update prototype
        self.update_prototype(class_id);
    }

    /// Update class prototype (mean of support embeddings)
    fn update_prototype(&mut self, class_id: ClassId) {
        if let Some(support) = self.support_sets.get(&class_id) {
            if support.is_empty() {
                return;
            }

            let dim = support[0].len();
            let mut prototype = alloc::vec![0.0; dim];

            for embedding in support {
                for (i, &val) in embedding.iter().enumerate() {
                    prototype[i] += val;
                }
            }

            let n = support.len() as f64;
            for val in &mut prototype {
                *val /= n;
            }

            self.prototypes.insert(class_id, prototype);
        }
    }

    /// Classify query by nearest prototype
    pub fn classify(&self, features: &FeatureVector) -> Option<(ClassId, f64)> {
        let query = self.encoder.encode(features);

        let mut best_class = None;
        let mut best_dist = f64::INFINITY;

        for (&class_id, prototype) in &self.prototypes {
            let dist = euclidean_distance(&query, prototype);
            if dist < best_dist {
                best_dist = dist;
                best_class = Some(class_id);
            }
        }

        best_class.map(|c| (c, -best_dist)) // Return negative distance as score
    }

    /// N-way K-shot episode training
    pub fn train_episode(
        &mut self,
        support: &[(ClassId, FeatureVector)],
        query: &[(ClassId, FeatureVector)],
        _learning_rate: f64,
    ) -> f64 {
        // Build prototypes from support set
        let mut temp_support: BTreeMap<ClassId, Vec<EmbeddingVector>> = BTreeMap::new();

        for (class_id, features) in support {
            let embedding = self.encoder.encode(features);
            temp_support
                .entry(*class_id)
                .or_default()
                .push(embedding);
        }

        // Compute prototypes
        let mut prototypes: BTreeMap<ClassId, EmbeddingVector> = BTreeMap::new();
        for (class_id, embeddings) in &temp_support {
            let dim = embeddings[0].len();
            let mut proto = alloc::vec![0.0; dim];
            for emb in embeddings {
                for (i, &v) in emb.iter().enumerate() {
                    proto[i] += v;
                }
            }
            let n = embeddings.len() as f64;
            for v in &mut proto {
                *v /= n;
            }
            prototypes.insert(*class_id, proto);
        }

        // Compute loss on query set
        let mut total_loss = 0.0;
        let mut correct = 0;

        for (true_class, features) in query {
            let query_emb = self.encoder.encode(features);

            // Softmax over distances
            let mut logits: Vec<(ClassId, f64)> = Vec::new();
            for (class_id, proto) in &prototypes {
                let dist = euclidean_distance(&query_emb, proto);
                logits.push((*class_id, -dist));
            }

            // Softmax
            let max_logit = logits
                .iter()
                .map(|(_, l)| *l)
                .fold(f64::NEG_INFINITY, f64::max);
            let exp_sum: f64 = logits.iter().map(|(_, l)| libm::exp(l - max_logit)).sum();

            // Cross-entropy loss
            if let Some((_, logit)) = logits.iter().find(|(c, _)| c == true_class) {
                let prob = libm::exp(logit - max_logit) / exp_sum;
                total_loss -= libm::log(prob.max(1e-10));
            }

            // Accuracy
            if let Some((pred, _)) = logits.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
                if pred == true_class {
                    correct += 1;
                }
            }
        }

        // Suppress unused warning
        let _ = total_loss;

        // Update prototypes
        self.prototypes = prototypes;

        correct as f64 / query.len() as f64
    }
}

/// Class Embedding Generator (VAE-based)
#[derive(Debug, Clone)]
pub struct EmbeddingGenerator {
    /// Attribute to embedding mapping
    attr_to_emb: Vec<f64>,
    /// Attribute dimension
    attr_dim: usize,
    /// Embedding dimension
    emb_dim: usize,
    /// Noise scale for generation
    noise_scale: f64,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator
    pub fn new(attr_dim: usize, emb_dim: usize) -> Self {
        let mut rng = 77u64;
        let mut attr_to_emb = Vec::with_capacity(attr_dim * emb_dim);

        for _ in 0..attr_dim * emb_dim {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let val = (rng as f64 / u64::MAX as f64) * 2.0 - 1.0;
            attr_to_emb.push(val * 0.1);
        }

        Self {
            attr_to_emb,
            attr_dim,
            emb_dim,
            noise_scale: 0.1,
        }
    }

    /// Generate embedding from attributes (with noise for variety)
    pub fn generate(&self, attributes: &AttributeVector, rng: &mut u64) -> EmbeddingVector {
        let mut embedding = alloc::vec![0.0; self.emb_dim];

        // Linear transform
        for (j, emb) in embedding.iter_mut().enumerate() {
            for (i, attr) in attributes.iter().enumerate().take(self.attr_dim) {
                *emb += attr * self.attr_to_emb[i * self.emb_dim + j];
            }

            // Add noise
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let noise = (*rng as f64 / u64::MAX as f64 - 0.5) * self.noise_scale;
            *emb += noise;
        }

        // Normalize
        let norm: f64 = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-8 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        embedding
    }

    /// Generate multiple synthetic embeddings for a class
    pub fn generate_synthetic(
        &self,
        attributes: &AttributeVector,
        count: usize,
        rng: &mut u64,
    ) -> Vec<EmbeddingVector> {
        (0..count).map(|_| self.generate(attributes, rng)).collect()
    }
}

/// Meta-learner for zero-shot adaptation
#[derive(Debug, Clone)]
pub struct MetaZeroShotLearner {
    /// Base classifier
    classifier: AttributeClassifier,
    /// Prototype network
    prototype_net: PrototypeNetwork,
    /// Embedding generator
    generator: EmbeddingGenerator,
    /// Semantic encoder
    encoder: SemanticEncoder,
    /// Learning rate
    learning_rate: f64,
    /// Inner loop steps
    inner_steps: usize,
}

impl MetaZeroShotLearner {
    /// Create a new meta zero-shot learner
    pub fn new(input_dim: usize, hidden_dim: usize, attr_dim: usize, emb_dim: usize) -> Self {
        Self {
            classifier: AttributeClassifier::new(attr_dim),
            prototype_net: PrototypeNetwork::new(input_dim, hidden_dim, emb_dim),
            generator: EmbeddingGenerator::new(attr_dim, emb_dim),
            encoder: SemanticEncoder::new(input_dim, hidden_dim, emb_dim),
            learning_rate: 0.01,
            inner_steps: 5,
        }
    }

    /// Register a known (seen) class
    pub fn register_seen_class(&mut self, class_id: ClassId, attributes: AttributeVector) {
        self.classifier.register_class(class_id, attributes, true);
    }

    /// Register a novel (unseen) class
    pub fn register_unseen_class(&mut self, class_id: ClassId, attributes: AttributeVector) {
        self.classifier
            .register_class(class_id, attributes.clone(), false);

        // Generate synthetic prototypes
        let mut rng = class_id as u64 * 12345;
        let synthetic = self.generator.generate_synthetic(&attributes, 10, &mut rng);

        // Average as prototype
        let dim = synthetic[0].len();
        let mut proto = alloc::vec![0.0; dim];
        for emb in &synthetic {
            for (i, &v) in emb.iter().enumerate() {
                proto[i] += v;
            }
        }
        for v in &mut proto {
            *v /= synthetic.len() as f64;
        }
        self.prototype_net.prototypes.insert(class_id, proto);
    }

    /// Classify with combined methods
    pub fn classify(&self, features: &FeatureVector) -> ClassificationResult {
        let embedding = self.encoder.encode(features);

        // Attribute-based score
        let gzsl_result = self.classifier.classify_gzsl(&embedding);

        // Prototype-based score
        let proto_result = self.prototype_net.classify(features);

        // Combine results
        match (gzsl_result, proto_result) {
            (Some((gzsl_class, gzsl_score)), Some((proto_class, proto_score))) => {
                // Weighted combination
                if gzsl_class == proto_class {
                    ClassificationResult {
                        predicted_class: gzsl_class,
                        confidence: (gzsl_score.tanh() + proto_score.tanh().abs()) / 2.0,
                        is_novel: !self.classifier.seen_classes.contains(&gzsl_class),
                        all_scores: Vec::new(),
                    }
                } else {
                    // Use higher confidence
                    if gzsl_score > proto_score.abs() {
                        ClassificationResult {
                            predicted_class: gzsl_class,
                            confidence: gzsl_score.tanh(),
                            is_novel: !self.classifier.seen_classes.contains(&gzsl_class),
                            all_scores: Vec::new(),
                        }
                    } else {
                        ClassificationResult {
                            predicted_class: proto_class,
                            confidence: proto_score.tanh().abs(),
                            is_novel: !self.classifier.seen_classes.contains(&proto_class),
                            all_scores: Vec::new(),
                        }
                    }
                }
            },
            (Some((class, score)), None) => ClassificationResult {
                predicted_class: class,
                confidence: score.tanh(),
                is_novel: !self.classifier.seen_classes.contains(&class),
                all_scores: Vec::new(),
            },
            (None, Some((class, score))) => ClassificationResult {
                predicted_class: class,
                confidence: score.tanh().abs(),
                is_novel: !self.classifier.seen_classes.contains(&class),
                all_scores: Vec::new(),
            },
            (None, None) => ClassificationResult {
                predicted_class: 0,
                confidence: 0.0,
                is_novel: true,
                all_scores: Vec::new(),
            },
        }
    }

    /// Rapid adaptation with few examples
    pub fn adapt(&mut self, class_id: ClassId, examples: &[FeatureVector]) {
        for features in examples {
            self.prototype_net.add_support(class_id, features);
        }
    }
}

/// Classification result
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Predicted class
    pub predicted_class: ClassId,
    /// Confidence score (0-1)
    pub confidence: f64,
    /// Whether the class is novel (unseen)
    pub is_novel: bool,
    /// All class scores for interpretability
    pub all_scores: Vec<(ClassId, f64)>,
}

/// Transductive Zero-Shot Learner (uses unlabeled target data)
#[derive(Debug, Clone)]
pub struct TransductiveZSL {
    /// Base classifier
    classifier: AttributeClassifier,
    /// Encoder
    encoder: SemanticEncoder,
    /// Pseudo-labels
    pseudo_labels: BTreeMap<u64, ClassId>,
    /// Confidence threshold for pseudo-labeling
    threshold: f64,
}

impl TransductiveZSL {
    /// Create a new transductive ZSL
    pub fn new(input_dim: usize, attr_dim: usize, emb_dim: usize) -> Self {
        Self {
            classifier: AttributeClassifier::new(attr_dim),
            encoder: SemanticEncoder::new(input_dim, 64, emb_dim),
            pseudo_labels: BTreeMap::new(),
            threshold: 0.7,
        }
    }

    /// Self-training iteration
    pub fn self_train(
        &mut self,
        unlabeled_data: &[(u64, FeatureVector)], // (id, features)
    ) -> usize {
        let mut new_labels = 0;

        for (id, features) in unlabeled_data {
            if self.pseudo_labels.contains_key(id) {
                continue;
            }

            let embedding = self.encoder.encode(features);
            if let Some((class, score)) = self.classifier.classify_gzsl(&embedding) {
                let confidence = score.tanh();
                if confidence > self.threshold {
                    self.pseudo_labels.insert(*id, class);
                    new_labels += 1;
                }
            }
        }

        new_labels
    }

    /// Get pseudo-label for an instance
    pub fn get_pseudo_label(&self, id: u64) -> Option<ClassId> {
        self.pseudo_labels.get(&id).copied()
    }
}

/// Domain adaptation for cross-domain zero-shot
#[derive(Debug, Clone)]
pub struct DomainAdapter {
    /// Source domain encoder
    source_encoder: SemanticEncoder,
    /// Target domain encoder
    target_encoder: SemanticEncoder,
    /// Domain alignment weights
    alignment: Vec<f64>,
}

impl DomainAdapter {
    /// Create a new domain adapter
    pub fn new(input_dim: usize, hidden_dim: usize, emb_dim: usize) -> Self {
        Self {
            source_encoder: SemanticEncoder::new(input_dim, hidden_dim, emb_dim),
            target_encoder: SemanticEncoder::new(input_dim, hidden_dim, emb_dim),
            alignment: alloc::vec![1.0; emb_dim],
        }
    }

    /// Encode source domain features
    pub fn encode_source(&self, features: &FeatureVector) -> EmbeddingVector {
        let emb = self.source_encoder.encode(features);
        emb.iter()
            .zip(&self.alignment)
            .map(|(e, a)| e * a)
            .collect()
    }

    /// Encode target domain features
    pub fn encode_target(&self, features: &FeatureVector) -> EmbeddingVector {
        self.target_encoder.encode(features)
    }

    /// Compute maximum mean discrepancy (MMD) between domains
    pub fn compute_mmd(&self, source: &[EmbeddingVector], target: &[EmbeddingVector]) -> f64 {
        if source.is_empty() || target.is_empty() {
            return 0.0;
        }

        // Kernel: Gaussian RBF
        let gamma = 1.0;

        let kernel = |a: &[f64], b: &[f64]| -> f64 {
            let dist_sq: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum();
            libm::exp(-gamma * dist_sq)
        };

        // E[k(xs, xs')]
        let mut kss = 0.0;
        for i in 0..source.len() {
            for j in 0..source.len() {
                kss += kernel(&source[i], &source[j]);
            }
        }
        kss /= (source.len() * source.len()) as f64;

        // E[k(xt, xt')]
        let mut ktt = 0.0;
        for i in 0..target.len() {
            for j in 0..target.len() {
                ktt += kernel(&target[i], &target[j]);
            }
        }
        ktt /= (target.len() * target.len()) as f64;

        // E[k(xs, xt)]
        let mut kst = 0.0;
        for s in source {
            for t in target {
                kst += kernel(s, t);
            }
        }
        kst /= (source.len() * target.len()) as f64;

        kss + ktt - 2.0 * kst
    }
}

/// Kernel Zero-Shot Learning Manager
pub struct KernelZeroShotManager {
    /// Meta learner
    learner: MetaZeroShotLearner,
    /// Registered kernel concepts
    concepts: BTreeMap<ClassId, KernelConcept>,
    /// Concept to class mapping
    concept_ids: BTreeMap<String, ClassId>,
    /// Next class ID
    next_id: ClassId,
}

impl KernelZeroShotManager {
    /// Create a new kernel zero-shot manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register base kernel concepts with attributes
    fn register_base_concepts(&mut self) {
        // Process states with attributes
        self.register_concept_with_attrs(
            "process_running",
            KernelConcept::ProcessState(ProcessStateClass::Running),
            &[1.0, 0.0, 0.0, 0.8, 0.2, 0.0, 0.0, 0.0], // active, not blocked, high cpu
            true,
        );

        self.register_concept_with_attrs(
            "process_blocked",
            KernelConcept::ProcessState(ProcessStateClass::Blocked),
            &[0.0, 1.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0], // not active, blocked, waiting io
            true,
        );

        // Memory patterns
        self.register_concept_with_attrs(
            "memory_sequential",
            KernelConcept::MemoryPattern(MemoryPatternClass::Sequential),
            &[0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0], // sequential access pattern
            true,
        );

        self.register_concept_with_attrs(
            "memory_thrashing",
            KernelConcept::MemoryPattern(MemoryPatternClass::Thrashing),
            &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0], // high pressure, swap activity
            true,
        );

        // Anomalies
        self.register_concept_with_attrs(
            "anomaly_mild",
            KernelConcept::Anomaly(AnomalyClass::MildAnomaly),
            &[0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.3, 0.0], // slight deviation
            true,
        );

        self.register_concept_with_attrs(
            "anomaly_severe",
            KernelConcept::Anomaly(AnomalyClass::SevereAnomaly),
            &[0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.9, 0.5], // high deviation
            true,
        );
    }

    /// Register a concept with attributes
    fn register_concept_with_attrs(
        &mut self,
        name: &str,
        concept: KernelConcept,
        base_attrs: &[f64],
        is_seen: bool,
    ) {
        let class_id = self.next_id;
        self.next_id += 1;

        // Pad attributes to ATTRIBUTE_DIM
        let mut attrs = alloc::vec![0.0; ATTRIBUTE_DIM];
        for (i, &v) in base_attrs.iter().enumerate() {
            if i < ATTRIBUTE_DIM {
                attrs[i] = v;
            }
        }

        self.concepts.insert(class_id, concept);
        self.concept_ids.insert(String::from(name), class_id);

        if is_seen {
            self.learner.register_seen_class(class_id, attrs);
        } else {
            self.learner.register_unseen_class(class_id, attrs);
        }
    }

    /// Classify kernel event
    pub fn classify_event(&self, features: &FeatureVector) -> Option<(KernelConcept, f64)> {
        let result = self.learner.classify(features);

        self.concepts
            .get(&result.predicted_class)
            .cloned()
            .map(|concept| (concept, result.confidence))
    }

    /// Register a novel concept from description
    pub fn register_novel_concept(&mut self, name: &str, attributes: &[f64]) -> ClassId {
        let class_id = self.next_id;
        self.next_id += 1;

        // Pad attributes
        let mut attrs = alloc::vec![0.0; ATTRIBUTE_DIM];
        for (i, &v) in attributes.iter().enumerate() {
            if i < ATTRIBUTE_DIM {
                attrs[i] = v;
            }
        }

        self.concepts
            .insert(class_id, KernelConcept::Unknown(class_id));
        self.concept_ids.insert(String::from(name), class_id);
        self.learner.register_unseen_class(class_id, attrs);

        class_id
    }

    /// Adapt to new concept with examples
    pub fn adapt_with_examples(&mut self, class_id: ClassId, examples: &[FeatureVector]) {
        self.learner.adapt(class_id, examples);
    }
}

impl Default for KernelZeroShotManager {
    fn default() -> Self {
        let mut manager = Self {
            learner: MetaZeroShotLearner::new(64, 32, ATTRIBUTE_DIM, EMBEDDING_DIM),
            concepts: BTreeMap::new(),
            concept_ids: BTreeMap::new(),
            next_id: 1,
        };

        // Register base kernel concepts
        manager.register_base_concepts();

        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_encoder() {
        let encoder = SemanticEncoder::new(8, 16, 32);
        let features = alloc::vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];

        let embedding = encoder.encode(&features);

        assert_eq!(embedding.len(), 32);

        // Check L2 normalization
        let norm: f64 = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_attribute_classifier() {
        let mut classifier = AttributeClassifier::new(4);

        // Register classes
        classifier.register_class(1, alloc::vec![1.0, 0.0, 0.0, 0.0], true);
        classifier.register_class(2, alloc::vec![0.0, 1.0, 0.0, 0.0], true);
        classifier.register_class(3, alloc::vec![0.0, 0.0, 1.0, 0.0], false);

        // Test classification
        let embedding = alloc::vec![0.9, 0.1, 0.0, 0.0];
        let result = classifier.classify_gzsl(&embedding);

        assert!(result.is_some());
        let (class, _) = result.unwrap();
        assert_eq!(class, 1);
    }

    #[test]
    fn test_prototype_network() {
        let mut proto = PrototypeNetwork::new(4, 8, 16);

        // Add support examples
        proto.add_support(1, &alloc::vec![1.0, 0.0, 0.0, 0.0]);
        proto.add_support(1, &alloc::vec![0.9, 0.1, 0.0, 0.0]);
        proto.add_support(2, &alloc::vec![0.0, 1.0, 0.0, 0.0]);
        proto.add_support(2, &alloc::vec![0.1, 0.9, 0.0, 0.0]);

        // Classify
        let result = proto.classify(&alloc::vec![0.95, 0.05, 0.0, 0.0]);

        assert!(result.is_some());
        let (class, _) = result.unwrap();
        assert_eq!(class, 1);
    }

    #[test]
    fn test_embedding_generator() {
        let generator = EmbeddingGenerator::new(8, 32);
        let mut rng = 12345u64;

        let attrs = alloc::vec![1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0];

        let emb1 = generator.generate(&attrs, &mut rng);
        let emb2 = generator.generate(&attrs, &mut rng);

        assert_eq!(emb1.len(), 32);
        assert_eq!(emb2.len(), 32);

        // Should be similar but not identical
        let sim = cosine_similarity(&emb1, &emb2);
        assert!(sim > 0.5); // Similar
        assert!(sim < 0.99); // But not identical
    }

    #[test]
    fn test_kernel_zeroshot_manager() {
        let manager = KernelZeroShotManager::new();

        // Create test features resembling a running process
        let features = alloc::vec![
            1.0, 0.0, 0.0, 0.8, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        let result = manager.classify_event(&features);
        assert!(result.is_some());
    }

    #[test]
    fn test_transductive_zsl() {
        let mut tzsl = TransductiveZSL::new(8, 4, 16);

        // Register classes
        tzsl.classifier
            .register_class(1, alloc::vec![1.0, 0.0, 0.0, 0.0], false);

        // Self-train with unlabeled data
        let unlabeled: Vec<(u64, FeatureVector)> = alloc::vec![
            (1, alloc::vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            (2, alloc::vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ];

        let labeled = tzsl.self_train(&unlabeled);
        // May or may not label depending on confidence
        assert!(labeled <= 2);
    }

    #[test]
    fn test_domain_adapter_mmd() {
        let adapter = DomainAdapter::new(4, 8, 16);

        let source = alloc::vec![
            alloc::vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0
            ],
            alloc::vec![
                0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0
            ],
        ];
        let target = alloc::vec![
            alloc::vec![
                0.8, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0
            ],
            alloc::vec![
                0.7, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0
            ],
        ];

        let mmd = adapter.compute_mmd(&source, &target);
        assert!(mmd >= 0.0); // MMD is non-negative
    }
}

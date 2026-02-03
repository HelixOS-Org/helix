//! # Learning Module for NEXUS
//!
//! Year 2 "COGNITION" - Advanced learning capabilities for kernel AI.
//!
//! ## Components
//!
//! - `reinforcement`: Q-learning, policy gradient, actor-critic
//! - `online`: Online learning, streaming updates, concept drift
//! - `meta`: Meta-learning, learning to learn, MAML-style algorithms
//! - `transfer`: Transfer learning, domain adaptation
//! - `curriculum`: Curriculum learning, progressive difficulty

#![allow(dead_code)]

pub mod curriculum;
pub mod meta;
pub mod online;
pub mod reinforcement;
pub mod transfer;

// Re-exports
pub use curriculum::{CurriculumLearner, Lesson, LessonDifficulty, TaskProgression};
pub use meta::{MAMLLearner, MetaLearner, MetaTask, TaskDistribution};
pub use online::{ConceptDriftDetector, OnlineLearner, StreamingClassifier, StreamingSample};
pub use reinforcement::{ActionSpace, Episode, PolicyGradient, QLearner, RewardSignal, StateSpace};
pub use transfer::{DomainAdapter, FeatureTransformer, KnowledgeTransfer, TransferLearner};

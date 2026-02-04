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
//! - `feedback`: Feedback loops for decision outcomes
//! - `hypothesis`: Hypothesis formation and testing
//! - `safety`: Safe exploration constraints
//! - `regression`: Performance regression detection
//! - `active`: Active learning strategies
//! - `imitation`: Learning from demonstrations
//! - `distill`: Knowledge distillation
//! - `replay`: Experience replay mechanisms

#![allow(dead_code)]

// Core learning modules
pub mod curriculum;
pub mod meta;
pub mod online;
pub mod reinforcement;
pub mod transfer;

// Additional learning modules (migrated from learn/)
pub mod active;
pub mod consolidate;
pub mod distill;
pub mod feedback;
pub mod generalize;
pub mod generalizer;
pub mod hypothesis;
pub mod imitation;
pub mod incremental;
pub mod intelligence;
pub mod meta_learn;
pub mod regression;
pub mod regress;
pub mod replay;
pub mod safety;
pub mod types;
pub mod validate;

// Re-exports from original learning modules
pub use curriculum::{CurriculumLearner, Lesson, LessonDifficulty, TaskProgression};
pub use meta::{MAMLLearner, MetaLearner, MetaTask, TaskDistribution};
pub use online::{ConceptDriftDetector, OnlineLearner, StreamingClassifier, StreamingSample};
pub use reinforcement::{ActionSpace, Episode, PolicyGradient, QLearner, RewardSignal, StateSpace};
pub use transfer::{DomainAdapter, FeatureTransformer, KnowledgeTransfer, TransferLearner};

// Re-exports from migrated learn/ modules
pub use feedback::{ActionStats, FeedbackEntry, FeedbackLoop, FeedbackType};
pub use generalizer::{
    ConditionOp, GeneralizationStrategy, Generalizer, LearnedRule, RuleCondition,
};
pub use hypothesis::{Hypothesis, HypothesisManager, HypothesisStatus};
pub use intelligence::{ContinuousLearningIntelligence, LearningAnalysis};
pub use regression::{
    MetricHistory, MetricSample, MetricType, RegressionDetector, RegressionEvent,
    RegressionSeverity,
};
pub use safety::{
    ConstraintType, ExplorationPolicy, SafeLearner, SafetyConstraint, SafetyViolation,
};
pub use types::{ExperienceId, HypothesisId, RuleId, SessionId, Timestamp};

//! # Neural Decision Engine
//!
//! The Neural Decision Engine is the brain of CORTEX. Unlike traditional AI systems
//! that use opaque machine learning models, this engine uses **transparent, deterministic,
//! and verifiable decision trees** that can explain every decision they make.
//!
//! ## Why Not Traditional ML?
//!
//! Machine learning models are:
//! - **Opaque**: You can't explain why they made a decision
//! - **Unbounded**: Inference time is unpredictable
//! - **Non-deterministic**: Same input can give different outputs
//! - **Vulnerable**: Adversarial inputs can cause misclassification
//!
//! The Neural Decision Engine is:
//! - **Transparent**: Every decision can be explained
//! - **Bounded**: Decision time has hard guarantees
//! - **Deterministic**: Same input always gives same output
//! - **Verifiable**: Decision trees can be formally verified
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │                        NEURAL DECISION ENGINE                            │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐         │
//! │  │    PATTERN     │    │   DECISION     │    │   PREDICTION   │         │
//! │  │   DETECTOR     │───▶│     TREES      │───▶│    ENGINE      │         │
//! │  └────────────────┘    └────────────────┘    └────────────────┘         │
//! │          │                     │                     │                   │
//! │          ▼                     ▼                     ▼                   │
//! │  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐         │
//! │  │    PATTERN     │    │   DECISION     │    │   ACCURACY     │         │
//! │  │    LIBRARY     │    │    EXPLAINER   │    │   TRACKER      │         │
//! │  └────────────────┘    └────────────────┘    └────────────────┘         │
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    LEARNING MODULE                              │     │
//! │  │  (Updates decision trees based on outcomes - still bounded)    │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                                                                          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::consciousness::{InvariantViolation, ViolationPrediction};
use crate::{CortexConfig, CortexEvent, CortexResult, DecisionAction, PatternId, SubsystemId};

// =============================================================================
// DECISION TYPES
// =============================================================================

/// Unique identifier for a decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecisionId(pub u64);

/// A decision made by the neural engine
#[derive(Debug, Clone)]
pub struct Decision {
    /// Unique identifier
    pub id: DecisionId,

    /// Action to take
    pub action: DecisionAction,

    /// Confidence level
    pub confidence: Confidence,

    /// Reasoning chain (explainability)
    pub reasoning: Vec<ReasoningStep>,

    /// Time taken to make decision (cycles)
    pub decision_time: u64,

    /// Decision tree used
    pub tree_id: DecisionTreeId,

    /// Node path taken in tree
    pub node_path: Vec<NodeId>,
}

/// Confidence level (0.0 - 1.0)
#[derive(Debug, Clone, Copy)]
pub struct Confidence(pub f64);

impl Confidence {
    /// Low confidence threshold
    pub const LOW_THRESHOLD: f64 = 0.3;

    /// Medium confidence threshold
    pub const MEDIUM_THRESHOLD: f64 = 0.5;

    /// High confidence threshold
    pub const HIGH_THRESHOLD: f64 = 0.8;

    /// Very high confidence threshold
    pub const VERY_HIGH_THRESHOLD: f64 = 0.95;

    /// Create new confidence
    pub fn new(value: f64) -> Self {
        Self(value.max(0.0).min(1.0))
    }

    /// Is confidence low?
    pub fn is_low(&self) -> bool {
        self.0 < Self::MEDIUM_THRESHOLD
    }

    /// Is confidence high?
    pub fn is_high(&self) -> bool {
        self.0 >= Self::HIGH_THRESHOLD
    }

    /// Is confidence very high?
    pub fn is_very_high(&self) -> bool {
        self.0 >= Self::VERY_HIGH_THRESHOLD
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self(0.5)
    }
}

/// A step in the reasoning chain
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    /// Description of this step
    pub description: String,

    /// Input to this step
    pub input: String,

    /// Output of this step
    pub output: String,

    /// Confidence contribution
    pub confidence_contribution: f64,
}

// =============================================================================
// PATTERNS
// =============================================================================

/// A detected pattern
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern identifier
    pub id: PatternId,

    /// Pattern name
    pub name: String,

    /// Pattern category
    pub category: PatternCategory,

    /// Confidence in detection
    pub confidence: Confidence,

    /// How many times this pattern was seen
    pub occurrence_count: u64,

    /// Last occurrence timestamp
    pub last_occurrence: u64,
}

/// Pattern category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCategory {
    /// Memory pressure pattern
    MemoryPressure,

    /// CPU contention pattern
    CpuContention,

    /// I/O bottleneck pattern
    IoBottleneck,

    /// Scheduling anomaly
    SchedulingAnomaly,

    /// Security anomaly
    SecurityAnomaly,

    /// Resource exhaustion
    ResourceExhaustion,

    /// Performance degradation
    PerformanceDegradation,

    /// Failure precursor
    FailurePrecursor,

    /// Attack signature
    AttackSignature,

    /// Custom pattern
    Custom,
}

/// Pattern detector
pub struct PatternDetector {
    /// Known patterns
    patterns: BTreeMap<PatternId, PatternDefinition>,

    /// Next pattern ID
    next_pattern_id: AtomicU64,

    /// Recent detections
    recent_detections: Vec<Pattern>,

    /// Detection window size
    window_size: usize,
}

/// Pattern definition
#[derive(Clone)]
pub struct PatternDefinition {
    pub id: PatternId,
    pub name: String,
    pub category: PatternCategory,
    pub matcher: PatternMatcher,
    pub min_confidence: f64,
}

/// Pattern matcher
#[derive(Clone)]
pub enum PatternMatcher {
    /// Threshold-based matching
    Threshold {
        metric: String,
        threshold: f64,
        direction: ThresholdDirection,
    },

    /// Sequence-based matching
    Sequence {
        events: Vec<EventType>,
        max_gap: u64,
    },

    /// Statistical matching
    Statistical {
        metric: String,
        z_score_threshold: f64,
    },

    /// Custom matcher
    Custom(fn(&CortexEvent) -> Option<f64>),
}

#[derive(Debug, Clone, Copy)]
pub enum ThresholdDirection {
    Above,
    Below,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    MemoryAlloc,
    MemoryFree,
    ContextSwitch,
    Interrupt,
    Syscall,
    PageFault,
    IoRequest,
    Custom(u32),
}

impl PatternDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            next_pattern_id: AtomicU64::new(1),
            recent_detections: Vec::new(),
            window_size: 1000,
        }
    }

    /// Register a pattern
    pub fn register(&mut self, mut pattern: PatternDefinition) -> PatternId {
        let id = PatternId(self.next_pattern_id.fetch_add(1, Ordering::SeqCst));
        pattern.id = id;
        self.patterns.insert(id, pattern);
        id
    }

    /// Detect patterns in event
    pub fn detect(&mut self, event: &CortexEvent, timestamp: u64) -> Option<Pattern> {
        for (id, definition) in &self.patterns {
            if let Some(confidence) = self.match_pattern(definition, event) {
                if confidence >= definition.min_confidence {
                    let pattern = Pattern {
                        id: *id,
                        name: definition.name.clone(),
                        category: definition.category,
                        confidence: Confidence::new(confidence),
                        occurrence_count: 1, // Would be tracked over time
                        last_occurrence: timestamp,
                    };

                    self.record_detection(pattern.clone());
                    return Some(pattern);
                }
            }
        }

        None
    }

    /// Match pattern against event
    fn match_pattern(&self, pattern: &PatternDefinition, event: &CortexEvent) -> Option<f64> {
        match &pattern.matcher {
            PatternMatcher::Threshold {
                threshold,
                direction,
                ..
            } => {
                // Extract value from event
                if let Some(value) = self.extract_value(event) {
                    let matches = match direction {
                        ThresholdDirection::Above => value >= *threshold,
                        ThresholdDirection::Below => value <= *threshold,
                    };

                    if matches {
                        // Confidence based on how far past threshold
                        let distance = (value - threshold).abs();
                        Some((0.5 + distance / threshold.max(1.0)).min(1.0))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },

            PatternMatcher::Statistical {
                z_score_threshold, ..
            } => {
                // Would compute z-score against historical data
                None
            },

            PatternMatcher::Sequence { .. } => {
                // Would match event sequences
                None
            },

            PatternMatcher::Custom(matcher) => matcher(event),
        }
    }

    /// Extract numeric value from event
    fn extract_value(&self, event: &CortexEvent) -> Option<f64> {
        match event {
            CortexEvent::MemoryPressure(level) => Some(*level as f64),
            CortexEvent::CpuLoad(load) => Some(*load as f64),
            CortexEvent::Latency(us) => Some(*us as f64),
            _ => None,
        }
    }

    /// Record detection for history
    fn record_detection(&mut self, pattern: Pattern) {
        if self.recent_detections.len() >= self.window_size {
            self.recent_detections.remove(0);
        }
        self.recent_detections.push(pattern);
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DECISION TREES
// =============================================================================

/// Decision tree identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecisionTreeId(pub u64);

/// Node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(pub u64);

/// A decision tree for making kernel decisions
#[derive(Clone)]
pub struct DecisionTree {
    /// Tree identifier
    pub id: DecisionTreeId,

    /// Tree name
    pub name: String,

    /// Root node
    pub root: NodeId,

    /// All nodes
    pub nodes: BTreeMap<NodeId, DecisionNode>,

    /// Maximum depth
    pub max_depth: u32,

    /// Decision count
    pub decision_count: u64,

    /// Accuracy (correct decisions / total)
    pub accuracy: f64,
}

/// A node in the decision tree
#[derive(Clone)]
pub struct DecisionNode {
    /// Node identifier
    pub id: NodeId,

    /// Node type
    pub node_type: NodeType,

    /// Depth in tree
    pub depth: u32,

    /// Times visited
    pub visit_count: u64,
}

/// Type of decision node
#[derive(Clone)]
pub enum NodeType {
    /// Decision (internal) node
    Branch {
        condition: Condition,
        true_child: NodeId,
        false_child: NodeId,
    },

    /// Leaf node with action
    Leaf {
        action: DecisionAction,
        confidence: f64,
    },

    /// Multi-way branch
    MultiBranch {
        feature: Feature,
        children: Vec<(Condition, NodeId)>,
        default: NodeId,
    },
}

/// Condition for branching
#[derive(Clone)]
pub struct Condition {
    /// Feature to evaluate
    pub feature: Feature,

    /// Operator
    pub operator: Operator,

    /// Threshold value
    pub threshold: f64,
}

/// Feature for decision making
#[derive(Clone, Debug)]
pub enum Feature {
    /// Memory usage percentage
    MemoryUsage,

    /// CPU load percentage
    CpuLoad,

    /// Interrupt latency
    InterruptLatency,

    /// Context switch rate
    ContextSwitchRate,

    /// Page fault rate
    PageFaultRate,

    /// Process count
    ProcessCount,

    /// Violation severity
    ViolationSeverity,

    /// Pattern category
    PatternCategory,

    /// Subsystem health
    SubsystemHealth,

    /// Custom feature
    Custom(String),
}

/// Comparison operator
#[derive(Clone, Debug, Copy)]
pub enum Operator {
    LessThan,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    GreaterThan,
    NotEqual,
    In,
    NotIn,
}

impl DecisionTree {
    /// Create new decision tree
    pub fn new(id: DecisionTreeId, name: &str) -> Self {
        Self {
            id,
            name: String::from(name),
            root: NodeId(0),
            nodes: BTreeMap::new(),
            max_depth: 0,
            decision_count: 0,
            accuracy: 1.0,
        }
    }

    /// Add a node
    pub fn add_node(&mut self, node: DecisionNode) {
        if node.depth > self.max_depth {
            self.max_depth = node.depth;
        }
        self.nodes.insert(node.id, node);
    }

    /// Traverse tree to make decision
    pub fn decide(&mut self, context: &DecisionContext) -> Option<Decision> {
        let mut current = self.root;
        let mut path = Vec::new();
        let mut reasoning = Vec::new();
        let start_cycles = get_cycles();

        loop {
            path.push(current);

            let node = self.nodes.get_mut(&current)?;
            node.visit_count += 1;

            match &node.node_type {
                NodeType::Leaf { action, confidence } => {
                    self.decision_count += 1;

                    return Some(Decision {
                        id: DecisionId(self.decision_count),
                        action: action.clone(),
                        confidence: Confidence::new(*confidence),
                        reasoning,
                        decision_time: get_cycles() - start_cycles,
                        tree_id: self.id,
                        node_path: path,
                    });
                },

                NodeType::Branch {
                    condition,
                    true_child,
                    false_child,
                } => {
                    let value = context.get_feature(&condition.feature);
                    let result = evaluate_condition(condition, value);

                    reasoning.push(ReasoningStep {
                        description: format!(
                            "{:?} {:?} {}",
                            condition.feature, condition.operator, condition.threshold
                        ),
                        input: format!("{}", value),
                        output: format!("{}", result),
                        confidence_contribution: 0.1,
                    });

                    current = if result { *true_child } else { *false_child };
                },

                NodeType::MultiBranch {
                    feature,
                    children,
                    default,
                } => {
                    let value = context.get_feature(feature);

                    let mut found = false;
                    for (condition, child) in children {
                        if evaluate_condition(condition, value) {
                            current = *child;
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        current = *default;
                    }
                },
            }

            // Safety limit
            if path.len() > self.max_depth as usize + 10 {
                break;
            }
        }

        None
    }
}

/// Evaluate a condition
fn evaluate_condition(condition: &Condition, value: f64) -> bool {
    match condition.operator {
        Operator::LessThan => value < condition.threshold,
        Operator::LessOrEqual => value <= condition.threshold,
        Operator::Equal => (value - condition.threshold).abs() < f64::EPSILON,
        Operator::GreaterOrEqual => value >= condition.threshold,
        Operator::GreaterThan => value > condition.threshold,
        Operator::NotEqual => (value - condition.threshold).abs() >= f64::EPSILON,
        Operator::In => false, // Would need set of values
        Operator::NotIn => true,
    }
}

/// Get CPU cycles
fn get_cycles() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

/// Context for decision making
#[derive(Default)]
pub struct DecisionContext {
    pub memory_usage: f64,
    pub cpu_load: f64,
    pub interrupt_latency: f64,
    pub context_switch_rate: f64,
    pub page_fault_rate: f64,
    pub process_count: u64,
    pub violation_severity: u32,
    pub subsystem_health: f64,
}

impl DecisionContext {
    /// Get feature value
    pub fn get_feature(&self, feature: &Feature) -> f64 {
        match feature {
            Feature::MemoryUsage => self.memory_usage,
            Feature::CpuLoad => self.cpu_load,
            Feature::InterruptLatency => self.interrupt_latency,
            Feature::ContextSwitchRate => self.context_switch_rate,
            Feature::PageFaultRate => self.page_fault_rate,
            Feature::ProcessCount => self.process_count as f64,
            Feature::ViolationSeverity => self.violation_severity as f64,
            Feature::SubsystemHealth => self.subsystem_health,
            _ => 0.0,
        }
    }
}

// =============================================================================
// PREDICTION
// =============================================================================

/// A prediction about future system state
#[derive(Debug, Clone)]
pub struct Prediction {
    /// What is being predicted
    pub target: PredictionTarget,

    /// Predicted value
    pub value: f64,

    /// Time horizon (in ticks)
    pub horizon: u64,

    /// Confidence
    pub confidence: Confidence,

    /// Reasoning
    pub reasoning: String,
}

/// What we're predicting
#[derive(Debug, Clone)]
pub enum PredictionTarget {
    /// Memory usage
    MemoryUsage,

    /// CPU load
    CpuLoad,

    /// Invariant violation probability
    ViolationProbability(crate::consciousness::InvariantId),

    /// System failure probability
    FailureProbability,

    /// Performance metric
    PerformanceMetric(String),
}

/// Prediction accuracy tracker
pub struct PredictionAccuracy {
    /// Total predictions made
    pub total: u64,

    /// Correct predictions
    pub correct: u64,

    /// Predictions by target type
    pub by_target: BTreeMap<String, (u64, u64)>,
}

impl PredictionAccuracy {
    pub fn new() -> Self {
        Self {
            total: 0,
            correct: 0,
            by_target: BTreeMap::new(),
        }
    }

    /// Record prediction outcome
    pub fn record(&mut self, target: &str, was_correct: bool) {
        self.total += 1;
        if was_correct {
            self.correct += 1;
        }

        let entry = self.by_target.entry(String::from(target)).or_insert((0, 0));
        entry.0 += 1;
        if was_correct {
            entry.1 += 1;
        }
    }

    /// Get overall accuracy
    pub fn accuracy(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.correct as f64 / self.total as f64
        }
    }
}

impl Default for PredictionAccuracy {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// NEURAL ENGINE
// =============================================================================

/// Neural engine configuration
#[derive(Debug, Clone)]
pub struct NeuralConfig {
    /// Maximum decision time (cycles)
    pub max_decision_cycles: u64,

    /// Minimum confidence to act
    pub min_confidence: f64,

    /// Enable learning
    pub learning_enabled: bool,

    /// Learning rate
    pub learning_rate: f64,
}

impl Default for NeuralConfig {
    fn default() -> Self {
        Self {
            max_decision_cycles: 10000,
            min_confidence: 0.5,
            learning_enabled: true,
            learning_rate: 0.01,
        }
    }
}

impl From<&CortexConfig> for NeuralConfig {
    fn from(config: &CortexConfig) -> Self {
        Self {
            max_decision_cycles: config.decision_timeout_us * 1000, // Approximate
            min_confidence: 0.5,
            learning_enabled: true,
            learning_rate: 0.01,
        }
    }
}

/// The Neural Decision Engine
pub struct NeuralEngine {
    /// Configuration
    config: NeuralConfig,

    /// Pattern detector
    pattern_detector: PatternDetector,

    /// Decision trees
    trees: BTreeMap<DecisionTreeId, DecisionTree>,

    /// Next tree ID
    next_tree_id: AtomicU64,

    /// Current decision context
    context: DecisionContext,

    /// Prediction accuracy tracker
    prediction_accuracy: PredictionAccuracy,

    /// Decisions made
    decisions_made: u64,

    /// Average decision time
    avg_decision_time: f64,
}

impl NeuralEngine {
    /// Create new neural engine
    pub fn new(config: NeuralConfig) -> Self {
        let mut engine = Self {
            config,
            pattern_detector: PatternDetector::new(),
            trees: BTreeMap::new(),
            next_tree_id: AtomicU64::new(1),
            context: DecisionContext::default(),
            prediction_accuracy: PredictionAccuracy::new(),
            decisions_made: 0,
            avg_decision_time: 0.0,
        };

        // Register built-in trees
        engine.register_builtin_trees();

        engine
    }

    /// Register built-in decision trees
    fn register_builtin_trees(&mut self) {
        // Memory pressure tree
        let memory_tree = self.create_memory_pressure_tree();
        self.trees.insert(memory_tree.id, memory_tree);

        // CPU overload tree
        let cpu_tree = self.create_cpu_overload_tree();
        self.trees.insert(cpu_tree.id, cpu_tree);

        // Security threat tree
        let security_tree = self.create_security_threat_tree();
        self.trees.insert(security_tree.id, security_tree);
    }

    /// Create memory pressure decision tree
    fn create_memory_pressure_tree(&mut self) -> DecisionTree {
        let id = DecisionTreeId(self.next_tree_id.fetch_add(1, Ordering::SeqCst));
        let mut tree = DecisionTree::new(id, "memory_pressure");

        // Root: Check memory usage
        let root = DecisionNode {
            id: NodeId(0),
            node_type: NodeType::Branch {
                condition: Condition {
                    feature: Feature::MemoryUsage,
                    operator: Operator::GreaterThan,
                    threshold: 90.0,
                },
                true_child: NodeId(1),
                false_child: NodeId(4),
            },
            depth: 0,
            visit_count: 0,
        };
        tree.add_node(root);
        tree.root = NodeId(0);

        // Critical memory: Check if we can swap
        tree.add_node(DecisionNode {
            id: NodeId(1),
            node_type: NodeType::Branch {
                condition: Condition {
                    feature: Feature::ProcessCount,
                    operator: Operator::GreaterThan,
                    threshold: 100.0,
                },
                true_child: NodeId(2),
                false_child: NodeId(3),
            },
            depth: 1,
            visit_count: 0,
        });

        // Many processes: Aggressive reclaim
        tree.add_node(DecisionNode {
            id: NodeId(2),
            node_type: NodeType::Leaf {
                action: DecisionAction::AdjustMemory(crate::MemoryParams {
                    swap_threshold: Some(50),
                    oom_score_adj: None,
                    memory_limit: None,
                    reclaim_aggressive: Some(true),
                }),
                confidence: 0.9,
            },
            depth: 2,
            visit_count: 0,
        });

        // Few processes: Gentle reclaim
        tree.add_node(DecisionNode {
            id: NodeId(3),
            node_type: NodeType::Leaf {
                action: DecisionAction::AdjustMemory(crate::MemoryParams {
                    swap_threshold: Some(70),
                    oom_score_adj: None,
                    memory_limit: None,
                    reclaim_aggressive: Some(false),
                }),
                confidence: 0.85,
            },
            depth: 2,
            visit_count: 0,
        });

        // Low memory: No action
        tree.add_node(DecisionNode {
            id: NodeId(4),
            node_type: NodeType::Leaf {
                action: DecisionAction::NoOp,
                confidence: 0.99,
            },
            depth: 1,
            visit_count: 0,
        });

        tree
    }

    /// Create CPU overload decision tree
    fn create_cpu_overload_tree(&mut self) -> DecisionTree {
        let id = DecisionTreeId(self.next_tree_id.fetch_add(1, Ordering::SeqCst));
        let mut tree = DecisionTree::new(id, "cpu_overload");

        // Root: Check CPU load
        tree.add_node(DecisionNode {
            id: NodeId(0),
            node_type: NodeType::Branch {
                condition: Condition {
                    feature: Feature::CpuLoad,
                    operator: Operator::GreaterThan,
                    threshold: 95.0,
                },
                true_child: NodeId(1),
                false_child: NodeId(3),
            },
            depth: 0,
            visit_count: 0,
        });
        tree.root = NodeId(0);

        // High CPU: Check context switches
        tree.add_node(DecisionNode {
            id: NodeId(1),
            node_type: NodeType::Branch {
                condition: Condition {
                    feature: Feature::ContextSwitchRate,
                    operator: Operator::GreaterThan,
                    threshold: 10000.0,
                },
                true_child: NodeId(2),
                false_child: NodeId(4),
            },
            depth: 1,
            visit_count: 0,
        });

        // Too many context switches: Increase timeslice
        tree.add_node(DecisionNode {
            id: NodeId(2),
            node_type: NodeType::Leaf {
                action: DecisionAction::AdjustScheduler(crate::SchedulerParams {
                    timeslice_us: Some(20000), // Double timeslice
                    priority_boost: None,
                    affinity_mask: None,
                    preemption_enabled: Some(true),
                }),
                confidence: 0.8,
            },
            depth: 2,
            visit_count: 0,
        });

        // Normal context switches: No action
        tree.add_node(DecisionNode {
            id: NodeId(3),
            node_type: NodeType::Leaf {
                action: DecisionAction::NoOp,
                confidence: 0.95,
            },
            depth: 1,
            visit_count: 0,
        });

        // High CPU but low switches: CPU-bound, no action
        tree.add_node(DecisionNode {
            id: NodeId(4),
            node_type: NodeType::Leaf {
                action: DecisionAction::NoOp,
                confidence: 0.9,
            },
            depth: 2,
            visit_count: 0,
        });

        tree
    }

    /// Create security threat decision tree
    fn create_security_threat_tree(&mut self) -> DecisionTree {
        let id = DecisionTreeId(self.next_tree_id.fetch_add(1, Ordering::SeqCst));
        let mut tree = DecisionTree::new(id, "security_threat");

        // Root: Check violation severity
        tree.add_node(DecisionNode {
            id: NodeId(0),
            node_type: NodeType::MultiBranch {
                feature: Feature::ViolationSeverity,
                children: vec![
                    (
                        Condition {
                            feature: Feature::ViolationSeverity,
                            operator: Operator::GreaterOrEqual,
                            threshold: 4.0, // Fatal
                        },
                        NodeId(1),
                    ),
                    (
                        Condition {
                            feature: Feature::ViolationSeverity,
                            operator: Operator::GreaterOrEqual,
                            threshold: 3.0, // Critical
                        },
                        NodeId(2),
                    ),
                    (
                        Condition {
                            feature: Feature::ViolationSeverity,
                            operator: Operator::GreaterOrEqual,
                            threshold: 2.0, // Error
                        },
                        NodeId(3),
                    ),
                ],
                default: NodeId(4),
            },
            depth: 0,
            visit_count: 0,
        });
        tree.root = NodeId(0);

        // Fatal: Isolate subsystem
        tree.add_node(DecisionNode {
            id: NodeId(1),
            node_type: NodeType::Leaf {
                action: DecisionAction::IsolateSubsystem(SubsystemId(0)), // Placeholder
                confidence: 0.99,
            },
            depth: 1,
            visit_count: 0,
        });

        // Critical: Disable code path
        tree.add_node(DecisionNode {
            id: NodeId(2),
            node_type: NodeType::Leaf {
                action: DecisionAction::DisablePath(crate::PathId(0)), // Placeholder
                confidence: 0.9,
            },
            depth: 1,
            visit_count: 0,
        });

        // Error: Monitor closely
        tree.add_node(DecisionNode {
            id: NodeId(3),
            node_type: NodeType::Leaf {
                action: DecisionAction::NoOp, // Would add monitoring
                confidence: 0.7,
            },
            depth: 1,
            visit_count: 0,
        });

        // Low severity: Ignore
        tree.add_node(DecisionNode {
            id: NodeId(4),
            node_type: NodeType::Leaf {
                action: DecisionAction::NoOp,
                confidence: 0.95,
            },
            depth: 1,
            visit_count: 0,
        });

        tree
    }

    /// Detect pattern in event
    pub fn detect_pattern(&mut self, event: &CortexEvent) -> Option<PatternId> {
        let timestamp = get_cycles();
        self.pattern_detector.detect(event, timestamp).map(|p| p.id)
    }

    /// Analyze event
    pub fn analyze(&mut self, event: &CortexEvent) -> AnalysisResult {
        // Update context from event
        self.update_context(event);

        // Detect patterns
        let pattern = self.pattern_detector.detect(event, get_cycles());

        // Make predictions
        let prediction = self.predict(event);

        AnalysisResult {
            pattern,
            prediction,
            context_updated: true,
        }
    }

    /// Update context from event
    fn update_context(&mut self, event: &CortexEvent) {
        match event {
            CortexEvent::MemoryPressure(level) => {
                self.context.memory_usage = *level as f64;
            },
            CortexEvent::CpuLoad(load) => {
                self.context.cpu_load = *load as f64;
            },
            CortexEvent::Latency(us) => {
                self.context.interrupt_latency = *us as f64;
            },
            CortexEvent::ContextSwitch => {
                self.context.context_switch_rate += 1.0;
            },
            CortexEvent::PageFault => {
                self.context.page_fault_rate += 1.0;
            },
            _ => {},
        }
    }

    /// Make prediction based on event
    pub fn predict(&self, _event: &CortexEvent) -> Option<Prediction> {
        // Simple linear extrapolation for memory usage
        if self.context.memory_usage > 70.0 {
            let trend = 0.5; // Placeholder - would be calculated from history
            let predicted = self.context.memory_usage + trend * 10.0;

            if predicted > 95.0 {
                return Some(Prediction {
                    target: PredictionTarget::MemoryUsage,
                    value: predicted,
                    horizon: 100,
                    confidence: Confidence::new(0.7),
                    reasoning: String::from("Memory usage trending upward"),
                });
            }
        }

        None
    }

    /// Decide action for event
    pub fn decide(&mut self, event: &CortexEvent) -> Option<Decision> {
        self.update_context(event);

        // Select appropriate tree based on event
        let tree_id = self.select_tree(event);

        if let Some(tree) = self.trees.get_mut(&tree_id) {
            let decision = tree.decide(&self.context);

            if let Some(ref d) = decision {
                self.decisions_made += 1;

                // Update rolling average
                let alpha = 0.1;
                self.avg_decision_time =
                    self.avg_decision_time * (1.0 - alpha) + d.decision_time as f64 * alpha;
            }

            decision
        } else {
            None
        }
    }

    /// Decide action for invariant violation
    pub fn decide_violation(&mut self, violation: &InvariantViolation) -> Option<Decision> {
        self.context.violation_severity = violation.severity as u32;

        let tree_id = DecisionTreeId(3); // Security tree

        if let Some(tree) = self.trees.get_mut(&tree_id) {
            tree.decide(&self.context)
        } else {
            None
        }
    }

    /// Decide action for prediction
    pub fn decide_prediction(&mut self, prediction: &Prediction) -> Option<Decision> {
        match prediction.target {
            PredictionTarget::MemoryUsage => {
                self.context.memory_usage = prediction.value;
                let tree_id = DecisionTreeId(1);

                if let Some(tree) = self.trees.get_mut(&tree_id) {
                    tree.decide(&self.context)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Select appropriate tree for event
    fn select_tree(&self, event: &CortexEvent) -> DecisionTreeId {
        match event {
            CortexEvent::MemoryPressure(_) => DecisionTreeId(1),
            CortexEvent::CpuLoad(_) => DecisionTreeId(2),
            CortexEvent::SecurityViolation(_) => DecisionTreeId(3),
            _ => DecisionTreeId(1), // Default to memory tree
        }
    }

    /// Get statistics
    pub fn stats(&self) -> NeuralStats {
        NeuralStats {
            decisions_made: self.decisions_made,
            avg_decision_time_cycles: self.avg_decision_time,
            trees_registered: self.trees.len(),
            patterns_registered: self.pattern_detector.patterns.len(),
            prediction_accuracy: self.prediction_accuracy.accuracy(),
        }
    }
}

/// Analysis result
#[derive(Debug)]
pub struct AnalysisResult {
    pub pattern: Option<Pattern>,
    pub prediction: Option<Prediction>,
    pub context_updated: bool,
}

/// Neural engine statistics
#[derive(Debug, Clone)]
pub struct NeuralStats {
    pub decisions_made: u64,
    pub avg_decision_time_cycles: f64,
    pub trees_registered: usize,
    pub patterns_registered: usize,
    pub prediction_accuracy: f64,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence() {
        let conf = Confidence::new(0.8);
        assert!(conf.is_high());
        assert!(!conf.is_very_high());
    }

    #[test]
    fn test_neural_engine_creation() {
        let engine = NeuralEngine::new(NeuralConfig::default());
        assert_eq!(engine.trees.len(), 3);
    }

    #[test]
    fn test_decision_tree() {
        let mut engine = NeuralEngine::new(NeuralConfig::default());

        // High memory usage
        let event = CortexEvent::MemoryPressure(95);
        engine.update_context(&event);
        engine.context.process_count = 150;

        let decision = engine.decide(&event);
        assert!(decision.is_some());
    }
}

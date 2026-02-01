//! # Action Sequencing
//!
//! Manages sequences of actions with dependencies.
//! Implements action pipelines and workflows.
//!
//! Part of Year 2 COGNITION - Act/Sequence

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SEQUENCE TYPES
// ============================================================================

/// Action sequence
#[derive(Debug, Clone)]
pub struct ActionSequence {
    /// Sequence ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Steps
    pub steps: Vec<SequenceStep>,
    /// Status
    pub status: SequenceStatus,
    /// Current step index
    pub current: usize,
    /// Created
    pub created: Timestamp,
    /// Started
    pub started: Option<Timestamp>,
    /// Completed
    pub completed: Option<Timestamp>,
}

/// Sequence step
#[derive(Debug, Clone)]
pub struct SequenceStep {
    /// Step ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Action
    pub action: StepAction,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Status
    pub status: StepStatus,
    /// Retry config
    pub retry: RetryConfig,
    /// Result
    pub result: Option<StepResult>,
}

/// Step action
#[derive(Debug, Clone)]
pub enum StepAction {
    Execute(String),
    Transform(TransformSpec),
    Branch(BranchSpec),
    Parallel(Vec<u64>),
    Wait(WaitSpec),
}

/// Transform specification
#[derive(Debug, Clone)]
pub struct TransformSpec {
    /// Input
    pub input: String,
    /// Transform type
    pub transform: String,
    /// Parameters
    pub params: BTreeMap<String, String>,
}

/// Branch specification
#[derive(Debug, Clone)]
pub struct BranchSpec {
    /// Condition
    pub condition: String,
    /// True branch
    pub on_true: u64,
    /// False branch
    pub on_false: u64,
}

/// Wait specification
#[derive(Debug, Clone)]
pub struct WaitSpec {
    /// Wait type
    pub wait_type: WaitType,
    /// Timeout ms
    pub timeout: u64,
}

/// Wait type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitType {
    Signal,
    Timer,
    Condition,
    External,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retries
    pub max_retries: u32,
    /// Current retry count
    pub current: u32,
    /// Delay between retries (ms)
    pub delay: u64,
    /// Backoff multiplier
    pub backoff: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            current: 0,
            delay: 1000,
            backoff: 2.0,
        }
    }
}

/// Sequence status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Step result
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Success
    pub success: bool,
    /// Output
    pub output: Option<String>,
    /// Error
    pub error: Option<String>,
    /// Duration (ms)
    pub duration: u64,
}

// ============================================================================
// SEQUENCE ENGINE
// ============================================================================

/// Sequence engine
pub struct SequenceEngine {
    /// Sequences
    sequences: BTreeMap<u64, ActionSequence>,
    /// Step outputs
    outputs: BTreeMap<u64, String>,
    /// Variables
    variables: BTreeMap<String, String>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: SequenceConfig,
    /// Statistics
    stats: SequenceStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SequenceConfig {
    /// Maximum parallel steps
    pub max_parallel: usize,
    /// Default timeout (ms)
    pub default_timeout: u64,
    /// Enable retry
    pub enable_retry: bool,
}

impl Default for SequenceConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            default_timeout: 30000,
            enable_retry: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct SequenceStats {
    /// Sequences created
    pub sequences_created: u64,
    /// Sequences completed
    pub sequences_completed: u64,
    /// Steps executed
    pub steps_executed: u64,
    /// Retries performed
    pub retries: u64,
}

impl SequenceEngine {
    /// Create new engine
    pub fn new(config: SequenceConfig) -> Self {
        Self {
            sequences: BTreeMap::new(),
            outputs: BTreeMap::new(),
            variables: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SequenceStats::default(),
        }
    }

    /// Create sequence
    pub fn create_sequence(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let sequence = ActionSequence {
            id,
            name: name.into(),
            steps: Vec::new(),
            status: SequenceStatus::Pending,
            current: 0,
            created: Timestamp::now(),
            started: None,
            completed: None,
        };

        self.sequences.insert(id, sequence);
        self.stats.sequences_created += 1;

        id
    }

    /// Add step
    pub fn add_step(
        &mut self,
        sequence_id: u64,
        name: &str,
        action: StepAction,
        dependencies: Vec<u64>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let step = SequenceStep {
            id,
            name: name.into(),
            action,
            dependencies,
            status: StepStatus::Pending,
            retry: RetryConfig::default(),
            result: None,
        };

        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            seq.steps.push(step);
        }

        id
    }

    /// Start sequence
    pub fn start(&mut self, sequence_id: u64) -> bool {
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            if seq.status == SequenceStatus::Pending {
                seq.status = SequenceStatus::Running;
                seq.started = Some(Timestamp::now());

                // Mark initial steps as ready
                self.update_ready_steps(sequence_id);

                return true;
            }
        }
        false
    }

    fn update_ready_steps(&mut self, sequence_id: u64) {
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            for step in &mut seq.steps {
                if step.status != StepStatus::Pending {
                    continue;
                }

                // Check if all dependencies are completed
                let deps_met = step.dependencies.iter().all(|dep_id| {
                    seq.steps
                        .iter()
                        .find(|s| s.id == *dep_id)
                        .map(|s| s.status == StepStatus::Completed)
                        .unwrap_or(true)
                });

                if deps_met {
                    step.status = StepStatus::Ready;
                }
            }
        }
    }

    /// Execute next step
    pub fn execute_next(&mut self, sequence_id: u64) -> Option<StepResult> {
        let (step_id, action) = {
            let seq = self.sequences.get_mut(&sequence_id)?;

            if seq.status != SequenceStatus::Running {
                return None;
            }

            // Find ready step
            let step = seq
                .steps
                .iter_mut()
                .find(|s| s.status == StepStatus::Ready)?;

            step.status = StepStatus::Running;
            (step.id, step.action.clone())
        };

        // Execute the step
        let result = self.execute_action(&action);
        self.stats.steps_executed += 1;

        // Update step with result
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            if let Some(step) = seq.steps.iter_mut().find(|s| s.id == step_id) {
                if result.success {
                    step.status = StepStatus::Completed;
                    if let Some(ref output) = result.output {
                        self.outputs.insert(step_id, output.clone());
                    }
                } else if self.config.enable_retry && step.retry.current < step.retry.max_retries {
                    step.retry.current += 1;
                    step.status = StepStatus::Ready;
                    self.stats.retries += 1;
                } else {
                    step.status = StepStatus::Failed;
                    seq.status = SequenceStatus::Failed;
                }

                step.result = Some(result.clone());
            }

            // Update ready steps
            self.update_ready_steps(sequence_id);

            // Check if complete
            self.check_completion(sequence_id);
        }

        Some(result)
    }

    fn execute_action(&mut self, action: &StepAction) -> StepResult {
        let start = Timestamp::now();

        let (success, output, error) = match action {
            StepAction::Execute(cmd) => {
                // Simulate execution
                (true, Some(format!("Executed: {}", cmd)), None)
            },
            StepAction::Transform(spec) => {
                // Apply transformation
                let result = format!("Transformed {} via {}", spec.input, spec.transform);
                (true, Some(result), None)
            },
            StepAction::Branch(spec) => {
                // Evaluate condition
                let condition_met = self.evaluate_condition(&spec.condition);
                let next = if condition_met {
                    spec.on_true
                } else {
                    spec.on_false
                };
                (true, Some(format!("Branch to {}", next)), None)
            },
            StepAction::Parallel(steps) => {
                // Track parallel steps
                (true, Some(format!("Parallel: {:?}", steps)), None)
            },
            StepAction::Wait(spec) => {
                // Simulate wait
                (true, Some(format!("Wait {:?}", spec.wait_type)), None)
            },
        };

        let duration = Timestamp::now().0 - start.0;

        StepResult {
            success,
            output,
            error,
            duration,
        }
    }

    fn evaluate_condition(&self, condition: &str) -> bool {
        // Simple condition evaluation
        if let Some(value) = self.variables.get(condition) {
            value == "true" || value == "1"
        } else {
            false
        }
    }

    fn check_completion(&mut self, sequence_id: u64) {
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            let all_done = seq.steps.iter().all(|s| {
                matches!(
                    s.status,
                    StepStatus::Completed | StepStatus::Skipped | StepStatus::Failed
                )
            });

            let any_failed = seq.steps.iter().any(|s| s.status == StepStatus::Failed);

            if all_done {
                if any_failed {
                    seq.status = SequenceStatus::Failed;
                } else {
                    seq.status = SequenceStatus::Completed;
                    self.stats.sequences_completed += 1;
                }
                seq.completed = Some(Timestamp::now());
            }
        }
    }

    /// Run until complete
    pub fn run_to_completion(&mut self, sequence_id: u64) -> SequenceStatus {
        self.start(sequence_id);

        loop {
            if self.execute_next(sequence_id).is_none() {
                break;
            }
        }

        self.sequences
            .get(&sequence_id)
            .map(|s| s.status)
            .unwrap_or(SequenceStatus::Failed)
    }

    /// Pause sequence
    pub fn pause(&mut self, sequence_id: u64) {
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            if seq.status == SequenceStatus::Running {
                seq.status = SequenceStatus::Paused;
            }
        }
    }

    /// Resume sequence
    pub fn resume(&mut self, sequence_id: u64) {
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            if seq.status == SequenceStatus::Paused {
                seq.status = SequenceStatus::Running;
            }
        }
    }

    /// Cancel sequence
    pub fn cancel(&mut self, sequence_id: u64) {
        if let Some(seq) = self.sequences.get_mut(&sequence_id) {
            seq.status = SequenceStatus::Cancelled;
            seq.completed = Some(Timestamp::now());
        }
    }

    /// Set variable
    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.into(), value.into());
    }

    /// Get step output
    pub fn get_output(&self, step_id: u64) -> Option<&String> {
        self.outputs.get(&step_id)
    }

    /// Get sequence
    pub fn get(&self, id: u64) -> Option<&ActionSequence> {
        self.sequences.get(&id)
    }

    /// Get progress
    pub fn progress(&self, sequence_id: u64) -> f64 {
        self.sequences
            .get(&sequence_id)
            .map(|seq| {
                if seq.steps.is_empty() {
                    1.0
                } else {
                    let completed = seq
                        .steps
                        .iter()
                        .filter(|s| s.status == StepStatus::Completed)
                        .count();
                    completed as f64 / seq.steps.len() as f64
                }
            })
            .unwrap_or(0.0)
    }

    /// Get statistics
    pub fn stats(&self) -> &SequenceStats {
        &self.stats
    }
}

impl Default for SequenceEngine {
    fn default() -> Self {
        Self::new(SequenceConfig::default())
    }
}

// ============================================================================
// BUILDER
// ============================================================================

/// Sequence builder
pub struct SequenceBuilder<'a> {
    engine: &'a mut SequenceEngine,
    sequence_id: u64,
}

impl<'a> SequenceBuilder<'a> {
    /// Create new builder
    pub fn new(engine: &'a mut SequenceEngine, name: &str) -> Self {
        let sequence_id = engine.create_sequence(name);
        Self {
            engine,
            sequence_id,
        }
    }

    /// Add execute step
    pub fn execute(self, name: &str, command: &str) -> Self {
        self.engine.add_step(
            self.sequence_id,
            name,
            StepAction::Execute(command.into()),
            Vec::new(),
        );
        self
    }

    /// Add step with dependencies
    pub fn execute_after(self, name: &str, command: &str, deps: Vec<u64>) -> Self {
        self.engine.add_step(
            self.sequence_id,
            name,
            StepAction::Execute(command.into()),
            deps,
        );
        self
    }

    /// Add transform step
    pub fn transform(self, name: &str, input: &str, transform: &str) -> Self {
        self.engine.add_step(
            self.sequence_id,
            name,
            StepAction::Transform(TransformSpec {
                input: input.into(),
                transform: transform.into(),
                params: BTreeMap::new(),
            }),
            Vec::new(),
        );
        self
    }

    /// Build and return sequence ID
    pub fn build(self) -> u64 {
        self.sequence_id
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sequence() {
        let mut engine = SequenceEngine::default();

        let id = engine.create_sequence("test");
        assert!(engine.get(id).is_some());
    }

    #[test]
    fn test_add_step() {
        let mut engine = SequenceEngine::default();

        let seq = engine.create_sequence("test");
        engine.add_step(seq, "step1", StepAction::Execute("cmd".into()), Vec::new());

        let s = engine.get(seq).unwrap();
        assert_eq!(s.steps.len(), 1);
    }

    #[test]
    fn test_execute() {
        let mut engine = SequenceEngine::default();

        let seq = engine.create_sequence("test");
        engine.add_step(
            seq,
            "step1",
            StepAction::Execute("echo hello".into()),
            Vec::new(),
        );

        engine.start(seq);
        let result = engine.execute_next(seq);

        assert!(result.is_some());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_dependencies() {
        let mut engine = SequenceEngine::default();

        let seq = engine.create_sequence("test");
        let s1 = engine.add_step(seq, "step1", StepAction::Execute("cmd1".into()), Vec::new());
        engine.add_step(seq, "step2", StepAction::Execute("cmd2".into()), vec![s1]);

        engine.start(seq);

        // Only first step should be ready
        let s = engine.get(seq).unwrap();
        assert_eq!(s.steps[0].status, StepStatus::Ready);
        assert_eq!(s.steps[1].status, StepStatus::Pending);
    }

    #[test]
    fn test_run_to_completion() {
        let mut engine = SequenceEngine::default();

        let seq = engine.create_sequence("test");
        engine.add_step(seq, "s1", StepAction::Execute("c1".into()), Vec::new());
        engine.add_step(seq, "s2", StepAction::Execute("c2".into()), Vec::new());

        let status = engine.run_to_completion(seq);
        assert_eq!(status, SequenceStatus::Completed);
    }

    #[test]
    fn test_progress() {
        let mut engine = SequenceEngine::default();

        let seq = engine.create_sequence("test");
        engine.add_step(seq, "s1", StepAction::Execute("c1".into()), Vec::new());
        engine.add_step(seq, "s2", StepAction::Execute("c2".into()), Vec::new());

        assert_eq!(engine.progress(seq), 0.0);

        engine.start(seq);
        engine.execute_next(seq);

        assert_eq!(engine.progress(seq), 0.5);
    }

    #[test]
    fn test_builder() {
        let mut engine = SequenceEngine::default();

        let seq = SequenceBuilder::new(&mut engine, "pipeline")
            .execute("compile", "rustc")
            .execute("test", "cargo test")
            .build();

        let s = engine.get(seq).unwrap();
        assert_eq!(s.steps.len(), 2);
    }
}

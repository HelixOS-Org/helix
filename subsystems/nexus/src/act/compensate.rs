//! # Action Compensation
//!
//! Compensates for failed or interrupted actions.
//! Implements saga pattern and compensation strategies.
//!
//! Part of Year 2 COGNITION - Action Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// COMPENSATION TYPES
// ============================================================================

/// Saga
#[derive(Debug, Clone)]
pub struct Saga {
    /// Saga ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Steps
    pub steps: Vec<SagaStep>,
    /// Status
    pub status: SagaStatus,
    /// Current step
    pub current_step: usize,
    /// Started
    pub started: Option<Timestamp>,
    /// Completed
    pub completed: Option<Timestamp>,
}

/// Saga step
#[derive(Debug, Clone)]
pub struct SagaStep {
    /// Step ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Action ID
    pub action_id: u64,
    /// Compensation action ID
    pub compensation_id: Option<u64>,
    /// Status
    pub status: StepStatus,
    /// Result
    pub result: Option<StepResult>,
}

/// Saga status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaStatus {
    Pending,
    Running,
    Completed,
    Compensating,
    Compensated,
    Failed,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Compensating,
    Compensated,
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
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Compensation action
#[derive(Debug, Clone)]
pub struct CompensationAction {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Original action ID
    pub original_id: u64,
    /// Strategy
    pub strategy: CompensationStrategy,
    /// Status
    pub status: CompensationStatus,
}

/// Compensation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompensationStrategy {
    /// Reverse the action
    Reverse,
    /// Retry the action
    Retry,
    /// Use alternative
    Alternative,
    /// Skip and continue
    Skip,
    /// Manual intervention
    Manual,
}

/// Compensation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompensationStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Compensation result
#[derive(Debug, Clone)]
pub struct CompensationResult {
    /// Saga ID
    pub saga_id: u64,
    /// Steps compensated
    pub steps_compensated: usize,
    /// Steps failed
    pub steps_failed: usize,
    /// Final status
    pub final_status: SagaStatus,
    /// Duration ns
    pub duration_ns: u64,
}

// ============================================================================
// COMPENSATOR
// ============================================================================

/// Action compensator
pub struct ActionCompensator {
    /// Sagas
    sagas: BTreeMap<u64, Saga>,
    /// Compensations
    compensations: BTreeMap<u64, CompensationAction>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CompensatorConfig,
    /// Statistics
    stats: CompensatorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CompensatorConfig {
    /// Maximum retries
    pub max_retries: usize,
    /// Compensation timeout ns
    pub timeout_ns: u64,
    /// Auto-compensate on failure
    pub auto_compensate: bool,
}

impl Default for CompensatorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_ns: 60_000_000_000, // 60 seconds
            auto_compensate: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct CompensatorStats {
    /// Sagas created
    pub sagas_created: u64,
    /// Sagas completed
    pub sagas_completed: u64,
    /// Sagas compensated
    pub sagas_compensated: u64,
    /// Compensation failures
    pub compensation_failures: u64,
}

impl ActionCompensator {
    /// Create new compensator
    pub fn new(config: CompensatorConfig) -> Self {
        Self {
            sagas: BTreeMap::new(),
            compensations: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: CompensatorStats::default(),
        }
    }

    /// Create saga
    pub fn create_saga(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let saga = Saga {
            id,
            name: name.into(),
            steps: Vec::new(),
            status: SagaStatus::Pending,
            current_step: 0,
            started: None,
            completed: None,
        };

        self.sagas.insert(id, saga);
        self.stats.sagas_created += 1;

        id
    }

    /// Add step
    pub fn add_step(
        &mut self,
        saga_id: u64,
        name: &str,
        action_id: u64,
        compensation_id: Option<u64>,
    ) -> u64 {
        let step_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let step = SagaStep {
            id: step_id,
            name: name.into(),
            action_id,
            compensation_id,
            status: StepStatus::Pending,
            result: None,
        };

        if let Some(saga) = self.sagas.get_mut(&saga_id) {
            saga.steps.push(step);
        }

        step_id
    }

    /// Start saga
    pub fn start(&mut self, saga_id: u64) -> bool {
        let saga = match self.sagas.get_mut(&saga_id) {
            Some(s) => s,
            None => return false,
        };

        if saga.status != SagaStatus::Pending {
            return false;
        }

        saga.status = SagaStatus::Running;
        saga.started = Some(Timestamp::now());
        saga.current_step = 0;

        true
    }

    /// Complete step
    pub fn complete_step(&mut self, saga_id: u64, success: bool, output: Option<String>) {
        let saga = match self.sagas.get_mut(&saga_id) {
            Some(s) => s,
            None => return,
        };

        if saga.status != SagaStatus::Running {
            return;
        }

        let step_idx = saga.current_step;
        if step_idx >= saga.steps.len() {
            return;
        }

        let step = &mut saga.steps[step_idx];

        if success {
            step.status = StepStatus::Completed;
            step.result = Some(StepResult {
                success: true,
                output,
                error: None,
                timestamp: Timestamp::now(),
            });

            saga.current_step += 1;

            // Check if saga is complete
            if saga.current_step >= saga.steps.len() {
                saga.status = SagaStatus::Completed;
                saga.completed = Some(Timestamp::now());
                self.stats.sagas_completed += 1;
            }
        } else {
            step.status = StepStatus::Failed;
            step.result = Some(StepResult {
                success: false,
                output: None,
                error: output,
                timestamp: Timestamp::now(),
            });

            if self.config.auto_compensate {
                saga.status = SagaStatus::Compensating;
            } else {
                saga.status = SagaStatus::Failed;
            }
        }
    }

    /// Compensate saga
    pub fn compensate(&mut self, saga_id: u64) -> Option<CompensationResult> {
        let start = Timestamp::now();

        let saga = self.sagas.get_mut(&saga_id)?;

        if saga.status != SagaStatus::Compensating && saga.status != SagaStatus::Running {
            return None;
        }

        saga.status = SagaStatus::Compensating;

        let mut steps_compensated = 0;
        let mut steps_failed = 0;

        // Compensate in reverse order
        let steps_to_compensate: Vec<usize> = (0..saga.current_step)
            .rev()
            .filter(|&i| saga.steps[i].status == StepStatus::Completed)
            .collect();

        for i in steps_to_compensate {
            let step = &mut saga.steps[i];

            if step.compensation_id.is_some() {
                // Execute compensation (simulated)
                step.status = StepStatus::Compensated;
                steps_compensated += 1;
            } else {
                // No compensation available
                step.status = StepStatus::Failed;
                steps_failed += 1;
            }
        }

        let final_status = if steps_failed == 0 {
            saga.status = SagaStatus::Compensated;
            self.stats.sagas_compensated += 1;
            SagaStatus::Compensated
        } else {
            saga.status = SagaStatus::Failed;
            self.stats.compensation_failures += 1;
            SagaStatus::Failed
        };

        saga.completed = Some(Timestamp::now());

        let end = Timestamp::now();

        Some(CompensationResult {
            saga_id,
            steps_compensated,
            steps_failed,
            final_status,
            duration_ns: end.0.saturating_sub(start.0),
        })
    }

    /// Register compensation action
    pub fn register_compensation(
        &mut self,
        original_id: u64,
        name: &str,
        strategy: CompensationStrategy,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let comp = CompensationAction {
            id,
            name: name.into(),
            original_id,
            strategy,
            status: CompensationStatus::Pending,
        };

        self.compensations.insert(id, comp);
        id
    }

    /// Execute compensation
    pub fn execute_compensation(&mut self, id: u64) -> Option<bool> {
        let comp = self.compensations.get_mut(&id)?;

        comp.status = CompensationStatus::Running;

        // Simulated execution
        let success = match comp.strategy {
            CompensationStrategy::Reverse => true,
            CompensationStrategy::Retry => true,
            CompensationStrategy::Alternative => true,
            CompensationStrategy::Skip => true,
            CompensationStrategy::Manual => false,
        };

        comp.status = if success {
            CompensationStatus::Completed
        } else {
            CompensationStatus::Failed
        };

        Some(success)
    }

    /// Get saga
    pub fn get_saga(&self, id: u64) -> Option<&Saga> {
        self.sagas.get(&id)
    }

    /// Get active sagas
    pub fn active_sagas(&self) -> Vec<&Saga> {
        self.sagas.values()
            .filter(|s| s.status == SagaStatus::Running || s.status == SagaStatus::Compensating)
            .collect()
    }

    /// Get failed sagas
    pub fn failed_sagas(&self) -> Vec<&Saga> {
        self.sagas.values()
            .filter(|s| s.status == SagaStatus::Failed)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &CompensatorStats {
        &self.stats
    }
}

impl Default for ActionCompensator {
    fn default() -> Self {
        Self::new(CompensatorConfig::default())
    }
}

// ============================================================================
// SAGA BUILDER
// ============================================================================

/// Saga builder
pub struct SagaBuilder<'a> {
    compensator: &'a mut ActionCompensator,
    saga_id: u64,
}

impl<'a> SagaBuilder<'a> {
    /// Create new builder
    pub fn new(compensator: &'a mut ActionCompensator, name: &str) -> Self {
        let saga_id = compensator.create_saga(name);

        Self {
            compensator,
            saga_id,
        }
    }

    /// Add step
    pub fn step(self, name: &str, action_id: u64, compensation_id: Option<u64>) -> Self {
        self.compensator.add_step(self.saga_id, name, action_id, compensation_id);
        self
    }

    /// Build
    pub fn build(self) -> u64 {
        self.saga_id
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_saga() {
        let mut compensator = ActionCompensator::default();

        let id = compensator.create_saga("test");
        assert!(compensator.get_saga(id).is_some());
    }

    #[test]
    fn test_add_step() {
        let mut compensator = ActionCompensator::default();

        let saga_id = compensator.create_saga("test");
        compensator.add_step(saga_id, "step1", 1, Some(10));

        let saga = compensator.get_saga(saga_id).unwrap();
        assert_eq!(saga.steps.len(), 1);
    }

    #[test]
    fn test_complete_saga() {
        let mut compensator = ActionCompensator::default();

        let saga_id = compensator.create_saga("test");
        compensator.add_step(saga_id, "step1", 1, None);
        compensator.add_step(saga_id, "step2", 2, None);

        compensator.start(saga_id);
        compensator.complete_step(saga_id, true, None);
        compensator.complete_step(saga_id, true, None);

        let saga = compensator.get_saga(saga_id).unwrap();
        assert_eq!(saga.status, SagaStatus::Completed);
    }

    #[test]
    fn test_compensate() {
        let mut compensator = ActionCompensator::default();

        let saga_id = compensator.create_saga("test");
        compensator.add_step(saga_id, "step1", 1, Some(10));
        compensator.add_step(saga_id, "step2", 2, Some(20));

        compensator.start(saga_id);
        compensator.complete_step(saga_id, true, None);
        compensator.complete_step(saga_id, false, Some("error".into()));

        let result = compensator.compensate(saga_id).unwrap();
        assert_eq!(result.steps_compensated, 1);
        assert_eq!(result.final_status, SagaStatus::Compensated);
    }

    #[test]
    fn test_builder() {
        let mut compensator = ActionCompensator::default();

        let saga_id = SagaBuilder::new(&mut compensator, "order")
            .step("reserve", 1, Some(10))
            .step("charge", 2, Some(20))
            .step("ship", 3, Some(30))
            .build();

        let saga = compensator.get_saga(saga_id).unwrap();
        assert_eq!(saga.steps.len(), 3);
    }
}

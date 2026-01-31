//! # Procedural Memory System
//!
//! Long-term memory for skills, procedures, and how-to knowledge.
//! Stores and retrieves action sequences and learned behaviors.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PROCEDURE TYPES
// ============================================================================

/// A procedure (skill or action sequence)
#[derive(Debug, Clone)]
pub struct Procedure {
    /// Procedure ID
    pub id: u64,
    /// Procedure name
    pub name: String,
    /// Description
    pub description: String,
    /// Procedure type
    pub procedure_type: ProcedureType,
    /// Steps
    pub steps: Vec<ProcedureStep>,
    /// Preconditions
    pub preconditions: Vec<Condition>,
    /// Postconditions (expected results)
    pub postconditions: Vec<Condition>,
    /// Parameters
    pub parameters: Vec<Parameter>,
    /// Skill level required
    pub skill_level: SkillLevel,
    /// Success rate
    pub success_rate: f64,
    /// Average execution time (ns)
    pub avg_time_ns: u64,
    /// Times executed
    pub execution_count: u64,
    /// Last executed
    pub last_executed: Option<Timestamp>,
    /// Created
    pub created: Timestamp,
}

/// Procedure type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcedureType {
    /// Motor skill
    Motor,
    /// Cognitive skill
    Cognitive,
    /// Problem solving
    ProblemSolving,
    /// Communication
    Communication,
    /// Routine task
    Routine,
    /// Emergency response
    Emergency,
}

/// Procedure step
#[derive(Debug, Clone)]
pub struct ProcedureStep {
    /// Step number
    pub number: u32,
    /// Action to perform
    pub action: Action,
    /// Duration estimate (ns)
    pub duration_ns: u64,
    /// Retry on failure
    pub retry: bool,
    /// Max retries
    pub max_retries: u32,
    /// Branching conditions
    pub branches: Vec<Branch>,
}

/// Action
#[derive(Debug, Clone)]
pub struct Action {
    /// Action type
    pub action_type: ActionType,
    /// Target
    pub target: Option<String>,
    /// Parameters
    pub params: BTreeMap<String, ActionValue>,
    /// Description
    pub description: String,
}

/// Action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    /// Execute command
    Execute,
    /// Wait for condition
    Wait,
    /// Check condition
    Check,
    /// Set value
    Set,
    /// Get value
    Get,
    /// Call sub-procedure
    CallProcedure,
    /// Loop
    Loop,
    /// Parallel execution
    Parallel,
    /// User interaction
    Interact,
}

/// Action value
#[derive(Debug, Clone)]
pub enum ActionValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Reference(String),
    List(Vec<ActionValue>),
}

/// Branch
#[derive(Debug, Clone)]
pub struct Branch {
    /// Condition for branch
    pub condition: Condition,
    /// Target step (or procedure)
    pub target: BranchTarget,
}

/// Branch target
#[derive(Debug, Clone)]
pub enum BranchTarget {
    /// Go to step number
    Step(u32),
    /// Call sub-procedure
    Procedure(u64),
    /// Exit with result
    Exit(bool),
}

/// Condition
#[derive(Debug, Clone)]
pub struct Condition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Variable/target
    pub subject: String,
    /// Expected value
    pub expected: Option<ActionValue>,
    /// Negated
    pub negated: bool,
}

/// Condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionType {
    /// Variable exists
    Exists,
    /// Value equals
    Equals,
    /// Value greater than
    GreaterThan,
    /// Value less than
    LessThan,
    /// Contains
    Contains,
    /// Matches pattern
    Matches,
    /// Custom predicate
    Custom,
}

/// Parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Name
    pub name: String,
    /// Type
    pub param_type: ParameterType,
    /// Required
    pub required: bool,
    /// Default value
    pub default: Option<ActionValue>,
    /// Description
    pub description: String,
}

/// Parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    Bool,
    Int,
    Float,
    String,
    Any,
}

/// Skill level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillLevel {
    Novice,
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

// ============================================================================
// EXECUTION
// ============================================================================

/// Execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Context ID
    pub id: u64,
    /// Current step
    pub current_step: u32,
    /// Variables
    pub variables: BTreeMap<String, ActionValue>,
    /// Call stack
    pub call_stack: Vec<u64>,
    /// Status
    pub status: ExecutionStatus,
    /// Start time
    pub started: Timestamp,
    /// End time
    pub ended: Option<Timestamp>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Ready,
    Running,
    Paused,
    WaitingForInput,
    Completed,
    Failed,
    Cancelled,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Procedure ID
    pub procedure_id: u64,
    /// Success
    pub success: bool,
    /// Output
    pub output: BTreeMap<String, ActionValue>,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Steps executed
    pub steps_executed: u32,
    /// Errors
    pub errors: Vec<String>,
}

// ============================================================================
// PROCEDURAL MEMORY
// ============================================================================

/// Procedural memory store
pub struct ProceduralMemory {
    /// Procedures
    procedures: BTreeMap<u64, Procedure>,
    /// Procedures by name
    by_name: BTreeMap<String, u64>,
    /// Procedures by type
    by_type: BTreeMap<ProcedureType, Vec<u64>>,
    /// Execution history
    history: Vec<ExecutionResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ProceduralConfig,
    /// Statistics
    stats: ProceduralStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ProceduralConfig {
    /// Maximum history size
    pub max_history: usize,
    /// Default timeout (ns)
    pub default_timeout_ns: u64,
    /// Enable learning from execution
    pub enable_learning: bool,
}

impl Default for ProceduralConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            default_timeout_ns: 60_000_000_000, // 60 seconds
            enable_learning: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ProceduralStats {
    /// Procedures stored
    pub procedures_stored: u64,
    /// Executions
    pub executions: u64,
    /// Successful executions
    pub successful: u64,
    /// Failed executions
    pub failed: u64,
    /// Average success rate
    pub avg_success_rate: f64,
}

impl ProceduralMemory {
    /// Create new memory
    pub fn new(config: ProceduralConfig) -> Self {
        Self {
            procedures: BTreeMap::new(),
            by_name: BTreeMap::new(),
            by_type: BTreeMap::new(),
            history: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ProceduralStats::default(),
        }
    }

    /// Store procedure
    pub fn store(&mut self, mut procedure: Procedure) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        procedure.id = id;

        self.by_name.insert(procedure.name.clone(), id);
        self.by_type.entry(procedure.procedure_type)
            .or_insert_with(Vec::new)
            .push(id);

        self.procedures.insert(id, procedure);
        self.stats.procedures_stored += 1;

        id
    }

    /// Get procedure
    pub fn get(&self, id: u64) -> Option<&Procedure> {
        self.procedures.get(&id)
    }

    /// Get by name
    pub fn get_by_name(&self, name: &str) -> Option<&Procedure> {
        let id = self.by_name.get(name)?;
        self.procedures.get(id)
    }

    /// Find by type
    pub fn find_by_type(&self, proc_type: ProcedureType) -> Vec<&Procedure> {
        self.by_type.get(&proc_type)
            .map(|ids| ids.iter().filter_map(|id| self.procedures.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find applicable procedures
    pub fn find_applicable(&self, context: &BTreeMap<String, ActionValue>) -> Vec<&Procedure> {
        self.procedures.values()
            .filter(|p| self.preconditions_met(p, context))
            .collect()
    }

    fn preconditions_met(&self, procedure: &Procedure, context: &BTreeMap<String, ActionValue>) -> bool {
        for cond in &procedure.preconditions {
            if !self.evaluate_condition(cond, context) {
                return false;
            }
        }
        true
    }

    fn evaluate_condition(&self, condition: &Condition, context: &BTreeMap<String, ActionValue>) -> bool {
        let value = context.get(&condition.subject);

        let result = match condition.condition_type {
            ConditionType::Exists => value.is_some(),
            ConditionType::Equals => {
                match (value, &condition.expected) {
                    (Some(v), Some(e)) => self.values_equal(v, e),
                    _ => false,
                }
            }
            ConditionType::GreaterThan => {
                match (value, &condition.expected) {
                    (Some(ActionValue::Float(v)), Some(ActionValue::Float(e))) => v > e,
                    (Some(ActionValue::Int(v)), Some(ActionValue::Int(e))) => v > e,
                    _ => false,
                }
            }
            ConditionType::LessThan => {
                match (value, &condition.expected) {
                    (Some(ActionValue::Float(v)), Some(ActionValue::Float(e))) => v < e,
                    (Some(ActionValue::Int(v)), Some(ActionValue::Int(e))) => v < e,
                    _ => false,
                }
            }
            ConditionType::Contains => {
                match (value, &condition.expected) {
                    (Some(ActionValue::String(v)), Some(ActionValue::String(e))) => v.contains(e.as_str()),
                    _ => false,
                }
            }
            _ => true,
        };

        if condition.negated { !result } else { result }
    }

    fn values_equal(&self, a: &ActionValue, b: &ActionValue) -> bool {
        match (a, b) {
            (ActionValue::Bool(x), ActionValue::Bool(y)) => x == y,
            (ActionValue::Int(x), ActionValue::Int(y)) => x == y,
            (ActionValue::Float(x), ActionValue::Float(y)) => (x - y).abs() < f64::EPSILON,
            (ActionValue::String(x), ActionValue::String(y)) => x == y,
            _ => false,
        }
    }

    /// Record execution result
    pub fn record_execution(&mut self, result: ExecutionResult) {
        self.stats.executions += 1;
        if result.success {
            self.stats.successful += 1;
        } else {
            self.stats.failed += 1;
        }

        // Update procedure statistics
        if let Some(procedure) = self.procedures.get_mut(&result.procedure_id) {
            procedure.execution_count += 1;
            procedure.last_executed = Some(Timestamp::now());

            // Update running average of success rate
            let n = procedure.execution_count as f64;
            let success = if result.success { 1.0 } else { 0.0 };
            procedure.success_rate = (procedure.success_rate * (n - 1.0) + success) / n;

            // Update average execution time
            procedure.avg_time_ns = ((procedure.avg_time_ns as f64 * (n - 1.0) + result.duration_ns as f64) / n) as u64;
        }

        // Update global average
        self.stats.avg_success_rate = self.stats.successful as f64 / self.stats.executions as f64;

        // Store in history
        if self.history.len() >= self.config.max_history {
            self.history.remove(0);
        }
        self.history.push(result);
    }

    /// Get recent executions
    pub fn recent_executions(&self, limit: usize) -> &[ExecutionResult] {
        let start = self.history.len().saturating_sub(limit);
        &self.history[start..]
    }

    /// Find similar procedures
    pub fn find_similar(&self, procedure_id: u64) -> Vec<&Procedure> {
        let procedure = match self.procedures.get(&procedure_id) {
            Some(p) => p,
            None => return Vec::new(),
        };

        self.procedures.values()
            .filter(|p| p.id != procedure_id)
            .filter(|p| {
                // Same type
                p.procedure_type == procedure.procedure_type
                // Or similar parameters
                || p.parameters.iter().any(|param| {
                    procedure.parameters.iter().any(|pp| pp.name == param.name)
                })
            })
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &ProceduralStats {
        &self.stats
    }
}

impl Default for ProceduralMemory {
    fn default() -> Self {
        Self::new(ProceduralConfig::default())
    }
}

// ============================================================================
// PROCEDURE BUILDER
// ============================================================================

/// Procedure builder
pub struct ProcedureBuilder {
    procedure: Procedure,
    step_number: u32,
}

impl ProcedureBuilder {
    /// Create new builder
    pub fn new(name: &str, procedure_type: ProcedureType) -> Self {
        Self {
            procedure: Procedure {
                id: 0,
                name: name.into(),
                description: String::new(),
                procedure_type,
                steps: Vec::new(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                parameters: Vec::new(),
                skill_level: SkillLevel::Beginner,
                success_rate: 1.0,
                avg_time_ns: 0,
                execution_count: 0,
                last_executed: None,
                created: Timestamp::now(),
            },
            step_number: 0,
        }
    }

    /// Set description
    pub fn description(mut self, desc: &str) -> Self {
        self.procedure.description = desc.into();
        self
    }

    /// Set skill level
    pub fn skill_level(mut self, level: SkillLevel) -> Self {
        self.procedure.skill_level = level;
        self
    }

    /// Add parameter
    pub fn parameter(mut self, name: &str, param_type: ParameterType, required: bool) -> Self {
        self.procedure.parameters.push(Parameter {
            name: name.into(),
            param_type,
            required,
            default: None,
            description: String::new(),
        });
        self
    }

    /// Add precondition
    pub fn precondition(mut self, subject: &str, condition_type: ConditionType) -> Self {
        self.procedure.preconditions.push(Condition {
            condition_type,
            subject: subject.into(),
            expected: None,
            negated: false,
        });
        self
    }

    /// Add step
    pub fn step(mut self, action_type: ActionType, description: &str) -> Self {
        self.step_number += 1;
        self.procedure.steps.push(ProcedureStep {
            number: self.step_number,
            action: Action {
                action_type,
                target: None,
                params: BTreeMap::new(),
                description: description.into(),
            },
            duration_ns: 0,
            retry: false,
            max_retries: 0,
            branches: Vec::new(),
        });
        self
    }

    /// Build
    pub fn build(self) -> Procedure {
        self.procedure
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedure_creation() {
        let procedure = ProcedureBuilder::new("test_procedure", ProcedureType::Routine)
            .description("A test procedure")
            .skill_level(SkillLevel::Beginner)
            .step(ActionType::Execute, "Step 1")
            .step(ActionType::Check, "Step 2")
            .build();

        assert_eq!(procedure.name, "test_procedure");
        assert_eq!(procedure.steps.len(), 2);
    }

    #[test]
    fn test_memory_storage() {
        let mut memory = ProceduralMemory::default();

        let procedure = ProcedureBuilder::new("test", ProcedureType::Cognitive)
            .step(ActionType::Execute, "Do something")
            .build();

        let id = memory.store(procedure);
        assert!(memory.get(id).is_some());
        assert!(memory.get_by_name("test").is_some());
    }

    #[test]
    fn test_execution_recording() {
        let mut memory = ProceduralMemory::default();

        let procedure = ProcedureBuilder::new("test", ProcedureType::Routine)
            .build();

        let id = memory.store(procedure);

        let result = ExecutionResult {
            procedure_id: id,
            success: true,
            output: BTreeMap::new(),
            duration_ns: 1000,
            steps_executed: 1,
            errors: Vec::new(),
        };

        memory.record_execution(result);

        assert_eq!(memory.stats().executions, 1);
        assert_eq!(memory.stats().successful, 1);
    }

    #[test]
    fn test_find_by_type() {
        let mut memory = ProceduralMemory::default();

        memory.store(ProcedureBuilder::new("routine1", ProcedureType::Routine).build());
        memory.store(ProcedureBuilder::new("routine2", ProcedureType::Routine).build());
        memory.store(ProcedureBuilder::new("cognitive", ProcedureType::Cognitive).build());

        let routines = memory.find_by_type(ProcedureType::Routine);
        assert_eq!(routines.len(), 2);
    }
}

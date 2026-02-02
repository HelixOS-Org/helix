//! Procedural Memory
//!
//! This module provides storage and retrieval of learned procedures.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{PatternId, ProcedureId};

/// Procedure type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProcedureType {
    /// Recovery procedure
    Recovery,
    /// Optimization procedure
    Optimization,
    /// Diagnostic procedure
    Diagnostic,
    /// Maintenance procedure
    Maintenance,
    /// Emergency procedure
    Emergency,
    /// Routine procedure
    Routine,
}

impl ProcedureType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Recovery => "recovery",
            Self::Optimization => "optimization",
            Self::Diagnostic => "diagnostic",
            Self::Maintenance => "maintenance",
            Self::Emergency => "emergency",
            Self::Routine => "routine",
        }
    }
}

/// Procedure step
#[derive(Debug, Clone)]
pub struct ProcedureStep {
    /// Step number
    pub number: u32,
    /// Action to take
    pub action: String,
    /// Expected outcome
    pub expected_outcome: String,
    /// Timeout (nanoseconds)
    pub timeout_ns: u64,
    /// Is optional
    pub is_optional: bool,
    /// Fallback step (if this fails)
    pub fallback: Option<u32>,
}

impl ProcedureStep {
    /// Create new step
    pub fn new(number: u32, action: String) -> Self {
        Self {
            number,
            action,
            expected_outcome: String::new(),
            timeout_ns: 1_000_000_000, // 1 second default
            is_optional: false,
            fallback: None,
        }
    }

    /// Set expected outcome
    pub fn with_outcome(mut self, outcome: String) -> Self {
        self.expected_outcome = outcome;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout_ns = timeout_ns;
        self
    }

    /// Set as optional
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// Set fallback step
    pub fn with_fallback(mut self, step: u32) -> Self {
        self.fallback = Some(step);
        self
    }
}

/// Procedure
#[derive(Debug, Clone)]
pub struct Procedure {
    /// Procedure ID
    pub id: ProcedureId,
    /// Name
    pub name: String,
    /// Procedure type
    pub procedure_type: ProcedureType,
    /// Description
    pub description: String,
    /// Preconditions
    pub preconditions: Vec<String>,
    /// Steps
    pub steps: Vec<ProcedureStep>,
    /// Postconditions
    pub postconditions: Vec<String>,
    /// Success count
    pub success_count: u64,
    /// Failure count
    pub failure_count: u64,
    /// Average duration (nanoseconds)
    pub avg_duration_ns: u64,
    /// Associated patterns
    pub patterns: Vec<PatternId>,
}

impl Procedure {
    /// Create new procedure
    pub fn new(id: ProcedureId, name: String, procedure_type: ProcedureType) -> Self {
        Self {
            id,
            name,
            procedure_type,
            description: String::new(),
            preconditions: Vec::new(),
            steps: Vec::new(),
            postconditions: Vec::new(),
            success_count: 0,
            failure_count: 0,
            avg_duration_ns: 0,
            patterns: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Add step
    pub fn add_step(&mut self, step: ProcedureStep) {
        self.steps.push(step);
    }

    /// Add precondition
    pub fn add_precondition(&mut self, condition: String) {
        self.preconditions.push(condition);
    }

    /// Add postcondition
    pub fn add_postcondition(&mut self, condition: String) {
        self.postconditions.push(condition);
    }

    /// Link to pattern
    pub fn link_pattern(&mut self, pattern_id: PatternId) {
        if !self.patterns.contains(&pattern_id) {
            self.patterns.push(pattern_id);
        }
    }

    /// Record execution
    pub fn record_execution(&mut self, success: bool, duration_ns: u64) {
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        // Update average duration (exponential moving average)
        if self.avg_duration_ns == 0 {
            self.avg_duration_ns = duration_ns;
        } else {
            self.avg_duration_ns = (self.avg_duration_ns * 9 + duration_ns) / 10;
        }
    }

    /// Success rate
    pub fn success_rate(&self) -> f32 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        self.success_count as f32 / total as f32
    }

    /// Total executions
    pub fn total_executions(&self) -> u64 {
        self.success_count + self.failure_count
    }
}

/// Procedural memory store
#[derive(Debug)]
pub struct ProceduralMemory {
    /// Procedures
    procedures: BTreeMap<ProcedureId, Procedure>,
    /// Procedures by type
    by_type: BTreeMap<ProcedureType, Vec<ProcedureId>>,
    /// Procedures by name
    by_name: BTreeMap<String, ProcedureId>,
    /// Procedure counter
    counter: AtomicU64,
}

impl ProceduralMemory {
    /// Create new procedural memory
    pub fn new() -> Self {
        Self {
            procedures: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_name: BTreeMap::new(),
            counter: AtomicU64::new(0),
        }
    }

    /// Create procedure
    pub fn create_procedure(&mut self, name: String, procedure_type: ProcedureType) -> ProcedureId {
        // Check if exists
        if let Some(&id) = self.by_name.get(&name) {
            return id;
        }

        let id = ProcedureId(self.counter.fetch_add(1, Ordering::Relaxed));
        let procedure = Procedure::new(id, name.clone(), procedure_type);

        self.by_name.insert(name, id);
        self.by_type
            .entry(procedure_type)
            .or_insert_with(Vec::new)
            .push(id);
        self.procedures.insert(id, procedure);

        id
    }

    /// Get procedure
    pub fn get(&self, id: ProcedureId) -> Option<&Procedure> {
        self.procedures.get(&id)
    }

    /// Get procedure mutably
    pub fn get_mut(&mut self, id: ProcedureId) -> Option<&mut Procedure> {
        self.procedures.get_mut(&id)
    }

    /// Find by name
    pub fn find_by_name(&self, name: &str) -> Option<&Procedure> {
        self.by_name
            .get(name)
            .and_then(|id| self.procedures.get(id))
    }

    /// Find by type
    pub fn find_by_type(&self, procedure_type: ProcedureType) -> Vec<&Procedure> {
        self.by_type
            .get(&procedure_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.procedures.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find best procedure for pattern
    pub fn find_for_pattern(&self, pattern_id: PatternId) -> Option<&Procedure> {
        // Find procedure linked to this pattern with best success rate
        self.procedures
            .values()
            .filter(|p| p.patterns.contains(&pattern_id) && p.success_rate() > 0.5)
            .max_by(|a, b| {
                a.success_rate()
                    .partial_cmp(&b.success_rate())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Find best procedure overall
    pub fn find_best(&self) -> Option<&Procedure> {
        self.procedures
            .values()
            .filter(|p| p.success_rate() > 0.5)
            .max_by(|a, b| {
                a.success_rate()
                    .partial_cmp(&b.success_rate())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Procedure count
    pub fn count(&self) -> usize {
        self.procedures.len()
    }
}

impl Default for ProceduralMemory {
    fn default() -> Self {
        Self::new()
    }
}

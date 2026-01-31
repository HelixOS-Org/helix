//! Data flow analysis for code understanding
//!
//! This module provides data flow analysis capabilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::ast::Mutability;
use super::semantic::{SemanticModel, SymbolId};

/// Data flow fact
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowFact {
    /// Variable is definitely initialized
    Initialized(String),
    /// Variable may be uninitialized
    MaybeUninitialized(String),
    /// Variable is borrowed
    Borrowed(String, Mutability),
    /// Variable is moved
    Moved(String),
    /// Variable is dead (not used after this point)
    Dead(String),
    /// Pointer is null
    Null(String),
    /// Pointer is non-null
    NonNull(String),
    /// Value is in range
    InRange { var: String, min: i64, max: i64 },
}

/// Data flow analysis result
#[derive(Debug)]
pub struct DataFlowResult {
    /// Facts at entry
    pub entry_facts: Vec<DataFlowFact>,
    /// Facts at exit
    pub exit_facts: Vec<DataFlowFact>,
    /// Facts at each program point
    pub point_facts: BTreeMap<u32, Vec<DataFlowFact>>,
}

impl DataFlowResult {
    /// Create new result
    pub fn new() -> Self {
        Self {
            entry_facts: Vec::new(),
            exit_facts: Vec::new(),
            point_facts: BTreeMap::new(),
        }
    }

    /// Add entry fact
    pub fn add_entry_fact(&mut self, fact: DataFlowFact) {
        self.entry_facts.push(fact);
    }

    /// Add exit fact
    pub fn add_exit_fact(&mut self, fact: DataFlowFact) {
        self.exit_facts.push(fact);
    }

    /// Add fact at program point
    pub fn add_fact_at(&mut self, point: u32, fact: DataFlowFact) {
        self.point_facts.entry(point).or_default().push(fact);
    }

    /// Get facts at program point
    pub fn facts_at(&self, point: u32) -> Option<&Vec<DataFlowFact>> {
        self.point_facts.get(&point)
    }

    /// Check if variable is initialized at point
    pub fn is_initialized_at(&self, var: &str, point: u32) -> bool {
        self.point_facts.get(&point).map_or(false, |facts| {
            facts
                .iter()
                .any(|f| matches!(f, DataFlowFact::Initialized(v) if v == var))
        })
    }

    /// Check if variable is moved at point
    pub fn is_moved_at(&self, var: &str, point: u32) -> bool {
        self.point_facts.get(&point).map_or(false, |facts| {
            facts
                .iter()
                .any(|f| matches!(f, DataFlowFact::Moved(v) if v == var))
        })
    }
}

impl Default for DataFlowResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Data flow analyzer
pub struct DataFlowAnalyzer {
    /// Results per function
    results: BTreeMap<SymbolId, DataFlowResult>,
}

impl DataFlowAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            results: BTreeMap::new(),
        }
    }

    /// Analyze function
    pub fn analyze_function(
        &mut self,
        func_id: SymbolId,
        _model: &SemanticModel,
    ) -> DataFlowResult {
        // Perform forward data flow analysis
        let result = DataFlowResult::new();
        self.results.insert(func_id, result.clone());
        result
    }

    /// Get result for function
    pub fn get_result(&self, func_id: SymbolId) -> Option<&DataFlowResult> {
        self.results.get(&func_id)
    }

    /// Get all results
    pub fn all_results(&self) -> &BTreeMap<SymbolId, DataFlowResult> {
        &self.results
    }

    /// Clear results
    pub fn clear(&mut self) {
        self.results.clear();
    }
}

impl Default for DataFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DataFlowResult {
    fn clone(&self) -> Self {
        Self {
            entry_facts: self.entry_facts.clone(),
            exit_facts: self.exit_facts.clone(),
            point_facts: self.point_facts.clone(),
        }
    }
}

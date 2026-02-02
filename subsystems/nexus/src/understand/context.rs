//! # Context Understanding
//!
//! Manages and understands context for code analysis.
//! Implements context tracking and scope management.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CONTEXT TYPES
// ============================================================================

/// Context
#[derive(Debug, Clone)]
pub struct Context {
    /// Context ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub context_type: ContextType,
    /// Parent
    pub parent: Option<u64>,
    /// Variables
    pub variables: BTreeMap<String, ContextValue>,
    /// Imports
    pub imports: Vec<Import>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Created
    pub created: Timestamp,
}

/// Context type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextType {
    Global,
    Module,
    Function,
    Block,
    Class,
    Loop,
    Conditional,
    Try,
    With,
}

/// Context value
#[derive(Debug, Clone)]
pub struct ContextValue {
    /// Value ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub value_type: Option<String>,
    /// Value
    pub value: Option<ValueData>,
    /// Mutable
    pub mutable: bool,
    /// Defined at
    pub defined_at: Location,
}

/// Value data
#[derive(Debug, Clone)]
pub enum ValueData {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<ValueData>),
    Map(BTreeMap<String, ValueData>),
    Ref(u64),
    Unknown,
}

/// Location
#[derive(Debug, Clone)]
pub struct Location {
    /// File
    pub file: String,
    /// Line
    pub line: u32,
    /// Column
    pub column: u32,
}

/// Import
#[derive(Debug, Clone)]
pub struct Import {
    /// Import ID
    pub id: u64,
    /// Module path
    pub module: String,
    /// Items
    pub items: Vec<ImportItem>,
    /// Alias
    pub alias: Option<String>,
}

/// Import item
#[derive(Debug, Clone)]
pub struct ImportItem {
    /// Name
    pub name: String,
    /// Alias
    pub alias: Option<String>,
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Constraint ID
    pub id: u64,
    /// Variable
    pub variable: String,
    /// Type
    pub constraint_type: ConstraintType,
    /// Description
    pub description: String,
}

/// Constraint type
#[derive(Debug, Clone)]
pub enum ConstraintType {
    TypeBound { bound: String },
    ValueRange { min: Option<f64>, max: Option<f64> },
    NotNull,
    Initialized,
    Lifetime { lifetime: String },
    Custom { name: String },
}

/// Context query
#[derive(Debug, Clone, Default)]
pub struct ContextQuery {
    /// Variable name
    pub variable: Option<String>,
    /// Context type
    pub context_type: Option<ContextType>,
    /// Include parents
    pub include_parents: bool,
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Found values
    pub values: Vec<ContextValue>,
    /// Context chain
    pub context_chain: Vec<u64>,
}

// ============================================================================
// CONTEXT MANAGER
// ============================================================================

/// Context manager
pub struct ContextManager {
    /// Contexts
    contexts: BTreeMap<u64, Context>,
    /// Active context
    active_context: u64,
    /// Context stack
    context_stack: Vec<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ContextConfig,
    /// Statistics
    stats: ContextStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum nesting
    pub max_nesting: usize,
    /// Track values
    pub track_values: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_nesting: 50,
            track_values: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ContextStats {
    /// Contexts created
    pub contexts_created: u64,
    /// Variables defined
    pub variables_defined: u64,
    /// Lookups performed
    pub lookups: u64,
}

impl ContextManager {
    /// Create new manager
    pub fn new(config: ContextConfig) -> Self {
        let mut manager = Self {
            contexts: BTreeMap::new(),
            active_context: 0,
            context_stack: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ContextStats::default(),
        };

        // Create global context
        let global_id = manager.create_context("global", ContextType::Global, None);
        manager.active_context = global_id;
        manager.context_stack.push(global_id);

        manager
    }

    /// Create context
    pub fn create_context(
        &mut self,
        name: &str,
        context_type: ContextType,
        parent: Option<u64>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let context = Context {
            id,
            name: name.into(),
            context_type,
            parent,
            variables: BTreeMap::new(),
            imports: Vec::new(),
            constraints: Vec::new(),
            created: Timestamp::now(),
        };

        self.contexts.insert(id, context);
        self.stats.contexts_created += 1;

        id
    }

    /// Enter context
    pub fn enter(&mut self, name: &str, context_type: ContextType) -> u64 {
        if self.context_stack.len() >= self.config.max_nesting {
            return self.active_context;
        }

        let parent = Some(self.active_context);
        let id = self.create_context(name, context_type, parent);

        self.context_stack.push(id);
        self.active_context = id;

        id
    }

    /// Exit context
    pub fn exit(&mut self) -> Option<u64> {
        if self.context_stack.len() <= 1 {
            return None;
        }

        let exited = self.context_stack.pop()?;
        self.active_context = *self.context_stack.last()?;

        Some(exited)
    }

    /// Define variable
    pub fn define(
        &mut self,
        name: &str,
        value_type: Option<&str>,
        mutable: bool,
        location: Location,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let value = ContextValue {
            id,
            name: name.into(),
            value_type: value_type.map(|s| s.into()),
            value: None,
            mutable,
            defined_at: location,
        };

        if let Some(context) = self.contexts.get_mut(&self.active_context) {
            context.variables.insert(name.into(), value);
        }

        self.stats.variables_defined += 1;

        id
    }

    /// Set variable value
    pub fn set_value(&mut self, name: &str, value: ValueData) {
        if let Some(context) = self.contexts.get_mut(&self.active_context) {
            if let Some(var) = context.variables.get_mut(name) {
                if var.mutable || var.value.is_none() {
                    var.value = Some(value);
                }
            }
        }
    }

    /// Lookup variable
    pub fn lookup(&mut self, name: &str) -> Option<ContextValue> {
        self.stats.lookups += 1;

        let mut current = Some(self.active_context);

        while let Some(context_id) = current {
            if let Some(context) = self.contexts.get(&context_id) {
                if let Some(var) = context.variables.get(name) {
                    return Some(var.clone());
                }
                current = context.parent;
            } else {
                current = None;
            }
        }

        None
    }

    /// Add import
    pub fn add_import(&mut self, module: &str, items: Vec<ImportItem>, alias: Option<&str>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let import = Import {
            id,
            module: module.into(),
            items,
            alias: alias.map(|s| s.into()),
        };

        if let Some(context) = self.contexts.get_mut(&self.active_context) {
            context.imports.push(import);
        }

        id
    }

    /// Add constraint
    pub fn add_constraint(&mut self, variable: &str, constraint_type: ConstraintType, description: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let constraint = Constraint {
            id,
            variable: variable.into(),
            constraint_type,
            description: description.into(),
        };

        if let Some(context) = self.contexts.get_mut(&self.active_context) {
            context.constraints.push(constraint);
        }

        id
    }

    /// Query context
    pub fn query(&mut self, query: &ContextQuery) -> QueryResult {
        self.stats.lookups += 1;

        let mut values = Vec::new();
        let mut context_chain = Vec::new();

        let mut current = Some(self.active_context);

        while let Some(context_id) = current {
            context_chain.push(context_id);

            if let Some(context) = self.contexts.get(&context_id) {
                // Check context type filter
                if let Some(ref ct) = query.context_type {
                    if context.context_type != *ct {
                        if query.include_parents {
                            current = context.parent;
                            continue;
                        } else {
                            break;
                        }
                    }
                }

                // Find variables
                if let Some(ref var_name) = query.variable {
                    if let Some(var) = context.variables.get(var_name) {
                        values.push(var.clone());
                    }
                } else {
                    // Return all variables
                    values.extend(context.variables.values().cloned());
                }

                if !query.include_parents {
                    break;
                }

                current = context.parent;
            } else {
                current = None;
            }
        }

        QueryResult {
            values,
            context_chain,
        }
    }

    /// Get context
    pub fn get_context(&self, id: u64) -> Option<&Context> {
        self.contexts.get(&id)
    }

    /// Get active context
    pub fn active(&self) -> &Context {
        self.contexts.get(&self.active_context).unwrap()
    }

    /// Get nesting depth
    pub fn depth(&self) -> usize {
        self.context_stack.len()
    }

    /// Get context path
    pub fn path(&self) -> Vec<String> {
        self.context_stack.iter()
            .filter_map(|id| self.contexts.get(id))
            .map(|c| c.name.clone())
            .collect()
    }

    /// Get variables in scope
    pub fn variables_in_scope(&self) -> Vec<&ContextValue> {
        let mut variables = Vec::new();
        let mut current = Some(self.active_context);

        while let Some(context_id) = current {
            if let Some(context) = self.contexts.get(&context_id) {
                variables.extend(context.variables.values());
                current = context.parent;
            } else {
                current = None;
            }
        }

        variables
    }

    /// Get statistics
    pub fn stats(&self) -> &ContextStats {
        &self.stats
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(ContextConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_location() -> Location {
        Location {
            file: "test.rs".into(),
            line: 1,
            column: 1,
        }
    }

    #[test]
    fn test_create_context() {
        let mut manager = ContextManager::default();

        let id = manager.create_context("test", ContextType::Function, Some(1));
        assert!(manager.get_context(id).is_some());
    }

    #[test]
    fn test_enter_exit() {
        let mut manager = ContextManager::default();

        assert_eq!(manager.depth(), 1); // Global

        manager.enter("func", ContextType::Function);
        assert_eq!(manager.depth(), 2);

        manager.exit();
        assert_eq!(manager.depth(), 1);
    }

    #[test]
    fn test_define_variable() {
        let mut manager = ContextManager::default();

        manager.define("x", Some("i32"), true, test_location());

        let var = manager.lookup("x");
        assert!(var.is_some());
    }

    #[test]
    fn test_set_value() {
        let mut manager = ContextManager::default();

        manager.define("x", Some("i32"), true, test_location());
        manager.set_value("x", ValueData::Int(42));

        let var = manager.lookup("x").unwrap();
        assert!(matches!(var.value, Some(ValueData::Int(42))));
    }

    #[test]
    fn test_lookup_parent() {
        let mut manager = ContextManager::default();

        manager.define("global_var", Some("i32"), false, test_location());

        manager.enter("func", ContextType::Function);

        // Should find in parent
        let var = manager.lookup("global_var");
        assert!(var.is_some());
    }

    #[test]
    fn test_shadowing() {
        let mut manager = ContextManager::default();

        manager.define("x", Some("i32"), false, test_location());
        manager.set_value("x", ValueData::Int(10));

        manager.enter("block", ContextType::Block);
        manager.define("x", Some("i32"), false, test_location());
        manager.set_value("x", ValueData::Int(20));

        // Should find local
        let var = manager.lookup("x").unwrap();
        assert!(matches!(var.value, Some(ValueData::Int(20))));
    }

    #[test]
    fn test_add_import() {
        let mut manager = ContextManager::default();

        let id = manager.add_import(
            "std::collections",
            vec![ImportItem { name: "HashMap".into(), alias: None }],
            None,
        );

        assert!(id > 0);
    }

    #[test]
    fn test_path() {
        let mut manager = ContextManager::default();

        manager.enter("module", ContextType::Module);
        manager.enter("function", ContextType::Function);

        let path = manager.path();
        assert_eq!(path.len(), 3);
        assert_eq!(path[2], "function");
    }

    #[test]
    fn test_variables_in_scope() {
        let mut manager = ContextManager::default();

        manager.define("a", None, false, test_location());
        manager.enter("func", ContextType::Function);
        manager.define("b", None, false, test_location());

        let vars = manager.variables_in_scope();
        assert_eq!(vars.len(), 2);
    }
}

//! # Scope Analysis
//!
//! Analyzes variable scopes and bindings in code.
//! Tracks definitions, references, and lifetimes.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SCOPE TYPES
// ============================================================================

/// Scope
#[derive(Debug, Clone)]
pub struct Scope {
    /// Scope ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Scope type
    pub scope_type: ScopeType,
    /// Parent scope
    pub parent: Option<u64>,
    /// Children scopes
    pub children: Vec<u64>,
    /// Bindings in this scope
    pub bindings: BTreeMap<String, Binding>,
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

/// Scope type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeType {
    Global,
    Module,
    Function,
    Block,
    Loop,
    Conditional,
    Match,
    Closure,
    Impl,
    Trait,
}

/// Binding
#[derive(Debug, Clone)]
pub struct Binding {
    /// Binding ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Binding kind
    pub kind: BindingKind,
    /// Type (if known)
    pub binding_type: Option<String>,
    /// Definition position
    pub definition: Position,
    /// References
    pub references: Vec<Reference>,
    /// Mutability
    pub mutable: bool,
}

/// Binding kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Variable,
    Constant,
    Function,
    Type,
    Struct,
    Enum,
    Trait,
    Module,
    Parameter,
    Field,
    Lifetime,
}

/// Reference
#[derive(Debug, Clone)]
pub struct Reference {
    /// Position
    pub position: Position,
    /// Reference type
    pub ref_type: ReferenceType,
}

/// Reference type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    Read,
    Write,
    Borrow,
    MutableBorrow,
    Move,
    Call,
}

/// Position
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    /// Line
    pub line: u32,
    /// Column
    pub column: u32,
}

/// Scope resolution result
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// Resolved binding
    pub binding: Option<Binding>,
    /// Scope where found
    pub scope: Option<u64>,
    /// Search path
    pub search_path: Vec<u64>,
}

// ============================================================================
// SCOPE ANALYZER
// ============================================================================

/// Scope analyzer
pub struct ScopeAnalyzer {
    /// Scopes
    scopes: BTreeMap<u64, Scope>,
    /// Current scope
    current_scope: Option<u64>,
    /// Root scope
    root_scope: Option<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ScopeConfig,
    /// Statistics
    stats: ScopeStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ScopeConfig {
    /// Track references
    pub track_references: bool,
    /// Maximum scope depth
    pub max_depth: usize,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            track_references: true,
            max_depth: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ScopeStats {
    /// Scopes created
    pub scopes_created: u64,
    /// Bindings tracked
    pub bindings_tracked: u64,
    /// Resolutions performed
    pub resolutions: u64,
}

impl ScopeAnalyzer {
    /// Create new analyzer
    pub fn new(config: ScopeConfig) -> Self {
        Self {
            scopes: BTreeMap::new(),
            current_scope: None,
            root_scope: None,
            next_id: AtomicU64::new(1),
            config,
            stats: ScopeStats::default(),
        }
    }

    /// Enter scope
    pub fn enter_scope(&mut self, name: &str, scope_type: ScopeType, start: Position) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let scope = Scope {
            id,
            name: name.into(),
            scope_type,
            parent: self.current_scope,
            children: Vec::new(),
            bindings: BTreeMap::new(),
            start,
            end: start, // Will be updated on exit
        };

        // Add as child to parent
        if let Some(parent_id) = self.current_scope {
            if let Some(parent) = self.scopes.get_mut(&parent_id) {
                parent.children.push(id);
            }
        }

        self.scopes.insert(id, scope);
        self.current_scope = Some(id);

        if self.root_scope.is_none() {
            self.root_scope = Some(id);
        }

        self.stats.scopes_created += 1;

        id
    }

    /// Exit scope
    #[inline]
    pub fn exit_scope(&mut self, end: Position) -> Option<u64> {
        let current_id = self.current_scope?;

        if let Some(scope) = self.scopes.get_mut(&current_id) {
            scope.end = end;
            self.current_scope = scope.parent;
        }

        Some(current_id)
    }

    /// Define binding
    pub fn define(&mut self, name: &str, kind: BindingKind, position: Position, mutable: bool) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let binding = Binding {
            id,
            name: name.into(),
            kind,
            binding_type: None,
            definition: position,
            references: Vec::new(),
            mutable,
        };

        if let Some(scope_id) = self.current_scope {
            if let Some(scope) = self.scopes.get_mut(&scope_id) {
                scope.bindings.insert(name.into(), binding);
            }
        }

        self.stats.bindings_tracked += 1;

        id
    }

    /// Define with type
    pub fn define_typed(
        &mut self,
        name: &str,
        kind: BindingKind,
        binding_type: &str,
        position: Position,
        mutable: bool,
    ) -> u64 {
        let id = self.define(name, kind, position, mutable);

        // Update type
        if let Some(scope_id) = self.current_scope {
            if let Some(scope) = self.scopes.get_mut(&scope_id) {
                if let Some(binding) = scope.bindings.get_mut(name) {
                    binding.binding_type = Some(binding_type.into());
                }
            }
        }

        id
    }

    /// Add reference
    pub fn add_reference(&mut self, name: &str, position: Position, ref_type: ReferenceType) {
        if !self.config.track_references {
            return;
        }

        // Find binding and add reference
        let mut scope_id = self.current_scope;

        while let Some(id) = scope_id {
            if let Some(scope) = self.scopes.get_mut(&id) {
                if let Some(binding) = scope.bindings.get_mut(name) {
                    binding.references.push(Reference { position, ref_type });
                    return;
                }
                scope_id = scope.parent;
            } else {
                break;
            }
        }
    }

    /// Resolve name
    pub fn resolve(&mut self, name: &str) -> ResolutionResult {
        self.stats.resolutions += 1;

        let mut search_path = Vec::new();
        let mut scope_id = self.current_scope;

        while let Some(id) = scope_id {
            search_path.push(id);

            if let Some(scope) = self.scopes.get(&id) {
                if let Some(binding) = scope.bindings.get(name) {
                    return ResolutionResult {
                        binding: Some(binding.clone()),
                        scope: Some(id),
                        search_path,
                    };
                }
                scope_id = scope.parent;
            } else {
                break;
            }
        }

        ResolutionResult {
            binding: None,
            scope: None,
            search_path,
        }
    }

    /// Get scope
    #[inline(always)]
    pub fn get_scope(&self, id: u64) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// Get current scope
    #[inline(always)]
    pub fn current(&self) -> Option<&Scope> {
        self.current_scope.and_then(|id| self.scopes.get(&id))
    }

    /// Get all bindings in scope chain
    pub fn visible_bindings(&self) -> Vec<&Binding> {
        let mut bindings = Vec::new();
        let mut seen = BTreeSet::new();
        let mut scope_id = self.current_scope;

        while let Some(id) = scope_id {
            if let Some(scope) = self.scopes.get(&id) {
                for (name, binding) in &scope.bindings {
                    if !seen.contains(name) {
                        seen.insert(name.clone());
                        bindings.push(binding);
                    }
                }
                scope_id = scope.parent;
            } else {
                break;
            }
        }

        bindings
    }

    /// Find unused bindings
    pub fn find_unused(&self) -> Vec<(&Scope, &Binding)> {
        let mut unused = Vec::new();

        for scope in self.scopes.values() {
            for binding in scope.bindings.values() {
                if binding.references.is_empty() &&
                   binding.kind != BindingKind::Parameter {
                    unused.push((scope, binding));
                }
            }
        }

        unused
    }

    /// Find shadowed bindings
    pub fn find_shadowed(&self, scope_id: u64) -> Vec<(&Binding, u64)> {
        let mut shadowed = Vec::new();

        let scope = match self.scopes.get(&scope_id) {
            Some(s) => s,
            None => return shadowed,
        };

        for (name, binding) in &scope.bindings {
            // Check parent scopes
            let mut parent_id = scope.parent;

            while let Some(pid) = parent_id {
                if let Some(parent) = self.scopes.get(&pid) {
                    if parent.bindings.contains_key(name) {
                        shadowed.push((binding, pid));
                        break;
                    }
                    parent_id = parent.parent;
                } else {
                    break;
                }
            }
        }

        shadowed
    }

    /// Get scope depth
    pub fn depth(&self, scope_id: u64) -> usize {
        let mut depth = 0;
        let mut current = Some(scope_id);

        while let Some(id) = current {
            if let Some(scope) = self.scopes.get(&id) {
                depth += 1;
                current = scope.parent;
            } else {
                break;
            }
        }

        depth
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ScopeStats {
        &self.stats
    }
}

impl Default for ScopeAnalyzer {
    fn default() -> Self {
        Self::new(ScopeConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: u32, col: u32) -> Position {
        Position { line, column: col }
    }

    #[test]
    fn test_enter_exit_scope() {
        let mut analyzer = ScopeAnalyzer::default();

        let id = analyzer.enter_scope("test", ScopeType::Function, pos(1, 0));
        assert!(analyzer.current_scope.is_some());

        analyzer.exit_scope(pos(10, 0));
        assert!(analyzer.current_scope.is_none());

        let scope = analyzer.get_scope(id).unwrap();
        assert_eq!(scope.end.line, 10);
    }

    #[test]
    fn test_define() {
        let mut analyzer = ScopeAnalyzer::default();

        analyzer.enter_scope("fn", ScopeType::Function, pos(1, 0));
        analyzer.define("x", BindingKind::Variable, pos(2, 4), true);

        let current = analyzer.current().unwrap();
        assert!(current.bindings.contains_key("x"));
    }

    #[test]
    fn test_resolve() {
        let mut analyzer = ScopeAnalyzer::default();

        analyzer.enter_scope("outer", ScopeType::Function, pos(1, 0));
        analyzer.define("x", BindingKind::Variable, pos(2, 4), false);

        analyzer.enter_scope("inner", ScopeType::Block, pos(3, 0));

        let result = analyzer.resolve("x");
        assert!(result.binding.is_some());
    }

    #[test]
    fn test_shadowing() {
        let mut analyzer = ScopeAnalyzer::default();

        let outer = analyzer.enter_scope("outer", ScopeType::Function, pos(1, 0));
        analyzer.define("x", BindingKind::Variable, pos(2, 4), false);

        let inner = analyzer.enter_scope("inner", ScopeType::Block, pos(3, 0));
        analyzer.define("x", BindingKind::Variable, pos(4, 4), true);

        let shadowed = analyzer.find_shadowed(inner);
        assert_eq!(shadowed.len(), 1);
    }

    #[test]
    fn test_references() {
        let mut analyzer = ScopeAnalyzer::default();

        analyzer.enter_scope("fn", ScopeType::Function, pos(1, 0));
        analyzer.define("x", BindingKind::Variable, pos(2, 4), true);
        analyzer.add_reference("x", pos(3, 4), ReferenceType::Read);
        analyzer.add_reference("x", pos(4, 4), ReferenceType::Write);

        let result = analyzer.resolve("x");
        let binding = result.binding.unwrap();

        assert_eq!(binding.references.len(), 2);
    }

    #[test]
    fn test_unused() {
        let mut analyzer = ScopeAnalyzer::default();

        analyzer.enter_scope("fn", ScopeType::Function, pos(1, 0));
        analyzer.define("used", BindingKind::Variable, pos(2, 4), false);
        analyzer.add_reference("used", pos(3, 4), ReferenceType::Read);
        analyzer.define("unused", BindingKind::Variable, pos(4, 4), false);

        let unused = analyzer.find_unused();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].1.name, "unused");
    }

    #[test]
    fn test_visible_bindings() {
        let mut analyzer = ScopeAnalyzer::default();

        analyzer.enter_scope("outer", ScopeType::Function, pos(1, 0));
        analyzer.define("x", BindingKind::Variable, pos(2, 4), false);

        analyzer.enter_scope("inner", ScopeType::Block, pos(3, 0));
        analyzer.define("y", BindingKind::Variable, pos(4, 4), false);

        let visible = analyzer.visible_bindings();
        assert_eq!(visible.len(), 2);
    }
}

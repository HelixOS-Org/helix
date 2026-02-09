//! Semantic model for code understanding
//!
//! This module provides semantic analysis and symbol resolution.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::ast::{TypeRef, Visibility};
use super::token::Span;

/// Symbol ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u64);

impl SymbolId {
    /// Create new symbol ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Function
    Function,
    /// Struct
    Struct,
    /// Enum
    Enum,
    /// Trait
    Trait,
    /// TypeAlias
    TypeAlias,
    /// Const
    Const,
    /// Static
    Static,
    /// Module
    Module,
    /// Field
    Field,
    /// Variant
    Variant,
    /// Local variable
    Local,
    /// Parameter
    Parameter,
    /// Lifetime
    Lifetime,
    /// TypeParam
    TypeParam,
    /// ConstParam
    ConstParam,
    /// Macro
    Macro,
    /// Label
    Label,
}

/// Symbol
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol ID
    pub id: SymbolId,
    /// Name
    pub name: String,
    /// Kind
    pub kind: SymbolKind,
    /// Visibility
    pub visibility: Visibility,
    /// Span
    pub span: Span,
    /// Parent scope
    pub parent: Option<SymbolId>,
    /// Type (if applicable)
    pub ty: Option<TypeRef>,
    /// Documentation
    pub docs: Option<String>,
}

impl Symbol {
    /// Create new symbol
    pub fn new(id: SymbolId, name: String, kind: SymbolKind, span: Span) -> Self {
        Self {
            id,
            name,
            kind,
            visibility: Visibility::Private,
            span,
            parent: None,
            ty: None,
            docs: None,
        }
    }
}

/// Scope
#[derive(Debug)]
pub struct Scope {
    /// Scope ID
    pub id: SymbolId,
    /// Parent scope
    pub parent: Option<SymbolId>,
    /// Symbols in scope
    pub symbols: BTreeMap<String, SymbolId>,
    /// Child scopes
    pub children: Vec<SymbolId>,
}

impl Scope {
    /// Create new scope
    pub fn new(id: SymbolId, parent: Option<SymbolId>) -> Self {
        Self {
            id,
            parent,
            symbols: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    /// Add symbol
    #[inline(always)]
    pub fn add_symbol(&mut self, name: String, id: SymbolId) {
        self.symbols.insert(name, id);
    }

    /// Lookup symbol
    #[inline(always)]
    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.symbols.get(name).copied()
    }
}

/// Semantic model
#[derive(Debug)]
pub struct SemanticModel {
    /// Symbols
    pub symbols: BTreeMap<SymbolId, Symbol>,
    /// Scopes
    pub scopes: BTreeMap<SymbolId, Scope>,
    /// Root scope
    pub root_scope: SymbolId,
    /// Symbol counter
    symbol_counter: AtomicU64,
}

impl SemanticModel {
    /// Create new semantic model
    pub fn new() -> Self {
        let root_id = SymbolId::new(0);
        let mut scopes = BTreeMap::new();
        scopes.insert(root_id, Scope::new(root_id, None));

        Self {
            symbols: BTreeMap::new(),
            scopes,
            root_scope: root_id,
            symbol_counter: AtomicU64::new(1),
        }
    }

    /// Create symbol
    #[inline]
    pub fn create_symbol(&mut self, name: String, kind: SymbolKind, span: Span) -> SymbolId {
        let id = SymbolId(self.symbol_counter.fetch_add(1, Ordering::Relaxed));
        let symbol = Symbol::new(id, name, kind, span);
        self.symbols.insert(id, symbol);
        id
    }

    /// Get symbol
    #[inline(always)]
    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    /// Get symbol mutably
    #[inline(always)]
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(&id)
    }

    /// Create scope
    pub fn create_scope(&mut self, parent: Option<SymbolId>) -> SymbolId {
        let id = SymbolId(self.symbol_counter.fetch_add(1, Ordering::Relaxed));
        let scope = Scope::new(id, parent);
        self.scopes.insert(id, scope);

        if let Some(parent_id) = parent {
            if let Some(parent_scope) = self.scopes.get_mut(&parent_id) {
                parent_scope.children.push(id);
            }
        }

        id
    }

    /// Get scope
    #[inline(always)]
    pub fn get_scope(&self, id: SymbolId) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// Get scope mutably
    #[inline(always)]
    pub fn get_scope_mut(&mut self, id: SymbolId) -> Option<&mut Scope> {
        self.scopes.get_mut(&id)
    }

    /// Lookup symbol in scope chain
    pub fn lookup(&self, name: &str, scope_id: SymbolId) -> Option<SymbolId> {
        let mut current = Some(scope_id);

        while let Some(id) = current {
            if let Some(scope) = self.scopes.get(&id) {
                if let Some(symbol_id) = scope.lookup(name) {
                    return Some(symbol_id);
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Symbol count
    #[inline(always)]
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }
}

impl Default for SemanticModel {
    fn default() -> Self {
        Self::new()
    }
}

//! # Symbol Table
//!
//! Symbol table for code analysis with scope management.
//! Tracks variables, functions, types, and their properties.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// SYMBOL TYPES
// ============================================================================

/// Symbol
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol ID
    pub id: u64,
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Type
    pub symbol_type: Option<SymbolType>,
    /// Scope
    pub scope_id: u64,
    /// Definition location
    pub definition: Location,
    /// Visibility
    pub visibility: Visibility,
    /// Mutability
    pub mutability: Mutability,
    /// References
    pub references: Vec<Location>,
    /// Documentation
    pub doc: Option<String>,
    /// Attributes
    pub attributes: Vec<Attribute>,
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Constant,
    Function,
    Method,
    Parameter,
    Field,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Module,
    Macro,
    Label,
}

/// Symbol type
#[derive(Debug, Clone)]
pub enum SymbolType {
    /// Primitive type
    Primitive(String),
    /// Reference
    Reference { inner: Box<SymbolType>, mutable: bool },
    /// Array
    Array { element: Box<SymbolType>, size: Option<usize> },
    /// Tuple
    Tuple(Vec<SymbolType>),
    /// Function
    Function { params: Vec<SymbolType>, ret: Box<SymbolType> },
    /// Generic
    Generic { base: String, params: Vec<SymbolType> },
    /// User-defined
    UserDefined(String),
    /// Inferred (not yet resolved)
    Inferred,
    /// Unknown
    Unknown,
}

/// Location
#[derive(Debug, Clone, Default)]
pub struct Location {
    /// File path
    pub file: String,
    /// Line
    pub line: u32,
    /// Column
    pub column: u32,
    /// Offset
    pub offset: usize,
}

/// Visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Crate,
    Super,
}

/// Mutability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Mutable,
    Immutable,
    Const,
}

/// Attribute
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Name
    pub name: String,
    /// Value
    pub value: Option<String>,
}

// ============================================================================
// SCOPE
// ============================================================================

/// Scope
#[derive(Debug, Clone)]
pub struct Scope {
    /// Scope ID
    pub id: u64,
    /// Scope kind
    pub kind: ScopeKind,
    /// Parent scope
    pub parent: Option<u64>,
    /// Children
    pub children: Vec<u64>,
    /// Symbols in this scope
    pub symbols: BTreeMap<String, u64>,
    /// Name (for named scopes)
    pub name: Option<String>,
    /// Location
    pub location: Location,
}

/// Scope kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Module,
    Function,
    Block,
    Loop,
    Struct,
    Enum,
    Impl,
    Trait,
    Match,
    Closure,
}

// ============================================================================
// SYMBOL TABLE
// ============================================================================

/// Symbol table
pub struct SymbolTable {
    /// Symbols
    symbols: BTreeMap<u64, Symbol>,
    /// Scopes
    scopes: BTreeMap<u64, Scope>,
    /// Current scope
    current_scope: u64,
    /// Global scope
    global_scope: u64,
    /// Next ID
    next_id: AtomicU64,
    /// Symbol index by name (for global lookup)
    by_name: BTreeMap<String, Vec<u64>>,
    /// Statistics
    stats: SymbolTableStats,
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct SymbolTableStats {
    /// Total symbols
    pub total_symbols: u64,
    /// Total scopes
    pub total_scopes: u64,
    /// Symbols by kind
    pub by_kind: BTreeMap<SymbolKind, u64>,
    /// Max scope depth
    pub max_scope_depth: u32,
}

impl SymbolTable {
    /// Create new symbol table
    pub fn new() -> Self {
        let global_scope_id = 1;
        let mut scopes = BTreeMap::new();

        scopes.insert(global_scope_id, Scope {
            id: global_scope_id,
            kind: ScopeKind::Global,
            parent: None,
            children: Vec::new(),
            symbols: BTreeMap::new(),
            name: Some("global".into()),
            location: Location::default(),
        });

        Self {
            symbols: BTreeMap::new(),
            scopes,
            current_scope: global_scope_id,
            global_scope: global_scope_id,
            next_id: AtomicU64::new(2),
            by_name: BTreeMap::new(),
            stats: SymbolTableStats { total_scopes: 1, ..Default::default() },
        }
    }

    /// Enter new scope
    pub fn enter_scope(&mut self, kind: ScopeKind, name: Option<&str>, location: Location) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let scope = Scope {
            id,
            kind,
            parent: Some(self.current_scope),
            children: Vec::new(),
            symbols: BTreeMap::new(),
            name: name.map(String::from),
            location,
        };

        // Add as child of current scope
        if let Some(parent) = self.scopes.get_mut(&self.current_scope) {
            parent.children.push(id);
        }

        self.scopes.insert(id, scope);
        self.current_scope = id;
        self.stats.total_scopes += 1;

        // Update max depth
        let depth = self.scope_depth(id);
        if depth > self.stats.max_scope_depth {
            self.stats.max_scope_depth = depth;
        }

        id
    }

    /// Exit current scope
    pub fn exit_scope(&mut self) -> Option<u64> {
        let current = self.scopes.get(&self.current_scope)?;
        let parent = current.parent?;

        let old = self.current_scope;
        self.current_scope = parent;
        Some(old)
    }

    fn scope_depth(&self, scope_id: u64) -> u32 {
        let mut depth = 0;
        let mut current = scope_id;

        while let Some(scope) = self.scopes.get(&current) {
            if let Some(parent) = scope.parent {
                depth += 1;
                current = parent;
            } else {
                break;
            }
        }

        depth
    }

    /// Define symbol in current scope
    pub fn define(&mut self, name: &str, kind: SymbolKind, location: Location) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let symbol = Symbol {
            id,
            name: name.into(),
            kind,
            symbol_type: None,
            scope_id: self.current_scope,
            definition: location,
            visibility: Visibility::Private,
            mutability: Mutability::Immutable,
            references: Vec::new(),
            doc: None,
            attributes: Vec::new(),
        };

        // Add to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope) {
            scope.symbols.insert(name.into(), id);
        }

        // Add to name index
        self.by_name.entry(name.into())
            .or_insert_with(Vec::new)
            .push(id);

        // Update stats
        self.stats.total_symbols += 1;
        *self.stats.by_kind.entry(kind).or_insert(0) += 1;

        self.symbols.insert(id, symbol);
        id
    }

    /// Define with type
    pub fn define_typed(
        &mut self,
        name: &str,
        kind: SymbolKind,
        symbol_type: SymbolType,
        location: Location,
    ) -> u64 {
        let id = self.define(name, kind, location);
        if let Some(symbol) = self.symbols.get_mut(&id) {
            symbol.symbol_type = Some(symbol_type);
        }
        id
    }

    /// Lookup symbol in current scope chain
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut scope_id = self.current_scope;

        loop {
            let scope = self.scopes.get(&scope_id)?;

            if let Some(&symbol_id) = scope.symbols.get(name) {
                return self.symbols.get(&symbol_id);
            }

            scope_id = scope.parent?;
        }
    }

    /// Lookup in specific scope
    pub fn lookup_in_scope(&self, name: &str, scope_id: u64) -> Option<&Symbol> {
        let scope = self.scopes.get(&scope_id)?;
        let symbol_id = scope.symbols.get(name)?;
        self.symbols.get(symbol_id)
    }

    /// Get symbol by ID
    pub fn get_symbol(&self, id: u64) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    /// Get mutable symbol
    pub fn get_symbol_mut(&mut self, id: u64) -> Option<&mut Symbol> {
        self.symbols.get_mut(&id)
    }

    /// Get scope
    pub fn get_scope(&self, id: u64) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// Get current scope
    pub fn current_scope(&self) -> &Scope {
        self.scopes.get(&self.current_scope).unwrap()
    }

    /// Add reference to symbol
    pub fn add_reference(&mut self, symbol_id: u64, location: Location) {
        if let Some(symbol) = self.symbols.get_mut(&symbol_id) {
            symbol.references.push(location);
        }
    }

    /// Set symbol type
    pub fn set_type(&mut self, symbol_id: u64, symbol_type: SymbolType) {
        if let Some(symbol) = self.symbols.get_mut(&symbol_id) {
            symbol.symbol_type = Some(symbol_type);
        }
    }

    /// Set visibility
    pub fn set_visibility(&mut self, symbol_id: u64, visibility: Visibility) {
        if let Some(symbol) = self.symbols.get_mut(&symbol_id) {
            symbol.visibility = visibility;
        }
    }

    /// Set mutability
    pub fn set_mutability(&mut self, symbol_id: u64, mutability: Mutability) {
        if let Some(symbol) = self.symbols.get_mut(&symbol_id) {
            symbol.mutability = mutability;
        }
    }

    /// Find all symbols by name
    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.by_name.get(name)
            .map(|ids| ids.iter().filter_map(|id| self.symbols.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find symbols by kind
    pub fn find_by_kind(&self, kind: SymbolKind) -> Vec<&Symbol> {
        self.symbols.values()
            .filter(|s| s.kind == kind)
            .collect()
    }

    /// Get all symbols in scope
    pub fn symbols_in_scope(&self, scope_id: u64) -> Vec<&Symbol> {
        self.scopes.get(&scope_id)
            .map(|scope| {
                scope.symbols.values()
                    .filter_map(|id| self.symbols.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get accessible symbols (from current scope)
    pub fn accessible_symbols(&self) -> Vec<&Symbol> {
        let mut result = Vec::new();
        let mut scope_id = self.current_scope;

        loop {
            if let Some(scope) = self.scopes.get(&scope_id) {
                for &symbol_id in scope.symbols.values() {
                    if let Some(symbol) = self.symbols.get(&symbol_id) {
                        result.push(symbol);
                    }
                }
                if let Some(parent) = scope.parent {
                    scope_id = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// Check if symbol is in scope
    pub fn is_in_scope(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Get statistics
    pub fn stats(&self) -> &SymbolTableStats {
        &self.stats
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SYMBOL TABLE BUILDER
// ============================================================================

/// Symbol builder
pub struct SymbolBuilder {
    name: String,
    kind: SymbolKind,
    symbol_type: Option<SymbolType>,
    visibility: Visibility,
    mutability: Mutability,
    location: Location,
    doc: Option<String>,
    attributes: Vec<Attribute>,
}

impl SymbolBuilder {
    /// Create new builder
    pub fn new(name: &str, kind: SymbolKind) -> Self {
        Self {
            name: name.into(),
            kind,
            symbol_type: None,
            visibility: Visibility::Private,
            mutability: Mutability::Immutable,
            location: Location::default(),
            doc: None,
            attributes: Vec::new(),
        }
    }

    /// Set type
    pub fn typed(mut self, ty: SymbolType) -> Self {
        self.symbol_type = Some(ty);
        self
    }

    /// Set visibility
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    /// Set mutability
    pub fn mutable(mut self) -> Self {
        self.mutability = Mutability::Mutable;
        self
    }

    /// Set location
    pub fn at(mut self, file: &str, line: u32, column: u32) -> Self {
        self.location = Location {
            file: file.into(),
            line,
            column,
            offset: 0,
        };
        self
    }

    /// Add documentation
    pub fn doc(mut self, doc: &str) -> Self {
        self.doc = Some(doc.into());
        self
    }

    /// Build and insert
    pub fn build(self, table: &mut SymbolTable) -> u64 {
        let id = if let Some(ty) = self.symbol_type {
            table.define_typed(&self.name, self.kind, ty, self.location)
        } else {
            table.define(&self.name, self.kind, self.location)
        };

        table.set_visibility(id, self.visibility);
        table.set_mutability(id, self.mutability);

        if let Some(doc) = self.doc {
            if let Some(symbol) = table.get_symbol_mut(id) {
                symbol.doc = Some(doc);
            }
        }

        id
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_lookup() {
        let mut table = SymbolTable::new();

        table.define("x", SymbolKind::Variable, Location::default());

        let symbol = table.lookup("x");
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().name, "x");
    }

    #[test]
    fn test_scope_chain() {
        let mut table = SymbolTable::new();

        // Define in global
        table.define("global_var", SymbolKind::Variable, Location::default());

        // Enter function scope
        table.enter_scope(ScopeKind::Function, Some("test_fn"), Location::default());
        table.define("local_var", SymbolKind::Variable, Location::default());

        // Can see both
        assert!(table.lookup("global_var").is_some());
        assert!(table.lookup("local_var").is_some());

        // Exit scope
        table.exit_scope();

        // Can't see local anymore
        assert!(table.lookup("global_var").is_some());
        assert!(table.lookup("local_var").is_none());
    }

    #[test]
    fn test_shadowing() {
        let mut table = SymbolTable::new();

        let outer_id = table.define("x", SymbolKind::Variable, Location::default());

        table.enter_scope(ScopeKind::Block, None, Location::default());
        let inner_id = table.define("x", SymbolKind::Variable, Location::default());

        // Inner shadows outer
        let found = table.lookup("x").unwrap();
        assert_eq!(found.id, inner_id);

        table.exit_scope();

        // Outer is visible again
        let found = table.lookup("x").unwrap();
        assert_eq!(found.id, outer_id);
    }

    #[test]
    fn test_symbol_builder() {
        let mut table = SymbolTable::new();

        let id = SymbolBuilder::new("count", SymbolKind::Variable)
            .typed(SymbolType::Primitive("i32".into()))
            .visibility(Visibility::Public)
            .mutable()
            .at("test.rs", 10, 5)
            .doc("A counter")
            .build(&mut table);

        let symbol = table.get_symbol(id).unwrap();
        assert_eq!(symbol.name, "count");
        assert_eq!(symbol.visibility, Visibility::Public);
        assert_eq!(symbol.mutability, Mutability::Mutable);
    }

    #[test]
    fn test_find_by_kind() {
        let mut table = SymbolTable::new();

        table.define("a", SymbolKind::Variable, Location::default());
        table.define("b", SymbolKind::Variable, Location::default());
        table.define("foo", SymbolKind::Function, Location::default());

        let vars = table.find_by_kind(SymbolKind::Variable);
        assert_eq!(vars.len(), 2);

        let funcs = table.find_by_kind(SymbolKind::Function);
        assert_eq!(funcs.len(), 1);
    }
}

//! # Code Reflection
//!
//! Year 3 EVOLUTION - Introspection and reflection for self-modification

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// REFLECTION TYPES
// ============================================================================

/// Symbol ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u64);

static SYMBOL_COUNTER: AtomicU64 = AtomicU64::new(1);

impl SymbolId {
    pub fn generate() -> Self {
        Self(SYMBOL_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Module ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u64);

/// Type ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u64);

/// Symbol information
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// ID
    pub id: SymbolId,
    /// Name
    pub name: String,
    /// Fully qualified name
    pub full_name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Parent module
    pub module: ModuleId,
    /// Type
    pub type_id: Option<TypeId>,
    /// Visibility
    pub visibility: Visibility,
    /// Attributes
    pub attributes: Vec<Attribute>,
    /// Source location
    pub location: Option<SourceLocation>,
    /// Documentation
    pub doc: Option<String>,
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Constant,
    Static,
    Module,
    Impl,
    Macro,
}

/// Visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Crate,
    Super,
    Restricted(ModuleId),
}

/// Attribute
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Name
    pub name: String,
    /// Arguments
    pub args: Vec<AttributeArg>,
}

/// Attribute argument
#[derive(Debug, Clone)]
pub enum AttributeArg {
    Ident(String),
    Literal(String),
    NameValue(String, String),
    Nested(Attribute),
}

/// Source location
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// File path
    pub file: String,
    /// Start line
    pub start_line: u32,
    /// End line
    pub end_line: u32,
    /// Start column
    pub start_col: u32,
    /// End column
    pub end_col: u32,
}

// ============================================================================
// TYPE INFORMATION
// ============================================================================

/// Type information
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// ID
    pub id: TypeId,
    /// Name
    pub name: String,
    /// Kind
    pub kind: TypeKind,
    /// Size in bytes
    pub size: Option<usize>,
    /// Alignment
    pub align: Option<usize>,
    /// Fields (for struct/enum)
    pub fields: Vec<FieldInfo>,
    /// Methods
    pub methods: Vec<MethodInfo>,
    /// Implemented traits
    pub traits: Vec<TypeId>,
    /// Generic parameters
    pub generics: Vec<GenericParam>,
}

/// Type kind
#[derive(Debug, Clone)]
pub enum TypeKind {
    Primitive(PrimitiveType),
    Struct,
    Enum {
        variants: Vec<VariantInfo>,
    },
    Union,
    Trait,
    Tuple(Vec<TypeId>),
    Array {
        element: TypeId,
        len: usize,
    },
    Slice {
        element: TypeId,
    },
    Reference {
        target: TypeId,
        mutable: bool,
        lifetime: Option<String>,
    },
    Pointer {
        target: TypeId,
        mutable: bool,
    },
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Closure {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Generic {
        name: String,
    },
    Never,
}

/// Primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    Char,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    F32,
    F64,
    Str,
    Unit,
}

/// Field info
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Name (None for tuple fields)
    pub name: Option<String>,
    /// Type
    pub type_id: TypeId,
    /// Offset
    pub offset: Option<usize>,
    /// Visibility
    pub visibility: Visibility,
    /// Attributes
    pub attributes: Vec<Attribute>,
}

/// Variant info (for enums)
#[derive(Debug, Clone)]
pub struct VariantInfo {
    /// Name
    pub name: String,
    /// Discriminant
    pub discriminant: Option<i128>,
    /// Fields
    pub fields: Vec<FieldInfo>,
}

/// Method info
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Symbol
    pub symbol: SymbolId,
    /// Name
    pub name: String,
    /// Self parameter
    pub receiver: Option<ReceiverKind>,
    /// Parameters
    pub params: Vec<ParamInfo>,
    /// Return type
    pub return_type: TypeId,
    /// Is async
    pub is_async: bool,
    /// Is const
    pub is_const: bool,
    /// Is unsafe
    pub is_unsafe: bool,
}

/// Receiver kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverKind {
    SelfValue,
    SelfRef,
    SelfMutRef,
    SelfBox,
}

/// Parameter info
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Name
    pub name: String,
    /// Type
    pub type_id: TypeId,
    /// Is mutable
    pub mutable: bool,
}

/// Generic parameter
#[derive(Debug, Clone)]
pub struct GenericParam {
    /// Name
    pub name: String,
    /// Kind
    pub kind: GenericKind,
    /// Bounds
    pub bounds: Vec<TypeId>,
    /// Default
    pub default: Option<TypeId>,
}

/// Generic kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericKind {
    Type,
    Lifetime,
    Const,
}

// ============================================================================
// REFLECTION REGISTRY
// ============================================================================

/// Reflection registry
pub struct ReflectionRegistry {
    /// Symbols by ID
    symbols: BTreeMap<SymbolId, SymbolInfo>,
    /// Symbols by name
    symbols_by_name: BTreeMap<String, SymbolId>,
    /// Types by ID
    types: BTreeMap<TypeId, TypeInfo>,
    /// Types by name
    types_by_name: BTreeMap<String, TypeId>,
    /// Modules
    modules: BTreeMap<ModuleId, ModuleInfo>,
    /// Module children
    module_children: BTreeMap<ModuleId, Vec<SymbolId>>,
    /// Type counter
    type_counter: AtomicU64,
    /// Module counter
    module_counter: AtomicU64,
}

/// Module info
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// ID
    pub id: ModuleId,
    /// Name
    pub name: String,
    /// Full path
    pub path: String,
    /// Parent
    pub parent: Option<ModuleId>,
    /// Is crate root
    pub is_root: bool,
}

impl ReflectionRegistry {
    pub fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
            symbols_by_name: BTreeMap::new(),
            types: BTreeMap::new(),
            types_by_name: BTreeMap::new(),
            modules: BTreeMap::new(),
            module_children: BTreeMap::new(),
            type_counter: AtomicU64::new(1000), // Reserve low IDs for primitives
            module_counter: AtomicU64::new(1),
        }
    }

    /// Register a symbol
    pub fn register_symbol(&mut self, info: SymbolInfo) -> SymbolId {
        let id = info.id;
        self.symbols_by_name.insert(info.full_name.clone(), id);
        self.symbols.insert(id, info);

        id
    }

    /// Get symbol by ID
    pub fn get_symbol(&self, id: SymbolId) -> Option<&SymbolInfo> {
        self.symbols.get(&id)
    }

    /// Get symbol by name
    pub fn find_symbol(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols_by_name
            .get(name)
            .and_then(|id| self.symbols.get(id))
    }

    /// Register a type
    pub fn register_type(&mut self, info: TypeInfo) -> TypeId {
        let id = info.id;
        self.types_by_name.insert(info.name.clone(), id);
        self.types.insert(id, info);

        id
    }

    /// Generate type ID
    pub fn generate_type_id(&self) -> TypeId {
        TypeId(self.type_counter.fetch_add(1, Ordering::SeqCst))
    }

    /// Get type by ID
    pub fn get_type(&self, id: TypeId) -> Option<&TypeInfo> {
        self.types.get(&id)
    }

    /// Get type by name
    pub fn find_type(&self, name: &str) -> Option<&TypeInfo> {
        self.types_by_name
            .get(name)
            .and_then(|id| self.types.get(id))
    }

    /// Register module
    pub fn register_module(&mut self, info: ModuleInfo) -> ModuleId {
        let id = info.id;
        self.modules.insert(id, info);
        self.module_children.insert(id, Vec::new());

        id
    }

    /// Generate module ID
    pub fn generate_module_id(&self) -> ModuleId {
        ModuleId(self.module_counter.fetch_add(1, Ordering::SeqCst))
    }

    /// Get module
    pub fn get_module(&self, id: ModuleId) -> Option<&ModuleInfo> {
        self.modules.get(&id)
    }

    /// Add symbol to module
    pub fn add_to_module(&mut self, module: ModuleId, symbol: SymbolId) {
        self.module_children.entry(module).or_default().push(symbol);
    }

    /// Get module children
    pub fn module_symbols(&self, module: ModuleId) -> Option<&[SymbolId]> {
        self.module_children.get(&module).map(|v| v.as_slice())
    }

    /// Query symbols by kind
    pub fn find_by_kind(&self, kind: SymbolKind) -> Vec<&SymbolInfo> {
        self.symbols.values().filter(|s| s.kind == kind).collect()
    }

    /// Query symbols with attribute
    pub fn find_with_attribute(&self, attr_name: &str) -> Vec<&SymbolInfo> {
        self.symbols
            .values()
            .filter(|s| s.attributes.iter().any(|a| a.name == attr_name))
            .collect()
    }

    /// Get methods of type
    pub fn type_methods(&self, type_id: TypeId) -> Option<&[MethodInfo]> {
        self.types.get(&type_id).map(|t| t.methods.as_slice())
    }

    /// Get implemented traits
    pub fn implemented_traits(&self, type_id: TypeId) -> Option<&[TypeId]> {
        self.types.get(&type_id).map(|t| t.traits.as_slice())
    }

    /// Check if type implements trait
    pub fn implements_trait(&self, type_id: TypeId, trait_id: TypeId) -> bool {
        self.types
            .get(&type_id)
            .map(|t| t.traits.contains(&trait_id))
            .unwrap_or(false)
    }
}

impl Default for ReflectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// INTROSPECTOR
// ============================================================================

/// Code introspector
pub struct Introspector {
    /// Registry
    registry: ReflectionRegistry,
    /// Call graph
    call_graph: BTreeMap<SymbolId, Vec<SymbolId>>,
    /// Dependency graph
    dep_graph: BTreeMap<SymbolId, Vec<SymbolId>>,
    /// Metrics
    metrics: BTreeMap<SymbolId, SymbolMetrics>,
}

/// Symbol metrics
#[derive(Debug, Clone, Default)]
pub struct SymbolMetrics {
    /// Lines of code
    pub loc: usize,
    /// Cyclomatic complexity
    pub complexity: usize,
    /// Depth of nesting
    pub max_depth: usize,
    /// Number of parameters
    pub params: usize,
    /// Number of calls
    pub call_count: usize,
    /// Called by count
    pub called_by_count: usize,
    /// Dependencies count
    pub deps_count: usize,
}

impl Introspector {
    pub fn new() -> Self {
        Self {
            registry: ReflectionRegistry::new(),
            call_graph: BTreeMap::new(),
            dep_graph: BTreeMap::new(),
            metrics: BTreeMap::new(),
        }
    }

    /// Get registry
    pub fn registry(&self) -> &ReflectionRegistry {
        &self.registry
    }

    /// Get registry mut
    pub fn registry_mut(&mut self) -> &mut ReflectionRegistry {
        &mut self.registry
    }

    /// Add call edge
    pub fn add_call(&mut self, caller: SymbolId, callee: SymbolId) {
        self.call_graph.entry(caller).or_default().push(callee);

        // Update metrics
        self.metrics.entry(caller).or_default().call_count += 1;
        self.metrics.entry(callee).or_default().called_by_count += 1;
    }

    /// Add dependency edge
    pub fn add_dependency(&mut self, from: SymbolId, to: SymbolId) {
        self.dep_graph.entry(from).or_default().push(to);
        self.metrics.entry(from).or_default().deps_count += 1;
    }

    /// Get callees
    pub fn callees(&self, symbol: SymbolId) -> Option<&[SymbolId]> {
        self.call_graph.get(&symbol).map(|v| v.as_slice())
    }

    /// Get callers (reverse lookup)
    pub fn callers(&self, symbol: SymbolId) -> Vec<SymbolId> {
        self.call_graph
            .iter()
            .filter(|(_, callees)| callees.contains(&symbol))
            .map(|(&caller, _)| caller)
            .collect()
    }

    /// Get dependencies
    pub fn dependencies(&self, symbol: SymbolId) -> Option<&[SymbolId]> {
        self.dep_graph.get(&symbol).map(|v| v.as_slice())
    }

    /// Get dependents (reverse lookup)
    pub fn dependents(&self, symbol: SymbolId) -> Vec<SymbolId> {
        self.dep_graph
            .iter()
            .filter(|(_, deps)| deps.contains(&symbol))
            .map(|(&dependent, _)| dependent)
            .collect()
    }

    /// Set metrics
    pub fn set_metrics(&mut self, symbol: SymbolId, metrics: SymbolMetrics) {
        self.metrics.insert(symbol, metrics);
    }

    /// Get metrics
    pub fn get_metrics(&self, symbol: SymbolId) -> Option<&SymbolMetrics> {
        self.metrics.get(&symbol)
    }

    /// Find high complexity functions
    pub fn high_complexity(&self, threshold: usize) -> Vec<SymbolId> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.complexity > threshold)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Find highly coupled symbols
    pub fn high_coupling(&self, threshold: usize) -> Vec<SymbolId> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.deps_count > threshold || m.called_by_count > threshold)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Compute transitive closure of dependencies
    pub fn transitive_deps(&self, symbol: SymbolId) -> Vec<SymbolId> {
        let mut visited = Vec::new();
        let mut stack = vec![symbol];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            if let Some(deps) = self.dep_graph.get(&current) {
                for &dep in deps {
                    if !visited.contains(&dep) {
                        stack.push(dep);
                    }
                }
            }
        }

        visited.into_iter().filter(|&s| s != symbol).collect()
    }

    /// Detect cycles in call graph
    pub fn detect_cycles(&self) -> Vec<Vec<SymbolId>> {
        let mut cycles = Vec::new();
        let mut visited = BTreeMap::new();
        let mut stack = Vec::new();

        for &start in self.call_graph.keys() {
            self.detect_cycles_dfs(start, &mut visited, &mut stack, &mut cycles);
        }

        cycles
    }

    fn detect_cycles_dfs(
        &self,
        node: SymbolId,
        visited: &mut BTreeMap<SymbolId, bool>,
        stack: &mut Vec<SymbolId>,
        cycles: &mut Vec<Vec<SymbolId>>,
    ) {
        if let Some(&on_stack) = visited.get(&node) {
            if on_stack {
                // Found cycle
                if let Some(pos) = stack.iter().position(|&s| s == node) {
                    cycles.push(stack[pos..].to_vec());
                }
            }
            return;
        }

        visited.insert(node, true);
        stack.push(node);

        if let Some(callees) = self.call_graph.get(&node) {
            for &callee in callees {
                self.detect_cycles_dfs(callee, visited, stack, cycles);
            }
        }

        stack.pop();
        visited.insert(node, false);
    }
}

impl Default for Introspector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let mut registry = ReflectionRegistry::new();

        let module_id = registry.generate_module_id();
        registry.register_module(ModuleInfo {
            id: module_id,
            name: String::from("test"),
            path: String::from("::test"),
            parent: None,
            is_root: true,
        });

        let symbol = SymbolInfo {
            id: SymbolId::generate(),
            name: String::from("my_fn"),
            full_name: String::from("::test::my_fn"),
            kind: SymbolKind::Function,
            module: module_id,
            type_id: None,
            visibility: Visibility::Public,
            attributes: vec![],
            location: None,
            doc: Some(String::from("My function")),
        };

        let id = registry.register_symbol(symbol);
        let found = registry.find_symbol("::test::my_fn");

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn test_introspector() {
        let mut introspector = Introspector::new();

        let s1 = SymbolId(1);
        let s2 = SymbolId(2);
        let s3 = SymbolId(3);

        introspector.add_call(s1, s2);
        introspector.add_call(s2, s3);

        let callees = introspector.callees(s1);
        assert!(callees.is_some());
        assert!(callees.unwrap().contains(&s2));

        let callers = introspector.callers(s2);
        assert!(callers.contains(&s1));
    }

    #[test]
    fn test_cycle_detection() {
        let mut introspector = Introspector::new();

        let s1 = SymbolId(1);
        let s2 = SymbolId(2);
        let s3 = SymbolId(3);

        introspector.add_call(s1, s2);
        introspector.add_call(s2, s3);
        introspector.add_call(s3, s1); // Creates cycle

        let cycles = introspector.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_type_info() {
        let mut registry = ReflectionRegistry::new();

        let type_id = registry.generate_type_id();
        let type_info = TypeInfo {
            id: type_id,
            name: String::from("MyStruct"),
            kind: TypeKind::Struct,
            size: Some(16),
            align: Some(8),
            fields: vec![FieldInfo {
                name: Some(String::from("x")),
                type_id: TypeId(1),
                offset: Some(0),
                visibility: Visibility::Public,
                attributes: vec![],
            }],
            methods: vec![],
            traits: vec![],
            generics: vec![],
        };

        registry.register_type(type_info);
        let found = registry.find_type("MyStruct");

        assert!(found.is_some());
        assert_eq!(found.unwrap().fields.len(), 1);
    }
}

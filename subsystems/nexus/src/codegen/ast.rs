//! # Abstract Syntax Tree
//!
//! Year 3 EVOLUTION - Extended AST representation for code generation

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// NODE IDENTIFICATION
// ============================================================================

/// AST node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

static NODE_COUNTER: AtomicU64 = AtomicU64::new(1);

impl NodeId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(NODE_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Source location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Start offset
    pub start: u32,
    /// End offset
    pub end: u32,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
}

impl Span {
    pub fn new(start: u32, end: u32, line: u32, column: u32) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            column: if self.line <= other.line {
                self.column
            } else {
                other.column
            },
        }
    }
}

// ============================================================================
// AST NODES
// ============================================================================

/// AST node
#[derive(Debug, Clone)]
pub struct AstNode {
    /// Node ID
    pub id: NodeId,
    /// Node kind
    pub kind: AstKind,
    /// Span
    pub span: Span,
    /// Attributes
    pub attributes: Vec<Attribute>,
    /// Type annotation (if resolved)
    pub ty: Option<TypeId>,
}

impl AstNode {
    pub fn new(kind: AstKind) -> Self {
        Self {
            id: NodeId::generate(),
            kind,
            span: Span::default(),
            attributes: Vec::new(),
            ty: None,
        }
    }

    #[inline(always)]
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    #[inline(always)]
    pub fn with_type(mut self, ty: TypeId) -> Self {
        self.ty = Some(ty);
        self
    }
}

/// Type ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u64);

/// AST node kind
#[derive(Debug, Clone)]
pub enum AstKind {
    // ========== Module Level ==========
    /// Module
    Module(Module),
    /// Import
    Import(Import),
    /// Export
    Export(Export),

    // ========== Declarations ==========
    /// Function declaration
    Function(Function),
    /// Struct declaration
    Struct(StructDef),
    /// Enum declaration
    Enum(EnumDef),
    /// Trait declaration
    Trait(TraitDef),
    /// Implementation
    Impl(ImplBlock),
    /// Type alias
    TypeAlias(TypeAlias),
    /// Constant
    Const(ConstDef),
    /// Static
    Static(StaticDef),

    // ========== Statements ==========
    /// Let binding
    Let(LetStmt),
    /// Expression statement
    ExprStmt(Box<AstNode>),
    /// Return
    Return(Option<Box<AstNode>>),
    /// Break
    Break(Option<Box<AstNode>>),
    /// Continue
    Continue,
    /// Block
    Block(Block),

    // ========== Expressions ==========
    /// Literal
    Literal(Literal),
    /// Identifier
    Ident(String),
    /// Path
    Path(Path),
    /// Binary operation
    BinaryOp(BinaryOp),
    /// Unary operation
    UnaryOp(UnaryOp),
    /// Function call
    Call(Call),
    /// Method call
    MethodCall(MethodCall),
    /// Field access
    Field(FieldAccess),
    /// Index
    Index(Index),
    /// If expression
    If(IfExpr),
    /// Match expression
    Match(MatchExpr),
    /// Loop
    Loop(LoopExpr),
    /// While
    While(WhileExpr),
    /// For
    For(ForExpr),
    /// Closure
    Closure(Closure),
    /// Array
    Array(ArrayExpr),
    /// Tuple
    Tuple(TupleExpr),
    /// Struct instantiation
    StructExpr(StructExpr),
    /// Reference
    Ref(RefExpr),
    /// Dereference
    Deref(Box<AstNode>),
    /// Cast
    Cast(CastExpr),
    /// Try (?)
    Try(Box<AstNode>),
    /// Await
    Await(Box<AstNode>),
    /// Async block
    Async(Block),
    /// Unsafe block
    Unsafe(Block),

    // ========== Patterns ==========
    /// Pattern
    Pattern(Pattern),

    // ========== Types ==========
    /// Type expression
    Type(TypeExpr),
}

// ============================================================================
// MODULE LEVEL
// ============================================================================

/// Module
#[derive(Debug, Clone)]
pub struct Module {
    /// Name
    pub name: String,
    /// Items
    pub items: Vec<AstNode>,
    /// Doc comment
    pub doc: Option<String>,
}

/// Import
#[derive(Debug, Clone)]
pub struct Import {
    /// Path
    pub path: Path,
    /// Alias
    pub alias: Option<String>,
    /// Import all (*)
    pub glob: bool,
    /// Nested imports
    pub nested: Vec<Import>,
}

/// Export
#[derive(Debug, Clone)]
pub struct Export {
    /// Item
    pub item: Box<AstNode>,
    /// Visibility
    pub visibility: Visibility,
}

/// Visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Private
    Private,
    /// Public
    Public,
    /// Crate-visible
    Crate,
    /// Super-visible
    Super,
    /// Path-restricted
    Restricted,
}

// ============================================================================
// DECLARATIONS
// ============================================================================

/// Function
#[derive(Debug, Clone)]
pub struct Function {
    /// Name
    pub name: String,
    /// Generic parameters
    pub generics: Generics,
    /// Parameters
    pub params: Vec<Param>,
    /// Return type
    pub return_type: Option<Box<AstNode>>,
    /// Body
    pub body: Option<Block>,
    /// Async
    pub is_async: bool,
    /// Const
    pub is_const: bool,
    /// Unsafe
    pub is_unsafe: bool,
    /// Visibility
    pub visibility: Visibility,
    /// Doc comment
    pub doc: Option<String>,
}

/// Parameter
#[derive(Debug, Clone)]
pub struct Param {
    /// Name
    pub name: String,
    /// Type
    pub ty: Box<AstNode>,
    /// Pattern
    pub pattern: Option<Pattern>,
    /// Default value
    pub default: Option<Box<AstNode>>,
    /// Is self
    pub is_self: bool,
    /// Is mutable self
    pub is_mut_self: bool,
}

/// Generics
#[derive(Debug, Clone, Default)]
pub struct Generics {
    /// Type parameters
    pub params: Vec<GenericParam>,
    /// Where clause
    pub where_clause: Vec<WherePredicate>,
}

/// Generic parameter
#[derive(Debug, Clone)]
pub struct GenericParam {
    /// Name
    pub name: String,
    /// Bounds
    pub bounds: Vec<TypeBound>,
    /// Default
    pub default: Option<Box<AstNode>>,
    /// Const generic
    pub is_const: bool,
}

/// Type bound
#[derive(Debug, Clone)]
pub struct TypeBound {
    /// Path (trait)
    pub path: Path,
    /// Lifetime
    pub lifetime: Option<String>,
}

/// Where predicate
#[derive(Debug, Clone)]
pub struct WherePredicate {
    /// Type
    pub ty: Box<AstNode>,
    /// Bounds
    pub bounds: Vec<TypeBound>,
}

/// Struct definition
#[derive(Debug, Clone)]
pub struct StructDef {
    /// Name
    pub name: String,
    /// Generics
    pub generics: Generics,
    /// Fields
    pub fields: StructFields,
    /// Visibility
    pub visibility: Visibility,
    /// Doc comment
    pub doc: Option<String>,
}

/// Struct fields
#[derive(Debug, Clone)]
pub enum StructFields {
    /// Named fields
    Named(Vec<Field>),
    /// Tuple fields
    Tuple(Vec<TupleField>),
    /// Unit struct
    Unit,
}

/// Named field
#[derive(Debug, Clone)]
pub struct Field {
    /// Name
    pub name: String,
    /// Type
    pub ty: Box<AstNode>,
    /// Visibility
    pub visibility: Visibility,
    /// Doc comment
    pub doc: Option<String>,
}

/// Tuple field
#[derive(Debug, Clone)]
pub struct TupleField {
    /// Type
    pub ty: Box<AstNode>,
    /// Visibility
    pub visibility: Visibility,
}

/// Enum definition
#[derive(Debug, Clone)]
pub struct EnumDef {
    /// Name
    pub name: String,
    /// Generics
    pub generics: Generics,
    /// Variants
    pub variants: Vec<EnumVariant>,
    /// Visibility
    pub visibility: Visibility,
    /// Doc comment
    pub doc: Option<String>,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Name
    pub name: String,
    /// Fields
    pub fields: StructFields,
    /// Discriminant
    pub discriminant: Option<Box<AstNode>>,
    /// Doc comment
    pub doc: Option<String>,
}

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// Name
    pub name: String,
    /// Generics
    pub generics: Generics,
    /// Super traits
    pub supertraits: Vec<TypeBound>,
    /// Items
    pub items: Vec<AstNode>,
    /// Auto trait
    pub is_auto: bool,
    /// Unsafe trait
    pub is_unsafe: bool,
    /// Visibility
    pub visibility: Visibility,
    /// Doc comment
    pub doc: Option<String>,
}

/// Implementation block
#[derive(Debug, Clone)]
pub struct ImplBlock {
    /// Generics
    pub generics: Generics,
    /// Trait (if implementing a trait)
    pub trait_path: Option<Path>,
    /// Type
    pub self_ty: Box<AstNode>,
    /// Items
    pub items: Vec<AstNode>,
    /// Unsafe impl
    pub is_unsafe: bool,
    /// Negative impl
    pub is_negative: bool,
}

/// Type alias
#[derive(Debug, Clone)]
pub struct TypeAlias {
    /// Name
    pub name: String,
    /// Generics
    pub generics: Generics,
    /// Type
    pub ty: Box<AstNode>,
    /// Visibility
    pub visibility: Visibility,
}

/// Constant definition
#[derive(Debug, Clone)]
pub struct ConstDef {
    /// Name
    pub name: String,
    /// Type
    pub ty: Box<AstNode>,
    /// Value
    pub value: Box<AstNode>,
    /// Visibility
    pub visibility: Visibility,
}

/// Static definition
#[derive(Debug, Clone)]
pub struct StaticDef {
    /// Name
    pub name: String,
    /// Type
    pub ty: Box<AstNode>,
    /// Value
    pub value: Box<AstNode>,
    /// Mutable
    pub is_mut: bool,
    /// Visibility
    pub visibility: Visibility,
}

// ============================================================================
// STATEMENTS
// ============================================================================

/// Let statement
#[derive(Debug, Clone)]
pub struct LetStmt {
    /// Pattern
    pub pattern: Pattern,
    /// Type annotation
    pub ty: Option<Box<AstNode>>,
    /// Initializer
    pub init: Option<Box<AstNode>>,
    /// Mutable
    pub is_mut: bool,
}

/// Block
#[derive(Debug, Clone)]
pub struct Block {
    /// Statements
    pub stmts: Vec<AstNode>,
    /// Expression (result)
    pub expr: Option<Box<AstNode>>,
}

// ============================================================================
// EXPRESSIONS
// ============================================================================

/// Literal
#[derive(Debug, Clone)]
pub enum Literal {
    /// Integer
    Int(i128),
    /// Float
    Float(f64),
    /// String
    String(String),
    /// Char
    Char(char),
    /// Bool
    Bool(bool),
    /// Byte string
    ByteString(Vec<u8>),
    /// Unit
    Unit,
}

/// Path
#[derive(Debug, Clone)]
pub struct Path {
    /// Segments
    pub segments: Vec<PathSegment>,
    /// Global (::)
    pub global: bool,
}

impl Path {
    #[inline]
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            segments: vec![PathSegment {
                name: name.into(),
                generics: None,
            }],
            global: false,
        }
    }
}

/// Path segment
#[derive(Debug, Clone)]
pub struct PathSegment {
    /// Name
    pub name: String,
    /// Generic arguments
    pub generics: Option<Vec<AstNode>>,
}

/// Binary operator
#[derive(Debug, Clone)]
pub struct BinaryOp {
    /// Left operand
    pub left: Box<AstNode>,
    /// Operator
    pub op: BinOp,
    /// Right operand
    pub right: Box<AstNode>,
}

/// Binary operator kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    // Logical
    And,
    Or,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Assignment
    Assign,
    // Compound assignment
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    // Range
    Range,
    RangeInclusive,
}

/// Unary operator
#[derive(Debug, Clone)]
pub struct UnaryOp {
    /// Operator
    pub op: UnOp,
    /// Operand
    pub operand: Box<AstNode>,
}

/// Unary operator kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Negation (-)
    Neg,
    /// Not (!)
    Not,
    /// Dereference (*)
    Deref,
    /// Reference (&)
    Ref,
    /// Mutable reference (&mut)
    RefMut,
}

/// Function call
#[derive(Debug, Clone)]
pub struct Call {
    /// Function
    pub func: Box<AstNode>,
    /// Arguments
    pub args: Vec<AstNode>,
}

/// Method call
#[derive(Debug, Clone)]
pub struct MethodCall {
    /// Receiver
    pub receiver: Box<AstNode>,
    /// Method name
    pub method: String,
    /// Generic arguments
    pub generics: Option<Vec<AstNode>>,
    /// Arguments
    pub args: Vec<AstNode>,
}

/// Field access
#[derive(Debug, Clone)]
pub struct FieldAccess {
    /// Object
    pub object: Box<AstNode>,
    /// Field name
    pub field: String,
}

/// Index expression
#[derive(Debug, Clone)]
pub struct Index {
    /// Object
    pub object: Box<AstNode>,
    /// Index
    pub index: Box<AstNode>,
}

/// If expression
#[derive(Debug, Clone)]
pub struct IfExpr {
    /// Condition
    pub condition: Box<AstNode>,
    /// Then branch
    pub then_branch: Block,
    /// Else branch
    pub else_branch: Option<Box<AstNode>>,
}

/// Match expression
#[derive(Debug, Clone)]
pub struct MatchExpr {
    /// Scrutinee
    pub scrutinee: Box<AstNode>,
    /// Arms
    pub arms: Vec<MatchArm>,
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Pattern
    pub pattern: Pattern,
    /// Guard
    pub guard: Option<Box<AstNode>>,
    /// Body
    pub body: Box<AstNode>,
}

/// Loop expression
#[derive(Debug, Clone)]
pub struct LoopExpr {
    /// Label
    pub label: Option<String>,
    /// Body
    pub body: Block,
}

/// While expression
#[derive(Debug, Clone)]
pub struct WhileExpr {
    /// Label
    pub label: Option<String>,
    /// Condition
    pub condition: Box<AstNode>,
    /// Body
    pub body: Block,
}

/// For expression
#[derive(Debug, Clone)]
pub struct ForExpr {
    /// Label
    pub label: Option<String>,
    /// Pattern
    pub pattern: Pattern,
    /// Iterator
    pub iter: Box<AstNode>,
    /// Body
    pub body: Block,
}

/// Closure
#[derive(Debug, Clone)]
pub struct Closure {
    /// Parameters
    pub params: Vec<ClosureParam>,
    /// Return type
    pub return_type: Option<Box<AstNode>>,
    /// Body
    pub body: Box<AstNode>,
    /// Async
    pub is_async: bool,
    /// Move
    pub is_move: bool,
}

/// Closure parameter
#[derive(Debug, Clone)]
pub struct ClosureParam {
    /// Pattern
    pub pattern: Pattern,
    /// Type
    pub ty: Option<Box<AstNode>>,
}

/// Array expression
#[derive(Debug, Clone)]
pub enum ArrayExpr {
    /// List of elements
    Elements(Vec<AstNode>),
    /// Repeat [expr; count]
    Repeat {
        element: Box<AstNode>,
        count: Box<AstNode>,
    },
}

/// Tuple expression
#[derive(Debug, Clone)]
pub struct TupleExpr {
    /// Elements
    pub elements: Vec<AstNode>,
}

/// Struct expression
#[derive(Debug, Clone)]
pub struct StructExpr {
    /// Path
    pub path: Path,
    /// Fields
    pub fields: Vec<FieldInit>,
    /// Base (..base)
    pub base: Option<Box<AstNode>>,
}

/// Field initializer
#[derive(Debug, Clone)]
pub struct FieldInit {
    /// Name
    pub name: String,
    /// Value (None for shorthand)
    pub value: Option<Box<AstNode>>,
}

/// Reference expression
#[derive(Debug, Clone)]
pub struct RefExpr {
    /// Inner expression
    pub inner: Box<AstNode>,
    /// Mutable
    pub is_mut: bool,
}

/// Cast expression
#[derive(Debug, Clone)]
pub struct CastExpr {
    /// Expression
    pub expr: Box<AstNode>,
    /// Target type
    pub ty: Box<AstNode>,
}

// ============================================================================
// PATTERNS
// ============================================================================

/// Pattern
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard (_)
    Wildcard,
    /// Identifier binding
    Ident {
        name: String,
        mutable: bool,
        binding: Option<Box<Pattern>>,
    },
    /// Literal pattern
    Literal(Literal),
    /// Path pattern
    Path(Path),
    /// Tuple pattern
    Tuple(Vec<Pattern>),
    /// Slice pattern
    Slice(Vec<Pattern>),
    /// Struct pattern
    Struct {
        path: Path,
        fields: Vec<PatternField>,
        rest: bool,
    },
    /// Enum/variant pattern
    TupleStruct { path: Path, patterns: Vec<Pattern> },
    /// Or pattern (|)
    Or(Vec<Pattern>),
    /// Reference pattern
    Ref {
        pattern: Box<Pattern>,
        mutable: bool,
    },
    /// Range pattern
    Range {
        start: Option<Box<AstNode>>,
        end: Option<Box<AstNode>>,
        inclusive: bool,
    },
    /// Rest pattern (..)
    Rest,
}

/// Pattern field
#[derive(Debug, Clone)]
pub struct PatternField {
    /// Name
    pub name: String,
    /// Pattern
    pub pattern: Pattern,
}

// ============================================================================
// TYPES
// ============================================================================

/// Type expression
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Path type
    Path(Path),
    /// Tuple type
    Tuple(Vec<AstNode>),
    /// Array type [T; N]
    Array {
        element: Box<AstNode>,
        size: Box<AstNode>,
    },
    /// Slice type [T]
    Slice(Box<AstNode>),
    /// Reference type &T or &mut T
    Ref {
        ty: Box<AstNode>,
        mutable: bool,
        lifetime: Option<String>,
    },
    /// Pointer type *const T or *mut T
    Ptr { ty: Box<AstNode>, mutable: bool },
    /// Function type fn(A) -> B
    Fn {
        params: Vec<AstNode>,
        ret: Option<Box<AstNode>>,
        is_unsafe: bool,
    },
    /// Trait object dyn Trait
    TraitObject(Vec<TypeBound>),
    /// Impl trait impl Trait
    ImplTrait(Vec<TypeBound>),
    /// Never type !
    Never,
    /// Inferred type _
    Infer,
    /// Parenthesized (T)
    Paren(Box<AstNode>),
}

// ============================================================================
// ATTRIBUTES
// ============================================================================

/// Attribute
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Path
    pub path: Path,
    /// Arguments
    pub args: AttributeArgs,
    /// Inner attribute (#![...])
    pub inner: bool,
}

/// Attribute arguments
#[derive(Debug, Clone)]
pub enum AttributeArgs {
    /// No args
    Empty,
    /// Equals value #[attr = "value"]
    Eq(Box<AstNode>),
    /// Delimited args #[attr(args)]
    Delimited(Vec<NestedAttr>),
}

/// Nested attribute
#[derive(Debug, Clone)]
pub enum NestedAttr {
    /// Literal
    Literal(Literal),
    /// Meta item
    Meta { path: Path, args: AttributeArgs },
}

// ============================================================================
// AST VISITOR
// ============================================================================

/// AST visitor trait
pub trait Visitor {
    fn visit_node(&mut self, node: &AstNode) {
        self.walk_node(node);
    }

    fn walk_node(&mut self, node: &AstNode) {
        match &node.kind {
            AstKind::Module(m) => {
                for item in &m.items {
                    self.visit_node(item);
                }
            },
            AstKind::Function(f) => {
                for param in &f.params {
                    self.visit_node(&param.ty);
                }
                if let Some(ret) = &f.return_type {
                    self.visit_node(ret);
                }
                if let Some(body) = &f.body {
                    for stmt in &body.stmts {
                        self.visit_node(stmt);
                    }
                }
            },
            AstKind::Block(b) => {
                for stmt in &b.stmts {
                    self.visit_node(stmt);
                }
                if let Some(expr) = &b.expr {
                    self.visit_node(expr);
                }
            },
            AstKind::BinaryOp(op) => {
                self.visit_node(&op.left);
                self.visit_node(&op.right);
            },
            AstKind::UnaryOp(op) => {
                self.visit_node(&op.operand);
            },
            AstKind::Call(call) => {
                self.visit_node(&call.func);
                for arg in &call.args {
                    self.visit_node(arg);
                }
            },
            AstKind::If(if_expr) => {
                self.visit_node(&if_expr.condition);
                for stmt in &if_expr.then_branch.stmts {
                    self.visit_node(stmt);
                }
                if let Some(else_branch) = &if_expr.else_branch {
                    self.visit_node(else_branch);
                }
            },
            _ => {},
        }
    }
}

/// Mutable AST visitor
pub trait MutVisitor {
    fn visit_node(&mut self, node: &mut AstNode) {
        self.walk_node(node);
    }

    fn walk_node(&mut self, node: &mut AstNode) {
        match &mut node.kind {
            AstKind::Module(m) => {
                for item in &mut m.items {
                    self.visit_node(item);
                }
            },
            AstKind::Block(b) => {
                for stmt in &mut b.stmts {
                    self.visit_node(stmt);
                }
                if let Some(expr) = &mut b.expr {
                    self.visit_node(expr);
                }
            },
            AstKind::BinaryOp(op) => {
                self.visit_node(&mut op.left);
                self.visit_node(&mut op.right);
            },
            _ => {},
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_creation() {
        let node = AstNode::new(AstKind::Literal(Literal::Int(42)));
        assert!(node.id.0 > 0);
    }

    #[test]
    fn test_span_merge() {
        let s1 = Span::new(0, 10, 1, 0);
        let s2 = Span::new(15, 25, 2, 5);
        let merged = s1.merge(s2);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 25);
    }

    #[test]
    fn test_path() {
        let path = Path::simple("foo");
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0].name, "foo");
    }
}
